//! Shared schematic-symbol pin auto-layout for the KiCad emitters.
//!
//! Produces a conventional boxed symbol: positive power pins across the top,
//! ground/thermal pins across the bottom, and the remaining signal pins split
//! between the left and right sides.  All pin connection points land exactly
//! on the 2.54 mm (100 mil) grid as required by KLC S4.2.

use copperleaf::Role;

use crate::common::role_to_pin_type;

/// Grid spacing in millimetres (0.1 inch).
pub const GRID: f64 = 2.54;
/// Default pin length in millimetres.
pub const PIN_LENGTH: f64 = 2.54;

/// Electrical classification inputs for one pin to be laid out.
#[derive(Clone, Debug)]
pub struct LayoutPin {
    pub name: String,
    pub number: String,
    pub role: Role,
}

/// A fully placed symbol pin.
#[derive(Clone, Debug)]
pub struct PlacedPin {
    pub name: String,
    pub number: String,
    /// KiCad electrical type string (e.g. `"power_in"`).
    pub etype: &'static str,
    pub x: f64,
    pub y: f64,
    /// Pin rotation in degrees: 0 = extends right (left side), 180 = extends
    /// left (right side), 270 = extends down (top), 90 = extends up (bottom).
    pub rotation: f64,
    pub length: f64,
}

/// The result of auto-laying-out a symbol's pins.
#[derive(Clone, Debug)]
pub struct SymbolLayout {
    /// Body rectangle: `(x1, y1)` top-left, `(x2, y2)` bottom-right, in symbol
    /// coordinates (Y axis points up).
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub pins: Vec<PlacedPin>,
}

/// Build the body `(rectangle ...)` S-expression for a layout.
pub fn body_rect_sexpr(layout: &SymbolLayout) -> crate::sexpr::Sexpr {
    use crate::common::format_float;
    use crate::sexpr::Sexpr;

    Sexpr::list([
        Sexpr::atom("rectangle"),
        Sexpr::list([
            Sexpr::atom("start"),
            Sexpr::atom(format_float(layout.x1, 2)),
            Sexpr::atom(format_float(layout.y1, 2)),
        ]),
        Sexpr::list([
            Sexpr::atom("end"),
            Sexpr::atom(format_float(layout.x2, 2)),
            Sexpr::atom(format_float(layout.y2, 2)),
        ]),
        Sexpr::list([
            Sexpr::atom("stroke"),
            Sexpr::list([Sexpr::atom("width"), Sexpr::atom("0.254")]),
            Sexpr::list([Sexpr::atom("type"), Sexpr::atom("default")]),
        ]),
        Sexpr::list([
            Sexpr::atom("fill"),
            Sexpr::list([Sexpr::atom("type"), Sexpr::atom("background")]),
        ]),
    ])
}

