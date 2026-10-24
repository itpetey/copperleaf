#!/usr/bin/env bash
# External KiCad acceptance: emit the representative W5500 project, load it
# with a pinned KiCad 10.x CLI, and compare ERC, DRC, and schematic-parity
# findings against the reviewed baseline.
#
# Findings are recorded, not hidden: the baseline is a reviewed snapshot of
# the current output and may legitimately contain known limitations. Any new,
# removed, or changed finding fails the run.
#
#   ./scripts/kicad-acceptance.sh
#   KICAD_ACCEPTANCE_BLESS=1 ./scripts/kicad-acceptance.sh   # after review
#   KICAD_CLI=/path/to/kicad-cli ./scripts/kicad-acceptance.sh

set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
kicad=${KICAD_CLI:-kicad-cli}
baseline="$root/tests/acceptance/w5500"
work=${KICAD_ACCEPTANCE_WORKDIR:-"$root/target/kicad-acceptance"}

if ! command -v "$kicad" >/dev/null 2>&1; then
  echo "kicad-acceptance: '$kicad' not found; install KiCad 10.x or set KICAD_CLI" >&2
  exit 1
fi

version=$("$kicad" --version)
case "$version" in
  10.*) ;;
  *)
    echo "kicad-acceptance: unsupported KiCad version '$version' (expected 10.x)" >&2
    exit 1
    ;;
esac

mkdir -p "$work"

if [[ "${KICAD_ACCEPTANCE_SKIP_EMIT:-}" == "1" ]]; then
  if [[ ! -f "$work/w5500.kicad_pro" ]]; then
    echo "kicad-acceptance: KICAD_ACCEPTANCE_SKIP_EMIT=1 but no emitted project in $work" >&2
    exit 1
  fi
else
  rm -rf "$work"
  mkdir -p "$work"
  cargo run --quiet --locked -p copperleaf-backend-kicad --example w5500_acceptance -- "$work" \
    >"$work/emit.log" 2>&1
fi

run_kicad() {
  # Fontconfig chatter is common on developer machines and not a finding.
  "$kicad" "$@" 2>>"$work/kicad.log"
}

run_kicad sch erc --format json --severity-all \
  -o "$work/erc.raw.json" "$work/w5500.kicad_sch"
run_kicad pcb drc --format json --severity-all \
  -o "$work/drc.raw.json" "$work/w5500.kicad_pcb"
run_kicad pcb drc --format json --severity-all --schematic-parity \
  -o "$work/parity.raw.json" "$work/w5500.kicad_pcb"

for report in erc drc parity; do
  python3 "$root/scripts/normalise_kicad_report.py" \
    "$work/$report.raw.json" "$work/$report.json"
done

if [[ "${KICAD_ACCEPTANCE_BLESS:-}" == "1" ]]; then
  mkdir -p "$baseline"
  cp "$work/erc.json" "$work/drc.json" "$work/parity.json" "$baseline/"
  echo "kicad-acceptance: reviewed baseline updated in $baseline"
  exit 0
fi

status=0
for report in erc drc parity; do
  if ! diff -u "$baseline/$report.json" "$work/$report.json"; then
    echo "kicad-acceptance: $report findings differ from the reviewed baseline" >&2
    status=1
  fi
done

if [[ $status -ne 0 ]]; then
  echo "kicad-acceptance: review the diff, then re-run with KICAD_ACCEPTANCE_BLESS=1" >&2
  exit 1
fi

echo "kicad-acceptance: ERC, DRC, and schematic parity match the reviewed baseline (KiCad $version)"
