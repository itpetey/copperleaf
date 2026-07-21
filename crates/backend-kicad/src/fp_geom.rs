//! Shared footprint pad geometry for the KiCad emitters.
//!
//! Builds a single, data-driven pad list from a [`CompiledComponent`] (pin pad
//! geometry, thermal vias, and mechanical pads) so that the standalone
//! `.kicad_mod` library files and the footprints embedded in the `.kicad_pcb`
//! are always identical.

use copperleaf::{CompiledComponent, MechanicalPad, Pin};

use crate::{
    common::{format_float, format_grid_float},
    sexpr::Sexpr,
};

/// Margin added around the pad bounding box to form the fab outline, in mm.
const BODY_MARGIN: f64 = 0.25;
/// Courtyard offset from the fab outline, in mm.
const COURTYARD_OFFSET: f64 = 0.25;
/// Default drill for through-hole pads, in mm.
pub const DEFAULT_DRILL: f64 = 0.762;
/// Default pad size when the part data carries no geometry, in mm.
pub const DEFAULT_PAD_SIZE: f64 = 1.524;
/// Default layers for through-hole pads.
pub const PTH_LAYERS: &str = "*.Cu *.Mask";
/// Silkscreen offset from the fab outline, in mm.
const SILK_OFFSET: f64 = 0.11;
/// Default layers for SMD pads.
pub const SMD_LAYERS: &str = "F.Cu F.Mask F.Paste";

/// One footprint pad with fully resolved geometry.
#[derive(Clone, Debug)]
pub struct PadGeom {
    pub number: String,
    pub pos: (f64, f64),
    pub rotation: f64,
    pub width: f64,
    pub height: f64,
    /// KiCad pad type: `smd`, `thru_hole`, or `np_thru_hole`.
    pub pad_type: String,
    /// Pad shape: `rect`, `roundrect`, `circle`, or `oval`.
    pub shape: String,
    pub roundrect_rratio: Option<f64>,
    /// Space-separated layer list, e.g. `"F.Cu F.Mask F.Paste"`.
    pub layers: String,
    pub drill: Option<f64>,
    pub solder_mask_margin: Option<f64>,
    /// Index into the component's pin list (used for net association in the
    /// PCB); `None` for thermal vias and mechanical pads.
    pub pin_index: Option<usize>,
}

/// Auto-layout position for pins without physical pad data: a single
/// horizontal row at 2.54 mm pitch with pad 1 at the origin (KLC F7.2).
pub fn auto_pad_pos(index: usize) -> (f64, f64) {
    (index as f64 * 2.54, 0.0)
}

/// KiCad footprint `attr` value for the pad set: `smd` when any pad is
/// surface-mount (mixed footprints included), `through_hole` only when all
/// pads are through-hole (KLC F6.1).
pub fn footprint_attr(pads: &[PadGeom]) -> &'static str {
    if pads.iter().any(|p| p.pad_type == "smd") || pads.is_empty() {
        "smd"
    } else {
        "through_hole"
    }
}

/// Build an `(fp_circle ...)` with coordinates on the 0.01 mm grid.
pub fn fp_circle(
    center: (f64, f64),
    radius: f64,
    layer: &str,
    width: f64,
    uuid_seed: Option<&str>,
) -> Sexpr {
    let mut children = vec![
        Sexpr::atom("fp_circle"),
        Sexpr::list([
            Sexpr::atom("center"),
            Sexpr::atom(format_grid_float(center.0)),
            Sexpr::atom(format_grid_float(center.1)),
        ]),
        Sexpr::list([
            Sexpr::atom("end"),
            Sexpr::atom(format_grid_float(center.0 + radius)),
            Sexpr::atom(format_grid_float(center.1)),
        ]),
        Sexpr::list([Sexpr::atom("layer"), Sexpr::atom(layer)]),
        Sexpr::list([Sexpr::atom("width"), Sexpr::atom(format_float(width, 2))]),
    ];
    if let Some(seed) = uuid_seed {
        children.push(Sexpr::list([
            Sexpr::atom("uuid"),
            Sexpr::str(crate::sexpr::deterministic_uuid(&format!(
                "{}:circle:{}:{}:{}",
                seed, center.0, center.1, layer
            ))),
        ]));
    }
    Sexpr::list(children)
}

