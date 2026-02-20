<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Releasing

This document describes the **maintainer** release flow for podCI.

podCI does not mandate a specific hosting platform (GitHub/GitLab/etc.). The release artifacts are:

- a signed git tag (`vX.Y.Z`)
- a source tarball (optional but recommended for distro packaging)

## Version sources of truth

- `VERSION` (repo root)
- `crates/cli/Cargo.toml` package version (`name = "podci"`)

These must stay in sync.

## Pre-release checks (recommended)

Run the release gate script:

```bash
./scripts/release_gate.sh
```

Or run the commands individually:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

# Stronger reproducibility posture (requires committed Cargo.lock)
cargo nextest run --workspace --all-features --locked
cargo test --workspace --all-features --doc --locked

# Supply-chain policy (optional gate)
cargo deny check
```

If you use podCI to validate itself:

```bash
./target/release/podci run --job default
```

## Tag a release

podCI provides a small helper script that bumps versions, runs tests, commits, and tags.

```bash
scripts/release.sh X.Y.Z
```

What it does:

- verifies the working tree is clean
- writes `VERSION`
- updates `crates/cli/Cargo.toml`
- ensures `Cargo.lock` exists (`./scripts/gen_lockfile.sh`)
- runs the release gate (`./scripts/release_gate.sh`: fmt/clippy/--locked tests/cargo-deny)
- commits and creates a signed tag `vX.Y.Z`

Notes:

- The tag is created with `git tag -s` (GPG signing). If your environment can’t sign tags, you must either configure signing or adjust the script.
- The script intentionally stays small; stronger gates (locked builds, cargo-deny) are expected to be run by maintainers/CI.

Push the tag:

```bash
git push --follow-tags
```

## Source tarball

For distro packagers (and for long-term reproducibility), generate a deterministic source tarball:

```bash
scripts/make_source_tarball.sh [out_dir]
```

This writes:

- `out_dir/podci-X.Y.Z.tar.gz` (default out_dir is `out/`)

Version is read from `VERSION`.

Use that tarball as the input for packaging builds.

## CI integration

podCI does not ship CI workflows, but the release flow above is designed to be easy to enforce in your own pipeline:

- run the pre-release checks
- run `scripts/release.sh` only from a protected branch or a maintainer machine
- publish the source tarball as a release asset
