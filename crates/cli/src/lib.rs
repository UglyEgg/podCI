// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Richard Majewski - Varanid Works

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use podci_config::Config;
use podci_manifest::{
    manifest_schema_v1, new_run_id, now_utc_rfc3339, state_dirs, write_manifest_v1,
    ManifestResultV1, ManifestStepV1, ManifestV1,
};
use podci_namespace::{blake3_fingerprint, namespace_from};
use podci_podman::Podman;
use podci_podman::{PodmanErrorKind, PodmanRunError};
use std::collections::BTreeMap;
use std::fs;
use std::io::IsTerminal;
use std::path::Path;
use std::path::PathBuf;
use tokio::fs as async_fs;
use tracing::{info, warn};

/// Public CLI definition used by the packaging-assets generator.
#[derive(Debug, Parser, Clone)]
#[command(name = "podci", version = env!("CARGO_PKG_VERSION"), arg_required_else_help = true, subcommand_required = false)]
#[command(about = "podCI: Podman Continuous Integration runner (local-first CI parity)", long_about = None)]
pub struct CliForGen {
    /// Path to podci.toml
    #[arg(long, default_value = "podci.toml")]
    pub config: PathBuf,

    /// Override templates root (contains per-template subdirectories).
    ///
    /// Equivalent to setting the PODCI_TEMPLATES_DIR environment variable.
    #[arg(long, env = "PODCI_TEMPLATES_DIR")]
    pub templates_dir: Option<PathBuf>,

    /// Log format: human or jsonl
    #[arg(long, env = "PODCI_LOG_FORMAT", default_value = "human")]
    pub log_format: String,

    /// Show branding/about info and exit
    #[arg(long)]
    pub about: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand, Clone)]
pub enum TemplatesCommand {
    /// List available templates (disk + embedded fallback).
    List,
    /// Show the resolved origin for a template name.
    Where {
        /// Template name.
        name: String,
    },
    /// Export a template bundle to a `.tar.gz` file.
    ///
    /// The archive layout matches the templates root: `<name>/template.toml` + `<name>/files/*`.
    ///
    /// The output file must end with `.tar.gz`. Export never writes the binary bundle to stdout.
    Export {
        /// Template name.
        name: String,

        /// Output path for the `.tar.gz` bundle.
        output: PathBuf,
    },
}

#[derive(Debug, Subcommand, Clone)]
pub enum Commands {
    Run {
        #[arg(long, default_value = "default")]
        job: String,
        #[arg(long)]
        step: Option<String>,
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        dry_run: bool,

        /// Pull base layers when (re)building template images
        #[arg(long)]
        pull: bool,

        /// Force rebuild of template images (implies --no-cache)
        #[arg(long)]
        rebuild: bool,
    },
    Doctor,
    Init {
        #[arg(long, default_value = "generic")]
        template: String,
        #[arg(long, default_value = ".")]
        dir: PathBuf,
        #[arg(long)]
        project: Option<String>,
    },
    /// Manage podCI templates
    Templates {
        #[command(subcommand)]
        cmd: TemplatesCommand,
    },
    Manifest {
        #[command(subcommand)]
        sub: ManifestCmd,
    },
    Prune {
        #[arg(long, default_value_t = 3)]
        keep: usize,
        #[arg(long)]
        older_than_days: Option<i64>,
        #[arg(long)]
        yes: bool,
    },
    Version,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ManifestCmd {
    Show {
        #[arg(long)]
        latest: bool,
        #[arg(long)]
        run: Option<String>,
    },
}

pub async fn run_cli(cli: CliForGen) -> Result<()> {
    if cli.about {
        print_about();
        return Ok(());
    }

    init_tracing(&cli.log_format)?;

    let cwd = std::env::current_dir().context("resolve current directory")?;
    let template_roots =
        podci_templates::template_search_roots(&cwd, cli.templates_dir.as_deref())?;

    let cmd = match cli.command {
        Some(c) => c,
        None => {
            // main() prints help and exits with code 2 for this case; keep a
            // defensive fallback here for library callers.
            bail!("missing command");
        }
    };

    match cmd {
        Commands::Version => {
            println!("{}", env!("CARGO_PKG_VERSION"));
        }
        Commands::Doctor => doctor().await?,
        Commands::Init {
            template,
            dir,
            project,
        } => {
            if dir.exists() {
                if !dir.is_dir() {
                    bail!("init --dir path is not a directory: {}", dir.display());
                }
            } else {
                fs::create_dir_all(&dir)
                    .with_context(|| format!("create directory {}", dir.display()))?;
            }

            // Per repo process decision: init destination must be empty (no overwrites).
            let mut it =
                fs::read_dir(&dir).with_context(|| format!("read directory {}", dir.display()))?;
            if it.next().is_some() {
                bail!(
                    "init destination directory must be empty: {}",
                    dir.display()
                );
            }

            let project_name = project.unwrap_or_else(|| {
                dir.file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "podci-project".to_string())
            });

            podci_templates::init_template(&template_roots, &template, &dir, &project_name)
                .await
                .with_context(|| format!("init from template '{template}'"))?;

            println!("Initialized {} from template '{template}'", dir.display());
        }
        Commands::Templates { cmd } => match cmd {
            TemplatesCommand::List => {
                for t in podci_templates::list_templates(&template_roots)? {
                    println!("{}", t.name);
                }
            }
            TemplatesCommand::Where { name } => {
                let t = podci_templates::resolve_template(&template_roots, &name)?;
                match t.origin {
                    podci_templates::TemplateOrigin::Disk(p) => println!("{}", p.display()),
                    podci_templates::TemplateOrigin::Embedded => println!("embedded"),
                }
            }
            TemplatesCommand::Export { name, output } => {
                if output.as_os_str() == std::ffi::OsStr::new("-") {
                    bail!("refusing to export template bundle to stdout; provide a .tar.gz output path");
                }
                podci_templates::export_template_tar_gz_to_path(&template_roots, &name, &output)?;
            }
        },
        Commands::Manifest { sub } => match sub {
            ManifestCmd::Show { latest, run } => manifest_show(latest, run).await?,
        },
        Commands::Prune {
            keep,
            older_than_days,
            yes,
        } => prune(keep, older_than_days, yes).await?,
        Commands::Run {
            job,
            step,
            profile,
            dry_run,
            pull,
            rebuild,
        } => run(cli.config, job, step, profile, dry_run, pull, rebuild).await?,
    }

