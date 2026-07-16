//! KiCad footprint emitter.
//!
//! Generates `.kicad_mod` files from Copperleaf [`Manifest`] data so the part
//! TOML can serve as the single source of truth for a component's footprint.

use copperleaf_part_codegen::{Manifest, PinDef};

use crate::sexpr::Sexpr;

/// Default body margin added around the pad bounding-box to form the package
/// outline, in millimetres.
const BODY_MARGIN: f64 = 0.25;

/// Clearance from body edge to courtyard edge, in millimetres.
const COURTYARD_CLEARANCE: f64 = 0.25;

/// Silk-screen offset from fab outline, in millimetres.
const SILK_OFFSET: f64 = 0.205;

/// Generate a `.kicad_mod` S-expression string from a component manifest.
///
/// The footprint name is taken from `manifest.component.lib_id` if present,
/// otherwise from `manifest.component.name`.
pub fn emit_footprint(manifest: &Manifest) -> String {
    let name = manifest
        .component
        .lib_id
        .as_deref()
        .unwrap_or(&manifest.component.name);

    let mut children = Vec::new();

    // Footprint header.
    children.push(Sexpr::str(name));
    children.push(Sexpr::list([Sexpr::atom("layer"), Sexpr::atom("F.Cu")]));
    children.push(Sexpr::list([
        Sexpr::atom("tedit"),
        Sexpr::atom("00000000"),
    ]));
    children.push(Sexpr::list([
        Sexpr::atom("descr"),
        Sexpr::str(""),
    ]));
    children.push(Sexpr::list([Sexpr::atom("attr"), Sexpr::atom("smd")]));

    // Reference text.
    children.push(fp_text("reference", "REF**", (-1.0, -2.0), "F.SilkS"));

    // Value text.
    children.push(fp_text("value", name, (1.0, 2.0), "F.Fab"));

    // Pads.
    for pin in &manifest.pins {
        children.push(pad_node(pin));
        // Thermal vias — emit as extra thru_hole pads on all copper layers.
        for via in &pin.thermal_vias {
            children.push(Sexpr::list([
                Sexpr::atom("pad"),
                Sexpr::str(""), // no number — not an electrical pad
                Sexpr::atom("thru_hole"),
                Sexpr::atom("circle"),
                Sexpr::list([
                    Sexpr::atom("at"),
                    Sexpr::atom(fmt_f64(via.pos.0)),
                    Sexpr::atom(fmt_f64(via.pos.1)),
                ]),
                Sexpr::list([
                    Sexpr::atom("size"),
                    Sexpr::atom(fmt_f64(via.size)),
                    Sexpr::atom(fmt_f64(via.size)),
                ]),
                Sexpr::list([
                    Sexpr::atom("drill"),
                    Sexpr::atom(fmt_f64(via.drill)),
                ]),
                Sexpr::list([
                    Sexpr::atom("layers"),
                    Sexpr::atom("*.Cu"),
                ]),
            ]));
        }
    }

    // Mechanical pads (mounting holes, fiducials, etc.).
    for mech in &manifest.mechanical {
        children.push(mechanical_pad_node(mech));
    }

    // Outline.
    if let Some((x1, y1, x2, y2)) = compute_extents(manifest) {
        // Fab.
        for &(start, end) in &outline_segments(x1, y1, x2, y2) {
            children.push(fp_line(start, end, "F.Fab", 0.127));
        }
        // Silk.
        let sx1 = x1 - SILK_OFFSET;
        let sy1 = y1 - SILK_OFFSET;
        let sx2 = x2 + SILK_OFFSET;
        let sy2 = y2 + SILK_OFFSET;
        for &(start, end) in &outline_segments(sx1, sy1, sx2, sy2) {
            children.push(fp_line(start, end, "F.SilkS", 0.127));
        }
        // Courtyard.
        let cx1 = x1 - COURTYARD_CLEARANCE;
        let cy1 = y1 - COURTYARD_CLEARANCE;
        let cx2 = x2 + COURTYARD_CLEARANCE;
        let cy2 = y2 + COURTYARD_CLEARANCE;
        for &(start, end) in &outline_segments(cx1, cy1, cx2, cy2) {
            children.push(fp_line(start, end, "F.CrtYd", 0.05));
        }

        // Pin-1 marker on silk and fab.
        if let Some((px, py)) = pin1_pos(manifest) {
            let dot_r = 0.1;
            children.push(fp_circle((px, py), dot_r, "F.SilkS", 0.2));
            children.push(fp_circle((px, py), dot_r, "F.Fab", 0.2));
        }
    }

    Sexpr::list([Sexpr::atom("footprint").into()].into_iter().chain(children)).to_string()
}

