#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Richard Majewski - Varanid Works
set -euo pipefail

trap 's=$?; echo "ERROR: line ${LINENO}: ${BASH_COMMAND}" >&2; exit $s' ERR

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found; install Rust toolchain first" >&2
  exit 2
fi

echo "==> generating Cargo.lock"
cargo generate-lockfile

if [[ ! -f Cargo.lock ]]; then
  echo "Cargo.lock was not generated" >&2
  exit 2
fi

echo "==> validating locked build/test"
cargo test --workspace --locked

echo "ok: Cargo.lock generated and --locked tests passed"
