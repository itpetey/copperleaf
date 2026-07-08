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
    /// Pins belonging to the symbol (after `extends` resolution).
    pub pins: Vec<PinDef>,
    /// Optional parent symbol name from `(extends "Parent")`, if any.
    pub extends: Option<String>,
    /// Optional default footprint (e.g. `"Package_SOIC:SOIC-8_3.9x4.9mm_P1.27mm"`).
    pub footprint: Option<String>,
    /// The full raw S-expression tree of this symbol, for embedding verbatim
    /// in schematic `lib_symbols` sections.
    pub raw: Sexpr,
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
///
/// After parsing, any `(extends "ParentName")` references are resolved so that
/// child symbols inherit all pins from their parent symbol.
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

    // Resolve (extends ...) inheritance chains.
    resolve_extends(&mut symbols);

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
        if footprint.is_none()
            && let Some(fp) = parse_footprint_property(node)
        {
            *footprint = Some(fp);
            continue;
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

    // Detect (extends "ParentName") in the top-level children.
    let mut extends: Option<String> = None;
    for child in &children[2..] {
        if let Sexpr::List(parts) = child
            && parts.len() == 2
            && let Sexpr::Atom(key) = &parts[0]
            && key == "extends"
        {
            extends = Some(string_value(&parts[1]));
        }
    }

    let mut pins = Vec::new();
    let mut footprint = None;
    collect_pins_and_footprint(&children[2..], &mut pins, &mut footprint);

    Some(SymbolDef {
        lib_id,
        pins,
        extends,
        footprint,
        raw: node.clone(),
    })
}

/// Resolve `(extends "ParentName")` chains by merging inherited pins into
/// child symbols. Performs a topological walk so that multi-level inheritance
/// (A extends B extends C) works correctly.
///
/// Parent symbols are identified by stripping any library prefix from the
/// extends target, matching the same logic as [`find_symbol`].
fn resolve_extends(symbols: &mut Vec<SymbolDef>) {
    // Build a lookup map.
    let mut by_name: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (i, sym) in symbols.iter().enumerate() {
        by_name.insert(sym.lib_id.clone(), i);
    }

    // Resolved flags to handle diamond inheritance / cycles.
    let mut resolved: Vec<bool> = vec![false; symbols.len()];

    fn resolve_one(
        i: usize,
        symbols: &mut [SymbolDef],
        by_name: &std::collections::HashMap<String, usize>,
        resolved: &mut Vec<bool>,
        visited: &mut std::collections::HashSet<usize>,
    ) {
        if resolved[i] {
            return;
        }
        let parent_name = match symbols[i].extends.clone() {
            Some(ref name) => name.clone(),
            None => {
                resolved[i] = true;
                return;
            }
        };

        // Find parent (strip library prefix if present).
        let parent_lib_id = parent_name.split(':').next_back().unwrap_or(&parent_name).to_string();
        let Some(&parent_idx) = by_name.get(&parent_lib_id) else {
            // Parent symbol not in this library — cannot resolve.
            resolved[i] = true;
            return;
        };

        if parent_idx == i {
            // Self-referencing extends — skip.
            resolved[i] = true;
            return;
        }

        // Cycle detection.
        if !visited.insert(i) {
            // Cycle detected; skip.
            resolved[i] = true;
            return;
        }

        // Resolve parent first.
        resolve_one(parent_idx, symbols, by_name, resolved, visited);

        // Merge parent's pins into child (avoiding duplicates by pin name).
        let parent_pins = symbols[parent_idx].pins.clone();
        let child_pin_names: std::collections::HashSet<String> =
            symbols[i].pins.iter().map(|p| p.name.clone()).collect();
        for pin in &parent_pins {
            if !child_pin_names.contains(&pin.name) {
                symbols[i].pins.push(pin.clone());
            }
        }

        resolved[i] = true;
    }

    for i in 0..symbols.len() {
        let mut visited = std::collections::HashSet::new();
        resolve_one(i, symbols, &by_name, &mut resolved, &mut visited);
    }
}