/// Lay out `pins` as a boxed symbol.
///
/// Classification: exposed-pad and ground pins go to the bottom edge, power
/// input/output pins to the top edge, and all remaining pins are split in
/// order between the left and right edges.  Symbols containing power-output
/// pins (power converters/regulators) instead place power inputs on the left
/// and power outputs on the right per KLC S4.2.  Layout is computed in
/// integer grid units so every pin connection point is exactly on the
/// 100 mil grid.
pub fn layout_symbol(pins: &[LayoutPin]) -> SymbolLayout {
    let has_pwr_out = pins.iter().any(|p| p.role == Role::PowerOut);

    let mut top: Vec<usize> = Vec::new();
    let mut bottom: Vec<usize> = Vec::new();
    let mut signals: Vec<usize> = Vec::new();
    let mut pwr_in: Vec<usize> = Vec::new();
    let mut pwr_out: Vec<usize> = Vec::new();

    for (i, p) in pins.iter().enumerate() {
        if is_thermal_name(&p.name) {
            bottom.push(i);
        } else {
            match p.role {
                Role::PowerIn if has_pwr_out => pwr_in.push(i),
                Role::PowerOut if has_pwr_out => pwr_out.push(i),
                Role::PowerIn | Role::PowerOut => top.push(i),
                Role::Gnd => bottom.push(i),
                _ => signals.push(i),
            }
        }
    }

    let mid = signals.len().div_ceil(2);
    let (sig_left, sig_right) = signals.split_at(mid);

    // Power converters: inputs left, outputs right, signals appended to the
    // shorter sides.  Everything else: signals split left/right.
    let (left, right): (Vec<usize>, Vec<usize>) = if has_pwr_out {
        let mut l = pwr_in;
        let mut r = pwr_out;
        l.extend_from_slice(sig_left);
        r.extend_from_slice(sig_right);
        (l, r)
    } else {
        (sig_left.to_vec(), sig_right.to_vec())
    };

    // ── body extents in integer grid units ──
    let n_side = left.len().max(right.len());
    let n_tb = top.len().max(bottom.len());

    let span_side = n_side.saturating_sub(1) as i64; // grid units between first/last row
    let span_tb = n_tb.saturating_sub(1) as i64;

    // First row/column offset from centre (kept on grid).
    let y0_u = (span_side as f64 / 2.0).floor() as i64;
    let x0_u = -(span_tb as f64 / 2.0).floor() as i64;

    let half_h_u = (span_side as f64 / 2.0 + 1.0).ceil().max(1.0) as i64;
    let half_w_u = (span_tb as f64 / 2.0 + 1.0).ceil().max(2.0) as i64;

    let mut placed: Vec<PlacedPin> = Vec::with_capacity(pins.len());

    for (row, &i) in left.iter().enumerate() {
        let p = &pins[i];
        placed.push(PlacedPin {
            name: p.name.clone(),
            number: p.number.clone(),
            etype: role_to_pin_type(p.role),
            x: -((half_w_u + 1) as f64) * GRID,
            y: (y0_u - row as i64) as f64 * GRID,
            rotation: 0.0,
            length: PIN_LENGTH,
        });
    }
    for (row, &i) in right.iter().enumerate() {
        let p = &pins[i];
        placed.push(PlacedPin {
            name: p.name.clone(),
            number: p.number.clone(),
            etype: role_to_pin_type(p.role),
            x: ((half_w_u + 1) as f64) * GRID,
            y: (y0_u - row as i64) as f64 * GRID,
            rotation: 180.0,
            length: PIN_LENGTH,
        });
    }
    for (col, &i) in top.iter().enumerate() {
        let p = &pins[i];
        placed.push(PlacedPin {
            name: p.name.clone(),
            number: p.number.clone(),
            etype: role_to_pin_type(p.role),
            x: (x0_u + col as i64) as f64 * GRID,
            y: ((half_h_u + 1) as f64) * GRID,
            rotation: 270.0,
            length: PIN_LENGTH,
        });
    }
    for (col, &i) in bottom.iter().enumerate() {
        let p = &pins[i];
        placed.push(PlacedPin {
            name: p.name.clone(),
            number: p.number.clone(),
            etype: role_to_pin_type(p.role),
            x: (x0_u + col as i64) as f64 * GRID,
            y: -((half_h_u + 1) as f64) * GRID,
            rotation: 90.0,
            length: PIN_LENGTH,
        });
    }

    SymbolLayout {
        x1: -(half_w_u as f64) * GRID,
        y1: (half_h_u as f64) * GRID,
        x2: (half_w_u as f64) * GRID,
        y2: -(half_h_u as f64) * GRID,
        pins: placed,
    }
}

/// Build the `(pin ...)` S-expression for a placed pin.
pub fn placed_pin_sexpr(pin: &PlacedPin) -> crate::sexpr::Sexpr {
    use crate::common::format_float;
    use crate::sexpr::Sexpr;

    let font = || {
        Sexpr::list([
            Sexpr::atom("font"),
            Sexpr::list([
                Sexpr::atom("size"),
                Sexpr::atom("1.27"),
                Sexpr::atom("1.27"),
            ]),
        ])
    };

    Sexpr::list([
        Sexpr::atom("pin"),
        Sexpr::atom(pin.etype),
        Sexpr::atom("line"),
        Sexpr::list([
            Sexpr::atom("at"),
            Sexpr::atom(format_float(pin.x, 2)),
            Sexpr::atom(format_float(pin.y, 2)),
            Sexpr::atom(format_float(pin.rotation, 0)),
        ]),
        Sexpr::list([
            Sexpr::atom("length"),
            Sexpr::atom(format_float(pin.length, 2)),
        ]),
        Sexpr::list([
            Sexpr::atom("name"),
            Sexpr::str(&pin.name),
            Sexpr::list([Sexpr::atom("effects"), font()]),
        ]),
        Sexpr::list([
            Sexpr::atom("number"),
            Sexpr::str(&pin.number),
            Sexpr::list([Sexpr::atom("effects"), font()]),
        ]),
    ])
}

