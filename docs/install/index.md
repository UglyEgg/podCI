<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Install

podCI runs on top of rootless Podman. Choose an install path and verify with `podci version`.

## Prerequisites

- Podman (rootless)
- Sufficient storage for container images and caches

## Install options

- Arch Linux (PKGBUILD)
- Debian/Ubuntu (deb)
- Fedora/RHEL (rpm)

## Templates (part of the base install)

podCI ships **init templates** as on-disk data. Packaged installs must install them to:

- `/usr/share/podci/templates`

Verify after install:

```bash
podci templates list
```

If you need custom templates or want to override the search path, use:

- `--templates-dir <PATH>`
- `PODCI_TEMPLATES_DIR=<PATH>`
