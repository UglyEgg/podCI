#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Richard Majewski - Varanid Works
set -euo pipefail

trap 's=$?; echo "ERROR: line ${LINENO}: ${BASH_COMMAND}" >&2; exit $s' ERR

# Local helper: bump VERSION + Cargo.toml version manually first.

ver="${1:-}"
if [[ -z "$ver" ]]; then
  echo "usage: $0 X.Y.Z" >&2
  exit 2
fi

# Ensure we are operating from a clean tree; this script may still tag without committing
# if the repo is already at the requested version.
git diff --quiet || { echo "working tree dirty" >&2; exit 2; }

# Refuse to reuse an existing tag.
if git rev-parse -q --verify "refs/tags/v${ver}" >/dev/null; then
  echo "tag v${ver} already exists" >&2
  exit 2
fi

echo "$ver" > VERSION

# Keep crates/cli version in sync.
sed -i -E "s/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"/version = \"$ver\"/" crates/cli/Cargo.toml

# Ensure lockfile exists and locked tests pass.
./scripts/gen_lockfile.sh

# Release gate: fmt/clippy/locked tests/cargo-deny.
./scripts/release_gate.sh

git add VERSION crates/cli/Cargo.toml Cargo.lock

# If the repo was already at this version, the commit would fail (exit 1).
# Tagging the current HEAD is still valid; skip the commit in that case.
if git diff --cached --quiet; then
  echo "release metadata already committed for v$ver; tagging current HEAD" >&2
else
  git commit -m "release: v$ver"
fi

git tag -s "v$ver" -m "podCI v$ver"

echo "tagged v$ver; push with: git push --follow-tags"
