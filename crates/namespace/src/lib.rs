// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Richard Majewski - Varanid Works

use anyhow::Result;
use blake3::Hasher;
use serde::Serialize;

/// Canonical JSON fingerprint of build-affecting inputs.
///
/// Caller decides what to include; this module guarantees stable hashing.
pub fn blake3_fingerprint<T: Serialize>(value: &T) -> Result<String> {
    // serde_json sorts map keys only for BTreeMap; caller should use stable containers.
    let bytes = serde_json::to_vec(value)?;
    let mut h = Hasher::new();
    h.update(&bytes);
    Ok(h.finalize().to_hex().to_string())
}

pub fn namespace_from(project: &str, job: &str, env_id: &str) -> String {
    // Conservative: only allow [a-z0-9_-.], replace everything else.
    fn safe(s: &str) -> String {
        s.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.') {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect()
    }
    format!(
        "podci_{}_{}_{}",
        safe(project),
        safe(job),
        &env_id[..12.min(env_id.len())]
    )
}
