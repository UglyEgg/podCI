<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# CLI

## Stability guarantees

- **Stable within a major series:** command names and their core semantics.
- **May change in a minor release:** help text, log wording, and additional flags/options.
- **Breaking changes:** removal/renaming of commands or flags occurs only with a major version bump.
- **Exit codes:** `0` indicates success; `1` indicates failure. Individual tool exit codes from step containers are recorded in the manifest output rather than being forwarded as podCI’s process exit code.

This page documents the `podci` command-line interface.

## Conventions

- On failure, `podci` exits non-zero.
- Human-readable errors are written to stderr.
- Logs can be human-readable or JSONL (see **Reference → Logging**).

## Global flags

| Flag | Default | Description |
|---|---|---|
| `--config <PATH>` | `podci.toml` | Path to the podCI configuration file |
| `--log-format <human|jsonl>` | `human` | Log output mode (`PODCI_LOG_FORMAT` env var is also supported) |
| `--about` | (none) | Print branding/about info and exit |

### Environment variables

| Variable | Purpose |
|---|---|
| `PODCI_LOG_FORMAT` | Default for `--log-format` |
| `RUST_LOG` | `tracing_subscriber` filter (e.g. `info`, `podci=debug`) |

## Commands

### `podci run`

Run a job (or a single step within a job).

**Flags**

| Flag | Default | Description |
|---|---|---|
| `--job <NAME>` | `default` | Job to run |
| `--step <NAME>` | (none) | Run only a single step |
| `--profile <NAME>` | (job default) | Override the job’s profile |
| `--dry-run` | false | Print what would run (no execution) |
| `--pull` | false | Pull base layers when (re)building template images |
| `--rebuild` | false | Force rebuild of template images (implies no-cache behavior) |

**Examples**

```bash
podci run
podci run --job default
podci run --job lint --step clippy
podci run --profile dev --job test
podci run --job test --dry-run
```

### `podci doctor`

Run a minimal environment check.

Current checks:

- verifies podCI XDG state/cache directories exist and are writable
- verifies `podman` is on `PATH`
- prints podman version and best-effort rootless status
- verifies podman can create/inspect/remove a **labeled** volume (required for safe prune)

```bash
podci doctor
```

### `podci init`

Write a starter template into a directory.

Templates are resolved from the first matching root:

1) `--templates-dir` / `PODCI_TEMPLATES_DIR`
2) `./.podci/templates`
3) `$XDG_CONFIG_HOME/podci/templates` (fallback: `~/.config/podci/templates`)
4) `/usr/share/podci/templates`

If a disk template is not found, `generic` is always available as an embedded fallback.

**Flags**

| Flag | Required | Default | Description |
|---|---:|---|---|
| `--template <NAME>` | no | `generic` | Template name |
| `--dir <PATH>` | no | `.` | Output directory (must be **empty**) |
| `--project <NAME>` | no | (derived) | Override project name used in generated files |

**Supported templates**

Run `podci templates list`.

**Examples**

```bash
podci init
podci init --template rust-musl --dir ./myproj
podci init --template cpp --dir /tmp/myproj --project myproj
```

Common templates shipped with podCI:

- `rust-musl`: Alpine/musl Rust workflow (recommended default)
- `rust-glibc`: Debian/glibc Rust workflow
- `cpp`: C++ (glibc)
- `kde-mixed`: KDE/Qt mixed toolchain


### `podci templates`

Manage templates.

**Subcommands**

- `podci templates list` — list available templates.
- `podci templates where <NAME>` — show the resolved origin (path or `embedded`).
- `podci templates export <NAME> <OUTPUT.tar.gz>` — write a deterministic `.tar.gz` bundle to a file.

**Examples**

```bash
podci templates list
podci templates where rust-musl
podci templates export rust-musl ./rust-musl-template.tar.gz

# Alternate template (glibc-based)
podci templates where rust-glibc
podci templates export rust-glibc ./rust-glibc-template.tar.gz
```
### `podci manifest show`

Print a manifest.

This command reads from XDG state (see **Reference → Manifest**).

**Flags**

| Flag | Default | Description |
|---|---|---|
| `--latest` | false | Show the latest manifest (`~/.local/state/podci/manifest.json`) |
| `--run <RUN_ID>` | (none) | Show manifest for a specific run ID |

**Examples**

```bash
podci manifest show --latest
podci manifest show --run 20260219T095112Z-ABC123defg
```

### `podci prune`

Prune podCI-owned caches/volumes using a **safe, namespaced** policy.

By default, this is a **dry-run**: it prints what would be deleted and exits.

**Flags**

| Flag | Default | Description |
|---|---|---|
| `--keep <N>` | `3` | Keep the newest N namespaces (best-effort by created time) |
| `--older-than-days <DAYS>` | (none) | Only prune namespaces older than this age |
| `--yes` | false | Apply deletions (without this, prune is dry-run only) |

**Examples**

```bash
podci prune
podci prune --keep 5
podci prune --older-than-days 30
podci prune --keep 3 --older-than-days 14 --yes
```

### `podci version`

Print the podCI version.

```bash
podci version
```

## Exit behavior

- Successful runs return exit code `0`.
- Failed runs return exit code `1`.
- For step-level exit codes and timing, consume the manifest (see **Reference → Manifest**).
