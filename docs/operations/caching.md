<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Caching operations

podCI uses Podman volumes for caches. Volumes are namespaced to avoid cross-project contamination.

## What is cached

Per namespace, podCI maintains:

- Cargo registry cache (mounted at `/usr/local/cargo/registry`)
- Cargo git cache (mounted at `/usr/local/cargo/git`)
- Rust build outputs (`target/`, mounted at `/work/target`)

podCI sets `CARGO_HOME=/usr/local/cargo` inside the container so Cargo uses the mounted caches.

## Ownership and safety

podCI does not rely on volume name matching alone. Volumes are created with explicit labels, including:

- `podci.managed=true`
- `podci.namespace=<namespace>`
- `podci.env_id=<env_id>`
- `podci.volume_kind=<kind>`

Safe prune operates on these labels (Operations → Prune).

## Debugging caches

List podCI-managed volumes:

```bash
podman volume ls --filter label=podci.managed=true
```

Inspect a specific volume:

```bash
podman volume inspect <name>
```

If you change anything that affects `env_id` (profile container, step argv, workdir, env), podCI will compute a new namespace and you should expect cache misses.
