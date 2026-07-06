//! Intermediate representation (IR) for Copperleaf designs.
//!
//! This crate models pins, nets, components, constraints and a lightweight
//! connectivity graph. It is intended to be serialized to and from JSON and
//! consumed by analysis passes and backends.

use std::collections::HashMap;

use copperleaf_core::{Amp, Celsius, Diagnostic, Farad, Meter, Ohm, Qty, Second, UnitExt, Volt};
use petgraph::graph::{Graph, NodeIndex};
use serde::{Deserialize, Serialize};

// Component trait and instance wrapper
/// Trait implemented by parts to expose identity, pins, and default constraints.
pub trait Block {
    fn pin(&self, idx: usize) -> Option<&Pin> {
        let pins = self.pins();
        // Decrement the idx here because pins are 1-based and the array is 0-based.
        // i.e. pin 1 = idx 0
        pins.get(idx - 1)
    }
    fn pins(&self) -> &[Pin];
    fn constraints(&self) -> Vec<Constraint> {
        vec![]
    }
}

// Roles and signal kinds
/// Electrical role of a pin used to infer ERC rules and routing.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Role {
    PowerIn,
    PowerOut,
    AnalogIn,
    AnalogOut,
    DigitalIO,
    DiffPos,
    DiffNeg,
    Gnd,
}

/// Classifies a signal family and integrity expectations.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum SigKind {
    Generic,
    Usb2Hs,
    Usb3,
    Ddr3,
    PcieGen2,
    Clock,
    AnalogLowNoise,
}

/// Absolute electrical limits for a pin.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Limits {
    pub v_min: Qty<Volt>,
    pub v_max: Qty<Volt>,
    pub i_max: Qty<Amp>,
}

/// Signal integrity specification for a net or pin.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SigSpec {
    pub kind: SigKind,
    pub bandwidth: Option<Qty<Second>>, // period
    pub edge_rate: Option<Qty<Second>>,
    pub target_impedance: Option<Qty<Ohm>>,
}

/// A logical pin on a component footprint.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pin {
    pub name: String,
    pub role: Role,
    pub limits: Limits,
    pub sig: Option<SigSpec>,
}

/// Different classes of nets supported by the IR.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetKind {
    Power {
        v_nom: Qty<Volt>,
        ripple: Option<Qty<Volt>>,
    },
    Signal {
        spec: SigSpec,
    },
}

/// Reusable constraints for a class of nets.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NetClass {
    pub min_width: Option<Qty<Meter>>,
    pub clearance: Option<Qty<Meter>>,
}

/// Physical and verification constraints associated with nets and designs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Constraint {
    Impedance {
        target: Qty<Ohm>,
        tol_pct: f64,
    },
    LengthMatch {
        group: String,
        skew_ps: f64,
    },
    ReturnPath {
        requires_plane: bool,
    },
    NetClass {
        min_width: Qty<Meter>,
        clearance: Qty<Meter>,
    },
    Creepage {
        min: Qty<Meter>,
        voltage: Qty<Volt>,
    },
    Decoupling {
        values: Vec<Qty<Farad>>,
        per_pin: bool,
    },
    ResonanceIndex {
        max: f64,
    },
    MaxJunction {
        temp: Qty<Celsius>,
    },
}

/// A named net with kind, class, and attached constraints.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Net {
    pub name: String,
    pub kind: NetKind,
    pub class: NetClass,
    pub constraints: Vec<Constraint>,
}

/// An instantiated component with a reference designator.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComponentInst<B: Block> {
    pub refdes: String,
    pub block: B,
}

/// A component placed in a design, capturing its reference designator, pins,
/// and constraints extracted from the original [`Block`] at insertion time.
///
/// This is the serializable shadow of a [`ComponentInst`] — the generic block
/// type is erased so that heterogeneous parts can live in a single [`Design`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComponentRecord {
    /// Reference designator (e.g. `U1`).
    pub refdes: String,
    /// Pins exposed by the component, copied from the [`Block`] definition.
    pub pins: Vec<Pin>,
    /// Constraints attached to the component, copied from the [`Block`] definition.
    pub constraints: Vec<Constraint>,
}

