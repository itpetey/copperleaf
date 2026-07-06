## Why

Consumer projects like halow-sta reimplement the same ERC rules, passive component definitions, and signal spec presets every time. The halow-sta project has a 22-line `erc_nc_pin_connected()` function that is a universal ERC rule ("NC pins must not be connected"), a 139-line `passive.rs` defining Capacitor/Resistor/Crystal, and 6 duplicate `SigSpec` helper functions that are identical to presets every project needs. All of this belongs in copperleaf, not in each consumer.

## What Changes

- Add `run_erc(&Design) -> Vec<Diagnostic>` to `copperleaf-analysis` that runs all built-in ERC rules in sequence. Initial rules: overvoltage (already exists, now orchestrated), NC-pin-connected, floating-input (unconnected input pins), multi-net-power-pin warning.
- Add standard passive components to `copperleaf-parts`: `Capacitor`, `Resistor`, `Crystal`, `Inductor`. Each is a 2-pin `Block` parameterized by value.
- Add `SigSpec` preset factory methods: `::spi(bw_mhz)`, `::spi_clk(bw_mhz)`, `::control()`, `::rf_50ohm()`.
- Add convenience constructors on passives: `Capacitor::decoupling(refdes, value)`, `Resistor::pullup(refdes, value, net)`, `Resistor::pulldown(refdes, value, net)`.

## Capabilities

### New Capabilities
- `erc-stdlib`: Built-in electrical rule check passes that run on any `&Design` without consumer-side implementation.
- `passive-parts`: Standard 2-pin passive component definitions (capacitor, resistor, crystal, inductor) in the parts library.
- `sigspec-presets`: Factory methods on `SigSpec` for common signal families (SPI, control, RF).

### Modified Capabilities

## Impact

- **`crates/analysis/src/lib.rs`**: Add `run_erc()`, `erc_nc_pin_connected()`, `erc_floating_inputs()`, `erc_multi_net_power()`. Re-export through facade.
- **`crates/parts/src/lib.rs`**: Add `Capacitor`, `Resistor`, `Crystal`, `Inductor` structs with `Block` impls.
- **`crates/ir/src/lib.rs`**: Add `SigSpec::spi()`, `::spi_clk()`, `::control()`, `::rf_50ohm()` factory methods.
- **`crates/copperleaf/src/lib.rs`**: Re-export new types/functions.
- Consumer projects (halow-sta) can delete their `passive.rs`, `erc_nc_pin_connected()`, and `sig_net()`/`spi_spec()` helpers.