// ── helpers ────────────────────────────────────────────────────────────

fn pad_node(pin: &PinDef) -> Sexpr {
    let mut children: Vec<Sexpr> = Vec::new();

    children.push(Sexpr::atom("pad"));
    let number_str = if pin.number.is_empty() {
        pin.num.to_string()
    } else {
        pin.number.clone()
    };
    children.push(Sexpr::str(&number_str));
    children.push(Sexpr::atom(
        pin.pad_type.as_deref().unwrap_or("smd"),
    ));
    children.push(Sexpr::atom(
        pin.pad_shape.as_deref().unwrap_or("rect"),
    ));

    if let Some(rr) = pin.roundrect_rratio {
        children.push(Sexpr::list([
            Sexpr::atom("roundrect_rratio"),
            Sexpr::atom(fmt_f64(rr)),
        ]));
    }

    let (x, y) = pin.pos.unwrap_or((0.0, 0.0));
    let rot = pin.rotation.unwrap_or(0.0);
    if rot != 0.0 {
        children.push(Sexpr::list([
            Sexpr::atom("at"),
            Sexpr::atom(fmt_f64(x)),
            Sexpr::atom(fmt_f64(y)),
            Sexpr::atom(fmt_f64(rot)),
        ]));
    } else {
        children.push(Sexpr::list([
            Sexpr::atom("at"),
            Sexpr::atom(fmt_f64(x)),
            Sexpr::atom(fmt_f64(y)),
        ]));
    }

    let w = pin.width.unwrap_or(pin.length.unwrap_or(0.0));
    let h = pin.height.or(pin.length).unwrap_or(0.0);
    children.push(Sexpr::list([
        Sexpr::atom("size"),
        Sexpr::atom(fmt_f64(w)),
        Sexpr::atom(fmt_f64(h)),
    ]));

    // Layers.
    if let Some(ref layers_str) = pin.layers {
        let layer_atoms: Vec<Sexpr> = std::iter::once(Sexpr::atom("layers"))
            .chain(
                layers_str
                    .split_whitespace()
                    .map(|s| Sexpr::atom(s.to_string())),
            )
            .collect();
        children.push(Sexpr::list(layer_atoms));
    }

    // Drill for thru-hole pads.
    if let Some(drill) = pin.drill {
        children.push(Sexpr::list([
            Sexpr::atom("drill"),
            Sexpr::atom(fmt_f64(drill)),
        ]));
    }

    // Solder mask margin.
    if let Some(smm) = pin.solder_mask_margin {
        children.push(Sexpr::list([
            Sexpr::atom("solder_mask_margin"),
            Sexpr::atom(fmt_f64(smm)),
        ]));
    }

    Sexpr::list(children)
}

fn mechanical_pad_node(mech: &copperleaf_part_codegen::MechanicalDef) -> Sexpr {
    let mut children: Vec<Sexpr> = Vec::new();

    children.push(Sexpr::atom("pad"));
    children.push(Sexpr::str(&mech.number));
    children.push(Sexpr::atom(&mech.pad_type));
    children.push(Sexpr::atom(&mech.pad_shape));

    if let Some(rr) = mech.roundrect_rratio {
        children.push(Sexpr::list([
            Sexpr::atom("roundrect_rratio"),
            Sexpr::atom(fmt_f64(rr)),
        ]));
    }

    children.push(Sexpr::list([
        Sexpr::atom("at"),
        Sexpr::atom(fmt_f64(mech.pos.0)),
        Sexpr::atom(fmt_f64(mech.pos.1)),
    ]));

    children.push(Sexpr::list([
        Sexpr::atom("size"),
        Sexpr::atom(fmt_f64(mech.width)),
        Sexpr::atom(fmt_f64(mech.height)),
    ]));

    if mech.drill > 0.0 {
        children.push(Sexpr::list([
            Sexpr::atom("drill"),
            Sexpr::atom(fmt_f64(mech.drill)),
        ]));
    }

    let layers_str = mech.layers.as_deref().unwrap_or("*.Cu *.Mask");
    let layer_atoms: Vec<Sexpr> = std::iter::once(Sexpr::atom("layers"))
        .chain(layers_str.split_whitespace().map(|s| Sexpr::atom(s.to_string())))
        .collect();
    children.push(Sexpr::list(layer_atoms));

    Sexpr::list(children)
}

