<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Architecture

This document describes podCI’s high-level architecture and the stability expectations of major surfaces.

## Components

- **CLI (`podci`)**: parses configuration, derives `namespace`/`env_id`, and orchestrates jobs.
- **Templates**: podCI-built container images that provide consistent toolchains.
- **Runner**: rootless Podman execution with safe, namespaced volumes/caches.
- **Outputs**:
  - **Manifest**: structured record of what ran and what it produced.
  - **Logs**: human-readable and JSONL modes for CI ingestion.

## Stable surfaces

- CLI flags and subcommands are versioned; see **Reference → CLI**.
- Manifest format is versioned; see **Reference → Manifest schema**.
- JSONL logs are *format-stable at a coarse level* (one JSON object per line), but individual keys are not a guaranteed schema unless explicitly documented; see **Reference → Logging**.

## Non-goals

- Scheduling across machines.
- Remote execution.
- Managing secrets beyond pass-through to container execution.
