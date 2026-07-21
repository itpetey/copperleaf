//! KiCad schematic emitter.

use std::collections::BTreeMap;

use copperleaf::{CompiledBoard, CompiledComponent, Connection, NetIdx, NetKind, Role};

use crate::{
    common::{footprint_ref, format_float, property_sym_node, refdes_prefix, symbol_lib_id},
    deterministic_id, fp_geom,
    lib_emitter::symbol_def_sexpr,
    sexpr::{Sexpr, kv},
    sym_layout::{self, LayoutPin, SymbolLayout},
};

/// Emit a minimal structurally-valid KiCad 10 schematic.
pub fn emit_schematic(board: &CompiledBoard) -> String {
    // One deterministic symbol layout per component, shared by the embedded
    // `lib_symbols` and the wire-tip computation so they always agree.
    let layouts: Vec<SymbolLayout> = board.components.iter().map(layout_for_comp).collect();
    let positions = symbol_positions(&layouts);

    let mut children: Vec<Sexpr> = vec![
        Sexpr::list([Sexpr::atom("version"), Sexpr::atom("20260306")]),
        kv("generator", "copperleaf"),
        kv("generator_version", "10.0"),
        Sexpr::list([
            Sexpr::atom("uuid"),
            Sexpr::str(deterministic_id("sch:root")),
        ]),
        kv("paper", "A4"),
        title_block_node(),
        lib_symbols_node(board),
    ];

    for (idx, comp) in board.components.iter().enumerate() {
        children.push(symbol_instance_node(comp, &layouts[idx], positions[idx]));
    }

    // A `BTreeMap` keeps wire/label emission order deterministic across
    // processes (std `HashMap` iteration order is randomised per process).
    let mut net_conns: BTreeMap<NetIdx, Vec<&Connection>> = BTreeMap::new();
    for conn in &board.connections {
        net_conns.entry(conn.net).or_default().push(conn);
    }

    for (net_idx, conns) in &net_conns {
        let net = board.net(*net_idx);
        let net_name = net.name.as_str();
        let tips: Vec<((f64, f64), f64)> = conns
            .iter()
            .filter_map(|conn| pin_tip_and_label(board, &layouts, &positions, conn))
            .collect();
        if tips.is_empty() {
            continue;
        }

        if is_power_net(board, *net_idx) {
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

fn is_power_net(board: &CompiledBoard, net_idx: NetIdx) -> bool {
    matches!(board.net(net_idx).kind, NetKind::Power { .. })
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
            Sexpr::str(deterministic_id(&format!(
                "sch:label:{}:{:.2}:{:.2}",
                name, x, y
            ))),
        ]),
    ])
}

/// Compute the symbol layout for a component from its pin roles.
///
/// Includes electrical pins, thermal vias, and mechanical pads so the
/// schematic pin count always matches the PCB pad count.
fn layout_for_comp(comp: &CompiledComponent) -> SymbolLayout {
    let mut pins: Vec<LayoutPin> = comp
        .pins
        .iter()
        .enumerate()
        .map(|(i, p)| LayoutPin {
            name: p.name().to_string(),
            number: fp_geom::pin_number(p, i),
            role: p.role(),
        })
        .collect();

    for (number, name) in fp_geom::mech_pad_names(comp) {
        pins.push(LayoutPin {
            name,
            number,
            role: Role::Passive,
        });
    }

    sym_layout::layout_symbol(&pins)
}

/// Embedded `lib_symbols` section: one entry per unique symbol identifier.
fn lib_symbols_node(board: &CompiledBoard) -> Sexpr {
    let mut seen = std::collections::HashSet::new();
    let symbols: Vec<_> = board
        .components
        .iter()
        .filter_map(|comp| {
            let id = symbol_lib_id(comp);
            if seen.insert(id.clone()) {
                Some(symbol_def_sexpr(comp, &id))
            } else {
                None
            }
        })
        .collect();
    Sexpr::list(std::iter::once(Sexpr::atom("lib_symbols")).chain(symbols))
}

