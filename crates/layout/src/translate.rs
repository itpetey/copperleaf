//! Translate [`CompiledBoard`] + layout constraints into the adapter input
//! model.

// Note: The unused code here will be required when we implement routing
#![allow(dead_code)]

use copperleaf::{
    CompiledBoard, LayoutConstraint, Net, NetClass, NetIdx,
    pin::{PadType, resolve_pad},
    stackup::{Stackup, StackupLayer},
};

use crate::LayoutError;

/// A resolved pad with geometry and net association.
#[derive(Clone, Debug)]
pub struct PadInfo {
    /// Position in millimetres relative to the footprint origin.
    pub pos: (f64, f64),
    /// Pad width in millimetres.
    pub width: f64,
    /// Pad height in millimetres.
    pub height: f64,
    /// SMD or through-hole.
    pub pad_type: PadType,
    /// Net this pad is connected to, if any.
    pub net: Option<NetIdx>,
}

/// One component's translated information.
#[derive(Clone, Debug)]
pub struct ComponentInfo {
    /// Reference designator, e.g. `"C1"`.
    pub refdes: String,
    /// Resolved pad geometry for every pin (one pad per pin).
    pub pads: Vec<PadInfo>,
    /// Per-component layout directives (PlaceAt, PlaceNear, etc.).
    pub layout: Vec<LayoutConstraint>,
}

/// A net as seen by the adapter.
#[derive(Clone, Debug)]
pub struct NetInfo {
    /// Net name, e.g. `"V3V3"`.
    pub name: String,
    /// Resolved net class (from `LayoutConstraint::NetClass` directives).
    pub class: NetClass,
    /// Per-net layout directives (Plane, etc.).
    pub layout: Vec<LayoutConstraint>,
}

/// One copper layer.
#[derive(Clone, Debug)]
pub struct LayerInfo {
    /// The layer index in the stackup (0 = top).
    pub index: usize,
    /// Layer name, e.g. `"F.Cu"`, `"B.Cu"`.
    pub name: String,
}

/// The input model passed to the topola adapter.
///
/// This is the format boundary: everything the adapter needs to build a
/// Topola board and drive autoplacement, using only copperleaf types.
#[derive(Clone, Debug)]
// fields consumed by topola_adapter
pub struct AdapterInput {
    /// Board outline polygon vertices in mm: `Vec<(x_mm, y_mm)>`.
    pub outline: Vec<(f64, f64)>,
    /// Every component on the board with its resolved pads.
    pub components: Vec<ComponentInfo>,
    /// Every net on the board with its resolved class and layout directives.
    pub nets: Vec<NetInfo>,
    /// Copper layer definitions.
    pub layers: Vec<LayerInfo>,
}

/// Translate a compiled board into the adapter input model.
///
/// # Errors
///
/// Returns [`LayoutError::NoBoardOutline`] if both `width` and `height` are
/// zero (no outline defined).
pub fn translate_board(board: &CompiledBoard) -> Result<AdapterInput, LayoutError> {
    if board.width <= 0.0 && board.height <= 0.0 {
        return Err(LayoutError::NoBoardOutline);
    }

    let outline = board_outline(board);
    let layers = translate_layers(&board.stackup);
    let nets = translate_nets(&board.nets);
    let components = translate_components(board)?;

    Ok(AdapterInput {
        outline,
        components,
        nets,
        layers,
    })
}

/// Derive a rectangular board outline from width/height.
fn board_outline(board: &CompiledBoard) -> Vec<(f64, f64)> {
    let w = board.width;
    let h = board.height;
    vec![(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)]
}

/// Build a map from `(component_index, pin_index)` to `NetIdx`.
fn build_pin_net_map(board: &CompiledBoard) -> std::collections::HashMap<(usize, usize), NetIdx> {
    let mut map = std::collections::HashMap::new();
    for conn in &board.connections {
        let comp = &board.components[conn.component];
        if let Some(pin_idx) = comp.pins.iter().position(|p| p.name() == conn.pin) {
            map.insert((conn.component, pin_idx), conn.net);
        }
    }
    map
}

/// Resolve `NetClass` from a net's `LayoutConstraint` list.
///
/// The first `NetClass` directive wins; if none is found the existing
/// `Net.class` is kept (which may already have been set during compilation).
fn resolve_net_class(layout: &[LayoutConstraint], default: &NetClass) -> NetClass {
    for constraint in layout {
        if let LayoutConstraint::NetClass {
            min_width,
            clearance,
        } = constraint
        {
            return NetClass {
                min_width: Some(*min_width),
                clearance: Some(*clearance),
            };
        }
    }
    default.clone()
}

/// Translate every component and resolve pad-to-net associations.
fn translate_components(board: &CompiledBoard) -> Result<Vec<ComponentInfo>, LayoutError> {
    if board.components.is_empty() {
        return Err(LayoutError::NoComponents);
    }

    // Build a map from (component_idx, pin_idx) → NetIdx for resolving pad nets.
    let pin_net_map = build_pin_net_map(board);

    let components: Vec<ComponentInfo> = board
        .components
        .iter()
        .enumerate()
        .map(|(comp_idx, comp)| {
            let pads: Vec<PadInfo> = comp
                .pins
                .iter()
                .enumerate()
                .map(|(pin_idx, pin)| {
                    let resolved = resolve_pad(pin, pin_idx);
                    let net = pin_net_map.get(&(comp_idx, pin_idx)).copied();
                    PadInfo {
                        pos: resolved.pos,
                        width: resolved.width,
                        height: resolved.height,
                        pad_type: resolved.pad_type,
                        net,
                    }
                })
                .collect();

            ComponentInfo {
                refdes: comp.refdes.clone(),
                pads,
                layout: comp.layout.clone(),
            }
        })
        .collect();

    Ok(components)
}

/// Extract copper layers from the stackup.
fn translate_layers(stackup: &Stackup) -> Vec<LayerInfo> {
    let mut layers = Vec::new();

    // The stackup's `layers()` returns an ordered list.  Copper layers are
    // interleaved with dielectrics; we only keep the copper ones.  We need
    // consistent naming for the adapter.
    for (i, layer) in stackup.layers.iter().enumerate() {
        match layer {
            StackupLayer::Copper { name, .. } => {
                layers.push(LayerInfo {
                    index: i,
                    name: name.into(),
                });
            }
            StackupLayer::Dielectric { .. } => {}
        }
    }

    layers
}

/// Translate nets: resolve `NetClass` from layout constraints and collect
/// per-net layout directives.
fn translate_nets(nets: &[Net]) -> Vec<NetInfo> {
    nets.iter()
        .map(|net| {
            // Resolve NetClass: explicit `LayoutConstraint::NetClass` on
            // the net wins; absent directives leave the default class.
            let resolved_class = resolve_net_class(&net.layout, &net.class);

            NetInfo {
                name: net.name.clone(),
                class: resolved_class,
                layout: net.layout.clone(),
            }
        })
        .collect()
}
