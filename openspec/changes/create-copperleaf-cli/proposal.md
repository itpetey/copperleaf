## Why

Parts are currently authored by hand: a developer finds a datasheet, reads a KiCad symbol library, and types a `.toml` definition pin by pin. The infrastructure to automate the tedious half already exists — `backend-kicad` ships a `sym_parser` that extracts pin names, numbers, positions, rotations, and lengths from `.kicad_sym` files, and `part-codegen` turns TOML into Rust via `build_component!` — but nothing wires the two together. This change adds the parts-creation CLI the `kicad-backend` spec explicitly anticipated: it creates and updates Copperleaf part TOML definitions from KiCad symbols, footprints, and (eventually) datasheets.

## What Changes

- **New crate `copperleaf-cli`** (`crates/cli`): the workspace's first binary target, a `clap`-based CLI exposing two commands — `new` and `update` — for creating and enriching part TOML definitions.
- **Modified `part-codegen`**: makes the manifest types (`Manifest`, `PinDef`, `ComponentMeta`) `pub` and adds `Serialize` derives so the CLI can both read and write TOML via the same schema. Adds fields `pos`, `rotation`, `length`, `nc` to `PinDef` and exposes a shared `validate()` function. `builder_expr` extended to emit `.pos(x, y).rotation(r).length(l)` when physical fields are present, so generated Rust round-trips physical data extracted by the CLI.
- **New `fp_parser` module in `backend-kicad`**: parses `.kicad_mod` / `.kicad_pcb` footprint files via the existing `sexpr` parser into `PadDef { number, pos, rotation, width, height, pad_type }`. Parallel to `sym_parser`; complements it — symbols give logical pin data, footprints give physical pad data, merged by pin number.
- **Part creation flows** (`new` command):
  - `new --symbol <FILE> --lib-id <ID>`: parse symbol, map pin types to kinds via a heuristic kind-map, emit a TOML file with pin names/kinds/positions. Power pins flagged as `# TODO` since voltages aren't in symbol files.
  - `new --footprint <FILE> --lib-id <ID>`: parse footprint pads, emit TOML keyed by pad number with `pos`/`rotation`/`length` populated and placeholder `kind = "dio"` / synthesised names.
  - `new --datasheet <FILE>`: hard-fails with `CLI:DATASHEET_STUB` — non-deterministic LLM-assisted datasheet parsing is a future capability, not stubbed as success.
- **Part update flow** (`update` command): loads an existing TOML, merges new source data by pin number:
  - `update --symbol`: fills names/kinds for pins missing them, appends new pins, preserves manually-authored voltages/bandwidths/notes.
  - `update --footprint`: sets/overwrites `pos`/`rotation`/`length` only, leaves logical fields untouched, warns on unmatched pads.
  - `update --datasheet`: hard-fails (same stub diagnostic).
- **Vendor crate scaffolding** (`new --crate <VENDOR>`): creates `parts/<vendor>/{Cargo.toml,lib.rs}` in the canonical shape and appends the member to the root `Cargo.toml`.
- **Kind-map**: built-in heuristic mapping KiCad `pin_type` → Copperleaf `kind` (`power_in`→`pwr`, `gnd`→`gnd`, `clock`→`clk`, etc.), overridable via a `--kind-map <FILE>` TOML file keyed by pin name or pin type.
- Add `clap` to `[workspace.dependencies]`.
- Use International English throughout (e.g. "analyse", "colour", "synthesise").

## Capabilities

### New Capabilities
- `parts-cli`: The `copperleaf-cli` binary — `new` and `update` commands for creating and enriching part TOML definitions from KiCad symbols and footprints, with vendor-crate scaffolding.
- `footprint-parser`: `fp_parser` module in `backend-kicad` that parses KiCad footprint files (`.kicad_mod`, `.pretty`) into `PadDef` structs with pad numbers, positions, rotations, and dimensions.

### Modified Capabilities
- `kicad-backend`: Adds the `fp_parser` module alongside `sym_parser`; both are available for CLI use but SHALL NOT be used during `Board::compile()` or `Backend::emit()`.

## Impact

- **New crate `crates/cli`** (`copperleaf-cli`): binary target `copperleaf`. Depends on `copperleaf-backend-kicad`, `copperleaf-part-codegen`, `clap`, `toml`, `thiserror`, `copperleaf` (for `Diagnostic`/`Severity`).
- **`crates/part-codegen`**: make `Manifest`/`PinDef`/`ComponentMeta` `pub` with `Serialize` derives; add physical/`nc` fields; extend `builder_expr`; expose `validate()`.
- **`crates/backend-kicad`**: add `fp_parser` module (`pub mod fp_parser`) re-exporting `PadDef`/`parse_footprint`.
- **Root `Cargo.toml`**: add `crates/cli` to `[workspace].members`; add `clap` to `[workspace.dependencies]`.
- **No changes to `crates/core`**: the CLI writes TOML files; `Board::compile()` and `Backend::emit()` remain pure and filesystem-free per `code-only-components`.
- **No changes to existing parts** (`parts/*`): existing `.toml` files remain valid; the schema additions are additive optional fields.