use std::collections::HashMap;

use copperleaf_ir::{Design, Role};

use crate::{
    common::{format_float, refdes_prefix},
    sexpr::{Sexpr, deterministic_uuid, kv, parse},
};

/// Emit a minimal structurally-valid KiCad 6 schematic for the given design.
pub fn emit_schematic(design: &Design) -> String {
    let mut children: Vec<Sexpr> = vec![
        Sexpr::list([Sexpr::atom("version"), Sexpr::atom("20211123")]),
        kv("generator", "copperleaf"),
        Sexpr::list([
            Sexpr::atom("uuid"),
            Sexpr::str(deterministic_uuid("sch:root")),
        ]),
        kv("paper", "A4"),
        title_block_node(),
        lib_symbols_node(design),
    ];

    for (idx, comp) in design.components.iter().enumerate() {
        children.push(symbol_instance_node(idx, comp));
    }

    // Group connections by net and emit wires in a daisy-chain per net
    // with a single label at the midpoint, rather than one label per pin.
    // This avoids hundreds of floating labels that don't move with components.
    let mut net_conns: HashMap<&str, Vec<&copperleaf_ir::Connection>> = HashMap::new();
    for conn in &design.connections {
        net_conns.entry(conn.net.as_str()).or_default().push(conn);
    }

    for (net_name, conns) in &net_conns {
        // Collect pin tip positions for all pins on this net.
        let mut tips: Vec<(f64, f64)> = Vec::with_capacity(conns.len());
        for conn in conns {
            if let Some((tip, _)) = pin_tip_and_label(design, conn) {
                tips.push(tip);
            }
        }
        if tips.is_empty() {
            continue;
        }

        // Daisy-chain: wire tip0 → tip1, tip1 → tip2, …
        for pair in tips.windows(2) {
            children.push(wire_between(pair[0], pair[1], net_name));
        }

        // Single label for the net, placed at the average of all pin tips.
        let avg_x = tips.iter().map(|(x, _)| x).sum::<f64>() / tips.len() as f64;
        let avg_y = tips.iter().map(|(_, y)| y).sum::<f64>() / tips.len() as f64;
        children.push(label_at(net_name, avg_x, avg_y));
    }

    children.push(sheet_instances_node());

    let sch = Sexpr::list(std::iter::once(Sexpr::atom("kicad_sch")).chain(children));
    format!("{}\n", sch)
}

fn component_index_by_refdes(design: &Design, refdes: &str) -> usize {
    design
        .components
        .iter()
        .position(|c| c.refdes == refdes)
        .unwrap_or(0)
}

/// Emit a `<label>` S‑expression at the given coordinates.
fn label_at(name: &str, x: f64, y: f64) -> Sexpr {
    Sexpr::list([
        Sexpr::atom("label"),
        Sexpr::str(name),
        Sexpr::list([
            Sexpr::atom("at"),
            Sexpr::atom(format_float(x, 2)),
            Sexpr::atom(format_float(y, 2)),
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
        Sexpr::list([
            Sexpr::atom("uuid"),
            Sexpr::str(deterministic_uuid(&format!("sch:label:{}", name))),
        ]),
    ])
}

/// A pin definition inside a lib_symbol — placed on the right edge of the
/// symbol body, pointing left.
fn lib_pin_node(pin: &copperleaf_ir::Pin, index: usize, total_pins: usize) -> Sexpr {
    let pin_type = role_to_pin_type(pin.role);

    let (x, y, rotation) = match pin.pos {
        Some((px, py)) => (px, py, pin.rotation.unwrap_or(180.0)),
        None => (7.62, pin_y_offset(index, total_pins), 180.0),
    };

    Sexpr::list([
        Sexpr::atom("pin"),
        Sexpr::atom(pin_type),
        Sexpr::atom("line"),
        Sexpr::list([
            Sexpr::atom("at"),
            Sexpr::atom(format_float(x, 2)),
            Sexpr::atom(format_float(y, 2)),
            Sexpr::atom(format_float(rotation, 0)),
        ]),
        Sexpr::list([
            Sexpr::atom("length"),
            Sexpr::atom(format_float(pin.length.unwrap_or(2.54), 2)),
        ]),
        Sexpr::list([
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
            ]),
        ]),
        Sexpr::list([
            Sexpr::atom("number"),
            Sexpr::str((index + 1).to_string()),
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
        ]),
    ])
}

