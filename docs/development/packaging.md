<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Packaging

Packaging recipes live under `packaging/`:

- `packaging/arch/PKGBUILD`
- `packaging/debian/`
- `packaging/rpm/podci.spec`

podCI ships:

- a single Rust binary (`podci`)
- **init templates** as on-disk data under `templates/` (installed to `/usr/share/podci/templates`)

Separately, podCI builds its **template images** (e.g. `rust-debian`, `rust-alpine`) from Containerfiles embedded in the binary.

## Recommended source input

Prefer one of:

- a signed tag checkout (`vX.Y.Z`)
- the generated source tarball from `scripts/make_source_tarball.sh X.Y.Z`

The tarball is the most packager-friendly input because it is stable and avoids VCS metadata.

## Build command

For packaging builds, use the release profile and lockfile.

```bash
cargo build -p podci --release --locked
```

Install the resulting binary:

- `target/release/podci`

## Templates

Templates are part of the base product. Packages must install the repo-root `templates/` directory to:

- `/usr/share/podci/templates`

Users can verify:

```bash
podci templates list
```

(Overrides are supported via `--templates-dir` / `PODCI_TEMPLATES_DIR`.)

## Reproducibility posture

podCI itself provides reproducible **execution environments** (containers + namespaced caches). Packaging is a separate concern.

For distro packaging builds, the usual Rust best practices apply:

- keep `Cargo.lock` committed and build with `--locked`
- disable incremental compilation:

  ```bash
  export CARGO_INCREMENTAL=0
  ```

- if your build system supports it, set `SOURCE_DATE_EPOCH` to the release tag timestamp to reduce timestamp drift

podCI does not claim bit-for-bit identical binaries across different toolchains or libcs, but these steps minimize avoidable variance.

## Optional supply-chain checks

If your org/distro requires policy checks, you can run `cargo-deny` as part of packaging CI:

```bash
cargo deny check
```

The default policy lives in `deny.toml`. Adjust it to your policy requirements.
