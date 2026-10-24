## MODIFIED Requirements

### Requirement: Constraint covers analysis directives
The `Constraint` enum SHALL include the variants `Impedance`, `LengthMatch`, `ReturnPath`, `Decoupling`, `ResonanceIndex`, and `MaxJunction`. The physical directives `NetClass` and `Creepage` SHALL NOT be variants of `Constraint`; they live on the `LayoutConstraint` enum (see the `layout-constraints` capability).

#### Scenario: Constraint enum is available
- **WHEN** a component returns `vec![Constraint::Impedance { target: 90.0.ohm(), tol_pct: 10.0 }]`
- **THEN** the constraint is stored and available for analysis during compilation

#### Scenario: Physical directives are not electrical constraints
- **WHEN** a component author attempts to use `Constraint::NetClass` or `Constraint::Creepage`
- **THEN** the code does not compile — those variants exist only on `LayoutConstraint`

### Requirement: Signal and net kinds carry integrity metadata
`SigSpec` SHALL include `kind: SigKind`, `bandwidth: Option<Qty<Hertz>>`, `edge_rate: Option<Qty<Second>>`, and `target_impedance: Option<Qty<Ohm>>`. `SigKind` SHALL include `Generic`, `Usb2Hs`, `Usb3`, `Ddr3`, `PcieGen2`, `Clock`, and `AnalogLowNoise`. `NetKind` SHALL include `Power { v_nom, ripple }` and `Signal { spec }`.

#### Scenario: SigSpec presets are available
- **WHEN** `SigSpec::spi(50.0.mhz())` is called
- **THEN** a `SigSpec` with `kind == SigKind::Clock` or appropriate kind and `bandwidth == Some(50 MHz)` is returned
