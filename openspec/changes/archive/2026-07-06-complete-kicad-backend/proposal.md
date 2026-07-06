## Why

The KiCad backend is a placeholder. `emit_netlist_text` produces a toy `(net "name")` list that omits components, pin-to-net connections, net classes, and is not valid KiCad format. Meanwhile the IR now carries rich, serializable data — components with pins, `Connection` records, `NetKind`/`NetClass`, and constraints — none of which the backend emits. This blocks the P0 milestone ("KiCad schematic + netlist") and P1 ("KiCad PCB export") and means `cl export` produces nothing KiCad can consume. Completing the backend turns the IR into actionable manufacturing/schematic output.

## What Changes

- Replace the toy `emit_netlist_text` with `emit_netlist`, a real KiCad S-expression netlist that lists components (`comp`/`ref`/`value`) and per-net pin-node connections (`net`/`node`/`ref`/`pin`).
- Add `emit_schematic` producing a minimal valid `.kicad_sch` with a generic box `lib_symbol`, auto-placed symbol instances carrying `Reference`/`Value` properties, and a net `label` placed at each connected pin.
- Add `emit_pcb` producing a `.kicad_pcb` with net classes derived from `NetClass` constraints (min width / clearance), a board outline, and footprint stubs (refdes + pad placeholders) for each component.
- All emitters are deterministic (snapshot-testable) and degrade gracefully on empty designs.
- Add CLI subcommands `export-sch` and `export-pcb`; update `export` to emit the real KiCad netlist.
- **BREAKING**: `emit_netlist_text` is removed in favor of `emit_netlist`. The output format changes from a toy list to S-expressions. The crate description is updated from "minimal placeholder."

## Capabilities

### New Capabilities

- `kicad-backend`: KiCad file emitters (netlist, schematic, PCB) that translate the design IR into valid KiCad S-expression output. Defines the library API (`emit_netlist`, `emit_schematic`, `emit_pcb`), output structure contracts, and determinism/empty-design guarantees.

### Modified Capabilities

- `cli-external-design`: Add `export-sch` and `export-pcb` subcommands that invoke the new backend emitters; update `export` to emit the real KiCad netlist instead of the toy placeholder.

## Impact

- **`crates/backends/kicad/src/lib.rs`**: Replace placeholder with three emitters and S-expression writer helpers. New module structure (e.g. `sexpr`, `netlist`, `schematic`, `pcb`).
- **`crates/backends/kicad/Cargo.toml`**: Update description; no new dependencies (S-expressions are hand-written).
- **`crates/cli/src/main.rs`**: Add `export-sch`/`export-pcb` dispatch and update `cmd_export` to call `emit_netlist`.
- **`crates/copperleaf/src/lib.rs`**: Already re-exports `backend_kicad`; no change needed.
- **Tests**: Snapshot-style tests per emitter in `crates/backends/kicad/`; CLI integration tests for new subcommands.
- **README.md / AGENTS.md**: Update the "toy netlist" language to reflect real KiCad export.
