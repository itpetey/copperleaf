use std::collections::{BTreeMap, HashMap};

use copperleaf_ir::{Design, NetClass};

use crate::common::{build_net_codes, fmt_mm, format_float};
use crate::sexpr::{Sexpr, kv};

/// Emit a KiCad S-expression PCB file for the given design.
pub fn emit_pcb(design: &Design) -> String {
    let net_codes = build_net_codes(design);
    let net_to_code: HashMap<&str, usize> = net_codes
        .iter()
        .map(|(name, code)| (name.as_str(), *code))
        .collect();

    let pin_to_net: HashMap<(&str, &str), &str> = design
        .connections
        .iter()
        .map(|c| ((c.refdes.as_str(), c.pin.as_str()), c.net.as_str()))
        .collect();

    let mut children: Vec<Sexpr> = vec![
        Sexpr::list([Sexpr::atom("version"), Sexpr::atom("20211014")]),
        kv("generator", "copperleaf"),
        general_node(),
        kv("paper", "A4"),
        layers_node(),
        setup_node(),
    ];

    for (name, code) in &net_codes {
        children.push(Sexpr::list([
            Sexpr::atom("net"),
            Sexpr::atom(code.to_string()),
            Sexpr::str(name),
        ]));
    }

    children.extend(net_class_nodes(design, &net_codes));
    children.extend(board_outline());

    for (idx, comp) in design.components.iter().enumerate() {
        children.push(footprint_node(idx, comp, &pin_to_net, &net_to_code));
    }

    let pcb = Sexpr::list(std::iter::once(Sexpr::atom("kicad_pcb")).chain(children));
    format!("{}\n", pcb)
}

fn general_node() -> Sexpr {
    Sexpr::list([
        Sexpr::atom("general"),
        Sexpr::list([Sexpr::atom("thickness"), Sexpr::atom("1.6")]),
        Sexpr::list([Sexpr::atom("legacy_teardrops"), Sexpr::atom("no")]),
    ])
}

fn layers_node() -> Sexpr {
    Sexpr::list([
        Sexpr::atom("layers"),
        Sexpr::list([Sexpr::atom("0"), Sexpr::str("F.Cu"), Sexpr::atom("signal")]),
        Sexpr::list([Sexpr::atom("31"), Sexpr::str("B.Cu"), Sexpr::atom("signal")]),
    ])
}

fn setup_node() -> Sexpr {
    Sexpr::list([
        Sexpr::atom("setup"),
        Sexpr::list([Sexpr::atom("pad_to_mask_clearance"), Sexpr::atom("0")]),
        Sexpr::list([
            Sexpr::atom("pcbplotparams"),
            Sexpr::list([
                Sexpr::atom("layerselection"),
                Sexpr::atom("0x00010fc_ffffffff"),
            ]),
        ]),
    ])
}

fn net_class_nodes(design: &Design, net_codes: &[(String, usize)]) -> Vec<Sexpr> {
    let mut groups: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    let mut default_nets: Vec<String> = Vec::new();

    for (name, _) in net_codes {
        let net = design.nets.iter().find(|n| &n.name == name);
        let class = net.map(|n| &n.class);
        match class {
            Some(NetClass {
                min_width: Some(w),
                clearance: Some(c),
            }) => {
                let key = (fmt_mm(w.as_base()), fmt_mm(c.as_base()));
                groups.entry(key).or_default().push(name.clone());
            }
            _ => default_nets.push(name.clone()),
        }
    }

    let mut nodes = vec![net_class_node("Default", "", "0.2", "0.25", &default_nets)];

    for ((width, clearance), nets) in groups {
        let name = format!("Power_{}mm_{}mm", width, clearance);
        nodes.push(net_class_node(&name, "", &clearance, &width, &nets));
    }

    nodes
}

fn net_class_node(
    name: &str,
    desc: &str,
    clearance: &str,
    trace_width: &str,
    nets: &[String],
) -> Sexpr {
    let mut children = vec![
        Sexpr::atom("net_class"),
        Sexpr::str(name),
        Sexpr::str(desc),
        Sexpr::list([Sexpr::atom("clearance"), Sexpr::atom(clearance)]),
        Sexpr::list([Sexpr::atom("trace_width"), Sexpr::atom(trace_width)]),
        Sexpr::list([Sexpr::atom("via_dia"), Sexpr::atom("0.8")]),
        Sexpr::list([Sexpr::atom("via_drill"), Sexpr::atom("0.4")]),
        Sexpr::list([Sexpr::atom("uvia_dia"), Sexpr::atom("0.3")]),
        Sexpr::list([Sexpr::atom("uvia_drill"), Sexpr::atom("0.1")]),
    ];
    for net in nets {
        children.push(Sexpr::list([Sexpr::atom("add_net"), Sexpr::str(net)]));
    }
    Sexpr::list(children)
}

fn board_outline() -> Vec<Sexpr> {
    let rect = [
        ((0.0, 0.0), (100.0, 0.0)),
        ((100.0, 0.0), (100.0, 80.0)),
        ((100.0, 80.0), (0.0, 80.0)),
        ((0.0, 80.0), (0.0, 0.0)),
    ];
    rect.iter()
        .map(|((x1, y1), (x2, y2))| {
            Sexpr::list([
                Sexpr::atom("gr_line"),
                Sexpr::list([
                    Sexpr::atom("start"),
                    Sexpr::atom(format_float(*x1, 2)),
                    Sexpr::atom(format_float(*y1, 2)),
                ]),
                Sexpr::list([
                    Sexpr::atom("end"),
                    Sexpr::atom(format_float(*x2, 2)),
                    Sexpr::atom(format_float(*y2, 2)),
                ]),
                Sexpr::list([Sexpr::atom("layer"), Sexpr::str("Edge.Cuts")]),
                Sexpr::list([Sexpr::atom("width"), Sexpr::atom("0.05")]),
            ])
        })
        .collect()
}

