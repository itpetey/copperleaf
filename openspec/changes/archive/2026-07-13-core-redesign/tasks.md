## 1. Model: Deterministic IDs and units

- [x] 1.1 Remove `uuid` from `Cargo.toml` workspace dependencies and from `crates/model/Cargo.toml`
- [x] 1.2 Implement `deterministic_id(seed: &str) -> String` using FNV-1a 64-bit hash formatted as 8-4-4-4-12 hex (bring across from `main/`'s `sexpr.rs`)
- [x] 1.3 Change `PinId` from `Uuid` to a `String` newtype; remove all `Uuid::new_v4()` calls
- [x] 1.4 Add `Hertz` unit marker and `hz()`/`khz()`/`mhz()` extension methods to `units.rs` (already present in `new/`, verify correctness)
- [x] 1.5 Add serde derives to `Diagnostic`, `Severity`, and `Role` (bring across from `main/`)
- [x] 1.6 Uncomment and verify `Qty<U>` `Serialize`/`Deserialize` implementations in `units.rs`
- [x] 1.7 Add `as_mhz()` to `Qty<Hertz>` (convert from Hz to MHz directly, unlike `main/`'s period-based `Qty<Second>::as_mhz()`)
- [x] 1.8 Add unit tests: deterministic ID stability, `Qty` serialisation round-trip, `Hertz` conversions

## 2. Model: PowerSpec and Pin refactor

- [x] 2.1 Rename `PowerLimit` to `PowerSpec` and add `v_nom: Option<Qty<Volt>>` field
- [x] 2.2 Add `PinBuilder::pwr_fixed(v, i)` — sets `Role::PowerIn`, `v_nom = v_min = v_max = v`, `decouple = true`
- [x] 2.3 Add `PinBuilder::pwr(v_min, v_max, i)` — sets `Role::PowerIn`, `v_nom = None`, `decouple = true`
- [x] 2.4 Add `PinBuilder::nominal(v)` — chainable method setting `v_nom = Some(v)` on flexible pins
- [x] 2.5 Add `pos: Option<(f64, f64)>`, `rotation: Option<f64>`, `length: Option<f64>` to `Pin` struct
- [x] 2.6 Add `PinBuilder::pos(x, y)`, `.rotation(deg)`, `.length(mm)` chainable methods
- [x] 2.7 Make `Pin` fields private with accessors (`id()`, `name()`, `role()`, `power_spec()`, `decouple()`, `pos()`, `rotation()`, `length()`)
- [x] 2.8 Add unit tests: `pwr_fixed` sets all fields, `pwr` leaves `v_nom` as `None`, `nominal` chain sets `v_nom`, physical fields round-trip

## 3. Model: Typed pin references

- [x] 3.1 Define `PinRef(pub &'static str)` newtype
- [x] 3.2 Define `ComponentHandle(usize)` struct with `pin(PinRef) -> PinHandle` method
- [x] 3.3 Define `PinHandle { component: usize, pin: &'static str }` struct
- [x] 3.4 Define `NetHandle` struct (stores representative edge/net identity for pre-compile annotation)
- [x] 3.5 Implement `NetHandle::set_voltage(Qty<Volt>)` and `NetHandle::set_name(&str)` — store overrides for compile-time resolution
- [x] 3.6 Remove `ComponentPin` struct and its `impl From<&str>` entirely
- [x] 3.7 Add unit tests: `PinRef` construction, `ComponentHandle::pin()` returns correct `PinHandle`

## 4. Model: Component trait and Board

- [x] 4.1 Update `Component` trait: keep `pins()`, `pin(id)`, `pin_name(name)`; add `constraints() -> Vec<Constraint>` (default empty), `symbol() -> Option<&'static str>` (default `None`), `footprint() -> Option<&'static str>` (default `None`)
- [x] 4.2 Define `Constraint` enum with variants: `Impedance`, `LengthMatch`, `ReturnPath`, `NetClass`, `Creepage`, `Decoupling`, `ResonanceIndex`, `MaxJunction` (bring across from `main/`)
- [x] 4.3 Define `SigSpec` struct with `kind: SigKind`, `bandwidth: Option<Qty<Hertz>>`, `edge_rate: Option<Qty<Second>>`, `target_impedance: Option<Qty<Ohm>>` and preset methods `spi()`, `spi_clk()`, `control()`, `rf_50ohm()` (bring across from `main/`, adapt `bandwidth` to `Qty<Hertz>`)
- [x] 4.4 Define `SigKind` enum: `Generic`, `Usb2Hs`, `Usb3`, `Ddr3`, `PcieGen2`, `Clock`, `AnalogLowNoise`
- [x] 4.5 Define `NetKind` enum: `Power { v_nom: Qty<Volt>, ripple: Option<Qty<Volt>> }`, `Signal { spec: SigSpec }`
- [x] 4.6 Define `NetClass` struct: `min_width: Option<Qty<Meter>>`, `clearance: Option<Qty<Meter>>`
- [x] 4.7 Define `Net` struct: `name`, `kind: NetKind`, `class: NetClass`, `constraints: Vec<Constraint>` with `power(name, v_nom)`, `ground()`, `ripple()` factory methods (bring across from `main/`)
- [x] 4.8 Update `Board` struct: store `components: Vec<ComponentEntry>`, `connections: Vec<Connection>`, `net_overrides: HashMap<NetId, NetOverride>`
- [x] 4.9 Define `Connection` struct: `component: usize`, `pin: &'static str`, `net: NetId` (serialisable)
- [x] 4.10 Implement `Board::add(name, component) -> ComponentHandle` — stores component, generates deterministic pin IDs from name + pin name
- [x] 4.11 Implement `Board::connect(from: PinHandle, to: PinHandle) -> Result<NetHandle, CompileError>` — validates both pins exist on their components, records connection
- [x] 4.12 Add unit tests: add component returns handle, connect two pins records connection, connect with invalid pin returns error

## 5. Model: Compile pipeline

- [x] 5.1 Define `CompiledBoard` struct: `components: Vec<CompiledComponent>`, `nets: Vec<Net>`, `connections: Vec<Connection>`, `constraints: Vec<Constraint>` (all serialisable)
- [x] 5.2 Define `CompiledComponent` struct: `refdes: String`, `pins: Vec<Pin>`, `constraints: Vec<Constraint>`, `symbol: Option<String>`, `footprint: Option<String>` (type-erased, serialisable)
- [x] 5.3 Define `CompileReport` struct: `board: CompiledBoard`, `warnings: Vec<Diagnostic>`, `summary: CompileSummary`
- [x] 5.4 Define `CompileSummary` struct: `nets: Vec<NetInfo>`, `caps_synthesised: Vec<SynthCap>`, `pin_count: usize`, `component_count: usize`
- [x] 5.5 Define `CompileError` struct: `errors: Vec<Diagnostic>` with `Display` impl listing all errors
- [x] 5.6 Define `NetInfo` struct: `name: String`, `kind: NetKind`, `pin_count: usize`
- [x] 5.7 Define `SynthCap` struct: `refdes: String`, `value: Qty<Farad>`, `net: String`, `source_component: String`, `source_pin: String`
- [x] 5.8 Implement `Board::compile(self) -> Result<CompileReport, CompileError>`: type-erase components into `CompiledComponent`, run net inference, run ERC, run synthesis, collect warnings, build summary
- [x] 5.9 Implement net inference: union-find on connections to identify connected components, propagate `v_nom` from pins, apply `NetHandle` overrides, detect voltage conflicts
- [x] 5.10 Implement deterministic refdes assignment for original and synthesised components
- [x] 5.11 Add unit tests: empty board compiles, board with errors returns all diagnostics, board with warnings returns report, summary counts are correct

## 6. Model: Backend trait

- [x] 6.1 Define `Backend` trait: `type Error; fn emit(&self, output_dir: &str, board: &CompiledBoard) -> Result<(), Self::Error>`
- [x] 6.2 Define `BackendError` enum for common backend errors (IoError, EmitError with message)
- [x] 6.3 Add unit test: trait can be implemented by a mock backend that collects output paths

## 7. Parts crate

- [x] 7.1 Remove `Package` enum entirely
- [x] 7.2 Implement `Component` for `Capacitor`: two pins, `constraints()` returns empty, `symbol()`/`footprint()` return `None` for now
- [x] 7.3 Implement `Component` for `Resistor`: two pins, same as above
- [x] 7.4 Implement `Component` for `Crystal`: two `AnalogIn` pins, same as above
- [x] 7.5 Implement `Component` for `Inductor`: two pins, same as above
- [x] 7.6 Add `PinRef` constants to each passive (e.g. `Capacitor::PIN1`, `Capacitor::PIN2`)
- [x] 7.7 Add `decoupling(value) -> Capacitor` constructor with `PowerIn` + `Gnd` pins (bring across from `main/`)
- [x] 7.8 Add `pullup(value, net) -> Resistor` and `pulldown(value, net) -> Resistor` constructors (bring across from `main/`)
- [x] 7.9 Add unit tests: each part implements `Component`, pins have correct roles, constants are accessible

## 8. Analysis crate

- [x] 8.1 Create `crates/analysis/` with `Cargo.toml` depending on `copperleaf-model`
- [x] 8.2 Implement `erc_floating_inputs(&CompiledBoard) -> Vec<Diagnostic>` — flags unconnected `DigitalIO`/`AnalogIn` pins, skips `NC*`
- [x] 8.3 Implement `erc_floating_power_inputs(&CompiledBoard) -> Vec<Diagnostic>` — flags unconnected `PowerIn` pins
- [x] 8.4 Implement `erc_overvoltage(&CompiledBoard) -> Vec<Diagnostic>` — checks net voltage against pin `v_max`
- [x] 8.5 Implement `erc_nc_pin_connected(&CompiledBoard) -> Vec<Diagnostic>` — flags connected `NC*` pins
- [x] 8.6 Implement `synthesize_decoupling(&CompiledBoard) -> (Vec<CompiledComponent>, Vec<Diagnostic>)` — reads `Constraint::Decoupling`, adds caps per power pin
- [x] 8.7 Add unit tests for each ERC rule and synthesis (bring across test cases from `main/`)

## 9. KiCad backend crate

- [x] 9.1 Create `crates/backend-kicad/` with `Cargo.toml` depending on `copperleaf-model`, `serde`, `serde_json`
- [x] 9.2 Bring across `sexpr.rs` from `main/` (S-expression builder, parser, `deterministic_uuid`)
- [x] 9.3 Bring across `sym_parser.rs` from `main/` (symbol library parser — for future generator CLI use)
- [x] 9.4 Implement `KiCad` struct implementing `Backend` trait
- [x] 9.5 Implement `emit_netlist(&CompiledBoard) -> String` (bring across from `main/`, adapt to new IR)
- [x] 9.6 Implement `emit_schematic(&CompiledBoard) -> String` (bring across from `main/`, adapt to `CompiledComponent` with embedded symbols)
- [x] 9.7 Implement `emit_pcb(&CompiledBoard) -> String` (bring across from `main/`, adapt to new IR)
- [x] 9.8 Implement `emit_project(name) -> String` (bring across from `main/`)
- [x] 9.9 Implement `KiCad::emit(output_dir, &CompiledBoard)` — creates output dir, writes all four files
- [x] 9.10 Add unit tests: empty board emits valid files, board with components emits correct `lib_id` and pin positions, netlist matches connections

## 10. Workspace and facade

- [x] 10.1 Add `serde` and `serde_json` to `[workspace.dependencies]` in root `Cargo.toml`
- [x] 10.2 Add `analysis` and `backend-kicad` to `[workspace].members`
- [x] 10.3 Add `crates/analysis/Cargo.toml` and `crates/backend-kicad/Cargo.toml` with correct dependencies
- [x] 10.4 Verify `cargo build --workspace` succeeds
- [x] 10.5 Verify `cargo test --workspace` succeeds
- [x] 10.6 Verify `cargo clippy --workspace --all-targets -- -D warnings` passes
- [x] 10.7 Verify `cargo fmt --all -- --check` passes

## 11. Integration test

- [x] 11.1 Create an integration test that builds a board with two components, connects pins, compiles, and emits to a temp directory via the KiCad backend
- [x] 11.2 Verify the emitted `.kicad_sch` file contains the correct `lib_id` values and pin coordinates
- [x] 11.3 Verify the emitted `.net` file contains the correct components and nets
- [x] 11.4 Verify compilation produces no errors for a valid board
- [x] 11.5 Verify compilation produces `CompileError` for an overvoltage condition
- [x] 11.6 Verify `CompileReport.summary` lists synthesised decoupling caps when applicable

## 12. Downstream validation

- [x] 12.1 Update `halow-sta/new/Cargo.toml` to depend on the updated `copperleaf-model` and `copperleaf-parts`
- [x] 12.2 Update `halow-sta/new/src/parts/rp2354a.rs` and `mm8108_mf15457.rs` to implement the new `Component` trait with `PinRef` constants
- [x] 12.3 Update `halow-sta/new/src/main.rs` to use the `board.add() -> handle`, `handle.pin()`, `board.compile()`, `backend.emit()` workflow
- [x] 12.4 Verify `cargo run` in `halow-sta/new` compiles and runs (even if `compile()` returns errors due to incomplete board)
- [x] 12.5 Verify `cargo run` produces a `CompileError` with meaningful diagnostics for the incomplete board
