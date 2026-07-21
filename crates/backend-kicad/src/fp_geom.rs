//! Shared footprint pad geometry for the KiCad emitters.
//!
//! Builds a single, data-driven pad list from a [`CompiledComponent`] (pin pad
//! geometry, thermal vias, and mechanical pads) so that the standalone
//! `.kicad_mod` library files and the footprints embedded in the `.kicad_pcb`
//! are always identical.

use copperleaf::{
    CompiledComponent, Pad, PadShape, PadType, Pin, pad_extent, resolve_mech_pad, resolve_pad,
};

// Re-exports for backward compatibility with callers.
pub use copperleaf::{
    DEFAULT_DRILL, DEFAULT_PAD_SIZE, PTH_LAYERS, SMD_LAYERS, auto_pad_pos, normalise_anchor,
};

use crate::{
    common::{format_float, format_grid_float},
    sexpr::Sexpr,
};

/// Margin added around the pad bounding box to form the fab outline, in mm.
const BODY_MARGIN: f64 = 0.25;
/// Courtyard offset from the fab outline, in mm.
const COURTYARD_OFFSET: f64 = 0.25;
/// Silkscreen offset from the fab outline, in mm.
const SILK_OFFSET: f64 = 0.11;

/// See [`copperleaf::pad_extent`].
#[inline]
pub fn pads_extent(pads: &[Pad]) -> Option<(f64, f64, f64, f64)> {
    pad_extent(pads)
}