/// A serializable connection record linking a component pin to a net.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Connection {
    pub refdes: String,
    pub pin: String,
    pub net: String,
}

/// Graph node: either a named net or a concrete (refdes.pin) endpoint.
#[derive(Clone, Debug)]
pub enum Node {
    Net(String),
    Pin { refdes: String, pin: String },
}

/// Graph edge kinds; currently only electrical connectivity is modeled.
#[derive(Clone, Debug)]
pub enum Edge {
    Electrical,
}

/// Internal connectivity graph used by [`Design`]. Not serialized.
#[derive(Default)]
pub struct DesignGraph {
    pub g: Graph<Node, Edge>,
    index: HashMap<String, NodeIndex>,
}

// Design container (graph elided for now)
/// Top-level container for a design’s nets, components, constraints and diagnostics.
#[derive(Default, Serialize)]
pub struct Design {
    pub nets: Vec<Net>,
    pub components: Vec<ComponentRecord>,
    pub constraints: Vec<Constraint>,
    pub diagnostics: Vec<Diagnostic>,
    pub connections: Vec<Connection>,
    #[serde(skip)]
    pub graph: DesignGraph,
}

/// Helper struct used to deserialize a [`Design`] without its derived graph.
/// After deserialization, connections are replayed into a fresh [`DesignGraph`].
#[derive(Deserialize)]
struct DesignRaw {
    nets: Vec<Net>,
    components: Vec<ComponentRecord>,
    constraints: Vec<Constraint>,
    diagnostics: Vec<Diagnostic>,
    connections: Vec<Connection>,
}

impl<'de> Deserialize<'de> for Design {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = DesignRaw::deserialize(deserializer)?;
        let mut design = Design {
            nets: raw.nets,
            components: raw.components,
            constraints: raw.constraints,
            diagnostics: raw.diagnostics,
            connections: raw.connections,
            graph: DesignGraph::default(),
        };
        // Take ownership so connect() can rebuild the graph and re-populate
        // the connections vec, naturally deduplicating any duplicate records.
        let conns = std::mem::take(&mut design.connections);
        for conn in &conns {
            design.connect(&conn.refdes, &conn.pin, &conn.net);
        }
        Ok(design)
    }
}

impl Limits {
    pub fn new(v_min: Qty<Volt>, v_max: Qty<Volt>, i_max: Qty<Amp>) -> Self {
        Self {
            v_min,
            v_max,
            i_max,
        }
    }
}

impl SigSpec {
    pub fn new(
        kind: SigKind,
        bandwidth: Option<Qty<Second>>,
        edge_rate: Option<Qty<Second>>,
        target_impedance: Option<Qty<Ohm>>,
    ) -> Self {
        Self {
            kind,
            bandwidth,
            edge_rate,
            target_impedance,
        }
    }

    /// Generic SPI signal with the given bandwidth in MHz and 50 Ω target impedance.
    pub fn spi(bw_mhz: f64) -> Self {
        Self {
            kind: SigKind::Generic,
            bandwidth: Some(bw_mhz.mhz()),
            edge_rate: None,
            target_impedance: Some(50.0.ohm()),
        }
    }

    /// SPI clock signal with the given bandwidth in MHz and 50 Ω target impedance.
    pub fn spi_clk(bw_mhz: f64) -> Self {
        Self {
            kind: SigKind::Clock,
            bandwidth: Some(bw_mhz.mhz()),
            edge_rate: None,
            target_impedance: Some(50.0.ohm()),
        }
    }

    /// Generic control signal with no bandwidth or impedance target.
    pub fn control() -> Self {
        Self {
            kind: SigKind::Generic,
            bandwidth: None,
            edge_rate: None,
            target_impedance: None,
        }
    }

    /// Analog low-noise 50 Ω signal (e.g., RF).
    pub fn rf_50ohm() -> Self {
        Self {
            kind: SigKind::AnalogLowNoise,
            bandwidth: None,
            edge_rate: None,
            target_impedance: Some(50.0.ohm()),
        }
    }
}

impl Pin {
    pub fn new(name: impl Into<String>, role: Role, limits: Limits, sig: Option<SigSpec>) -> Self {
        Self {
            name: name.into(),
            role,
            limits,
            sig,
        }
    }

