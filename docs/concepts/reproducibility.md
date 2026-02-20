<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Reproducibility posture

podCI is designed to make build/test execution **repeatable** and **auditable** by reducing environmental drift.

## What podCI provides

podCI improves repeatability by:

- running steps inside containerized toolchains
- deriving `env_id` and `namespace` deterministically from build-affecting inputs
- isolating caches per namespace so cross-project contamination is avoided
- producing a per-run manifest plus captured stdout/stderr for each step

podCI also makes “gates” easy to standardize because jobs/steps are explicit:

- locked builds/tests (`cargo ... --locked`)
- supply-chain checks (`cargo deny check`)

Those gates are **opt-in**: you add them as steps and enforce them in your workflow.

## What podCI does not guarantee

- bit-for-bit identical artifacts across different CPUs, kernels, or libcs
- identical timestamps unless your build system is configured for it
- identical results if you use unpinned external images or mutable base images

## Recommended workflow

- Keep `Cargo.lock` committed for Rust projects.
- Add a locked step for release-critical jobs:

  ```toml
  [jobs.release.steps.test]
  run = ["cargo", "test", "--workspace", "--locked"]
  ```

- Add a `cargo-deny` step and treat it as a merge/release gate in CI:

  ```toml
  [jobs.lint.steps.deny]
  run = ["cargo", "deny", "check"]
  ```

- Pin external images by digest (`@sha256:...`) when provenance matters.