/// KiCad footprint `attr` value for the pad set: `smd` when any pad is
/// surface-mount (mixed footprints included), `through_hole` only when all
/// pads are through-hole (KLC F6.1).
pub fn footprint_attr(pads: &[Pad]) -> &'static str {
    if pads.iter().any(|p| p.pad_type == PadType::Smd) || pads.is_empty() {
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
            Sexpr::str(crate::deterministic_id(&format!(
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
            Sexpr::str(crate::deterministic_id(&format!(
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
                Sexpr::atom(format!("{ox}")),
                Sexpr::atom(format!("{oy}")),
                Sexpr::atom(format!("{oz}")),
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
                Sexpr::atom(format!("{rx}")),
                Sexpr::atom(format!("{ry}")),
                Sexpr::atom(format!("{rz}")),
            ]),
        ]),
    ])
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
pub fn pad_sexpr(pad: &Pad, uuid: Option<&str>, net: Option<(usize, &str)>) -> Sexpr {
    let mut children = vec![
        Sexpr::atom("pad"),
        Sexpr::str(&pad.number),
        Sexpr::atom(pad.pad_type.as_str()),
        Sexpr::atom(pad.pad_shape.as_str()),
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

    let layers_str = pad.layers.as_deref().unwrap_or("");
    children.push(Sexpr::list(
        std::iter::once(Sexpr::atom("layers"))
            .chain(layers_str.split_whitespace().map(Sexpr::atom)),
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

/// Internal helper: gather all pads with pin-index metadata, without
/// normalisation.  Returns `(pads, pin_indices)` where `pin_indices[i]` is
/// `Some(pin_index)` for electrical pins and `None` for thermal vias and
/// mechanical pads.
fn gather_pads_internal(comp: &CompiledComponent) -> (Vec<Pad>, Vec<Option<usize>>) {
    let mut pads = Vec::new();
    let mut pin_indices = Vec::new();

    for (i, pin) in comp.pins.iter().enumerate() {
        pads.push(resolve_pad(pin, i));
        pin_indices.push(Some(i));
        for via in pin.thermal_vias() {
            pads.push(thermal_via_pad(via.pos, via.drill, via.size));
            pin_indices.push(None);
        }
    }

    for mech in &comp.mechanical {
        pads.push(resolve_mech_pad(mech));
        pin_indices.push(None);
    }

    // Assign sequential pad numbers to any un-numbered pads (thermal vias
    // and mechanical pads) so that every pad can be matched to a schematic
    // pin.  The numbering continues after the highest numeric electrical
    // pin number.
    let max_electrical = pads
        .iter()
        .zip(pin_indices.iter())
        .filter(|(_, i)| i.is_some())
        .filter_map(|(p, _)| p.number.parse::<usize>().ok())
        .max()
        .unwrap_or(0);
    let mut next_num = max_electrical + 1;
    for (pad, pi) in pads.iter_mut().zip(pin_indices.iter()) {
        if pi.is_none() && pad.number.is_empty() {
            pad.number = next_num.to_string();
            next_num += 1;
        }
    }

    (pads, pin_indices)
}

/// Collect all pads for a component: electrical pins (with thermal vias)
/// followed by mechanical pads.  The anchor is normalised per KLC (see
/// [`normalise_anchor`]).
///
/// Un-numbered pads (thermal vias and mechanical pads) are assigned
/// sequential pad numbers continuing after the last electrical pin so
/// that every pad has a unique number matching its schematic pin.
pub fn pads_from_component(comp: &CompiledComponent) -> Vec<Pad> {
    let (mut pads, _pin_indices) = gather_pads_internal(comp);
    normalise_anchor(&mut pads);
    pads
}

/// Like [`pads_from_component`], but also returns a parallel vector of
/// pin indices: `Some(i)` for electrical pins (the `i`th pin in
/// `comp.pins`), `None` for thermal vias and mechanical pads.
pub fn pads_from_component_with_indices(
    comp: &CompiledComponent,
) -> (Vec<Pad>, Vec<Option<usize>>) {
    let (mut pads, pin_indices) = gather_pads_internal(comp);
    normalise_anchor(&mut pads);
    (pads, pin_indices)
}

/// Return `(number, MECH_name)` pairs for pads that do not correspond to an
/// electrical pin (thermal vias and mechanical pads).  Schematic and netlist
/// emitters add these as extra symbol pins so pin counts match.
pub fn mech_pad_names(comp: &CompiledComponent) -> Vec<(String, String)> {
    let (pads, pin_indices) = gather_pads_internal(comp);
    pads.iter()
        .zip(pin_indices.iter())
        .filter(|(_, i)| i.is_none())
        .enumerate()
        .map(|(i, (p, _))| (p.number.clone(), format!("MECH{}", i + 1)))
        .collect()
}

/// Position of the first electrical pad, used for the pin-1 marker.
pub fn pin1_pos(pads: &[Pad]) -> Option<(f64, f64)> {
    pads.first().map(|p| p.pos)
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
pub fn thermal_via_pad(pos: (f64, f64), drill: f64, size: f64) -> Pad {
    Pad {
        number: String::new(),
        pos,
        rotation: 0.0,
        width: size,
        height: size,
        pad_type: PadType::ThruHole,
        pad_shape: PadShape::Circle,
        roundrect_rratio: None,
        layers: Some(PTH_LAYERS.to_string()),
        drill: Some(drill),
        solder_mask_margin: None,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf::{ComponentMeta, UnitExt};

    fn qfn_comp() -> CompiledComponent {
        CompiledComponent {
            refdes: "U1".into(),
            meta: ComponentMeta {
                symbol: Some("TestPart".into()),
                footprint: Some("TestPart".into()),
                ..ComponentMeta::default()
            },
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
            mechanical: vec![Pad {
                number: String::new(),
                pos: (0.0, 0.0),
                rotation: 0.0,
                width: 0.91,
                height: 0.91,
                pad_type: PadType::Smd,
                pad_shape: PadShape::RoundRect,
                roundrect_rratio: Some(0.25),
                layers: Some("F.Paste".into()),
                drill: None,
                solder_mask_margin: None,
            }],
        }
    }

    #[test]
    fn pads_use_physical_geometry() {
        let pads = pads_from_component(&qfn_comp());
        assert_eq!(pads.len(), 3); // 2 pins + 1 mechanical
        assert_eq!(pads[0].width, 0.8);
        assert_eq!(pads[0].height, 0.2);
        assert_eq!(pads[0].pad_type, PadType::Smd);
        assert_eq!(pads[0].pad_shape, PadShape::RoundRect);
        assert_eq!(pads[0].roundrect_rratio, Some(0.25));
        assert_eq!(pads[2].layers.as_deref(), Some("F.Paste"));
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
        assert_eq!(pads[0].pad_type, PadType::ThruHole);
        assert_eq!(pads[0].drill, Some(DEFAULT_DRILL));
        assert_eq!(footprint_attr(&pads), "through_hole");
        // KLC F7.3: pad 1 rectangular, the rest circular.
        assert_eq!(pads[0].pad_shape, PadShape::Rect);
        assert_eq!(pads[1].pad_shape, PadShape::Circle);
        assert_eq!(pads[2].pad_shape, PadShape::Circle);
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
        let pad = Pad {
            number: "1".into(),
            pos: (-3.45, -2.8),
            rotation: 0.0,
            width: 0.8,
            height: 0.2,
            pad_type: PadType::Smd,
            pad_shape: PadShape::RoundRect,
            roundrect_rratio: Some(0.25),
            layers: Some(SMD_LAYERS.to_string()),
            drill: None,
            solder_mask_margin: None,
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

    // ── Characterisation tests for resolve_pad (phase 2 baseline) ──

    /// pad_type: SMD when pos is Some, through-hole when pos is None.
    #[test]
    fn resolve_pad_infers_pad_type_from_pos() {
        let comp = CompiledComponent {
            refdes: "U1".into(),
            pins: vec![
                Pin::build("1").pos(1.0, 0.0).dio(),
                Pin::build("2").dio(), // no pos → through-hole row
            ],
            ..empty_comp()
        };
        let pads = pads_from_component(&comp);
        // Pin with explicit pos → SMD
        assert_eq!(pads[0].pad_type, PadType::Smd);
        // Pin without pos → through-hole (auto-row)
        assert_eq!(pads[1].pad_type, PadType::ThruHole);
    }

    /// Through-hole pads get a default drill; SMD pads get None.
    #[test]
    fn resolve_pad_defaults_drill_for_th() {
        let comp = CompiledComponent {
            refdes: "U1".into(),
            pins: vec![
                Pin::build("1").pos(0.0, 0.0).pad_type("thru_hole").dio(),
                Pin::build("2").pos(1.0, 0.0).pad_type("smd").dio(),
            ],
            ..empty_comp()
        };
        let pads = pads_from_component(&comp);
        // TH pad: drill defaults to DEFAULT_DRILL.
        assert_eq!(pads[0].drill, Some(DEFAULT_DRILL));
        // SMD pad: no drill.
        assert_eq!(pads[1].drill, None);
    }

    /// Explicit drill is preserved.
    #[test]
    fn resolve_pad_preserves_explicit_drill() {
        let comp = CompiledComponent {
            refdes: "U1".into(),
            pins: vec![
                Pin::build("1")
                    .pos(0.0, 0.0)
                    .pad_type("thru_hole")
                    .drill(1.0)
                    .dio(),
            ],
            ..empty_comp()
        };
        let pads = pads_from_component(&comp);
        assert_eq!(pads[0].drill, Some(1.0));
    }

    /// Layers default by pad type: PTH_LAYERS for through-hole, SMD_LAYERS for SMD.
    #[test]
    fn resolve_pad_defaults_layers_by_type() {
        let comp = CompiledComponent {
            refdes: "U1".into(),
            pins: vec![
                Pin::build("1").pos(0.0, 0.0).pad_type("thru_hole").dio(),
                Pin::build("2").pos(1.0, 0.0).pad_type("smd").dio(),
            ],
            ..empty_comp()
        };
        let pads = pads_from_component(&comp);
        assert_eq!(pads[0].layers.as_deref(), Some(PTH_LAYERS));
        assert_eq!(pads[1].layers.as_deref(), Some(SMD_LAYERS));
    }

    /// Auto-generated through-hole rows: pad 1 is rect, the rest are circle (KLC F7.3).
    #[test]
    fn pad_from_pin_shape_default_for_auto_row() {
        let comp = CompiledComponent {
            refdes: "U1".into(),
            pins: vec![
                Pin::build("1").dio(),
                Pin::build("2").dio(),
                Pin::build("3").dio(),
            ],
            ..empty_comp()
        };
        let pads = pads_from_component(&comp);
        assert_eq!(pads[0].pad_shape, PadShape::Rect);
        assert_eq!(pads[1].pad_shape, PadShape::Circle);
        assert_eq!(pads[2].pad_shape, PadShape::Circle);
    }

    /// Explicit shape is preserved even in auto-row context.
    #[test]
    fn pad_from_pin_preserves_explicit_shape() {
        let comp = CompiledComponent {
            refdes: "U1".into(),
            pins: vec![
                Pin::build("1")
                    .pad_shape("roundrect")
                    .roundrect_rratio(0.25)
                    .dio(),
            ],
            ..empty_comp()
        };
        let pads = pads_from_component(&comp);
        assert_eq!(pads[0].pad_shape, PadShape::RoundRect);
        assert_eq!(pads[0].roundrect_rratio, Some(0.25));
    }

    /// Width/height fall back to pin length, then DEFAULT_PAD_SIZE.
    /// NOTE: [`resolve_pad`] treats zero-width/zero-height pads as
    /// "not set" and falls through to the symbol length or
    /// [`DEFAULT_PAD_SIZE`] — this is the single source of truth.
    #[test]
    fn pad_from_pin_falls_back_width_height_to_length() {
        let comp = CompiledComponent {
            refdes: "U1".into(),
            pins: vec![
                // No pad fields at all: width/height fall back to DEFAULT_PAD_SIZE.
                Pin::build("1").dio(),
                // Only pos set → width=0.0 in pad; resolve_pad treats 0.0 as
                // unset, so falls through to DEFAULT_PAD_SIZE.
                Pin::build("2").pos(1.0, 0.0).dio(),
            ],
            ..empty_comp()
        };
        let pads = pads_from_component(&comp);
        assert_eq!(pads[0].width, DEFAULT_PAD_SIZE);
        assert_eq!(pads[0].height, DEFAULT_PAD_SIZE);
        // Zero-width pad is treated as unset by resolve_pad → falls to default.
        assert_eq!(pads[1].width, DEFAULT_PAD_SIZE);
        assert_eq!(pads[1].height, DEFAULT_PAD_SIZE);
    }

    /// Pin number defaults to 1-based index when not set.
    #[test]
    fn pad_from_pin_defaults_number_to_index() {
        let comp = CompiledComponent {
            refdes: "U1".into(),
            pins: vec![Pin::build("A").dio(), Pin::build("B").dio()],
            ..empty_comp()
        };
        let pads = pads_from_component(&comp);
        assert_eq!(pads[0].number, "1");
        assert_eq!(pads[1].number, "2");
    }

    /// Explicit pin number is preserved.
    #[test]
    fn pad_from_pin_preserves_explicit_number() {
        let comp = CompiledComponent {
            refdes: "U1".into(),
            pins: vec![Pin::build("A").number("7").dio()],
            ..empty_comp()
        };
        let pads = pads_from_component(&comp);
        assert_eq!(pads[0].number, "7");
    }

    /// Auto-row positions: 2.54 mm pitch starting at origin.
    #[test]
    fn pad_from_pin_auto_row_positions() {
        let comp = CompiledComponent {
            refdes: "U1".into(),
            pins: vec![
                Pin::build("1").dio(),
                Pin::build("2").dio(),
                Pin::build("4").dio(),
            ],
            ..empty_comp()
        };
        let pads = pads_from_component(&comp);
        assert_eq!(pads[0].pos, (0.0, 0.0));
        assert_eq!(pads[1].pos, (2.54, 0.0));
        assert_eq!(pads[2].pos, (5.08, 0.0));
    }

    /// Anchor normalisation: fully auto row is left untouched.
    #[test]
    fn pad_from_pin_anchor_auto_row_untouched() {
        let comp = CompiledComponent {
            refdes: "U1".into(),
            pins: vec![Pin::build("1").dio(), Pin::build("2").dio()],
            ..empty_comp()
        };
        let pads = pads_from_component(&comp);
        // Pad 1 stays at origin for pure auto-row.
        assert_eq!(pads[0].pos, (0.0, 0.0));
    }

    /// Anchor normalisation: SMD pads are centred on the bounding box.
    #[test]
    fn pad_from_pin_anchor_smd_centred() {
        let comp = CompiledComponent {
            refdes: "U1".into(),
            pins: vec![
                Pin::build("1").pos(1.0, 1.0).dio(),
                Pin::build("2").pos(3.0, 3.0).dio(),
            ],
            ..empty_comp()
        };
        let pads = pads_from_component(&comp);
        let (x1, y1, x2, y2) = pads_extent(&pads).unwrap();
        assert!((x1 + x2).abs() < 1e-9, "SMD anchor not centred: {x1}..{x2}");
        assert!((y1 + y2).abs() < 1e-9, "SMD anchor not centred: {y1}..{y2}");
    }

    /// Anchor normalisation: explicit through-hole pads anchor on pad 1.
    #[test]
    fn pad_from_pin_anchor_th_on_pad1() {
        let comp = CompiledComponent {
            refdes: "U1".into(),
            pins: vec![
                Pin::build("1").pos(5.0, 3.0).pad_type("thru_hole").dio(),
                Pin::build("2").pos(7.54, 3.0).pad_type("thru_hole").dio(),
            ],
            ..empty_comp()
        };
        let pads = pads_from_component(&comp);
        assert_eq!(pads[0].pos, (0.0, 0.0));
        assert_eq!(pads[1].pos, (2.54, 0.0));
    }

    /// rotation defaults to 0.0 when not set.
    #[test]
    fn pad_from_pin_defaults_rotation_to_zero() {
        let comp = CompiledComponent {
            refdes: "U1".into(),
            pins: vec![Pin::build("1").dio()],
            ..empty_comp()
        };
        let pads = pads_from_component(&comp);
        assert_eq!(pads[0].rotation, 0.0);
    }

    /// solder_mask_margin passes through (None when unset).
    #[test]
    fn pad_from_pin_solder_mask_margin_passthrough() {
        let comp = CompiledComponent {
            refdes: "U1".into(),
            pins: vec![
                Pin::build("1").solder_mask_margin(0.102).dio(),
                Pin::build("2").dio(),
            ],
            ..empty_comp()
        };
        let pads = pads_from_component(&comp);
        assert_eq!(pads[0].solder_mask_margin, Some(0.102));
        assert_eq!(pads[1].solder_mask_margin, None);
    }

    fn empty_comp() -> CompiledComponent {
        CompiledComponent {
            refdes: String::new(),
            meta: ComponentMeta::default(),
            pins: vec![],
            constraints: vec![],
            mechanical: vec![],
        }
    }
}
