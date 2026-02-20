<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Install: RPM (Fedora/RHEL family)

podCI includes an RPM spec at `packaging/rpm/podci.spec`.

## Prerequisites

- `podman` (runtime)
- `rpmbuild`
- Rust toolchain (`cargo`, `rustc`)

## Build

```bash
# Create an rpmbuild tree if you don't already have one
mkdir -p ~/rpmbuild/{SOURCES,SPECS}

# Copy the spec
cp packaging/rpm/podci.spec ~/rpmbuild/SPECS/

# Download or place the source tarball into SOURCES.
# The spec references the upstream tag tarball URL; update it before publishing.

rpmbuild -ba ~/rpmbuild/SPECS/podci.spec
```

The spec:

- generates a man page and completions via `cargo run -p podci --bin podci-assets --features gen-assets -- gen`
- installs the `podci` binary
- installs init templates to `/usr/share/podci/templates`

## Install

```bash
sudo dnf install ~/rpmbuild/RPMS/*/podci-*.rpm
```

## Templates

Templates are part of the base product. Verify:

```bash
podci templates list
```

Override search path if needed:

- `--templates-dir <PATH>`
- `PODCI_TEMPLATES_DIR=<PATH>`
