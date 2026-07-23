//! The mutable board builder.
//!
//! [`Board`] accumulates components and connections at design time.  Once the
//! design is complete, the board can be compiled using
//! [`copperleaf_compile::run`](https://docs.rs/copperleaf-compile).

use crate::{
    CompileError, Component, ComponentMeta, Constraint, Net, NetIdx, Pad, Pin,
    net::NetHandle,
    pin::{PinHandle, PinId, PinRef, RawConnection},
    stackup::Stackup,
    units::{Diagnostic, Qty, Severity, Volt},
    util::deterministic_id,
};

#[derive(Clone, Debug)]
pub struct CompiledComponent {
    pub refdes: String,
    pub meta: ComponentMeta,
    pub pins: Vec<Pin>,
    /// Mechanical (non-electrical) pads belonging to the component's footprint.
    pub mechanical: Vec<Pad>,
    pub constraints: Vec<Constraint>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Connection {
    pub component: usize,
    pub pin: String,
    pub net: NetIdx,
}

/// An immutable structure representing a finished [`Board`](crate::Board) that is ready for export.
#[derive(Clone, Debug)]
pub struct CompiledBoard {
    pub components: Vec<CompiledComponent>,
    pub nets: Vec<Net>,
    pub connections: Vec<Connection>,
    pub constraints: Vec<Constraint>,
    /// Board width in millimetres.
    pub width: f64,
    /// Board height in millimetres.
    pub height: f64,
    /// PCB layer stackup defining the physical layer structure.
    pub stackup: Stackup,
}

/// Precomputed connectivity index for a [`CompiledBoard`].
///
/// Maps component/pin pairs to their owning net and tracks which pins are
/// connected.  Built once during compilation and consumed by ERC and emitters.
#[derive(Clone, Debug)]
pub struct BoardView<'a> {
    /// Reference to the underlying board.
    pub board: &'a CompiledBoard,
    /// Map from (component_index, pin_index) to the owning net.
    pub net_of: std::collections::HashMap<(usize, usize), NetIdx>,
    /// Set of (component_index, pin_index) pairs that are connected to a net.
    pub connected: std::collections::HashSet<(usize, usize)>,
}

/// Handle to a component instance on a [`Board`].
#[derive(Clone, Copy, Debug)]
pub struct ComponentHandle(pub usize);

pub struct ComponentEntry {
    pub name: String,
    pub component: Box<dyn Component>,
}

#[derive(Clone, Debug, Default)]
pub struct RawNetOverride {
    pub voltage: Option<Qty<Volt>>,
    pub name: Option<String>,
}

/// Top level structure representing the PCB being designed.
pub struct Board {
    name: String,
    pub components: Vec<ComponentEntry>,
    pub connections: Vec<RawConnection>,
    pub net_overrides: std::collections::BTreeMap<usize, RawNetOverride>,
    pub next_edge: usize,
    /// Each entry is `(override_key, PinHandle)`.  The override key is
    /// allocated from `next_edge` so it never collides with connection edges.
    pub single_pin_nets: Vec<(usize, PinHandle)>,
    /// Board width in millimetres (default 100.0).
    width: f64,
    /// Board height in millimetres (default 80.0).
    height: f64,
    /// PCB layer stackup (default: standard 2‑layer FR‑4).
    stackup: Stackup,
}

impl CompiledComponent {
    /// Build a compiled component from a refdes and any [`Component`] impl,
    /// assigning deterministic pin IDs.
    pub fn from_component(refdes: &str, component: &dyn Component) -> Self {
        let pins: Vec<Pin> = component
            .pins()
            .iter()
            .map(|p| {
                let seed = format!("{}:{}", refdes, p.name());
                p.clone().with_id(PinId(deterministic_id(&seed)))
            })
            .collect();
        Self {
            refdes: refdes.to_owned(),
            meta: component.meta().clone(),
            pins,
            constraints: component.constraints(),
            mechanical: component.mechanical().to_vec(),
        }
    }

    /// Minimal constructor for tests.  Returns a component with empty metadata
    /// and mechanical pad list.
    pub fn test(refdes: &str, pins: Vec<Pin>) -> Self {
        Self {
            refdes: refdes.to_owned(),
            meta: ComponentMeta::default(),
            pins,
            mechanical: vec![],
            constraints: vec![],
        }
    }