/// A property inside a lib_symbol definition.  All lib_symbol properties sit at
/// the origin; hidden ones (Footprint/Datasheet) carry a `(hide yes)` effect.
fn lib_property_node(key: &str, value: &str, hide: bool) -> Sexpr {
    let mut effects_children = vec![Sexpr::list([
        Sexpr::atom("font"),
        Sexpr::list([
            Sexpr::atom("size"),
            Sexpr::atom("1.27"),
            Sexpr::atom("1.27"),
        ]),
    ])];
    if hide {
        effects_children.push(Sexpr::list([Sexpr::atom("hide"), Sexpr::atom("yes")]));
    }
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
        Sexpr::list(std::iter::once(Sexpr::atom("effects")).chain(effects_children)),
    ])
}

/// Build a single `<symbol>` S-expression for one component.
///
/// When `kicad_symbol_raw` is set (populated by `resolve_symbols` from a
/// `.kicad_sym` library), the real symbol definition is embedded verbatim —
/// with the library nickname prepended to the symbol name — so KiCad renders
/// the actual graphics, properties, and pins. Otherwise a placeholder box is
/// generated.
fn lib_symbol_for_component(comp: &copperleaf_ir::ComponentRecord) -> Sexpr {
    // If we have a raw symbol definition from the .kicad_sym library, embed it
    // verbatim (with the library prefix added to the symbol name) instead of
    // generating a placeholder.
    if let Some(raw) = &comp.kicad_symbol_raw
        && let Ok(Sexpr::List(mut children)) = parse(raw)
    {
        // The second element is the symbol name; replace it with the
        // library-prefixed name (e.g. "RP2350A" -> "MCU_RaspberryPi:RP2350A").
        if children.len() >= 2 {
            let symbol_name = comp.kicad_symbol.as_deref().unwrap_or("");
            children[1] = Sexpr::str(symbol_name);
        }
        return Sexpr::List(children);
    }

    // --- Placeholder fallback (no library symbol resolved) ---
    let fallback = format!("copperleaf:{}", comp.refdes);
    let symbol_name = comp.kicad_symbol.as_deref().unwrap_or(&fallback);
    // Extract the symbol base name (after library prefix) for the unit sub-symbol.
    // KiCad requires the unit name to start with the symbol name, not the refdes.
    let symbol_base = symbol_name.split(':').next_back().unwrap_or(&comp.refdes);
    let fp_default = comp.kicad_footprint.as_deref().unwrap_or("");
    let mut body = vec![
        Sexpr::atom("symbol"),
        Sexpr::str(symbol_name),
        Sexpr::list([
            Sexpr::atom("pin_names"),
            Sexpr::list([Sexpr::atom("offset"), Sexpr::atom("0")]),
        ]),
        Sexpr::list([Sexpr::atom("exclude_from_sim"), Sexpr::atom("no")]),
        Sexpr::list([Sexpr::atom("in_bom"), Sexpr::atom("yes")]),
        Sexpr::list([Sexpr::atom("on_board"), Sexpr::atom("yes")]),
        lib_property_node("Reference", "U", false),
        lib_property_node("Value", "Box", false),
        lib_property_node("Footprint", fp_default, true),
        lib_property_node("Datasheet", "", true),
        Sexpr::list([
            Sexpr::atom("symbol"),
            Sexpr::str(format!("{}_0_1", symbol_base)),
            Sexpr::list([
                Sexpr::atom("rectangle"),
                Sexpr::list([
                    Sexpr::atom("start"),
                    Sexpr::atom("-5.08"),
                    Sexpr::atom("-5.08"),
                ]),
                Sexpr::list([Sexpr::atom("end"), Sexpr::atom("5.08"), Sexpr::atom("5.08")]),
                Sexpr::list([
                    Sexpr::atom("stroke"),
                    Sexpr::list([Sexpr::atom("width"), Sexpr::atom("0.1524")]),
                    Sexpr::list([Sexpr::atom("type"), Sexpr::atom("default")]),
                ]),
                Sexpr::list([
                    Sexpr::atom("fill"),
                    Sexpr::list([Sexpr::atom("type"), Sexpr::atom("none")]),
                ]),
            ]),
        ]),
    ];

    // Add a `<pin>` entry for every pin on the component.
    for (i, pin) in comp.pins.iter().enumerate() {
        body.push(lib_pin_node(pin, i, comp.pins.len()));
    }

    Sexpr::list(body)
}

