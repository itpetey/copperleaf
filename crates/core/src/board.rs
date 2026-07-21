//! The mutable board builder.
//!
//! [`Board`] accumulates components and connections at design time.  Once the
//! design is complete, the board can be compiled using
//! [`copperleaf_compile::run`](https://docs.rs/copperleaf-compile).

use crate::{
    CompileError, Component, ComponentMeta, Constraint, Net, NetId, Pad, Pin,
    net::NetHandle,
    pin::{PinHandle, PinId, PinRef, RawConnection},
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
    /// Board width in millimetres.
    pub width: f64,
    /// Board height in millimetres.
    pub height: f64,
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
    pub net_overrides: Vec<RawNetOverride>,
    pub(crate) next_edge: usize,
    /// Board width in millimetres (default 100.0).
    width: f64,
    /// Board height in millimetres (default 80.0).
    height: f64,
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
            width: 100.0,
            height: 80.0,
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