    pub fn duplicate(&self, name: impl Into<String>) -> Self {
        let mut dupe = self.clone();
        dupe.name = name.into();
        dupe
    }
}

impl Net {
    /// Create a power net with nominal voltage.
    pub fn power(name: &str, v_nom: Qty<Volt>) -> Self {
        Self {
            name: name.to_string(),
            kind: NetKind::Power {
                v_nom,
                ripple: None,
            },
            class: NetClass::default(),
            constraints: vec![],
        }
    }
    /// Convenience constructor for a ground net named `GND`.
    pub fn ground() -> Self {
        Self::power("GND", 0.0.volt())
    }
    /// Set allowed ripple for a power net.
    pub fn ripple(mut self, r: Qty<Volt>) -> Self {
        if let NetKind::Power { v_nom, .. } = self.kind {
            self.kind = NetKind::Power {
                v_nom,
                ripple: Some(r),
            };
        }
        self
    }
}

impl<B: Block> ComponentInst<B> {
    /// Construct a new component instance with the provided reference and part.
    pub fn new(refdes: &str, block: B) -> Self {
        Self {
            refdes: refdes.to_owned(),
            block,
        }
    }
}

impl DesignGraph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    fn key_net(name: &str) -> String {
        format!("net:{}", name)
    }
    fn key_pin(refdes: &str, pin: &str) -> String {
        format!("pin:{}:{}", refdes, pin)
    }

    /// Ensure a net node exists and return its index.
    pub fn ensure_net(&mut self, name: &str) -> NodeIndex {
        let key = Self::key_net(name);
        if let Some(&idx) = self.index.get(&key) {
            return idx;
        }
        let idx = self.g.add_node(Node::Net(name.to_string()));
        self.index.insert(key, idx);
        idx
    }
    /// Ensure a pin node exists and return its index.
    pub fn ensure_pin(&mut self, refdes: &str, pin: &str) -> NodeIndex {
        let key = Self::key_pin(refdes, pin);
        if let Some(&idx) = self.index.get(&key) {
            return idx;
        }
        let idx = self.g.add_node(Node::Pin {
            refdes: refdes.to_string(),
            pin: pin.to_string(),
        });
        self.index.insert(key, idx);
        idx
    }

    /// Returns `(node_count, edge_count)` for basic introspection.
    pub fn counts(&self) -> (usize, usize) {
        (self.g.node_count(), self.g.edge_count())
    }

    /// Returns a list of `(refdes, pin)` tuples connected to the given net.
    pub fn pins_on_net(&self, net: &str) -> Vec<(String, String)> {
        let key = Self::key_net(net);
        let Some(&nidx) = self.index.get(&key) else {
            return vec![];
        };
        let mut out = Vec::new();
        for other in self.g.neighbors_undirected(nidx) {
            if let Some(Node::Pin { refdes, pin }) = self.g.node_weight(other) {
                out.push((refdes.clone(), pin.clone()));
            }
        }
        out
    }

    /// Returns the list of net names connected to a particular pin.
    pub fn nets_of_pin(&self, refdes: &str, pin: &str) -> Vec<String> {
        let key = Self::key_pin(refdes, pin);
        let Some(&pidx) = self.index.get(&key) else {
            return vec![];
        };
        let mut out = Vec::new();
        for other in self.g.neighbors_undirected(pidx) {
            if let Some(Node::Net(name)) = self.g.node_weight(other) {
                out.push(name.clone());
            }
        }
        out
    }
}

impl Design {
    /// Add a net to the design.
    pub fn add_net(&mut self, n: Net) {
        self.nets.push(n);
    }
    /// Add a component to the design, capturing its pins and constraints.
    ///
    /// Graph connectivity is separate — use [`Design::connect`] to wire pins
    /// to nets after the component has been added.
    pub fn add_component<B: Block>(&mut self, inst: ComponentInst<B>) {
        self.components.push(ComponentRecord {
            refdes: inst.refdes,
            pins: inst.block.pins().to_vec(),
            constraints: inst.block.constraints(),
        });
    }
    /// Returns the component record with the given reference designator.
    pub fn component_by_refdes(&self, refdes: &str) -> Option<&ComponentRecord> {
        self.components.iter().find(|c| c.refdes == refdes)
    }
    /// Add a top-level constraint to the design.
    pub fn add_constraint(&mut self, c: Constraint) {
        self.constraints.push(c);
    }

