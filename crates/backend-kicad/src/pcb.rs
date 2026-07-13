//! KiCad PCB emitter.

use std::collections::HashMap;

use copperleaf_model::{CompiledBoard, NetClass};

use crate::common::{build_net_codes, fmt_mm, format_float};
use crate::sexpr::{Sexpr, deterministic_uuid, kv};

/// Emit a KiCad S-expression PCB file for the given compiled board.
pub fn emit_pcb(board: &CompiledBoard) -> String {
    let net_codes = build_net_codes(board);
    let net_to_code: HashMap<&str, usize> = net_codes
        .iter()
        .map(|(name, code)| (name.as_str(), *code))
        .collect();

    let pin_to_net: HashMap<(usize, &str), &str> = board
        .connections
        .iter()
        .map(|c| ((c.component, c.pin.as_str()), c.net.0.as_str()))
        .collect();

    let mut children: Vec<Sexpr> = vec![
        Sexpr::list([Sexpr::atom("version"), Sexpr::atom("20241229")]),
        kv("generator", "copperleaf"),
        kv("generator_version", "10.0"),
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

    children.extend(net_class_nodes(board, &net_codes));
    children.extend(board_outline());

    for (idx, comp) in board.components.iter().enumerate() {
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
        Sexpr::list([Sexpr::atom("2"), Sexpr::str("B.Cu"), Sexpr::atom("signal")]),
        Sexpr::list([Sexpr::atom("1"), Sexpr::str("F.Mask"), Sexpr::atom("user")]),
        Sexpr::list([Sexpr::atom("3"), Sexpr::str("B.Mask"), Sexpr::atom("user")]),
        Sexpr::list([
            Sexpr::atom("13"),
            Sexpr::str("F.Paste"),
            Sexpr::atom("user"),
        ]),
        Sexpr::list([
            Sexpr::atom("15"),
            Sexpr::str("B.Paste"),
            Sexpr::atom("user"),
        ]),
        Sexpr::list([
            Sexpr::atom("5"),
            Sexpr::str("F.SilkS"),
            Sexpr::atom("user"),
            Sexpr::str("F.Silkscreen"),
        ]),
        Sexpr::list([
            Sexpr::atom("7"),
            Sexpr::str("B.SilkS"),
            Sexpr::atom("user"),
            Sexpr::str("B.Silkscreen"),
        ]),
        Sexpr::list([
            Sexpr::atom("25"),
            Sexpr::str("Edge.Cuts"),
            Sexpr::atom("user"),
        ]),
        Sexpr::list([Sexpr::atom("27"), Sexpr::str("Margin"), Sexpr::atom("user")]),
        Sexpr::list([
            Sexpr::atom("31"),
            Sexpr::str("F.CrtYd"),
            Sexpr::atom("user"),
            Sexpr::str("F.Courtyard"),
        ]),
        Sexpr::list([
            Sexpr::atom("29"),
            Sexpr::str("B.CrtYd"),
            Sexpr::atom("user"),
            Sexpr::str("B.Courtyard"),
        ]),
        Sexpr::list([Sexpr::atom("35"), Sexpr::str("F.Fab"), Sexpr::atom("user")]),
        Sexpr::list([Sexpr::atom("33"), Sexpr::str("B.Fab"), Sexpr::atom("user")]),
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

fn board_outline() -> Vec<Sexpr> {
    let rect = [
        ((0.0, 0.0), (100.0, 0.0), "top"),
        ((100.0, 0.0), (100.0, 80.0), "right"),
        ((100.0, 80.0), (0.0, 80.0), "bottom"),
        ((0.0, 80.0), (0.0, 0.0), "left"),
    ];
    rect.iter()
        .map(|((x1, y1), (x2, y2), side)| {
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
                Sexpr::list([
                    Sexpr::atom("stroke"),
                    Sexpr::list([Sexpr::atom("width"), Sexpr::atom("0.05")]),
                    Sexpr::list([Sexpr::atom("type"), Sexpr::atom("solid")]),
                ]),
                Sexpr::list([Sexpr::atom("layer"), Sexpr::str("Edge.Cuts")]),
                Sexpr::list([
                    Sexpr::atom("uuid"),
                    Sexpr::str(deterministic_uuid(&format!("pcb:outline:{}", side))),
                ]),
            ])
        })
        .collect()
}

fn footprint_node(
    idx: usize,
    comp: &copperleaf_model::CompiledComponent,
    pin_to_net: &HashMap<(usize, &str), &str>,
    net_to_code: &HashMap<&str, usize>,
) -> Sexpr {
    const PITCH: f64 = 10.0;
    let x = 10.0 + (idx as f64 % 10.0) * PITCH;
    let y = 10.0 + (idx as f64 / 10.0).floor() * PITCH;

    let n_pins = comp.pins.len();
    let pad_span = if n_pins == 0 {
        0.0
    } else {
        (n_pins as f64 - 1.0) * 2.54
    };
    let body_w = pad_span + 2.0 * (0.762 + 0.5);
    let body_h = 2.0 * (0.762 + 0.5);
    let half_w = body_w / 2.0;
    let body_cx = pad_span / 2.0;

    let fp_uuid = deterministic_uuid(&format!("pcb:{}", comp.refdes));
    let fp_name = comp.footprint.as_deref().unwrap_or("copperleaf:Generic");

    let mut children = vec![
        Sexpr::atom("footprint"),
        Sexpr::str(fp_name),
        Sexpr::list([Sexpr::atom("locked"), Sexpr::atom("no")]),
        Sexpr::list([Sexpr::atom("layer"), Sexpr::str("F.Cu")]),
        Sexpr::list([Sexpr::atom("uuid"), Sexpr::str(&fp_uuid)]),
        Sexpr::list([
            Sexpr::atom("at"),
            Sexpr::atom(format_float(x, 2)),
            Sexpr::atom(format_float(y, 2)),
            Sexpr::atom("0"),
        ]),
        fp_text_node(
            "reference",
            &comp.refdes,
            body_cx,
            -body_h / 2.0 - 1.0,
            "F.SilkS",
            &format!("{}:ref", fp_uuid),
        ),
        fp_text_node(
            "value",
            &crate::common::refdes_prefix(&comp.refdes),
            body_cx,
            body_h / 2.0 + 1.0,
            "F.Fab",
            &format!("{}:val", fp_uuid),
        ),
    ];

    let x1 = body_cx - half_w;
    let y1 = -body_h / 2.0;
    let x2 = body_cx + half_w;
    let y2 = body_h / 2.0;
    let seed = format!("pcb:{}:outline", comp.refdes);
    children.push(fp_line(
        (x1, y1),
        (x2, y1),
        "F.SilkS",
        &format!("{}_top", seed),
    ));
    children.push(fp_line(
        (x2, y1),
        (x2, y2),
        "F.SilkS",
        &format!("{}_right", seed),
    ));
    children.push(fp_line(
        (x2, y2),
        (x1, y2),
        "F.SilkS",
        &format!("{}_bot", seed),
    ));
    children.push(fp_line(
        (x1, y2),
        (x1, y1),
        "F.SilkS",
        &format!("{}_left", seed),
    ));

    for (i, pin) in comp.pins.iter().enumerate() {
        let pad_x = i as f64 * 2.54;
        let pad_y = 0.0;
        let pad_num = (i + 1).to_string();
        let pad_uuid = deterministic_uuid(&format!("pcb:{}:pad{}", comp.refdes, pad_num));
        let mut pad_children = vec![
            Sexpr::atom("pad"),
            Sexpr::str(&pad_num),
            Sexpr::atom("thru_hole"),
            Sexpr::atom("circle"),
            Sexpr::list([
                Sexpr::atom("at"),
                Sexpr::atom(format_float(pad_x, 2)),
                Sexpr::atom(format_float(pad_y, 2)),
                Sexpr::atom("0"),
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
            Sexpr::list([Sexpr::atom("remove_unused_layers"), Sexpr::atom("no")]),
            Sexpr::list([Sexpr::atom("uuid"), Sexpr::str(&pad_uuid)]),
        ];
        if let Some(&net_name) = pin_to_net.get(&(idx, pin.name()))
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

fn fp_text_node(
    text_type: &str,
    text: &str,
    x: f64,
    y: f64,
    layer: &str,
    uuid_seed: &str,
) -> Sexpr {
    Sexpr::list([
        Sexpr::atom("fp_text"),
        Sexpr::atom(text_type),
        Sexpr::str(text),
        Sexpr::list([
            Sexpr::atom("at"),
            Sexpr::atom(format_float(x, 2)),
            Sexpr::atom(format_float(y, 2)),
            Sexpr::atom("0"),
        ]),
        Sexpr::list([Sexpr::atom("layer"), Sexpr::str(layer)]),
        Sexpr::list([
            Sexpr::atom("effects"),
            Sexpr::list([
                Sexpr::atom("font"),
                Sexpr::list([Sexpr::atom("size"), Sexpr::atom("1.0"), Sexpr::atom("1.0")]),
                Sexpr::list([Sexpr::atom("thickness"), Sexpr::atom("0.15")]),
            ]),
            Sexpr::list([Sexpr::atom("justify"), Sexpr::atom("left")]),
        ]),
        Sexpr::list([
            Sexpr::atom("uuid"),
            Sexpr::str(deterministic_uuid(uuid_seed)),
        ]),
    ])
}

fn fp_line(from: (f64, f64), to: (f64, f64), layer: &str, uuid_seed: &str) -> Sexpr {
    Sexpr::list([
        Sexpr::atom("fp_line"),
        Sexpr::list([
            Sexpr::atom("start"),
            Sexpr::atom(format_float(from.0, 2)),
            Sexpr::atom(format_float(from.1, 2)),
        ]),
        Sexpr::list([
            Sexpr::atom("end"),
            Sexpr::atom(format_float(to.0, 2)),
            Sexpr::atom(format_float(to.1, 2)),
        ]),
        Sexpr::list([
            Sexpr::atom("stroke"),
            Sexpr::list([Sexpr::atom("width"), Sexpr::atom("0.12")]),
            Sexpr::list([Sexpr::atom("type"), Sexpr::atom("solid")]),
        ]),
        Sexpr::list([Sexpr::atom("layer"), Sexpr::str(layer)]),
        Sexpr::list([
            Sexpr::atom("uuid"),
            Sexpr::str(deterministic_uuid(uuid_seed)),
        ]),
    ])
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

fn net_class_nodes(board: &CompiledBoard, net_codes: &[(String, usize)]) -> Vec<Sexpr> {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    let mut default_nets: Vec<String> = Vec::new();

    for (name, _) in net_codes {
        let net = board.nets.iter().find(|n| &n.name == name);
        match net.map(|n| &n.class) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf_model::{CompiledComponent, Connection, Net, NetClass, NetId, Pin, UnitExt};

    #[test]
    fn pcb_starts_with_kicad_pcb() {
        let board = CompiledBoard {
            components: vec![CompiledComponent {
                refdes: "U1".into(),
                pins: vec![Pin::build("VDD").pwr_fixed(3.3.volt(), 0.1.amp()).pin()],
                constraints: vec![],
                symbol: None,
                footprint: None,
            }],
            nets: vec![Net {
                name: "V3V3".into(),
                kind: copperleaf_model::NetKind::Power {
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
        let out = emit_pcb(&board);
        assert!(out.starts_with("(kicad_pcb"));
        assert!(out.contains("(net_class \"Default\""));
        assert!(out.contains("(footprint"));
    }
}
