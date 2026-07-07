//! KiCad symbol library parser.
//!
//! Reads `.kicad_sym` files (S-expressions) and extracts symbol names plus
//! per-pin geometry: position, rotation, type, and length.

use crate::sexpr::{ParseError, Sexpr, parse};

/// A pin extracted from a KiCad symbol definition.
#[derive(Clone, Debug, PartialEq)]
pub struct PinDef {
    /// Pin name, e.g. `"VDD"`.
    pub name: String,
    /// Pin number as a string, e.g. `"1"`.
    pub number: String,
    /// Pin position relative to the symbol origin, in mm.
    pub pos: (f64, f64),
    /// Pin rotation in degrees.
    pub rotation: f64,
    /// KiCad electrical type, e.g. `"power_in"`.
    pub pin_type: String,
    /// Pin stub length in mm.
    pub length: f64,
}

/// A symbol extracted from a KiCad symbol library file.
#[derive(Clone, Debug, PartialEq)]
pub struct SymbolDef {
    /// Library ID, e.g. `"RP2354a"`.
    pub lib_id: String,
    /// Pins belonging to the symbol.
    pub pins: Vec<PinDef>,
    /// Optional default footprint (e.g. `"Package_SOIC:SOIC-8_3.9x4.9mm_P1.27mm"`).
    pub footprint: Option<String>,
}

/// Find a symbol definition by library ID.
///
/// `lib_id` may include a library nickname prefix (e.g. `"RP2040:RP2354a"`).
/// The prefix is stripped before matching because a `.kicad_sym` file contains
/// only the symbol name, not the library nickname.
pub fn find_symbol<'a>(symbols: &'a [SymbolDef], lib_id: &str) -> Option<&'a SymbolDef> {
    let name = lib_id.split(':').next_back().unwrap_or(lib_id);
    symbols.iter().find(|s| s.lib_id == name)
}

/// Parse a `.kicad_sym` file contents into a list of [`SymbolDef`]s.
pub fn parse_symbol_lib(input: &str) -> Result<Vec<SymbolDef>, ParseError> {
    let sexpr = parse(input)?;
    let Sexpr::List(nodes) = &sexpr else {
        return Ok(vec![]);
    };

    let mut symbols = Vec::new();
    for node in nodes {
        if let Some(sym) = parse_symbol_node(node) {
            symbols.push(sym);
        }
    }
    Ok(symbols)
}

/// Recursively collect `(pin ...)` and `(property "Footprint" ...)` nodes from
/// a symbol tree. KiCad nests pins inside unit sub-symbols such as
/// `(symbol "Name_0_1" (pin ...) ...)`.
fn collect_pins_and_footprint(
    nodes: &[Sexpr],
    pins: &mut Vec<PinDef>,
    footprint: &mut Option<String>,
) {
    for node in nodes {
        if let Some(pin) = parse_pin_node(node) {
            pins.push(pin);
            continue;
        }
        if footprint.is_none() {
            if let Some(fp) = parse_footprint_property(node) {
                *footprint = Some(fp);
                continue;
            }
        }
        // Recurse into nested `(symbol ...)` sub-nodes.
        if let Sexpr::List(children) = node
            && let Some(Sexpr::Atom(head)) = children.first()
            && head == "symbol"
            && children.len() > 2
        {
            collect_pins_and_footprint(&children[2..], pins, footprint);
        }
    }
}

/// Try to parse a `(property "Footprint" "value" ...)` node and return the
/// footprint value if found.
fn parse_footprint_property(node: &Sexpr) -> Option<String> {
    let Sexpr::List(children) = node else {
        return None;
    };
    if children.len() < 3 {
        return None;
    }
    let Sexpr::Atom(head) = &children[0] else {
        return None;
    };
    if head != "property" {
        return None;
    }
    let key = string_value(children.get(1)?);
    if key != "Footprint" {
        return None;
    }
    let val = string_value(children.get(2)?);
    if val.is_empty() {
        return None;
    }
    Some(val)
}

fn parse_pin_node(node: &Sexpr) -> Option<PinDef> {
    let Sexpr::List(children) = node else {
        return None;
    };
    if children.is_empty() {
        return None;
    }
    let Sexpr::Atom(head) = &children[0] else {
        return None;
    };
    if head != "pin" {
        return None;
    }

    // Expected: (pin <type> line (at x y rot) (length L) (name "...") (number "...") ...)
    let pin_type = string_value(children.get(1)?);
    let mut pos = (0.0, 0.0);
    let mut rotation = 0.0;
    let mut length = 2.54;
    let mut name = String::new();
    let mut number = String::new();

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
                let rs = string_value(parts.get(3)?);
                pos.0 = xs.parse().ok()?;
                pos.1 = ys.parse().ok()?;
                rotation = rs.parse().ok()?;
            }
            "length" => {
                length = string_value(parts.get(1)?).parse().ok()?;
            }
            "name" => {
                name = string_value(parts.get(1)?);
            }
            "number" => {
                number = string_value(parts.get(1)?);
            }
            _ => {}
        }
    }

    Some(PinDef {
        name,
        number,
        pos,
        rotation,
        pin_type,
        length,
    })
}

fn parse_symbol_node(node: &Sexpr) -> Option<SymbolDef> {
    let Sexpr::List(children) = node else {
        return None;
    };
    if children.is_empty() {
        return None;
    }
    let Sexpr::Atom(head) = &children[0] else {
        return None;
    };
    if head != "symbol" {
        return None;
    }

    let lib_id = string_value(children.get(1)?);

    let mut pins = Vec::new();
    let mut footprint = None;
    collect_pins_and_footprint(&children[2..], &mut pins, &mut footprint);

    Some(SymbolDef {
        lib_id,
        pins,
        footprint,
    })
}

