//! KiCad footprint emitter.
//!
//! Generates `.kicad_mod` files from Copperleaf [`Manifest`] data so the part
//! TOML can serve as the single source of truth for a component's footprint.

use copperleaf_part_codegen::{Manifest, MechanicalDef, PinDef};

use crate::{
    fp_geom::{self, PadGeom},
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
use base64::Engine as _;

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

    let mut children = Vec::new();

    // Footprint header.
    children.push(Sexpr::str(name));
    children.push(Sexpr::list([
        Sexpr::atom("version"),
        Sexpr::atom("20231218"),
    ]));
    children.push(Sexpr::list([
        Sexpr::atom("generator"),
        Sexpr::str("copperleaf"),
    ]));
    children.push(Sexpr::list([Sexpr::atom("layer"), Sexpr::atom("F.Cu")]));
    children.push(Sexpr::list([Sexpr::atom("tedit"), Sexpr::atom("00000000")]));
    // KLC F9.1: the description should carry the datasheet URL when known.
    let descr = match (
        &manifest.component.description,
        &manifest.component.datasheet,
    ) {
        (Some(d), Some(url)) => format!("{}, {}", d, url),
        (Some(d), None) => d.clone(),
        (None, Some(url)) => url.clone(),
        (None, None) => String::new(),
    };
    children.push(Sexpr::list([Sexpr::atom("descr"), Sexpr::str(&descr)]));
    children.push(Sexpr::list([Sexpr::atom("tags"), Sexpr::str("copperleaf")]));
    children.push(Sexpr::list([
        Sexpr::atom("attr"),
        Sexpr::atom(fp_geom::footprint_attr(&pads)),
    ]));

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
    let rot = manifest.component.model_3d_rotation.unwrap_or((0.0, 0.0, 0.0));
    let off = manifest.component.model_3d_offset.unwrap_or((0.0, 0.0, 0.0));
    children.push(fp_geom::model_sexpr(
        name,
        model_path_for_sexpr.as_deref(),
        off,
        rot,
    ));

    Ok(Sexpr::list(
        [Sexpr::atom("footprint").into()]
            .into_iter()
            .chain(children),
    )
    .to_string())
}/// Collect all pads for a manifest: electrical pins (with thermal vias)
/// followed by mechanical pads.
pub fn pads_from_manifest(manifest: &Manifest) -> Vec<PadGeom> {
    let mut pads: Vec<PadGeom> = Vec::new();

    for (i, pin) in manifest.pins.iter().enumerate() {
        pads.push(pad_from_pin_def(pin, i));
        // Thermal vias are emitted as un-numbered through-holes on all copper.
        for via in &pin.thermal_vias {
            pads.push(fp_geom::thermal_via_pad(via.pos, via.drill, via.size));
        }
    }

    for mech in &manifest.mechanical {
        pads.push(pad_from_mechanical_def(mech));
    }

    fp_geom::normalise_anchor(&mut pads);
    pads
}

fn pad_from_mechanical_def(mech: &MechanicalDef) -> PadGeom {
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

fn pad_from_pin_def(pin: &PinDef, index: usize) -> PadGeom {
    let pos = pin.pos.unwrap_or_else(|| fp_geom::auto_pad_pos(index));
    let number = if pin.number.is_empty() {
        pin.num.to_string()
    } else {
        pin.number.clone()
    };
    let pad_type = pin.pad_type.clone().unwrap_or_else(|| "smd".to_string());
    let default_shape = if pin.pos.is_some() || pad_type != "thru_hole" {
        "rect"
    } else if index == 0 {
        "rect"
    } else {
        "circle"
    };
    PadGeom {
        number,
        pos,
        rotation: pin.rotation.unwrap_or(0.0),
        width: pin
            .width
            .or(pin.length)
            .unwrap_or(fp_geom::DEFAULT_PAD_SIZE),
        height: pin
            .height
            .or(pin.length)
            .unwrap_or(fp_geom::DEFAULT_PAD_SIZE),
        pad_type: pad_type.clone(),
        shape: pin
            .pad_shape
            .clone()
            .unwrap_or_else(|| default_shape.to_string()),
        roundrect_rratio: pin.roundrect_rratio,
        layers: pin.layers.clone().unwrap_or_else(|| {
            if pad_type == "thru_hole" || pad_type == "np_thru_hole" {
                fp_geom::PTH_LAYERS.to_string()
            } else {
                fp_geom::SMD_LAYERS.to_string()
            }
        }),
        drill: pin.drill,
        solder_mask_margin: pin.solder_mask_margin,
        pin_index: Some(index),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf_part_codegen::{ComponentMeta, PinDef};

    fn pin_def(num: usize, name: &str, pos: (f64, f64)) -> PinDef {
        PinDef {
            num,
            number: num.to_string(),
            name: name.into(),
            purpose: "Test".into(),
            notes: String::new(),
            kind: "dio".into(),
            bw_mhz: None,
            v: None,
            v_min: None,
            v_max: None,
            i: None,
            i_max: None,
            pos: Some(pos),
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
}