/// Build an `(fp_line ...)` with coordinates on the 0.01 mm grid.
pub fn fp_line(
    start: (f64, f64),
    end: (f64, f64),
    layer: &str,
    width: f64,
    uuid_seed: Option<&str>,
) -> Sexpr {
    let mut children = vec![
        Sexpr::atom("fp_line"),
        Sexpr::list([
            Sexpr::atom("start"),
            Sexpr::atom(format_grid_float(start.0)),
            Sexpr::atom(format_grid_float(start.1)),
        ]),
        Sexpr::list([
            Sexpr::atom("end"),
            Sexpr::atom(format_grid_float(end.0)),
            Sexpr::atom(format_grid_float(end.1)),
        ]),
        Sexpr::list([Sexpr::atom("layer"), Sexpr::atom(layer)]),
        Sexpr::list([Sexpr::atom("width"), Sexpr::atom(format_float(width, 2))]),
    ];
    if let Some(seed) = uuid_seed {
        children.push(Sexpr::list([
            Sexpr::atom("uuid"),
            Sexpr::str(crate::sexpr::deterministic_uuid(&format!(
                "{}:{}:{}:{}:{}:{}",
                seed, start.0, start.1, end.0, end.1, layer
            ))),
        ]));
    }
    Sexpr::list(children)
}

/// Build an `(fp_text ...)` node.
pub fn fp_text(kind: &str, value: &str, pos: (f64, f64), layer: &str) -> Sexpr {
    Sexpr::list([
        Sexpr::atom("fp_text"),
        Sexpr::atom(kind),
        Sexpr::str(value),
        Sexpr::list([
            Sexpr::atom("at"),
            Sexpr::atom(format_grid_float(pos.0)),
            Sexpr::atom(format_grid_float(pos.1)),
        ]),
        Sexpr::list([Sexpr::atom("layer"), Sexpr::atom(layer)]),
        Sexpr::list([
            Sexpr::atom("effects"),
            Sexpr::list([
                Sexpr::atom("font"),
                Sexpr::list([Sexpr::atom("size"), Sexpr::atom("1.0"), Sexpr::atom("1.0")]),
                Sexpr::list([Sexpr::atom("thickness"), Sexpr::atom("0.15")]),
            ]),
        ]),
    ])
}

/// Build the `(model ...)` 3D-model reference for a footprint.
///
/// If `model_path` is `Some`, uses that exact path. Otherwise follows the KLC
/// F9.3 convention of referencing the model by name even when the `.step` file
/// does not (yet) exist; KiCad silently ignores missing models.
///
/// `offset` is the model offset in millimetres (x, y, z) relative to the
/// footprint origin.  `rotation` is the model rotation in degrees around the
/// X, Y, and Z axes (e.g. `(0.0, 0.0, 90.0)` to rotate 90° around Z).
pub fn model_sexpr(
    fp_name: &str,
    model_path: Option<&str>,
    offset: (f64, f64, f64),
    rotation: (f64, f64, f64),
) -> Sexpr {
    let path = match model_path {
        Some(p) => p.to_string(),
        None => format!("${{KICAD10_3DMODEL_DIR}}/{}.step", fp_name),
    };
    let (ox, oy, oz) = offset;
    let (rx, ry, rz) = rotation;
    Sexpr::list([
        Sexpr::atom("model"),
        Sexpr::str(&path),
        Sexpr::list([
            Sexpr::atom("offset"),
            Sexpr::list([
                Sexpr::atom("xyz"),
                Sexpr::atom(&format!("{ox}")),
                Sexpr::atom(&format!("{oy}")),
                Sexpr::atom(&format!("{oz}")),
            ]),
        ]),
        Sexpr::list([
            Sexpr::atom("scale"),
            Sexpr::list([
                Sexpr::atom("xyz"),
                Sexpr::atom("1"),
                Sexpr::atom("1"),
                Sexpr::atom("1"),
            ]),
        ]),
        Sexpr::list([
            Sexpr::atom("rotate"),
            Sexpr::list([
                Sexpr::atom("xyz"),
                Sexpr::atom(&format!("{rx}")),
                Sexpr::atom(&format!("{ry}")),
                Sexpr::atom(&format!("{rz}")),
            ]),
        ]),
    ])
}

