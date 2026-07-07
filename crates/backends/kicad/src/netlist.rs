use std::collections::HashMap;

use copperleaf_ir::{Design, Role};

use crate::{
    common::{build_net_codes, refdes_prefix},
    sexpr::{Sexpr, kv},
};

/// Emit a KiCad S-expression netlist for the given design.
pub fn emit_netlist(design: &Design) -> String {
    let net_codes = build_net_codes(design);

    let children = vec![
        Sexpr::list([Sexpr::atom("version"), Sexpr::str("E")]),
        design_node(),
        components_node(design),
        nets_node(design, &net_codes),
    ];

    let export = Sexpr::list(std::iter::once(Sexpr::atom("export")).chain(children));
    format!("{}\n", export)
}

fn components_node(design: &Design) -> Sexpr {
    let comps: Vec<_> = design
        .components
        .iter()
        .map(|c| {
            Sexpr::list([
                Sexpr::atom("comp"),
                kv("ref", &c.refdes),
                kv("value", refdes_prefix(&c.refdes)),
            ])
        })
        .collect();
    Sexpr::list(std::iter::once(Sexpr::atom("components")).chain(comps))
}

fn connections_by_net(design: &Design) -> HashMap<&str, Vec<&copperleaf_ir::Connection>> {
    let mut map: HashMap<&str, Vec<&copperleaf_ir::Connection>> = HashMap::new();
    for conn in &design.connections {
        map.entry(&conn.net).or_default().push(conn);
    }
    map
}

fn design_node() -> Sexpr {
    Sexpr::list([
        Sexpr::atom("design"),
        kv("source", "copperleaf"),
        kv("date", ""),
        Sexpr::list([
            Sexpr::atom("tool"),
            Sexpr::str("copperleaf"),
            Sexpr::atom("version"),
            Sexpr::atom("0.1"),
        ]),
        Sexpr::list([
            Sexpr::atom("sheet"),
            kv("number", "1"),
            kv("name", "/"),
            kv("tstamps", "/"),
        ]),
    ])
}

fn find_pin<'d>(design: &'d Design, refdes: &str, pin: &str) -> Option<&'d copperleaf_ir::Pin> {
    design
        .component_by_refdes(refdes)
        .and_then(|c| c.pins.iter().find(|p| p.name == pin))
}

fn nets_node(design: &Design, net_codes: &[(String, usize)]) -> Sexpr {
    let conns_by_net = connections_by_net(design);

    let nets: Vec<_> = net_codes
        .iter()
        .map(|(name, code)| {
            let mut net_children = vec![
                Sexpr::atom("net"),
                Sexpr::list([Sexpr::atom("code"), Sexpr::atom(code.to_string())]),
                kv("name", name),
            ];
            if let Some(conns) = conns_by_net.get(name.as_str()) {
                for conn in conns {
                    net_children.push(node_sexpr(design, conn));
                }
            }
            Sexpr::list(net_children)
        })
        .collect();

    Sexpr::list(std::iter::once(Sexpr::atom("nets")).chain(nets))
}

fn node_sexpr(design: &Design, conn: &copperleaf_ir::Connection) -> Sexpr {
    let mut children = vec![
        Sexpr::atom("node"),
        kv("ref", &conn.refdes),
        kv("pin", &conn.pin),
    ];
    if let Some(pin) = find_pin(design, &conn.refdes, &conn.pin) {
        children.push(kv("pinfunction", &pin.name));
        if let Some(ptype) = role_to_pintype(pin.role) {
            children.push(kv("pintype", ptype));
        }
    }
    Sexpr::list(children)
}

