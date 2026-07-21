## ADDED Requirements

### Requirement: Single-pin nets are first-class
`Board` SHALL provide a `net(pin: PinHandle) -> Result<NetHandle, CompileError>` method that registers a net containing exactly one pin (e.g. a lone power pin needing a named net). The API SHALL NOT create a self-connection edge; the single-pin net SHALL be represented directly during net grouping. The `helpers::pwr_net` self-connection helper SHALL be removed.

#### Scenario: Single-pin power net gets a name and voltage
- **WHEN** `let h = board.net(src.pin(Pwr::VCC))?` is called and `h` is annotated with a name and voltage
- **THEN** the compiled board contains a one-pin net with that name and voltage
- **AND** the connection graph contains no edge from the pin to itself
