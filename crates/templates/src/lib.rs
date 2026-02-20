// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Richard Majewski - Varanid Works

use anyhow::{bail, Context, Result};
use etcetera::{choose_base_strategy, BaseStrategy};
use flate2::{Compression, GzBuilder};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use tokio::fs;

/// Template resolution order:
///  1) explicit override (`--templates-dir` / `PODCI_TEMPLATES_DIR`)
///  2) project-local: `./.podci/templates`
///  3) XDG config: `$XDG_CONFIG_HOME/podci/templates` (fallback: `~/.config/podci/templates`)
///  4) system: `/usr/share/podci/templates`
///
/// The embedded `generic` template is always available as a fallback.
pub fn template_search_roots(cwd: &Path, override_dir: Option<&Path>) -> Result<Vec<PathBuf>> {
    let mut roots = Vec::new();

    if let Some(p) = override_dir {
        roots.push(p.to_path_buf());
    }

    roots.push(cwd.join(".podci").join("templates"));

    let base = choose_base_strategy().context("unable to resolve home directory")?;
    let xdg_config_home = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| base.config_dir());
    roots.push(xdg_config_home.join("podci").join("templates"));

    roots.push(PathBuf::from("/usr/share/podci/templates"));

    Ok(roots)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateOrigin {
    /// Template was found on disk; path points at the template directory.
    Disk(PathBuf),
    /// Built-in fallback (only `generic`).
    Embedded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateEntry {
    pub name: String,
    pub origin: TemplateOrigin,
}

#[derive(Debug, Clone, Deserialize)]
struct TemplateToml {
    pub name: String,
}

/// List available templates by resolving across search roots.
///
/// If multiple roots contain the same template name, the first root wins.
pub fn list_templates(roots: &[PathBuf]) -> Result<Vec<TemplateEntry>> {
    let mut found: BTreeMap<String, TemplateEntry> = BTreeMap::new();

    for root in roots {
        if !root.is_dir() {
            continue;
        }

        for entry in std::fs::read_dir(root)
            .with_context(|| format!("read templates dir {}", root.display()))?
        {
            let entry = entry?;
            let p = entry.path();
            if !p.is_dir() {
                continue;
            }
            let name = match p.file_name() {
                Some(n) => n.to_string_lossy().to_string(),
                None => continue,
            };
            if found.contains_key(&name) {
                continue;
            }
            let tt = p.join("template.toml");
            if !tt.is_file() {
                continue;
            }

            // Best-effort parse; if it fails, still expose the directory by name.
            if let Ok(s) = std::fs::read_to_string(&tt) {
                if let Ok(meta) = toml::from_str::<TemplateToml>(&s) {
                    if meta.name != name {
                        // Directory name is the lookup key; metadata mismatch is informational only.
                    }
                }
            }

            found.insert(
                name.clone(),
                TemplateEntry {
                    name,
                    origin: TemplateOrigin::Disk(p),
                },
            );
        }
    }

    if !found.contains_key("generic") {
        found.insert(
            "generic".to_string(),
            TemplateEntry {
                name: "generic".to_string(),
                origin: TemplateOrigin::Embedded,
            },
        );
    }

    Ok(found.into_values().collect())
}

pub fn resolve_template(roots: &[PathBuf], name: &str) -> Result<TemplateEntry> {
    for root in roots {
        let dir = root.join(name);
        if dir.is_dir() && dir.join("template.toml").is_file() {
            return Ok(TemplateEntry {
                name: name.to_string(),
                origin: TemplateOrigin::Disk(dir),
            });
        }
    }

    if name == "generic" {
        return Ok(TemplateEntry {
            name: "generic".to_string(),
            origin: TemplateOrigin::Embedded,
        });
    }

    bail!("unknown template '{name}'. Use 'podci templates list' to see available templates.")
}

/// Initialize a directory from a named template.
///
/// Safety rules:
/// - Destination directory must exist and be empty (no overwrites).
/// - Template paths are sanitized (no absolute paths / `..`).
/// - Template payload must not contain symlinks.
pub async fn init_template(
    roots: &[PathBuf],
    name: &str,
    out_dir: &Path,
    project: &str,
) -> Result<()> {
    let entry = resolve_template(roots, name)?;
    ensure_dir_empty(out_dir)?;

    match entry.origin {
        TemplateOrigin::Disk(dir) => {
            let files_root = dir.join("files");
            if !files_root.is_dir() {
                bail!(
                    "template '{name}' is missing files/ directory: {}",
                    files_root.display()
                );
            }

            let files = collect_files_sorted(&files_root)?;
            for (rel, abs) in files {
                ensure_safe_rel_path(&rel)?;
                let dst = out_dir.join(&rel);

                if let Some(parent) = dst.parent() {
                    fs::create_dir_all(parent).await?;
                }

                let bytes = fs::read(&abs).await?;
                let out = replace_project_placeholder(&bytes, project);
                fs::write(&dst, out).await?;
            }
        }
        TemplateOrigin::Embedded => {
            // Embedded generic template (minimal, language-agnostic).
            let podci_path = out_dir.join("podci.toml");
            let bytes = GENERIC_PODCI_TOML.as_bytes();
            let out = replace_project_placeholder(bytes, project);
            fs::write(&podci_path, out).await?;
        }
    }

    Ok(())
}

/// Export a template as a deterministic `.tar.gz` stream.
///
/// Note: the CLI writes bundles to a file; this function supports generic writers for testing and internal use.
///
/// The archive layout matches the on-disk templates layout:
///
///   <name>/template.toml
///   <name>/files/<...>
///
/// This allows users to extract directly into a templates root.
pub fn export_template_tar_gz<W: Write>(roots: &[PathBuf], name: &str, w: W) -> Result<()> {
    let entry = resolve_template(roots, name)?;

    // Deterministic gzip header.
    let mut gz = GzBuilder::new().mtime(0).write(w, Compression::default());
    let mut tar = tar::Builder::new(&mut gz);
    tar.mode(tar::HeaderMode::Deterministic);

    match entry.origin {
        TemplateOrigin::Disk(dir) => {
            let meta_path = dir.join("template.toml");
            let meta = std::fs::read(&meta_path)
                .with_context(|| format!("read {}", meta_path.display()))?;
            append_bytes(&mut tar, &format!("{name}/template.toml"), &meta)?;

            let files_root = dir.join("files");
            if !files_root.is_dir() {
                bail!(
                    "template '{name}' is missing files/ directory: {}",
                    files_root.display()
                );
            }

            let files = collect_files_sorted(&files_root)?;
            for (rel, abs) in files {
                ensure_safe_rel_path(&rel)?;
                let bytes =
                    std::fs::read(&abs).with_context(|| format!("read {}", abs.display()))?;
                let path = format!("{name}/files/{}", rel.display());
                append_bytes(&mut tar, &path, &bytes)?;
            }
        }
        TemplateOrigin::Embedded => {
            append_bytes(
                &mut tar,
                "generic/template.toml",
                GENERIC_TEMPLATE_TOML.as_bytes(),
            )?;
            append_bytes(
                &mut tar,
                "generic/files/podci.toml",
                GENERIC_PODCI_TOML.as_bytes(),
            )?;
        }
    }

    tar.finish()?;
    drop(tar);
    gz.finish()?;
    Ok(())
}

/// Export a template bundle to a `.tar.gz` file on disk.
///
/// The output path must end with `.tar.gz` and must not already exist.
pub fn export_template_tar_gz_to_path(roots: &[PathBuf], name: &str, output: &Path) -> Result<()> {
    let out_str = output.to_string_lossy();
    if !out_str.ends_with(".tar.gz") {
        bail!("output path must end with .tar.gz: {}", output.display());
    }
    if output.exists() {
        bail!("output file already exists: {}", output.display());
    }

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create directory {}", parent.display()))?;
        }
    }

    let pid = std::process::id();
    let file_name = output
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "template.tar.gz".to_string());
    let tmp = output.with_file_name(format!(".{file_name}.tmp-{pid}"));

    // Best-effort cleanup on failure.
    let res = (|| -> Result<()> {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .with_context(|| format!("create temp file {}", tmp.display()))?;
        export_template_tar_gz(roots, name, &mut f)?;
        f.sync_all().ok(); // best-effort durability; failure is not fatal for local exports
        drop(f);
        std::fs::rename(&tmp, output)
            .with_context(|| format!("rename {} -> {}", tmp.display(), output.display()))?;
        Ok(())
    })();

    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res
}

