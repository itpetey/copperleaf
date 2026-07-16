//! KiCad netlist emitter.

use copperleaf::CompiledBoard;

use crate::{
    common::{build_net_codes, refdes_prefix, role_to_pintype},
    sexpr::{Sexpr, kv},
};

/// Emit a KiCad S-expression netlist for the given compiled board.
pub fn emit_netlist(board: &CompiledBoard) -> String {
    let net_codes = build_net_codes(board);
    let children = vec![
        Sexpr::list([Sexpr::atom("version"), Sexpr::str("E")]),
        design_node(),
        components_node(board),
        nets_node(board, &net_codes),
    ];
    let export = Sexpr::list(std::iter::once(Sexpr::atom("export")).chain(children));
    format!("{}\n", export)
}

fn components_node(board: &CompiledBoard) -> Sexpr {
    let comps: Vec<_> = board
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

fn nets_node(board: &CompiledBoard, net_codes: &[(String, usize)]) -> Sexpr {
    use std::collections::HashMap;
    let mut by_net: HashMap<&str, Vec<&copperleaf::Connection>> = HashMap::new();
    for conn in &board.connections {
        by_net.entry(conn.net.0.as_str()).or_default().push(conn);
    }

    let nets: Vec<_> = net_codes
        .iter()
        .map(|(name, code)| {
            let mut net_children = vec![
                Sexpr::atom("net"),
                Sexpr::list([Sexpr::atom("code"), Sexpr::atom(code.to_string())]),
                kv("name", name),
            ];
            if let Some(conns) = by_net.get(name.as_str()) {
                for conn in conns {
                    net_children.push(node_sexpr(board, conn));
                }
            }
            Sexpr::list(net_children)
        })
        .collect();

    Sexpr::list(std::iter::once(Sexpr::atom("nets")).chain(nets))
}

fn node_sexpr(board: &CompiledBoard, conn: &copperleaf::Connection) -> Sexpr {
    let mut children = vec![Sexpr::atom("node"), kv("ref", conn.component.to_string())];
    let comp = board.components.get(conn.component);
    let pin = comp.and_then(|c| c.pins.iter().find(|p| p.name() == conn.pin));
    if let Some(pin) = pin {
        children.push(kv("pin", pin.name()));
        children.push(kv("pinfunction", pin.name()));
        children.push(kv("pintype", role_to_pintype(pin.role())));
    } else {
        children.push(kv("pin", &conn.pin));
    }
    Sexpr::list(children)
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf::{Connection, Net, NetClass, NetId, Pin, UnitExt};

    fn make_board() -> CompiledBoard {
        CompiledBoard {
            components: vec![
                copperleaf::CompiledComponent {
                    refdes: "U1".into(),
                    pins: vec![
                        Pin::build("VIN").pwr_fixed(5.0.volt(), 1.0.amp()).pin(),
                        Pin::build("GND").gnd(),
                    ],
                    constraints: vec![],                    symbol: None,
                    footprint: None,
                },
                copperleaf::CompiledComponent {
                    refdes: "U2".into(),
                    pins: vec![
                        Pin::build("VDD").pwr_fixed(3.3.volt(), 0.5.amp()).pin(),
                        Pin::build("GPIO").dio(),
                    ],
                    constraints: vec![],                    symbol: None,
                    footprint: None,
                },
            ],
            nets: vec![
                Net {
                    name: "VBUS".into(),
                    kind: copperleaf::NetKind::Power {
                        v_nom: 5.0.volt(),
                        ripple: None,
                    },
                    class: NetClass::default(),
                    constraints: vec![],                },
                Net {
                    name: "V3V3".into(),
                    kind: copperleaf::NetKind::Power {
                        v_nom: 3.3.volt(),
                        ripple: None,
                    },
                    class: NetClass::default(),
                    constraints: vec![],                },
            ],
            connections: vec![
                Connection {
                    component: 0,
                    pin: "VIN".into(),
                    net: NetId("VBUS".into()),
                },
                Connection {
                    component: 1,
                    pin: "VDD".into(),
                    net: NetId("V3V3".into()),
                },
                Connection {
                    component: 1,
                    pin: "GPIO".into(),
                    net: NetId("V3V3".into()),
                },
            ],
            constraints: vec![],        }
    }

    #[test]
    fn netlist_contains_components_and_nets() {
        let board = make_board();
        let out = emit_netlist(&board);
        assert!(out.starts_with("(export"));
        assert!(out.contains("(ref \"U1\")"));
        assert!(out.contains("(name \"VBUS\")"));
    }

    #[test]
    fn netlist_is_deterministic() {
        let board = make_board();
        assert_eq!(emit_netlist(&board), emit_netlist(&board));
    }
}
