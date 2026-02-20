<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Quickstart: KDE mixed (C++ + Rust)

This guide targets repos that build KDE/Qt C++ components alongside Rust tooling.

## Prerequisites

- rootless Podman installed and working (`podman info` succeeds)
- `podci` installed

Verify templates are available:

```bash
podci templates list
```

## Existing repo: add `podci.toml`

`podci init` requires an **empty** destination directory. For an existing repo, generate into a temp directory and copy only the config:

```bash
tmpdir="$(mktemp -d)"
podci init --template kde-mixed --dir "$tmpdir"
cp "$tmpdir/podci.toml" ./podci.toml
rm -rf "$tmpdir"
```

Edit `podci.toml` and set:

- `project = "your-repo-name"`

Commit the config.

## Run

```bash
podci run --job default
```

## Notes

- This template assumes a Debian-based toolchain image (`kde-mixed-debian`). If you need additional packages, create a custom image and reference it in your profile.
- For digest pinning and reproducibility posture, see Concepts → Reproducibility.
