## ADDED Requirements

### Requirement: Wire method parses refdes.pin notation
`Design` SHALL provide `fn wire(&mut self, pin: &str, net: &str)` that parses a `"refdes.pin"` string and calls `connect()` internally. If the string does not contain a `.`, it SHALL panic with a clear message.

#### Scenario: Wire a pin to a net
- **WHEN** `d.wire("U1.VDD", "V3V3")` is called
- **THEN** `d.pins_on_net("V3V3")` contains `("U1", "VDD")`

#### Scenario: Wire with no dot panics
- **WHEN** `d.wire("invalid", "V3V3")` is called
- **THEN** a panic occurs with a message mentioning the malformed pin string

### Requirement: Connect net connects multiple pins
`Design` SHALL provide `fn connect_net(&mut self, net: &str, pins: &[&str])` that connects each pin in `pins` to `net` using `wire()` semantics.

#### Scenario: Connect two pins to one net
- **WHEN** `d.connect_net("SDIO_CLK", &["U1.SDIO_CLK", "U2.GPIO2"])` is called
- **THEN** `d.pins_on_net("SDIO_CLK")` returns both `("U1", "SDIO_CLK")` and `("U2", "GPIO2")`

### Requirement: Add cap convenience method
`Design` SHALL provide `fn add_cap(&mut self, refdes: &str, value: Qty<Farad>, net_pos: &str, net_neg: &str)` that constructs a `Capacitor`, adds it as a component, and wires both pins to the specified nets in one call.

#### Scenario: Add a decoupling capacitor
- **WHEN** `d.add_cap("C1", 100.0.nf(), "VDD", "GND")` is called
- **THEN** a component with refdes "C1" exists in the design
- **AND** `d.pins_on_net("VDD")` contains `("C1", "1")`
- **AND** `d.pins_on_net("GND")` contains `("C1", "2")`

### Requirement: Add res convenience method
`Design` SHALL provide `fn add_res(&mut self, refdes: &str, value: Qty<Ohm>, net_a: &str, net_b: &str)` that constructs a `Resistor`, adds it as a component, and wires both pins.

#### Scenario: Add a pullup resistor
- **WHEN** `d.add_res("R1", 10.0.kohm(), "VDD", "SDIO_CS")` is called
- **THEN** a component with refdes "R1" exists in the design
- **AND** both nets have the corresponding pin connected

### Requirement: Add component consumes ComponentInst
`Design::add_component()` SHALL take `ComponentInst<B>` by value (consuming it) rather than by reference.

#### Scenario: Add component by value
- **WHEN** `d.add_component(ComponentInst::new("U1", block))` is called
- **THEN** the component is added to the design with its pins and constraints

### Requirement: Block trait has no id method
The `Block` trait SHALL NOT include an `id()` method. Part identity is carried by the `refdes` on `ComponentInst` and `ComponentRecord`.

#### Scenario: Block impl without id
- **WHEN** a struct implements `Block` with only `pins()` and optionally `constraints()`
- **THEN** it compiles successfully without providing `id()`
