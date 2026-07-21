//! KiCad footprint emitter.
//!
//! Generates `.kicad_mod` files from Copperleaf [`Manifest`] data so the part
//! TOML can serve as the single source of truth for a component's footprint.

use base64::Engine as _;
use copperleaf_part_codegen::{Manifest, MechanicalDef, PinDef};

use copperleaf::{
    DEFAULT_DRILL, DEFAULT_PAD_SIZE, PTH_LAYERS, Pad, PadShape, PadType, SMD_LAYERS,
    resolve_mech_pad,
};

use crate::{
    fp_geom::{self},
    sexpr::Sexpr,
};

/// Errors that can occur when emitting a footprint.
#[derive(Debug, thiserror::Error)]
pub enum EmitError {
    /// Failed to decode embedded 3D model data.
    #[error("failed to decode embedded 3D model data: {0}")]
    Base64Decode(String),
    /// Failed to write a 3D model file to disk.
    #[error("failed to write 3D model to {path}: {source}")]
    ModelWrite {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
}

/// Generate a `.kicad_mod` S-expression string from a component manifest.
///
/// The footprint name is taken from `manifest.component.lib_id` if present,
/// otherwise from `manifest.component.name`.
pub fn emit_footprint(manifest: &Manifest) -> Result<String, EmitError> {
    emit_footprint_to(manifest, None)
}

/// Generate a `.kicad_mod` S-expression string from a component manifest,
/// optionally writing any embedded 3D model to `output_dir`.
///
/// When `output_dir` is `Some` and the manifest contains `model_3d_data`, the
/// decoded `.step` file is written alongside the output and the model path in
/// the S-expression uses just the filename (so it is relative to the
/// `.kicad_mod`).
pub fn emit_footprint_to(
    manifest: &Manifest,
    output_dir: Option<&std::path::Path>,
) -> Result<String, EmitError> {
    let name = manifest
        .component
        .lib_id
        .as_deref()
        .unwrap_or(&manifest.component.name);

    // Determine the model path for the S-expression.  If we have embedded data
    // and an output directory, write the file and use just the filename.
    let model_path_for_sexpr = if let (Some(dir), Some(data)) =
        (output_dir, manifest.component.model_3d_data.as_deref())
    {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|e| EmitError::Base64Decode(e.to_string()))?;
        // Extract filename from the original model path, or default to
        // <name>.step.
        let default_name = format!("{}.step", name);
        let filename = manifest
            .component
            .model_3d
            .as_deref()
            .and_then(|p| std::path::Path::new(p).file_name())
            .and_then(|s| s.to_str())
            .unwrap_or(&default_name);
        let out_path = dir.join(filename);
        std::fs::write(&out_path, &bytes).map_err(|source| EmitError::ModelWrite {
            path: out_path.clone(),
            source,
        })?;
        Some(filename.to_string())
    } else {
        manifest.component.model_3d.clone()
    };

    let pads = pads_from_manifest(manifest);
    let extent = fp_geom::pads_extent(&pads);

    // Footprint header.
    let descr = match (
        &manifest.component.description,
        &manifest.component.datasheet,
    ) {
        (Some(d), Some(url)) => format!("{}, {}", d, url),
        (Some(d), None) => d.clone(),
        (None, Some(url)) => url.clone(),
        (None, None) => String::new(),
    };
    let mut children = vec![
        Sexpr::str(name),
        Sexpr::list([Sexpr::atom("version"), Sexpr::atom("20231218")]),
        Sexpr::list([Sexpr::atom("generator"), Sexpr::str("copperleaf")]),
        Sexpr::list([Sexpr::atom("layer"), Sexpr::atom("F.Cu")]),
        Sexpr::list([Sexpr::atom("tedit"), Sexpr::atom("00000000")]),
        Sexpr::list([Sexpr::atom("descr"), Sexpr::str(&descr)]),
        Sexpr::list([Sexpr::atom("tags"), Sexpr::str("copperleaf")]),
        Sexpr::list([
            Sexpr::atom("attr"),
            Sexpr::atom(fp_geom::footprint_attr(&pads)),
        ]),
    ];

