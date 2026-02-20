<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Quickstart: drop-in for an existing repo

Use this when you already have build/test commands and just want podCI to run them in a consistent container environment.

## 1) Add `podci.toml`

Create `podci.toml` at the repo root.

Minimal example (Rust workspace):

```toml
version = 1
project = "myrepo"

[profiles.dev]
container = "rust-debian"

[jobs.default]
profile = "dev"
step_order = ["fmt", "test"]

[jobs.default.steps.fmt]
run = ["cargo", "fmt", "--all", "--", "--check"]

[jobs.default.steps.test]
run = ["cargo", "test", "--workspace"]
```

Notes:

- `container = "rust-debian"` selects a podCI template image.
- Any string containing `/` or `:` is treated as an explicit image reference.

## 2) Run it

```bash
podci run --job default
```

## 3) Commit and standardize

Treat `podci.toml` as part of your repo contract:

- stable job/step names reduce CI drift
- step ordering is explicit and deterministic

## 4) Add a CI runner (optional)

podCI does not ship CI workflows, but it’s designed to be straightforward:

- install Podman on the runner
- run `podci run --job <job>`
- treat non-zero exit as a merge/release blocker
