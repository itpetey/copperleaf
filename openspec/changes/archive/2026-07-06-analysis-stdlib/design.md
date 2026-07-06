## Context

Three categories of boilerplate are duplicated across consumer projects:

1. **ERC rules**: halow-sta's `erc_nc_pin_connected()` is a universal rule. The overvoltage check (`erc_voltage_pin_to_net`) exists in copperleaf but consumers must orchestrate it themselves (iterate all components × pins × nets). Every consumer writes the same nested loop.

2. **Passive components**: halow-sta's `passive.rs` (139 lines) defines `Capacitor`, `Resistor`, `Crystal` — universal 2-pin components. `copperleaf-parts` only ships `Buck` and `Mcu`.

3. **SigSpec presets**: halow-sta defines `spi_spec()`, `spi_clk_spec()`, `spi1_spec()`, `spi1_clk_spec()`, `ctrl_spec()`, `rf_spec()` — many are identical (50 MHz, 50Ω, Generic). The same presets appear in `ht_hc01.rs` and `w5500.rs`.

## Goals / Non-Goals

**Goals:**
- `run_erc(&Design)` runs all built-in ERC rules and returns a flat `Vec<Diagnostic>`.
- Standard passives in `copperleaf-parts` eliminate per-project `passive.rs`.
- `SigSpec` presets eliminate per-project signal spec helpers.

**Non-Goals:**
- A plugin/registry system for custom ERC rules (future work).
- Parametric part search (e.g., "find a 100nF cap in 0402") — the passives are value-parameterized only.
- Full SI/PI analysis passes (separate future change).

## Decisions

### D1: ERC as a flat function, not a trait-based plugin system

**Decision:** `pub fn run_erc(design: &Design) -> Vec<Diagnostic>` calls each rule function internally and concatenates results. Rules are plain functions: `fn erc_nc_pin_connected(design: &Design) -> Vec<Diagnostic>`.

**Rationale:** Simpler than a trait-based `CheckPass` registry. Consumers can call individual rules or `run_erc()` for all. A trait registry can be layered on later without breaking the function API.

**Alternative considered:** `Vec<Box<dyn CheckPass>>` registry. Rejected for now — adds indirection for no benefit at this stage.

### D2: Passives as simple structs, not table-driven

**Decision:** Each passive is a struct with `id: String`, `value: Qty<U>`, and `pins: Vec<Pin>`. Pin roles are fixed (e.g., cap = PowerIn/Gnd, resistor = DigitalIO/DigitalIO).

**Rationale:** Matches the existing `Block` pattern. Consumers construct with `Capacitor::new("C1", 100.0.nf())`. No need for a TOML/table format yet.

**Convenience constructors:** `Capacitor::decoupling("C1", 100.0.nf())` sets pins to PowerIn/Gnd with appropriate limits. `Resistor::pullup("R1", 10.0.kohm())` and `Resistor::pulldown(...)` are similar. These don't change the struct — they're just `new()` with preset pin roles.

### D3: SigSpec presets as associated functions

**Decision:** Add `impl SigSpec { pub fn spi(bw_mhz: f64) -> Self, pub fn spi_clk(bw_mhz: f64) -> Self, pub fn control() -> Self, pub fn rf_50ohm() -> Self }`.

**Rationale:** Associated functions are the idiomatic Rust way to provide presets. They take frequency in MHz (not `Qty<Second>`) for ergonomics — the function calls `.mhz()` internally.

**Presets:**
- `spi(bw_mhz)`: Generic, bandwidth=bw_mhz, 50Ω
- `spi_clk(bw_mhz)`: Clock, bandwidth=bw_mhz, 50Ω
- `control()`: Generic, no bandwidth, no impedance
- `rf_50ohm()`: AnalogLowNoise, no bandwidth, 50Ω

## Risks / Trade-offs

- **[Pin role assumptions]** Capacitor pins are PowerIn/Gnd, but a coupling cap might be AnalogIn/AnalogOut. → The base `Capacitor::new()` uses generic DigitalIO pins; `Capacitor::decoupling()` uses PowerIn/Gnd. Consumers can use the base constructor for non-decoupling applications.
- **[ERC rule completeness]** The initial set (overvoltage, NC, floating input, multi-net) is small. → Extensible — new rules are just new functions added to `run_erc()`.
- **[SigSpec preset coverage]** Only SPI/control/RF presets are included initially. USB/DDR/PCIe presets can be added later since `SigKind` already has those variants.
