# W5500 external acceptance baseline

These JSON files are the reviewed `kicad-cli` 10.0.7 output for the
representative W5500 project emitted by
`cargo run -p copperleaf-backend-kicad --example w5500_acceptance`.

They are a **reviewed baseline, not a claim that the board is clean**:

- `erc.json` records the current electrical-rule findings, including
  unconnected reserved/power pins on the W5500. Several are expected because
  the fixture intentionally leaves signal and power pins unwired.
- `drc.json` records design-rule findings and the unconnected-item report.
- `parity.json` records the PCB-versus-schematic parity findings.

Acceptance fails when any of these reports changes: a new finding, a removed
finding, a moved item, or a different KiCad version. Review any diff before
updating the baseline with:

```sh
KICAD_ACCEPTANCE_BLESS=1 ./scripts/kicad-acceptance.sh
```

Timestamps and KiCad's per-load item UUIDs are removed by
`scripts/normalise_kicad_report.py`; all findings, coordinates, severities,
and the emitted UUID path are retained.
