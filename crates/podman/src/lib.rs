// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Richard Majewski - Varanid Works

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::error::Error as StdError;
use std::fmt;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct Podman {
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ExecResult {
    pub exit_code: i32,
    pub duration: Duration,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct VolumeInfo {
    pub created_at: Option<DateTime<Utc>>,
    pub labels: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy)]
pub enum ExecMode {
    Capture,
}

#[derive(Debug, Clone)]
pub enum PodmanErrorKind {
    NotInstalled,
    PermissionDenied,
    StorageError,
    CommandFailed,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct PodmanRunError {
    pub kind: PodmanErrorKind,
    pub command: String,
    pub status: Option<i32>,
    pub stderr_trunc: String,
    pub stdout_trunc: String,
    pub stderr_path: Option<PathBuf>,
    pub stdout_path: Option<PathBuf>,
}

impl PodmanRunError {
    pub fn from_exec(
        command: String,
        exit_code: i32,
        stdout: &[u8],
        stderr: &[u8],
        stdout_path: Option<PathBuf>,
        stderr_path: Option<PathBuf>,
    ) -> Self {
        let kind = classify_failure(exit_code, stderr);
        Self {
            kind,
            command,
            status: Some(exit_code),
            stderr_trunc: trunc_utf8_lossy(stderr, 16 * 1024),
            stdout_trunc: trunc_utf8_lossy(stdout, 16 * 1024),
            stderr_path,
            stdout_path,
        }
    }
}

impl fmt::Display for PodmanRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "podman failed ({:?}) exit_code={}: {}",
            self.kind,
            self.status.unwrap_or(1),
            self.stderr_trunc
        )?;
        if let Some(p) = &self.stderr_path {
            write!(f, " (stderr: {})", p.display())?;
        }
        if let Some(p) = &self.stdout_path {
            write!(f, " (stdout: {})", p.display())?;
        }
        Ok(())
    }
}

impl StdError for PodmanRunError {}

impl Podman {
    pub fn detect() -> Result<Self> {
        let path = which::which("podman").context("find podman on PATH")?;
        Ok(Self { path })
    }

    pub async fn run_capture(
        &self,
        args: &[&str],
        timeout_dur: Option<Duration>,
    ) -> Result<ExecResult> {
        self.run_capture_with_env(args, &[], None, timeout_dur)
            .await
    }

    pub async fn run_capture_with_env(
        &self,
        args: &[&str],
        env: &[(&str, &str)],
        cwd: Option<&std::path::Path>,
        timeout_dur: Option<Duration>,
    ) -> Result<ExecResult> {
        let mut cmd = Command::new(&self.path);
        cmd.args(args);
        for (k, v) in env {
            cmd.env(k, v);
        }
        if let Some(cwd) = cwd {
            cmd.current_dir(cwd);
        }
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let start = Instant::now();
        info!(cmd=%format_cmd(&self.path, args), event="podman_start");

        let fut = cmd.output();
        let out = if let Some(td) = timeout_dur {
            timeout(td, fut).await.context("podman timed out")??
        } else {
            fut.await?
        };

        self.finish_capture(args, out, start)
    }

    /// Run `podman` and capture stdout/stderr without converting non-zero exit status into an error.
    ///
    /// This is intended for running untrusted workloads (e.g. job steps) where the caller wants access
    /// to the full output even when the process returns a failure exit code.
    pub async fn run_capture_allow_failure(
        &self,
        args: &[&str],
        timeout_dur: Option<Duration>,
    ) -> Result<ExecResult> {
        self.run_capture_with_env_allow_failure(args, &[], None, timeout_dur)
            .await
    }

    pub async fn run_capture_with_env_allow_failure(
        &self,
        args: &[&str],
        env: &[(&str, &str)],
        cwd: Option<&std::path::Path>,
        timeout_dur: Option<Duration>,
    ) -> Result<ExecResult> {
        let mut cmd = Command::new(&self.path);
        cmd.args(args);
        for (k, v) in env {
            cmd.env(k, v);
        }
        if let Some(cwd) = cwd {
            cmd.current_dir(cwd);
        }
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let start = Instant::now();
        info!(cmd=%format_cmd(&self.path, args), event="podman_start");

        let fut = cmd.output();
        let out = if let Some(td) = timeout_dur {
            timeout(td, fut).await.context("podman timed out")??
        } else {
            fut.await?
        };

        self.finish_capture_allow_failure(args, out, start)
    }