/// Generate a `<symbol>` definition for each component in the design, embedded
/// inside `<lib_symbols>`.  Every symbol gets a rectangular body and a `(pin
/// ...)` entry for each of the component's pins, placed on the right edge of
/// the body.
fn lib_symbols_node(design: &Design) -> Sexpr {
    let symbols: Vec<Sexpr> = design
        .components
        .iter()
        .map(lib_symbol_for_component)
        .collect();

    Sexpr::list(std::iter::once(Sexpr::atom("lib_symbols")).chain(symbols))
}

fn pin_index_by_name(pins: &[copperleaf_ir::Pin], pin_name: &str) -> Option<usize> {
    pins.iter().position(|p| p.name == pin_name)
}

/// Compute the schematic coordinates for a pin tip and the label placed just
/// past it. Returns `None` when the component or pin cannot be found.
fn pin_tip_and_label(
    design: &Design,
    conn: &copperleaf_ir::Connection,
) -> Option<((f64, f64), (f64, f64))> {
    let comp_idx = component_index_by_refdes(design, &conn.refdes);
    let comp = design.components.get(comp_idx)?;
    let pin_idx = pin_index_by_name(&comp.pins, &conn.pin)?;
    let pin = &comp.pins[pin_idx];
    let (sym_x, sym_y) = symbol_position(comp_idx);

    let (tip_x, tip_y) = match pin.pos {
        Some((px, py)) => {
            let rot = pin.rotation.unwrap_or(180.0);
            let rad = rot.to_radians();
            let length = pin.length.unwrap_or(2.54);
            (
                sym_x + px + length * rad.cos(),
                sym_y + py + length * rad.sin(),
            )
        }
        None => {
            let y_off = pin_y_offset(pin_idx, comp.pins.len());
            (sym_x + 7.62, sym_y + y_off)
        }
    };

    Some(((tip_x, tip_y), (tip_x + 2.54, tip_y)))
}

/// Y-offset for pin *index* out of *total_pins*, centred vertically.
/// Pins are spaced 2.54 mm apart.
fn pin_y_offset(index: usize, total_pins: usize) -> f64 {
    if total_pins <= 1 {
        0.0
    } else {
        let spacing = 2.54;
        let total_height = (total_pins as f64 - 1.0) * spacing;
        -total_height / 2.0 + index as f64 * spacing
    }
}

fn property_node(key: &str, value: &str, x: f64, y: f64, hide: bool) -> Sexpr {
    let mut children = vec![
        Sexpr::atom("property"),
        Sexpr::str(key),
        Sexpr::str(value),
        Sexpr::list([
            Sexpr::atom("at"),
            Sexpr::atom(format_float(x, 2)),
            Sexpr::atom(format_float(y, 2)),
            Sexpr::atom("0"),
        ]),
    ];
    if hide {
        children.push(Sexpr::list([Sexpr::atom("hide"), Sexpr::atom("yes")]));
    }
    children.push(Sexpr::list([
        Sexpr::atom("effects"),
        Sexpr::list([
            Sexpr::atom("font"),
            Sexpr::list([
                Sexpr::atom("size"),
                Sexpr::atom("1.27"),
                Sexpr::atom("1.27"),
            ]),
        ]),
    ]));
    Sexpr::list(children)
}

fn role_to_pin_type(role: Role) -> &'static str {
    match role {
        Role::PowerIn | Role::Gnd => "power_in",
        Role::PowerOut => "power_out",
        Role::AnalogIn => "input",
        Role::AnalogOut => "output",
        Role::DigitalIO | Role::DiffPos | Role::DiffNeg => "bidirectional",
    }
}

