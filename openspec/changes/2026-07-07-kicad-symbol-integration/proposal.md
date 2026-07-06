## Why

When Copperleaf generates a `.kicad_sch`, every symbol is a generic rectangle with pins algorithmically spaced on the right edge. Connections are coordinate-based: wires end exactly where pin tips sit. If a user swaps a symbol in KiCad (e.g., from `copperleaf:U1` to `RP2040:RP2354a`), the new symbol's pins are at different coordinates, so every wire dangles and all nets disconnect. The user must manually rewire after every symbol change.

## What Changes

- Add `kicad_symbol: Option<String>` to `ComponentRecord` (IR) so a component can carry a KiCad library reference (e.g., `"RP2040:RP2354a"`).
- Add `pos: Option<(f64, f64)>` and `rotation: Option<f64>` to `Pin` (IR) so each pin can carry its position within the symbol body (in mm). When `None`, the schematic generator falls back to the current algorithmic spacing.
- Create a new `copperleaf-derive` proc-macro crate with `#[derive(Component)]` that generates `impl Block` from a struct's fields, and an accompanying `#[component(symbol = "...")]` attribute that sets `kicad_symbol`.
- Add a `.kicad_sym` parser to the KiCad backend (`sym_parser` module) that reads a symbol library file, finds a symbol by name, extracts each pin's position/rotation/number, and returns a `SymbolDef` struct.
- Add a `resolve_symbols` pass (in the KiCad backend) that, when `kicad_symbol` is set and pin positions are `None`, opens the referenced `.kicad_sym` file, looks up the symbol, and populates each pin's `pos`/`rotation` by matching pin names.
- Update `schematic.rs` to use `kicad_symbol` for `lib_id` when present, and use per-pin `pos`/`rotation` (when available) for both lib-symbol pin placement and wire/label coordinates.
- Re-export `#[derive(Component)]` through `copperleaf-edsl` and the `copperleaf` facade.

## Capabilities

### New Capabilities
- `kicad-symbol-integration`: IR metadata for KiCad symbol references and pin positions, automated `.kicad_sym` parsing, schematic emission with real symbol geometry, and a `#[derive(Component)]` proc macro.