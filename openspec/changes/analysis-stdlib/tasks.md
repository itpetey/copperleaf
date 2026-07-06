## 1. Built-in ERC rules

- [ ] 1.1 Add `pub fn erc_nc_pin_connected(design: &Design) -> Vec<Diagnostic>` to `crates/analysis/src/lib.rs` — flags pins named "NC" or starting with "NC_" that are connected to any net
- [ ] 1.2 Add `pub fn erc_floating_inputs(design: &Design) -> Vec<Diagnostic>` — flags DigitalIO/AnalogIn pins with no signal spec and no net connection
- [ ] 1.3 Add `pub fn erc_overvoltage(design: &Design) -> Vec<Diagnostic>` — iterates all components × pins × nets and calls `erc_voltage_pin_to_net()` (moves the orchestration loop out of consumer code)
- [ ] 1.4 Add `pub fn run_erc(design: &Design) -> Vec<Diagnostic>` that calls all ERC rule functions and concatenates results
- [ ] 1.5 Add unit tests for each ERC rule function with both violation and clean scenarios
- [ ] 1.6 Re-export `run_erc` and individual rule functions through the `copperleaf` facade

## 2. Standard passive components

- [ ] 2.1 Add `Capacitor` struct to `crates/parts/src/lib.rs` with `id: String`, `value: Qty<Farad>`, `pins: Vec<Pin>`, `Block` impl, and `::new(refdes, value)` constructor with generic 2-pin layout
- [ ] 2.2 Add `Capacitor::decoupling(refdes, value)` constructor with PowerIn/Gnd pin roles and 50V-rated limits
- [ ] 2.3 Add `Resistor` struct with `::new(refdes, value)`, `::pullup(refdes, value)`, and `::pulldown(refdes, value)` constructors
- [ ] 2.4 Add `Crystal` struct with `::new(refdes, frequency)` constructor, both pins AnalogIn
- [ ] 2.5 Add `Inductor` struct with `::new(refdes, value)` constructor
- [ ] 2.6 Add unit tests verifying pin counts, roles, and value storage for each passive type
- [ ] 2.7 Re-export all passive types through the `copperleaf` facade (already re-exported via `pub use copperleaf_parts as parts`)

## 3. SigSpec presets

- [ ] 3.1 Add `impl SigSpec { pub fn spi(bw_mhz: f64) -> Self }` — Generic, bandwidth=bw_mhz.mhz(), 50Ω
- [ ] 3.2 Add `SigSpec::spi_clk(bw_mhz: f64)` — Clock variant
- [ ] 3.3 Add `SigSpec::control()` — Generic, no bandwidth, no impedance
- [ ] 3.4 Add `SigSpec::rf_50ohm()` — AnalogLowNoise, no bandwidth, 50Ω
- [ ] 3.5 Add unit tests verifying each preset produces the expected kind, bandwidth, and impedance values

## 4. Validate

- [ ] 4.1 Run `cargo test -p copperleaf-analysis` and ensure all tests pass
- [ ] 4.2 Run `cargo test -p copperleaf-parts` and ensure all tests pass
- [ ] 4.3 Run `cargo test -p copperleaf-ir` and ensure SigSpec preset tests pass
- [ ] 4.4 Run `cargo build` across the workspace
