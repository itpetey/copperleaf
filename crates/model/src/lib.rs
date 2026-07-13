use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use serde::{Deserialize, Serialize};

pub mod units;

pub use units::{
    Amp, Celsius, Diagnostic, Farad, Henry, Hertz, Meter, Ohm, Qty, Second, Severity, UnitExt, Volt,
};

// ---------------------------------------------------------------------------
// Deterministic IDs
// ---------------------------------------------------------------------------

/// Deterministic UUID-formatted string (8-4-4-4-12 hex) derived from `seed`.
pub fn deterministic_id(seed: &str) -> String {
    let h1 = fnv1a_64(seed, 0);
    let h2 = fnv1a_64(seed, 0x6c14_4f3a_7af5_c5d2); // arbitrary fixed salt
    let b1 = h1.to_be_bytes();
    let b2 = h2.to_be_bytes();
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b1[0],
        b1[1],
        b1[2],
        b1[3],
        b1[4],
        b1[5],
        b1[6],
        b1[7],
        b2[0],
        b2[1],
        b2[2],
        b2[3],
        b2[4],
        b2[5],
        b2[6],
        b2[7]
    )
}

fn fnv1a_64(seed: &str, salt: u64) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET ^ salt;
    for b in seed.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

// ---------------------------------------------------------------------------
// Identifiers
// ---------------------------------------------------------------------------

/// Identifier for a specific pin on a component.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PinId(pub String);

/// Identifier for a net name.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NetId(pub String);

// ---------------------------------------------------------------------------
// Roles
// ---------------------------------------------------------------------------

/// Electrical role of a pin used to infer ERC rules and routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

// ---------------------------------------------------------------------------
// PowerSpec
// ---------------------------------------------------------------------------

/// Absolute electrical limits and nominal voltage for a pin.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PowerSpec {
    pub v_min: Qty<Volt>,
    pub v_max: Qty<Volt>,
    pub v_nom: Option<Qty<Volt>>,
    pub i_max: Qty<Amp>,
}

// ---------------------------------------------------------------------------
// Pin and PinBuilder
// ---------------------------------------------------------------------------

/// A logical pin on a component footprint.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pin {
    id: PinId,
    name: String,
    role: Role,
    power_spec: PowerSpec,
    decouple: bool,
    sig_spec: Option<SigSpec>,
    pos: Option<(f64, f64)>,
    rotation: Option<f64>,
    length: Option<f64>,
}

pub struct PinBuilder {
    name: String,
    role: Option<Role>,
    power_spec: Option<PowerSpec>,
    decouple: bool,
    sig_spec: Option<SigSpec>,
    pos: Option<(f64, f64)>,
    rotation: Option<f64>,
    length: Option<f64>,
}

impl Pin {
    /// Start building a new [`Pin`].
    pub fn build(name: &str) -> PinBuilder {
        PinBuilder::new(name)
    }

    pub fn with_id(mut self, id: PinId) -> Self {
        self.id = id;
        self
    }

    pub fn id(&self) -> &PinId {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn role(&self) -> Role {
        self.role
    }
    pub fn power_spec(&self) -> &PowerSpec {
        &self.power_spec
    }
    pub fn decouple(&self) -> bool {
        self.decouple
    }
    pub fn sig_spec(&self) -> Option<SigSpec> {
        self.sig_spec
    }
    pub fn pos(&self) -> Option<(f64, f64)> {
        self.pos
    }
    pub fn rotation(&self) -> Option<f64> {
        self.rotation
    }
    pub fn length(&self) -> Option<f64> {
        self.length
    }
}

impl PinBuilder {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            role: None,
            power_spec: None,
            decouple: false,
            sig_spec: None,
            pos: None,
            rotation: None,
            length: None,
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_owned();
        self
    }

    pub fn role(mut self, role: Role) -> Self {
        self.role = Some(role);
        self
    }

    pub fn power_spec(mut self, p: PowerSpec) -> Self {
        self.power_spec = Some(p);
        self
    }

    pub fn decouple(mut self, decouple: bool) -> Self {
        self.decouple = decouple;
        self
    }

    pub fn sig_spec(mut self, spec: SigSpec) -> Self {
        self.sig_spec = Some(spec);
        self
    }

    pub fn pos(mut self, x: f64, y: f64) -> Self {
        self.pos = Some((x, y));
        self
    }

    pub fn rotation(mut self, deg: f64) -> Self {
        self.rotation = Some(deg);
        self
    }

    pub fn length(mut self, mm: f64) -> Self {
        self.length = Some(mm);
        self
    }

