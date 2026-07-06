## ADDED Requirements

### Requirement: Connection records are serializable
The IR SHALL include a `Connection` struct with `refdes: String`, `pin: String`, and `net: String` fields. It SHALL derive `Clone`, `Debug`, `Serialize`, and `Deserialize`.

#### Scenario: Connection serializes to JSON
- **WHEN** a `Connection { refdes: "U1", pin: "VDD", net: "V3V3" }` is serialized to JSON
- **THEN** the output contains `"refdes": "U1"`, `"pin": "VDD"`, and `"net": "V3V3"`

#### Scenario: Connection deserializes from JSON
- **WHEN** a JSON object `{"refdes":"U1","pin":"VDD","net":"V3V3"}` is deserialized as a `Connection`
- **THEN** the resulting struct has `refdes == "U1"`, `pin == "VDD"`, `net == "V3V3"`

### Requirement: Design stores connections as a serializable vec
The `Design` struct SHALL include a `connections: Vec<Connection>` field that is serialized as part of the JSON IR. The `DesignGraph` SHALL remain `#[serde(skip)]` as a derived index.

#### Scenario: Serialized design includes connections
- **WHEN** a design with two connections (U1.VDD→V3V3, U1.GND→GND) is serialized to JSON
- **THEN** the JSON contains a `"connections"` array with two entries matching those connections

#### Scenario: Empty design has empty connections array
- **WHEN** a `Design::default()` is serialized to JSON
- **THEN** the JSON contains `"connections": []`

### Requirement: Design round-trips through JSON losslessly
Deserializing a serialized `Design` SHALL produce a design with the same nets, components, connections, and constraints. The `DesignGraph` SHALL be rebuilt from the `connections` vec during deserialization.

#### Scenario: Round-trip preserves connectivity
- **WHEN** a design is built with `d.connect("U1", "VDD", "V3V3")`, serialized to JSON, and deserialized back
- **THEN** `pins_on_net("V3V3")` on the deserialized design returns `[("U1", "VDD")]`
- **AND** `nets_of_pin("U1", "VDD")` returns `["V3V3"]`

#### Scenario: Round-trip preserves graph counts
- **WHEN** a design with 3 nets and 5 connections is serialized and deserialized
- **THEN** `graph.counts()` on the deserialized design returns the same node and edge counts as the original

### Requirement: Connect writes to both connections vec and graph
`Design::connect(refdes, pin, net)` SHALL push a `Connection` to `self.connections` and add the corresponding edge to `self.graph`. Duplicate connections (same refdes, pin, net) SHALL NOT be added to either store.

#### Scenario: Connect populates both stores
- **WHEN** `d.connect("U1", "VDD", "V3V3")` is called on an empty design
- **THEN** `d.connections` contains one `Connection` entry
- **AND** `d.pins_on_net("V3V3")` returns `[("U1", "VDD")]`

#### Scenario: Duplicate connect is ignored
- **WHEN** `d.connect("U1", "VDD", "V3V3")` is called twice
- **THEN** `d.connections` contains exactly one entry
- **AND** `d.pins_on_net("V3V3")` returns one pin

### Requirement: Qty Second has as_mhz method
`Qty<Second>` SHALL provide an `as_mhz(&self) -> f64` method that converts the stored period to a frequency in megahertz via `1.0 / self.as_base() / 1.0e6`.

#### Scenario: 50 MHz period converts back to 50
- **WHEN** `50.0.mhz().as_mhz()` is called
- **THEN** the result is approximately `50.0` (within 1e-9 tolerance)