/// Flatten a symbol that uses `(extends "ParentName")` into a self-contained
/// symbol by removing the `extends` reference and copying the parent's
/// sub-symbols (units with pins and graphics) into the child.
///
/// This is needed when embedding extends-based symbols in `lib_symbols`
/// without the parent definition — KiCad cannot resolve the extends reference
/// from the schematic alone.
pub fn flatten_extends(sym: &SymbolDef, symbols: &[SymbolDef]) -> Sexpr {
    let Sexpr::List(mut children) = sym.raw.clone() else {
        return sym.raw.clone();
    };

    let parent_name = match &sym.extends {
        Some(name) => name.split(':').next_back().unwrap_or(name).to_string(),
        None => return sym.raw.clone(),
    };

    let Some(parent) = symbols.iter().find(|s| s.lib_id == parent_name) else {
        return sym.raw.clone();
    };

    // Remove the (extends "ParentName") node.
    children.retain(|child| match child {
        Sexpr::List(parts) if parts.len() >= 2 => {
            match &parts[0] {
                Sexpr::Atom(key) => key != "extends",
                _ => true,
            }
        }
        _ => true,
    });

    // Copy parent's sub-symbol units and graphical elements into the child.
    // Sub-symbols must be renamed from "ParentName_*" to "ChildName_*" because
    // KiCad validates that sub-symbol unit names start with the symbol name.
    let child_prefix = format!("{}_", sym.lib_id);
    let parent_prefix = format!("{}_", parent_name);
    if let Sexpr::List(parent_children) = &parent.raw {
        for pchild in &parent_children[2..] {
            let include = match pchild {
                Sexpr::List(parts) if parts.len() >= 2 => {
                    match &parts[0] {
                        Sexpr::Atom(key) if key == "symbol" => true,
                        Sexpr::Atom(key) if matches!(key.as_str(), "polyline" | "rectangle" | "circle" | "arc" | "pin") => true,
                        _ => false,
                    }
                }
                _ => false,
            };
            if !include {
                continue;
            }
            // Rename sub-symbols from parent prefix to child prefix.
            let renamed = match pchild {
                Sexpr::List(parts) if parts.len() >= 2
                    && matches!(&parts[0], Sexpr::Atom(key) if key == "symbol")
                => {
                    if let Sexpr::Atom(name) = &parts[1] {
                        let name_unquoted = name.trim_matches('"');
                        if let Some(rest) = name_unquoted.strip_prefix(&parent_prefix) {
                            let new_name = format!("{}{}", child_prefix, rest);
                            let mut new_parts = parts.clone();
                            new_parts[1] = Sexpr::str(&new_name);
                            Sexpr::List(new_parts)
                        } else {
                            pchild.clone()
                        }
                    } else {
                        pchild.clone()
                    }
                }
                _ => pchild.clone(),
            };
            if !children.contains(&renamed) {
                children.push(renamed);
            }
        }
    }

    // Merge multiple conversions of the same unit into a single conversion.
    // Some KiCad symbols (e.g. RP2350A/RP2354A) split pins across body styles
    // named like `<base>_<unit>_<convert>`. A single schematic instance can only
    // show one conversion, so pins in other conversions become invisible and
    // appear unconnected. Flatten them so one `(unit N)` instance shows all pins.
    children = merge_conversions(children, &child_prefix);

    Sexpr::List(children)
}