fn footprint_node(
    idx: usize,
    comp: &copperleaf_ir::ComponentRecord,
    pin_to_net: &HashMap<(&str, &str), &str>,
    net_to_code: &HashMap<&str, usize>,
) -> Sexpr {
    const PITCH: f64 = 10.0;
    let x = 10.0 + (idx as f64 % 10.0) * PITCH;
    let y = 10.0 + (idx as f64 / 10.0).floor() * PITCH;

    let mut children = vec![
        Sexpr::atom("footprint"),
        Sexpr::str("copperleaf:Generic"),
        Sexpr::list([Sexpr::atom("layer"), Sexpr::str("F.Cu")]),
        Sexpr::list([
            Sexpr::atom("at"),
            Sexpr::atom(format_float(x, 2)),
            Sexpr::atom(format_float(y, 2)),
        ]),
        Sexpr::list([
            Sexpr::atom("fp_text"),
            Sexpr::atom("reference"),
            Sexpr::str(&comp.refdes),
            Sexpr::list([Sexpr::atom("at"), Sexpr::atom("0"), Sexpr::atom("-2")]),
            Sexpr::list([Sexpr::atom("layer"), Sexpr::str("F.SilkS")]),
        ]),
    ];

    for (i, pin) in comp.pins.iter().enumerate() {
        let pad_x = i as f64 * 2.54;
        let pad_y = 0.0;
        let pad_num = (i + 1).to_string();
        let mut pad_children = vec![
            Sexpr::atom("pad"),
            Sexpr::str(&pad_num),
            Sexpr::atom("thru_hole"),
            Sexpr::atom("circle"),
            Sexpr::list([
                Sexpr::atom("at"),
                Sexpr::atom(format_float(pad_x, 2)),
                Sexpr::atom(format_float(pad_y, 2)),
            ]),
            Sexpr::list([
                Sexpr::atom("size"),
                Sexpr::atom("1.524"),
                Sexpr::atom("1.524"),
            ]),
            Sexpr::list([Sexpr::atom("drill"), Sexpr::atom("0.762")]),
            Sexpr::list([
                Sexpr::atom("layers"),
                Sexpr::str("*.Cu"),
                Sexpr::str("*.Mask"),
            ]),
        ];
        if let Some(&net_name) = pin_to_net.get(&(comp.refdes.as_str(), pin.name.as_str()))
            && let Some(&code) = net_to_code.get(net_name)
        {
            pad_children.push(Sexpr::list([
                Sexpr::atom("net"),
                Sexpr::atom(code.to_string()),
                Sexpr::str(net_name),
            ]));
        }
        children.push(Sexpr::list(pad_children));
    }

    Sexpr::list(children)
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf_core::UnitExt;
    use copperleaf_ir::{ComponentInst, Constraint, Design, Limits, Net, NetClass, Pin, Role};

    fn make_design() -> Design {
        let vbus = Net::power("VBUS", 5.0_f64.volt());
        let gnd = Net::ground();
        let mut v3v3 = Net::power("V3V3", 3.3_f64.volt());
        v3v3.class = NetClass {
            min_width: Some(0.3_f64.mm()),
            clearance: Some(0.2_f64.mm()),
        };
        v3v3.constraints.push(Constraint::NetClass {
            min_width: 0.3_f64.mm(),
            clearance: 0.2_f64.mm(),
        });

        let u1 = ComponentInst::new(
            "U1",
            TestBlock {
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
                        Limits::new(0.0_f64.volt(), 0.3_f64.volt(), 1.0_f64.amp()),
                        None,
                    ),
                ],
            },
        );

        let mut d = Design::default();
        d.add_net(vbus);
        d.add_net(gnd);
        d.add_net(v3v3);
        d.add_component(u1);
        d.connect("U1", "VDD", "V3V3");
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
    fn pcb_starts_with_kicad_pcb() {
        let d = make_design();
        let out = emit_pcb(&d);
        assert!(out.starts_with("(kicad_pcb"));
    }

    #[test]
    fn default_net_class_always_present() {
        let d = make_design();
        let out = emit_pcb(&d);
        assert!(out.contains("(net_class \"Default\""));
    }

    #[test]
    fn net_class_values_in_mm() {
        let d = make_design();
        let out = emit_pcb(&d);
        assert!(out.contains("(trace_width 0.3)"));
        assert!(out.contains("(clearance 0.2)"));
        assert!(out.contains("(add_net \"V3V3\")"));
    }

    #[test]
    fn footprint_pad_carries_net() {
        let d = make_design();
        let out = emit_pcb(&d);
        assert!(out.contains("(footprint"));
        assert!(out.contains("(net 3 \"V3V3\")"));
    }

    #[test]
    fn coordinates_are_cleanly_formatted() {
        let d = make_design();
        let out = emit_pcb(&d);
        // Ensure no float imprecision artifacts like 19.049999999999997
        assert!(!out.contains("99999999999"));
        assert!(out.contains("(at 10 10)"));
    }

    #[test]
    fn empty_design_pcb_is_valid() {
        let d = Design::default();
        let out = emit_pcb(&d);
        assert!(out.starts_with("(kicad_pcb"));
        assert!(out.contains("(net_class \"Default\""));
        assert!(!out.contains("(footprint"));
    }

    #[test]
    fn pcb_is_deterministic() {
        let d = make_design();
        let a = emit_pcb(&d);
        let b = emit_pcb(&d);
        assert_eq!(a, b);
    }
}
