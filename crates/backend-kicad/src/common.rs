//! Shared helpers for the KiCad backend emitters.

use copperleaf::{CompiledBoard, Role};

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

/// Format a floating-point value, trimming trailing zeros.
pub fn format_float(v: f64, decimals: usize) -> String {
    if decimals == 0 {
        return format!("{}", v.round() as i64);
    }
    let s = format!("{:.prec$}", v, prec = decimals);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
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
