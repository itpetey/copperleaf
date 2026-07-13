//! KiCad schematic emitter.

use std::collections::HashMap;

use copperleaf_model::{CompiledBoard, CompiledComponent, NetKind, Pin};

use crate::common::{format_float, refdes_prefix, role_to_pin_type};
use crate::sexpr::{Sexpr, deterministic_uuid, kv};

/// Emit a minimal structurally-valid KiCad 10 schematic.
pub fn emit_schematic(board: &CompiledBoard) -> String {
    let mut children: Vec<Sexpr> = vec![
        Sexpr::list([Sexpr::atom("version"), Sexpr::atom("20260306")]),
        kv("generator", "copperleaf"),
        kv("generator_version", "10.0"),
        Sexpr::list([
            Sexpr::atom("uuid"),
            Sexpr::str(deterministic_uuid("sch:root")),
        ]),
        kv("paper", "A4"),
        title_block_node(),
        lib_symbols_node(board),
    ];

    for (idx, comp) in board.components.iter().enumerate() {
        children.push(symbol_instance_node(idx, comp));
    }

    let mut net_conns: HashMap<&str, Vec<&copperleaf_model::Connection>> = HashMap::new();
    for conn in &board.connections {
        net_conns.entry(conn.net.0.as_str()).or_default().push(conn);
    }

    for (net_name, conns) in &net_conns {
        let tips: Vec<((f64, f64), f64)> = conns
            .iter()
            .filter_map(|conn| pin_tip_and_label(board, conn))
            .collect();
        if tips.is_empty() {
            continue;
        }

        if is_power_net(board, net_name) {
            let mut seen = std::collections::HashSet::new();
            for ((tip_x, tip_y), rotation) in &tips {
                let end = stub_end((*tip_x, *tip_y), *rotation);
                let key = format!("{:.2}:{:.2}", end.0, end.1);
                if !seen.insert(key) {
                    continue;
                }
                children.push(wire_seg((*tip_x, *tip_y), end, net_name));
                children.push(label_at(net_name, end.0, end.1));
            }
        } else {
            let positions: Vec<(f64, f64)> = tips.iter().map(|(p, _)| *p).collect();
            for pair in positions.windows(2) {
                for wire in manhattan_wires(pair[0], pair[1], net_name) {
                    children.push(wire);
                }
            }
            children.push(label_at(net_name, positions[0].0, positions[0].1));
        }
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
    ])
}

fn lib_symbols_node(board: &CompiledBoard) -> Sexpr {
    let symbols: Vec<_> = board
        .components
        .iter()
        .map(lib_symbol_for_component)
        .collect();
    Sexpr::list(std::iter::once(Sexpr::atom("lib_symbols")).chain(symbols))
}

fn lib_symbol_for_component(comp: &CompiledComponent) -> Sexpr {
    let fallback = format!("copperleaf:{}", comp.refdes);
    let symbol_name = comp.symbol.as_deref().unwrap_or(&fallback);
    let fp_default = comp.footprint.as_deref().unwrap_or("");
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
            Sexpr::str(format!("{}_0_1", comp.refdes)),
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

    for (i, pin) in comp.pins.iter().enumerate() {
        body.push(lib_pin_node(pin, i, comp.pins.len()));
    }

    Sexpr::list(body)
}

fn lib_pin_node(pin: &Pin, index: usize, total_pins: usize) -> Sexpr {
    let pin_type = role_to_pin_type(pin.role());
    let (x, y, rotation) = match pin.pos() {
        Some((px, py)) => (px, py, pin.rotation().unwrap_or(180.0)),
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
            Sexpr::atom(format_float(pin.length().unwrap_or(2.54), 2)),
        ]),
        Sexpr::list([
            Sexpr::atom("name"),
            Sexpr::str(pin.name()),
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

fn symbol_instance_node(idx: usize, comp: &CompiledComponent) -> Sexpr {
    let (x, y) = symbol_position(idx);
    let fallback = format!("copperleaf:{}", comp.refdes);
    let lib_id = comp.symbol.as_deref().unwrap_or(&fallback);

    let mut properties = vec![
        property_node("Reference", &comp.refdes, x, y - 6.35, false),
        property_node("Value", &refdes_prefix(&comp.refdes), x, y + 6.35, false),
    ];
    if let Some(fp) = &comp.footprint {
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
                        Sexpr::str(format!("/ {}", deterministic_uuid("sch:root"))),
                        Sexpr::list([Sexpr::atom("reference"), Sexpr::str(&comp.refdes)]),
                        Sexpr::list([Sexpr::atom("unit"), Sexpr::atom("1")]),
                    ]),
                ]),
            ]))),
    )
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

