# KiCad acceptance

External acceptance proves that an emitted project loads in KiCad and that
its electrical and design-rule findings match a reviewed snapshot. It is
deliberately **not** a claim that the representative board is fabrication
ready: the baseline records the findings the fixture currently produces.

## Supported version

Acceptance runs against **KiCad 10.0.7**. CI uses the pinned container image

```
kicad/kicad@sha256:c8f05703e865a1caa7266851cf4986dd01d5fb1148e4f63382b6a73466ba7e71
```

A different KiCad version is refused by the script rather than compared
against a baseline produced by another parser.

## Local setup

Required: Rust (locked workspace), Python 3, and `kicad-cli` 10.0.7 on
`PATH` (or set `KICAD_CLI=/path/to/kicad-cli`).

```sh
cargo run --locked -p copperleaf-backend-kicad \
  --example w5500_acceptance -- target/kicad-acceptance
./scripts/kicad-acceptance.sh
```

The script emits the representative W5500 fixture, runs ERC, DRC, and
schematic parity, normalises volatile fields, and diffs all three reports
against `tests/acceptance/w5500/`.

## The fixture

The fixture is a real W5500 board with:

- repeated display names (`AVDD`, `AGND`, and other same-named pads), each
  addressed by its physical pad number;
- a ground plane pour and a power pour (`V3V3`);
- a connector with a mechanical pad;
- a track and a via on a signal net;
- a back-side resistor (90° rotation) to exercise the back-side footprint
  transform.

## Baseline review

A passing run means *unchanged findings*, not zero findings. When output
changes:

1. Inspect the diff from `./scripts/kicad-acceptance.sh`; it names the file
   and every added, removed, or moved finding.
2. Decide whether each difference is intended. Fix the emitter for
   unintended differences; do not bless them.
3. Update the reviewed baseline only after that review:

   ```sh
   KICAD_ACCEPTANCE_BLESS=1 ./scripts/kicad-acceptance.sh
   ```

4. Commit the updated `tests/acceptance/w5500/*.json` with the code change.

Timestamps and KiCad's per-load item UUIDs are removed by
`scripts/normalise_kicad_report.py`. Findings, item order within a finding,
coordinates, and severities are retained; order-only changes between
platforms are canonicalised.

## Current reviewed findings

At the last review the baseline records:

| Report | Findings | Notes |
|--------|----------|-------|
| ERC | 62 violations | Mostly intentionally unconnected signal, reserved, and power pins on the fixture. |
| DRC | 13 violations, 14 unconnected items | No copper clearance errors; unconnected items reflect the incomplete fixture. |
| Schematic parity | 13 violations, 14 unconnected items, 61 parity issues | Missing-connection and pad-parity findings for the unwired pins. |

These counts are informational. The checked-in JSON is authoritative; the
script fails on any change.

## CI

`.github/workflows/ci.yml` runs the same acceptance in a `kicad-acceptance`
job. The job builds the fixture on the runner, then runs the script inside
the pinned KiCad container with `KICAD_ACCEPTANCE_SKIP_EMIT=1`. The image
runs as uid 1000, which cannot write to the runner-owned checkout, so the
emitted project is copied into the container's scratch space first:

```sh
docker run --rm -v "$PWD:/work:ro" -w /work \
  -e KICAD_ACCEPTANCE_SKIP_EMIT=1 \
  -e KICAD_ACCEPTANCE_WORKDIR=/tmp/kicad-acceptance \
  kicad/kicad@sha256:c8f05703e865a1caa7266851cf4986dd01d5fb1148e4f63382b6a73466ba7e71 \
  bash -lc 'cp -R target/kicad-acceptance /tmp/kicad-acceptance && ./scripts/kicad-acceptance.sh'
```

## Known limitations

- The fixture is intentionally incomplete: unconnected signal, reserved, and
  power pins produce reviewed ERC/DRC/parity findings.
- Copperleaf does not autoroute. Tracks and vias in the fixture are authored
  layout data; the layout crate only places components.
- External acceptance covers loading, ERC, DRC, and schematic parity. It does
  not prove fabrication readiness, impedance, or thermal performance.
- Acceptance requires the pinned KiCad version; report formats and checks may
  change between major versions.
- Only one representative board is covered until a real downstream design is
  available.
