## MODIFIED Requirements

### Requirement: ERC detects floating inputs
The compiler SHALL flag `DigitalIO` and `AnalogIn` pins with no `SigSpec` and no net connection as warnings with code `ERC:FLOATING_INPUT`. Pins marked no-connect (via the `nc` pin attribute) or whose names start with `NC` or `NC_` SHALL be excluded.

#### Scenario: Floating GPIO produces warning
- **WHEN** a `DigitalIO` pin named "GPIO0" is not connected to any net
- **THEN** a `Diagnostic` with code `ERC:FLOATING_INPUT` and severity `Warning` is produced

#### Scenario: No-connect pin is not flagged
- **WHEN** a pin marked `nc` (or named "NC"/"NC_1") is not connected
- **THEN** no floating-input diagnostic is produced

### Requirement: ERC detects NC pins connected
The compiler SHALL flag pins marked no-connect that are connected to a net as errors with code `ERC:NC_CONNECTED`. A pin SHALL be treated as no-connect when its `nc` attribute is set or its name is `NC` or starts with `NC_` (name-prefix matching retained for hand-written parts). This error SHALL block compilation.

#### Scenario: Connected nc-flagged pin produces error
- **WHEN** a pin with `nc == true` is connected to a net
- **THEN** a `Diagnostic` with code `ERC:NC_CONNECTED` and severity `Error` is produced

#### Scenario: Connected NC-named pin produces error
- **WHEN** a pin named "NC_1" (without the `nc` attribute) is connected to a net
- **THEN** a `Diagnostic` with code `ERC:NC_CONNECTED` and severity `Error` is produced

### Requirement: Decoupling synthesis adds capacitors during compilation
The compiler SHALL synthesise decoupling capacitors for components that declare `Constraint::Decoupling { values, per_pin }`. For each `PowerIn` pin on the component (`per_pin == true`) or one representative pin per connected power net (`per_pin == false`), the compiler SHALL add one capacitor per value in `values`, wired between the pin's net and the board's ground net (creating a fallback `GND` net when none exists). Synthesised capacitors SHALL get deterministic refdes (C1, C2, ...), SHALL use the shared `CompiledComponent` constructor, and SHALL be reported via a `DECOUPLE:SUMMARY` info diagnostic.

#### Scenario: Per-pin decoupling adds caps to every power pin
- **WHEN** a component has two `PowerIn` pins and declares `Decoupling { values: [100.0.nf()], per_pin: true }`
- **THEN** two capacitors are synthesised, one per power pin, each 100nF

#### Scenario: Non-per-pin decoupling adds one cap set per power net
- **WHEN** a component has two `PowerIn` pins on two different power nets and declares `Decoupling { values: [100.0.nf()], per_pin: false }`
- **THEN** two capacitors are synthesised, one per net

#### Scenario: Multiple values produce multiple caps per pin
- **WHEN** a component has one `PowerIn` pin and declares `Decoupling { values: [100.0.nf(), 1.0.uf()], per_pin: true }`
- **THEN** two capacitors are synthesised for that pin: one 100nF and one 1µF

#### Scenario: No power pins produces warning
- **WHEN** a component declares `Decoupling` but has no `PowerIn` pins
- **THEN** a `Diagnostic` with code `DECOUPLE:NO_PWR_PIN` and severity `Warning` is produced
- **AND** no capacitors are synthesised

## ADDED Requirements

### Requirement: ERC queries connectivity through the shared board view
ERC rules SHALL obtain pin connectivity and net membership from the shared board view produced by compilation (see the `board-compile-pipeline` spec). They SHALL NOT build their own connection scans or refdes-to-component lookups, and SHALL NOT use sentinel indices for missing components.

#### Scenario: ERC performs no linear connection scans
- **WHEN** any ERC rule checks whether a pin is connected
- **THEN** the answer comes from the precomputed view, not from iterating `CompiledBoard.connections`
