## ADDED Requirements

### Requirement: PinRef is a typed pin name constant
A `PinRef` newtype SHALL wrap a `&'static str` pin name. Components SHALL expose pin constants as `pub const PIN_NAME: PinRef = PinRef("PIN_NAME")`. `PinRef` SHALL be zero-cost at runtime (no allocation, no lookup).

#### Scenario: Component defines pin constants
- **WHEN** a component `Rp2354a` defines `pub const IOVDD: PinRef = PinRef("IOVDD")`
- **THEN** `Rp2354a::IOVDD` can be used to reference the IOVDD pin in a type-safe manner

#### Scenario: PinRef is static and allocation-free
- **WHEN** a `PinRef` is constructed
- **THEN** it contains a `&'static str` with no heap allocation

### Requirement: ComponentHandle is returned by board.add()
`Board::add(name, component)` SHALL return a `ComponentHandle`. `ComponentHandle::pin(PinRef)` SHALL return a `PinHandle`. The handle SHALL reference the component by its insertion index, not by name string.

#### Scenario: Add returns a usable handle
- **WHEN** `let rpi = board.add("rpi", Rp2354a::new())` is called
- **THEN** `rpi.pin(Rp2354a::IOVDD)` returns a `PinHandle` referencing the IOVDD pin on the "rpi" component

#### Scenario: Handle outlives the add call
- **WHEN** a `ComponentHandle` is stored in a local variable and used for multiple `connect()` calls
- **THEN** all connections correctly reference the same component

### Requirement: PinHandle is the only accepted pin reference for connect()
`Board::connect()` SHALL accept exactly two `PinHandle` arguments. No `impl From<&str>` for `PinHandle` or `ComponentHandle` SHALL exist. String-based pin references SHALL NOT compile.

#### Scenario: Typed connection compiles
- **WHEN** `board.connect(rpi.pin(Rp2354a::IOVDD), radio.pin(Mm8108::VBAT))` is called
- **THEN** the connection is accepted and a `NetHandle` is returned

#### Scenario: String connection does not compile
- **WHEN** `board.connect("rpi.IOVDD", "radio.VBAT")` is written
- **THEN** the code fails to compile because `&str` does not implement `Into<PinHandle>`

### Requirement: PinHandle validates pin existence at compile time
During `Board::compile()`, every `PinHandle` used in a connection SHALL be validated against the component's pin list. If a pin name in a `PinHandle` does not match any pin on the referenced component, a `CompileError` SHALL be produced with a diagnostic naming the component and the invalid pin.

#### Scenario: Valid pin handle passes
- **WHEN** a `PinHandle` references pin "IOVDD" and the component has a pin named "IOVDD"
- **THEN** compilation succeeds for that connection

#### Scenario: Invalid pin handle produces error
- **WHEN** a `PinHandle` references pin "VDD" but the component's pins are named "IOVDD" and "DVDD"
- **THEN** `compile()` returns `CompileError` with a diagnostic naming the component and the unmatched pin "VDD"