fn manhattan_wires(from: (f64, f64), to: (f64, f64), net_name: &str) -> Vec<Sexpr> {
    if (from.0 - to.0).abs() < 0.01 || (from.1 - to.1).abs() < 0.01 {
        vec![wire_seg(from, to, net_name)]
    } else {
        let corner = (to.0, from.1);
        vec![
            wire_seg(from, corner, net_name),
            wire_seg(corner, to, net_name),
        ]
    }
}

/// Locate the sheet position and rotation of a connected pin's tip using the
/// same layout that produced the embedded `lib_symbols`.
fn pin_tip_and_label(
    board: &CompiledBoard,
    layouts: &[SymbolLayout],
    positions: &[(f64, f64)],
    conn: &copperleaf::Connection,
) -> Option<((f64, f64), f64)> {
    let layout = layouts.get(conn.component)?;
    let (sym_x, sym_y) = positions.get(conn.component)?;
    let _ = board;
    let pin = layout.pins.iter().find(|p| p.name == conn.pin)?;
    // Symbol coordinates have Y up; the sheet has Y down.
    Some(((sym_x + pin.x, sym_y - pin.y), pin.rotation))
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

fn stub_end((tip_x, tip_y): (f64, f64), rotation: f64) -> (f64, f64) {
    let len = 2.54;
    match rotation.round() as i32 {
        0 => (tip_x - len, tip_y),
        90 => (tip_x, tip_y + len),
        180 => (tip_x + len, tip_y),
        _ => (tip_x, tip_y - len),
    }
}

fn symbol_instance_node(comp: &CompiledComponent, layout: &SymbolLayout, pos: (f64, f64)) -> Sexpr {
    let (x, y) = pos;
    let lib_id = symbol_lib_id(comp);
    let fp_value = footprint_ref(comp);

    let properties = vec![
        property_sym_node(
            "Reference",
            &comp.refdes,
            (x, y - layout.y1 - 1.27),
            false,
            false,
        ),
        property_sym_node(
            "Value",
            &refdes_prefix(&comp.refdes),
            (x, y - layout.y2 + 1.27),
            false,
            false,
        ),
        property_sym_node("Footprint", &fp_value, (x, y), true, false),
    ];

    Sexpr::list(
        std::iter::once(Sexpr::atom("symbol"))
            .chain(std::iter::once(Sexpr::list([
                Sexpr::atom("lib_id"),
                Sexpr::str(&lib_id),
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
                Sexpr::str(deterministic_id(&format!("sch:{}", comp.refdes))),
            ])))
            .chain(properties)
            .chain(std::iter::once(Sexpr::list([
                Sexpr::atom("instances"),
                Sexpr::list([
                    Sexpr::atom("project"),
                    Sexpr::str(""),
                    Sexpr::list([
                        Sexpr::atom("path"),
                        Sexpr::str(format!("/ {}", deterministic_id("sch:root"))),
                        Sexpr::list([Sexpr::atom("reference"), Sexpr::str(&comp.refdes)]),
                        Sexpr::list([Sexpr::atom("unit"), Sexpr::atom("1")]),
                    ]),
                ]),
            ]))),
    )
}

/// Row-pack symbol instances on the sheet, keeping every origin on the
/// 2.54 mm connection grid so pin tips always land on-grid.
fn symbol_positions(layouts: &[SymbolLayout]) -> Vec<(f64, f64)> {
    const START: f64 = 25.4;
    const GAP: f64 = 12.7;
    const MAX_X: f64 = 300.0;
    const MARGIN: f64 = sym_layout::PIN_LENGTH + 2.54;

    let mut out = Vec::with_capacity(layouts.len());
    let mut cursor_x = START;
    let mut cursor_y = START;
    let mut row_height: f64 = 0.0;

    for l in layouts {
        let w = (l.x2 - l.x1) + 2.0 * MARGIN;
        let h = (l.y1 - l.y2) + 2.0 * MARGIN;

        if cursor_x + w > MAX_X && cursor_x > START {
            cursor_x = START;
            cursor_y += row_height + GAP;
            row_height = 0.0;
        }

        // Origin such that the symbol extent (body plus pin stubs) lands at
        // the cursor; sheet Y is down, so the top edge is origin_y - y1.
        let origin_x = cursor_x + MARGIN - l.x1;
        let origin_y = cursor_y + MARGIN + l.y1;
        out.push((origin_x, origin_y));

        cursor_x += w + GAP;
        row_height = row_height.max(h);
    }

    out
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
            Sexpr::str(deterministic_id(&format!(
                "sch:wire:{}:{:.2}:{:.2}:{:.2}:{:.2}",
                net_name, from.0, from.1, to.0, to.1
            ))),
        ]),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf::{Connection, Net, NetClass, NetIdx, Pin, UnitExt};

    fn test_board() -> CompiledBoard {
        CompiledBoard {
            components: vec![CompiledComponent {
                refdes: "U1".into(),
                meta: copperleaf::ComponentMeta {
                    symbol: Some("MCU:RP2354a".into()),
                    footprint: Some("Package_QFP:LQFP-64".into()),
                    ..copperleaf::ComponentMeta::default()
                },
                pins: vec![
                    Pin::build("VDD")
                        .number("1")
                        .pwr_fixed(3.3.volt(), 0.1.amp())
                        .pin(),
                ],
                constraints: vec![],
                mechanical: vec![],
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
                net: NetIdx(0),
            }],
            constraints: vec![],
            width: 100.0,
            height: 80.0,
        }
    }

    #[test]
    fn schematic_starts_with_kicad_sch() {
        let out = emit_schematic(&test_board());
        assert!(out.starts_with("(kicad_sch"));
        assert!(out.contains("MCU:RP2354a"));
        assert!(out.contains("Package_QFP:LQFP-64"));
    }

    #[test]
    fn lib_symbols_deduplicated() {
        let mut board = test_board();
        let mut comp2 = board.components[0].clone();
        comp2.refdes = "U2".into();
        board.components.push(comp2);
        let out = emit_schematic(&board);
        // Only one embedded definition of MCU:RP2354a, two instances.
        assert_eq!(out.matches("(symbol \"MCU:RP2354a\"").count(), 1, "{}", out);
        assert_eq!(
            out.matches("(lib_id \"MCU:RP2354a\")").count(),
            2,
            "{}",
            out
        );
    }

    #[test]
    fn project_local_symbols_get_copperleaf_prefix() {
        let mut board = test_board();
        board.components[0].meta.symbol = Some("RP2354A".into());
        let out = emit_schematic(&board);
        assert!(out.contains("(lib_id \"copperleaf:RP2354A\")"), "{}", out);
        assert!(out.contains("(symbol \"copperleaf:RP2354A\""), "{}", out);
    }

    #[test]
    fn pin_tips_match_embedded_symbol() {
        let board = test_board();
        let layouts: Vec<SymbolLayout> = board.components.iter().map(layout_for_comp).collect();
        let positions = symbol_positions(&layouts);
        let conn = &board.connections[0];
        let ((tip_x, tip_y), _rot) = pin_tip_and_label(&board, &layouts, &positions, conn).unwrap();
        // VDD is a power pin on the top edge (rotation 270), so the tip is
        // above the symbol origin on the sheet.
        let (ox, oy) = positions[0];
        assert!(
            tip_y < oy,
            "top pin tip should be above origin: {tip_y} vs {oy}"
        );
        let pin = layouts[0].pins.iter().find(|p| p.name == "VDD").unwrap();
        assert!((tip_x - (ox + pin.x)).abs() < 1e-9);
        assert!((tip_y - (oy - pin.y)).abs() < 1e-9);
    }
}
