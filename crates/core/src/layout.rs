//! Physical layout directives — solver input vocabulary and the layout IR.
//!
//! [`LayoutConstraint`] is the single representation of physical placement and
//! routing directives, attaching at component, net, and board level alongside
//! the electrical [`Constraint`](crate::Constraint) fields.

use crate::units::{Meter, Qty, Volt};

// ---------------------------------------------------------------------------
// Supporting vocabulary types
// ---------------------------------------------------------------------------

/// Which side of the board a component, keepout, or zone lives on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoardSide {
    Front,
    Back,
}

/// A geometric region on the board, used by keepout and future constraints.
#[derive(Clone, Debug, PartialEq)]
pub enum Region {
    /// Axis-aligned rectangle in millimetres, `(x1, y1)` to `(x2, y2)`.
    Rect { x1: f64, y1: f64, x2: f64, y2: f64 },
    /// Circle in millimetres, centre `(cx, cy)` with radius `r`.
    Circle { cx: f64, cy: f64, r: f64 },
}

/// A set of copper layers referenced by index.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct LayerSet {
    pub layers: Vec<usize>,
}

/// Reference to a component targeted by a placement directive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaceTarget {
    /// A component by its index into `CompiledBoard.components`.
    Component(usize),
}

// ---------------------------------------------------------------------------
// LayoutConstraint — physical directives
// ---------------------------------------------------------------------------

/// Physical layout directives consumed by the solver and layout DRC.
///
/// These live on [`crate::CompiledComponent::layout`],
/// [`crate::Net::layout`], and [`crate::CompiledBoard::layout`],
/// mirroring the electrical [`Constraint`](crate::Constraint) fields.
#[derive(Clone, Debug)]
pub enum LayoutConstraint {
    /// Net width and clearance rules for the solver and backend net-class
    /// emission.
    NetClass {
        min_width: Qty<Meter>,
        clearance: Qty<Meter>,
    },
    /// Minimum creepage distance for a given working voltage.
    Creepage { min: Qty<Meter>, voltage: Qty<Volt> },
    /// Fixed placement — position (mm, mm), rotation (degrees), board side.
    PlaceAt {
        pos: (f64, f64),
        rotation: f64,
        side: BoardSide,
    },
    /// Place this component near a target component, within `max_radius`.
    PlaceNear {
        target: PlaceTarget,
        max_radius: Qty<Meter>,
    },
    /// Keep this component on the same board side as others in the group.
    SameSide { group: String },
    /// Exclude a region from placement and routing on the given layers.
    Keepout { region: Region, layers: LayerSet },
    /// Dedicate a copper layer as a plane / pour (net-level only).  The
    /// net is excluded from routing and emitted as a board-outline zone.
    Plane { layer: usize },
}

// ---------------------------------------------------------------------------
// Layout IR — placed into core so backends can consume it without depending
// on `copperleaf-layout`.  Phase 2 populates this.
// ---------------------------------------------------------------------------

/// Index into [`CompiledBoard::nets`](crate::CompiledBoard::nets).
pub type LayoutNetIdx = crate::NetIdx;

/// The solved physical layout — placements, tracks, vias, and zones.
///
/// No type from the solver crate (`topola`) appears here; this is the
/// format boundary the solver writes and backends read.
#[derive(Clone, Debug)]
pub struct Layout {
    pub placements: Vec<Placement>,
    pub tracks: Vec<Track>,
    pub vias: Vec<Via>,
    pub zones: Vec<Zone>,
}

/// One component's physical placement.
#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    /// Index into `CompiledBoard.components`.
    pub component: usize,
    /// Position in millimetres.
    pub at: (f64, f64),
    /// Rotation in degrees.
    pub rotation: f64,
    /// Board side.
    pub side: BoardSide,
}

/// One routed track segment on a specific net and copper layer.
#[derive(Clone, Debug)]
pub struct Track {
    pub net: LayoutNetIdx,
    pub layer: usize,
    pub width: Qty<Meter>,
    pub path: Vec<(f64, f64)>,
}

/// One plated via between two layers.
#[derive(Clone, Debug)]
pub struct Via {
    pub net: LayoutNetIdx,
    pub at: (f64, f64),
    pub drill: Qty<Meter>,
    pub diameter: Qty<Meter>,
    /// (start_layer, end_layer) inclusive.
    pub layers: (usize, usize),
}

/// One copper pour / zone on a single layer, typically for a plane net.
#[derive(Clone, Debug, PartialEq)]
pub struct Zone {
    pub net: LayoutNetIdx,
    pub layer: usize,
    pub outline: Vec<(f64, f64)>,
}