    Ok(())
}

/// Return short operator-oriented remediation hints for common failures.
///
/// This is intentionally kept in the CLI layer (not the podman wrapper) so the
/// error classifier remains pure and the operator guidance can evolve without
/// changing lower-level APIs.
pub fn operator_hints_for_error(err: &anyhow::Error) -> Option<&'static str> {
    for cause in err.chain() {
        if let Some(podman_err) = cause.downcast_ref::<PodmanRunError>() {
            return Some(hints_for_podman_kind(&podman_err.kind));
        }
    }
    None
}

fn hints_for_podman_kind(kind: &PodmanErrorKind) -> &'static str {
    match *kind {

        PodmanErrorKind::NotInstalled => {
            "podman is not installed or not on PATH. Install Podman and ensure `podman` is available in your shell PATH."
        }
        PodmanErrorKind::PermissionDenied => {
            "podman returned a permission error. Verify rootless Podman is working for your user (try `podman info`). If SELinux is enforcing, ensure volume mounts use proper labels (e.g., `:Z`) and that your storage directory is writable."
        }
        PodmanErrorKind::StorageError => {
            "podman storage appears unhealthy. Common fixes: (1) ensure you have free disk space/inodes, (2) run `podman system check`, (3) if storage is corrupt, consider `podman system reset` (destructive). If podCI printed stderr/stdout file paths, review those logs for the exact storage error."
        }
        PodmanErrorKind::CommandFailed => {
            "the container step failed. Review the step stderr/stdout (podCI prints log paths when available) and re-run with `RUST_LOG=info` for more context. If the failure is deterministic, it should reproduce locally with the same podCI profile/job."
        }
        PodmanErrorKind::Unknown => {
            "podman failed for an unknown reason. Re-run with `RUST_LOG=info` and inspect the stderr/stdout logs if paths are shown. If this persists, capture `podman info --debug` output."
        }
    }
}

fn print_about() {
    const ABOUT_SPLIT_COL: usize = 19;
    const ART_LINES: [&str; 6] = [
        "                 _   ___ _____ ",
        " _ __   ___   __| | / __\\_    \\",
        "| '_ \\ / _ \\ / _` |/ /    / /\\/",
        "| |_) | (_) | (_| / /__/\\/ /_  ",
        "| .__/ \\___/ \\__,_\\____\\____/  ",
        "|_|",
    ];

    const TITLE: &str = "Podman Continuous Integration";
    const TAGLINE: &str = "Build it the same way. Every time.";
    const COPYRIGHT: &str = "(c) 2026 Richard Majewski - Varanid Works";
    const VERSION: &str = env!("CARGO_PKG_VERSION");

    let color = supports_color_stdout();

    let art_width = ART_LINES.iter().map(|s| s.len()).max().unwrap_or(0);
    // Align the right-hand text with the art so the title lands on the line
    // that includes the two '\\' characters.
    let right_lines: [Option<&str>; 6] = [
        None,
        Some(TITLE),
        None,
        Some(TAGLINE),
        None,
        Some(COPYRIGHT),
    ];

    // Blank line at top for a cleaner terminal presentation.
    println!();

    for (i, line) in ART_LINES.iter().enumerate() {
        // Add the version on the last line, positioned near the tail of the 'p'.
        let line = if i == ART_LINES.len() - 1 {
            format!("{line}  v {VERSION}")
        } else {
            line.to_string()
        };

        let line_width = if i == ART_LINES.len() - 1 {
            art_width.saturating_sub(3)
        } else {
            art_width
        };

        let padded = format!("{line:<line_width$}", line_width = line_width);
        let padded_len = padded.len();
        let art = if color {
            let (left, right) = if padded.len() > ABOUT_SPLIT_COL {
                padded.split_at(ABOUT_SPLIT_COL)
            } else {
                (padded.as_str(), "")
            };
            format!(
                "\x1b[32m{left}\x1b[96m{right}\x1b[0m",
                left = left,
                right = right
            )
        } else {
            padded
        };

        let rhs = right_lines[i].unwrap_or("");
        if rhs.is_empty() {
            println!("{art}");
            continue;
        }

        // Keep total line length <= 80 (approx; ANSI is excluded from width).
        let avail = 80usize.saturating_sub(padded_len + 2);
        let rhs = rhs.chars().take(avail).collect::<String>();
        println!("{art}  {rhs}");
    }

    // Blank line at bottom.
    println!();
}

