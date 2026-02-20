<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# podCI

podCI is a local-first, rootless Podman runner that executes repeatable build/test jobs inside pinned “template” images. It is designed for teams that want CI-like behavior on developer machines **without** turning their workstation into a snowflake.

## What podCI does

- Runs named jobs composed of ordered steps inside containers.
- Derives `namespace` and `env_id` deterministically so caches and artifacts are isolated and reproducible.
- Uses podCI-built template images (rustfmt/clippy/nextest/expand/binutils/audit/deny) so toolchains are consistent.
- Produces machine-readable manifests and JSONL logs for CI ingestion.
- Provides safe prune behaviors for namespaced caches.

## What podCI does not do

- It is not a build system (it runs your build tools).
- It is not a container runtime (it drives Podman).
- It does not promise bit-for-bit identical artifacts across differing CPUs/OSes.

## Execution model

```mermaid
flowchart LR
  A[Config: profiles/jobs/steps] --> B[Derive namespace + env_id]
  B --> C[Select template image (pinned)]
  C --> D[Run step in container]
  D --> E[Write logs (jsonl) + manifest (json)]
  D --> F[Use namespaced caches]
  E --> G[CI upload / local inspection]
```

## Next steps

- Start with the **Quickstart** section.
- For configuration details, see **Configuration → Config reference**.
- For policy and gates, see **Development → Supply-chain gates**.
- For planned features, see **Roadmap**.