    /// Same as [`test`] but with constraints.
    pub fn test_with(refdes: &str, pins: Vec<Pin>, constraints: Vec<Constraint>) -> Self {
        Self {
            refdes: refdes.to_owned(),
            meta: ComponentMeta::default(),
            pins,
            mechanical: vec![],
            constraints,
        }
    }
}

impl CompiledBoard {
    /// Get a reference to the net at the given index.
    pub fn net(&self, idx: NetIdx) -> &Net {
        &self.nets[idx.0]
    }

    /// Find a net by name, returning its index if present.
    pub fn find_net(&self, name: &str) -> Option<NetIdx> {
        self.nets.iter().position(|n| n.name == name).map(NetIdx)
    }
}

impl<'a> BoardView<'a> {
    /// Build a board view from a compiled board.
    pub fn new(board: &'a CompiledBoard) -> Self {
        let mut net_of = std::collections::HashMap::new();
        let mut connected = std::collections::HashSet::new();
        for conn in &board.connections {
            let comp = &board.components[conn.component];
            if let Some(pin_idx) = comp.pins.iter().position(|p| p.name() == conn.pin) {
                net_of.insert((conn.component, pin_idx), conn.net);
                connected.insert((conn.component, pin_idx));
            }
        }
        Self {
            board,
            net_of,
            connected,
        }
    }
}

impl ComponentHandle {
    /// Create a [`PinHandle`] for a pin on this component.
    pub fn pin(&self, pin: PinRef) -> PinHandle {
        PinHandle {
            component: self.0,
            pin: pin.0,
        }
    }
}

impl Board {
    /// Creates a new, unpopulated [`Board`].
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            components: Vec::new(),
            connections: Vec::new(),
            net_overrides: std::collections::BTreeMap::new(),
            single_pin_nets: Vec::new(),
            next_edge: 0,
            width: 100.0,
            height: 80.0,
            stackup: Stackup::two_layer(),
        }
    }

    /// Set the board outline dimensions in millimetres.
    pub fn set_dimensions(&mut self, width: f64, height: f64) {
        self.width = width;
        self.height = height;
    }

    /// Board width in millimetres.
    pub fn width(&self) -> f64 {
        self.width
    }

    /// Board height in millimetres.
    pub fn height(&self) -> f64 {
        self.height
    }

    /// Set the PCB layer stackup.
    pub fn set_stackup(&mut self, stackup: Stackup) {
        self.stackup = stackup;
    }

    /// Reference to the current PCB layer stackup.
    pub fn stackup(&self) -> &Stackup {
        &self.stackup
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
        self.connections.push(RawConnection { from, to, id: edge });
        self.net_overrides.insert(edge, RawNetOverride::default());
        Ok(NetHandle { id: edge })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Register a single-pin net (e.g. a lone power pin needing a named net).
    ///
    /// Unlike [`Board::connect`], this does not create a connection edge;
    /// the single-pin net is represented directly during compilation.
    ///
    /// The returned [`NetHandle`] carries an id allocated from the same counter
    /// as [`Board::connect`] so keys never collide regardless of call order.
    pub fn net(&mut self, pin: PinHandle) -> Result<NetHandle, CompileError> {
        if let Some(diag) = self.validate_pin(&pin) {
            return Err(CompileError::new(vec![diag]));
        }
        let id = self.next_edge;
        self.next_edge += 1;
        self.single_pin_nets.push((id, pin));
        self.net_overrides.insert(id, RawNetOverride::default());
        Ok(NetHandle { id })
    }

    /// Set an explicit voltage override for a net returned by [`Board::connect`]
    /// or [`Board::net`].
    pub fn set_net_voltage(&mut self, handle: NetHandle, v: Qty<Volt>) {
        if let Some(ov) = self.net_overrides.get_mut(&handle.id) {
            ov.voltage = Some(v);
        }
    }

    /// Set an explicit name override for a net returned by [`Board::connect`]
    /// or [`Board::net`].
    pub fn set_net_name(&mut self, handle: NetHandle, name: &str) {
        if let Some(ov) = self.net_overrides.get_mut(&handle.id) {
            ov.name = Some(name.to_owned());
        }
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

    /// Validate that every [`PinHandle`] in a connection refers to an existing
    /// component and pin.
    pub fn validate_connections(&self) -> Result<(), CompileError> {
        for conn in &self.connections {
            if let Some(diag) = self.validate_pin(&conn.from) {
                return Err(CompileError::new(vec![diag]));
            }
            if let Some(diag) = self.validate_pin(&conn.to) {
                return Err(CompileError::new(vec![diag]));
            }
        }
        Ok(())
    }
}
