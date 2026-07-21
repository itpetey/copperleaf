## MODIFIED Requirements

### Requirement: Nets are inferred from connectivity
During compilation, the compiler SHALL identify nets as connected components of pins in the connection graph (including single-pin nets registered via `Board::net()`). Each connected component SHALL become a `Net` with an auto-generated name (deterministic, derived from the first connected pin's component and pin name) unless explicitly named. Nets SHALL have indexed identity: each resolved `Connection` SHALL reference its net by index into `CompiledBoard.nets`, not by name string. Net names remain display data on `Net` and SHALL be unique within a compiled board.

#### Scenario: Two connected pins form one net
- **WHEN** `board.connect(rpi.pin(Rp2354a::IOVDD), radio.pin(Mm8108::VBAT))` is called and compiled
- **THEN** the `CompiledBoard` contains one `Net`, and both resulting `Connection`s carry the same net index

#### Scenario: Unconnected pins do not form nets
- **WHEN** a board has two components with no connections between them
- **THEN** the `CompiledBoard` contains no nets

#### Scenario: Daisy-chain forms single net
- **WHEN** pin A is connected to pin B, and pin B is connected to pin C
- **THEN** the `CompiledBoard` contains one net containing all three pins

#### Scenario: Every connection's net index resolves
- **WHEN** any board is compiled
- **THEN** every `Connection`'s net index is a valid index into `CompiledBoard.nets` — no net may exist only as a name referenced by connections

### Requirement: Net resolution is a single pass with explicit precedence
Net name, voltage, and kind SHALL be resolved in one pass per net, applying precedence in exactly this order: (1) explicit override from `NetHandle` annotation; (2) consensus of connected power pins' `v_nom` (conflicting values produce a `NET:VOLTAGE_CONFLICT` error); (3) ground fallback (`Gnd`-role pins imply 0 V when no other source exists); (4) a power net with no resolvable voltage produces a `NET:NO_VOLTAGE_SOURCE` error. A net connected to any `PowerIn`/`PowerOut`/`Gnd` pin SHALL be `NetKind::Power`; otherwise `NetKind::Signal` carrying the first declared `SigSpec` if any. `Net::is_ground()` (nominal voltage ≈ 0) SHALL be the single ground test used by compilation and synthesis.

#### Scenario: Override beats inference
- **WHEN** a net's pins infer 3.3 V but an explicit `set_voltage(5.0.volt())` override exists
- **THEN** the compiled net has voltage 5.0 V

#### Scenario: Ground pins imply zero volts
- **WHEN** a net contains only `Gnd`-role pins with no explicit override
- **THEN** the net is `NetKind::Power` with `v_nom == 0.0.volt()` and `Net::is_ground()` returns true

#### Scenario: Power net without voltage is an error
- **WHEN** a net contains a `PowerIn` pin with `v_nom == None` and no override or ground pin
- **THEN** compilation fails with a `NET:NO_VOLTAGE_SOURCE` diagnostic naming the net

#### Scenario: Signal net inherits SigSpec
- **WHEN** a net connects two `DigitalIO` pins and one declares `SigSpec::spi(50.0)`
- **THEN** the net is `NetKind::Signal` carrying that `SigSpec`

### Requirement: NetHandle provides explicit override
`Board::connect()` and `Board::net()` SHALL return a `NetHandle` that allows the developer to annotate the emerging net before compilation. `set_net_voltage(handle, v)` SHALL set an explicit voltage override. `set_net_name(handle, name)` SHALL set an explicit net name. Overrides SHALL be recorded per net (not per connection edge) and SHALL take precedence over inference during compilation.

#### Scenario: Voltage override takes precedence
- **WHEN** a net connects two flexible pins (both `v_nom = None`) and `set_net_voltage(handle, 3.3.volt())` is called
- **THEN** the compiled net has voltage 3.3V with no "no voltage source" error

#### Scenario: Name override sets net name
- **WHEN** `set_net_name(handle, "VBUS")` is called on a `NetHandle`
- **THEN** the compiled net is named "VBUS" instead of the auto-generated name
