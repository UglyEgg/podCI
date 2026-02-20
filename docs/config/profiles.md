<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Profiles

Profiles select the execution environment for jobs.

In `version = 1`, profiles are intentionally small:

- `container`: which container image (or podCI template) to run
- `env`: environment variables applied to every step in jobs using the profile

## When to use multiple profiles

Use multiple profiles when the same repo needs multiple toolchains, for example:

- `rust-debian` vs `rust-alpine`
- stable vs nightly (via different container images)
- different system dependency sets

## Container selection

A profile’s `container` can be one of:

1) **podCI template name** (recommended)
- Example: `rust-debian`
- podCI builds and tags a local image (`localhost/podci-…`) using the embedded template Containerfile.

2) **Explicit image reference**
- Any value containing `/` or `:` is treated as an image reference.
- Example: `ghcr.io/org/custom-toolchain:2026-02-01`

Operational note: explicit references increase supply-chain surface area. If you use them, pin to immutable tags/digests and gate changes via review.

## `env` guidance

- Keep profile env non-secret.
- Inject secrets via CI environment, not `podci.toml`.
- Prefer per-step env for step-specific knobs.

## Example

```toml
[profiles.dev]
container = "rust-debian"
env = { RUST_BACKTRACE = "1" }

[profiles.alpine]
container = "rust-alpine"
```

