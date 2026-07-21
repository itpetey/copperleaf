//! Shared helpers for the KiCad backend emitters.

use crate::sexpr::Sexpr;
use copperleaf::{CompiledBoard, CompiledComponent, Role};

/// Nickname of the project-local library written into `symbols/`/`footprints/`.
pub const PROJECT_LIB: &str = "copperleaf";

/// Build deterministic 1-based net codes from a compiled board.
pub fn build_net_codes(board: &CompiledBoard) -> Vec<(String, usize)> {
    board
        .nets
        .iter()
        .enumerate()
        .map(|(i, n)| (n.name.clone(), i + 1))
        .collect()
}

/// Convert metres to millimetres as a formatted string.
pub fn fmt_mm(meters: f64) -> String {
    format_float(meters * 1000.0, 6)
}

/// Full `lib:footprint` reference used in symbol properties and the PCB.
pub fn footprint_ref(comp: &CompiledComponent) -> String {
    match comp.meta.footprint.as_deref() {
        Some(s) => s.to_owned(),
        None => format!("{}:{}", PROJECT_LIB, comp.refdes),
    }
}

/// Format a floating-point value, trimming trailing zeros and normalising
/// negative zero (`-0.0` → `"0"`).
pub fn format_float(v: f64, decimals: usize) -> String {
    let v = if v == 0.0 { 0.0 } else { v };
    if decimals == 0 {
        return format!("{}", v.round() as i64);
    }
    let s = format!("{:.prec$}", v, prec = decimals);
    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
    if trimmed == "-0" {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Format a coordinate rounded to the 0.01 mm grid required by KLC for
/// courtyard (and fab) geometry.
pub fn format_grid_float(v: f64) -> String {
    format_float((v * 100.0).round() / 100.0, 2)
}

/// The component's footprint name when it refers to a project-local
/// footprint (`None` for external `lib:name` references).
pub fn local_footprint_name(comp: &CompiledComponent) -> Option<String> {
    match comp.meta.footprint.as_deref() {
        Some(s) if s.contains(':') => None,
        Some(s) => Some(s.to_string()),
        None => Some(comp.refdes.clone()),
    }
}

/// Extract refdes prefix (e.g. `U1` -> `U`).
pub fn refdes_prefix(refdes: &str) -> String {
    let alpha: String = refdes.chars().take_while(|c| c.is_alphabetic()).collect();
    if alpha.is_empty() {
        "?".to_string()
    } else {
        alpha
    }
}

/// Map a Copperleaf pin role to a KiCad pin type string.
pub fn role_to_pin_type(role: Role) -> &'static str {
    match role {
        Role::PowerIn | Role::Gnd => "power_in",
        Role::PowerOut => "power_out",
        Role::AnalogIn => "input",
        Role::AnalogOut => "output",
        Role::DigitalIO | Role::DiffPos | Role::DiffNeg => "bidirectional",
        Role::Passive => "passive",
    }
}

/// Build a `(property …)` node for schematic and symbol-library contexts.
///
/// Emitted properties always use 1.27 mm font size.  Pass `justify_left: true`
/// when the property appears at a non-zero position so KiCad left-aligns the
/// text; pass `false` for properties at the origin (hidden metadata).
pub fn property_sym_node(
    key: &str,
    value: &str,
    pos: (f64, f64),
    hide: bool,
    justify_left: bool,
) -> Sexpr {
    let mut effects = vec![Sexpr::list([
        Sexpr::atom("font"),
        Sexpr::list([
            Sexpr::atom("size"),
            Sexpr::atom("1.27"),
            Sexpr::atom("1.27"),
        ]),
    ])];
    if justify_left {
        effects.push(Sexpr::list([Sexpr::atom("justify"), Sexpr::atom("left")]));
    }

    let mut children = vec![
        Sexpr::atom("property"),
        Sexpr::str(key),
        Sexpr::str(value),
        Sexpr::list([
            Sexpr::atom("at"),
            Sexpr::atom(format_float(pos.0, 2)),
            Sexpr::atom(format_float(pos.1, 2)),
            Sexpr::atom("0"),
        ]),
    ];
    if hide {
        children.push(Sexpr::list([Sexpr::atom("hide"), Sexpr::atom("yes")]));
    }
    children.push(Sexpr::list(
        std::iter::once(Sexpr::atom("effects")).chain(effects),
    ));
    Sexpr::list(children)
}

/// Full `lib:symbol` identifier used by schematic instances.
pub fn symbol_lib_id(comp: &CompiledComponent) -> String {
    match comp.meta.symbol.as_deref() {
        Some(s) if s.contains(':') => s.to_string(),
        Some(s) => format!("{}:{s}", PROJECT_LIB),
        None => format!("{}:{}", PROJECT_LIB, comp.refdes),
    }
}

/// Symbol library nickname for a component: the prefix of `symbol()` when it
/// contains a `':'`, otherwise the project-local library.
pub fn symbol_lib_nick(comp: &CompiledComponent) -> String {
    comp.meta
        .symbol
        .as_deref()
        .and_then(|s| s.split_once(':').map(|(l, _)| l.to_string()))
        .unwrap_or_else(|| PROJECT_LIB.to_string())
}

/// Symbol name (without library prefix) for a component.
pub fn symbol_name(comp: &CompiledComponent) -> &str {
    comp.meta
        .symbol
        .as_deref()
        .map(|s| s.split_once(':').map(|(_, n)| n).unwrap_or(s))
        .unwrap_or(&comp.refdes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_float_trims_zeros() {
        assert_eq!(format_float(25.4, 2), "25.4");
        assert_eq!(format_float(100.0, 2), "100");
    }
}
