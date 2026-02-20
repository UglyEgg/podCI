<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Roadmap

This page lists **backlog** items for podCI. It is **not** a commitment, schedule, or compatibility guarantee.

## Guiding constraints

- podCI remains a **Rust-only** product (no runtime Python embedded in the tool).
- Workloads may use non-Rust toolchains **inside containers** via podCI template images.
- Namespaced caches, safe prune, and deterministic `namespace`/`env_id` derivation remain required behaviors.

## Backlog items

### Python + uv support

Goal: add first-class Python workflows without changing podCI’s Rust-only constraint.

Likely shape:

- Provide podCI-built template images that include:
  - Python toolchain
  - `uv`
  - common build deps for native wheels
- Provide example profiles/jobs for:
  - dependency resolution (locked)
  - lint/test
  - packaging

Non-goals:

- No embedded Python runtime inside podCI.

### Android development support

Goal: make Android build/test workflows reproducible and portable.

Likely shape:

- Provide podCI-built template images that include:
  - JDK toolchain
  - Android SDK / platform tools
  - optional NDK
- Provide example profiles/jobs for:
  - Gradle builds using the wrapper
  - unit tests
  - artifact export

Notes:

- Images will be large; docs must specify storage expectations and cache strategies.

### Configuration TUI

Goal: improve day-to-day ergonomics for editing and managing profiles without hand-editing files.

Likely shape:

- Terminal UI for:
  - listing and selecting profiles
  - editing profile fields
  - archiving/duplicating profiles
  - validating configuration

Non-goals:

- Not a full IDE.
- No background daemon requirement.


### Interactive shell

Goal: allow developers to open an interactive shell inside the same container environment used by `podci run` for fast debugging.

Likely shape:

- `podci shell --job <NAME> [--profile <NAME>] [--workdir <REL>]`
- Uses the same image resolution, mounts, caches, and enforced env (`CARGO_HOME`) as normal runs.
- Runs with `podman run --rm -it ...` and defaults to `bash` if present, otherwise `sh`.

Guardrails:

- Intended for diagnostics only; reproducibility comes from updating source/config, not mutating container state.
