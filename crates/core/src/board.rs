//! The mutable board builder.
//!
//! [`Board`] accumulates components and connections at design time.  Once the
//! design is complete, [`Board::compile`] consumes the builder and runs the
//! full compilation pipeline (see [`compile`](crate::compile)) to produce an
//! immutable [`CompileReport`](crate::CompileReport).

use crate::{
    Component, Constraint, Net, NetId, Pin,
    compile::{CompileError, CompileReport},
    net::NetHandle,
    pin::{PinHandle, PinRef, RawConnection},
    units::{Diagnostic, Qty, Severity, Volt},
};

/// A mechanical pad — not an electrical pin — e.g. a mounting hole, fiducial,
/// or paste-only stencil aperture on an exposed pad.
#[derive(Clone, Debug)]
pub struct MechanicalPad {
    /// KiCad pad number. `"None"` for mounting holes / fiducials, `""` for
    /// unnamed pads (e.g. paste stencil apertures).
    pub number: String,
    /// Position in millimetres, relative to the footprint origin.
    pub pos: (f64, f64),
    /// Pad width in millimetres (X dimension).
    pub width: f64,
    /// Pad height in millimetres (Y dimension).
    pub height: f64,
    /// KiCad pad type: `np_thru_hole`, `thru_hole`, or `smd`.
    pub pad_type: String,
    /// Pad shape: `circle`, `rect`, `oval`, or `roundrect`.
    pub pad_shape: String,
    /// Roundrect corner radius ratio (only for `roundrect` shape).
    pub roundrect_rratio: Option<f64>,
    /// Copper layers, e.g. `"*.Cu *.Mask"` or `"F.Paste"`.
    pub layers: Option<String>,
    /// Drill diameter in millimetres.
    pub drill: f64,
}

#[derive(Clone, Debug)]
pub struct CompiledComponent {
    pub refdes: String,
    pub pins: Vec<Pin>,
    pub constraints: Vec<Constraint>,
    pub symbol: Option<String>,
    pub footprint: Option<String>,
    /// Mechanical (non-electrical) pads belonging to the component's footprint.
    pub mechanical: Vec<MechanicalPad>,
    /// Datasheet URL carried through to the symbol library.
    pub datasheet: Option<String>,
    /// Human-readable description carried through to library metadata.
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Connection {
    pub component: usize,
    pub pin: String,
    pub net: NetId,
}

/// An immutable structure representing a finished [`Board`](crate::Board) that is ready for export.
#[derive(Clone, Debug)]
pub struct CompiledBoard {
    pub components: Vec<CompiledComponent>,
    pub nets: Vec<Net>,
    pub connections: Vec<Connection>,
    pub constraints: Vec<Constraint>,
}

/// Handle to a component instance on a [`Board`].
#[derive(Clone, Copy, Debug)]
pub struct ComponentHandle(pub usize);

/// Top level structure representing the PCB being designed.
pub struct Board {
    name: String,
    pub(crate) components: Vec<ComponentEntry>,
    pub(crate) connections: Vec<RawConnection>,
    pub(crate) net_overrides: Vec<RawNetOverride>,
    pub(crate) next_edge: usize,
}

pub(crate) struct ComponentEntry {
    pub(crate) name: String,
    pub(crate) component: Box<dyn Component>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct RawNetOverride {
    pub(crate) voltage: Option<Qty<Volt>>,
    pub(crate) name: Option<String>,
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
            net_overrides: Vec::new(),
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
        self.net_overrides.push(RawNetOverride::default());
        Ok(NetHandle { edge })
    }

    /// Compiles this board into a [`CompileReport`] in a single pass.
    ///
    /// The board is consumed and run through lowering, ERC validation, and
    /// generation.  The resulting [`CompiledBoard`](crate::CompiledBoard) is
    /// constructed exactly once and never mutated afterwards.
    pub fn compile(self) -> Result<CompileReport, CompileError> {
        self.validate_connections()?;
        crate::compile::run(self)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set an explicit voltage override for a net returned by [`Board::connect`].
    pub fn set_net_voltage(&mut self, handle: NetHandle, v: Qty<Volt>) {
        if let Some(ov) = self.net_overrides.get_mut(handle.edge) {
            ov.voltage = Some(v);
        }
    }

    /// Set an explicit name override for a net returned by [`Board::connect`].
    pub fn set_net_name(&mut self, handle: NetHandle, name: &str) {
        if let Some(ov) = self.net_overrides.get_mut(handle.edge) {
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
    fn validate_connections(&self) -> Result<(), CompileError> {
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
