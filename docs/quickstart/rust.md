<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Quickstart: Rust

This guide gets a Rust repo running under podCI.

podCI ships:

- **init templates** (on disk; used by `podci init`)
- **template images** (Containerfiles embedded in the binary; built locally on first use)

## Prerequisites

- rootless Podman installed and working (`podman info` succeeds)
- `podci` installed (from a package or from source)

Verify templates are available:

```bash
podci templates list
```

## Existing repo: add `podci.toml`

`podci init` requires an **empty** destination directory. For an existing repo, generate into a temp directory and copy only the config:

```bash
tmpdir="$(mktemp -d)"
podci init --template rust-musl --dir "$tmpdir"
cp "$tmpdir/podci.toml" ./podci.toml
rm -rf "$tmpdir"
```

Edit `podci.toml` and set:

- `project = "your-repo-name"`

Commit the config.

If you need a glibc/gnu toolchain (e.g. for system integration constraints), replace `rust-musl` with `rust-glibc` in the init command above.


## Run a job

```bash
podci run --job default
```

On first use, podCI will build a local template image (e.g. `rust-debian`) from embedded Containerfiles.

## Typical additions

### Add a supply-chain gate

```toml
[jobs.default]
step_order = ["fmt", "clippy", "deny", "test"]

[jobs.default.steps.deny]
run = ["cargo", "deny", "check"]
```

### Locked tests for release posture

```toml
[jobs.release]
profile = "dev"
step_order = ["test"]

[jobs.release.steps.test]
run = ["cargo", "test", "--workspace", "--locked"]
```

## Outputs

Every run writes:

- a manifest (Operations → Manifests)
- per-step stdout/stderr under `~/.local/state/podci/runs/<run_id>/logs/`

If you need structured output for CI, consume the manifest.