fn supports_color_stdout() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if let Ok(term) = std::env::var("TERM") {
        if term == "dumb" {
            return false;
        }
    }
    std::io::stdout().is_terminal()
}

fn init_tracing(format: &str) -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));

    match format {
        "human" => {
            tracing_subscriber::fmt().with_env_filter(filter).init();
        }
        "jsonl" => {
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .json()
                .with_current_span(true)
                .init();
        }
        other => bail!("invalid --log-format '{other}' (expected human|jsonl)"),
    }
    Ok(())
}

async fn doctor() -> Result<()> {
    fn ok(msg: &str) {
        println!("OK   {msg}");
    }
    fn warn(msg: &str) {
        println!("WARN {msg}");
    }
    fn fail(msg: &str) {
        println!("FAIL {msg}");
    }

    // 1) XDG state/cache dirs
    let (state_dir, cache_dir) = state_dirs()?;
    if state_dir.exists() {
        ok(&format!("state dir: {}", state_dir.display()));
    } else {
        tokio::fs::create_dir_all(&state_dir)
            .await
            .with_context(|| format!("create {}", state_dir.display()))?;
        ok(&format!("state dir created: {}", state_dir.display()));
    }
    if cache_dir.exists() {
        ok(&format!("cache dir: {}", cache_dir.display()));
    } else {
        tokio::fs::create_dir_all(&cache_dir)
            .await
            .with_context(|| format!("create {}", cache_dir.display()))?;
        ok(&format!("cache dir created: {}", cache_dir.display()));
    }

    // Basic writeability probe.
    let probe = state_dir.join("doctor-write-probe.tmp");
    match tokio::fs::write(&probe, b"ok").await {
        Ok(()) => {
            let _ = tokio::fs::remove_file(&probe).await;
            ok("state dir writable");
        }
        Err(e) => {
            fail(&format!("state dir not writable: {e}"));
        }
    }

    // 2) Podman presence
    let podman = match Podman::detect() {
        Ok(p) => {
            ok(&format!("podman found: {}", p.path.display()));
            p
        }
        Err(e) => {
            fail(&format!("podman not found on PATH: {e}"));
            bail!("podman not found");
        }
    };

    // 3) Podman version/info
    let v = podman
        .version()
        .await
        .unwrap_or_else(|_| "(unknown)".to_string());
    ok(&format!("podman version: {v}"));

    let info = podman
        .info_json()
        .await
        .context("podman info (rootless environment check)")?;

    if let Some(host) = info.get("host") {
        if let Some(os) = host.get("os").and_then(|v| v.as_str()) {
            ok(&format!("podman host os: {os}"));
        }
        // Rootless hint (best-effort; schema differs by version).
        if let Some(rootless) = host
            .get("security")
            .and_then(|s| s.get("rootless"))
            .and_then(|v| v.as_bool())
        {
            if rootless {
                ok("podman rootless: true");
            } else {
                warn("podman rootless: false (podCI expects rootless + userns=keep-id)");
            }
        } else {
            warn("podman rootless status: unavailable (info schema differs)");
        }
    }

    // 4) Volume create/remove with labels (prune safety prerequisite)
    let vol = format!("podci_doctor_{}", new_run_id());
    let labels = [("podci.managed", "true"), ("podci.doctor", "true")];
    match podman.volume_create_with_labels(&vol, &labels).await {
        Ok(()) => {
            ok("podman volume create (labeled)");
            match podman.volume_inspect_info(&vol).await {
                Ok(info) => {
                    if info.labels.get("podci.managed").map(|v| v.as_str()) == Some("true") {
                        ok("podman volume labels readable");
                    } else {
                        warn("podman volume labels missing/unreadable");
                    }
                }
                Err(e) => warn(&format!("podman volume inspect failed: {e}")),
            }
            let _ = podman.volume_remove(&vol, true).await;
            ok("podman volume remove");
        }
        Err(e) => {
            fail(&format!("podman volume create failed: {e}"));
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct PodmanCacheVolumes<'a> {
    cargo_registry: &'a str,
    cargo_git: &'a str,
    target: &'a str,
}

#[derive(Debug)]
struct PodmanRunArgsInputs<'a> {
    repo_root: &'a Path,
    workdir_display: String,
    volumes: PodmanCacheVolumes<'a>,
    image: &'a str,
    env_kv: &'a [(String, String)],
    argv: &'a [String],
}

fn build_podman_run_args(input: PodmanRunArgsInputs<'_>) -> Vec<String> {
    let PodmanRunArgsInputs {
        repo_root,
        workdir_display,
        volumes,
        image,
        env_kv,
        argv,
    } = input;

    let mut args: Vec<String> = Vec::new();
    args.push("run".to_string());
    args.push("--rm".to_string());
    args.push("--userns=keep-id".to_string());

    // Cache mounts (SELinux: :Z).
    args.push("-v".to_string());
    args.push(format!(
        "{0}:/usr/local/cargo/registry:Z",
        volumes.cargo_registry
    ));
    args.push("-v".to_string());
    args.push(format!("{0}:/usr/local/cargo/git:Z", volumes.cargo_git));
    args.push("-v".to_string());
    args.push(format!("{0}:/work/target:Z", volumes.target));

    // Repo mount.
    args.push("-v".to_string());
    args.push(format!("{}:/work:Z", repo_root.display()));
    args.push("-w".to_string());
    args.push(workdir_display);

    // Enforced contracts for podCI template images.
    args.push("--env".to_string());
    args.push("CARGO_HOME=/usr/local/cargo".to_string());

    for (k, v) in env_kv {
        args.push("--env".to_string());
        args.push(format!("{k}={v}"));
    }

    args.push(image.to_string());
    for a in argv {
        args.push(a.clone());
    }
    args
}

fn compute_env_id(cfg: &Config, job_name: &str, profile_name: &str) -> Result<String> {
    let job = cfg.job(job_name)?;
    let profile = cfg.profile(profile_name)?;

    #[derive(serde::Serialize)]
    struct StepFp<'a> {
        run: &'a [String],
        workdir: &'a Option<String>,
        env: &'a BTreeMap<String, String>,
    }

    #[derive(serde::Serialize)]
    struct Fingerprint<'a> {
        version: u32,
        project: &'a str,
        job: &'a str,
        profile: &'a str,
        container: &'a str,
        profile_env: &'a BTreeMap<String, String>,
        step_order: &'a [String],
        steps: BTreeMap<&'a str, StepFp<'a>>,
    }

    let mut steps_map: BTreeMap<&str, StepFp<'_>> = BTreeMap::new();
    for (name, step) in &job.steps {
        steps_map.insert(
            name.as_str(),
            StepFp {
                run: step.run.as_slice(),
                workdir: &step.workdir,
                env: &step.env,
            },
        );
    }

    let fp = Fingerprint {
        version: cfg.version,
        project: &cfg.project,
        job: job_name,
        profile: profile_name,
        container: &profile.container,
        profile_env: &profile.env,
        step_order: &job.step_order,
        steps: steps_map,
    };

    blake3_fingerprint(&fp)
}