    pub async fn run_inherit(
        &self,
        args: &[&str],
        env: &[(&str, &str)],
        cwd: Option<&std::path::Path>,
        timeout_dur: Option<Duration>,
    ) -> Result<ExecResult> {
        let mut cmd = Command::new(&self.path);
        cmd.args(args);
        for (k, v) in env {
            cmd.env(k, v);
        }
        if let Some(cwd) = cwd {
            cmd.current_dir(cwd);
        }
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let start = Instant::now();
        info!(cmd=%format_cmd(&self.path, args), event="podman_start");

        let fut = cmd.status();
        let status = if let Some(td) = timeout_dur {
            timeout(td, fut).await.context("podman timed out")??
        } else {
            fut.await?
        };

        let duration = start.elapsed();
        let exit_code = status.code().unwrap_or(1);
        info!(cmd=%format_cmd(&self.path, args), exit_code, duration_ms=%duration.as_millis(), event="podman_exit");

        if !status.success() {
            // We don't have stderr bytes in inherit mode; provide a short classification-only error.
            let err = PodmanRunError {
                kind: PodmanErrorKind::CommandFailed,
                command: format_cmd(&self.path, args),
                status: Some(exit_code),
                stderr_trunc: String::new(),
                stdout_trunc: String::new(),
                stderr_path: None,
                stdout_path: None,
            };
            return Err(anyhow::Error::new(err));
        }

        Ok(ExecResult {
            exit_code,
            duration,
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }

    fn finish_capture(
        &self,
        args: &[&str],
        out: std::process::Output,
        start: Instant,
    ) -> Result<ExecResult> {
        let duration = start.elapsed();
        let exit_code = out.status.code().unwrap_or(1);
        info!(cmd=%format_cmd(&self.path, args), exit_code, duration_ms=%duration.as_millis(), event="podman_exit");

        if !out.status.success() {
            let kind = classify_failure(exit_code, &out.stderr);
            let err = PodmanRunError {
                kind,
                command: format_cmd(&self.path, args),
                status: Some(exit_code),
                stderr_trunc: trunc_utf8_lossy(&out.stderr, 16 * 1024),
                stdout_trunc: trunc_utf8_lossy(&out.stdout, 16 * 1024),
                stderr_path: None,
                stdout_path: None,
            };
            return Err(anyhow::Error::new(err));
        }

        Ok(ExecResult {
            exit_code,
            duration,
            stdout: out.stdout,
            stderr: out.stderr,
        })
    }

    fn finish_capture_allow_failure(
        &self,
        args: &[&str],
        out: std::process::Output,
        start: Instant,
    ) -> Result<ExecResult> {
        let duration = start.elapsed();
        let exit_code = out.status.code().unwrap_or(1);
        info!(cmd=%format_cmd(&self.path, args), exit_code, duration_ms=%duration.as_millis(), event="podman_exit");

        Ok(ExecResult {
            exit_code,
            duration,
            stdout: out.stdout,
            stderr: out.stderr,
        })
    }

    pub async fn run_capture_allow_fail(
        &self,
        args: &[&str],
        timeout_dur: Option<Duration>,
    ) -> Result<ExecResult> {
        let mut cmd = Command::new(&self.path);
        cmd.args(args);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        let start = Instant::now();
        info!(cmd=%format_cmd(&self.path, args), event="podman_start");
        let fut = cmd.output();
        let out = if let Some(td) = timeout_dur {
            timeout(td, fut).await.context("podman timed out")??
        } else {
            fut.await?
        };
        let duration = start.elapsed();
        let exit_code = out.status.code().unwrap_or(1);
        info!(cmd=%format_cmd(&self.path, args), exit_code, duration_ms=%duration.as_millis(), event="podman_exit");
        Ok(ExecResult {
            exit_code,
            duration,
            stdout: out.stdout,
            stderr: out.stderr,
        })
    }

    pub async fn image_exists(&self, image: &str) -> Result<bool> {
        let r = self
            .run_capture_allow_fail(
                ["image", "exists", image].as_slice(),
                Some(Duration::from_secs(15)),
            )
            .await?;
        Ok(r.exit_code == 0)
    }

    pub async fn volume_exists(&self, name: &str) -> Result<bool> {
        let r = self
            .run_capture_allow_fail(
                ["volume", "exists", name].as_slice(),
                Some(Duration::from_secs(15)),
            )
            .await?;
        Ok(r.exit_code == 0)
    }

    pub async fn volume_create(&self, name: &str) -> Result<()> {
        self.volume_create_with_labels(name, &[]).await
    }

    /// Create a Podman volume with labels.
    ///
    /// Labels are used to mark ownership for safe pruning.
    pub async fn volume_create_with_labels(
        &self,
        name: &str,
        labels: &[(&str, &str)],
    ) -> Result<()> {
        let mut args: Vec<String> = Vec::new();
        args.push("volume".to_string());
        args.push("create".to_string());
        for (k, v) in labels {
            args.push("--label".to_string());
            args.push(format!("{k}={v}"));
        }
        args.push(name.to_string());
        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let _ = self
            .run_capture(arg_refs.as_slice(), Some(Duration::from_secs(30)))
            .await?;
        Ok(())
    }

    pub async fn volume_inspect_info(&self, name: &str) -> Result<VolumeInfo> {
        let r = self
            .run_capture(
                ["volume", "inspect", name, "--format", "json"].as_slice(),
                Some(Duration::from_secs(30)),
            )
            .await?;

        #[derive(Deserialize)]
        struct VolInspect {
            #[serde(rename = "CreatedAt")]
            created_at: Option<String>,
            #[serde(rename = "Labels")]
            labels: Option<std::collections::BTreeMap<String, String>>,
        }

        let rows: Vec<VolInspect> =
            serde_json::from_slice(&r.stdout).context("parse podman volume inspect json")?;
        let Some(row) = rows.into_iter().next() else {
            return Ok(VolumeInfo {
                created_at: None,
                labels: Default::default(),
            });
        };

        let created_at = if let Some(s) = row.created_at {
            Some(
                DateTime::parse_from_rfc3339(&s)
                    .map(|d| d.with_timezone(&Utc))
                    .or_else(|_| {
                        DateTime::parse_from_str(&format!("{s}Z"), "%Y-%m-%dT%H:%M:%S%.f%#z")
                            .map(|d| d.with_timezone(&Utc))
                    })
                    .context("parse podman volume CreatedAt")?,
            )
        } else {
            None
        };

        Ok(VolumeInfo {
            created_at,
            labels: row.labels.unwrap_or_default(),
        })
    }

    pub async fn volume_list(&self) -> Result<Vec<String>> {
        let r = self
            .run_capture(
                ["volume", "ls", "--format", "json"].as_slice(),
                Some(Duration::from_secs(30)),
            )
            .await?;

        #[derive(Deserialize)]
        struct VolRow {
            #[serde(rename = "Name")]
            name: String,
        }

        let rows: Vec<VolRow> =
            serde_json::from_slice(&r.stdout).context("parse podman volume ls json")?;
        Ok(rows.into_iter().map(|r| r.name).collect())
    }

    pub async fn volume_list_by_label(&self, key: &str, value: &str) -> Result<Vec<String>> {
        let filter = format!("label={key}={value}");
        let r = self
            .run_capture(
                [
                    "volume",
                    "ls",
                    "--filter",
                    filter.as_str(),
                    "--format",
                    "json",
                ]
                .as_slice(),
                Some(Duration::from_secs(30)),
            )
            .await?;

        #[derive(Deserialize)]
        struct VolRow {
            #[serde(rename = "Name")]
            name: String,
        }

        let rows: Vec<VolRow> =
            serde_json::from_slice(&r.stdout).context("parse podman volume ls json")?;
        Ok(rows.into_iter().map(|r| r.name).collect())
    }

    pub async fn volume_created_at(&self, name: &str) -> Result<Option<DateTime<Utc>>> {
        let info = self.volume_inspect_info(name).await?;
        Ok(info.created_at)
    }

    pub async fn volume_remove(&self, name: &str, force: bool) -> Result<()> {
        let mut args: Vec<&str> = vec!["volume", "rm"];
        if force {
            args.push("-f");
        }
        args.push(name);
        let _ = self
            .run_capture(args.as_slice(), Some(Duration::from_secs(60)))
            .await?;
        Ok(())
    }

    pub async fn remove_image_force(&self, image: &str) -> Result<()> {
        let _ = self
            .run_capture_allow_fail(
                ["rmi", "-f", image].as_slice(),
                Some(Duration::from_secs(60)),
            )
            .await?;
        Ok(())
    }

    pub async fn build_image(
        &self,
        context_dir: &std::path::Path,
        containerfile_path: &std::path::Path,
        tag: &str,
        pull: bool,
        no_cache: bool,
    ) -> Result<()> {
        let mut args: Vec<String> = Vec::new();
        args.push("build".to_string());
        if pull {
            args.push("--pull".to_string());
        }
        if no_cache {
            args.push("--no-cache".to_string());
        }
        args.push("-f".to_string());
        args.push(containerfile_path.display().to_string());
        args.push("-t".to_string());
        args.push(tag.to_string());
        args.push(context_dir.display().to_string());

        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let _ = self
            .run_inherit(arg_refs.as_slice(), &[], None, None)
            .await?;
        Ok(())
    }

    pub async fn version(&self) -> Result<String> {
        let r = self
            .run_capture(["--version"].as_slice(), Some(Duration::from_secs(10)))
            .await?;
        Ok(String::from_utf8_lossy(&r.stdout).trim().to_string())
    }

    pub async fn info_json(&self) -> Result<serde_json::Value> {
        let r = self
            .run_capture(
                ["info", "--format", "json"].as_slice(),
                Some(Duration::from_secs(30)),
            )
            .await?;
        let v: serde_json::Value =
            serde_json::from_slice(&r.stdout).context("parse podman info json")?;
        Ok(v)
    }

    pub async fn inspect_image_digest(&self, image: &str) -> Result<Option<String>> {
        let st = self.inspect_image_digest_status(image).await?;
        Ok(match st {
            ImageDigestStatus::Present(d) => Some(d),
            ImageDigestStatus::Unavailable => None,
            ImageDigestStatus::Error(e) => {
                warn!(error=%e, image, "image_digest_unavailable");
                None
            }
        })
    }

    pub async fn inspect_image_digest_status(&self, image: &str) -> Result<ImageDigestStatus> {
        // Best-effort: different Podman versions and storage drivers can yield different inspect output.
        let args = ["image", "inspect", "--format", "{{.Digest}}", image];
        let r = self
            .run_capture_allow_fail(args.as_slice(), Some(Duration::from_secs(30)))
            .await?;

        if r.exit_code != 0 {
            return Ok(ImageDigestStatus::Error(trunc_utf8_lossy(
                &r.stderr,
                16 * 1024,
            )));
        }

        let s = String::from_utf8_lossy(&r.stdout).trim().to_string();
        if s.is_empty() || s == "<no value>" {
            Ok(ImageDigestStatus::Unavailable)
        } else {
            Ok(ImageDigestStatus::Present(s))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageDigestStatus {
    Present(String),
    Unavailable,
    Error(String),
}

fn trunc_utf8_lossy(bytes: &[u8], max_len: usize) -> String {
    if bytes.len() <= max_len {
        return String::from_utf8_lossy(bytes).to_string();
    }
    // Keep tail (often contains the actionable error) and mark truncation.
    let tail = &bytes[bytes.len() - max_len..];
    format!(
        "…(truncated, showing last {max_len} bytes)…\n{}",
        String::from_utf8_lossy(tail)
    )
}

fn classify_failure(exit_code: i32, stderr: &[u8]) -> PodmanErrorKind {
    let s = String::from_utf8_lossy(stderr).to_lowercase();
    if s.contains("permission denied") {
        return PodmanErrorKind::PermissionDenied;
    }
    if s.contains("creating container storage") || s.contains("containers/storage") {
        return PodmanErrorKind::StorageError;
    }
    if exit_code == 127 || s.contains("not found") {
        return PodmanErrorKind::NotInstalled;
    }
    PodmanErrorKind::CommandFailed
}

fn format_cmd(bin: &std::path::Path, args: &[&str]) -> String {
    let mut s = String::new();
    s.push_str(bin.to_string_lossy().as_ref());
    for a in args {
        s.push(' ');
        s.push_str(a);
    }
    s
}

#[derive(Debug, Deserialize)]
struct _PodmanInfoMinimal {
    #[allow(dead_code)]
    host: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::{trunc_utf8_lossy, PodmanRunError};

    #[test]
    fn trunc_utf8_lossy_returns_full_when_short() {
        let b = b"hello";
        let s = trunc_utf8_lossy(b, 16);
        assert_eq!(s, "hello");
    }

    #[test]
    fn trunc_utf8_lossy_truncates_tail() {
        let b = b"abcdefghijklmnopqrstuvwxyz";
        let s = trunc_utf8_lossy(b, 5);
        assert!(s.contains("truncated"));
        assert!(s.ends_with("vwxyz"));
    }

    #[test]
    fn podman_run_error_includes_paths_when_present() {
        let err = PodmanRunError::from_exec(
            "podman run ...".to_string(),
            125,
            b"stdout",
            b"stderr",
            Some("/tmp/stdout.log".into()),
            Some("/tmp/stderr.log".into()),
        );
        let s = err.to_string();
        assert!(s.contains("stderr: /tmp/stderr.log"));
        assert!(s.contains("stdout: /tmp/stdout.log"));
    }
}
