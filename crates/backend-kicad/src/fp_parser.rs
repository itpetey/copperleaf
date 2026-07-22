//! KiCad footprint parser.
//!
//! Parses `.kicad_mod` footprint files into structured [`PadDef`] records.
//! This module is intended for the parts-creation CLI; it is not used during
//! compilation or `Backend::emit()`.

use std::path::Path;

use crate::sexpr::{ParseError, Sexpr, parse};

/// A single pad extracted from a KiCad footprint.
#[derive(Clone, Debug, PartialEq)]
pub struct PadDef {
    /// Pad number, matching the `number` field of `sym_parser::PinDef`.
    pub number: String,
    /// Pad position in millimetres.
    pub pos: (f64, f64),
    /// Pad rotation in degrees.
    pub rotation: f64,
    /// Pad width in millimetres.
    pub width: f64,
    /// Pad height in millimetres.
    pub height: f64,
    /// KiCad pad type, e.g. `smd` or `thru_hole`.
    pub pad_type: String,
    /// Pad shape: `rect`, `roundrect`, `circle`, `oval`, `custom`, or empty.
    pub shape: String,
    /// Roundrect corner radius ratio (only for `roundrect` shape).
    pub roundrect_rratio: Option<f64>,
    /// Solder mask margin in millimetres.
    pub solder_mask_margin: Option<f64>,
    /// Copper layers, e.g. `"F.Cu F.Mask F.Paste"` or `"*.Cu"`.
    pub layers: String,
    /// Drill diameter in millimetres (thru_hole pads only).
    pub drill: Option<f64>,
}

/// Parse a single `.kicad_mod` file into a list of pad definitions.
pub fn parse_footprint(path: impl AsRef<Path>) -> Result<Vec<PadDef>, ParseError> {
    let source = std::fs::read_to_string(path.as_ref())?;
    let sexpr = parse(&source)?;
    Ok(extract_pads(&sexpr))
}

/// Parse a `.pretty` footprint library directory, returning one entry per
/// `.kicad_mod` file found inside.
pub fn parse_footprint_lib(
    dir: impl AsRef<Path>,
) -> Result<Vec<(String, Vec<PadDef>)>, ParseError> {
    let dir = dir.as_ref();
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("kicad_mod") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let pads = parse_footprint(&path)?;
        entries.push((name, pads));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(entries)
}

/// Extract the 3D model path from a `.kicad_mod` file's `(model ...)` node.
///
/// Returns the path string (e.g. `"${KICAD10_3DMODEL_DIR}/footprints.3dshapes/QFN-60.step"`)
/// if a model node is found, or `None` otherwise.
pub fn parse_footprint_model(path: impl AsRef<Path>) -> Result<Option<String>, ParseError> {
    let source = std::fs::read_to_string(path.as_ref())?;
    let sexpr = parse(&source)?;
    Ok(extract_model_path(&sexpr))
}

/// Extract the fabrication (body) outline extent from a `.kicad_mod` file's
/// `(fp_line ... (layer F.Fab) ...)` nodes.
///
/// Returns `(x1, y1, x2, y2)` in millimetres, or `None` if there are no
/// F.Fab lines.
pub fn parse_footprint_fab_extent(
    path: impl AsRef<Path>,
) -> Result<Option<(f64, f64, f64, f64)>, ParseError> {
    let source = std::fs::read_to_string(path.as_ref())?;
    let sexpr = parse(&source)?;
    Ok(extract_fab_extent(&sexpr))
}

/// Extract the 3D model path for a named footprint within a `.pretty` library
/// directory.
pub fn parse_footprint_model_lib(
    dir: impl AsRef<Path>,
    footprint_name: &str,
) -> Result<Option<String>, ParseError> {
    let dir = dir.as_ref();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("kicad_mod") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if name == footprint_name {
            return parse_footprint_model(&path);
        }
    }
    Ok(None)
}

fn collect_pads(node: &Sexpr, pads: &mut Vec<PadDef>) {
    let Sexpr::List(children) = node else {
        return;
    };
    if let Some(pad) = parse_pad_node(node) {
        pads.push(pad);
    }
    for child in children {
        collect_pads(child, pads);
    }
}

