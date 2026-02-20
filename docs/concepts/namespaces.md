<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Namespaces and env_id

podCI isolates caches and artifacts by deriving a deterministic `namespace` and `env_id`.

## Why it exists

Without namespacing, caches become shared global state:
- builds “randomly” become fast/slow,
- stale outputs leak across unrelated repos,
- safe prune becomes impossible.

## Behavior

- `namespace` identifies the logical project space.
- `env_id` identifies an execution environment derived from stable inputs (profile + toolchain/image identity).

The derivation mechanism is intentionally strict. Do not hand-edit derived identifiers.

## Operational impact

- Caches are safe to prune by namespace without risking other projects.
- Build/test results are more predictable across machines.

## Inputs that affect env_id

`env_id` is derived from stable configuration inputs. Changing any of these will intentionally produce a new `env_id` (and therefore new cache volume names):

- `profile.container`
- profile environment (`profiles.<name>.env`)
- step ordering and step argv
- step `workdir`
- step environment (`jobs.<job>.steps.<step>.env`)