/// Normalise the footprint anchor:
///
/// - Footprints with any SMD pad are recentred so the pad bounding box is
///   centred on the origin (KLC F6.2).
/// - Pure through-hole footprints are translated so pad 1 sits at the origin
///   (KLC F7.2).  Auto-generated rows already start at the origin.
/// - Fully automatic footprints (no explicit positions at all) are left
///   untouched.
pub fn normalise_anchor(pads: &mut Vec<PadGeom>) {
    if pads.is_empty() {
        return;
    }

    let any_explicit = pads.iter().any(|p| {
        p.pin_index.is_some() && p.pos != auto_pad_pos(p.pin_index.unwrap())
            || p.pin_index.is_none()
    });
    if !any_explicit {
        return; // fully automatic row: pad 1 already at the origin
    }

    let any_smd = pads.iter().any(|p| p.pad_type == "smd");
    if any_smd {
        // Recentre on the pad bounding box.
        if let Some((x1, y1, x2, y2)) = pads_extent(pads) {
            let cx = (x1 + x2) / 2.0;
            let cy = (y1 + y2) / 2.0;
            for p in pads.iter_mut() {
                p.pos.0 -= cx;
                p.pos.1 -= cy;
            }
        }
    } else if let Some(anchor) = pads.iter().find(|p| p.pin_index == Some(0)).map(|p| p.pos) {
        // Through-hole: pad 1 at the origin.
        for p in pads.iter_mut() {
            p.pos.0 -= anchor.0;
            p.pos.1 -= anchor.1;
        }
    }
}

/// Build the fab/silk/courtyard outlines plus the pin-1 marker for a pad
/// bounding box `(x1, y1, x2, y2)`.  All coordinates are rounded to the
/// 0.01 mm grid and use KLC-legal line widths.
pub fn outline_sexprs(
    extent: (f64, f64, f64, f64),
    pin1: Option<(f64, f64)>,
    uuid_seed: Option<&str>,
) -> Vec<Sexpr> {
    let (x1, y1, x2, y2) = extent;

    // Fab outline: pad bounding box plus body margin.
    let fx1 = x1 - BODY_MARGIN;
    let fy1 = y1 - BODY_MARGIN;
    let fx2 = x2 + BODY_MARGIN;
    let fy2 = y2 + BODY_MARGIN;

    let mut out = Vec::new();
    for &(start, end) in &outline_segments(fx1, fy1, fx2, fy2) {
        out.push(fp_line(start, end, "F.Fab", 0.1, uuid_seed));
    }

    // Silk outline: offset outside the fab.
    let sx1 = fx1 - SILK_OFFSET;
    let sy1 = fy1 - SILK_OFFSET;
    let sx2 = fx2 + SILK_OFFSET;
    let sy2 = fy2 + SILK_OFFSET;
    for &(start, end) in &outline_segments(sx1, sy1, sx2, sy2) {
        out.push(fp_line(start, end, "F.SilkS", 0.12, uuid_seed));
    }

    // Courtyard: 0.25 mm outside the fab, on the 0.01 mm grid.
    let cx1 = fx1 - COURTYARD_OFFSET;
    let cy1 = fy1 - COURTYARD_OFFSET;
    let cx2 = fx2 + COURTYARD_OFFSET;
    let cy2 = fy2 + COURTYARD_OFFSET;
    for &(start, end) in &outline_segments(cx1, cy1, cx2, cy2) {
        out.push(fp_line(start, end, "F.CrtYd", 0.05, uuid_seed));
    }

    // Pin-1 marker: small circle at the corner nearest pad 1.
    if pin1.is_some() {
        out.push(fp_circle((fx1, fy1), 0.1, "F.SilkS", 0.12, uuid_seed));
    }

    out
}

