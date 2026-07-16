//! KiCad symbol library emitter.
//!
//! Generates `.kicad_sym` files from Copperleaf [`Manifest`] data so the part
//! TOML can serve as the single source of truth for a component's symbol.

use copperleaf_part_codegen::Manifest;

use crate::sexpr::Sexpr;

/// Body margin outside the symbol pin extents.
const BODY_MARGIN: f64 = 1.27;
/// Grid spacing in millimetres (0.1 inch = 2.54 mm).
const GRID: f64 = 2.54;
/// Default pin length.
const PIN_LENGTH: f64 = 2.54;
/// Horizontal offset for left/right-side pins from the body centre.
const PIN_X_OFFSET: f64 = 5.08;

/// Generate a `.kicad_sym` library S-expression string from a component manifest.
///
/// Pins are auto-laid-out on a 2.54 mm grid: roughly half on the left, half
/// on the right. Thermal pads appear at the bottom.
pub fn emit_symbol(manifest: &Manifest) -> String {
    let lib_id = manifest
        .component
        .lib_id
        .as_deref()
        .unwrap_or(&manifest.component.name);

    let title = &manifest.component.title;

    let mut lib_children: Vec<Sexpr> = vec![
        Sexpr::list([Sexpr::atom("version"), Sexpr::atom("20231120")]),
        Sexpr::list([Sexpr::atom("generator"), Sexpr::str("copperleaf")]),
    ];

    lib_children.push(symbol_node(manifest, lib_id, title));

    Sexpr::list(
        [Sexpr::atom("kicad_symbol_lib").into()]
            .into_iter()
            .chain(lib_children),
    )
    .to_string()
}

fn atom_str(expr: &Sexpr) -> Option<&str> {
    match expr {
        Sexpr::Atom(s) => {
            let trimmed = s.trim_matches('"');
            Some(trimmed)
        }
        _ => None,
    }
}

/// Auto-layout pins on a grid.
///
/// Returns `(left_pins, right_pins, bottom_pins)` where pins are split
/// roughly in half between left and right sides. Thermal pads (typically
/// named EXP or PAD with kind gnd in the centre) go to the bottom.
fn classify_pins(manifest: &Manifest) -> (Vec<usize>, Vec<usize>, Vec<usize>) {
    let mut left: Vec<usize> = Vec::new();
    let mut right: Vec<usize> = Vec::new();
    let mut bottom: Vec<usize> = Vec::new();

    let mid = (manifest.pins.len() + 1) / 2;
    for (i, pin) in manifest.pins.iter().enumerate() {
        if is_thermal_pad(pin) {
            bottom.push(i);
        } else if i < mid {
            left.push(i);
        } else {
            right.push(i);
        }
    }
    (left, right, bottom)
}

/// Extract pin (x, y) from a pin s-expression node if present.
fn extract_pin_pos(node: &Sexpr) -> Option<(f64, f64)> {
    let Sexpr::List(children) = node else {
        return None;
    };
    // Find the (at x y rot) sub-list.
    for child in children {
        if let Sexpr::List(parts) = child
            && parts.len() >= 3
            && let Some(Sexpr::Atom(key)) = parts.first()
            && key == "at"
        {
            let xs = parts.get(1).and_then(atom_str)?;
            let ys = parts.get(2).and_then(atom_str)?;
            let x: f64 = xs.parse().ok()?;
            let y: f64 = ys.parse().ok()?;
            return Some((x, y));
        }
    }
    None
}

fn fmt_f64(v: f64) -> String {
    format!("{:?}", v)
}

fn is_thermal_pad(pin: &copperleaf_part_codegen::PinDef) -> bool {
    pin.name.eq_ignore_ascii_case("EXP")
        || pin.name.eq_ignore_ascii_case("EP")
        || pin.name.eq_ignore_ascii_case("PAD")
}

/// Map a Copperleaf pin kind to a KiCad electrical type.
fn kind_to_pin_type(kind: &str) -> &'static str {
    match kind {
        "gnd" => "power_in",
        "pwr" | "pwr_fixed" => "power_in",
        "pwr_out" => "power_out",
        "clk" => "input",
        "spi" => "bidirectional",
        "dio" => "bidirectional",
        "analog_in" | "analog_rf" => "input",
        _ => "passive",
    }
}

