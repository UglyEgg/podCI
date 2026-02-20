# Changelog

All notable changes to this project will be documented in this file.

This project follows Semantic Versioning.

## 0.1.0

- Initial product skeleton (CLI contract, config validation, manifest/log scaffolding, packaging skeletons)
- On-disk init templates (packaging installs to /usr/share/podci/templates)
- Release gate script (fmt/clippy/--locked tests/cargo-deny)
- Release helper script (version bump + commit + signed tag)
