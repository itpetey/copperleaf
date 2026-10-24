#!/usr/bin/env bash
# Aggregate verification: one command for every gate that protects the
# workspace. CI invokes this script; local runs use the same ordering.
#
#   ./scripts/verify.sh                 # full run including KiCad acceptance
#   VERIFY_SKIP_KICAD=1 ./scripts/verify.sh   # fast run without KiCad

set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

stage() {
  printf '\n==> %s\n' "$1"
}

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "verify: must run inside a git worktree" >&2
  exit 1
fi

status_before=$(git status --porcelain)

stage "locked build (all targets)"
cargo build --workspace --all-targets --locked

stage "format check"
cargo fmt --all -- --check

stage "Clippy (deny warnings)"
cargo clippy --workspace --all-targets --locked -- -D warnings

stage "rustdoc (deny warnings)"
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

stage "workspace tests"
cargo test --workspace --locked

stage "determinism (two-process emission)"
cargo test -p copperleaf-backend-kicad --test determinism --locked

stage "strict OpenSpec validation"
openspec validate --all --strict

stage "dependency policy (cargo-deny)"
cargo deny check

stage "unused dependencies (cargo-shear)"
cargo shear

stage "golden cleanliness"
status_after=$(git status --porcelain)
if [[ "$status_before" != "$status_after" ]]; then
  echo "verify: tests modified tracked or untracked files" >&2
  diff -u <(printf '%s\n' "$status_before") <(printf '%s\n' "$status_after") || true
  exit 1
fi

if [[ "${VERIFY_SKIP_KICAD:-}" == "1" ]]; then
  stage "KiCad acceptance (skipped)"
  echo "VERIFY_SKIP_KICAD=1: external KiCad acceptance not run in this invocation"
else
  stage "KiCad acceptance (pinned kicad-cli)"
  ./scripts/kicad-acceptance.sh
fi

printf '\nverify: all gates passed\n'
