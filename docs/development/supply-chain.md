<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Supply-chain gates (cargo-deny)

podCI ships a conservative **default policy** for dependency and licensing checks via [`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny).

Important distinction:

- podCI **includes** the policy file (`deny.toml`) and template images that contain the `cargo-deny` binary.
- podCI **does not automatically run** supply-chain checks unless you add a step (and run it locally or in your CI).

If you want a release gate, make it an explicit step in your job graph.

## What `cargo-deny` checks

`cargo-deny` inspects your full Cargo dependency graph (including transitive crates) and can enforce policies such as:

- allowed/denied licenses
- duplicate dependencies
- banned crates
- security advisories
- sources (e.g., disallowing git dependencies)

podCI’s default posture is: keep the policy readable, keep the output actionable, and prefer **explicit allow lists** over “it probably won’t matter”.

## Files

- `deny.toml` (repo root): the policy
- podCI Rust template images: include `cargo-deny` so you don’t need to install it globally

## Add a gate to your `podci.toml`

Add a `deny` step and include it in the job’s `step_order`.

Example (Rust project):

```toml
[jobs.lint]
profile = "dev"
step_order = ["fmt", "clippy", "deny"]

[jobs.lint.steps.fmt]
run = ["cargo", "fmt", "--all", "--", "--check"]

[jobs.lint.steps.clippy]
run = ["cargo", "clippy", "--workspace", "--all-targets", "--all-features", "--", "-D", "warnings"]

[jobs.lint.steps.deny]
run = ["cargo", "deny", "check"]
```

Then run it:

```bash
podci run --job lint
```

## CI integration

podCI does not ship CI workflows for you. The intended approach is:

1. keep `podci.toml` in-repo
2. run `podci run --job <job>` in your CI runner
3. treat failures as merge/release blockers

### GitHub Actions example (optional)

This is a minimal illustration. Adjust to your org’s standards.

```yaml
name: ci
on:
  push:
  pull_request:

jobs:
  podci:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Podman
        run: |
          sudo apt-get update
          sudo apt-get install -y podman
      - name: Build podCI
        run: cargo build --release -p podci-cli
      - name: Run lint gates
        run: ./target/release/podci run --job lint
```

## How to tune the policy

You will eventually hit “policy vs reality”. That’s normal.

### Licenses

The dual-license of podCI (MIT OR Apache-2.0) is about **podCI itself**. Your dependency graph is separate.

To allow or deny licenses for your project, edit the `licenses` section in `deny.toml`.

Typical changes:

- allow a license that is acceptable to your org but not in the default list
- deny a license family you want to keep out (e.g., copyleft in proprietary products)

### Advisories

Advisory checks can be noisy when an upstream fix isn’t available yet.

Options:

- accept the risk temporarily via an exception entry (with an expiry date in a comment)
- pin/patch to a fixed version
- replace the dependency

### Bans, duplicates, and sources

- **bans**: use for “we never want this crate” decisions
- **duplicates**: keep your graph sane (multiple versions of core crates can be a foot-gun)
- **sources**: disallow git dependencies if you need a stricter provenance posture

## “Turn it off so it just works”

If you remove the `deny` step from your job graph, podCI will not run `cargo-deny`.

If you keep the step but want it non-blocking, that’s a CI policy decision (e.g., run it in a separate job and don’t fail the pipeline). podCI intentionally keeps the decision *in your config*, not in hidden defaults.

## Operational notes

- `cargo-deny` reads `Cargo.lock` when present. Keep it committed for deterministic results.
- Prefer running `cargo deny check` in the same container/toolchain used for builds to avoid “works on my machine” drift.
- For machine-readable outcomes, consume the podCI **manifest** (Reference → Manifest). Logs are diagnostic.