fn fp_text(kind: &str, value: &str, pos: (f64, f64), layer: &str) -> Sexpr {
    Sexpr::list([
        Sexpr::atom("fp_text"),
        Sexpr::atom(kind),
        Sexpr::str(value),
        Sexpr::list([
            Sexpr::atom("at"),
            Sexpr::atom(fmt_f64(pos.0)),
            Sexpr::atom(fmt_f64(pos.1)),
        ]),
        Sexpr::list([Sexpr::atom("layer"), Sexpr::atom(layer)]),
        Sexpr::list([
            Sexpr::atom("effects"),
            Sexpr::list([
                Sexpr::atom("font"),
                Sexpr::list([
                    Sexpr::atom("size"),
                    Sexpr::atom("0.64"),
                    Sexpr::atom("0.64"),
                ]),
                Sexpr::list([
                    Sexpr::atom("thickness"),
                    Sexpr::atom("0.15"),
                ]),
            ]),
        ]),
    ])
}

fn fp_line(start: (f64, f64), end: (f64, f64), layer: &str, width: f64) -> Sexpr {
    Sexpr::list([
        Sexpr::atom("fp_line"),
        Sexpr::list([
            Sexpr::atom("start"),
            Sexpr::atom(fmt_f64(start.0)),
            Sexpr::atom(fmt_f64(start.1)),
        ]),
        Sexpr::list([
            Sexpr::atom("end"),
            Sexpr::atom(fmt_f64(end.0)),
            Sexpr::atom(fmt_f64(end.1)),
        ]),
        Sexpr::list([Sexpr::atom("layer"), Sexpr::atom(layer)]),
        Sexpr::list([
            Sexpr::atom("width"),
            Sexpr::atom(fmt_f64(width)),
        ]),
    ])
}

fn fp_circle(center: (f64, f64), radius: f64, layer: &str, width: f64) -> Sexpr {
    Sexpr::list([
        Sexpr::atom("fp_circle"),
        Sexpr::list([
            Sexpr::atom("center"),
            Sexpr::atom(fmt_f64(center.0)),
            Sexpr::atom(fmt_f64(center.1)),
        ]),
        Sexpr::list([
            Sexpr::atom("end"),
            Sexpr::atom(fmt_f64(center.0 + radius)),
            Sexpr::atom(fmt_f64(center.1)),
        ]),
        Sexpr::list([Sexpr::atom("layer"), Sexpr::atom(layer)]),
        Sexpr::list([
            Sexpr::atom("width"),
            Sexpr::atom(fmt_f64(width)),
        ]),
    ])
}

/// Compute package body extents from pin positions, returning `(x1, y1, x2, y2)`.
fn compute_extents(manifest: &Manifest) -> Option<(f64, f64, f64, f64)> {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for pin in &manifest.pins {
        let Some((px, py)) = pin.pos else { continue };
        let half_w = pin.width.unwrap_or(pin.length.unwrap_or(0.0)) / 2.0;
        let half_h = pin.height.or(pin.length).unwrap_or(0.0) / 2.0;
        min_x = min_x.min(px - half_w);
        max_x = max_x.max(px + half_w);
        min_y = min_y.min(py - half_h);
        max_y = max_y.max(py + half_h);
    }

    if min_x == f64::MAX {
        return None;
    }

    Some((
        min_x - BODY_MARGIN,
        min_y - BODY_MARGIN,
        max_x + BODY_MARGIN,
        max_y + BODY_MARGIN,
    ))
}

/// Four line segments forming a rectangle.
fn outline_segments(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
) -> [((f64, f64), (f64, f64)); 4] {
    [
        ((x1, y1), (x2, y1)),
        ((x2, y1), (x2, y2)),
        ((x2, y2), (x1, y2)),
        ((x1, y2), (x1, y1)),
    ]
}

/// Position of pin 1 (lowest-numbered pin with a known position).
fn pin1_pos(manifest: &Manifest) -> Option<(f64, f64)> {
    manifest
        .pins
        .iter()
        .filter(|p| p.pos.is_some())
        .min_by_key(|p| p.num)
        .and_then(|p| p.pos)
}

