## ADDED Requirements

### Requirement: Standard passive components in parts library
The `copperleaf-parts` crate SHALL provide `Capacitor`, `Resistor`, `Crystal`, and `Inductor` structs, each implementing `Block` with two pins parameterized by value.

#### Scenario: Capacitor construction
- **WHEN** `Capacitor::new("C1", 100.0.nf())` is called
- **THEN** the resulting block has 2 pins named "1" and "2"
- **AND** `block.pins().len() == 2`

#### Scenario: Resistor construction
- **WHEN** `Resistor::new("R1", 10.0.kohm())` is called
- **THEN** the resulting block has 2 pins and implements `Block`

#### Scenario: Crystal construction
- **WHEN** `Crystal::new("Y1", 25.0.mhz())` is called
- **THEN** the resulting block has 2 pins with `AnalogIn` role

### Requirement: Passive convenience constructors
`Capacitor` SHALL provide `::decoupling(refdes, value)` with PowerIn/Gnd pin roles. `Resistor` SHALL provide `::pullup(refdes, value, net)` and `::pulldown(refdes, value, net)` with appropriate pin roles and the supplied net stored on the part.

#### Scenario: Decoupling capacitor pin roles
- **WHEN** `Capacitor::decoupling("C1", 100.0.nf())` is called
- **THEN** pin "1" has role `PowerIn` and pin "2" has role `Gnd`

#### Scenario: Pullup resistor pin roles and stored net
- **WHEN** `Resistor::pullup("R1", 10.0.kohm(), "VCC")` is called
- **THEN** both pins have role `DigitalIO`
- **AND** the resistor's `net` field equals `"VCC"`
