<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Quickstart: C++

This guide assumes a C++ project using CMake. The intent is to run C++ builds/tests in a pinned, reproducible container toolchain.

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
podci init --template cpp --dir "$tmpdir"
cp "$tmpdir/podci.toml" ./podci.toml
rm -rf "$tmpdir"
```

Edit `podci.toml` and set:

- `project = "your-repo-name"`

Commit the config.

## Typical job structure

A pragmatic baseline:

- configure (out-of-tree)
- build
- test

Example:

```toml
[profiles.dev]
container = "cpp-debian"

[jobs.default]
profile = "dev"
step_order = ["configure", "build", "test"]

[jobs.default.steps.configure]
run = ["cmake", "-S", ".", "-B", "build", "-DCMAKE_BUILD_TYPE=Release"]

[jobs.default.steps.build]
run = ["cmake", "--build", "build", "-j"]

[jobs.default.steps.test]
run = ["ctest", "--test-dir", "build", "--output-on-failure"]
```

## Run

```bash
podci run --job default
```

## Notes

- If your project needs additional system dependencies, create a custom container image and reference it in the profile. Pin the image by digest when you care about strict provenance.
- If you want artifact export (e.g., `build/` outputs), prefer writing artifacts into the repo workspace or add a dedicated host bind mount.
