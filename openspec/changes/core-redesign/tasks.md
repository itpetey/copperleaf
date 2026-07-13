## 1. Model: Deterministic IDs and units

- [ ] 1.1 Remove `uuid` from `Cargo.toml` workspace dependencies and from `crates/model/Cargo.toml`
- [ ] 1.2 Implement `deterministic_id(seed: &str) -> String` using FNV-1a 64-bit hash formatted as 8-4-4-4-12 hex (bring across from `main/`'s `sexpr.rs`)
- [ ] 1.3 Change `PinId` from `Uuid` to a `String` newtype; remove all `Uuid::new_v4()` calls
- [ ] 1.4 Add `Hertz` unit marker and `hz()`/`khz()`/`mhz()` extension methods to `units.rs` (already present in `new/`, verify correctness)
- [ ] 1.5 Add serde derives to `Diagnostic`, `Severity`, and `Role` (bring across from `main/`)
- [ ] 1.6 Uncomment and verify `Qty<U>` `Serialize`/`Deserialize` implementations in `units.rs`
- [ ] 1.7 Add `as_mhz()` to `Qty<Hertz>` (convert from Hz to MHz directly, unlike `main/`'s period-based `Qty<Second>::as_mhz()`)
- [ ] 1.8 Add unit tests: deterministic ID stability, `Qty` serialisation round-trip, `Hertz` conversions

## 2. Model: PowerSpec and Pin refactor

- [ ] 2.1 Rename `PowerLimit` to `PowerSpec` and add `v_nom: Option<Qty<Volt>>` field
- [ ] 2.2 Add `PinBuilder::pwr_fixed(v, i)` — sets `Role::PowerIn`, `v_nom = v_min = v_max = v`, `decouple = true`
- [ ] 2.3 Add `PinBuilder::pwr(v_min, v_max, i)` — sets `Role::PowerIn`, `v_nom = None`, `decouple = true`
- [ ] 2.4 Add `PinBuilder::nominal(v)` — chainable method setting `v_nom = Some(v)` on flexible pins
- [ ] 2.5 Add `pos: Option<(f64, f64)>`, `rotation: Option<f64>`, `length: Option<f64>` to `Pin` struct
- [ ] 2.6 Add `PinBuilder::pos(x, y)`, `.rotation(deg)`, `.length(mm)` chainable methods
- [ ] 2.7 Make `Pin` fields private with accessors (`id()`, `name()`, `role()`, `power_spec()`, `decouple()`, `pos()`, `rotation()`, `length()`)
- [ ] 2.8 Add unit tests: `pwr_fixed` sets all fields, `pwr` leaves `v_nom` as `None`, `nominal` chain sets `v_nom`, physical fields round-trip

## 3. Model: Typed pin references

- [ ] 3.1 Define `PinRef(pub &'static str)` newtype
- [ ] 3.2 Define `ComponentHandle(usize)` struct with `pin(PinRef) -> PinHandle` method
- [ ] 3.3 Define `PinHandle { component: usize, pin: &'static str }` struct
- [ ] 3.4 Define `NetHandle` struct (stores representative edge/net identity for pre-compile annotation)
- [ ] 3.5 Implement `NetHandle::set_voltage(Qty<Volt>)` and `NetHandle::set_name(&str)` — store overrides for compile-time resolution
- [ ] 3.6 Remove `ComponentPin` struct and its `impl From<&str>` entirely
- [ ] 3.7 Add unit tests: `PinRef` construction, `ComponentHandle::pin()` returns correct `PinHandle`

## 4. Model: Component trait and Board

- [ ] 4.1 Update `Component` trait: keep `pins()`, `pin(id)`, `pin_name(name)`; add `constraints() -> Vec<Constraint>` (default empty), `symbol() -> Option<&'static str>` (default `None`), `footprint() -> Option<&'static str>` (default `None`)
- [ ] 4.2 Define `Constraint` enum with variants: `Impedance`, `LengthMatch`, `ReturnPath`, `NetClass`, `Creepage`, `Decoupling`, `ResonanceIndex`, `MaxJunction` (bring across from `main/`)
- [ ] 4.3 Define `SigSpec` struct with `kind: SigKind`, `bandwidth: Option<Qty<Hertz>>`, `edge_rate: Option<Qty<Second>>`, `target_impedance: Option<Qty<Ohm>>` and preset methods `spi()`, `spi_clk()`, `control()`, `rf_50ohm()` (bring across from `main/`, adapt `bandwidth` to `Qty<Hertz>`)
- [ ] 4.4 Define `SigKind` enum: `Generic`, `Usb2Hs`, `Usb3`, `Ddr3`, `PcieGen2`, `Clock`, `AnalogLowNoise`
- [ ] 4.5 Define `NetKind` enum: `Power { v_nom: Qty<Volt>, ripple: Option<Qty<Volt>> }`, `Signal { spec: SigSpec }`
- [ ] 4.6 Define `NetClass` struct: `min_width: Option<Qty<Meter>>`, `clearance: Option<Qty<Meter>>`
- [ ] 4.7 Define `Net` struct: `name`, `kind: NetKind`, `class: NetClass`, `constraints: Vec<Constraint>` with `power(name, v_nom)`, `ground()`, `ripple()` factory methods (bring across from `main/`)
- [ ] 4.8 Update `Board` struct: store `components: Vec<ComponentEntry>`, `connections: Vec<Connection>`, `net_overrides: HashMap<NetId, NetOverride>`
- [ ] 4.9 Define `Connection` struct: `component: usize`, `pin: &'static str`, `net: NetId` (serialisable)
- [ ] 4.10 Implement `Board::add(name, component) -> ComponentHandle` — stores component, generates deterministic pin IDs from name + pin name
- [ ] 4.11 Implement `Board::connect(from: PinHandle, to: PinHandle) -> Result<NetHandle, CompileError>` — validates both pins exist on their components, records connection
- [ ] 4.12 Add unit tests: add component returns handle, connect two pins records connection, connect with invalid pin returns error

## 5. Model: Compile pipeline

- [ ] 5.1 Define `CompiledBoard` struct: `components: Vec<CompiledComponent>`, `nets: Vec<Net>`, `connections: Vec<Connection>`, `constraints: Vec<Constraint>` (all serialisable)
- [ ] 5.2 Define `CompiledComponent` struct: `refdes: String`, `pins: Vec<Pin>`, `constraints: Vec<Constraint>`, `symbol: Option<String>`, `footprint: Option<String>` (type-erased, serialisable)
- [ ] 5.3 Define `CompileReport` struct: `board: CompiledBoard`, `warnings: Vec<Diagnostic>`, `summary: CompileSummary`
- [ ] 5.4 Define `CompileSummary` struct: `nets: Vec<NetInfo>`, `caps_synthesised: Vec<SynthCap>`, `pin_count: usize`, `component_count: usize`
- [ ] 5.5 Define `CompileError` struct: `errors: Vec<Diagnostic>` with `Display` impl listing all errors
- [ ] 5.6 Define `NetInfo` struct: `name: String`, `kind: NetKind`, `pin_count: usize`
- [ ] 5.7 Define `SynthCap` struct: `refdes: String`, `value: Qty<Farad>`, `net: String`, `source_component: String`, `source_pin: String`
- [ ] 5.8 Implement `Board::compile(self) -> Result<CompileReport, CompileError>`: type-erase components into `CompiledComponent`, run net inference, run ERC, run synthesis, collect warnings, build summary
- [ ] 5.9 Implement net inference: union-find on connections to identify connected components, propagate `v_nom` from pins, apply `NetHandle` overrides, detect voltage conflicts
- [ ] 5.10 Implement deterministic refdes assignment for original and synthesised components
- [ ] 5.11 Add unit tests: empty board compiles, board with errors returns all diagnostics, board with warnings returns report, summary counts are correct

## 6. Model: Backend trait

- [ ] 6.1 Define `Backend` trait: `type Error; fn emit(&self, output_dir: &str, board: &CompiledBoard) -> Result<(), Self::Error>`
- [ ] 6.2 Define `BackendError` enum for common backend errors (IoError, EmitError with message)
- [ ] 6.3 Add unit test: trait can be implemented by a mock backend that collects output paths

## 7. Parts crate

- [ ] 7.1 Remove `Package` enum entirely
- [ ] 7.2 Implement `Component` for `Capacitor`: two pins, `constraints()` returns empty, `symbol()`/`footprint()` return `None` for now
- [ ] 7.3 Implement `Component` for `Resistor`: two pins, same as above
- [ ] 7.4 Implement `Component` for `Crystal`: two `AnalogIn` pins, same as above
- [ ] 7.5 Implement `Component` for `Inductor`: two pins, same as above
- [ ] 7.6 Add `PinRef` constants to each passive (e.g. `Capacitor::PIN1`, `Capacitor::PIN2`)
- [ ] 7.7 Add `decoupling(value) -> Capacitor` constructor with `PowerIn` + `Gnd` pins (bring across from `main/`)
- [ ] 7.8 Add `pullup(value, net) -> Resistor` and `pulldown(value, net) -> Resistor` constructors (bring across from `main/`)
- [ ] 7.9 Add unit tests: each part implements `Component`, pins have correct roles, constants are accessible

## 8. Analysis crate

- [ ] 8.1 Create `crates/analysis/` with `Cargo.toml` depending on `copperleaf-model`
- [ ] 8.2 Implement `erc_floating_inputs(&CompiledBoard) -> Vec<Diagnostic>` — flags unconnected `DigitalIO`/`AnalogIn` pins, skips `NC*`
- [ ] 8.3 Implement `erc_floating_power_inputs(&CompiledBoard) -> Vec<Diagnostic>` — flags unconnected `PowerIn` pins
- [ ] 8.4 Implement `erc_overvoltage(&CompiledBoard) -> Vec<Diagnostic>` — checks net voltage against pin `v_max`
- [ ] 8.5 Implement `erc_nc_pin_connected(&CompiledBoard) -> Vec<Diagnostic>` — flags connected `NC*` pins
- [ ] 8.6 Implement `synthesize_decoupling(&CompiledBoard) -> (Vec<CompiledComponent>, Vec<Diagnostic>)` — reads `Constraint::Decoupling`, adds caps per power pin
- [ ] 8.7 Add unit tests for each ERC rule and synthesis (bring across test cases from `main/`)

## 9. KiCad backend crate

- [ ] 9.1 Create `crates/backend-kicad/` with `Cargo.toml` depending on `copperleaf-model`, `serde`, `serde_json`
- [ ] 9.2 Bring across `sexpr.rs` from `main/` (S-expression builder, parser, `deterministic_uuid`)
- [ ] 9.3 Bring across `sym_parser.rs` from `main/` (symbol library parser — for future generator CLI use)
- [ ] 9.4 Implement `KiCad` struct implementing `Backend` trait
- [ ] 9.5 Implement `emit_netlist(&CompiledBoard) -> String` (bring across from `main/`, adapt to new IR)
- [ ] 9.6 Implement `emit_schematic(&CompiledBoard) -> String` (bring across from `main/`, adapt to `CompiledComponent` with embedded symbols)
- [ ] 9.7 Implement `emit_pcb(&CompiledBoard) -> String` (bring across from `main/`, adapt to new IR)
- [ ] 9.8 Implement `emit_project(name) -> String` (bring across from `main/`)
- [ ] 9.9 Implement `KiCad::emit(output_dir, &CompiledBoard)` — creates output dir, writes all four files
- [ ] 9.10 Add unit tests: empty board emits valid files, board with components emits correct `lib_id` and pin positions, netlist matches connections

## 10. Workspace and facade

- [ ] 10.1 Add `serde` and `serde_json` to `[workspace.dependencies]` in root `Cargo.toml`
- [ ] 10.2 Add `analysis` and `backend-kicad` to `[workspace].members`
- [ ] 10.3 Add `crates/analysis/Cargo.toml` and `crates/backend-kicad/Cargo.toml` with correct dependencies
- [ ] 10.4 Verify `cargo build --workspace` succeeds
- [ ] 10.5 Verify `cargo test --workspace` succeeds
- [ ] 10.6 Verify `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] 10.7 Verify `cargo fmt --all -- --check` passes

## 11. Integration test

- [ ] 11.1 Create an integration test that builds a board with two components, connects pins, compiles, and emits to a temp directory via the KiCad backend
- [ ] 11.2 Verify the emitted `.kicad_sch` file contains the correct `lib_id` values and pin coordinates
- [ ] 11.3 Verify the emitted `.net` file contains the correct components and nets
- [ ] 11.4 Verify compilation produces no errors for a valid board
- [ ] 11.5 Verify compilation produces `CompileError` for an overvoltage condition
- [ ] 11.6 Verify `CompileReport.summary` lists synthesised decoupling caps when applicable

## 12. Downstream validation

- [ ] 12.1 Update `halow-sta/new/Cargo.toml` to depend on the updated `copperleaf-model` and `copperleaf-parts`
- [ ] 12.2 Update `halow-sta/new/src/parts/rp2354a.rs` and `mm8108_mf15457.rs` to implement the new `Component` trait with `PinRef` constants
- [ ] 12.3 Update `halow-sta/new/src/main.rs` to use the `board.add() -> handle`, `handle.pin()`, `board.compile()`, `backend.emit()` workflow
- [ ] 12.4 Verify `cargo run` in `halow-sta/new` compiles and runs (even if `compile()` returns errors due to incomplete board)
- [ ] 12.5 Verify `cargo run` produces a `CompileError` with meaningful diagnostics for the incomplete board
