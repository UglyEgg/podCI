<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Config reference (`podci.toml`)

## Stability guarantees

- The configuration format is **versioned** and validated by podCI.
- **Non-breaking additions:** new optional fields may be added. Unknown fields are rejected unless documented otherwise.
- **Breaking changes:** removal/renaming of fields or semantic changes require a major version bump.
- Template/profile names are treated as user-facing identifiers; changes to their meaning should be considered breaking.

podCI is configured by a single TOML file (default: `podci.toml`). Unknown keys are rejected.

This reference matches the current `version = 1` config schema.

## Top-level

| Key | Type | Required | Notes |
|---|---:|---:|---|
| `version` | integer | yes | Must be `1` |
| `project` | string | yes | Used to derive namespaces; keep stable |
| `profiles` | table | yes | Named profiles |
| `jobs` | table | yes | Named jobs |

## Profiles (`[profiles.<name>]`)

A profile defines the container/toolchain used by jobs, plus environment defaults.

| Key | Type | Required | Notes |
|---|---:|---:|---|
| `container` | string | yes | Template name (e.g. `rust-debian`) or explicit image ref |
| `env` | table | no | Key/value env vars injected for all steps in the job |

### `container` resolution

- If `container` matches a known **podCI template name** (e.g. `rust-debian`), podCI will build/tag it locally.
- Otherwise, **external images must be explicit** to avoid ambiguity with template names.
  - An explicit image reference contains at least one of: `/`, `:`, `@` (e.g. `docker.io/library/ubuntu:24.04`).
  - Bare names like `ubuntu` are rejected.

See **Concepts → Execution model** for details.

## Jobs (`[jobs.<name>]`)

A job selects a profile and defines an ordered set of steps.

| Key | Type | Required | Notes |
|---|---:|---:|---|
| `profile` | string | yes | Must reference an existing profile |
| `step_order` | array<string> | yes | Ordered list of step names |
| `steps` | table | yes | Map of step definitions keyed by step name |

### `step_order` rules

podCI enforces the following:

- Every name in `step_order` must exist under `steps`.
- `step_order` must not contain duplicates.
- `steps` must not contain entries not listed in `step_order`.

This prevents “hidden steps” and keeps `env_id` derivation deterministic.

## Steps (`[jobs.<job>.steps.<step>]`)

A step is a single container execution.

| Key | Type | Required | Notes |
|---|---:|---:|---|
| `run` | array<string> | yes | argv to execute inside the container |
| `workdir` | string | no | Relative path inside repo (host must exist) |
| `env` | table | no | Step-scoped env overrides/additions |

### `workdir` constraints

`workdir` is resolved relative to the repo root.

- Must be relative (no leading `/`).
- Must not contain `..`.
- Must exist on the host at runtime.

## Minimal example

```toml
version = 1
project = "example"

[profiles.dev]
container = "rust-debian"

[jobs.default]
profile = "dev"
step_order = ["fmt", "clippy", "test"]

[jobs.default.steps.fmt]
run = ["cargo", "fmt", "--all", "--", "--check"]

[jobs.default.steps.clippy]
run = ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"]

[jobs.default.steps.test]
run = ["cargo", "nextest", "run", "--workspace"]
```