/// Build a `(pad ...)` S-expression.
///
/// `uuid` is included when `Some`; `net` as `(code, name)` is appended when
/// `Some` (only meaningful inside `.kicad_pcb` files).
pub fn pad_sexpr(pad: &PadGeom, uuid: Option<&str>, net: Option<(usize, &str)>) -> Sexpr {
    let mut children = vec![
        Sexpr::atom("pad"),
        Sexpr::str(&pad.number),
        Sexpr::atom(&pad.pad_type),
        Sexpr::atom(&pad.shape),
    ];

    if let Some(rr) = pad.roundrect_rratio {
        children.push(Sexpr::list([
            Sexpr::atom("roundrect_rratio"),
            Sexpr::atom(format_float(rr, 4)),
        ]));
    }

    let at = if pad.rotation == 0.0 {
        vec![
            Sexpr::atom("at"),
            Sexpr::atom(format_float(pad.pos.0, 4)),
            Sexpr::atom(format_float(pad.pos.1, 4)),
        ]
    } else {
        vec![
            Sexpr::atom("at"),
            Sexpr::atom(format_float(pad.pos.0, 4)),
            Sexpr::atom(format_float(pad.pos.1, 4)),
            Sexpr::atom(format_float(pad.rotation, 2)),
        ]
    };
    children.push(Sexpr::list(at));

    children.push(Sexpr::list([
        Sexpr::atom("size"),
        Sexpr::atom(format_float(pad.width, 4)),
        Sexpr::atom(format_float(pad.height, 4)),
    ]));

    if let Some(drill) = pad.drill {
        children.push(Sexpr::list([
            Sexpr::atom("drill"),
            Sexpr::atom(format_float(drill, 4)),
        ]));
    }

    children.push(Sexpr::list(
        std::iter::once(Sexpr::atom("layers"))
            .chain(pad.layers.split_whitespace().map(Sexpr::atom)),
    ));

    if let Some(smm) = pad.solder_mask_margin {
        children.push(Sexpr::list([
            Sexpr::atom("solder_mask_margin"),
            Sexpr::atom(format_float(smm, 4)),
        ]));
    }

    if let Some(u) = uuid {
        children.push(Sexpr::list([Sexpr::atom("uuid"), Sexpr::str(u)]));
    }

    if let Some((code, name)) = net {
        children.push(Sexpr::list([
            Sexpr::atom("net"),
            Sexpr::atom(code.to_string()),
            Sexpr::str(name),
        ]));
    }

    Sexpr::list(children)
}

/// Bounding box over all pads, accounting for 90°/270° pad rotation.
/// Returns `(x1, y1, x2, y2)`.
pub fn pads_extent(pads: &[PadGeom]) -> Option<(f64, f64, f64, f64)> {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for pad in pads {
        let rot = pad.rotation.rem_euclid(360.0);
        let (w, h) = if (rot - 90.0).abs() < 1.0 || (rot - 270.0).abs() < 1.0 {
            (pad.height, pad.width)
        } else {
            (pad.width, pad.height)
        };
        min_x = min_x.min(pad.pos.0 - w / 2.0);
        max_x = max_x.max(pad.pos.0 + w / 2.0);
        min_y = min_y.min(pad.pos.1 - h / 2.0);
        max_y = max_y.max(pad.pos.1 + h / 2.0);
    }

    if min_x == f64::MAX {
        return None;
    }
    Some((min_x, min_y, max_x, max_y))
}