/// True for pin names that conventionally indicate an exposed thermal pad
/// or mechanical feature (placed at the bottom of the symbol).
fn is_thermal_name(name: &str) -> bool {
    let upper = name.to_uppercase();
    upper == "EXP"
        || upper == "EP"
        || upper == "PAD"
        || upper.starts_with("EXP")
        || upper.starts_with("EP")
        || upper.starts_with("PAD")
        || upper.starts_with("MECH")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin(name: &str, role: Role) -> LayoutPin {
        LayoutPin {
            name: name.to_string(),
            number: name.to_string(),
            role,
        }
    }

    #[test]
    fn power_top_ground_bottom_signals_split() {
        let pins = [
            pin("VDD", Role::PowerIn),
            pin("GND", Role::Gnd),
            pin("D0", Role::DigitalIO),
            pin("D1", Role::DigitalIO),
            pin("D2", Role::DigitalIO),
            pin("D3", Role::DigitalIO),
        ];
        let layout = layout_symbol(&pins);
        let get = |name: &str| layout.pins.iter().find(|p| p.name == name).unwrap();

        // VDD on top (rotation 270), GND on bottom (rotation 90).
        assert_eq!(get("VDD").rotation, 270.0);
        assert_eq!(get("GND").rotation, 90.0);
        // Signals split left/right.
        assert_eq!(get("D0").rotation, 0.0);
        assert_eq!(get("D1").rotation, 0.0);
        assert_eq!(get("D2").rotation, 180.0);
        assert_eq!(get("D3").rotation, 180.0);
    }

    #[test]
    fn all_pins_on_grid() {
        let mut v: Vec<LayoutPin> = (0..61)
            .map(|i| pin(&format!("P{i}"), Role::DigitalIO))
            .collect();
        v.push(LayoutPin {
            name: "VDD".to_string(),
            number: "62".to_string(),
            role: Role::PowerIn,
        });
        let layout = layout_symbol(&v);
        for p in &layout.pins {
            let gx = p.x / GRID;
            let gy = p.y / GRID;
            assert!(
                (gx - gx.round()).abs() < 1e-9,
                "pin {} x={} off grid",
                p.name,
                p.x
            );
            assert!(
                (gy - gy.round()).abs() < 1e-9,
                "pin {} y={} off grid",
                p.name,
                p.y
            );
        }
    }

    #[test]
    fn no_overlapping_pins() {
        let v: Vec<LayoutPin> = (0..30)
            .map(|i| pin(&format!("P{i}"), Role::DigitalIO))
            .collect();
        let layout = layout_symbol(&v);
        let mut positions: Vec<(i64, i64)> = layout
            .pins
            .iter()
            .map(|p| ((p.x * 100.0).round() as i64, (p.y * 100.0).round() as i64))
            .collect();
        positions.sort();
        positions.dedup();
        assert_eq!(positions.len(), layout.pins.len(), "pins overlap");
    }

    #[test]
    fn body_encloses_pin_rows() {
        let pins = [pin("A", Role::DigitalIO), pin("B", Role::DigitalIO)];
        let layout = layout_symbol(&pins);
        for p in &layout.pins {
            assert!(p.x.abs() > layout.x2.abs(), "pin anchor outside body edge");
        }
        assert!(layout.y1 > 0.0 && layout.y2 < 0.0);
    }

    #[test]
    fn thermal_name_goes_bottom() {
        let pins = [pin("EXP", Role::DigitalIO), pin("IO", Role::DigitalIO)];
        let layout = layout_symbol(&pins);
        let exp = layout.pins.iter().find(|p| p.name == "EXP").unwrap();
        assert_eq!(exp.rotation, 90.0);
    }
}