fn pin_y_offset(index: usize, total_pins: usize) -> f64 {
    if total_pins <= 1 {
        0.0
    } else {
        let spacing = 2.54;
        let total_height = (total_pins as f64 - 1.0) * spacing;
        -total_height / 2.0 + index as f64 * spacing
    }
}

fn symbol_position(idx: usize) -> (f64, f64) {
    const GRID: f64 = 25.4;
    let x = GRID + (idx as f64 % 10.0) * GRID;
    let y = GRID + (idx as f64 / 10.0).floor() * GRID;
    (x, y)
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

fn pin_tip_and_label(
    board: &CompiledBoard,
    conn: &copperleaf_model::Connection,
) -> Option<((f64, f64), f64)> {
    let comp = board.components.get(conn.component)?;
    let pin = comp.pins.iter().find(|p| p.name() == conn.pin)?;
    let (sym_x, sym_y) = symbol_position(conn.component);
    let (tip_x, tip_y) = match pin.pos() {
        Some((px, py)) => (sym_x + px, sym_y - py),
        None => {
            let y_off = pin_y_offset(
                comp.pins
                    .iter()
                    .position(|p| p.name() == pin.name())
                    .unwrap_or(0),
                comp.pins.len(),
            );
            (sym_x + 7.62, sym_y + y_off)
        }
    };
    let rotation = pin.rotation().unwrap_or(180.0);
    Some(((tip_x, tip_y), rotation))
}

fn stub_end((tip_x, tip_y): (f64, f64), rotation: f64) -> (f64, f64) {
    let len = 2.54;
    match rotation.round() as i32 {
        0 => (tip_x - len, tip_y),
        90 => (tip_x, tip_y + len),
        180 => (tip_x + len, tip_y),
        _ => (tip_x, tip_y - len),
    }
}

fn is_power_net(board: &CompiledBoard, net_name: &str) -> bool {
    board
        .nets
        .iter()
        .any(|n| n.name == net_name && matches!(n.kind, NetKind::Power { .. }))
}

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
            Sexpr::str(deterministic_uuid(&format!(
                "sch:label:{}:{:.2}:{:.2}",
                name, x, y
            ))),
        ]),
    ])
}

fn manhattan_wires(from: (f64, f64), to: (f64, f64), net_name: &str) -> Vec<Sexpr> {
    if (from.0 - to.0).abs() < 0.01 {
        vec![wire_seg(from, to, net_name)]
    } else if (from.1 - to.1).abs() < 0.01 {
        vec![wire_seg(from, to, net_name)]
    } else {
        let corner = (to.0, from.1);
        vec![
            wire_seg(from, corner, net_name),
            wire_seg(corner, to, net_name),
        ]
    }
}

fn wire_seg(from: (f64, f64), to: (f64, f64), net_name: &str) -> Sexpr {
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
                "sch:wire:{}:{:.2}:{:.2}:{:.2}:{:.2}",
                net_name, from.0, from.1, to.0, to.1
            ))),
        ]),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf_model::{CompiledComponent, Connection, Net, NetClass, NetId, Pin, UnitExt};

    #[test]
    fn schematic_starts_with_kicad_sch() {
        let board = CompiledBoard {
            components: vec![CompiledComponent {
                refdes: "U1".into(),
                pins: vec![Pin::build("VDD").pwr_fixed(3.3.volt(), 0.1.amp()).pin()],
                constraints: vec![],
                symbol: Some("MCU:RP2354a".into()),
                footprint: Some("Package_QFP:LQFP-64".into()),
            }],
            nets: vec![Net {
                name: "V3V3".into(),
                kind: NetKind::Power {
                    v_nom: 3.3.volt(),
                    ripple: None,
                },
                class: NetClass::default(),
                constraints: vec![],
            }],
            connections: vec![Connection {
                component: 0,
                pin: "VDD".into(),
                net: NetId("V3V3".into()),
            }],
            constraints: vec![],
        };
        let out = emit_schematic(&board);
        assert!(out.starts_with("(kicad_sch"));
        assert!(out.contains("MCU:RP2354a"));
        assert!(out.contains("Package_QFP:LQFP-64"));
    }
}
