//! Shared helpers for the KiCad backend emitters.

use std::collections::BTreeSet;

use copperleaf_ir::Design;

/// Extract the leading alphabetic prefix of a refdes (e.g. `U1` → `U`).
/// Returns `?` when the refdes has no alphabetic prefix.
pub fn refdes_prefix(refdes: &str) -> String {
    let alpha: String = refdes.chars().take_while(|c| c.is_alphabetic()).collect();
    if alpha.is_empty() {
        "?".to_string()
    } else {
        alpha
    }
}

/// Build a deterministic 1-based net code table for a design.
/// Codes are assigned in `design.nets` order, then any net name appearing only
/// in `design.connections` is appended in sorted order.
pub fn build_net_codes(design: &Design) -> Vec<(String, usize)> {
    let mut codes: Vec<(String, usize)> = design
        .nets
        .iter()
        .enumerate()
        .map(|(i, n)| (n.name.clone(), i + 1))
        .collect();

    let mut extra: BTreeSet<String> = BTreeSet::new();
    for conn in &design.connections {
        if !codes.iter().any(|(name, _)| name == &conn.net) {
            extra.insert(conn.net.clone());
        }
    }
    let start = codes.len();
    for (i, name) in extra.into_iter().enumerate() {
        codes.push((name, start + i + 1));
    }
    codes
}

/// Format a floating-point value with the given number of decimal places,
/// trimming trailing zeros and the trailing decimal point.
pub fn format_float(v: f64, decimals: usize) -> String {
    if decimals == 0 {
        return format!("{}", v.round() as i64);
    }
    let s = format!("{:.prec$}", v, prec = decimals);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Convert a length in metres to a millimetre string.
pub fn fmt_mm(meters: f64) -> String {
    format_float(meters * 1000.0, 6)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refdes_prefix_extracted() {
        assert_eq!(refdes_prefix("U1"), "U");
        assert_eq!(refdes_prefix("C2"), "C");
        assert_eq!(refdes_prefix("3V3"), "?");
    }

    #[test]
    fn format_float_trims_trailing_zeros() {
        assert_eq!(format_float(25.4, 2), "25.4");
        assert_eq!(format_float(19.049999999999997, 2), "19.05");
        assert_eq!(format_float(33.019999999999996, 2), "33.02");
        assert_eq!(format_float(100.0, 2), "100");
        assert_eq!(format_float(2.54, 2), "2.54");
        assert_eq!(format_float(0.0, 2), "0");
    }

    #[test]
    fn fmt_mm_converts_metres() {
        assert_eq!(fmt_mm(0.0003), "0.3");
        assert_eq!(fmt_mm(0.0002), "0.2");
        assert_eq!(fmt_mm(0.001), "1");
    }
}
