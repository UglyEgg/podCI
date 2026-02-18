<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

<p align="center">
  <img src="docs/assets/Varanid_Works-podci-1600x400.webp" alt="Varanid Works – podCI" />
</p>

`podCI` ("Podman Continuous Integration") is a **local-first, Podman-backed CI runner** meant to be embedded into repository templates so teams can compile and test quickly with consistent behavior across machines.

It is designed for **multi-language repos** and mixed stacks (common in KDE/Qt and platform tooling): Rust, C++, and (soon) Python via `uv`.

## Features

- Containerized step execution via Podman
- Profile-driven workflows (`dev`, `ci`, `release`, etc.)
- Namespaced caches (fast reruns, no cross-project contamination)
- Machine-readable run manifests
- JSONL logging mode
- Safe pruning of podCI-owned caches
- Arch/.deb/.rpm packaging scaffolding + GitHub release automation

## Licensing

Dual-licensed under either:

- Apache License, Version 2.0 (`LICENSE-APACHE`)
- MIT License (`LICENSE-MIT`)

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion is licensed under the same terms.