fn sheet_instances_node() -> Sexpr {
    Sexpr::list([
        Sexpr::atom("sheet_instances"),
        Sexpr::list([
            Sexpr::atom("path"),
            Sexpr::str("/"),
            Sexpr::list([Sexpr::atom("page"), Sexpr::str("1")]),
        ]),
    ])
}

fn symbol_instance_node(idx: usize, comp: &copperleaf_ir::ComponentRecord) -> Sexpr {
    let (x, y) = symbol_position(idx);
    let fallback = format!("copperleaf:{}", comp.refdes);
    let lib_id = comp.kicad_symbol.as_deref().unwrap_or(&fallback);

    let mut properties = vec![
        property_node("Reference", &comp.refdes, x, y - 6.35, false),
        property_node("Value", &refdes_prefix(&comp.refdes), x, y + 6.35, false),
    ];
    if let Some(fp) = &comp.kicad_footprint {
        // Hidden footprint property at the symbol position, matching KiCad's
        // default layout.
        properties.push(property_node("Footprint", fp, x, y, true));
    }

    Sexpr::list(
        std::iter::once(Sexpr::atom("symbol"))
            .chain(std::iter::once(Sexpr::list([
                Sexpr::atom("lib_id"),
                Sexpr::str(lib_id),
            ])))
            .chain(std::iter::once(Sexpr::list([
                Sexpr::atom("at"),
                Sexpr::atom(format_float(x, 2)),
                Sexpr::atom(format_float(y, 2)),
                Sexpr::atom("0"),
            ])))
            .chain(std::iter::once(Sexpr::list([
                Sexpr::atom("unit"),
                Sexpr::atom("1"),
            ])))
            .chain(std::iter::once(Sexpr::list([
                Sexpr::atom("in_bom"),
                Sexpr::atom("yes"),
            ])))
            .chain(std::iter::once(Sexpr::list([
                Sexpr::atom("on_board"),
                Sexpr::atom("yes"),
            ])))
            .chain(std::iter::once(Sexpr::list([
                Sexpr::atom("dnp"),
                Sexpr::atom("no"),
            ])))
            .chain(std::iter::once(Sexpr::list([
                Sexpr::atom("uuid"),
                Sexpr::str(deterministic_uuid(&format!("sch:{}", comp.refdes))),
            ])))
            .chain(properties)
            .chain(std::iter::once(Sexpr::list([
                Sexpr::atom("instances"),
                Sexpr::list([
                    Sexpr::atom("project"),
                    Sexpr::str(""),
                    Sexpr::list([
                        Sexpr::atom("path"),
                        Sexpr::str(format!("/{}", deterministic_uuid("sch:root"))),
                        Sexpr::list([Sexpr::atom("reference"), Sexpr::str(&comp.refdes)]),
                        Sexpr::list([Sexpr::atom("unit"), Sexpr::atom("1")]),
                    ]),
                ]),
            ]))),
    )
}

fn symbol_position(idx: usize) -> (f64, f64) {
    const GRID: f64 = 25.4;
    let x = GRID + (idx as f64 % 10.0) * GRID;
    let y = GRID + (idx as f64 / 10.0).floor() * GRID;
    (x, y)
}

fn title_block_node() -> Sexpr {
    Sexpr::list([
        Sexpr::atom("title_block"),
        kv("title", ""),
        kv("company", ""),
        kv("rev", ""),
        kv("date", ""),
    ])
}

