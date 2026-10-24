# erc-and-synthesis Specification

## Purpose
Define the electrical-rule checks and decoupling synthesis the compiler performs, including the diagnostics that warn about or block a design.
## Requirements
### Requirement: ERC detects floating inputs
The compiler SHALL flag `DigitalIO` and `AnalogIn` pins with no `SigSpec` and no net connection as warnings with code `ERC:FLOATING_INPUT`. Pins whose names start with `NC` or `NC_` SHALL be excluded.

#### Scenario: Floating GPIO produces warning
- **WHEN** a `DigitalIO` pin named "GPIO0" is not connected to any net
- **THEN** a `Diagnostic` with code `ERC:FLOATING_INPUT` and severity `Warning` is produced

#### Scenario: NC pin is not flagged
- **WHEN** a pin named "NC" or "NC_1" is not connected
- **THEN** no floating-input diagnostic is produced

### Requirement: ERC detects floating power inputs
The compiler SHALL flag unconnected `PowerIn` pins as warnings with code `ERC:FLOATING_POWER_INPUT`.

#### Scenario: Unconnected power pin produces warning
- **WHEN** a `PowerIn` pin named "VDD" is not connected to any net
- **THEN** a `Diagnostic` with code `ERC:FLOATING_POWER_INPUT` and severity `Warning` is produced

### Requirement: ERC detects overvoltage
The compiler SHALL check each connected `PowerIn` pin against its net's inferred voltage. If the net voltage exceeds the pin's `v_max`, an error with code `ERC:OVERVOLT` SHALL be produced. This error SHALL block compilation.

#### Scenario: Overvoltage blocks compilation
- **WHEN** a pin with `v_max = 3.6.volt()` is connected to a net with inferred voltage 5.0V
- **THEN** a `Diagnostic` with code `ERC:OVERVOLT` and severity `Error` is produced
- **AND** `compile()` returns `CompileError`

#### Scenario: Voltage within range passes
- **WHEN** a pin with `v_max = 3.6.volt()` is connected to a net with inferred voltage 3.3V
- **THEN** no overvoltage diagnostic is produced

### Requirement: ERC detects NC pins connected
The compiler SHALL flag `NC` or `NC_` prefixed pins that are connected to a net as errors with code `ERC:NC_CONNECTED`. This error SHALL block compilation.

#### Scenario: Connected NC pin produces error
- **WHEN** a pin named "NC_1" is connected to a net
- **THEN** a `Diagnostic` with code `ERC:NC_CONNECTED` and severity `Error` is produced

### Requirement: Decoupling synthesis adds capacitors during compilation
The compiler SHALL synthesise decoupling capacitors for components declaring `Constraint::Decoupling { values, per_pin }`. It SHALL add one capacitor per value for every `PowerIn` pin when `per_pin` is true, or for the first power pin otherwise. Synthesised refdes SHALL be deterministic, SHALL start after the highest existing capacitor refdes, and SHALL skip occupied refdes. Every synthesised capacitor SHALL appear in `CompileSummary.caps_synthesised`.

#### Scenario: Per-pin decoupling adds caps to every power pin
- **WHEN** a component has two `PowerIn` pins and declares `Decoupling { values: [100.0.nf()], per_pin: true }`
- **THEN** two capacitors are synthesised, one per power pin, each 100nF

#### Scenario: Non-per-pin decoupling adds cap to first power pin only
- **WHEN** a component has two `PowerIn` pins and declares `Decoupling { values: [100.0.nf()], per_pin: false }`
- **THEN** one capacitor is synthesised for the first power pin

#### Scenario: Multiple values produce multiple caps per pin
- **WHEN** a component has one `PowerIn` pin and declares `Decoupling { values: [100.0.nf(), 1.0.uf()], per_pin: true }`
- **THEN** two capacitors are synthesised for that pin

#### Scenario: Existing capacitor refdes is never reused
- **WHEN** a board already contains `C1` and `C3` and synthesis adds a capacitor
- **THEN** the new capacitor is `C4`
- **AND** no existing component is renamed

#### Scenario: No power pins produces warning
- **WHEN** a component declares `Decoupling` but has no `PowerIn` pins
- **THEN** a `Diagnostic` with code `DECOUPLE:NO_PWR_PIN` and severity `Warning` is produced
- **AND** no capacitors are synthesised

### Requirement: Ground return is selected by electrical meaning
Decoupling synthesis SHALL select a ground return from connected nets whose kind is zero-volt power or whose pins have a ground role. A net named `GND` with a non-zero voltage SHALL NOT be selected solely because of its name. If no ground net exists, compilation SHALL create a deterministic synthetic ground net.

#### Scenario: Zero-volt ground net is selected by meaning
- **WHEN** a connected net has a ground-role pin and zero-volt power kind
- **THEN** synthesised decoupling capacitors connect their return pads to that net

#### Scenario: Non-zero net named GND is rejected
- **WHEN** a 5 V net is named `GND` and a zero-volt ground net also exists
- **THEN** synthesis chooses the zero-volt ground net
- **AND** reports a warning about the misleading `GND` name

#### Scenario: Missing ground creates one synthetic net
- **WHEN** a board requires decoupling but contains no electrical ground net
- **THEN** compilation creates one deterministic `GND` net
- **AND** all synthesised return connections use that net

### Requirement: Constraint covers analysis directives
The `Constraint` enum SHALL include the variants `Impedance`, `LengthMatch`, `ReturnPath`, `Decoupling`, `ResonanceIndex`, and `MaxJunction`. Physical directives such as net classes and creepage SHALL live on `LayoutConstraint`, not `Constraint`.

#### Scenario: Constraint enum is available
- **WHEN** a component returns `vec![Constraint::Impedance { target: 90.0.ohm(), tol_pct: 10.0 }]`
- **THEN** the constraint is stored and available for analysis during compilation

### Requirement: Signal and net kinds carry integrity metadata
`SigSpec` SHALL include `kind: SigKind`, `bandwidth: Option<Qty<Hertz>>`, `edge_rate: Option<Qty<Second>>`, and `target_impedance: Option<Qty<Ohm>>`. `SigKind` SHALL include `Generic`, `Usb2Hs`, `Usb3`, `Ddr3`, `PcieGen2`, `Clock`, and `AnalogLowNoise`. `NetKind` SHALL include `Power { v_nom, ripple }` and `Signal { spec }`.

#### Scenario: SigSpec presets are available
- **WHEN** `SigSpec::spi(50.0.mhz())` is called
- **THEN** a `SigSpec` with `kind == SigKind::Clock` or appropriate kind and `bandwidth == Some(50 MHz)` is returned