fn append_bytes<W: Write>(tar: &mut tar::Builder<W>, path: &str, bytes: &[u8]) -> Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_mtime(0);
    header.set_uid(0);
    header.set_gid(0);
    header.set_cksum();
    tar.append_data(&mut header, path, bytes)?;
    Ok(())
}

fn collect_files_sorted(root: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
    let mut out = Vec::new();
    walk_dir(root, Path::new(""), &mut out)?;
    out.sort_by(|(a, _), (b, _)| a.cmp(b));
    Ok(out)
}

fn walk_dir(root: &Path, rel: &Path, out: &mut Vec<(PathBuf, PathBuf)>) -> Result<()> {
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let p = entry.path();
        let file_name = match p.file_name() {
            Some(n) => n.to_os_string(),
            None => continue,
        };
        let child_rel = rel.join(file_name);
        let ft = entry.file_type()?;
        if ft.is_symlink() {
            bail!("template contains symlink (refused): {}", p.display());
        }
        if ft.is_dir() {
            walk_dir(&p, &child_rel, out)?;
        } else if ft.is_file() {
            out.push((child_rel, p));
        } else {
            bail!("template contains unsupported entry: {}", p.display());
        }
    }
    Ok(())
}

fn ensure_safe_rel_path(p: &Path) -> Result<()> {
    for c in p.components() {
        match c {
            Component::Normal(_) => {}
            _ => bail!("template produced unsafe path component: {}", p.display()),
        }
    }
    Ok(())
}

