<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Caching

podCI uses namespaced caches to speed up repeated runs while avoiding cross-project contamination.

## Current cache set

For each namespace, podCI maintains three Podman volumes:

- Cargo registry cache
- Cargo git cache
- Build outputs (`target/`)

These are mounted into step containers automatically.

podCI enforces `CARGO_HOME=/usr/local/cargo` inside the container to ensure the
cargo registry/git caches are stored in the mounted volumes.

## Guarantees

- Caches are isolated by `namespace`/`env_id`.
- Cache volume names are namespaced (and start with `podci_`), but podCI ownership is enforced via Podman labels (e.g. `podci.managed=true`, `podci.namespace=<...>`).
- `podci prune` only targets **labeled** podCI-managed volumes and supports a dry-run mode by default.

## Expected cache misses

A cache miss is expected when any build-affecting input changes, including:

- `profile.container`
- profile environment (`profiles.<name>.env`)
- step argv
- step `workdir`
- step environment

Those inputs are part of `env_id` derivation.