    /// Connect a component pin to a net using `"refdes.pin"` notation.
    ///
    /// Panics if `pin` does not contain a `.` separator.
    pub fn wire(&mut self, pin: &str, net: &str) {
        let Some((refdes, pin_name)) = pin.split_once('.') else {
            panic!("malformed pin string '{}': expected refdes.pin", pin);
        };
        self.connect(refdes, pin_name, net);
    }

    /// Connect each pin in `pins` to `net` using [`Design::wire`] semantics.
    pub fn connect_net(&mut self, net: &str, pins: &[&str]) {
        for pin in pins {
            self.wire(pin, net);
        }
    }

    /// Connect a component pin to a net (creates nodes if missing).
    pub fn connect(&mut self, refdes: &str, pin: &str, net: &str) {
        let conn = Connection {
            refdes: refdes.to_string(),
            pin: pin.to_string(),
            net: net.to_string(),
        };
        if !self.connections.iter().any(|c| {
            c.refdes == conn.refdes && c.pin == conn.pin && c.net == conn.net
        }) {
            self.connections.push(conn);
        }
        let p_idx = self.graph.ensure_pin(refdes, pin);
        let n_idx = self.graph.ensure_net(net);
        if self.graph.g.find_edge(p_idx, n_idx).is_none() {
            self.graph.g.add_edge(p_idx, n_idx, Edge::Electrical);
        }
    }

    /// List pins currently connected to a given net.
    pub fn pins_on_net(&self, net: &str) -> Vec<(String, String)> {
        self.graph.pins_on_net(net)
    }

