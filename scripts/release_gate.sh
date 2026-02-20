#!/usr/bin/env bash
set -euo pipefail

# Release gate: deterministic formatting, linting, tests, and supply-chain checks.
#
# Uses cargo-nextest for normal tests and cargo-pretty-test for doctests (if installed)
# to avoid noisy libtest output.

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root_dir"

export CARGO_TERM_COLOR=always

step() { printf '\n==> %s\n' "$*"; }

step "cargo fmt (check)"
cargo fmt --all -- --check

step "cargo clippy (-D warnings)"
cargo clippy --workspace --all-targets --all-features -- -D warnings

step "doctests"
if command -v cargo-pretty-test >/dev/null 2>&1; then
  # cargo-pretty-test installs as a cargo subcommand: `cargo pretty-test ...`
  cargo pretty-test --doc --workspace --all-features --locked --color=always
else
  echo "note: cargo-pretty-test not found; falling back to cargo test --doc (quiet)" >&2
  cargo test --doc --workspace --all-features --locked -- -q
fi

step "tests (nextest)"
if ! command -v cargo-nextest >/dev/null 2>&1; then
  echo "error: cargo-nextest not found. Install with: cargo install cargo-nextest --locked" >&2
  exit 1
fi
cargo nextest run --workspace --all-features --locked

step "cargo deny check"
if ! command -v cargo-deny >/dev/null 2>&1; then
  echo "error: cargo-deny not found. Install with: cargo install cargo-deny --locked" >&2
  exit 1
fi
cargo deny check