    // Text items: reference on silk, value + second reference on fab.
    let (cx, ref_y, val_y) = match extent {
        Some((x1, y1, x2, y2)) => ((x1 + x2) / 2.0, y1 - 1.52, y2 + 1.52),
        None => (0.0, -2.54, 2.54),
    };
    children.push(fp_geom::fp_text(
        "reference",
        "REF**",
        (cx, ref_y),
        "F.SilkS",
    ));
    children.push(fp_geom::fp_text("value", name, (cx, val_y), "F.Fab"));
    children.push(fp_geom::fp_text("user", "${REFERENCE}", (cx, 0.0), "F.Fab"));

    // Pads.
    for pad in &pads {
        children.push(fp_geom::pad_sexpr(pad, None, None));
    }

    // Outlines (fab, silk, courtyard, pin-1 marker).
    if let Some(ext) = extent {
        for node in fp_geom::outline_sexprs(ext, fp_geom::pin1_pos(&pads), None) {
            children.push(node);
        }
    }

    // 3D model reference (KLC F9.3; missing files are ignored by KiCad).
    let rot = manifest
        .component
        .model_3d_rotation
        .unwrap_or((0.0, 0.0, 0.0));
    let off = manifest
        .component
        .model_3d_offset
        .unwrap_or((0.0, 0.0, 0.0));
    children.push(fp_geom::model_sexpr(
        name,
        model_path_for_sexpr.as_deref(),
        off,
        rot,
    ));

    Ok(Sexpr::list([Sexpr::atom("footprint")].into_iter().chain(children)).to_string())
}

/// Collect all pads for a manifest: electrical pins (with thermal vias)
/// followed by mechanical pads.  Pin pads are resolved via
/// [`resolve_pin_def_pad`], which implements the same defaulting rules as
/// [`copperleaf::resolve_pad`] (design D2) so the generate and board pipelines
/// are always consistent.
pub fn pads_from_manifest(manifest: &Manifest) -> Vec<Pad> {
    let mut pads: Vec<Pad> = Vec::new();

    for (i, pin) in manifest.pins.iter().enumerate() {
        pads.push(resolve_pin_def_pad(pin, i));
        // Thermal vias are emitted as un-numbered through-holes on all copper.
        for via in &pin.thermal_vias {
            pads.push(fp_geom::thermal_via_pad(via.pos, via.drill, via.size));
        }
    }

    for mech in &manifest.mechanical {
        let raw = mech_to_pad(mech);
        pads.push(resolve_mech_pad(&raw));
    }

    fp_geom::normalise_anchor(&mut pads);
    pads
}

