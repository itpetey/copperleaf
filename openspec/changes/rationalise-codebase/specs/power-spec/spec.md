## MODIFIED Requirements

### Requirement: Builder methods for power pin construction
The `PinBuilder` SHALL provide `pwr_fixed(v, i)` for fixed-voltage power inputs, `pwr(v_min, v_max, i)` for flexible power inputs, and a chainable `.nominal(v)` method to set `v_nom` on flexible pins. `pwr_fixed` SHALL set `v_nom = v_min = v_max = v`. `pwr` SHALL set `v_nom = None` and `decouple = true`. `PinBuilder` SHALL construct electrical identity and specification only; pad geometry and symbol graphics SHALL be attached via `.pad(Pad)` and `.symbol(SymPin)` builder methods (see the `pad-model` spec), not via per-field physical setters on `PinBuilder`.

#### Scenario: pwr_fixed sets all fields
- **WHEN** `Pin::build("DVDD").pwr_fixed(1.1.volt(), 0.1.amp())` is called
- **THEN** the resulting pin has `Role::PowerIn`, `v_nom == Some(1.1.volt())`, `v_min == v_max == 1.1.volt()`, `i_max == 0.1.amp()`, and `decouple == true`

#### Scenario: pwr sets range with no nominal
- **WHEN** `Pin::build("IOVDD").pwr(1.8.volt(), 3.3.volt(), 0.1.amp())` is called
- **THEN** the resulting pin has `Role::PowerIn`, `v_nom == None`, `v_min == 1.8.volt()`, `v_max == 3.3.volt()`, `decouple == true`

#### Scenario: nominal chain sets v_nom on flexible pin
- **WHEN** `Pin::build("VBAT").pwr(3.0.volt(), 3.6.volt(), 0.3.amp()).nominal(3.3.volt())` is called
- **THEN** the resulting pin has `v_nom == Some(3.3.volt())`

#### Scenario: Pad geometry attaches as a unit
- **WHEN** `Pin::build("1").pad(Pad { number: "1", .. }).dio()` is called
- **THEN** the resulting pin's `pad()` returns the complete `Pad` and `PinBuilder` exposes no individual `pos`/`width`/`pad_type` setter methods