    /// Fixed-voltage power input: `v_nom = v_min = v_max = v`, `decouple = true`.
    pub fn pwr_fixed(mut self, v: Qty<Volt>, i: Qty<Amp>) -> Self {
        self.role = Some(Role::PowerIn);
        self.power_spec = Some(PowerSpec {
            v_min: v,
            v_max: v,
            v_nom: Some(v),
            i_max: i,
        });
        self.decouple = true;
        self
    }

    /// Flexible power input: `v_nom = None`, `decouple = true`.
    pub fn pwr(mut self, v_min: Qty<Volt>, v_max: Qty<Volt>, i: Qty<Amp>) -> Self {
        self.role = Some(Role::PowerIn);
        self.power_spec = Some(PowerSpec {
            v_min,
            v_max,
            v_nom: None,
            i_max: i,
        });
        self.decouple = true;
        self
    }

    /// Chainable override to set `v_nom` on a flexible pin.
    pub fn nominal(mut self, v: Qty<Volt>) -> Self {
        if let Some(ref mut p) = self.power_spec {
            p.v_nom = Some(v);
        }
        self
    }

    pub fn digital_limits(mut self) -> Self {
        self.power_spec = Some(PowerSpec {
            v_min: 0.0.volt(),
            v_max: 3.6.volt(),
            v_nom: None,
            i_max: 0.02.amp(),
        });
        self
    }

    pub fn rf_limits(mut self) -> Self {
        self.power_spec = Some(PowerSpec {
            v_min: 0.0.volt(),
            v_max: 1.2.volt(),
            v_nom: None,
            i_max: 1.0.amp(),
        });
        self
    }

    /// Creates a new digital I/O [`Pin`].
    pub fn dio(mut self) -> Pin {
        self.role = Some(Role::DigitalIO);
        self.digital_limits().pin()
    }

    /// Creates a new digital I/O [`Pin`] for SPI.
    pub fn spi(mut self, bw_mhz: f64) -> Pin {
        self.role = Some(Role::DigitalIO);
        self.sig_spec = Some(SigSpec::spi(bw_mhz));
        self.digital_limits().pin()
    }

    /// Creates a new digital clock signal [`Pin`].
    pub fn clk(mut self, bw_mhz: f64) -> Pin {
        self.role = Some(Role::DigitalIO);
        self.sig_spec = Some(SigSpec::spi_clk(bw_mhz));
        self.digital_limits().pin()
    }

    /// Creates a new ground [`Pin`].
    pub fn gnd(mut self) -> Pin {
        self.role = Some(Role::Gnd);
        self.power_spec = Some(PowerSpec {
            v_min: 0.0.volt(),
            v_max: 0.0.volt(),
            v_nom: Some(0.0.volt()),
            i_max: 100.0.amp(),
        });
        self.pin()
    }

    /// Creates a new analogue input [`Pin`].
    pub fn analog_in(mut self) -> Pin {
        self.role = Some(Role::AnalogIn);
        self.digital_limits().pin()
    }

    /// Returns a [`Pin`] with the settings configured with this builder.
    ///
    /// # Panics
    ///
    /// This method will panic if `role` or `power_spec` is not set.
    pub fn pin(self) -> Pin {
        Pin {
            id: PinId(String::new()),
            name: self.name,
            role: self.role.unwrap(),
            power_spec: self.power_spec.unwrap(),
            decouple: self.decouple,
            sig_spec: self.sig_spec,
            pos: self.pos,
            rotation: self.rotation,
            length: self.length,
        }
    }
}

// ---------------------------------------------------------------------------
// Signal specifications
// ---------------------------------------------------------------------------

/// Classifies a signal family and integrity expectations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SigKind {
    Generic,
    Usb2Hs,
    Usb3,
    Ddr3,
    PcieGen2,
    Clock,
    AnalogLowNoise,
}

/// Signal integrity specification for a net or pin.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SigSpec {
    pub kind: SigKind,
    pub bandwidth: Option<Qty<Hertz>>,
    pub edge_rate: Option<Qty<Second>>,
    pub target_impedance: Option<Qty<Ohm>>,
}

