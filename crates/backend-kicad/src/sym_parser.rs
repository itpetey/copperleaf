//! KiCad symbol library parser.

use crate::sexpr::{ParseError, Sexpr, parse};

/// A pin extracted from a KiCad symbol definition.
#[derive(Clone, Debug, PartialEq)]
pub struct PinDef {
    pub name: String,
    pub number: String,
    pub pos: (f64, f64),
    pub rotation: f64,
    pub pin_type: String,
    pub length: f64,
}

/// A symbol extracted from a KiCad symbol library file.
#[derive(Clone, Debug, PartialEq)]
pub struct SymbolDef {
    pub lib_id: String,
    pub pins: Vec<PinDef>,
    pub extends: Option<String>,
    pub footprint: Option<String>,
    pub datasheet: Option<String>,
    pub raw: Sexpr,
}

pub fn find_symbol<'a>(symbols: &'a [SymbolDef], lib_id: &str) -> Option<&'a SymbolDef> {
    let name = lib_id.split(':').next_back().unwrap_or(lib_id);
    symbols.iter().find(|s| s.lib_id == name)
}

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

    children.retain(|child| match child {
        Sexpr::List(parts) if parts.len() >= 2 => match &parts[0] {
            Sexpr::Atom(key) => key != "extends",
            _ => true,
        },
        _ => true,
    });

    let child_prefix = format!("{}_", sym.lib_id);
    let parent_prefix = format!("{}_", parent_name);
    if let Sexpr::List(parent_children) = &parent.raw {
        for pchild in &parent_children[2..] {
            let include = match pchild {
                Sexpr::List(parts) if parts.len() >= 2 => match &parts[0] {
                    Sexpr::Atom(key) if key == "symbol" => true,
                    Sexpr::Atom(key)
                        if matches!(
                            key.as_str(),
                            "polyline" | "rectangle" | "circle" | "arc" | "pin"
                        ) =>
                    {
                        true
                    }
                    _ => false,
                },
                _ => false,
            };
            if !include {
                continue;
            }
            let renamed = match pchild {
                Sexpr::List(parts)
                    if parts.len() >= 2
                        && matches!(&parts[0], Sexpr::Atom(key) if key == "symbol") =>
                {
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

    children = merge_conversions(children, &child_prefix);
    Sexpr::List(children)
}

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

    resolve_extends(&mut symbols);
    Ok(symbols)
}

fn collect_pins_and_properties(
    nodes: &[Sexpr],
    pins: &mut Vec<PinDef>,
    footprint: &mut Option<String>,
    datasheet: &mut Option<String>,
) {
    for node in nodes {
        if let Some(pin) = parse_pin_node(node) {
            pins.push(pin);
            continue;
        }
        if footprint.is_none()
            && let Some(fp) = parse_property_value(node, "Footprint")
        {
            *footprint = Some(fp);
            continue;
        }
        if datasheet.is_none()
            && let Some(ds) = parse_property_value(node, "Datasheet")
        {
            *datasheet = Some(ds);
            continue;
        }
        if let Sexpr::List(children) = node
            && let Some(Sexpr::Atom(head)) = children.first()
            && head == "symbol"
            && children.len() > 2
        {
            collect_pins_and_properties(&children[2..], pins, footprint, datasheet);
        }
    }
}

fn merge_conversions(children: Vec<Sexpr>, child_prefix: &str) -> Vec<Sexpr> {
    let mut result = Vec::with_capacity(children.len());
    let mut unit_groups: std::collections::HashMap<i64, Vec<(i64, Vec<Sexpr>)>> =
        std::collections::HashMap::new();

    for child in children {
        if let Sexpr::List(parts) = &child
            && parts.len() >= 2
            && matches!(&parts[0], Sexpr::Atom(key) if key == "symbol")
            && let Sexpr::Atom(name) = &parts[1]
        {
            let name_unquoted = name.trim_matches('"');
            if let Some(rest) = name_unquoted.strip_prefix(child_prefix) {
                let mut split = rest.rsplitn(3, '_');
                let convert_str = split.next();
                let unit_str = split.next();
                if let (Some(c), Some(u)) = (convert_str, unit_str)
                    && let (Ok(unit), Ok(convert)) = (u.parse::<i64>(), c.parse::<i64>())
                    && unit > 0
                {
                    let inner = parts[2..].to_vec();
                    unit_groups.entry(unit).or_default().push((convert, inner));
                    continue;
                }
            }
        }
        result.push(child);
    }

    for (unit, mut conversions) in unit_groups {
        if conversions.len() <= 1 {
            let (convert, inner) = conversions.into_iter().next().unwrap();
            let name = format!("{}{}_{}", child_prefix, unit, convert);
            let mut parts = vec![Sexpr::atom("symbol"), Sexpr::str(&name)];
            parts.extend(inner);
            result.push(Sexpr::List(parts));
            continue;
        }
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

fn parse_property_value(node: &Sexpr, key: &str) -> Option<String> {
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
    let prop_key = string_value(children.get(1)?);
    if prop_key != key {
        return None;
    }
    let val = string_value(children.get(2)?);
    if val.is_empty() {
        return None;
    }
    Some(val)
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
    let mut extends = None;
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
    let mut datasheet = None;
    collect_pins_and_properties(&children[2..], &mut pins, &mut footprint, &mut datasheet);

    Some(SymbolDef {
        lib_id,
        pins,
        extends,
        footprint,
        datasheet,
        raw: node.clone(),
    })
}

fn resolve_extends(symbols: &mut [SymbolDef]) {
    let mut by_name: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (i, sym) in symbols.iter().enumerate() {
        by_name.insert(sym.lib_id.clone(), i);
    }
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
        let parent_lib_id = parent_name
            .split(':')
            .next_back()
            .unwrap_or(&parent_name)
            .to_string();
        let Some(&parent_idx) = by_name.get(&parent_lib_id) else {
            resolved[i] = true;
            return;
        };
        if parent_idx == i {
            resolved[i] = true;
            return;
        }
        if !visited.insert(i) {
            resolved[i] = true;
            return;
        }
        resolve_one(parent_idx, symbols, by_name, resolved, visited);
        let parent_pins = symbols[parent_idx].pins.clone();
        let child_pin_names: std::collections::HashSet<String> =
            symbols[i].pins.iter().map(|p| p.name.clone()).collect();
        for pin in &parent_pins {
            if !child_pin_names.contains(&pin.name) {
                symbols[i].pins.push(pin.clone());
            }
        }
        // Inherit footprint and datasheet from parent if child doesn't have one.
        if symbols[i].footprint.is_none() {
            symbols[i].footprint = symbols[parent_idx].footprint.clone();
        }
        if symbols[i].datasheet.is_none() {
            symbols[i].datasheet = symbols[parent_idx].datasheet.clone();
        }
        resolved[i] = true;
    }

    for i in 0..symbols.len() {
        let mut visited = std::collections::HashSet::new();
        resolve_one(i, symbols, &by_name, &mut resolved, &mut visited);
    }
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

    fn sample_lib() -> &'static str {
        r#"(kicad_symbol_lib
  (symbol "RP2354a"
    (property "Datasheet" "https://example.com/rp2354a.pdf"
      (at 0 0 0)
      (show_name no)
      (do_not_autoplace no)
      (hide yes)
      (effects (font (size 1.27 1.27)))
    )
    (pin power_in line (at -15.24 5.08 0) (length 2.54) (name "VDD") (number "1"))
    (pin power_in line (at -15.24 -5.08 0) (length 2.54) (name "GND") (number "2"))
  )
)"#
    }

    #[test]
    fn parse_extracts_symbol_names() {
        let symbols = parse_symbol_lib(sample_lib()).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].lib_id, "RP2354a");
        assert_eq!(symbols[0].pins.len(), 2);
    }

    #[test]
    fn parse_extracts_datasheet() {
        let symbols = parse_symbol_lib(sample_lib()).unwrap();
        assert_eq!(
            symbols[0].datasheet.as_deref(),
            Some("https://example.com/rp2354a.pdf")
        );
    }

    #[test]
    fn extends_inherits_datasheet() {
        let lib = r#"(kicad_symbol_lib
  (symbol "PARENT"
    (property "Datasheet" "https://example.com/parent.pdf"
      (at 0 0 0)
      (show_name no)
      (do_not_autoplace no)
      (hide yes)
      (effects (font (size 1.27 1.27)))
    )
    (pin power_in line (at 0 0 0) (length 2.54) (name "VDD") (number "1"))
  )
  (symbol "CHILD"
    (extends "PARENT")
  )
)"#;
        let symbols = parse_symbol_lib(lib).unwrap();
        assert_eq!(symbols.len(), 2);
        let child = symbols.iter().find(|s| s.lib_id == "CHILD").unwrap();
        assert_eq!(
            child.datasheet.as_deref(),
            Some("https://example.com/parent.pdf")
        );
    }
}
