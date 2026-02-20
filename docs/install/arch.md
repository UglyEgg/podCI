<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Install: Arch Linux

podCI ships an Arch packaging recipe under `packaging/arch/PKGBUILD`.

## Prerequisites

- `podman` (runtime)
- `rust` / `cargo` (to build from source)

## Build with `makepkg`

From a source tarball or a tag checkout:

```bash
cd packaging/arch
makepkg -si
```

Notes:

- The PKGBUILD uses the upstream tag tarball as its source. Update `_github_owner`/`_github_repo` before publishing.
- The build runs `cargo run -p podci --bin podci-assets --features gen-assets -- gen` to generate:
  - a man page (`dist/podci.1`)
  - shell completions (`dist/completions/`)

## Templates

Templates are part of the base product. The package installs them to:

- `/usr/share/podci/templates`

Verify:

```bash
podci templates list
```

If you need an override:

- `podci --templates-dir <PATH> ...`
- `PODCI_TEMPLATES_DIR=<PATH> podci ...`

## Rootless Podman

podCI is intended to work with rootless Podman. If Podman is not configured for rootless use on your machine, fix that first (typical issues: missing subuid/subgid mappings).