async fn run(
    config_path: PathBuf,
    job_name: String,
    step_only: Option<String>,
    profile_override: Option<String>,
    dry_run: bool,
    pull: bool,
    rebuild: bool,
) -> Result<()> {
    let cfg_text = fs::read_to_string(&config_path)
        .with_context(|| format!("read {}", config_path.display()))?;
    let cfg = Config::from_toml_str(&cfg_text)?;

    let job = cfg.job(&job_name)?;
    let profile_name = profile_override.unwrap_or_else(|| job.profile.clone());
    let profile = cfg.profile(&profile_name)?;

    let env_id = compute_env_id(&cfg, &job_name, &profile_name)?;
    let ns = namespace_from(&cfg.project, &job_name, &env_id);

    let cfg_parent = config_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let repo_root = cfg_parent.canonicalize().context("resolve repo root")?;

    let podman = Podman::detect().context("podman not found on PATH")?;
    let (image, base_digest, base_digest_status) =
        resolve_or_build_image(&profile.container, &podman, pull, rebuild).await?;

    // Default caches: cargo registry/git and target directory.
    // These are namespaced by the computed namespace to avoid cross-project poisoning.
    // Volumes are labeled for safe, ownership-based pruning.
    let vol_cargo_registry = format!("{ns}_cargo_registry");
    let vol_cargo_git = format!("{ns}_cargo_git");
    let vol_target = format!("{ns}_target");

    let volumes = PodmanCacheVolumes {
        cargo_registry: &vol_cargo_registry,
        cargo_git: &vol_cargo_git,
        target: &vol_target,
    };

    let ns_label = ns.clone();
    let env_label = env_id.clone();

    for (v, kind) in [
        (&volumes.cargo_registry, "cargo_registry"),
        (&volumes.cargo_git, "cargo_git"),
        (&volumes.target, "target"),
    ] {
        if !podman.volume_exists(v).await? {
            let labels = [
                ("podci.managed", "true"),
                ("podci.namespace", ns_label.as_str()),
                ("podci.env_id", env_label.as_str()),
                ("podci.volume_kind", kind),
            ];
            podman
                .volume_create_with_labels(v, &labels)
                .await
                .with_context(|| format!("create volume {v}"))?;
        } else {
            // If a volume predates label ownership, podCI will still use it, but it won't be
            // eligible for safe pruning until recreated.
            if let Ok(info) = podman.volume_inspect_info(v).await {
                if info.labels.get("podci.managed").map(|v| v.as_str()) != Some("true") {
                    warn!(volume=%v, "existing_volume_missing_podci_labels");
                }
            }
        }
    }

    let run_id = new_run_id();
    info!(%run_id, project=%cfg.project, job=%job_name, profile=%profile_name, namespace=%ns, "run_start");

    if base_digest.is_none() {
        warn!(status=%base_digest_status, image=%image, "base_image_digest_missing_reproducibility_weakened");
    }

    let (state_dir, _) = state_dirs()?;
    let run_dir = state_dir.join("runs").join(&run_id);
    let logs_dir = run_dir.join("logs");
    async_fs::create_dir_all(&logs_dir)
        .await
        .with_context(|| format!("create {}", logs_dir.display()))?;
    let mut manifest_steps: Vec<ManifestStepV1> = Vec::new();
    let mut final_ok = true;
    let mut final_exit = 0;
    let mut final_err: Option<String> = None;

    let steps_to_run: Vec<String> = match step_only {
        Some(s) => vec![s],
        None => job.step_order.clone(),
    };

    for s in &steps_to_run {
        if !job.steps.contains_key(s) {
            bail!("unknown step '{s}' for job '{job_name}'");
        }
    }

    for s in steps_to_run {
        let step = &job.steps[&s];
        info!(job=%job_name, step=%s, "step_start");

        if dry_run {
            println!("+ {}", shell_quote(&step.run));
            manifest_steps.push(ManifestStepV1 {
                name: s.clone(),
                argv: step.run.clone(),
                duration_ms: None,
                exit_code: Some(0),
                stdout_path: None,
                stderr_path: None,
            });
            info!(job=%job_name, step=%s, "step_end");
            continue;
        }

        let (_workdir, workdir_display) = resolve_workdir(&repo_root, step.workdir.as_deref())?;
        let start = std::time::Instant::now();
        println!("+ {}", shell_quote(&step.run));

        // Build env: profile.env + step.env
        let mut env_kv: Vec<(String, String)> = Vec::new();
        for (k, v) in &profile.env {
            env_kv.push((k.clone(), v.clone()));
        }
        for (k, v) in &step.env {
            env_kv.push((k.clone(), v.clone()));
        }

        let args = build_podman_run_args(PodmanRunArgsInputs {
            repo_root: &repo_root,
            workdir_display,
            volumes,
            image: &image,
            env_kv: &env_kv,
            argv: &step.run,
        });
        // Convert args to &str slices for the podman layer.
        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let r = podman
            .run_capture_allow_failure(arg_refs.as_slice(), None)
            .await;

        let dur = start.elapsed();
        match r {
            Ok(exec) => {
                let tag = sanitize_for_filename(&s);
                let stdout_rel = format!("logs/{tag}.stdout");
                let stderr_rel = format!("logs/{tag}.stderr");
                let stdout_path = logs_dir.join(format!("{tag}.stdout"));
                let stderr_path = logs_dir.join(format!("{tag}.stderr"));

                async_fs::write(&stdout_path, &exec.stdout)
                    .await
                    .with_context(|| format!("write {}", stdout_path.display()))?;
                async_fs::write(&stderr_path, &exec.stderr)
                    .await
                    .with_context(|| format!("write {}", stderr_path.display()))?;

                if exec.exit_code == 0 {
                    manifest_steps.push(ManifestStepV1 {
                        name: s.clone(),
                        argv: step.run.clone(),
                        duration_ms: Some(dur.as_millis() as u64),
                        exit_code: Some(exec.exit_code),
                        stdout_path: Some(stdout_rel),
                        stderr_path: Some(stderr_rel),
                    });
                    info!(job=%job_name, step=%s, "step_end");
                } else {
                    let cmd = format!("podman {}", shell_quote(&args));
                    let err = podci_podman::PodmanRunError::from_exec(
                        cmd,
                        exec.exit_code,
                        &exec.stdout,
                        &exec.stderr,
                        Some(stdout_path),
                        Some(stderr_path),
                    );

                    final_ok = false;
                    final_exit = exec.exit_code;
                    final_err = Some(format!("step '{s}' failed: {err}"));
                    manifest_steps.push(ManifestStepV1 {
                        name: s.clone(),
                        argv: step.run.clone(),
                        duration_ms: Some(dur.as_millis() as u64),
                        exit_code: Some(exec.exit_code),
                        stdout_path: Some(stdout_rel),
                        stderr_path: Some(stderr_rel),
                    });
                    info!(job=%job_name, step=%s, "step_end");
                    break;
                }
            }
            Err(e) => {
                final_ok = false;
                final_exit = 1;
                final_err = Some(format!("step '{s}' failed: {e}"));
                manifest_steps.push(ManifestStepV1 {
                    name: s.clone(),
                    argv: step.run.clone(),
                    duration_ms: Some(dur.as_millis() as u64),
                    exit_code: Some(1),
                    stdout_path: None,
                    stderr_path: None,
                });
                info!(job=%job_name, step=%s, "step_end");
                break;
            }
        }
    }

    let m = ManifestV1 {
        schema: manifest_schema_v1().to_string(),
        podci_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp_utc: now_utc_rfc3339(),
        project: cfg.project.clone(),
        job: job_name.clone(),
        profile: profile_name.clone(),
        namespace: ns,
        env_id,
        base_image_digest: base_digest,
        base_image_digest_status: Some(base_digest_status),
        steps: manifest_steps,
        result: ManifestResultV1 {
            ok: final_ok,
            exit_code: final_exit,
            error: final_err,
        },
    };

    let out = write_manifest_v1(&run_id, &m).await?;
    info!(path=%out.display(), "manifest_written");

    if final_ok {
        Ok(())
    } else {
        bail!(m.result.error.unwrap_or_else(|| "run failed".to_string()));
    }
}

