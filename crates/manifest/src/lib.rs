// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Richard Majewski - Varanid Works

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use etcetera::{choose_base_strategy, BaseStrategy};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestV1 {
    pub schema: String,
    pub podci_version: String,
    pub timestamp_utc: String,
    pub project: String,
    pub job: String,
    pub profile: String,
    pub namespace: String,
    pub env_id: String,
    pub base_image_digest: Option<String>,
    /// Best-effort status for `base_image_digest` capture.
    ///
    /// Values (current):
    ///   - "present": digest captured successfully
    ///   - "unavailable": digest not present in inspect output
    ///   - "error": digest capture failed (inspect error)
    ///
    /// This field is additive and may evolve; consumers should treat unknown
    /// values as "unknown".
    #[serde(default)]
    pub base_image_digest_status: Option<String>,
    pub steps: Vec<ManifestStepV1>,
    pub result: ManifestResultV1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestStepV1 {
    pub name: String,
    pub argv: Vec<String>,
    pub duration_ms: Option<u64>,
    pub exit_code: Option<i32>,
    /// Relative path (from the per-run directory) to the captured stdout log for this step.
    pub stdout_path: Option<String>,
    /// Relative path (from the per-run directory) to the captured stderr log for this step.
    pub stderr_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestResultV1 {
    pub ok: bool,
    pub exit_code: i32,
    pub error: Option<String>,
}

pub fn new_run_id() -> String {
    let ts = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let rand: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();
    format!("{ts}-{rand}")
}

pub fn state_dirs() -> Result<(PathBuf, PathBuf)> {
    // XDG compliance is intentionally explicit and minimal:
    //   state: $XDG_STATE_HOME/podci (fallback: ~/.local/state/podci)
    //   cache: $XDG_CACHE_HOME/podci (fallback: ~/.cache/podci)
    //
    // Avoid surprising directory layouts derived from (qualifier, org, app)
    // tuples; multiple SRE teams expect the canonical XDG locations.
    let base = choose_base_strategy().context("unable to resolve home directory")?;

    let state_home = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| base.state_dir())
        .unwrap_or_else(|| base.home_dir().join(".local").join("state"));

    let cache_home = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| base.cache_dir());

    Ok((state_home.join("podci"), cache_home.join("podci")))
}

pub async fn write_manifest_v1(run_id: &str, m: &ManifestV1) -> Result<PathBuf> {
    let (state_dir, _) = state_dirs()?;
    let run_dir = state_dir.join("runs").join(run_id);
    fs::create_dir_all(&run_dir).await?;

    let path = run_dir.join("manifest.json");
    let bytes = serde_json::to_vec_pretty(m)?;
    fs::write(&path, bytes).await?;

    // Also update "latest" pointer by copying.
    let latest = state_dir.join("manifest.json");
    fs::write(&latest, serde_json::to_vec_pretty(m)?).await?;

    Ok(path)
}

pub fn now_utc_rfc3339() -> String {
    let now: DateTime<Utc> = Utc::now();
    now.to_rfc3339()
}

pub fn manifest_schema_v1() -> &'static str {
    "podci-manifest.v1"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_dirs_respects_xdg_overrides() {
        // Manual temp dir creation to avoid additional dev-deps.
        let root = std::env::temp_dir().join(format!("podci-test-{}", new_run_id()));
        let state = root.join("state");
        let cache = root.join("cache");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::create_dir_all(&cache).unwrap();

        let prev_state = std::env::var_os("XDG_STATE_HOME");
        let prev_cache = std::env::var_os("XDG_CACHE_HOME");
        std::env::set_var("XDG_STATE_HOME", &state);
        std::env::set_var("XDG_CACHE_HOME", &cache);

        let (sd, cd) = state_dirs().unwrap();
        assert_eq!(sd, state.join("podci"));
        assert_eq!(cd, cache.join("podci"));

        // Restore env to avoid cross-test pollution.
        match prev_state {
            Some(v) => std::env::set_var("XDG_STATE_HOME", v),
            None => std::env::remove_var("XDG_STATE_HOME"),
        }
        match prev_cache {
            Some(v) => std::env::set_var("XDG_CACHE_HOME", v),
            None => std::env::remove_var("XDG_CACHE_HOME"),
        }

        let _ = std::fs::remove_dir_all(&root);
    }
}