/// Merge sub-symbols that share the same unit but have different conversions
/// (e.g. `RP2354A_1_0` and `RP2354A_1_1`) into a single sub-symbol named
/// `<prefix><unit>_1`.
fn merge_conversions(children: Vec<Sexpr>, child_prefix: &str) -> Vec<Sexpr> {
    let mut result = Vec::with_capacity(children.len());
    let mut unit_groups: std::collections::HashMap<i64, Vec<(i64, Vec<Sexpr>)>> =
        std::collections::HashMap::new();

    for child in children {
        if let Sexpr::List(parts) = &child {
            if parts.len() >= 2 && matches!(&parts[0], Sexpr::Atom(key) if key == "symbol") {
                if let Sexpr::Atom(name) = &parts[1] {
                    let name_unquoted = name.trim_matches('"');
                    if let Some(rest) = name_unquoted.strip_prefix(child_prefix) {
                        // Parse "<unit>_<convert>" from the end of the name.
                        let mut split = rest.rsplitn(3, '_');
                        let convert_str = split.next();
                        let unit_str = split.next();
                        if let (Some(c), Some(u)) = (convert_str, unit_str) {
                            if let (Ok(unit), Ok(convert)) = (u.parse::<i64>(), c.parse::<i64>()) {
                                if unit > 0 {
                                    let inner = parts[2..].to_vec();
                                    unit_groups
                                        .entry(unit)
                                        .or_default()
                                        .push((convert, inner));
                                    continue;
                                }
                            }
                        }
                    }
                }
            }
        }
        result.push(child);
    }

    for (unit, mut conversions) in unit_groups {
        if conversions.len() <= 1 {
            // Re-create the original sub-symbol unchanged.
            let (convert, inner) = conversions.into_iter().next().unwrap();
            let name = format!("{}{}_{}", child_prefix, unit, convert);
            let mut parts = vec![Sexpr::atom("symbol"), Sexpr::str(&name)];
            parts.extend(inner);
            result.push(Sexpr::List(parts));
            continue;
        }

        // Sort by conversion number so the order is deterministic.
        conversions.sort_by_key(|(c, _)| *c);

        let mut merged_inner: Vec<Sexpr> = Vec::new();
        for (_convert, inner) in conversions {
            for item in inner {
                if !merged_inner.contains(&item) {
                    merged_inner.push(item);
                }
            }
        }

        let name = format!("{}{}_1", child_prefix, unit);
        let mut parts = vec![Sexpr::atom("symbol"), Sexpr::str(&name)];
        parts.extend(merged_inner);
        result.push(Sexpr::List(parts));
    }

    result
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

    #[test]
    fn flatten_extends_merges_split_conversions() {
        // RP2350A/RP2354A style: pins are split across `_1_0` (power) and
        // `_1_1` (signals). A single `(unit 1)` instance can only show one
        // conversion, so flattening must merge them so all pins are visible.
        let lib = r#"(kicad_symbol_lib
  (symbol "Parent"
    (symbol "Parent_0_1"
      (rectangle (start -5.08 -5.08) (end 5.08 5.08))
    )
    (symbol "Parent_1_0"
      (pin power_in line (at -2.54 -5.08 90) (length 2.54) (name "VREG_PGND") (number "1"))
    )
    (symbol "Parent_1_1"
      (pin power_in line (at 0 -5.08 90) (length 2.54) (name "GND") (number "2"))
      (pin bidirectional line (at 5.08 0 180) (length 2.54) (name "GPIO0") (number "3"))
    )
  )
  (symbol "Child" (extends "Parent")
    (property "Reference" "U" (at 0 0 0))
  )
)"#;
        let symbols = parse_symbol_lib(lib).unwrap();
        let child = symbols.iter().find(|s| s.lib_id == "Child").unwrap();
        let flat = flatten_extends(child, &symbols);
        let flat_str = format!("{}", flat);
        // Only the merged conversion should remain for unit 1.
        assert!(flat_str.contains("\"Child_1_1\""));
        assert!(!flat_str.contains("\"Child_1_0\""));
        // Both power and signal pins must be present in the merged unit.
        assert!(flat_str.contains("(name \"VREG_PGND\")"));
        assert!(flat_str.contains("(name \"GND\")"));
        assert!(flat_str.contains("(name \"GPIO0\")"));
        // Common graphics must still be present.
        assert!(flat_str.contains("\"Child_0_1\""));
        assert!(flat_str.contains("rectangle"));
    }
}