fn fmt_f64(v: f64) -> String {
    format!("{:?}", v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf_part_codegen::{ComponentMeta, PinDef};

    fn make_manifest() -> Manifest {
        Manifest {
            component: ComponentMeta {
                name: "Test".into(),
                title: "Test Part".into(),
                description: None,
                datasheet: None,
                lib_id: Some("TestPart".into()),
            },
            pins: vec![
                PinDef {
                    num: 1,
                    number: "1".into(),
                    name: "VDD".into(),
                    purpose: "Supply".into(),
                    notes: String::new(),
                    kind: "pwr".into(),
                    bw_mhz: None,
                    v: None,
                    v_min: None,
                    v_max: None,
                    i: None,
                    i_max: None,
                    pos: Some((-2.54, 0.0)),
                    rotation: Some(0.0),
                    length: Some(1.0),
                    nc: None,
                    width: Some(0.5),
                    height: Some(1.0),
                    pad_type: Some("smd".into()),
                    pad_shape: Some("rect".into()),
                    roundrect_rratio: None,
                    solder_mask_margin: Some(0.102),
                    layers: Some("F.Cu F.Mask F.Paste".into()),
                    drill: None,
                    thermal_vias: vec![],
                },
                PinDef {
                    num: 2,
                    number: "2".into(),
                    name: "GND".into(),
                    purpose: "Ground".into(),
                    notes: String::new(),
                    kind: "gnd".into(),
                    bw_mhz: None,
                    v: None,
                    v_min: None,
                    v_max: None,
                    i: None,
                    i_max: None,
                    pos: Some((2.54, 0.0)),
                    rotation: Some(0.0),
                    length: Some(1.0),
                    nc: None,
                    width: Some(0.5),
                    height: Some(1.0),
                    pad_type: Some("smd".into()),
                    pad_shape: Some("rect".into()),
                    roundrect_rratio: None,
                    solder_mask_margin: Some(0.102),
                    layers: Some("F.Cu F.Mask F.Paste".into()),
                    drill: None,
                    thermal_vias: vec![],
                },
            ],
            constraints: vec![],

            mechanical: vec![],
        }
    }

    #[test]
    fn emits_valid_s_expression() {
        let out = emit_footprint(&make_manifest());
        // Should parse as valid S-expression.
        let parsed = crate::sexpr::parse(&out);
        assert!(parsed.is_ok(), "failed to parse: {out}");
    }

    #[test]
    fn contains_footprint_header() {
        let out = emit_footprint(&make_manifest());
        assert!(out.starts_with("(footprint"), "missing footprint header");
        assert!(out.contains("\"TestPart\""), "missing footprint name");
    }

    #[test]
    fn contains_pads() {
        let out = emit_footprint(&make_manifest());
        assert!(out.contains("(pad \"1\" smd rect"), "missing pad 1");
        assert!(out.contains("(pad \"2\" smd rect"), "missing pad 2");
    }

    #[test]
    fn contains_outline_when_pads_have_positions() {
        let out = emit_footprint(&make_manifest());
        assert!(out.contains("fp_line"), "missing outline");
        assert!(out.contains("F.Fab"), "missing fab layer");
        assert!(out.contains("F.CrtYd"), "missing courtyard layer");
    }

    #[test]
    fn empty_manifest_no_outline() {
        let manifest = Manifest {
            component: ComponentMeta {
                name: "Empty".into(),
                title: "Empty".into(),
                description: None,
                datasheet: None,
                lib_id: None,
            },
            pins: vec![],
            constraints: vec![],

            mechanical: vec![],
        };
        let out = emit_footprint(&manifest);
        assert!(!out.contains("fp_line"), "should have no outline");
    }

    #[test]
    fn thermal_vias_emit_extra_pads() {
        let mut manifest = make_manifest();
        manifest.pins[1].thermal_vias = vec![
            copperleaf_part_codegen::ThermalViaDef {
                pos: (0.35, 0.0),
                drill: 0.2,
                size: 0.3,
            },
        ];
        let out = emit_footprint(&manifest);
        assert!(out.contains("thru_hole"), "missing via pad");
        assert!(out.contains("*.Cu"), "missing via layers");
        assert!(out.contains("0.35"), "missing via position");
    }
}
