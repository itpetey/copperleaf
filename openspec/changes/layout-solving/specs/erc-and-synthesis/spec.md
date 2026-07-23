## MODIFIED Requirements

### Requirement: Constraint and SigSpec types are carried across from main/
The `Constraint` enum SHALL include variants: `Impedance`, `LengthMatch`, `ReturnPath`, `Decoupling`, `ResonanceIndex`, `MaxJunction`. The physical directives `NetClass` and `Creepage` SHALL NOT be variants of `Constraint`; they live on the `LayoutConstraint` enum (see the `layout-constraints` capability). The `SigSpec` struct SHALL include `kind: SigKind`, `bandwidth: Option<Qty<Hertz>>`, `edge_rate: Option<Qty<Second>>`, `target_impedance: Option<Qty<Ohm>>`. `SigKind` SHALL include variants: `Generic`, `Usb2Hs`, `Usb3`, `Ddr3`, `PcieGen2`, `Clock`, `AnalogLowNoise`. `NetKind` SHALL include `Power { v_nom: Qty<Volt>, ripple: Option<Qty<Volt>> }` and `Signal { spec: SigSpec }`.

#### Scenario: Constraint enum is available
- **WHEN** a component returns `vec![Constraint::Impedance { target: 90.0.ohm(), tol_pct: 10.0 }]`
- **THEN** the constraint is stored and available for analysis during compilation

#### Scenario: Physical directives are not electrical constraints
- **WHEN** a component author attempts to use `Constraint::NetClass` or `Constraint::Creepage`
- **THEN** the code does not compile — those variants exist only on `LayoutConstraint`

#### Scenario: SigSpec presets are available
- **WHEN** `SigSpec::spi(50.0.mhz())` is called
- **THEN** a `SigSpec` with `kind == SigKind::Clock` or appropriate kind and `bandwidth == Some(50 MHz)` is returned
