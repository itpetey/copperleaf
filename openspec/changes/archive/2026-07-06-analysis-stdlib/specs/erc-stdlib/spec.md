## ADDED Requirements

### Requirement: run_erc runs all built-in ERC rules
The `copperleaf-analysis` crate SHALL provide `pub fn run_erc(design: &Design) -> Vec<Diagnostic>` that executes all built-in ERC rules and returns a flat list of diagnostics.

#### Scenario: Design with no violations
- **WHEN** `run_erc()` is called on a correctly-wired design
- **THEN** an empty `Vec<Diagnostic>` is returned

#### Scenario: Design with NC pin connected
- **WHEN** a pin named "NC" is connected to a net
- **THEN** `run_erc()` returns a diagnostic with code `ERC:NC_CONNECTED` and severity `Error`

### Requirement: ERC rule for NC pin connected
The `copperleaf-analysis` crate SHALL provide `pub fn erc_nc_pin_connected(design: &Design) -> Vec<Diagnostic>` that flags any pin named "NC" or starting with "NC_" that is connected to a net.

#### Scenario: NC pin connected to GND
- **WHEN** a component has a pin named "NC" connected to the "GND" net
- **THEN** a diagnostic is returned with code `ERC:NC_CONNECTED`, severity `Error`, and a message mentioning the refdes and pin name

#### Scenario: NC pin floating (correct)
- **WHEN** a component has a pin named "NC" with no connections
- **THEN** no diagnostic is returned

### Requirement: ERC rule for floating input pins
The `copperleaf-analysis` crate SHALL provide `pub fn erc_floating_inputs(design: &Design) -> Vec<Diagnostic>` that flags input pins (AnalogIn, DigitalIO with no SigSpec) that are not connected to any net.

#### Scenario: Unconnected digital input
- **WHEN** a component has a DigitalIO pin with no signal spec and no net connection
- **THEN** a diagnostic is returned with code `ERC:FLOATING_INPUT` and severity `Warning`

### Requirement: ERC rule for floating power inputs
The `copperleaf-analysis` crate SHALL provide `pub fn erc_floating_power_inputs(design: &Design) -> Vec<Diagnostic>` that flags `PowerIn` pins that are not connected to any net.

#### Scenario: Unconnected power input
- **WHEN** a component has a `PowerIn` pin with no net connection
- **THEN** a diagnostic is returned with code `ERC:FLOATING_POWER_INPUT` and severity `Warning`

### Requirement: ERC rule for multi-net power pins
The `copperleaf-analysis` crate SHALL provide `pub fn erc_multi_net_power(design: &Design) -> Vec<Diagnostic>` that flags `PowerIn` pins connected to more than one net.

#### Scenario: Power input on two nets
- **WHEN** a `PowerIn` pin is connected to both "VBUS" and "VCC"
- **THEN** a diagnostic is returned with code `ERC:MULTI_NET_POWER` and severity `Warning`


