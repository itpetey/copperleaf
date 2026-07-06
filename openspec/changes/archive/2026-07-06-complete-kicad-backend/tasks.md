## 1. S-expression foundation

- [x] 1.1 Create `crates/backends/kicad/src/sexpr.rs` with a `Sexpr` enum (`List(Vec<Sexpr>)`, `Atom(String)`, `Raw(String)`) and a `to_string()` that pretty-prints nested lists with indentation and trailing-newline handling
- [x] 1.2 Add constructors/helpers: `Sexpr::list(...)`, `Sexpr::atom(...)`, `Sexpr::raw(...)`, and a `kv(key, val)` convenience for `(key "val")` pairs
- [x] 1.3 Add a deterministic UUID function `pub fn deterministic_uuid(seed: &str) -> String` using an inline FNV-1a hash formatted into 8-4-4-4-12 hex layout
- [x] 1.4 Add unit tests: pretty-printing a nested list, and `deterministic_uuid` stability + distinctness for different seeds
- [x] 1.5 Declare the module in `crates/backends/kicad/src/lib.rs` (mod sexpr; pub use sexpr::* as needed) and confirm `cargo build -p copperleaf-backend-kicad`

## 2. Netlist emitter

- [x] 2.1 Create `crates/backends/kicad/src/netlist.rs` with `pub fn emit_netlist(design: &Design) -> String` that builds the `(export (version "E") (design ...) (components ...) (nets ...))` S-expression
- [x] 2.2 Implement component section: iterate `design.components`, emit `(comp (ref "<refdes>") (value "<prefix>"))` where prefix is leading-alpha of refdes (`?` if none)
- [x] 2.3 Implement net section: assign 1-based codes in `design.nets` order, then append (sorted) net names only present in `design.connections`; group connections by net into `(net (code) (name) ... (node ...))`
- [x] 2.4 In each `(node ...)`, look up the pin on the component record by name and emit `(pinfunction "<name>")` and `(pintype "<type>")` using the Role→pintype map; omit both if the pin isn't found
- [x] 2.5 Handle empty design: emit valid empty `(components)` and `(nets)` sections
- [x] 2.6 Add unit tests: netlist-with-connections assertions (comp refs, net names, node refs/pins), empty-design validity, and byte-for-byte determinism on a second call

## 3. PCB emitter

- [x] 3.1 Create `crates/backends/kicad/src/pcb.rs` with `pub fn emit_pcb(design: &Design) -> String` building `(kicad_pcb (version ...) (generator "copperleaf") ...)`
- [x] 3.2 Emit the `net` table: `(net <code> "<name>")` per net using the same code-assignment order as the netlist
- [x] 3.3 Emit net classes: always emit `Default`; group nets by distinct `NetClass { min_width, clearance }` into named classes, converting metres→mm via `as_base() * 1000.0`; list members under each class
- [x] 3.4 Emit a fixed rectangular board outline (100×80 mm) via `gr_line`/`gr_rect`
- [x] 3.5 Emit one `footprint` per component (`lib_id "copperleaf:Generic"`), auto-placed row-major on a fixed grid pitch, with one `thru_hole circle` `pad` per pin in a row; assign each pad's `(net <code> "<name>")` from connections (omit if unconnected)
- [x] 3.6 Handle empty design: valid `(kicad_pcb` with `Default` net class and no footprints
- [x] 3.7 Add unit tests: net class width/clearance values, Default always present, footprint pad net assignment, empty-design validity, determinism

## 4. Schematic emitter

- [x] 4.1 Create `crates/backends/kicad/src/schematic.rs` with `pub fn emit_schematic(design: &Design) -> String` building `(kicad_sch (version 20211123) (generator "copperleaf") (uuid ...) (paper "A4") ...)`
- [x] 4.2 Emit a single generic box `lib_symbol` (rectangle graphic) under `lib_symbols`
- [x] 4.3 Emit one `symbol` instance per component, auto-placed on a grid, carrying `(property "Reference" "<refdes>")` and `(property "Value" "<prefix>")` and a deterministic `(uuid ...)`
- [x] 4.4 Emit a `(label "<netname>")` for each connected pin (from `design.connections`), placed near the owning symbol instance
- [x] 4.5 Emit the `sheet_instances` node (path `/`, page `1`)
- [x] 4.6 Handle empty design: valid `(kicad_sch` with `sheet_instances` and no `symbol` instances
- [x] 4.7 Add unit tests: symbol instance + Reference property presence, net label presence, empty-design validity, UUID stability + distinctness, determinism

## 5. Crate facade and exports

- [x] 5.1 Update `crates/backends/kicad/src/lib.rs` to re-export `emit_netlist`, `emit_schematic`, `emit_pcb` and remove `emit_netlist_text`
- [x] 5.2 Update `crates/backends/kicad/Cargo.toml` description from "minimal placeholder" to reflect the real emitters
- [x] 5.3 Confirm `crates/copperleaf/src/lib.rs` still re-exports `backend_kicad` correctly (no change expected) and that `backend_kicad::emit_netlist` is reachable via the facade

## 6. CLI integration

- [x] 6.1 Update `cmd_export` in `crates/cli/src/main.rs` to call `backend_kicad::emit_netlist` instead of the removed `emit_netlist_text`
- [x] 6.2 Add `export-sch` and `export-pcb` to the `match` dispatch, calling `emit_schematic`/`emit_pcb` and printing to stdout
- [x] 6.3 Update `usage()` to list `export-sch` and `export-pcb`
- [x] 6.4 Add CLI integration tests in `crates/cli/tests/cli.rs`: `cl export` output begins with `(export`; `cl export-sch` begins with `(kicad_sch`; `cl export-pcb` begins with `(kicad_pcb` and contains `(net_class "Default"`

## 7. Documentation

- [x] 7.1 Update `README.md` to replace "toy/placeholder netlist" language with real KiCad export (netlist/schematic/PCB) and list the new CLI subcommands
- [x] 7.2 Update `AGENTS.md` crate description for `crates/backends/kicad` from "minimal netlist/schematic emitter" to the three emitters

## 8. Verify and validate

- [x] 8.1 Run `cargo build` across the workspace and fix any compile errors
- [x] 8.2 Run `cargo test -p copperleaf-backend-kicad` and ensure all new unit tests pass
- [x] 8.3 Run `cargo test -p copperleaf-cli` and ensure CLI integration tests pass
- [x] 8.4 Run `cargo test` across the workspace to ensure no regressions in IR/analysis/parts
- [x] 8.5 Run `cargo clippy --all-targets -- -D warnings` and `cargo fmt --all`, fixing any issues