/// Emit a `<wire>` S‑expression between two arbitrary points.
fn wire_between(from: (f64, f64), to: (f64, f64), net_name: &str) -> Sexpr {
    Sexpr::list([
        Sexpr::atom("wire"),
        Sexpr::list([
            Sexpr::atom("pts"),
            Sexpr::list([
                Sexpr::atom("xy"),
                Sexpr::atom(format_float(from.0, 2)),
                Sexpr::atom(format_float(from.1, 2)),
            ]),
            Sexpr::list([
                Sexpr::atom("xy"),
                Sexpr::atom(format_float(to.0, 2)),
                Sexpr::atom(format_float(to.1, 2)),
            ]),
        ]),
        Sexpr::list([
            Sexpr::atom("stroke"),
            Sexpr::list([Sexpr::atom("width"), Sexpr::atom("0")]),
        ]),
        Sexpr::list([
            Sexpr::atom("uuid"),
            Sexpr::str(deterministic_uuid(&format!(
                "sch:wire:{}:{:.2}:{:.2}",
                net_name, from.0, from.1
            ))),
        ]),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf_core::UnitExt;
    use copperleaf_ir::{ComponentInst, Design, Limits, Net, Pin, Role};

    fn make_design() -> Design {
        let v3v3 = Net::power("V3V3", 3.3_f64.volt());
        let u1 = ComponentInst::new(
            "U1",
            TestBlock {
                pins: vec![Pin::new(
                    "VDD",
                    Role::PowerIn,
                    Limits::new(1.7_f64.volt(), 3.6_f64.volt(), 0.5_f64.amp()),
                    None,
                )],
            },
        );
        let u2 = ComponentInst::new(
            "U2",
            TestBlock {
                pins: vec![Pin::new(
                    "VDD",
                    Role::PowerIn,
                    Limits::new(1.7_f64.volt(), 3.6_f64.volt(), 0.5_f64.amp()),
                    None,
                )],
            },
        );

        let mut d = Design::default();
        d.add_net(v3v3);
        d.add_component(u1);
        d.add_component(u2);
        d.connect("U1", "VDD", "V3V3");
        d.connect("U2", "VDD", "V3V3");
        d
    }

    #[derive(Clone, Debug)]
    struct TestBlock {
        pins: Vec<Pin>,
    }

    impl copperleaf_ir::Block for TestBlock {
        fn pins(&self) -> &[Pin] {
            &self.pins
        }
    }

    #[test]
    fn schematic_starts_with_kicad_sch() {
        let d = make_design();
        let out = emit_schematic(&d);
        assert!(out.starts_with("(kicad_sch"));
    }

    #[test]
    fn schematic_has_reference_property() {
        let d = make_design();
        let out = emit_schematic(&d);
        assert!(out.contains("(property \"Reference\" \"U1\""));
        assert!(out.contains("(property \"Reference\" \"U2\""));
    }

    #[test]
    fn schematic_has_net_label() {
        let d = make_design();
        let out = emit_schematic(&d);
        assert!(out.contains("(label \"V3V3\""));
    }

    #[test]
    fn components_have_individual_lib_symbols() {
        let d = make_design();
        let out = emit_schematic(&d);
        assert!(out.contains("(symbol \"copperleaf:U1\""));
        assert!(out.contains("(symbol \"copperleaf:U2\""));
        assert!(out.contains("(lib_id \"copperleaf:U1\")"));
        assert!(out.contains("(lib_id \"copperleaf:U2\")"));
    }

    #[test]
    fn lib_symbols_contain_pin_definitions() {
        let d = make_design();
        let out = emit_schematic(&d);
        // Each component has a VDD pin with power_in type (multi-line formatted).
        assert!(out.contains("(pin power_in line"));
        assert!(out.contains("(at 7.62 0 180)"));
        assert!(out.contains("(length 2.54)"));
        assert!(out.contains("(name \"VDD\""));
        assert!(out.contains("(number \"1\""));
    }

    #[test]
    fn wires_connect_pins_to_labels() {
        let d = make_design();
        let out = emit_schematic(&d);
        // U1 at (25.4, 25.4), pin tip at (33.02, 25.4), label at (35.56, 25.4)
        // Coordinates appear on separate lines in the formatted output.
        assert!(out.contains("(xy 33.02 25.4)"));
        assert!(out.contains("(xy 35.56 25.4)"));
        // U2 at (50.8, 25.4), pin tip at (58.42, 25.4), label at (60.96, 25.4)
        assert!(out.contains("(xy 58.42 25.4)"));
        assert!(out.contains("(xy 60.96 25.4)"));
    }

    #[test]
    fn labels_placed_at_end_of_wire() {
        let d = make_design();
        let out = emit_schematic(&d);
        // U2 label should be at (60.96, 25.4) — 2.54 mm right of its pin tip.
        assert!(out.contains("(at 60.96 25.4 0)"));
        // Ensure no float imprecision artifacts.
        assert!(!out.contains("99999999999"));
    }

    #[test]
    fn empty_design_schematic_is_valid() {
        let d = Design::default();
        let out = emit_schematic(&d);
        assert!(out.starts_with("(kicad_sch"));
        assert!(out.contains("(sheet_instances"));
        assert!(!out.contains("(symbol (lib_id"));
    }

    #[test]
    fn schematic_uuid_stable_and_distinct() {
        let d = make_design();
        let out1 = emit_schematic(&d);
        let out2 = emit_schematic(&d);
        assert_eq!(out1, out2);

        let u1_uuid = deterministic_uuid("sch:U1");
        let u2_uuid = deterministic_uuid("sch:U2");
        assert_ne!(u1_uuid, u2_uuid);
        assert!(out1.contains(&u1_uuid));
        assert!(out1.contains(&u2_uuid));
    }

    #[test]
    fn multi_pin_component_has_pins_spaced_vertically() {
        let u1 = ComponentInst::new(
            "U1",
            TestBlock {
                pins: vec![
                    Pin::new(
                        "VIN",
                        Role::PowerIn,
                        Limits::new(0.0_f64.volt(), 6.0_f64.volt(), 1.0_f64.amp()),
                        None,
                    ),
                    Pin::new(
                        "GND",
                        Role::Gnd,
                        Limits::new(0.0_f64.volt(), 0.3_f64.volt(), 1.0_f64.amp()),
                        None,
                    ),
                    Pin::new(
                        "VOUT",
                        Role::PowerOut,
                        Limits::new(0.0_f64.volt(), 6.0_f64.volt(), 0.5_f64.amp()),
                        None,
                    ),
                ],
            },
        );

        let mut d = Design::default();
        d.add_net(Net::power("VIN", 5.0_f64.volt()));
        d.add_net(Net::ground());
        d.add_net(Net::power("VOUT", 3.3_f64.volt()));
        d.add_component(u1);
        d.connect("U1", "VIN", "VIN");
        d.connect("U1", "GND", "GND");
        d.connect("U1", "VOUT", "VOUT");

        let out = emit_schematic(&d);

        // Three pins should be at y = -2.54 (VIN), 0 (GND), +2.54 (VOUT)
        assert!(out.contains("(at 7.62 -2.54 180)"));
        assert!(out.contains("(at 7.62 0 180)"));
        assert!(out.contains("(at 7.62 2.54 180)"));

        // Wires should go from each pin tip to a label 2.54 mm to the right.
        // Coordinates appear on separate lines in the formatted output.
        assert!(out.contains("(xy 33.02 22.86)"));
        assert!(out.contains("(xy 35.56 22.86)"));
        assert!(out.contains("(xy 33.02 25.4)"));
        assert!(out.contains("(xy 35.56 25.4)"));
        assert!(out.contains("(xy 33.02 27.94)"));
        assert!(out.contains("(xy 35.56 27.94)"));

        // Labels at the end of each wire (multi-line formatted).
        assert!(out.contains("\"VIN\""));
        assert!(out.contains("\"GND\""));
        assert!(out.contains("\"VOUT\""));
        assert!(out.contains("(at 35.56 22.86 0)"));
        assert!(out.contains("(at 35.56 25.4 0)"));
        assert!(out.contains("(at 35.56 27.94 0)"));
    }

    #[derive(Clone, Debug)]
    struct SymbolBlock {
        pins: Vec<Pin>,
        symbol: Option<&'static str>,
    }

    impl copperleaf_ir::Block for SymbolBlock {
        fn pins(&self) -> &[Pin] {
            &self.pins
        }
        fn kicad_symbol(&self) -> Option<&str> {
            self.symbol
        }
    }

    #[test]
    fn component_with_kicad_symbol_uses_real_lib_id() {
        let mut pin = Pin::new(
            "VDD",
            Role::PowerIn,
            Limits::new(1.7_f64.volt(), 3.6_f64.volt(), 0.5_f64.amp()),
            None,
        );
        pin.pos = Some((-15.24, 5.08));
        pin.rotation = Some(0.0);

        let u1 = ComponentInst::new(
            "U1",
            SymbolBlock {
                pins: vec![pin],
                symbol: Some("RP2040:RP2354a"),
            },
        );

        let mut d = Design::default();
        d.add_net(Net::power("V3V3", 3.3_f64.volt()));
        d.add_component(u1);
        d.connect("U1", "VDD", "V3V3");

        let out = emit_schematic(&d);
        assert!(out.contains("(lib_id \"RP2040:RP2354a\")"));
        assert!(out.contains("(symbol \"RP2040:RP2354a\""));
        assert!(out.contains("(symbol \"RP2354a_0_1\""));
        assert!(out.contains("(at -15.24 5.08 0)"));
    }

    #[test]
    fn resolved_symbol_produces_wire_at_real_pin_position() {
        use std::io::Write;

        let lib = r#"(kicad_symbol_lib
  (symbol "RP2354a"
    (pin power_in line (at -15.24 5.08 0) (length 2.54) (name "VDD") (number "1"))
  )
)"#;
        let mut path = std::env::temp_dir();
        path.push("copperleaf_sch_test.kicad_sym");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(lib.as_bytes()).unwrap();

        let u1 = ComponentInst::new(
            "U1",
            SymbolBlock {
                pins: vec![Pin::new(
                    "VDD",
                    Role::PowerIn,
                    Limits::new(1.7_f64.volt(), 3.6_f64.volt(), 0.5_f64.amp()),
                    None,
                )],
                symbol: Some("RP2040:RP2354a"),
            },
        );

        let mut d = Design::default();
        d.add_net(Net::power("V3V3", 3.3_f64.volt()));
        d.add_component(u1);
        d.connect("U1", "VDD", "V3V3");

        crate::resolve_symbols(&mut d, Some(path.to_str().unwrap()));
        let out = emit_schematic(&d);

        // U1 is the first component, so symbol_position(0) = (25.4, 25.4).
        // Pin tip for VDD at local (-15.24, 5.08) with rotation 0 and length 2.54
        // is at absolute (25.4 + (-15.24) + 2.54, 25.4 + 5.08) = (12.7, 30.48).
        assert!(out.contains("(at -15.24 5.08 0)"));
        assert!(out.contains("(xy 12.7 30.48)"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn resolved_symbol_uses_library_pin_length() {
        use std::io::Write;

        let lib = r#"(kicad_symbol_lib
  (symbol "RP2354a"
    (pin power_in line (at -15.24 5.08 0) (length 3.81) (name "VDD") (number "1"))
  )
)"#;
        let mut path = std::env::temp_dir();
        path.push("copperleaf_sch_length_test.kicad_sym");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(lib.as_bytes()).unwrap();

        let u1 = ComponentInst::new(
            "U1",
            SymbolBlock {
                pins: vec![Pin::new(
                    "VDD",
                    Role::PowerIn,
                    Limits::new(1.7_f64.volt(), 3.6_f64.volt(), 0.5_f64.amp()),
                    None,
                )],
                symbol: Some("RP2040:RP2354a"),
            },
        );

        let mut d = Design::default();
        d.add_net(Net::power("V3V3", 3.3_f64.volt()));
        d.add_component(u1);
        d.connect("U1", "VDD", "V3V3");

        crate::resolve_symbols(&mut d, Some(path.to_str().unwrap()));
        let out = emit_schematic(&d);

        // lib_symbol pin should use the library length.
        assert!(out.contains("(length 3.81)"));
        // Pin tip uses 3.81 instead of the default 2.54:
        // absolute x = 25.4 + (-15.24) + 3.81 = 13.97, y = 25.4 + 5.08 = 30.48.
        assert!(out.contains("(xy 13.97 30.48)"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn resolved_symbol_embeds_real_definition_not_placeholder() {
        use std::io::Write;

        let lib = r#"(kicad_symbol_lib
  (symbol "RP2354a"
    (property "Reference" "U" (at 0 0 0) (effects (font (size 1.27 1.27))))
    (property "Value" "RP2354a" (at 0 0 0) (effects (font (size 1.27 1.27))))
    (property "Footprint" "Package_QFP:LQFP-64_10x10mm_P0.5mm" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))
    (property "Datasheet" "https://example.com/datasheet.pdf" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))
    (property "Description" "A test microcontroller" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))
    (symbol "RP2354a_0_1"
      (rectangle (start -5.08 5.08) (end 5.08 -5.08)
        (stroke (width 0.254) (type default))
        (fill (type background)))
    )
    (symbol "RP2354a_1_1"
      (pin power_in line (at -10.16 2.54 0) (length 3.81) (name "VDD") (number "42"))
      (pin power_in line (at -10.16 -2.54 0) (length 3.81) (name "GND") (number "1"))
    )
  )
)"#;
        let mut path = std::env::temp_dir();
        path.push("copperleaf_sch_raw_embed_test.kicad_sym");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(lib.as_bytes()).unwrap();

        let u1 = ComponentInst::new(
            "U1",
            SymbolBlock {
                pins: vec![
                    Pin::new(
                        "VDD",
                        Role::PowerIn,
                        Limits::new(1.7_f64.volt(), 3.6_f64.volt(), 0.5_f64.amp()),
                        None,
                    ),
                    Pin::new(
                        "GND",
                        Role::Gnd,
                        Limits::new(0.0_f64.volt(), 0.0_f64.volt(), 0.1_f64.amp()),
                        None,
                    ),
                ],
                symbol: Some("RP2040:RP2354a"),
            },
        );

        let mut d = Design::default();
        d.add_net(Net::power("V3V3", 3.3_f64.volt()));
        d.add_component(u1);
        d.connect("U1", "VDD", "V3V3");
        d.connect("U1", "GND", "GND");

        crate::resolve_symbols(&mut d, Some(path.to_str().unwrap()));
        let out = emit_schematic(&d);

        // The lib_symbol should use the library-prefixed name.
        assert!(out.contains("(symbol \"RP2040:RP2354a\""));
        // Real property values from the library, not the placeholder "Box".
        assert!(out.contains("(property \"Value\" \"RP2354a\""));
        assert!(out.contains("(property \"Datasheet\" \"https://example.com/datasheet.pdf\""));
        assert!(out.contains("(property \"Description\" \"A test microcontroller\""));
        // Real pin numbers from the library (42 and 1), not sequential (1 and 2).
        assert!(out.contains("(number \"42\")"));
        assert!(out.contains("(number \"1\")"));
        // Real graphics from the library.
        assert!(out.contains("(rectangle"));
        assert!(out.contains("(start -5.08 5.08)"));
        // The placeholder "Box" value must NOT appear for this symbol.
        // (It may appear for other components without a library symbol.)
        let sch_section = out.split("(symbol").nth(1).unwrap_or("");
        assert!(!sch_section.contains("(property \"Value\" \"Box\""));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn footprint_property_in_symbol_instance_is_hidden() {
        use std::io::Write;

        let lib = r#"(kicad_symbol_lib
  (symbol "RP2354a"
    (property "Footprint" "Package_QFP:LQFP-64_10x10mm_P0.5mm" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))
    (pin power_in line (at 0 0 0) (length 2.54) (name "VDD") (number "1"))
  )
)"#;
        let mut path = std::env::temp_dir();
        path.push("copperleaf_sch_hide_test.kicad_sym");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(lib.as_bytes()).unwrap();

        let u1 = ComponentInst::new(
            "U1",
            SymbolBlock {
                pins: vec![Pin::new(
                    "VDD",
                    Role::PowerIn,
                    Limits::new(1.7_f64.volt(), 3.6_f64.volt(), 0.5_f64.amp()),
                    None,
                )],
                symbol: Some("RP2040:RP2354a"),
            },
        );

        let mut d = Design::default();
        d.add_net(Net::power("V3V3", 3.3_f64.volt()));
        d.add_component(u1);
        d.connect("U1", "VDD", "V3V3");

        crate::resolve_symbols(&mut d, Some(path.to_str().unwrap()));
        let out = emit_schematic(&d);

        // The symbol instance's Footprint property should carry (hide yes).
        // Find the symbol instance section (after lib_symbols) and check it.
        let instance_section = out.split("(instances").next().unwrap_or("");
        let last_symbol = instance_section.rsplit("(symbol").next().unwrap_or("");
        assert!(last_symbol.contains("(property \"Footprint\""));
        assert!(last_symbol.contains("(hide yes)"));

        std::fs::remove_file(&path).ok();
    }
}
