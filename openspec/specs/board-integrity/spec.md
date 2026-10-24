# board-integrity Specification

## Purpose
Defines the identities and invariants that make a compiled board internally coherent, so every physical pin can be connected independently and invalid boards are rejected before emitter output.

## Requirements

### Requirement: Electrical pins have unique physical identities
Every electrical pin SHALL have a component-local canonical identity derived from its non-empty pad number. If a pin has no pad number, compilation SHALL assign a deterministic insertion-ordinal fallback and emit a warning. Duplicate physical identities within one component SHALL be errors. Human-readable pin names SHALL NOT be required to be unique.

#### Scenario: Repeated names remain distinct physical pins
- **WHEN** a component has `AVDD` pins with pad numbers `4`, `8`, `11`, `15`, `17`, and `21`
- **THEN** each pin has a distinct canonical identity
- **AND** compilation accepts all six pins

#### Scenario: Missing pad number receives a stable fallback
- **WHEN** a hand-written component has an electrical pin without a pad number
- **THEN** compilation assigns a deterministic component-local fallback identity
- **AND** reports a warning naming the component and pin

#### Scenario: Duplicate pad number blocks compilation
- **WHEN** two electrical pins on one component declare pad number `7`
- **THEN** compilation returns an error identifying both pins and the duplicated pad number

### Requirement: Connections resolve to one physical pin
A board connection SHALL identify a physical pin by its canonical component-local identity, not by its display name. Net inference, ERC, the netlist, the schematic, and the PCB SHALL resolve that identity to exactly one pad. Ambiguous name-only lookup SHALL be rejected.

#### Scenario: Duplicate-name pins connect independently
- **WHEN** `U1` has two `GND` pins with pad numbers `3` and `29`
- **AND** only pad `29` is connected to another component
- **THEN** only pad `29` belongs to the resulting net
- **AND** pad `3` remains unconnected

#### Scenario: Ambiguous internal lookup is an error
- **WHEN** an internal operation supplies only a display name that matches more than one physical pin
- **THEN** it produces an ambiguous-pin error instead of selecting the first match

### Requirement: Compiled boards are validated before emission
The compiler SHALL run a central validation pass after synthesis and before returning a `CompiledBoard`. Validation SHALL report every discovered error, including duplicate refdes, duplicate physical pin identities, non-finite or negative dimensions, out-of-range layer references, malformed stackups, and connections or layout geometry referencing unknown components or nets. Any validation error SHALL block successful compilation.

#### Scenario: Multiple independent defects are reported together
- **WHEN** a compiled board has a duplicate refdes, a non-finite board dimension, and a connection to an unknown net
- **THEN** compilation returns all three error diagnostics in one result

#### Scenario: Validated board is safe to emit
- **WHEN** compilation succeeds
- **THEN** every component, pin, connection, net, layer, and stackup reference resolves within the compiled board

### Requirement: Copper layer references use one canonical vocabulary
Core physical layout and board models SHALL identify copper layers by zero-based stackup ordinal, independent of KiCad layer numbers. Backends SHALL translate canonical ordinals to their target format and SHALL reject ordinals outside the configured stackup.

#### Scenario: Two-layer back copper is canonical ordinal one
- **WHEN** a two-layer board places copper on its back layer
- **THEN** the core model represents that layer as ordinal `1`
- **AND** the KiCad backend emits `B.Cu`

#### Scenario: Out-of-range layer is rejected
- **WHEN** layout geometry references copper ordinal `7` on a four-layer board
- **THEN** validation reports an out-of-range layer error
- **AND** no backend output is written
