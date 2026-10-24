# Spec Delta

## MODIFIED Requirements

### Requirement: PinRef is a typed pin name constant

A `PinRef` newtype SHALL wrap a `&'static str` canonical physical-pin key. Components SHALL expose pin constants whose `PinRef` values identify exactly one physical pin, even when multiple pins share a display name. Generated constants SHALL preserve a readable name when the name is unique and add a physical-number suffix when it is not. `PinRef` SHALL be zero-cost at runtime.

#### Scenario: Component defines pin constants
- **WHEN** a component defines `pub const IOVDD: PinRef` for the only pin named `IOVDD`
- **THEN** `Rp2354a::IOVDD` references that physical pin

#### Scenario: Repeated names receive distinct constants
- **WHEN** a component has two pins named `GND` with pad numbers `3` and `29`
- **THEN** it exposes distinct constants such as `GND_3` and `GND_29`
- **AND** each constant resolves to its own physical pin

#### Scenario: PinRef is static and allocation-free
- **WHEN** a `PinRef` is constructed
- **THEN** it contains a `&'static str` with no heap allocation

### Requirement: PinHandle is the only accepted pin reference for connect()

`Board::connect()` SHALL accept exactly two `PinHandle` arguments. No `impl From<&str>` for `PinHandle` or `ComponentHandle` SHALL exist. String-based pin references SHALL NOT compile. A `PinHandle` SHALL carry the canonical physical identity rather than a display name.

#### Scenario: Typed connection compiles
- **WHEN** `board.connect(rpi.pin(Rp2354a::IOVDD), radio.pin(Mm8108::VBAT))` is called
- **THEN** the connection is accepted and a `NetHandle` is returned

#### Scenario: String connection does not compile
- **WHEN** `board.connect("rpi.IOVDD", "radio.VBAT")` is written
- **THEN** the code fails to compile because `&str` does not implement `Into<PinHandle>`

#### Scenario: Repeated-name pins are independently addressable
- **WHEN** a component exposes constants for two pins named `AVDD`
- **THEN** `connect()` records two distinct physical pins
- **AND** neither selection depends on declaration order

### Requirement: PinHandle validates pin existence at compile time

During compilation, every `PinHandle` used in a connection SHALL be validated against the component's canonical pin identities. If a handle does not match exactly one electrical pin, compilation SHALL produce an error naming the component, supplied identity, and any matching display name or physical number.

#### Scenario: Valid pin handle passes
- **WHEN** a `PinHandle` references the canonical identity for pad `7`
- **AND** component `U1` has exactly one electrical pin with that identity and pad number
- **THEN** compilation succeeds for that connection

#### Scenario: Invalid pin handle produces error
- **WHEN** a `PinHandle` references a canonical identity absent from component `U1`
- **THEN** compilation returns an error naming component `U1` and that identity

#### Scenario: Ambiguous identity is rejected
- **WHEN** a supplied identity matches more than one physical pin on a component
- **THEN** compilation reports an ambiguous-pin error rather than choosing the first match