impl SigSpec {
    pub fn new(
        kind: SigKind,
        bandwidth: Option<Qty<Hertz>>,
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

// ---------------------------------------------------------------------------
// Net kinds, classes, and nets
// ---------------------------------------------------------------------------

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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NetClass {
    pub min_width: Option<Qty<Meter>>,
    pub clearance: Option<Qty<Meter>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Net {
    pub name: String,
    pub kind: NetKind,
    pub class: NetClass,
    pub constraints: Vec<Constraint>,
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

// ---------------------------------------------------------------------------
// Constraints
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Component trait and typed handles
// ---------------------------------------------------------------------------

/// Represents a single part (e.g. a resistor, chip, etc.) on a PCB.
pub trait Component {
    /// Retrieves all [`Pin`]s attached to the [`Component`].
    fn pins(&self) -> &[Pin];

    /// Retrieves a [`Pin`] from this [`Component`] by ID, if it exists.
    fn pin(&self, id: PinId) -> Option<&Pin> {
        self.pins().iter().find(|p| *p.id() == id)
    }

    /// Retrieves a [`Pin`] by its name from this [`Component`], if it exists.
    fn pin_name(&self, name: &str) -> Option<&Pin> {
        self.pins().iter().find(|p| p.name() == name)
    }

    /// Constraints declared by this component for synthesis and analysis.
    fn constraints(&self) -> Vec<Constraint> {
        vec![]
    }

    /// Embedded symbol S-expression, if any.
    fn symbol(&self) -> Option<&'static str> {
        None
    }

    /// Embedded footprint S-expression, if any.
    fn footprint(&self) -> Option<&'static str> {
        None
    }
}

/// Typed reference to a pin name constant on a component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PinRef(pub &'static str);

/// Handle to a component instance on a [`Board`].
#[derive(Clone, Copy, Debug)]
pub struct ComponentHandle(pub usize);

impl ComponentHandle {
    /// Create a [`PinHandle`] for a pin on this component.
    pub fn pin(&self, pin: PinRef) -> PinHandle {
        PinHandle {
            component: self.0,
            pin: pin.0,
        }
    }
}

/// Handle to a specific pin on a specific component instance.
#[derive(Clone, Copy, Debug)]
pub struct PinHandle {
    pub component: usize,
    pub pin: &'static str,
}

/// Handle to an emerging net, returned by [`Board::connect`].
#[derive(Clone, Debug)]
pub struct NetHandle {
    edge: usize,
    overrides: Rc<RefCell<HashMap<usize, NetOverride>>>,
}

impl NetHandle {
    /// Set an explicit voltage override for this net.
    pub fn set_voltage(&self, v: Qty<Volt>) {
        self.overrides
            .borrow_mut()
            .entry(self.edge)
            .or_default()
            .voltage = Some(v);
    }

    /// Set an explicit name override for this net.
    pub fn set_name(&self, name: &str) {
        self.overrides
            .borrow_mut()
            .entry(self.edge)
            .or_default()
            .name = Some(name.to_owned());
    }
}

#[derive(Clone, Debug, Default)]
struct NetOverride {
    voltage: Option<Qty<Volt>>,
    name: Option<String>,
}

// ---------------------------------------------------------------------------
// Board and connections
// ---------------------------------------------------------------------------

struct ComponentEntry {
    name: String,
    component: Box<dyn Component>,
}

#[derive(Clone, Copy, Debug)]
struct RawConnection {
    from: PinHandle,
    to: PinHandle,
}

/// Top level structure representing the PCB being designed.
pub struct Board {
    components: Vec<ComponentEntry>,
    connections: Vec<RawConnection>,
    net_overrides: Rc<RefCell<HashMap<usize, NetOverride>>>,
    next_edge: usize,
}

/// An immutable structure representing a finished [`Board`] that is ready for export.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompiledBoard {
    pub components: Vec<CompiledComponent>,
    pub nets: Vec<Net>,
    pub connections: Vec<Connection>,
    pub constraints: Vec<Constraint>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompiledComponent {
    pub refdes: String,
    pub pins: Vec<Pin>,
    pub constraints: Vec<Constraint>,
    pub symbol: Option<String>,
    pub footprint: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Connection {
    pub component: usize,
    pub pin: String,
    pub net: NetId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompileReport {
    pub board: CompiledBoard,
    pub warnings: Vec<Diagnostic>,
    pub summary: CompileSummary,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompileSummary {
    pub nets: Vec<NetInfo>,
    pub caps_synthesised: Vec<SynthCap>,
    pub pin_count: usize,
    pub component_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetInfo {
    pub name: String,
    pub kind: NetKind,
    pub pin_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SynthCap {
    pub refdes: String,
    pub value: Qty<Farad>,
    pub net: String,
    pub source_component: String,
    pub source_pin: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompileError {
    pub errors: Vec<Diagnostic>,
}

impl CompileError {
    pub fn new(errors: Vec<Diagnostic>) -> Self {
        Self { errors }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for e in &self.errors {
            writeln!(f, "[{:?}] {} — {}", e.severity, e.code, e.message)?;
            if let Some(hint) = &e.hint {
                writeln!(f, "         hint: {}", hint)?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for CompileError {}

// ---------------------------------------------------------------------------
// Backend trait
// ---------------------------------------------------------------------------

/// Trait implemented by backends that emit a [`CompiledBoard`] to a target format.
pub trait Backend {
    type Error;
    fn emit(&self, output_dir: &str, board: &CompiledBoard) -> Result<(), Self::Error>;
}

/// Common backend errors.
#[derive(Debug)]
pub enum BackendError {
    IoError(std::io::Error),
    EmitError(String),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "backend I/O error: {}", e),
            Self::EmitError(msg) => write!(f, "backend emit error: {}", msg),
        }
    }
}

impl std::error::Error for BackendError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
            Self::EmitError(_) => None,
        }
    }
}

impl From<std::io::Error> for BackendError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

// ---------------------------------------------------------------------------
// Board implementation
// ---------------------------------------------------------------------------

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl Board {
    /// Creates a new, unpopulated [`Board`].
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            connections: Vec::new(),
            net_overrides: Rc::new(RefCell::new(HashMap::new())),
            next_edge: 0,
        }
    }

    /// Add a [`Component`] to this board.
    pub fn add<C: Component + 'static>(&mut self, name: &str, component: C) -> ComponentHandle {
        let idx = self.components.len();
        self.components.push(ComponentEntry {
            name: name.to_owned(),
            component: Box::new(component),
        });
        ComponentHandle(idx)
    }

    /// Connect one [`PinHandle`] to another.
    ///
    /// Returns a [`NetHandle`] that can be used to annotate the emerging net.
    pub fn connect(&mut self, from: PinHandle, to: PinHandle) -> Result<NetHandle, CompileError> {
        if let Some(diag) = self.validate_pin(&from) {
            return Err(CompileError::new(vec![diag]));
        }
        if let Some(diag) = self.validate_pin(&to) {
            return Err(CompileError::new(vec![diag]));
        }
        let edge = self.next_edge;
        self.next_edge += 1;
        self.connections.push(RawConnection { from, to });
        Ok(NetHandle {
            edge,
            overrides: self.net_overrides.clone(),
        })
    }

    fn validate_pin(&self, handle: &PinHandle) -> Option<Diagnostic> {
        let Some(entry) = self.components.get(handle.component) else {
            return Some(Diagnostic {
                code: "BOARD:INVALID_COMPONENT".into(),
                severity: Severity::Error,
                message: format!("component index {} does not exist", handle.component),
                entities: vec![format!("{}", handle.component)],
                hint: None,
            });
        };
        if entry.component.pin_name(handle.pin).is_none() {
            return Some(Diagnostic {
                code: "BOARD:INVALID_PIN".into(),
                severity: Severity::Error,
                message: format!(
                    "pin '{}' does not exist on component '{}'",
                    handle.pin, entry.name
                ),
                entities: vec![format!("{}.{}", entry.name, handle.pin)],
                hint: None,
            });
        }
        None
    }

    /// Compiles this board design into an electronically correct model for export via
    /// any [`Backend`].
    pub fn compile(self) -> Result<CompileReport, CompileError> {
        // Validate all handles.
        for conn in &self.connections {
            if let Some(diag) = self.validate_pin(&conn.from) {
                return Err(CompileError::new(vec![diag]));
            }
            if let Some(diag) = self.validate_pin(&conn.to) {
                return Err(CompileError::new(vec![diag]));
            }
        }

        // Type-erase components into CompiledComponents with deterministic pin IDs.
        let mut compiled_components: Vec<CompiledComponent> = Vec::new();
        for entry in &self.components {
            let pins: Vec<Pin> = entry
                .component
                .pins()
                .iter()
                .map(|p| {
                    let seed = format!("{}:{}", entry.name, p.name());
                    p.clone().with_id(PinId(deterministic_id(&seed)))
                })
                .collect();
            compiled_components.push(CompiledComponent {
                refdes: entry.name.clone(),
                pins,
                constraints: entry.component.constraints(),
                symbol: entry.component.symbol().map(|s| s.to_owned()),
                footprint: entry.component.footprint().map(|s| s.to_owned()),
            });
        }

        // Build union-find over connected pins.
        let mut pin_to_node: HashMap<(usize, &'static str), usize> = HashMap::new();
        let mut nodes: Vec<(usize, &'static str)> = Vec::new();
        for conn in &self.connections {
            for handle in [conn.from, conn.to] {
                if let std::collections::hash_map::Entry::Vacant(e) =
                    pin_to_node.entry((handle.component, handle.pin))
                {
                    e.insert(nodes.len());
                    nodes.push((handle.component, handle.pin));
                }
            }
        }

        let mut uf = UnionFind::new(nodes.len());
        for conn in &self.connections {
            let a = pin_to_node[&(conn.from.component, conn.from.pin)];
            let b = pin_to_node[&(conn.to.component, conn.to.pin)];
            uf.union(a, b);
        }

        // Group nodes into nets.
        let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
        for i in 0..nodes.len() {
            groups.entry(uf.find(i)).or_default().push(i);
        }

        // Map each representative node to a net name and collect overrides.
        let overrides = self.net_overrides.borrow();
        let mut net_names: HashMap<usize, String> = HashMap::new();
        let mut net_voltages: HashMap<usize, Option<Qty<Volt>>> = HashMap::new();
        let mut errors: Vec<Diagnostic> = Vec::new();

        for (&rep, members) in &groups {
            // Gather all edge indices whose connected pins belong to this net.
            let mut edge_ids: Vec<usize> = Vec::new();
            for (edge_id, conn) in self.connections.iter().enumerate() {
                let a = (conn.from.component, conn.from.pin);
                let b = (conn.to.component, conn.to.pin);
                let a_rep = uf.find(pin_to_node[&a]);
                if a_rep == rep {
                    edge_ids.push(edge_id);
                    continue;
                }
                let b_rep = uf.find(pin_to_node[&b]);
                if b_rep == rep {
                    edge_ids.push(edge_id);
                }
            }

            // Merge overrides.
            let mut explicit_voltage: Option<Qty<Volt>> = None;
            let mut explicit_name: Option<String> = None;
            for eid in &edge_ids {
                if let Some(ov) = overrides.get(eid) {
                    if let Some(v) = ov.voltage {
                        if let Some(existing) = explicit_voltage {
                            if (existing.as_base() - v.as_base()).abs() > 1e-9 {
                                errors.push(Diagnostic {
                                    code: "NET:VOLTAGE_CONFLICT".into(),
                                    severity: Severity::Error,
                                    message: "conflicting voltage overrides on merged net".into(),
                                    entities: vec![format!("net:{}", rep)],
                                    hint: Some("resolve the voltage mismatch".into()),
                                });
                            }
                        } else {
                            explicit_voltage = Some(v);
                        }
                    }
                    if let Some(name) = &ov.name {
                        explicit_name = Some(name.clone());
                    }
                }
            }

            // Determine net name.
            let name = explicit_name.unwrap_or_else(|| {
                let (comp, pin) = nodes[rep];
                let comp_name = &self.components[comp].name;
                format!("NET_{}_{}", comp_name, pin)
            });
            net_names.insert(rep, name.clone());

            // Determine net voltage from pin v_nom values.
            let mut inferred_voltage: Option<Qty<Volt>> = None;
            for &node_idx in members {
                let (comp_idx, pin_name) = nodes[node_idx];
                let comp = &compiled_components[comp_idx];
                let pin = comp.pins.iter().find(|p| p.name() == pin_name).unwrap();
                if let Some(v) = pin.power_spec().v_nom {
                    if let Some(existing) = inferred_voltage {
                        if (existing.as_base() - v.as_base()).abs() > 1e-9 {
                            errors.push(Diagnostic {
                                code: "NET:VOLTAGE_CONFLICT".into(),
                                severity: Severity::Error,
                                message: format!(
                                    "conflicting v_nom values on net '{}' ({:.2}V vs {:.2}V)",
                                    name,
                                    existing.as_base(),
                                    v.as_base()
                                ),
                                entities: vec![name.clone()],
                                hint: Some("check connected power pins".into()),
                            });
                        }
                    } else {
                        inferred_voltage = Some(v);
                    }
                }
            }

            // Override takes precedence.
            let final_voltage = explicit_voltage.or(inferred_voltage);
            net_voltages.insert(rep, final_voltage);
        }

        // Build nets and connections.
        let mut nets: Vec<Net> = Vec::new();
        let mut connections: Vec<Connection> = Vec::new();

        for (&rep, members) in &groups {
            let name = net_names[&rep].clone();
            let voltage = net_voltages[&rep];

            // Determine kind.
            let mut is_power = false;
            let mut sig_spec: Option<SigSpec> = None;
            for &node_idx in members {
                let (comp_idx, pin_name) = nodes[node_idx];
                let pin = compiled_components[comp_idx]
                    .pins
                    .iter()
                    .find(|p| p.name() == pin_name)
                    .unwrap();
                match pin.role() {
                    Role::PowerIn | Role::PowerOut | Role::Gnd => is_power = true,
                    _ => {}
                }
                if sig_spec.is_none() && pin.sig_spec().is_some() {
                    sig_spec = pin.sig_spec();
                }
            }

            let kind = if is_power {
                match voltage {
                    Some(v_nom) => NetKind::Power {
                        v_nom,
                        ripple: None,
                    },
                    None => {
                        errors.push(Diagnostic {
                            code: "NET:NO_VOLTAGE_SOURCE".into(),
                            severity: Severity::Error,
                            message: format!(
                                "power net '{}' has no voltage source; use set_voltage()",
                                name
                            ),
                            entities: vec![name.clone()],
                            hint: Some("call NetHandle::set_voltage()".into()),
                        });
                        NetKind::Power {
                            v_nom: 0.0.volt(),
                            ripple: None,
                        }
                    }
                }
            } else {
                NetKind::Signal {
                    spec: sig_spec.unwrap_or_else(SigSpec::control),
                }
            };

            nets.push(Net {
                name: name.clone(),
                kind,
                class: NetClass::default(),
                constraints: vec![],
            });

            for &node_idx in members {
                let (comp_idx, pin_name) = nodes[node_idx];
                connections.push(Connection {
                    component: comp_idx,
                    pin: pin_name.to_owned(),
                    net: NetId(name.clone()),
                });
            }
        }

        // Collect top-level constraints.
        let constraints: Vec<Constraint> = compiled_components
            .iter()
            .flat_map(|c| c.constraints.clone())
            .collect();

        let board = CompiledBoard {
            components: compiled_components.clone(),
            nets,
            connections,
            constraints,
        };

        // Run ERC and synthesis.
        let mut warnings: Vec<Diagnostic> = Vec::new();
        let mut errors_erc: Vec<Diagnostic> = Vec::new();

        warnings.extend(erc_floating_inputs(&board, &self.connections));
        warnings.extend(erc_floating_power_inputs(&board, &self.connections));
        errors_erc.extend(erc_overvoltage(&board));
        errors_erc.extend(erc_nc_pin_connected(&board, &self.connections));

        errors.extend(errors_erc);
        if !errors.is_empty() {
            return Err(CompileError::new(errors));
        }

        let (synth_components, synth_caps, synth_diags) = synthesize_decoupling(&board);
        warnings.extend(synth_diags);

        let mut final_components = board.components;
        final_components.extend(synth_components);

        // Assign deterministic refdes to synthesised components.
        // They were already created with C1, C2, ... so no further action needed.

        let final_board = CompiledBoard {
            components: final_components,
            nets: board.nets,
            connections: board.connections,
            constraints: board.constraints,
        };

        let pin_count = final_board.components.iter().map(|c| c.pins.len()).sum();
        let component_count = final_board.components.len();

        let summary = CompileSummary {
            nets: final_board
                .nets
                .iter()
                .map(|n| NetInfo {
                    name: n.name.clone(),
                    kind: n.kind.clone(),
                    pin_count: final_board
                        .connections
                        .iter()
                        .filter(|c| c.net.0 == n.name)
                        .count(),
                })
                .collect(),
            caps_synthesised: synth_caps,
            pin_count,
            component_count,
        };

        Ok(CompileReport {
            board: final_board,
            warnings,
            summary,
        })
    }
}

// ---------------------------------------------------------------------------
// Union-find
// ---------------------------------------------------------------------------

struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra != rb {
            self.parent[rb] = ra;
        }
    }
}

// ---------------------------------------------------------------------------
// ERC and synthesis helpers
// ---------------------------------------------------------------------------

fn pin_is_connected(comp: usize, pin: &str, connections: &[RawConnection]) -> bool {
    connections.iter().any(|c| {
        (c.from.component == comp && c.from.pin == pin)
            || (c.to.component == comp && c.to.pin == pin)
    })
}

fn erc_floating_inputs(board: &CompiledBoard, connections: &[RawConnection]) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for (comp_idx, comp) in board.components.iter().enumerate() {
        for pin in &comp.pins {
            if pin.name == "NC" || pin.name.starts_with("NC_") {
                continue;
            }
            if matches!(pin.role, Role::DigitalIO | Role::AnalogIn)
                && pin.sig_spec.is_none()
                && !pin_is_connected(comp_idx, &pin.name, connections)
            {
                diags.push(Diagnostic {
                    code: "ERC:FLOATING_INPUT".into(),
                    severity: Severity::Warning,
                    message: format!("Input pin {}.{} is floating", comp.refdes, pin.name),
                    entities: vec![format!("{}.{}", comp.refdes, pin.name)],
                    hint: Some("Connect the pin or assign a signal specification".into()),
                });
            }
        }
    }
    diags
}

fn erc_floating_power_inputs(
    board: &CompiledBoard,
    connections: &[RawConnection],
) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for (comp_idx, comp) in board.components.iter().enumerate() {
        for pin in &comp.pins {
            if matches!(pin.role, Role::PowerIn)
                && !pin_is_connected(comp_idx, &pin.name, connections)
            {
                diags.push(Diagnostic {
                    code: "ERC:FLOATING_POWER_INPUT".into(),
                    severity: Severity::Warning,
                    message: format!(
                        "Power input pin {}.{} is unconnected",
                        comp.refdes, pin.name
                    ),
                    entities: vec![format!("{}.{}", comp.refdes, pin.name)],
                    hint: Some("Connect the pin to a power net".into()),
                });
            }
        }
    }
    diags
}

fn erc_overvoltage(board: &CompiledBoard) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for (comp_idx, comp) in board.components.iter().enumerate() {
        for pin in &comp.pins {
            if !matches!(pin.role, Role::PowerIn) {
                continue;
            }
            for conn in &board.connections {
                if conn.component == comp_idx
                    && conn.pin == pin.name
                    && let Some(net) = board.nets.iter().find(|n| n.name == conn.net.0)
                    && let NetKind::Power { v_nom, .. } = net.kind
                    && v_nom.as_base() > pin.power_spec.v_max.as_base() + 1e-9
                {
                    diags.push(Diagnostic {
                        code: "ERC:OVERVOLT".into(),
                        severity: Severity::Error,
                        message: format!(
                            "Pin {}.{} max {:.2}V, connected to {:.2}V net {}",
                            comp.refdes,
                            pin.name,
                            pin.power_spec.v_max.as_base(),
                            v_nom.as_base(),
                            net.name
                        ),
                        entities: vec![format!("{}.{}", comp.refdes, pin.name), net.name.clone()],
                        hint: Some("Use a level shifter or different pin".into()),
                    });
                }
            }
        }
    }
    diags
}