/// Resolve one pin's pad from TOML data, applying the exact same defaulting
/// rules as [`copperleaf::resolve_pad`] (design D2).  This is the only place
/// the generate-path pad defaulting lives.
fn resolve_pin_def_pad(pin_def: &PinDef, index: usize) -> Pad {
    // ── Gather explicit values ──
    let has_explicit_pos = pin_def.pos.is_some();
    let explicit_pad_type = pin_def.pad_type.as_deref().and_then(PadType::parse);
    let explicit_pad_shape = pin_def.pad_shape.as_deref().and_then(PadShape::parse);
    let sym_len = pin_def.length; // symbol pin stub length (width/height fallback)

    // 1. pad_type — explicit wins; SMD iff explicit pos, else through-hole.
    let pad_type = explicit_pad_type.unwrap_or(if has_explicit_pos {
        PadType::Smd
    } else {
        PadType::ThruHole
    });
    let is_through_hole = matches!(pad_type, PadType::ThruHole | PadType::NpThruHole);

    // 2. pos — explicit wins; else auto row at 2.54 mm pitch.
    let pos = pin_def.pos.unwrap_or_else(|| fp_geom::auto_pad_pos(index));

    // 3. width / height — explicit (> 0) wins; else sym_len; else default.
    let width = pin_def
        .width
        .filter(|&w| w > 0.0)
        .or(sym_len)
        .unwrap_or(DEFAULT_PAD_SIZE);
    let height = pin_def
        .height
        .filter(|&h| h > 0.0)
        .or(sym_len)
        .unwrap_or(DEFAULT_PAD_SIZE);

    // 4. layers — explicit wins; else PTH_LAYERS / SMD_LAYERS.
    let layers = pin_def.layers.clone().unwrap_or(if is_through_hole {
        PTH_LAYERS.to_string()
    } else {
        SMD_LAYERS.to_string()
    });

    // 5. drill — explicit wins; else DEFAULT_DRILL for TH, None for SMD.
    let drill = if is_through_hole {
        pin_def.drill.or(Some(DEFAULT_DRILL))
    } else {
        pin_def.drill
    };

    // 6. pad_shape — explicit wins; else auto TH row: pad 1 rect, rest circle.
    let pad_shape =
        explicit_pad_shape.unwrap_or(if !has_explicit_pos && is_through_hole && index > 0 {
            PadShape::Circle
        } else {
            PadShape::Rect
        });

    // 7. number — explicit (non-empty) wins; else TOML num or 1-based index.
    let number = if !pin_def.number.is_empty() {
        pin_def.number.clone()
    } else {
        pin_def.num.to_string()
    };

    Pad {
        number,
        pos,
        rotation: pin_def.rotation.unwrap_or(0.0),
        width,
        height,
        pad_type,
        pad_shape,
        roundrect_rratio: pin_def.roundrect_rratio,
        layers: Some(layers),
        drill,
        solder_mask_margin: pin_def.solder_mask_margin,
    }
}