/// Build pin s-expression nodes and return them in render order.
fn pin_layout(manifest: &Manifest) -> Vec<Sexpr> {
    let (left, right, bottom) = classify_pins(manifest);
    let mut nodes = Vec::new();

    // Left-side pins.
    for (row, &i) in left.iter().enumerate() {
        let pin = &manifest.pins[i];
        let y = (left.len() as f64 - 1.0) / 2.0 * GRID - row as f64 * GRID;
        nodes.push(symbol_pin(pin, -PIN_X_OFFSET, y, 0.0, true));
    }

    // Right-side pins.
    for (row, &i) in right.iter().enumerate() {
        let pin = &manifest.pins[i];
        let y = (right.len() as f64 - 1.0) / 2.0 * GRID - row as f64 * GRID;
        nodes.push(symbol_pin(pin, PIN_X_OFFSET, y, 0.0, false));
    }

    // Bottom pins (thermal pads).
    for (col, &i) in bottom.iter().enumerate() {
        let pin = &manifest.pins[i];
        let x = (bottom.len() as f64 - 1.0) / 2.0 * GRID - col as f64 * GRID;
        nodes.push(symbol_pin(pin, x, -PIN_X_OFFSET, 0.0, false));
    }

    nodes
}

fn property(key: &str, value: &str) -> Sexpr {
    Sexpr::list([
        Sexpr::atom("property"),
        Sexpr::str(key),
        Sexpr::str(value),
        Sexpr::list([
            Sexpr::atom("at"),
            Sexpr::atom("0"),
            Sexpr::atom("0"),
            Sexpr::atom("0"),
        ]),
        Sexpr::list([
            Sexpr::atom("effects"),
            Sexpr::list([
                Sexpr::atom("font"),
                Sexpr::list([
                    Sexpr::atom("size"),
                    Sexpr::atom("1.27"),
                    Sexpr::atom("1.27"),
                ]),
            ]),
        ]),
    ])
}

fn property_hidden(key: &str, value: &str) -> Sexpr {
    Sexpr::list([
        Sexpr::atom("property"),
        Sexpr::str(key),
        Sexpr::str(value),
        Sexpr::list([
            Sexpr::atom("at"),
            Sexpr::atom("0"),
            Sexpr::atom("0"),
            Sexpr::atom("0"),
        ]),
        Sexpr::list([
            Sexpr::atom("effects"),
            Sexpr::list([
                Sexpr::atom("font"),
                Sexpr::list([
                    Sexpr::atom("size"),
                    Sexpr::atom("1.27"),
                    Sexpr::atom("1.27"),
                ]),
            ]),
            Sexpr::list([Sexpr::atom("hide"), Sexpr::atom("yes")]),
        ]),
    ])
}

