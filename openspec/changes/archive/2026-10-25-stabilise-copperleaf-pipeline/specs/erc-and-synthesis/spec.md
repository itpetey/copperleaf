# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