/// Convert a [`MechanicalDef`] into a raw [`Pad`] (before
/// [`resolve_mech_pad`] fills in remaining defaults).
fn mech_to_pad(mech: &MechanicalDef) -> Pad {
    Pad {
        number: mech.number.clone(),
        pos: mech.pos,
        rotation: 0.0,
        width: mech.width,
        height: mech.height,
        pad_type: PadType::parse(&mech.pad_type).unwrap_or(PadType::Smd),
        pad_shape: PadShape::parse(&mech.pad_shape).unwrap_or(PadShape::Rect),
        roundrect_rratio: mech.roundrect_rratio,
        layers: mech.layers.clone(),
        drill: if mech.drill > 0.0 {
            Some(mech.drill)
        } else {
            None
        },
        solder_mask_margin: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf_part_codegen::{ComponentMeta, ElectricalFields, PinDef};

    fn pin_def(num: usize, name: &str, pos: (f64, f64)) -> PinDef {
        PinDef {
            num,
            number: num.to_string(),
            name: name.into(),
            purpose: "Test".into(),
            notes: String::new(),
            electrical: ElectricalFields {
                kind: "dio".into(),
                ..Default::default()
            },
            pos: Some(pos),
            rotation: Some(0.0),
            length: Some(1.0),
            width: Some(0.5),
            height: Some(1.0),
            pad_type: Some("smd".into()),
            pad_shape: Some("rect".into()),
            roundrect_rratio: None,
            solder_mask_margin: Some(0.102),
            layers: Some("F.Cu F.Mask F.Paste".into()),
            drill: None,
            thermal_vias: vec![],
        }
    }

    fn make_manifest() -> Manifest {
        Manifest {
            component: ComponentMeta {
                name: "Test".into(),
                title: "Test Part".into(),
                description: None,
                datasheet: None,
                lib_id: Some("TestPart".into()),
                model_3d: None,
                model_3d_data: None,
                model_3d_rotation: None,
                model_3d_offset: None,
            },
            pins: vec![
                pin_def(1, "VDD", (-2.54, 0.0)),
                pin_def(2, "GND", (2.54, 0.0)),
            ],
            constraints: vec![],
            mechanical: vec![],
        }
    }

    #[test]
    fn emits_valid_s_expression() {
        let out = emit_footprint(&make_manifest()).unwrap();
        // Should parse as valid S-expression.
        let parsed = crate::sexpr::parse(&out);
        assert!(parsed.is_ok(), "failed to parse: {out}");
    }

    #[test]
    fn contains_footprint_header() {
        let out = emit_footprint(&make_manifest()).unwrap();
        assert!(out.starts_with("(footprint"), "missing footprint header");
        assert!(out.contains("\"TestPart\""), "missing footprint name");
    }

    #[test]
    fn contains_pads() {
        let out = emit_footprint(&make_manifest()).unwrap();
        assert!(out.contains("(pad \"1\" smd rect"), "missing pad 1");
        assert!(out.contains("(pad \"2\" smd rect"), "missing pad 2");
        assert!(out.contains("(size 0.5 1)"), "missing pad size: {out}");
        assert!(out.contains("solder_mask_margin"), "missing mask margin");
    }

    #[test]
    fn contains_outline_when_pads_have_positions() {
        let out = emit_footprint(&make_manifest()).unwrap();
        assert!(out.contains("fp_line"), "missing outline");
        assert!(out.contains("F.Fab"), "missing fab layer");
        assert!(out.contains("F.CrtYd"), "missing courtyard layer");
        assert!(out.contains("${REFERENCE}"), "missing fab reference");
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
                model_3d: None,
                model_3d_data: None,
                model_3d_rotation: None,
                model_3d_offset: None,
            },
            pins: vec![],
            constraints: vec![],
            mechanical: vec![],
        };
        let out = emit_footprint(&manifest).unwrap();
        assert!(!out.contains("fp_line"), "should have no outline");
    }

    #[test]
    fn thermal_vias_emit_extra_pads() {
        let mut manifest = make_manifest();
        manifest.pins[1].thermal_vias = vec![copperleaf_part_codegen::ThermalViaDef {
            pos: (0.35, 0.0),
            drill: 0.2,
            size: 0.3,
        }];
        let out = emit_footprint(&manifest).unwrap();
        assert!(out.contains("thru_hole"), "missing via pad");
        assert!(out.contains("*.Cu"), "missing via layers");
        assert!(out.contains("0.35"), "missing via position");
    }

    // ── Characterisation tests for pad_from_pin_def (Phase 2 baseline) ──

    /// pad_type: without an explicit position, defaults to through-hole
    /// (design D2: SMD iff explicit pos, else TH).
    #[test]
    fn pad_from_pin_def_defaults_pad_type_to_th() {
        let pd = pin_without_pos();
        let pads = pads_from_manifest(&make_single_pin_manifest(pd));
        assert_eq!(pads[0].pad_type, PadType::ThruHole);
    }

    /// drill: TH pads get the default drill (design D2).
    #[test]
    fn pad_from_pin_def_defaults_drill_for_th() {
        let pd = PinDef {
            pad_type: Some("thru_hole".into()),
            ..pin_without_pos()
        };
        let pads = pads_from_manifest(&make_single_pin_manifest(pd));
        assert_eq!(pads[0].drill, Some(copperleaf::DEFAULT_DRILL));
    }

    /// Explicit drill is preserved.
    #[test]
    fn pad_from_pin_def_preserves_explicit_drill() {
        let pd = PinDef {
            pad_type: Some("thru_hole".into()),
            drill: Some(1.0),
            ..pin_without_pos()
        };
        let pads = pads_from_manifest(&make_single_pin_manifest(pd));
        assert_eq!(pads[0].drill, Some(1.0));
    }

    /// Layers default by pad type: PTH_LAYERS for through-hole, SMD_LAYERS for SMD.
    #[test]
    fn pad_from_pin_def_defaults_layers_by_type() {
        // smd pin
        let smd = PinDef {
            pad_type: Some("smd".into()),
            pos: Some((1.0, 0.0)),
            ..pin_without_pos()
        };
        let pads = pads_from_manifest(&make_single_pin_manifest(smd));
        assert_eq!(pads[0].layers.as_deref(), Some(fp_geom::SMD_LAYERS));

        // thru_hole pin
        let th = PinDef {
            pad_type: Some("thru_hole".into()),
            ..pin_without_pos()
        };
        let pads = pads_from_manifest(&make_single_pin_manifest(th));
        assert_eq!(pads[0].layers.as_deref(), Some(fp_geom::PTH_LAYERS));

        // np_thru_hole pin
        let npth = PinDef {
            pad_type: Some("np_thru_hole".into()),
            ..pin_without_pos()
        };
        let pads = pads_from_manifest(&make_single_pin_manifest(npth));
        assert_eq!(pads[0].layers.as_deref(), Some(fp_geom::PTH_LAYERS));
    }

    /// Without explicit geometry, pad_type defaults to TH and auto-row
    /// KLC F7.3 applies: pad 1 rect, the rest circle.
    #[test]
    fn pad_from_pin_def_shape_default_auto_th_row() {
        let manifest = Manifest {
            component: ComponentMeta {
                name: "Test".into(),
                title: "Test".into(),
                description: None,
                datasheet: None,
                lib_id: None,
                model_3d: None,
                model_3d_data: None,
                model_3d_rotation: None,
                model_3d_offset: None,
            },
            pins: vec![
                PinDef {
                    num: 1,
                    ..pin_without_pos()
                },
                PinDef {
                    num: 2,
                    ..pin_without_pos()
                },
                PinDef {
                    num: 3,
                    ..pin_without_pos()
                },
            ],
            constraints: vec![],
            mechanical: vec![],
        };
        let pads = pads_from_manifest(&manifest);
        // D2: no pos → pad_type = TH → auto row: pad 1 rect, rest circle.
        assert_eq!(pads[0].pad_shape, PadShape::Rect);
        assert_eq!(pads[1].pad_shape, PadShape::Circle);
        assert_eq!(pads[2].pad_shape, PadShape::Circle);
    }

    /// With explicit thru_hole + no pos, index>0 pads get "circle" default.
    #[test]
    fn pad_from_pin_def_shape_circle_for_th_auto_row() {
        let manifest = Manifest {
            component: ComponentMeta {
                name: "Test".into(),
                title: "Test".into(),
                description: None,
                datasheet: None,
                lib_id: None,
                model_3d: None,
                model_3d_data: None,
                model_3d_rotation: None,
                model_3d_offset: None,
            },
            pins: vec![
                PinDef {
                    num: 1,
                    pad_type: Some("thru_hole".into()),
                    ..pin_without_pos()
                },
                PinDef {
                    num: 2,
                    pad_type: Some("thru_hole".into()),
                    ..pin_without_pos()
                },
                PinDef {
                    num: 3,
                    pad_type: Some("thru_hole".into()),
                    ..pin_without_pos()
                },
            ],
            constraints: vec![],
            mechanical: vec![],
        };
        let pads = pads_from_manifest(&manifest);
        assert_eq!(pads[0].pad_shape, PadShape::Rect);
        assert_eq!(pads[1].pad_shape, PadShape::Circle);
        assert_eq!(pads[2].pad_shape, PadShape::Circle);
    }

    /// Explicit shape is preserved.
    #[test]
    fn pad_from_pin_def_preserves_explicit_shape() {
        let pd = PinDef {
            pad_shape: Some("roundrect".into()),
            roundrect_rratio: Some(0.25),
            ..pin_without_pos()
        };
        let pads = pads_from_manifest(&make_single_pin_manifest(pd));
        assert_eq!(pads[0].pad_shape, PadShape::RoundRect);
        assert_eq!(pads[0].roundrect_rratio, Some(0.25));
    }

    /// Width/height fall back to pin length, then DEFAULT_PAD_SIZE.
    #[test]
    fn pad_from_pin_def_falls_back_width_height_to_length() {
        let with_length = PinDef {
            length: Some(2.0),
            ..pin_without_pos()
        };
        let pads = pads_from_manifest(&make_single_pin_manifest(with_length));
        assert_eq!(pads[0].width, 2.0);
        assert_eq!(pads[0].height, 2.0);

        let bare = pin_without_pos();
        let pads = pads_from_manifest(&make_single_pin_manifest(bare));
        assert_eq!(pads[0].width, fp_geom::DEFAULT_PAD_SIZE);
        assert_eq!(pads[0].height, fp_geom::DEFAULT_PAD_SIZE);
    }

    /// Pin number falls back to pin.num when number is empty.
    #[test]
    fn pad_from_pin_def_falls_back_number_to_num() {
        let pd = PinDef {
            num: 42,
            number: String::new(),
            ..pin_without_pos()
        };
        let pads = pads_from_manifest(&make_single_pin_manifest(pd));
        assert_eq!(pads[0].number, "42");
    }

    /// Explicit number is preserved.
    #[test]
    fn pad_from_pin_def_preserves_explicit_number() {
        let pd = PinDef {
            num: 1,
            number: "A1".into(),
            ..pin_without_pos()
        };
        let pads = pads_from_manifest(&make_single_pin_manifest(pd));
        assert_eq!(pads[0].number, "A1");
    }

    /// Auto-row positions: 2.54 mm pitch starting at origin.
    #[test]
    fn pad_from_pin_def_auto_row_positions() {
        let manifest = Manifest {
            component: ComponentMeta {
                name: "Test".into(),
                title: "Test".into(),
                description: None,
                datasheet: None,
                lib_id: None,
                model_3d: None,
                model_3d_data: None,
                model_3d_rotation: None,
                model_3d_offset: None,
            },
            pins: vec![
                PinDef {
                    num: 1,
                    ..pin_without_pos()
                },
                PinDef {
                    num: 2,
                    ..pin_without_pos()
                },
            ],
            constraints: vec![],
            mechanical: vec![],
        };
        let pads = pads_from_manifest(&manifest);
        assert_eq!(pads[0].pos, (0.0, 0.0));
        assert_eq!(pads[1].pos, (2.54, 0.0));
    }

    /// rotation defaults to 0.0 when not set.
    #[test]
    fn pad_from_pin_def_defaults_rotation_to_zero() {
        let pads = pads_from_manifest(&make_single_pin_manifest(pin_without_pos()));
        assert_eq!(pads[0].rotation, 0.0);
    }

    /// solder_mask_margin passes through (None when unset).
    #[test]
    fn pad_from_pin_def_solder_mask_margin_passthrough() {
        let with_smm = PinDef {
            solder_mask_margin: Some(0.102),
            ..pin_without_pos()
        };
        let pads = pads_from_manifest(&make_single_pin_manifest(with_smm));
        assert_eq!(pads[0].solder_mask_margin, Some(0.102));

        let bare = pin_without_pos();
        let pads = pads_from_manifest(&make_single_pin_manifest(bare));
        assert_eq!(pads[0].solder_mask_margin, None);
    }

    // ── Helpers ──

    fn pin_without_pos() -> PinDef {
        PinDef {
            num: 1,
            number: String::new(),
            name: "P1".into(),
            purpose: "Test".into(),
            notes: String::new(),
            electrical: ElectricalFields {
                kind: "dio".into(),
                ..Default::default()
            },
            pos: None,
            rotation: None,
            length: None,
            width: None,
            height: None,
            pad_type: None,
            pad_shape: None,
            roundrect_rratio: None,
            solder_mask_margin: None,
            layers: None,
            drill: None,
            thermal_vias: vec![],
        }
    }

    fn make_single_pin_manifest(pin: PinDef) -> Manifest {
        Manifest {
            component: ComponentMeta {
                name: "Test".into(),
                title: "Test".into(),
                description: None,
                datasheet: None,
                lib_id: None,
                model_3d: None,
                model_3d_data: None,
                model_3d_rotation: None,
                model_3d_offset: None,
            },
            pins: vec![pin],
            constraints: vec![],
            mechanical: vec![],
        }
    }
}
