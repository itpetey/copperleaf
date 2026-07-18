//! Shared helpers for the KiCad backend emitters.

use copperleaf::{CompiledBoard, CompiledComponent, Role};

/// Nickname of the project-local library written into `symbols/`/`footprints/`.
pub const PROJECT_LIB: &str = "copperleaf";

/// Symbol name (without library prefix) for a component.
pub fn symbol_name(comp: &CompiledComponent) -> &str {
    comp.symbol
        .as_deref()
        .map(|s| s.split_once(':').map(|(_, n)| n).unwrap_or(s))
        .unwrap_or(&comp.refdes)
}

/// Symbol library nickname for a component: the prefix of `symbol()` when it
/// contains a `':'`, otherwise the project-local library.
pub fn symbol_lib_nick(comp: &CompiledComponent) -> String {
    comp.symbol
        .as_deref()
        .and_then(|s| s.split_once(':').map(|(l, _)| l.to_string()))
        .unwrap_or_else(|| PROJECT_LIB.to_string())
}

/// Full `lib:symbol` identifier used by schematic instances.
pub fn symbol_lib_id(comp: &CompiledComponent) -> String {
    match comp.symbol.as_deref() {
        Some(s) if s.contains(':') => s.to_string(),
        Some(s) => format!("{}:{s}", PROJECT_LIB),
        None => format!("{}:{}", PROJECT_LIB, comp.refdes),
    }
}

/// The component's footprint name when it refers to a project-local
/// footprint (`None` for external `lib:name` references).
pub fn local_footprint_name(comp: &CompiledComponent) -> Option<String> {
    match comp.footprint.as_deref() {
        Some(s) if s.contains(':') => None,
        Some(s) => Some(s.to_string()),
        None => Some(comp.refdes.clone()),
    }
}

/// Full `lib:footprint` reference used in symbol properties and the PCB.
pub fn footprint_ref(comp: &CompiledComponent) -> String {
    match comp.footprint.as_deref() {
        Some(s) if s.contains(':') => s.to_string(),
        Some(s) => format!("{}:{s}", PROJECT_LIB),
        None => format!("{}:{}", PROJECT_LIB, comp.refdes),
    }
}

/// Build deterministic 1-based net codes from a compiled board.
pub fn build_net_codes(board: &CompiledBoard) -> Vec<(String, usize)> {
    let mut codes: Vec<(String, usize)> = board
        .nets
        .iter()
        .enumerate()
        .map(|(i, n)| (n.name.clone(), i + 1))
        .collect();
    let mut seen: std::collections::BTreeSet<String> =
        board.nets.iter().map(|n| n.name.clone()).collect();
    let mut extra = Vec::new();
    for conn in &board.connections {
        if seen.insert(conn.net.0.clone()) {
            extra.push(conn.net.0.clone());
        }
    }
    let start = codes.len();
    for (i, name) in extra.into_iter().enumerate() {
        codes.push((name, start + i + 1));
    }
    codes
}

/// Convert metres to millimetres as a formatted string.
pub fn fmt_mm(meters: f64) -> String {
    format_float(meters * 1000.0, 6)
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
    if trimmed == "-0" { "0".to_string() } else { trimmed.to_string() }
}

/// Format a coordinate rounded to the 0.01 mm grid required by KLC for
/// courtyard (and fab) geometry.
pub fn format_grid_float(v: f64) -> String {
    format_float((v * 100.0).round() / 100.0, 2)
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
    }
}

/// Map a Copperleaf pin role to a KiCad netlist pintype string.
pub fn role_to_pintype(role: Role) -> &'static str {
    role_to_pin_type(role)
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