/// Collect all pads for a component: electrical pins (with thermal vias)
/// followed by mechanical pads.  The anchor is normalised per KLC (see
/// [`normalise_anchor`]).
///
/// Un-numbered pads (thermal vias and mechanical pads) are assigned
/// sequential pad numbers continuing after the last electrical pin so
/// that every pad has a unique number matching its schematic pin.
pub fn pads_from_component(comp: &CompiledComponent) -> Vec<PadGeom> {
    let mut pads: Vec<PadGeom> = Vec::new();

    for (i, pin) in comp.pins.iter().enumerate() {
        pads.push(pad_from_pin(pin, i));
        for via in pin.thermal_vias() {
            pads.push(thermal_via_pad(via.pos, via.drill, via.size));
        }
    }

    for mech in &comp.mechanical {
        pads.push(pad_from_mechanical(mech));
    }

    // Assign sequential pad numbers to any un-numbered pads (thermal vias
    // and mechanical pads) so that every pad can be matched to a schematic
    // pin.  The numbering continues after the highest numeric electrical
    // pin number.
    let max_electrical = pads
        .iter()
        .filter(|p| p.pin_index.is_some())
        .filter_map(|p| p.number.parse::<usize>().ok())
        .max()
        .unwrap_or(0);
    let mut next_num = max_electrical + 1;
    for pad in pads.iter_mut() {
        if pad.number.is_empty() {
            pad.number = next_num.to_string();
            next_num += 1;
        }
    }

    normalise_anchor(&mut pads);
    pads
}

/// Position of pad `"1"` (the first electrical pad), used for the pin-1
/// marker.
pub fn pin1_pos(pads: &[PadGeom]) -> Option<(f64, f64)> {
    pads.iter()
        .filter(|p| p.pin_index.is_some())
        .min_by_key(|p| p.pin_index.unwrap())
        .map(|p| p.pos)
}

/// Resolve the physical pad number for a pin: the explicit `number` when
/// set, otherwise the 1-based pin index.
pub fn pin_number(pin: &Pin, index: usize) -> String {
    pin.number()
        .map(str::to_owned)
        .unwrap_or_else(|| (index + 1).to_string())
}

/// Build an un-numbered through-hole pad for a thermal via.
///
/// Thermal vias live on all copper layers plus the solder mask (KLC F7.4).
pub fn thermal_via_pad(pos: (f64, f64), drill: f64, size: f64) -> PadGeom {
    PadGeom {
        number: String::new(),
        pos,
        rotation: 0.0,
        width: size,
        height: size,
        pad_type: "thru_hole".to_string(),
        shape: "circle".to_string(),
        roundrect_rratio: None,
        layers: "*.Cu *.Mask".to_string(),
        drill: Some(drill),
        solder_mask_margin: None,
        pin_index: None,
    }
}

fn is_through_hole(pad_type: &str) -> bool {
    pad_type == "thru_hole" || pad_type == "np_thru_hole"
}

/// Four line segments forming a rectangle.
fn outline_segments(x1: f64, y1: f64, x2: f64, y2: f64) -> [((f64, f64), (f64, f64)); 4] {
    [
        ((x1, y1), (x2, y1)),
        ((x2, y1), (x2, y2)),
        ((x2, y2), (x1, y2)),
        ((x1, y2), (x1, y1)),
    ]
}

/// Build a pad from a mechanical (non-electrical) pad definition.
fn pad_from_mechanical(mech: &MechanicalPad) -> PadGeom {
    // KiCad writes un-numbered pads as `(pad "" ...)`; normalise the legacy
    // `"None"` marker to an empty number.
    let number = if mech.number.eq_ignore_ascii_case("none") {
        String::new()
    } else {
        mech.number.clone()
    };
    PadGeom {
        number,
        pos: mech.pos,
        rotation: 0.0,
        width: mech.width,
        height: mech.height,
        pad_type: mech.pad_type.clone(),
        shape: mech.pad_shape.clone(),
        roundrect_rratio: mech.roundrect_rratio,
        layers: mech
            .layers
            .clone()
            .unwrap_or_else(|| "*.Cu *.Mask".to_string()),
        drill: if mech.drill > 0.0 {
            Some(mech.drill)
        } else {
            None
        },
        solder_mask_margin: None,
        pin_index: None,
    }
}