fn role_to_pintype(role: Role) -> Option<&'static str> {
    match role {
        Role::PowerIn | Role::Gnd => Some("power_in"),
        Role::PowerOut => Some("power_out"),
        Role::AnalogIn => Some("input"),
        Role::AnalogOut => Some("output"),
        Role::DigitalIO | Role::DiffPos | Role::DiffNeg => Some("bidirectional"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf_core::UnitExt;
    use copperleaf_ir::{ComponentInst, Design, Limits, Net, Pin, Role};

    fn make_design() -> Design {
        let vbus = Net::power("VBUS", 5.0_f64.volt());
        let gnd = Net::ground();
        let v3v3 = Net::power("V3V3", 3.3_f64.volt());

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
                ],
            },
        );
        let u2 = ComponentInst::new(
            "U2",
            TestBlock {
                pins: vec![
                    Pin::new(
                        "VDD",
                        Role::PowerIn,
                        Limits::new(1.7_f64.volt(), 3.6_f64.volt(), 0.5_f64.amp()),
                        None,
                    ),
                    Pin::new(
                        "GPIO",
                        Role::DigitalIO,
                        Limits::new(0.0_f64.volt(), 3.6_f64.volt(), 0.02_f64.amp()),
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
        d.add_component(u2);
        d.connect("U1", "VIN", "VBUS");
        d.connect("U2", "VDD", "V3V3");
        d.connect("U2", "GPIO", "V3V3");
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
    fn netlist_contains_components_and_nets() {
        let d = make_design();
        let out = emit_netlist(&d);
        assert!(out.starts_with("(export"));
        assert!(out.contains("(ref \"U1\")"));
        assert!(out.contains("(value \"U\")"));
        assert!(out.contains("(ref \"U2\")"));
        assert!(out.contains("(code 1)"));
        assert!(out.contains("(name \"VBUS\")"));
        assert!(out.contains("(code 3)"));
        assert!(out.contains("(name \"V3V3\")"));
    }

    #[test]
    fn netlist_nodes_include_pinfunction_and_pintype() {
        let d = make_design();
        let out = emit_netlist(&d);
        assert!(out.contains("(pinfunction \"VIN\")"));
        assert!(out.contains("(pintype \"power_in\")"));
        assert!(out.contains("(pinfunction \"VDD\")"));
    }

    #[test]
    fn digital_io_pin_gets_bidirectional_pintype() {
        let d = make_design();
        let out = emit_netlist(&d);
        assert!(out.contains("(pinfunction \"GPIO\")"));
        assert!(out.contains("(pintype \"bidirectional\")"));
    }

    #[test]
    fn missing_pin_omits_pinfunction_and_pintype() {
        let mut d = make_design();
        // Connect a pin that does not exist on the component.
        d.connect("U1", "NO_SUCH_PIN", "VBUS");
        let out = emit_netlist(&d);
        // The node for the missing pin should have ref/pin but no pinfunction/pintype.
        assert!(out.contains("(ref \"U1\")"));
        assert!(out.contains("(pin \"NO_SUCH_PIN\")"));
        assert!(!out.contains("(pinfunction \"NO_SUCH_PIN\")"));
    }

    #[test]
    fn refdes_prefixes_appear_in_netlist() {
        let c2 = ComponentInst::new(
            "C2",
            TestBlock {
                pins: vec![Pin::new(
                    "1",
                    Role::DigitalIO,
                    Limits::new(0.0_f64.volt(), 3.6_f64.volt(), 0.1_f64.amp()),
                    None,
                )],
            },
        );
        let no_prefix = ComponentInst::new(
            "3V3",
            TestBlock {
                pins: vec![Pin::new(
                    "1",
                    Role::DigitalIO,
                    Limits::new(0.0_f64.volt(), 3.6_f64.volt(), 0.1_f64.amp()),
                    None,
                )],
            },
        );

        let mut d = Design::default();
        d.add_net(Net::power("V3V3", 3.3_f64.volt()));
        d.add_component(c2);
        d.add_component(no_prefix);
        d.connect("C2", "1", "V3V3");
        d.connect("3V3", "1", "V3V3");

        let out = emit_netlist(&d);
        assert!(out.contains("(value \"C\")"));
        assert!(out.contains("(value \"?\")"));
    }

    #[test]
    fn empty_design_emits_valid_netlist() {
        let d = Design::default();
        let out = emit_netlist(&d);
        assert!(out.starts_with("(export"));
        assert!(out.contains("(components)"));
        assert!(out.contains("(nets)"));
        assert!(!out.contains("(comp "));
        assert!(!out.contains("(node "));
    }

    #[test]
    fn netlist_is_deterministic() {
        let d = make_design();
        let a = emit_netlist(&d);
        let b = emit_netlist(&d);
        assert_eq!(a, b);
    }
}
