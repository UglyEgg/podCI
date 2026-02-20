// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Richard Majewski - Varanid Works

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub name: String,
    pub created: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PrunePolicy {
    pub keep: usize,
    pub older_than_days: Option<i64>,
}

pub fn select_prune_candidates(
    mut resources: Vec<Resource>,
    policy: &PrunePolicy,
) -> Result<Vec<Resource>> {
    resources.sort_by(|a, b| b.created.cmp(&a.created)); // newest first

    let cutoff = policy
        .older_than_days
        .map(|d| Utc::now() - Duration::days(d));

    let mut candidates = Vec::new();
    for (idx, r) in resources.into_iter().enumerate() {
        if idx < policy.keep {
            continue;
        }
        if let Some(cut) = cutoff {
            if r.created >= cut {
                continue;
            }
        }
        candidates.push(r);
    }

    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_newest_n() {
        let now = Utc::now();
        let res = (0..5)
            .map(|i| Resource {
                name: format!("r{i}"),
                created: now - Duration::days(i),
            })
            .collect::<Vec<_>>();
        let pol = PrunePolicy {
            keep: 2,
            older_than_days: None,
        };
        let c = select_prune_candidates(res, &pol).unwrap();
        assert_eq!(c.len(), 3);
    }
}
