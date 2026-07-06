use copperleaf_ir::Design;

use crate::common::{format_float, refdes_prefix};
use crate::sexpr::{Sexpr, deterministic_uuid, kv};

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
        lib_symbols_node(),
    ];

    for (idx, comp) in design.components.iter().enumerate() {
        children.push(symbol_instance_node(idx, comp));
    }

    for conn in &design.connections {
        children.push(label_node(design, conn));
    }

    children.push(sheet_instances_node());

    let sch = Sexpr::list(std::iter::once(Sexpr::atom("kicad_sch")).chain(children));
    format!("{}\n", sch)
}

fn title_block_node() -> Sexpr {
    Sexpr::list([
        Sexpr::atom("title_block"),
        kv("title", ""),
        kv("company", ""),
        kv("rev", ""),
        kv("date", ""),
        kv("source", "copperleaf"),
    ])
}

fn lib_symbols_node() -> Sexpr {
    let symbol = Sexpr::list([
        Sexpr::atom("symbol"),
        Sexpr::str("Generic:Box"),
        Sexpr::list([Sexpr::atom("in_bom"), Sexpr::atom("yes")]),
        Sexpr::list([Sexpr::atom("on_board"), Sexpr::atom("yes")]),
        Sexpr::list([
            Sexpr::atom("property"),
            Sexpr::str("Reference"),
            Sexpr::str("U"),
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
        ]),
        Sexpr::list([
            Sexpr::atom("symbol"),
            Sexpr::str("Box_0_1"),
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
                Sexpr::list([Sexpr::atom("fill"), Sexpr::atom("none")]),
            ]),
        ]),
    ]);
    Sexpr::list([Sexpr::atom("lib_symbols"), symbol])
}

fn symbol_instance_node(idx: usize, comp: &copperleaf_ir::ComponentRecord) -> Sexpr {
    let (x, y) = symbol_position(idx);
    let prefix = refdes_prefix(&comp.refdes);
    Sexpr::list([
        Sexpr::atom("symbol"),
        Sexpr::list([Sexpr::atom("lib_id"), Sexpr::str("Generic:Box")]),
        Sexpr::list([
            Sexpr::atom("at"),
            Sexpr::atom(format_float(x, 2)),
            Sexpr::atom(format_float(y, 2)),
            Sexpr::atom("0"),
        ]),
        Sexpr::list([Sexpr::atom("unit"), Sexpr::atom("1")]),
        Sexpr::list([Sexpr::atom("in_bom"), Sexpr::atom("1")]),
        Sexpr::list([Sexpr::atom("on_board"), Sexpr::atom("1")]),
        Sexpr::list([
            Sexpr::atom("uuid"),
            Sexpr::str(deterministic_uuid(&format!("sch:{}", comp.refdes))),
        ]),
        property_node("Reference", &comp.refdes, x, y - 6.35),
        property_node("Value", &prefix, x, y + 6.35),
    ])
}

fn property_node(key: &str, value: &str, x: f64, y: f64) -> Sexpr {
    Sexpr::list([
        Sexpr::atom("property"),
        Sexpr::str(key),
        Sexpr::str(value),
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
    ])
}

fn label_node(design: &Design, conn: &copperleaf_ir::Connection) -> Sexpr {
    let idx = component_index_by_refdes(design, conn.refdes.as_str());
    let (x, y) = symbol_position(idx);
    let label_x = x + 7.62;
    let label_y = y;
    Sexpr::list([
        Sexpr::atom("label"),
        Sexpr::str(&conn.net),
        Sexpr::list([
            Sexpr::atom("at"),
            Sexpr::atom(format_float(label_x, 2)),
            Sexpr::atom(format_float(label_y, 2)),
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
            Sexpr::str(deterministic_uuid(&format!(
                "sch:label:{}:{}",
                conn.refdes, conn.pin
            ))),
        ]),
    ])
}

fn component_index_by_refdes(design: &Design, refdes: &str) -> usize {
    design
        .components
        .iter()
        .position(|c| c.refdes == refdes)
        .unwrap_or(0)
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

fn symbol_position(idx: usize) -> (f64, f64) {
    const GRID: f64 = 25.4;
    let x = GRID + (idx as f64 % 10.0) * GRID;
    let y = GRID + (idx as f64 / 10.0).floor() * GRID;
    (x, y)
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
    fn labels_placed_near_owning_symbol() {
        let d = make_design();
        let out = emit_schematic(&d);
        // U2 is at (50.8, 25.4); its label should be at (58.42, 25.4).
        assert!(out.contains("(at 58.42 25.4 0)"));
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
}