async fn resolve_or_build_image(
    container: &str,
    podman: &Podman,
    pull: bool,
    rebuild: bool,
) -> Result<(String, Option<String>, String)> {
    match classify_container_ref(container)? {
        ContainerRefKind::ExplicitImageRef => {
            let st = podman.inspect_image_digest_status(container).await?;
            let (digest, status) = digest_from_status(st);
            return Ok((container.to_string(), digest, status));
        }
        ContainerRefKind::SymbolicTemplate => {}
    }

    // Template images: we build them locally from embedded Containerfiles.
    let cf = podci_templates::containerfile_for(container)
        .expect("classify_container_ref guarantees template exists");

    let (_state_dir, cache_dir) = podci_manifest::state_dirs()?;
    let image_dir = cache_dir.join("images").join(container);
    tokio::fs::create_dir_all(&image_dir)
        .await
        .with_context(|| format!("create {}", image_dir.display()))?;
    let containerfile_path = image_dir.join("Containerfile");
    tokio::fs::write(&containerfile_path, cf)
        .await
        .with_context(|| format!("write {}", containerfile_path.display()))?;

    let tag = format!("localhost/podci-{container}:v{}", env!("CARGO_PKG_VERSION"));

    let exists = podman.image_exists(&tag).await?;
    if rebuild && exists {
        podman.remove_image_force(&tag).await?;
    }

    let should_build = rebuild || !exists;
    if should_build {
        podman
            .build_image(&image_dir, &containerfile_path, &tag, pull, rebuild)
            .await
            .with_context(|| format!("build image {tag}"))?;
    }

    let st = podman.inspect_image_digest_status(&tag).await?;
    let (digest, status) = digest_from_status(st);
    Ok((tag, digest, status))
}