/// Build a pad from an electrical pin.
fn pad_from_pin(pin: &Pin, index: usize) -> PadGeom {
    let pos = pin.pos().unwrap_or_else(|| auto_pad_pos(index));
    let pad_type = pin
        .pad_type()
        .unwrap_or(if pin.pos().is_some() {
            "smd"
        } else {
            "thru_hole"
        })
        .to_string();
    let width = pin.width().or(pin.length()).unwrap_or(DEFAULT_PAD_SIZE);
    let height = pin.height().or(pin.length()).unwrap_or(DEFAULT_PAD_SIZE);
    let layers = pin.layers().map(str::to_owned).unwrap_or_else(|| {
        if is_through_hole(&pad_type) {
            PTH_LAYERS.to_string()
        } else {
            SMD_LAYERS.to_string()
        }
    });
    let drill = pin.drill().or(if is_through_hole(&pad_type) {
        Some(DEFAULT_DRILL)
    } else {
        None
    });

    // KLC F7.3: for auto-generated through-hole rows, pad 1 is rectangular
    // and the rest circular.  Explicit vendor geometry is preserved as-is.
    let default_shape = if pin.pos().is_some() || !is_through_hole(&pad_type) {
        "rect"
    } else if index == 0 {
        "rect"
    } else {
        "circle"
    };

    PadGeom {
        number: pin_number(pin, index),
        pos,
        rotation: pin.rotation().unwrap_or(0.0),
        width,
        height,
        pad_type,
        shape: pin.pad_shape().unwrap_or(default_shape).to_string(),
        roundrect_rratio: pin.roundrect_rratio(),
        layers,
        drill,
        solder_mask_margin: pin.solder_mask_margin(),
        pin_index: Some(index),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf::UnitExt;

    fn qfn_comp() -> CompiledComponent {
        CompiledComponent {
            refdes: "U1".into(),
            pins: vec![
                Pin::build("1")
                    .pos(-3.45, -2.8)
                    .rotation(0.0)
                    .length(0.8)
                    .width(0.8)
                    .height(0.2)
                    .pad_type("smd")
                    .pad_shape("roundrect")
                    .roundrect_rratio(0.25)
                    .layers("F.Cu F.Mask F.Paste")
                    .pwr_fixed(3.3.volt(), 0.1.amp())
                    .pin(),
                Pin::build("2")
                    .pos(-3.45, -2.4)
                    .length(0.8)
                    .width(0.8)
                    .height(0.2)
                    .pad_type("smd")
                    .dio(),
            ],
            constraints: vec![],
            symbol: Some("TestPart".into()),
            footprint: Some("TestPart".into()),
            mechanical: vec![MechanicalPad {
                number: String::new(),
                pos: (0.0, 0.0),
                width: 0.91,
                height: 0.91,
                pad_type: "smd".into(),
                pad_shape: "roundrect".into(),
                roundrect_rratio: Some(0.25),
                layers: Some("F.Paste".into()),
                drill: 0.0,
            }],
            datasheet: None,
            description: None,
            model_3d: None,
            model_3d_data: None,
            model_3d_rotation: (0.0, 0.0, 0.0),
            model_3d_offset: (0.0, 0.0, 0.0),
        }
    }

    #[test]
    fn pads_use_physical_geometry() {
        let pads = pads_from_component(&qfn_comp());
        assert_eq!(pads.len(), 3); // 2 pins + 1 mechanical
        assert_eq!(pads[0].width, 0.8);
        assert_eq!(pads[0].height, 0.2);
        assert_eq!(pads[0].pad_type, "smd");
        assert_eq!(pads[0].shape, "roundrect");
        assert_eq!(pads[0].roundrect_rratio, Some(0.25));
        assert_eq!(pads[2].layers, "F.Paste");
        // SMD anchor: pad bounding box is centred on the origin.
        let (x1, y1, x2, y2) = pads_extent(&pads).unwrap();
        assert!((x1 + x2).abs() < 1e-9, "anchor not centred: {x1}..{x2}");
        assert!((y1 + y2).abs() < 1e-9, "anchor not centred: {y1}..{y2}");
        // Relative pad spacing is preserved.
        assert!((pads[1].pos.1 - pads[0].pos.1 - 0.4).abs() < 1e-9);
    }

    #[test]
    fn missing_geometry_falls_back_to_origin_row() {
        let mut comp = qfn_comp();
        comp.pins = vec![
            Pin::build("1").dio(),
            Pin::build("2").dio(),
            Pin::build("3").dio(),
        ];
        comp.mechanical = vec![];
        let pads = pads_from_component(&comp);
        // KLC F7.2: pad 1 at the origin, row extending in +X.
        assert_eq!(pads[0].pos, (0.0, 0.0));
        assert_eq!(pads[1].pos, (2.54, 0.0));
        assert_eq!(pads[2].pos, (5.08, 0.0));
        assert_eq!(pads[0].pad_type, "thru_hole");
        assert_eq!(pads[0].drill, Some(DEFAULT_DRILL));
        assert_eq!(footprint_attr(&pads), "through_hole");
        // KLC F7.3: pad 1 rectangular, the rest circular.
        assert_eq!(pads[0].shape, "rect");
        assert_eq!(pads[1].shape, "circle");
        assert_eq!(pads[2].shape, "circle");
    }

    #[test]
    fn extent_covers_pads() {
        let pads = pads_from_component(&qfn_comp());
        let (x1, y1, x2, y2) = pads_extent(&pads).unwrap();
        assert!(x2 - x1 > 4.0, "extent too small: {x1}..{x2}");
        assert!(y2 - y1 > 3.0, "extent too small: {y1}..{y2}");
    }

    #[test]
    fn pad_sexpr_omits_zero_rotation() {
        let pad = PadGeom {
            number: "1".into(),
            pos: (-3.45, -2.8),
            rotation: 0.0,
            width: 0.8,
            height: 0.2,
            pad_type: "smd".into(),
            shape: "roundrect".into(),
            roundrect_rratio: Some(0.25),
            layers: SMD_LAYERS.into(),
            drill: None,
            solder_mask_margin: None,
            pin_index: Some(0),
        };
        let s = pad_sexpr(&pad, None, None).to_string();
        assert!(s.contains("(at -3.45 -2.8)"), "{s}");
        assert!(s.contains("(size 0.8 0.2)"), "{s}");
        assert!(s.contains("roundrect_rratio"), "{s}");
    }

    #[test]
    fn through_hole_anchor_on_pad_1() {
        let mut comp = qfn_comp();
        // Explicit through-hole positions: pad 1 must end up at the origin.
        comp.pins = vec![
            Pin::build("1").pos(5.0, 3.0).pad_type("thru_hole").dio(),
            Pin::build("2").pos(7.54, 3.0).pad_type("thru_hole").dio(),
        ];
        comp.mechanical = vec![];
        let pads = pads_from_component(&comp);
        assert_eq!(pads[0].pos, (0.0, 0.0));
        assert_eq!(pads[1].pos, (2.54, 0.0));
    }

    #[test]
    fn outlines_are_on_grid_and_klc_widths() {
        let pads = pads_from_component(&qfn_comp());
        let extent = pads_extent(&pads).unwrap();
        let out = outline_sexprs(extent, pin1_pos(&pads), None);
        let text: String = out
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("F.Fab"));
        assert!(text.contains("F.SilkS"));
        assert!(text.contains("F.CrtYd"));
        assert!(text.contains("(width 0.1)"));
        assert!(text.contains("(width 0.12)"));
        assert!(text.contains("(width 0.05)"));
        assert!(
            !text.contains("0000000000"),
            "float noise in output: {text}"
        );
    }
}
