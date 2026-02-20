<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Install: Debian/Ubuntu (.deb)

podCI includes Debian packaging under `packaging/debian/`.

## Prerequisites

- `podman` (runtime)
- a standard Debian packaging toolchain (e.g. `devscripts`, `debhelper`)
- Rust toolchain (`cargo`, `rustc`) available to the build

## Build

From a tag checkout or source tarball:

```bash
# from repo root
cp -a packaging/debian ./debian

dpkg-buildpackage -us -uc
```

Outputs are written in the parent directory (`../`).

The Debian rules:

- build with `--locked`
- generate a man page and completions via `cargo run -p podci --bin podci-assets --features gen-assets -- gen`
- install init templates to `/usr/share/podci/templates`

## Install

```bash
sudo dpkg -i ../podci_*.deb
```

If dependencies are missing:

```bash
sudo apt-get -f install
```

## Templates

Templates are part of the base product. Verify:

```bash
podci templates list
```

Override search path if needed:

- `--templates-dir <PATH>`
- `PODCI_TEMPLATES_DIR=<PATH>`
