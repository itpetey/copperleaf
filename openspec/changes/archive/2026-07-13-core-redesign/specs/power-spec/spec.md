## ADDED Requirements

### Requirement: PowerSpec replaces PowerLimit with nominal voltage
A `PowerSpec` struct SHALL replace `PowerLimit`. It SHALL contain `v_min: Qty<Volt>`, `v_max: Qty<Volt>`, `v_nom: Option<Qty<Volt>>`, and `i_max: Qty<Amp>`. The `v_nom` field represents the pin's nominal operating voltage, if fixed. For flexible pins, `v_nom` SHALL be `None`.

#### Scenario: Fixed-voltage pin has v_nom set
- **WHEN** a pin is created with `Pin::build("DVDD").pwr_fixed(1.1.volt(), 0.1.amp())`
- **THEN** `power_spec.v_nom == Some(1.1.volt())` and `v_min == v_max == 1.1.volt()`

#### Scenario: Flexible pin has v_nom as None
- **WHEN** a pin is created with `Pin::build("IOVDD").pwr(1.8.volt(), 3.3.volt(), 0.1.amp())`
- **THEN** `power_spec.v_nom == None`, `v_min == 1.8.volt()`, `v_max == 3.3.volt()`

#### Scenario: Flexible pin with nominal override
- **WHEN** a pin is created with `Pin::build("VBAT").pwr(3.0.volt(), 3.6.volt(), 0.3.amp()).nominal(3.3.volt())`
- **THEN** `power_spec.v_nom == Some(3.3.volt())`, `v_min == 3.0.volt()`, `v_max == 3.6.volt()`

### Requirement: Builder methods for power pin construction
The `PinBuilder` SHALL provide `pwr_fixed(v, i)` for fixed-voltage power inputs, `pwr(v_min, v_max, i)` for flexible power inputs, and a chainable `.nominal(v)` method to set `v_nom` on flexible pins. `pwr_fixed` SHALL set `v_nom = v_min = v_max = v`. `pwr` SHALL set `v_nom = None` and `decouple = true`.

#### Scenario: pwr_fixed sets all fields
- **WHEN** `Pin::build("DVDD").pwr_fixed(1.1.volt(), 0.1.amp())` is called
- **THEN** the resulting pin has `Role::PowerIn`, `v_nom == Some(1.1.volt())`, `v_min == v_max == 1.1.volt()`, `i_max == 0.1.amp()`, and `decouple == true`

#### Scenario: pwr sets range with no nominal
- **WHEN** `Pin::build("IOVDD").pwr(1.8.volt(), 3.3.volt(), 0.1.amp())` is called
- **THEN** the resulting pin has `Role::PowerIn`, `v_nom == None`, `v_min == 1.8.volt()`, `v_max == 3.3.volt()`, `decouple == true`

#### Scenario: nominal chain sets v_nom on flexible pin
- **WHEN** `Pin::build("VBAT").pwr(3.0.volt(), 3.6.volt(), 0.3.amp()).nominal(3.3.volt())` is called
- **THEN** the resulting pin has `v_nom == Some(3.3.volt())`

### Requirement: Net voltage inference uses v_nom during compilation
During `Board::compile()`, the compiler SHALL infer net voltage from connected pins. If any pin on a net has `v_nom = Some(V)`, the net voltage SHALL be V. If multiple pins on the same net have disagreeing `v_nom` values, an ERC error SHALL be produced. If no pins have `v_nom`, the compiler SHALL require an explicit override via `NetHandle::set_voltage()`.

#### Scenario: Net inherits voltage from fixed pin
- **WHEN** a net connects a pin with `v_nom = Some(1.1.volt())` to another pin
- **THEN** the inferred net voltage is 1.1V

#### Scenario: Disagreeing v_nom produces ERC error
- **WHEN** a net connects a pin with `v_nom = Some(1.1.volt())` to a pin with `v_nom = Some(3.3.volt())`
- **THEN** `compile()` returns `CompileError` with a diagnostic about voltage mismatch

#### Scenario: No v_nom and no override produces error
- **WHEN** a net connects two flexible pins (both `v_nom = None`) with no explicit voltage override
- **THEN** `compile()` returns `CompileError` with a "no voltage source" diagnostic for that net
