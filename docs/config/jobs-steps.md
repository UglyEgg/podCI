<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Jobs and steps

A **job** is an ordered sequence of **steps**.

A **step** is a single container execution with a fixed argv (`run`) and optional `workdir` and `env`.

## Design guidance

- Keep steps single-purpose: `fmt`, `clippy`, `test`, `build`, `package`.
- Treat the config as part of the repo’s public contract: stable job/step names reduce CI drift.
- Prefer tools shipped in podCI template images (e.g. `cargo nextest`, `cargo deny`).

## Deterministic ordering (`step_order`)

podCI requires an explicit `step_order` for each job.

This has two operational benefits:

- prevents “hidden steps” not actually executed
- ensures `env_id` derivation is deterministic (step order is part of the fingerprint)

Rules:

- `step_order` must list every step exactly once.
- Steps not listed in `step_order` are rejected.

## Step working directory (`workdir`)

Use `workdir` when a step should run from a subdirectory.

Constraints:

- must be relative
- must not contain `..`
- must exist on the host

## Example

```toml
[jobs.default]
profile = "dev"
step_order = ["fmt", "test"]

[jobs.default.steps.fmt]
run = ["cargo", "fmt", "--all", "--", "--check"]

[jobs.default.steps.test]
workdir = "crates/cli"
run = ["cargo", "nextest", "run"]
```

