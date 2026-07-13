# inferred-nets Specification

## Purpose
TBD - created by archiving change core-redesign. Update Purpose after archive.
## Requirements
### Requirement: Nets are inferred from connectivity
During `Board::compile()`, the compiler SHALL identify nets as connected components of pins in the connection graph. Each connected component SHALL become a `Net` with an auto-generated name (deterministic, derived from the first connected pin's component and pin name) unless explicitly named.

#### Scenario: Two connected pins form one net
- **WHEN** `board.connect(rpi.pin(Rp2354a::IOVDD), radio.pin(Mm8108::VBAT))` is called and compiled
- **THEN** the `CompiledBoard` contains one `Net` connecting both pins

#### Scenario: Unconnected pins do not form nets
- **WHEN** a board has two components with no connections between them
- **THEN** the `CompiledBoard` contains no nets

#### Scenario: Daisy-chain forms single net
- **WHEN** pin A is connected to pin B, and pin B is connected to pin C
- **THEN** the `CompiledBoard` contains one net containing all three pins

### Requirement: Net carries inferred electrical properties
Each inferred `Net` SHALL carry a `NetKind` (Power or Signal) determined by the connected pins. A net connected to any `PowerIn` or `PowerOut` pin SHALL be `NetKind::Power` with an inferred voltage. A net connected only to digital/analog pins SHALL be `NetKind::Signal` with an inferred `SigSpec` if any pin declares one. The net SHALL also carry a `NetClass` with trace width and clearance defaults.

#### Scenario: Power net gets voltage from v_nom
- **WHEN** a net connects a `PowerIn` pin with `v_nom = Some(3.3.volt())` to another pin
- **THEN** the net is `NetKind::Power` with `v_nom = 3.3.volt()`

#### Scenario: Signal net inherits SigSpec from pin
- **WHEN** a net connects two `DigitalIO` pins and one declares `SigSpec::spi(50.0)`
- **THEN** the net is `NetKind::Signal` carrying that `SigSpec`

### Requirement: NetHandle provides explicit override
`Board::connect()` SHALL return a `NetHandle` that allows the developer to annotate the emerging net before compilation. `NetHandle::set_voltage(v)` SHALL set an explicit voltage override. `NetHandle::set_name(name)` SHALL set an explicit net name. Overrides SHALL take precedence over inference during compilation.

#### Scenario: Voltage override takes precedence
- **WHEN** a net connects two flexible pins (both `v_nom = None`) and `net.set_voltage(3.3.volt())` is called
- **THEN** the compiled net has voltage 3.3V with no "no voltage source" error

#### Scenario: Name override sets net name
- **WHEN** `net.set_name("VBUS")` is called on a `NetHandle`
- **THEN** the compiled net is named "VBUS" instead of the auto-generated name

### Requirement: Merged nets combine overrides
When two `connect()` calls join the same connected component, the resulting net SHALL combine all overrides. If voltage overrides conflict, an ERC error SHALL be produced. If only one side specifies a voltage, that value SHALL apply to the merged net.

#### Scenario: Merging nets with same voltage override
- **WHEN** net A has `set_voltage(3.3.volt())` and net B has `set_voltage(3.3.volt())` and they are joined by a third connection
- **THEN** the merged net has voltage 3.3V with no error

#### Scenario: Merging nets with conflicting voltage overrides
- **WHEN** net A has `set_voltage(3.3.volt())` and net B has `set_voltage(5.0.volt())` and they are joined by a third connection
- **THEN** `compile()` returns `CompileError` with a voltage conflict diagnostic

