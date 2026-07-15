//! KiCad footprint parser.
//!
//! Parses `.kicad_mod` footprint files into structured [`PadDef`] records.
//! This module is intended for the parts-creation CLI; it is not used during
//! `Board::compile()` or `Backend::emit()`.

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

    let number = string_value(children.get(1)?);
    let pad_type = string_value(children.get(2)?);

    let mut pos = (0.0, 0.0);
    let mut rotation = 0.0;
    let mut width = 0.0;
    let mut height = 0.0;

    for child in &children[3..] {
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
                let xs = string_value(parts.get(1)?);
                let ys = string_value(parts.get(2)?);
                pos.0 = xs.parse().ok()?;
                pos.1 = ys.parse().ok()?;
                if let Some(rs) = parts.get(3) {
                    rotation = string_value(rs).parse().ok()?;
                }
            }
            "size" => {
                let ws = string_value(parts.get(1)?);
                let hs = string_value(parts.get(2)?);
                width = ws.parse().ok()?;
                height = hs.parse().ok()?;
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
    })
}

fn string_value(expr: &Sexpr) -> String {
    match expr {
        Sexpr::Atom(s) => {
            if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
                s[1..s.len() - 1]
                    .replace("\\\"", "\"")
                    .replace("\\\\", "\\")
            } else {
                s.clone()
            }
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_footprint() -> &'static str {
        r#"(footprint "QFN-60"
  (pad "1" smd rect (at -2.54 3.81 90.0) (size 0.5 0.25))
  (pad "2" smd rect (at -1.27 3.81) (size 0.5 0.25))
  (pad "60" smd rect (at 2.54 -3.81 180.0) (size 0.5 0.25))
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
        assert!((pads[0].pos.0 - -2.54).abs() < 1e-9);
        assert!((pads[0].pos.1 - 3.81).abs() < 1e-9);
        assert!((pads[0].rotation - 90.0).abs() < 1e-9);
        assert!((pads[0].width - 0.5).abs() < 1e-9);
        assert!((pads[0].height - 0.25).abs() < 1e-9);
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