    /// List nets a given component pin is connected to.
    pub fn nets_of_pin(&self, refdes: &str, pin: &str) -> Vec<String> {
        self.graph.nets_of_pin(refdes, pin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_parses_refdes_pin_notation() {
        let mut d = Design::default();
        d.add_net(Net::power("V3V3", 3.3.volt()));
        d.wire("U1.VDD", "V3V3");
        let pins = d.pins_on_net("V3V3");
        assert_eq!(pins, vec![("U1".into(), "VDD".into())]);
    }

    #[test]
    #[should_panic(expected = "malformed pin string")]
    fn wire_panics_on_missing_dot() {
        let mut d = Design::default();
        d.wire("invalid", "V3V3");
    }

    #[test]
    fn connect_net_wires_multiple_pins() {
        let mut d = Design::default();
        d.add_net(Net::power("SDIO_CLK", 3.3.volt()));
        d.connect_net("SDIO_CLK", &["U1.SDIO_CLK", "U2.GPIO2"]);
        let mut pins = d.pins_on_net("SDIO_CLK");
        pins.sort();
        let mut expected = vec![("U1".into(), "SDIO_CLK".into()), ("U2".into(), "GPIO2".into())];
        expected.sort();
        assert_eq!(pins, expected);
    }

    #[test]
    fn connect_builds_graph() {
        let mut d = Design::default();
        d.add_net(Net::power("V3V3", 3.3.volt()));
        d.connect("U1", "VDD", "V3V3");
        let (n, e) = d.graph.counts();
        assert!(n >= 2 && e >= 1);
        let pins = d.pins_on_net("V3V3");
        assert_eq!(pins, vec![("U1".into(), "VDD".into())]);
        let nets = d.nets_of_pin("U1", "VDD");
        assert_eq!(nets, vec![String::from("V3V3")]);
    }

    #[test]
    fn default_design_serializes_empty_connections_array() {
        let d = Design::default();
        let json = serde_json::to_string(&d).unwrap();
        assert!(json.contains("\"connections\":[]"));
    }

    #[test]
    fn connection_round_trips_through_json() {
        let conn = Connection {
            refdes: "U1".into(),
            pin: "VDD".into(),
            net: "V3V3".into(),
        };
        let json = serde_json::to_string(&conn).unwrap();
        assert!(json.contains("\"refdes\":\"U1\""));
        assert!(json.contains("\"pin\":\"VDD\""));
        assert!(json.contains("\"net\":\"V3V3\""));

        let restored: Connection = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.refdes, "U1");
        assert_eq!(restored.pin, "VDD");
        assert_eq!(restored.net, "V3V3");
    }

    #[test]
    fn connect_populates_connections_and_deduplicates() {
        let mut d = Design::default();
        d.add_net(Net::power("V3V3", 3.3.volt()));
        d.connect("U1", "VDD", "V3V3");
        assert_eq!(d.connections.len(), 1);
        assert_eq!(d.connections[0].refdes, "U1");
        assert_eq!(d.connections[0].pin, "VDD");
        assert_eq!(d.connections[0].net, "V3V3");

        d.connect("U1", "VDD", "V3V3");
        assert_eq!(d.connections.len(), 1);
        assert_eq!(d.pins_on_net("V3V3"), vec![("U1".into(), "VDD".into())]);
    }

    #[test]
    fn design_round_trip_preserves_connectivity() {
        let mut d = Design::default();
        d.add_net(Net::power("V3V3", 3.3.volt()));
        d.connect("U1", "VDD", "V3V3");
        d.connect("U1", "GND", "GND");
        d.connect("U2", "VDD", "V3V3");

        let json = serde_json::to_string(&d).unwrap();
        let restored: Design = serde_json::from_str(&json).unwrap();

        assert_eq!(d.connections, restored.connections);

        let mut restored_pins = restored.pins_on_net("V3V3");
        restored_pins.sort();
        let mut expected_pins = vec![("U1".into(), "VDD".into()), ("U2".into(), "VDD".into())];
        expected_pins.sort();
        assert_eq!(restored_pins, expected_pins);
        assert_eq!(restored.nets_of_pin("U1", "VDD"), vec![String::from("V3V3")]);
        assert_eq!(restored.nets_of_pin("U1", "GND"), vec![String::from("GND")]);
    }

    #[test]
    fn design_round_trip_preserves_graph_counts() {
        let mut d = Design::default();
        d.add_net(Net::power("V3V3", 3.3.volt()));
        d.add_net(Net::power("GND", 0.0.volt()));
        d.connect("U1", "VDD", "V3V3");
        d.connect("U1", "GND", "GND");
        d.connect("U2", "VDD", "V3V3");

        let before = d.graph.counts();
        let json = serde_json::to_string(&d).unwrap();
        let restored: Design = serde_json::from_str(&json).unwrap();
        let after = restored.graph.counts();

        assert_eq!(before, after);
    }

    #[test]
    fn spi_preset_is_generic_with_bandwidth_and_impedance() {
        let spec = SigSpec::spi(50.0);
        assert!(matches!(spec.kind, SigKind::Generic));
        assert!((spec.bandwidth.unwrap().as_mhz() - 50.0).abs() < 1e-9);
        assert!(approx_eq(spec.target_impedance.unwrap().as_base(), 50.0));
    }

    #[test]
    fn spi_clk_preset_is_clock() {
        let spec = SigSpec::spi_clk(50.0);
        assert!(matches!(spec.kind, SigKind::Clock));
        assert!((spec.bandwidth.unwrap().as_mhz() - 50.0).abs() < 1e-9);
        assert!(approx_eq(spec.target_impedance.unwrap().as_base(), 50.0));
    }

    #[test]
    fn control_preset_is_generic_with_no_bandwidth_or_impedance() {
        let spec = SigSpec::control();
        assert!(matches!(spec.kind, SigKind::Generic));
        assert!(spec.bandwidth.is_none());
        assert!(spec.target_impedance.is_none());
    }

    #[test]
    fn rf_50ohm_preset_is_analog_low_noise() {
        let spec = SigSpec::rf_50ohm();
        assert!(matches!(spec.kind, SigKind::AnalogLowNoise));
        assert!(spec.bandwidth.is_none());
        assert!(approx_eq(spec.target_impedance.unwrap().as_base(), 50.0));
    }

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }
}