/// Walk the S-expression tree looking for a `(model <path> ...)` node and
/// return the model path string if found.
fn extract_model_path(node: &Sexpr) -> Option<String> {
    let Sexpr::List(children) = node else {
        return None;
    };
    if let Some(Sexpr::Atom(head)) = children.first()
        && head == "model"
        && let Some(path_node) = children.get(1)
    {
        return Some(path_node.as_string());
    }
    for child in children {
        if let Some(path) = extract_model_path(child) {
            return Some(path);
        }
    }
    None
}

/// Walk the S-expression tree and compute the bounding box of all
/// `(fp_line ... (layer F.Fab) ...)` nodes.
fn extract_fab_extent(node: &Sexpr) -> Option<(f64, f64, f64, f64)> {
    let mut lines: Vec<((f64, f64), (f64, f64))> = Vec::new();
    collect_fab_lines(node, &mut lines);
    if lines.is_empty() {
        return None;
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (start, end) in &lines {
        min_x = min_x.min(start.0).min(end.0);
        min_y = min_y.min(start.1).min(end.1);
        max_x = max_x.max(start.0).max(end.0);
        max_y = max_y.max(start.1).max(end.1);
    }
    Some((min_x, min_y, max_x, max_y))
}

fn collect_fab_lines(node: &Sexpr, lines: &mut Vec<((f64, f64), (f64, f64))>) {
    let Sexpr::List(children) = node else {
        return;
    };
    if let Some(line) = parse_fab_line(node) {
        lines.push(line);
    }
    for child in children {
        collect_fab_lines(child, lines);
    }
}

/// If `node` is an `(fp_line ... (layer F.Fab) ...)`, return its
/// `(start, end)` coordinates.
fn parse_fab_line(node: &Sexpr) -> Option<((f64, f64), (f64, f64))> {
    let Sexpr::List(children) = node else {
        return None;
    };
    if children.is_empty() {
        return None;
    }
    let Sexpr::Atom(head) = &children[0] else {
        return None;
    };
    if head != "fp_line" {
        return None;
    }

    let mut start: Option<(f64, f64)> = None;
    let mut end: Option<(f64, f64)> = None;
    let mut is_fab = false;

    for child in &children[1..] {
        let Sexpr::List(parts) = child else {
            continue;
        };
        if parts.is_empty() {
            continue;
        }
        let Sexpr::Atom(key) = &parts[0] else {
            continue;
        };
        match key.as_str() {
            "start" => {
                let xs = parts.get(1)?.as_string();
                let ys = parts.get(2)?.as_string();
                start = Some((xs.parse().ok()?, ys.parse().ok()?));
            }
            "end" => {
                let xs = parts.get(1)?.as_string();
                let ys = parts.get(2)?.as_string();
                end = Some((xs.parse().ok()?, ys.parse().ok()?));
            }
            "layer" => {
                if let Some(layer) = parts.get(1) {
                    if layer.as_string() == "F.Fab" {
                        is_fab = true;
                    }
                }
            }
            _ => {}
        }
    }

    if is_fab {
        Some((start?, end?))
    } else {
        None
    }
}

fn extract_pads(sexpr: &Sexpr) -> Vec<PadDef> {
    let mut pads = Vec::new();
    collect_pads(sexpr, &mut pads);
    pads
}

fn parse_pad_node(node: &Sexpr) -> Option<PadDef> {
    let Sexpr::List(children) = node else {
        return None;
    };
    if children.is_empty() {
        return None;
    }
    let Sexpr::Atom(head) = &children[0] else {
        return None;
    };
    if head != "pad" {
        return None;
    }

    let number = children.get(1)?.as_string();
    let pad_type = children.get(2)?.as_string();
    let shape = children.get(3)?.as_string();

    let mut pos = (0.0, 0.0);
    let mut rotation = 0.0;
    let mut width = 0.0;
    let mut height = 0.0;
    let mut roundrect_rratio = None;
    let mut solder_mask_margin = None;
    let mut layers = String::new();
    let mut drill = None;

    for child in &children[4..] {
        let Sexpr::List(parts) = child else {
            continue;
        };
        if parts.is_empty() {
            continue;
        }
        let Sexpr::Atom(key) = &parts[0] else {
            continue;
        };
        match key.as_str() {
            "at" => {
                let xs = parts.get(1)?.as_string();
                let ys = parts.get(2)?.as_string();
                pos.0 = xs.parse().ok()?;
                pos.1 = ys.parse().ok()?;
                if let Some(rs) = parts.get(3) {
                    rotation = rs.as_string().parse().ok()?;
                }
            }
            "size" => {
                let ws = parts.get(1)?.as_string();
                let hs = parts.get(2)?.as_string();
                width = ws.parse().ok()?;
                height = hs.parse().ok()?;
            }
            "roundrect_rratio" => {
                if let Some(rs) = parts.get(1) {
                    roundrect_rratio = rs.as_string().parse().ok();
                }
            }
            "solder_mask_margin" => {
                if let Some(ms) = parts.get(1) {
                    solder_mask_margin = ms.as_string().parse().ok();
                }
            }
            "layers" => {
                let layer_strs: Vec<String> = parts[1..].iter().map(|s| s.as_string()).collect();
                layers = layer_strs.join(" ");
            }
            "drill" => {
                if let Some(ds) = parts.get(1) {
                    drill = ds.as_string().parse().ok();
                }
            }
            _ => {}
        }
    }

    Some(PadDef {
        number,
        pos,
        rotation,
        width,
        height,
        pad_type,
        shape,
        roundrect_rratio,
        solder_mask_margin,
        layers,
        drill,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_footprint() -> &'static str {
        r#"(footprint "QFN-60"
  (pad "1" smd rect (at -2.54 3.81 90.0) (size 0.5 0.25) (layers F.Cu F.Mask F.Paste))
  (pad "2" smd roundrect (roundrect_rratio 0.125) (at -1.27 3.81) (size 0.5 0.25) (layers F.Cu F.Mask F.Paste) (solder_mask_margin 0.102))
  (pad "60" smd rect (at 2.54 -3.81 180.0) (size 0.5 0.25) (layers F.Cu F.Mask F.Paste))
)"#
    }

    #[test]
    fn parse_extracts_pads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.kicad_mod");
        std::fs::write(&path, sample_footprint()).unwrap();
        let pads = parse_footprint(&path).unwrap();
        assert_eq!(pads.len(), 3);

        assert_eq!(pads[0].number, "1");
        assert_eq!(pads[0].pad_type, "smd");
        assert_eq!(pads[0].shape, "rect");
        assert!((pads[0].pos.0 - -2.54).abs() < 1e-9);
        assert!((pads[0].pos.1 - 3.81).abs() < 1e-9);
        assert!((pads[0].rotation - 90.0).abs() < 1e-9);
        assert!((pads[0].width - 0.5).abs() < 1e-9);
        assert!((pads[0].height - 0.25).abs() < 1e-9);
        assert_eq!(pads[0].layers, "F.Cu F.Mask F.Paste");
        assert!(pads[0].roundrect_rratio.is_none());
        assert!(pads[0].solder_mask_margin.is_none());
        assert!(pads[0].drill.is_none());

        assert_eq!(pads[1].shape, "roundrect");
        assert!((pads[1].roundrect_rratio.unwrap() - 0.125).abs() < 1e-9);
        assert!((pads[1].solder_mask_margin.unwrap() - 0.102).abs() < 1e-9);
    }

    #[test]
    fn empty_footprint_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.kicad_mod");
        std::fs::write(&path, "(footprint \"EMPTY\")").unwrap();
        let pads = parse_footprint(&path).unwrap();
        assert!(pads.is_empty());
    }

    #[test]
    fn parse_lib_lists_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("A.kicad_mod"), sample_footprint()).unwrap();
        std::fs::write(dir.path().join("B.kicad_mod"), "(footprint \"B\")").unwrap();
        let libs = parse_footprint_lib(dir.path()).unwrap();
        assert_eq!(libs.len(), 2);
        assert_eq!(libs[0].0, "A");
        assert_eq!(libs[0].1.len(), 3);
        assert_eq!(libs[1].0, "B");
        assert!(libs[1].1.is_empty());
    }
}
