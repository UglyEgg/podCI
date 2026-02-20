// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Richard Majewski - Varanid Works

use anyhow::{anyhow, bail, Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub project: String,
    pub profiles: BTreeMap<String, Profile>,
    pub jobs: BTreeMap<String, Job>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub container: String,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Job {
    pub profile: String,
    pub step_order: Vec<String>,
    pub steps: BTreeMap<String, Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub run: Vec<String>,
    #[serde(default)]
    pub workdir: Option<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

impl Config {
    pub fn from_toml_str(s: &str) -> Result<Self> {
        let cfg: Config = toml::from_str(s).context("parse podci.toml")?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<()> {
        if self.version != 1 {
            bail!("unsupported config version {} (expected 1)", self.version);
        }
        if self.project.trim().is_empty() {
            bail!("project must be non-empty");
        }
        if self.profiles.is_empty() {
            bail!("profiles must be non-empty");
        }
        if self.jobs.is_empty() {
            bail!("jobs must be non-empty");
        }

        for (job_name, job) in &self.jobs {
            if !self.profiles.contains_key(&job.profile) {
                bail!(
                    "job '{job_name}' references missing profile '{}'",
                    job.profile
                );
            }
            validate_step_order(job_name, job)?;
        }

        Ok(())
    }

    pub fn job(&self, name: &str) -> Result<&Job> {
        self.jobs
            .get(name)
            .ok_or_else(|| anyhow!("unknown job '{name}'"))
    }

    pub fn profile(&self, name: &str) -> Result<&Profile> {
        self.profiles
            .get(name)
            .ok_or_else(|| anyhow!("unknown profile '{name}'"))
    }
}

fn validate_step_order(job_name: &str, job: &Job) -> Result<()> {
    if job.step_order.is_empty() {
        if !job.steps.is_empty() {
            bail!("job '{job_name}' has steps but empty step_order");
        }
        return Ok(());
    }

    let mut seen = BTreeSet::new();
    for s in &job.step_order {
        if !seen.insert(s.clone()) {
            bail!("job '{job_name}' step_order contains duplicate step '{s}'");
        }
        if !job.steps.contains_key(s) {
            bail!("job '{job_name}' step_order references missing step '{s}'");
        }
    }

    // No steps outside step_order (prevents hidden drift)
    let order: BTreeSet<_> = job.step_order.iter().cloned().collect();
    let extras: Vec<_> = job
        .steps
        .keys()
        .filter(|k| !order.contains(*k))
        .cloned()
        .collect();
    if !extras.is_empty() {
        bail!(
            "job '{job_name}' has steps not listed in step_order: {:?}",
            extras
        );
    }

    // Basic sanity: each step must have a non-empty argv
    for (step_name, step) in &job.steps {
        if step.run.is_empty() {
            bail!("job '{job_name}' step '{step_name}' has empty run argv");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_version() {
        let s = r#"
version = 2
project = "x"

[profiles.dev]
container = "rust-debian"

[jobs.default]
profile = "dev"
step_order = []
steps = {}
"#;
        let err = Config::from_toml_str(s).unwrap_err();
        assert!(err.to_string().contains("unsupported config version"));
    }

    #[test]
    fn enforces_step_order_refs() {
        let s = r#"
version = 1
project = "x"

[profiles.dev]
container = "rust-debian"

[jobs.default]
profile = "dev"
step_order = ["a"]

[jobs.default.steps.b]
run = ["echo", "hi"]
"#;
        let err = Config::from_toml_str(s).unwrap_err();
        assert!(err.to_string().contains("references missing step"));
    }

    #[test]
    fn accepts_minimal_valid() {
        let s = r#"
version = 1
project = "x"

[profiles.dev]
container = "rust-debian"

[jobs.default]
profile = "dev"
step_order = ["a"]

[jobs.default.steps.a]
run = ["echo", "hi"]
"#;
        let cfg = Config::from_toml_str(s).unwrap();
        assert_eq!(cfg.version, 1);
        assert!(cfg.jobs.contains_key("default"));
    }
}