fn erc_nc_pin_connected(board: &CompiledBoard, connections: &[RawConnection]) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for (comp_idx, comp) in board.components.iter().enumerate() {
        for pin in &comp.pins {
            if (pin.name == "NC" || pin.name.starts_with("NC_"))
                && pin_is_connected(comp_idx, &pin.name, connections)
            {
                diags.push(Diagnostic {
                    code: "ERC:NC_CONNECTED".into(),
                    severity: Severity::Error,
                    message: format!("NC pin {}.{} is connected to a net", comp.refdes, pin.name),
                    entities: vec![format!("{}.{}", comp.refdes, pin.name)],
                    hint: Some("Leave no-connect pins unconnected".into()),
                });
            }
        }
    }
    diags
}

fn synthesize_decoupling(
    board: &CompiledBoard,
) -> (Vec<CompiledComponent>, Vec<SynthCap>, Vec<Diagnostic>) {
    let mut components = Vec::new();
    let mut caps = Vec::new();
    let mut diagnostics = Vec::new();
    let mut next_c = 1u32;

    for (comp_idx, comp) in board.components.iter().enumerate() {
        for constraint in &comp.constraints {
            let Constraint::Decoupling { values, per_pin } = constraint else {
                continue;
            };

            let power_pins: Vec<&Pin> = comp
                .pins
                .iter()
                .filter(|p| matches!(p.role(), Role::PowerIn))
                .collect();

            if power_pins.is_empty() {
                diagnostics.push(Diagnostic {
                    code: "DECOUPLE:NO_PWR_PIN".into(),
                    severity: Severity::Warning,
                    message: format!(
                        "{} has a decoupling constraint but no power-input pins",
                        comp.refdes
                    ),
                    entities: vec![comp.refdes.clone()],
                    hint: Some("add a PowerIn pin to the part definition".into()),
                });
                continue;
            }

            let target_pins: Vec<&Pin> = if *per_pin {
                power_pins.clone()
            } else {
                vec![power_pins[0]]
            };

            for pin in target_pins {
                let net_name = board
                    .connections
                    .iter()
                    .find(|c| c.component == comp_idx && c.pin == pin.name())
                    .map(|c| c.net.0.clone());

                let Some(net) = net_name else {
                    diagnostics.push(Diagnostic {
                        code: "DECOUPLE:UNCONNECTED".into(),
                        severity: Severity::Warning,
                        message: format!(
                            "power pin {}.{} is not connected to a net",
                            comp.refdes,
                            pin.name()
                        ),
                        entities: vec![format!("{}.{}", comp.refdes, pin.name())],
                        hint: Some("connect the pin to a power net".into()),
                    });
                    continue;
                };

                for value in values {
                    let refdes = format!("C{}", next_c);
                    next_c += 1;
                    let pin1_id = PinId(deterministic_id(&format!("{}:1", refdes)));
                    let pin2_id = PinId(deterministic_id(&format!("{}:2", refdes)));
                    components.push(CompiledComponent {
                        refdes: refdes.clone(),
                        pins: vec![
                            Pin::build("1")
                                .pwr_fixed(50.0.volt(), 0.1.amp())
                                .decouple(false)
                                .pin()
                                .with_id(pin1_id),
                            Pin::build("2").gnd().with_id(pin2_id),
                        ],
                        constraints: vec![],
                        symbol: None,
                        footprint: None,
                    });
                    caps.push(SynthCap {
                        refdes: refdes.clone(),
                        value: *value,
                        net: net.clone(),
                        source_component: comp.refdes.clone(),
                        source_pin: pin.name().to_owned(),
                    });
                }
            }
        }
    }

    if !caps.is_empty() {
        diagnostics.push(Diagnostic {
            code: "DECOUPLE:SUMMARY".into(),
            severity: Severity::Info,
            message: format!("placed {} decoupling capacitor(s)", caps.len()),
            entities: caps.iter().map(|c| c.refdes.clone()).collect(),
            hint: None,
        });
    }

    (components, caps, diagnostics)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_id_is_stable() {
        let a = deterministic_id("U1:VDD");
        let b = deterministic_id("U1:VDD");
        assert_eq!(a, b);
        assert_ne!(a, deterministic_id("U1:GND"));
    }

    #[test]
    fn pin_id_is_string_newtype() {
        let id = PinId(deterministic_id("seed"));
        assert_eq!(id.0.len(), 36);
    }

    #[test]
    fn pwr_fixed_sets_all_fields() {
        let p = Pin::build("DVDD").pwr_fixed(1.1.volt(), 0.1.amp()).pin();
        assert!(matches!(p.role(), Role::PowerIn));
        assert!(p.decouple());
        assert!((p.power_spec().v_nom.unwrap().as_base() - 1.1).abs() < 1e-9);
        assert!((p.power_spec().v_min.as_base() - 1.1).abs() < 1e-9);
        assert!((p.power_spec().v_max.as_base() - 1.1).abs() < 1e-9);
    }

    #[test]
    fn pwr_leaves_v_nom_none() {
        let p = Pin::build("IOVDD")
            .pwr(1.8.volt(), 3.3.volt(), 0.1.amp())
            .pin();
        assert!(p.power_spec().v_nom.is_none());
    }

    #[test]
    fn nominal_chain_sets_v_nom() {
        let p = Pin::build("VBAT")
            .pwr(3.0.volt(), 3.6.volt(), 0.3.amp())
            .nominal(3.3.volt())
            .pin();
        assert!((p.power_spec().v_nom.unwrap().as_base() - 3.3).abs() < 1e-9);
    }

    #[test]
    fn physical_fields_round_trip() {
        let p = Pin::build("GPIO")
            .pos(1.0, 2.0)
            .rotation(90.0)
            .length(2.54)
            .dio();
        assert_eq!(p.pos(), Some((1.0, 2.0)));
        assert_eq!(p.rotation(), Some(90.0));
        assert_eq!(p.length(), Some(2.54));
    }

    #[test]
    fn pin_ref_and_handle() {
        pub const TEST_PIN: PinRef = PinRef("TEST");
        let handle = ComponentHandle(3).pin(TEST_PIN);
        assert_eq!(handle.component, 3);
        assert_eq!(handle.pin, "TEST");
    }

    struct TestPart;

    impl Component for TestPart {
        fn pins(&self) -> &[Pin] {
            static PINS: std::sync::OnceLock<Vec<Pin>> = std::sync::OnceLock::new();
            PINS.get_or_init(|| vec![Pin::build("VDD").pwr_fixed(3.3.volt(), 0.1.amp()).pin()])
        }
    }

    #[test]
    fn board_add_returns_handle() {
        let mut board = Board::new();
        let h = board.add("U1", TestPart);
        assert_eq!(h.0, 0);
    }

    #[test]
    fn board_connect_records_connection() {
        struct TwoPins;
        impl Component for TwoPins {
            fn pins(&self) -> &[Pin] {
                static PINS: std::sync::OnceLock<Vec<Pin>> = std::sync::OnceLock::new();
                PINS.get_or_init(|| {
                    vec![
                        Pin::build("A").pwr_fixed(3.3.volt(), 0.1.amp()).pin(),
                        Pin::build("B").pwr_fixed(3.3.volt(), 0.1.amp()).pin(),
                    ]
                })
            }
        }
        impl TwoPins {
            pub const A: PinRef = PinRef("A");
            pub const B: PinRef = PinRef("B");
        }

        let mut board = Board::new();
        let u1 = board.add("U1", TwoPins);
        let _ = board.connect(u1.pin(TwoPins::A), u1.pin(TwoPins::B));
        let report = board.compile().unwrap();
        assert_eq!(report.board.nets.len(), 1);
    }

    #[test]
    fn empty_board_compiles() {
        let board = Board::new();
        let report = board.compile().unwrap();
        assert_eq!(report.board.components.len(), 0);
        assert_eq!(report.board.nets.len(), 0);
    }
}