fn string_value(expr: &Sexpr) -> String {
    match expr {
        Sexpr::Atom(s) => {
            // Strip surrounding quotes added by Sexpr::str.
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

    fn sample_lib() -> &'static str {
        r#"(kicad_symbol_lib
  (symbol "RP2354a"
    (pin power_in line (at -15.24 5.08 0) (length 2.54) (name "VDD") (number "1"))
    (pin power_in line (at -15.24 -5.08 0) (length 2.54) (name "GND") (number "2"))
  )
  (symbol "RP2040"
    (pin bidirectional line (at 10.16 0 180) (length 3.81) (name "GPIO0") (number "3"))
  )
)"#
    }

    #[test]
    fn parse_extracts_symbol_names() {
        let symbols = parse_symbol_lib(sample_lib()).unwrap();
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].lib_id, "RP2354a");
        assert_eq!(symbols[1].lib_id, "RP2040");
    }

    #[test]
    fn parse_extracts_pin_geometry() {
        let symbols = parse_symbol_lib(sample_lib()).unwrap();
        let vdd = &symbols[0].pins[0];
        assert_eq!(vdd.name, "VDD");
        assert_eq!(vdd.number, "1");
        assert!((vdd.pos.0 - -15.24).abs() < 1e-9);
        assert!((vdd.pos.1 - 5.08).abs() < 1e-9);
        assert!((vdd.rotation - 0.0).abs() < 1e-9);
        assert!((vdd.length - 2.54).abs() < 1e-9);
        assert_eq!(vdd.pin_type, "power_in");
    }

    #[test]
    fn parse_extracts_multiple_pins() {
        let symbols = parse_symbol_lib(sample_lib()).unwrap();
        assert_eq!(symbols[0].pins.len(), 2);
        assert_eq!(symbols[0].pins[1].name, "GND");
    }

    #[test]
    fn find_symbol_strips_library_prefix() {
        let symbols = parse_symbol_lib(sample_lib()).unwrap();
        assert!(find_symbol(&symbols, "RP2040:RP2354a").is_some());
        assert!(find_symbol(&symbols, "RP2354a").is_some());
        assert!(find_symbol(&symbols, "Missing").is_none());
    }

    #[test]
    fn parse_extracts_pins_from_nested_unit_symbol() {
        let lib = r#"(kicad_symbol_lib
  (symbol "RP2354a"
    (symbol "RP2354a_0_1"
      (pin power_in line (at -15.24 5.08 0) (length 2.54) (name "VDD") (number "1"))
    )
  )
)"#;
        let symbols = parse_symbol_lib(lib).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].pins.len(), 1);
        assert_eq!(symbols[0].pins[0].name, "VDD");
    }

    #[test]
    fn parse_uses_defaults_for_missing_fields() {
        let lib = r#"(kicad_symbol_lib
  (symbol "Bad"
    (pin power_in line (at -15.24 5.08 0) (length 2.54) (name "VDD"))
    (pin power_in line (at -15.24 -5.08 0) (length 2.54) (number "2"))
    (pin power_in line (length 2.54) (name "NO_AT") (number "3"))
  )
)"#;
        let symbols = parse_symbol_lib(lib).unwrap();
        let pins = &symbols[0].pins;
        assert_eq!(pins.len(), 3);
        // Missing number defaults to empty string.
        assert_eq!(pins[0].name, "VDD");
        assert_eq!(pins[0].number, "");
        // Missing name defaults to empty string.
        assert_eq!(pins[1].name, "");
        assert_eq!(pins[1].number, "2");
        // Missing (at ...) defaults to (0, 0) and 0 rotation.
        assert_eq!(pins[2].name, "NO_AT");
        assert_eq!(pins[2].pos, (0.0, 0.0));
        assert_eq!(pins[2].rotation, 0.0);
    }

    #[test]
    fn parse_extracts_footprint_property() {
        let lib = r#"(kicad_symbol_lib
  (symbol "SOIC8"
    (property "Reference" "U" (at 0 0 0) (effects (font (size 1.27 1.27))))
    (property "Value" "SOIC8" (at 0 0 0) (effects (font (size 1.27 1.27))))
    (property "Footprint" "Package_SOIC:SOIC-8_3.9x4.9mm_P1.27mm" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))
    (pin power_in line (at 0 0 180) (length 2.54) (name "VDD") (number "1"))
  )
  (symbol "NoFootprint"
    (pin power_in line (at 0 0 180) (length 2.54) (name "VIN") (number "1"))
  )
)"#;
        let symbols = parse_symbol_lib(lib).unwrap();
        assert_eq!(
            symbols[0].footprint,
            Some("Package_SOIC:SOIC-8_3.9x4.9mm_P1.27mm".to_string())
        );
        assert_eq!(symbols[1].footprint, None);
    }

    #[test]
    fn parse_extracts_footprint_from_nested_unit_symbol() {
        let lib = r#"(kicad_symbol_lib
  (symbol "OpAmp"
    (property "Footprint" "Package_DIP:DIP-8_W7.62mm" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))
    (symbol "OpAmp_0_1"
      (pin input line (at -5.08 2.54 0) (length 2.54) (name "IN+") (number "3"))
    )
  )
)"#;
        let symbols = parse_symbol_lib(lib).unwrap();
        assert_eq!(
            symbols[0].footprint,
            Some("Package_DIP:DIP-8_W7.62mm".to_string())
        );
        assert_eq!(symbols[0].pins.len(), 1);
        assert_eq!(symbols[0].pins[0].name, "IN+");
    }
}
