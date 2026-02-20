<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Execution model

podCI runs a **job** as an ordered list of **steps**. Each step is executed in a container, with a deterministic namespace used to scope caches.

## Terms

- **Profile**: selects the container environment (`profile.container`) and provides default environment variables.
- **Job**: an ordered sequence of steps (`step_order`).
- **Step**: a container invocation with a fixed argv (`run`) plus optional `workdir` and `env`.
- **Template image**: a podCI-built image with required tools installed (when `profile.container` is a template name).

## Step lifecycle

1. Read `podci.toml`.
2. Resolve the job and profile.
3. Compute `env_id` from build-affecting inputs.
4. Derive `namespace` from `(project, job, env_id)`.
5. Resolve the container image:
   - if `profile.container` matches a known template name, build/tag it locally as needed
   - otherwise, require an explicit external image reference (contains `/`, `:`, or `@`)
     - bare names (e.g. `ubuntu`) are rejected to avoid ambiguity
6. Ensure namespaced cache volumes exist.
7. Run the step’s argv inside the container.
8. Write a manifest with per-step results.

## Determinism posture

podCI enforces a reproducible execution posture by:

- using containerized toolchains
- deriving `namespace`/`env_id` deterministically
- isolating caches per namespace
- making reproducibility/supply-chain gates explicit and repeatable (e.g. `cargo ... --locked`, `cargo deny check`) via job steps

This does not imply bit-for-bit identical artifacts across platforms.