fn ensure_dir_empty(out_dir: &Path) -> Result<()> {
    if !out_dir.is_dir() {
        bail!("init destination is not a directory: {}", out_dir.display());
    }

    let mut it = std::fs::read_dir(out_dir)
        .with_context(|| format!("read directory {}", out_dir.display()))?;
    if it.next().is_some() {
        bail!(
            "init destination directory must be empty: {}",
            out_dir.display()
        );
    }

    Ok(())
}

fn replace_project_placeholder(bytes: &[u8], project: &str) -> Vec<u8> {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.replace("REPLACE_ME", project).into_bytes(),
        Err(_) => bytes.to_vec(),
    }
}

const GENERIC_TEMPLATE_TOML: &str = r#"# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Richard Majewski - Varanid Works

name = \"generic\"
description = \"Minimal generic starter; edit podci.toml to fit your repo\"
"#;

const GENERIC_PODCI_TOML: &str = r#"# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Richard Majewski - Varanid Works

version = 1
project = \"REPLACE_ME\"

# Generic default: runs a no-op step so `podci run` works immediately.
# Replace this file with a language-specific template (rust/cpp/kde-mixed) or edit by hand.

[profiles.dev]
container = \"alpine:3.20\"

[jobs.default]
profile = \"dev\"
step_order = [\"info\"]

[jobs.default.steps.info]
run = [\"sh\", \"-c\", \"echo 'podCI initialized for REPLACE_ME'; echo 'Edit podci.toml to define real steps.'\"]
"#;

// ---- Containerfile templates (still embedded; used to build podCI's own template images) ----

const CONTAINERFILE_RUST_ALPINE: &str =
    include_str!("../templates/containerfiles/Containerfile.rust-alpine");
const CONTAINERFILE_RUST_DEBIAN: &str =
    include_str!("../templates/containerfiles/Containerfile.rust-debian");
const CONTAINERFILE_CPP_DEBIAN: &str =
    include_str!("../templates/containerfiles/Containerfile.cpp-debian");
const CONTAINERFILE_KDE_MIXED_DEBIAN: &str =
    include_str!("../templates/containerfiles/Containerfile.kde-mixed-debian");

pub fn containerfile_for(platform: &str) -> Result<&'static str> {
    match platform {
        "rust-alpine" => Ok(CONTAINERFILE_RUST_ALPINE),
        "rust-debian" => Ok(CONTAINERFILE_RUST_DEBIAN),
        "cpp-debian" => Ok(CONTAINERFILE_CPP_DEBIAN),
        "kde-mixed-debian" => Ok(CONTAINERFILE_KDE_MIXED_DEBIAN),
        _ => bail!("unknown template image container: {platform}"),
    }
}

pub async fn write_containerfile(platform: &str, dst: &Path) -> Result<()> {
    let template = containerfile_for(platform)?;
    fs::write(dst, template).await?;
    Ok(())
}

#[cfg(test)]
mod tests;