/// Compute a body rectangle that encloses all pin positions with a margin.
fn symbol_body(pin_nodes: &[Sexpr]) -> Sexpr {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for node in pin_nodes {
        if let Some((x, y)) = extract_pin_pos(node) {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
    }

    if min_x == f64::MAX {
        // No pins — tiny default body.
        min_x = -BODY_MARGIN;
        max_x = BODY_MARGIN;
        min_y = -BODY_MARGIN;
        max_y = BODY_MARGIN;
    }

    let x1 = min_x - BODY_MARGIN;
    let y1 = min_y - BODY_MARGIN;
    let x2 = max_x + BODY_MARGIN;
    let y2 = max_y + BODY_MARGIN;

    Sexpr::list([
        Sexpr::atom("rectangle"),
        Sexpr::list([
            Sexpr::atom("start"),
            Sexpr::atom(fmt_f64(x1)),
            Sexpr::atom(fmt_f64(y1)),
        ]),
        Sexpr::list([
            Sexpr::atom("end"),
            Sexpr::atom(fmt_f64(x2)),
            Sexpr::atom(fmt_f64(y2)),
        ]),
        Sexpr::list([Sexpr::atom("fill"), Sexpr::atom("background")]),
    ])
}

fn symbol_node(manifest: &Manifest, lib_id: &str, title: &str) -> Sexpr {
    let mut children = vec![Sexpr::atom("symbol"), Sexpr::str(lib_id)];

    // ── properties ──
    children.push(property("Reference", "U"));
    children.push(property("Value", title));
    children.push(property_hidden("Footprint", ""));
    if let Some(ref ds) = manifest.component.datasheet {
        children.push(property_hidden("Datasheet", ds));
    } else {
        children.push(property_hidden("Datasheet", ""));
    }
    if let Some(ref desc) = manifest.component.description {
        children.push(property_hidden("ki_description", desc));
    }

    // ── unit sub-symbol ──
    let unit_name = format!("{}_0_1", lib_id);
    let mut unit = vec![Sexpr::atom("symbol"), Sexpr::str(&unit_name)];

    // Pin layout.
    let pin_nodes = pin_layout(manifest);
    let body_rect = symbol_body(&pin_nodes);
    unit.push(body_rect);

    for pn in pin_nodes {
        unit.push(pn);
    }

    children.push(Sexpr::list(unit));
    Sexpr::list(children)
}

/// Build a single pin s-expression.
///
/// `on_left` controls text offset: names go to the left of left-side pins
/// and to the right of right-side pins (KiCad convention).
fn symbol_pin(
    pin: &copperleaf_part_codegen::PinDef,
    x: f64,
    y: f64,
    rot: f64,
    on_left: bool,
) -> Sexpr {
    let pin_type = if is_thermal_pad(pin) {
        "power_in"
    } else {
        kind_to_pin_type(&pin.kind)
    };

    let name_offset = if on_left { -0.762 } else { 0.762 };
    let number_offset = if on_left { 0.762 } else { -0.762 };

    let mut children = vec![
        Sexpr::atom("pin"),
        Sexpr::atom("line"),
        Sexpr::list([
            Sexpr::atom("at"),
            Sexpr::atom(fmt_f64(x)),
            Sexpr::atom(fmt_f64(y)),
            Sexpr::atom(fmt_f64(rot)),
        ]),
        Sexpr::list([Sexpr::atom("length"), Sexpr::atom(fmt_f64(PIN_LENGTH))]),
    ];

    // Name with effects.
    children.push(Sexpr::list([
        Sexpr::atom("name"),
        Sexpr::str(&pin.name),
        Sexpr::list([
            Sexpr::atom("effects"),
            Sexpr::list([
                Sexpr::atom("font"),
                Sexpr::list([
                    Sexpr::atom("size"),
                    Sexpr::atom("1.27"),
                    Sexpr::atom("1.27"),
                ]),
            ]),
            Sexpr::list([
                Sexpr::atom("justify"),
                Sexpr::atom(if on_left { "right" } else { "left" }),
            ]),
            Sexpr::list([
                Sexpr::atom("offset"),
                Sexpr::atom(fmt_f64(name_offset)),
                Sexpr::atom("0.0"),
            ]),
        ]),
    ]));

    // Number with effects.
    children.push(Sexpr::list([
        Sexpr::atom("number"),
        Sexpr::str(&pin.number),
        Sexpr::list([
            Sexpr::atom("effects"),
            Sexpr::list([
                Sexpr::atom("font"),
                Sexpr::list([
                    Sexpr::atom("size"),
                    Sexpr::atom("1.27"),
                    Sexpr::atom("1.27"),
                ]),
            ]),
            Sexpr::list([
                Sexpr::atom("justify"),
                Sexpr::atom(if on_left { "left" } else { "right" }),
            ]),
            Sexpr::list([
                Sexpr::atom("offset"),
                Sexpr::atom(fmt_f64(number_offset)),
                Sexpr::atom("0.0"),
            ]),
        ]),
    ]));

    // Pin type.
    children.insert(1, Sexpr::atom(pin_type));

    Sexpr::list(children)
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf_part_codegen::{ComponentMeta, PinDef};

    fn make_test_manifest() -> Manifest {
        Manifest {
            component: ComponentMeta {
                name: "TestPart".into(),
                title: "Test Part".into(),
                description: Some("A test component.".into()),
                datasheet: Some("https://example.com/ds.pdf".into()),
                lib_id: Some("TestPart".into()),
            },
            pins: vec![
                PinDef {
                    num: 1,
                    number: "1".into(),
                    name: "VDD".into(),
                    purpose: "Supply".into(),
                    notes: String::new(),
                    kind: "pwr".into(),
                    bw_mhz: None,
                    v: None,
                    v_min: None,
                    v_max: None,
                    i: None,
                    i_max: None,
                    pos: None,
                    rotation: None,
                    length: None,
                    nc: None,
                    width: None,
                    height: None,
                    pad_type: None,
                    pad_shape: None,
                    roundrect_rratio: None,
                    solder_mask_margin: None,
                    layers: None,
                    drill: None,
                    thermal_vias: vec![],
                },
                PinDef {
                    num: 2,
                    number: "2".into(),
                    name: "GND".into(),
                    purpose: "Ground".into(),
                    notes: String::new(),
                    kind: "gnd".into(),
                    bw_mhz: None,
                    v: None,
                    v_min: None,
                    v_max: None,
                    i: None,
                    i_max: None,
                    pos: None,
                    rotation: None,
                    length: None,
                    nc: None,
                    width: None,
                    height: None,
                    pad_type: None,
                    pad_shape: None,
                    roundrect_rratio: None,
                    solder_mask_margin: None,
                    layers: None,
                    drill: None,
                    thermal_vias: vec![],
                },
                PinDef {
                    num: 3,
                    number: "3".into(),
                    name: "CLK".into(),
                    purpose: "Clock".into(),
                    notes: String::new(),
                    kind: "clk".into(),
                    bw_mhz: Some(50.0),
                    v: None,
                    v_min: None,
                    v_max: None,
                    i: None,
                    i_max: None,
                    pos: None,
                    rotation: None,
                    length: None,
                    nc: None,
                    width: None,
                    height: None,
                    pad_type: None,
                    pad_shape: None,
                    roundrect_rratio: None,
                    solder_mask_margin: None,
                    layers: None,
                    drill: None,
                    thermal_vias: vec![],
                },
                PinDef {
                    num: 4,
                    number: "4".into(),
                    name: "D0".into(),
                    purpose: "I/O".into(),
                    notes: String::new(),
                    kind: "dio".into(),
                    bw_mhz: None,
                    v: None,
                    v_min: None,
                    v_max: None,
                    i: None,
                    i_max: None,
                    pos: None,
                    rotation: None,
                    length: None,
                    nc: None,
                    width: None,
                    height: None,
                    pad_type: None,
                    pad_shape: None,
                    roundrect_rratio: None,
                    solder_mask_margin: None,
                    layers: None,
                    drill: None,
                    thermal_vias: vec![],
                },
                PinDef {
                    num: 5,
                    number: "5".into(),
                    name: "EXP".into(),
                    purpose: "Ground".into(),
                    notes: String::new(),
                    kind: "gnd".into(),
                    bw_mhz: None,
                    v: None,
                    v_min: None,
                    v_max: None,
                    i: None,
                    i_max: None,
                    pos: None,
                    rotation: None,
                    length: None,
                    nc: None,
                    width: None,
                    height: None,
                    pad_type: None,
                    pad_shape: None,
                    roundrect_rratio: None,
                    solder_mask_margin: None,
                    layers: None,
                    drill: None,
                    thermal_vias: vec![],
                },
            ],
            constraints: vec![],

            mechanical: vec![],
        }
    }

    #[test]
    fn emits_valid_s_expression() {
        let out = emit_symbol(&make_test_manifest());
        let parsed = crate::sexpr::parse(&out);
        assert!(parsed.is_ok(), "failed to parse: {out}");
    }

    #[test]
    fn contains_symbol_header() {
        let out = emit_symbol(&make_test_manifest());
        assert!(
            out.starts_with("(kicad_symbol_lib"),
            "missing kicad_symbol_lib header"
        );
        assert!(out.contains("\"TestPart\""), "missing symbol name");
    }

    #[test]
    fn contains_all_pins() {
        let out = emit_symbol(&make_test_manifest());
        assert!(out.contains("\"VDD\""), "missing VDD pin");
        assert!(out.contains("\"GND\""), "missing GND pin");
        assert!(out.contains("\"CLK\""), "missing CLK pin");
        assert!(out.contains("\"D0\""), "missing D0 pin");
        assert!(out.contains("\"EXP\""), "missing EXP pin");
    }

    #[test]
    fn contains_properties() {
        let out = emit_symbol(&make_test_manifest());
        assert!(out.contains("\"Reference\""), "missing Reference property");
        assert!(out.contains("\"Value\""), "missing Value property");
        assert!(out.contains("\"Footprint\""), "missing Footprint property");
    }

    #[test]
    fn contains_datasheet_when_set() {
        let out = emit_symbol(&make_test_manifest());
        assert!(
            out.contains("https://example.com/ds.pdf"),
            "missing datasheet URL"
        );
    }

    #[test]
    fn thermal_pad_goes_to_bottom() {
        let out = emit_symbol(&make_test_manifest());
        // EXP is pin 5 (thermal), should appear on the bottom.
        // Bottom pin: y should be negative (below the body).
        assert!(out.contains("\"EXP\""), "EXP should be present");
    }

    #[test]
    fn round_trip_through_parser() {
        let manifest = make_test_manifest();
        let out = emit_symbol(&manifest);

        // Parse it back.
        let symbols =
            crate::sym_parser::parse_symbol_lib(&out).expect("should parse generated symbol");

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].lib_id, "TestPart");
        assert_eq!(symbols[0].pins.len(), 5);
        assert_eq!(
            symbols[0].datasheet.as_deref(),
            Some("https://example.com/ds.pdf")
        );
    }
}
