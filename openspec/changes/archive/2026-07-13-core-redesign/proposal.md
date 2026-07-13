## Why

The `main/` codebase validates the Copperleaf concept but its workflow is inverted: the user authors a design in Rust, serialises to JSON, then runs a CLI to import into a backend. The `main.rs` should be the source of truth — `cargo run` is the entire workflow, no JSON intermediary. Concurrently, components should be code-only (no filesystem dependencies), enabling distribution via `crates.io` with deterministic, environment-independent behaviour. This change restructures the core model to support that workflow while bringing across the proven substance from `main/`: nets, constraints, sigspecs, ERC, decoupling synthesis, and KiCad emission.

## What Changes

- **BREAKING**: Replace the `Design`-centric workflow with a `Board` → `compile()` → `CompiledBoard` pipeline. `Board` is the mutable, in-progress design; `CompiledBoard` is the frozen, verified, fully-resolved artifact.
- **BREAKING**: Remove `ComponentPin::From<&str>` string-based pin references. All pin references go through typed `PinRef` constants on component structs and `ComponentHandle`/`PinHandle` types returned by `board.add()` and `handle.pin()`.
- **BREAKING**: Rename `PowerLimit` to `PowerSpec` and add `v_nom: Option<Qty<Volt>>` for nominal voltage. Add builder methods `pwr_fixed()` (v_nom = v_min = v_max), `pwr()` (flexible, no nominal), and `.nominal()` override.
- **BREAKING**: Components carry embedded physical data (`symbol()`, `footprint()` returning `&'static str`) instead of filesystem paths. No `kicad_symbol_lib_path`, no `KICAD_SYMBOL_DIR`, no runtime file resolution.
- Add `Net` as an inferred entity: connections create pin-to-pin edges; the compiler infers nets as connected components with propagated voltage/signal properties. Explicit net annotation available via `NetHandle` returned from `connect()`.
- Add `PinRef` newtype and associated constants on components for type-safe pin referencing.
- Add `pos`, `rotation`, `length` fields to `Pin` — extracted during component generation (by the generator CLI), not at board compile time.
- Add `CompileReport` containing `CompiledBoard`, warnings (`Vec<Diagnostic>`), and `CompileSummary` (inferred nets, synthesised caps, counts).
- Add `CompileError` carrying `Vec<Diagnostic>` so the developer sees all errors, not just the first.
- Add `Constraint` enum, `SigSpec`, `NetKind`, `NetClass` brought across from `main/`.
- Add ERC rules and `synthesize_decoupling` brought across from `main/`, adapted to the new model.
- Add `Backend` trait with `emit(&self, path, &CompiledBoard) -> Result<(), BackendError>`.
- Bring across KiCad emitters (schematic, PCB, netlist, project) from `main/`, adapted to consume `CompiledBoard`.
- Bring across S-expression parser and `sym_parser` from `main/` for use by the generator CLI.
- Add deterministic ID generation (FNV-1a) replacing random `Uuid::new_v4()`.
- Add serialisation as a derived view (`CompiledBoard::to_json()`), not as a workflow intermediary.
- Fix `ComponentPin::From<&str>` iterator bug (now removed entirely).
- Use International English throughout code and documentation (e.g. "synthesised", "colour").

## Capabilities

### New Capabilities
- `board-compile-pipeline`: The `Board` → `compile()` → `CompileReport`/`CompileError` pipeline, `CompiledBoard` as immutable verified artifact, `Backend` trait for emission.
- `typed-pin-refs`: `PinRef` newtype, `ComponentHandle`/`PinHandle` types, associated pin constants on components, `board.connect()` accepting only typed handles.
- `power-spec`: `PowerSpec` with `v_nom`, builder methods `pwr_fixed`/`pwr`/`nominal`, voltage inference rules.
- `inferred-nets`: `Net` as inferred entity from connectivity, voltage/signal propagation, explicit override via `NetHandle`.
- `code-only-components`: `Component` trait with `symbol()`/`footprint()` returning `&'static str`, no filesystem dependencies, embedded physical data.
- `compile-report`: `CompileReport` with `CompiledBoard`, warnings, `CompileSummary` (nets, caps, counts); `CompileError` with all diagnostics.
- `erc-and-synthesis`: ERC rules and decoupling synthesis adapted to the new model, running during `compile()`.
- `kicad-backend`: KiCad schematic/PCB/netlist/project emitters consuming `CompiledBoard` via `Backend` trait.
- `deterministic-ids`: FNV-1a based deterministic ID generation replacing random UUIDs.

### Modified Capabilities
_(none — this is a fresh codebase with no existing specs)_

## Impact

- **`crates/model/`**: Major rewrite — `Board`, `CompiledBoard`, `CompileReport`, `CompileError`, `Net`, `Pin` with physical fields, `PowerSpec`, `PinRef`/`ComponentHandle`/`PinHandle`, `Component` trait with `symbol()`/`footprint()`/`constraints()`, `Constraint`/`SigSpec`/`NetKind`/`NetClass`, deterministic IDs.
- **`crates/parts/`**: Implement `Component` for `Capacitor`/`Resistor`/`Crystal`/`Inductor`; remove `Package` enum (KiCad owns footprints).
- **New crate `crates/analysis/`**: ERC rules and `synthesize_decoupling` from `main/`.
- **New crate `crates/backend-kicad/`**: KiCad emitters, S-expression parser, `sym_parser` from `main/`.
- **New crate `crates/derive/`** (future): `#[derive(Component)]` proc macro — not in this changeset but the trait shape supports it.
- **`Cargo.toml`**: Add `serde`/`serde_json` to workspace deps; add new crate members.
- **Downstream projects** (e.g. `halow-sta`): Update `main.rs` to use `board.compile()?` then `backend.emit(path, &compiled)?` workflow; update part definitions with `PinRef` constants and `Component` impl.