fn digest_from_status(st: podci_podman::ImageDigestStatus) -> (Option<String>, String) {
    match st {
        podci_podman::ImageDigestStatus::Present(d) => (Some(d), "present".to_string()),
        podci_podman::ImageDigestStatus::Unavailable => (None, "unavailable".to_string()),
        podci_podman::ImageDigestStatus::Error(_) => (None, "error".to_string()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContainerRefKind {
    SymbolicTemplate,
    ExplicitImageRef,
}

fn classify_container_ref(container: &str) -> Result<ContainerRefKind> {
    if podci_templates::containerfile_for(container).is_ok() {
        return Ok(ContainerRefKind::SymbolicTemplate);
    }

    // External images must be explicit to avoid ambiguity with symbolic template names.
    // Accepted forms include:
    //   - registry/namespace/name[:tag]
    //   - name[:tag]
    //   - name@sha256:<digest>
    if container.contains('/') || container.contains(':') || container.contains('@') {
        // Minimal validation: ensure no whitespace and only common image-ref characters.
        // This is not a full OCI reference parser; it is a guardrail to prevent surprises.
        if container
            .chars()
            .any(|c| c.is_whitespace() || !(c.is_ascii_alphanumeric() || ".-_/@:".contains(c)))
        {
            bail!(
                "invalid container reference '{container}': use only ASCII alphanumerics and .-_/ @ : (no whitespace)"
            );
        }
        return Ok(ContainerRefKind::ExplicitImageRef);
    }

    bail!(
        "unknown container template '{container}'. To use an external image, specify an explicit image reference (e.g. 'docker.io/library/ubuntu:24.04')."
    );
}

fn resolve_workdir(repo_root: &std::path::Path, rel: Option<&str>) -> Result<(PathBuf, String)> {
    let wd = match rel {
        None => repo_root.to_path_buf(),
        Some(s) => {
            if s.starts_with('/') {
                bail!("step.workdir must be relative (got absolute '{s}')");
            }
            if s.contains("..") {
                bail!("step.workdir must not contain '..' (got '{s}')");
            }
            repo_root.join(s)
        }
    };

    if !wd.exists() {
        bail!("step.workdir does not exist on host: {}", wd.display());
    }

    // Container workdir is always rooted at /work.
    let display = match rel {
        None => "/work".to_string(),
        Some(s) => format!("/work/{s}"),
    };
    Ok((wd, display))
}

fn shell_quote(argv: &[String]) -> String {
    argv.iter()
        .map(|s| {
            if s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-._/:".contains(c))
            {
                s.clone()
            } else {
                format!("'{}'", s.replace('\'', "'\\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn sanitize_for_filename(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "step".to_string()
    } else {
        out
    }
}

async fn manifest_show(latest: bool, run: Option<String>) -> Result<()> {
    let (state_dir, _) = podci_manifest::state_dirs()?;
    let path = if latest {
        state_dir.join("manifest.json")
    } else if let Some(id) = run {
        state_dir.join("runs").join(id).join("manifest.json")
    } else {
        bail!("specify --latest or --run <id>");
    };

    if !path.exists() {
        bail!(
            "no manifest found at {} (run `podci run` first)",
            path.display()
        );
    }
    let s =
        fs::read_to_string(&path).with_context(|| format!("read manifest {}", path.display()))?;
    println!("{}", s);
    Ok(())
}

#[derive(Debug, Clone)]
struct PodciVolumeMeta {
    name: String,
    namespace: String,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
}

fn plan_prune_volumes(
    vols: Vec<PodciVolumeMeta>,
    keep: usize,
    older_than_days: Option<i64>,
) -> anyhow::Result<(Vec<podci_gc::Resource>, Vec<String>)> {
    use podci_gc::{select_prune_candidates, PrunePolicy, Resource};
    use std::collections::BTreeMap;

    let mut by_ns: BTreeMap<String, Vec<PodciVolumeMeta>> = BTreeMap::new();
    for v in vols {
        by_ns.entry(v.namespace.clone()).or_default().push(v);
    }

    let mut bases: Vec<Resource> = Vec::new();
    for (ns, members) in &by_ns {
        let mut created: Option<chrono::DateTime<chrono::Utc>> = None;
        for m in members {
            if let Some(dt) = m.created_at {
                created = Some(match created {
                    Some(cur) => cur.max(dt),
                    None => dt,
                });
            }
        }
        bases.push(Resource {
            name: ns.clone(),
            created: created.unwrap_or_else(chrono::Utc::now),
        });
    }

    let policy = PrunePolicy {
        keep,
        older_than_days,
    };
    let candidates = select_prune_candidates(bases.clone(), &policy)?;

    let mut to_delete: Vec<String> = Vec::new();
    for c in &candidates {
        if let Some(members) = by_ns.get(&c.name) {
            to_delete.extend(members.iter().map(|m| m.name.clone()));
        }
    }
    to_delete.sort();
    to_delete.dedup();

    Ok((candidates, to_delete))
}

async fn prune(keep: usize, older_than_days: Option<i64>, yes: bool) -> Result<()> {
    use podci_podman::Podman;

    println!(
        "prune policy: keep={keep} older_than_days={:?}",
        older_than_days
    );

    let podman = Podman::detect()?;

    // Only consider volumes explicitly labeled as podCI-managed.
    // This avoids accidentally pruning volumes created by other tools that happen to share a name prefix.
    let vols = podman.volume_list_by_label("podci.managed", "true").await?;
    if vols.is_empty() {
        println!("no podci-managed volumes found");
        return Ok(());
    }
    let mut owned: Vec<PodciVolumeMeta> = Vec::new();
    for v in vols {
        let info = podman
            .volume_inspect_info(&v)
            .await
            .with_context(|| format!("inspect volume {v}"))?;
        let Some(ns) = info.labels.get("podci.namespace").cloned() else {
            // Defensive: treat missing namespace as non-owned.
            continue;
        };
        owned.push(PodciVolumeMeta {
            name: v,
            namespace: ns,
            created_at: info.created_at,
        });
    }
    if owned.is_empty() {
        println!("no podci-managed volumes with namespace labels found");
        return Ok(());
    }

    let (candidates, to_delete) = plan_prune_volumes(owned, keep, older_than_days)?;
    if to_delete.is_empty() {
        println!("nothing to prune (within keep/age policy)");
        return Ok(());
    }

    println!(
        "prune plan: delete {} volumes across {} namespaces",
        to_delete.len(),
        candidates.len()
    );
    for v in &to_delete {
        println!("  - {v}");
    }

    if !yes {
        println!("dry-run only (re-run with --yes to apply)");
        return Ok(());
    }

    println!("applying prune...");
    for v in &to_delete {
        podman.volume_remove(v, true).await?;
    }
    println!("prune complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use podci_podman::PodmanRunError;

    fn cfg_base() -> Config {
        let s = r#"
version = 1
project = "x"

[profiles.dev]
container = "rust-debian"

[jobs.default]
profile = "dev"
step_order = ["fmt"]

[jobs.default.steps.fmt]
run = ["cargo", "fmt", "--all", "--", "--check"]
"#;
        Config::from_toml_str(s).unwrap()
    }

    #[test]
    fn env_id_is_deterministic() {
        let cfg = cfg_base();
        let a = compute_env_id(&cfg, "default", "dev").unwrap();
        let b = compute_env_id(&cfg, "default", "dev").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn env_id_changes_when_step_run_changes() {
        let mut cfg = cfg_base();
        let a = compute_env_id(&cfg, "default", "dev").unwrap();
        cfg.jobs
            .get_mut("default")
            .unwrap()
            .steps
            .get_mut("fmt")
            .unwrap()
            .run
            .push("--verbose".to_string());
        let b = compute_env_id(&cfg, "default", "dev").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn digest_status_mapping_is_stable() {
        let (d, s) = digest_from_status(podci_podman::ImageDigestStatus::Present(
            "sha256:x".to_string(),
        ));
        assert_eq!(d.as_deref(), Some("sha256:x"));
        assert_eq!(s, "present");

        let (d, s) = digest_from_status(podci_podman::ImageDigestStatus::Unavailable);
        assert!(d.is_none());
        assert_eq!(s, "unavailable");

        let (d, s) = digest_from_status(podci_podman::ImageDigestStatus::Error("boom".to_string()));
        assert!(d.is_none());
        assert_eq!(s, "error");
    }

    #[test]
    fn env_id_changes_when_container_changes() {
        let mut cfg = cfg_base();
        let a = compute_env_id(&cfg, "default", "dev").unwrap();
        cfg.profiles.get_mut("dev").unwrap().container = "rust-alpine".to_string();
        let b = compute_env_id(&cfg, "default", "dev").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn env_id_changes_when_profile_env_changes() {
        let mut cfg = cfg_base();
        let a = compute_env_id(&cfg, "default", "dev").unwrap();
        cfg.profiles
            .get_mut("dev")
            .unwrap()
            .env
            .insert("RUSTFLAGS".to_string(), "-C target-cpu=native".to_string());
        let b = compute_env_id(&cfg, "default", "dev").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn env_id_profile_env_is_order_insensitive() {
        let mut cfg1 = cfg_base();
        cfg1.profiles
            .get_mut("dev")
            .unwrap()
            .env
            .insert("A".to_string(), "1".to_string());
        cfg1.profiles
            .get_mut("dev")
            .unwrap()
            .env
            .insert("B".to_string(), "2".to_string());

        let mut cfg2 = cfg_base();
        cfg2.profiles
            .get_mut("dev")
            .unwrap()
            .env
            .insert("B".to_string(), "2".to_string());
        cfg2.profiles
            .get_mut("dev")
            .unwrap()
            .env
            .insert("A".to_string(), "1".to_string());

        let a = compute_env_id(&cfg1, "default", "dev").unwrap();
        let b = compute_env_id(&cfg2, "default", "dev").unwrap();
        assert_eq!(a, b);
    }
    #[test]
    fn namespace_includes_project_job_and_env_prefix() {
        let cfg = cfg_base();
        let env_id = compute_env_id(&cfg, "default", "dev").unwrap();
        let ns = namespace_from(&cfg.project, "default", &env_id);
        assert!(ns.starts_with("podci_"));
        assert!(ns.contains("_x_"));
        assert!(ns.contains("_default_"));
        // The namespace truncates env_id to 12 characters.
        assert!(ns.ends_with(&env_id[..12]));
    }

    #[test]
    fn podman_args_enforce_cargo_home_and_selinux_labels() {
        let repo = std::path::PathBuf::from("/repo");
        let argv = vec!["cargo".to_string(), "test".to_string()];
        let args = build_podman_run_args(PodmanRunArgsInputs {
            repo_root: &repo,
            workdir_display: "/work".to_string(),
            volumes: PodmanCacheVolumes {
                cargo_registry: "podci_ns_cargo_registry",
                cargo_git: "podci_ns_cargo_git",
                target: "podci_ns_target",
            },
            image: "rust-debian",
            env_kv: &[("RUST_LOG".to_string(), "info".to_string())],
            argv: &argv,
        });
        assert!(args.iter().any(|a| a == "--userns=keep-id"));
        assert!(args.iter().any(|a| a == "CARGO_HOME=/usr/local/cargo"));
        assert!(args
            .iter()
            .any(|a| a.contains(":/usr/local/cargo/registry:Z")));
        assert!(args.iter().any(|a| a.contains(":/usr/local/cargo/git:Z")));
        assert!(args.iter().any(|a| a.contains(":/work/target:Z")));
    }

    #[test]
    fn operator_hints_detect_podman_error_in_chain() {
        let pe = PodmanRunError {
            kind: podci_podman::PodmanErrorKind::StorageError,
            command: "podman run ...".to_string(),
            status: Some(125),
            stderr_trunc: "storage error".to_string(),
            stdout_trunc: "".to_string(),
            stderr_path: None,
            stdout_path: None,
        };
        let err = anyhow::Error::new(pe);
        let hints = operator_hints_for_error(&err).unwrap();
        assert!(hints.contains("storage"));
    }

    #[test]
    fn container_ref_classification_prefers_symbolic_templates() {
        assert_eq!(
            classify_container_ref("rust-debian").unwrap(),
            ContainerRefKind::SymbolicTemplate
        );
    }

    #[test]
    fn container_ref_classification_allows_explicit_image_refs() {
        assert_eq!(
            classify_container_ref("docker.io/library/ubuntu:24.04").unwrap(),
            ContainerRefKind::ExplicitImageRef
        );
        assert_eq!(
            classify_container_ref("ubuntu:24.04").unwrap(),
            ContainerRefKind::ExplicitImageRef
        );
        assert_eq!(
            classify_container_ref("ghcr.io/org/img@sha256:deadbeef").unwrap(),
            ContainerRefKind::ExplicitImageRef
        );
    }

    #[test]
    fn container_ref_classification_rejects_ambiguous_names() {
        let err = classify_container_ref("ubuntu").unwrap_err().to_string();
        assert!(err.contains("unknown container template"));
        assert!(err.contains("explicit image reference"));
    }

    #[test]
    fn prune_plan_uses_keep_policy_and_groups_by_namespace() {
        use chrono::{TimeZone, Utc};

        let vols = vec![
            PodciVolumeMeta {
                name: "podci_ns1_cargo_registry".to_string(),
                namespace: "podci_ns1".to_string(),
                created_at: Some(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()),
            },
            PodciVolumeMeta {
                name: "podci_ns1_target".to_string(),
                namespace: "podci_ns1".to_string(),
                created_at: Some(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()),
            },
            PodciVolumeMeta {
                name: "podci_ns2_cargo_registry".to_string(),
                namespace: "podci_ns2".to_string(),
                created_at: Some(Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap()),
            },
        ];

        // keep newest 1 namespace => prune ns1 (2 vols)
        let (_candidates, to_delete) = plan_prune_volumes(vols, 1, None).unwrap();
        assert_eq!(to_delete.len(), 2);
        assert!(to_delete.iter().any(|v| v == "podci_ns1_cargo_registry"));
        assert!(to_delete.iter().any(|v| v == "podci_ns1_target"));
    }
}
