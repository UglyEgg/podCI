#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Richard Majewski - Varanid Works
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

version="$(cat VERSION)"
tag="v${version}"
prefix="podci-${version}/"

out_dir="${1:-out}"
mkdir -p "$out_dir"

out_tar="${out_dir}/podci-${version}.tar.gz"
out_sha="${out_tar}.sha256"

if git rev-parse -q --verify "refs/tags/${tag}" >/dev/null; then
  git archive --format=tar.gz --prefix="$prefix" "$tag" -o "$out_tar"
else
  # Fallback for local dev: archive current HEAD, but keep the release-style prefix.
  git archive --format=tar.gz --prefix="$prefix" HEAD -o "$out_tar"
fi

sha256sum "$out_tar" > "$out_sha"
echo "wrote $out_tar"
