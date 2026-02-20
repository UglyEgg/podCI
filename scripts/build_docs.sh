#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Richard Majewski - Varanid Works
set -euo pipefail

# NOTE:
# Zensical is currently alpha. Its compatibility layer intentionally ignores
# MkDocs' `--strict` mode for now, and builds may succeed even when there are
# configuration/link issues. To keep CI from silently "green"-lighting broken
# docs, this script treats emitted "Error:" / Python tracebacks as failures.
#
# When Zensical adds strict validation, we should remove the grep heuristic and
# rely on exit codes.

tmp="$(mktemp)"
cleanup() { rm -f -- "$tmp"; }
trap cleanup EXIT

if [[ "${PODCI_SKIP_RUSTDOC:-}" != "1" ]]; then
  ./scripts/gen_rustdoc_into_docs.sh
fi

# Preserve the tool's exit code even when tee'ing output.
set +e
zensical build --clean 2>&1 | tee "$tmp"
status=${PIPESTATUS[0]}
set -e

if [[ $status -ne 0 ]]; then
  echo "docs build failed (zensical exit=$status)" >&2
  exit "$status"
fi

# Ensure rustdoc output is present in the built site.
#
# In Zensical, navigation entries that don't resolve to Markdown are treated as
# raw URLs. We link to rustdoc via a Markdown landing page (docs/rustdoc/index.md),
# and the rustdoc HTML itself must be present in the output tree.
if [[ -d "docs/rustdoc/api" ]]; then
  rm -rf -- site/rustdoc/api
  mkdir -p -- site/rustdoc
  cp -a -- docs/rustdoc/api site/rustdoc/api
fi

# Heuristic failure detection.
if grep -Eq '^(Error:|Traceback \(most recent call last\):)' "$tmp"; then
  echo "docs build emitted errors; failing" >&2
  exit 1
fi
