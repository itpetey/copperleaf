//! KiCad PCB emitter.

use std::{collections::HashMap, path::Path};

use copperleaf::{CompiledBoard, NetClass, NetIdx};

use crate::{
    common::{build_net_codes, fmt_mm, footprint_ref, format_float, format_grid_float},
    deterministic_id, fp_geom,
    sexpr::{Sexpr, kv},
};

/// Emit a KiCad S-expression PCB file for the given compiled board.
pub fn emit_pcb(board: &CompiledBoard, project_name: &str) -> String {
    let net_codes = build_net_codes(board);
    let net_to_code: HashMap<usize, usize> = net_codes
        .iter()
        .enumerate()
        .map(|(idx, (_, code))| (idx, *code))
        .collect();

    let pin_to_net: HashMap<(usize, &str), NetIdx> = board
        .connections
        .iter()
        .map(|c| ((c.component, c.pin.as_str()), c.net))
        .collect();

    let mut children: Vec<Sexpr> = vec![
        Sexpr::list([Sexpr::atom("version"), Sexpr::atom("20260206")]),
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
    children.extend(board_outline(board.width, board.height));

    // Auto-place components in rows, packing by their courtyard extents so
    // footprints do not overlap.
    let placements = auto_place(board, board.width);

    for (idx, comp) in board.components.iter().enumerate() {
        children.push(footprint_node(
            idx,
            comp,
            placements[idx],
            &pin_to_net,
            &net_to_code,
            board,
            project_name,
        ));
    }

    let pcb = Sexpr::list(std::iter::once(Sexpr::atom("kicad_pcb")).chain(children));
    format!("{}\n", pcb)
}

/// Simple row packing: place footprints left-to-right with a gap, wrapping
/// before they cross the board outline.  Positions are footprint origins.
fn auto_place(board: &CompiledBoard, board_width: f64) -> Vec<(f64, f64)> {
    const START_X: f64 = 10.0;
    const START_Y: f64 = 10.0;
    const MARGIN: f64 = 5.0;
    const GAP: f64 = 5.0;

    let max_x = board_width - MARGIN;

    let mut placements = Vec::with_capacity(board.components.len());
    let mut cursor_x = START_X;
    let mut cursor_y = START_Y;
    let mut row_height: f64 = 0.0;

    for comp in &board.components {
        let pads = fp_geom::pads_from_component(comp);
        let (w, h, off_x, off_y) = match fp_geom::pads_extent(&pads) {
            Some((x1, y1, x2, y2)) => (
                x2 - x1 + 1.0,
                y2 - y1 + 1.0,
                // Offset so the extent's top-left lands at the cursor.
                -x1 + 0.5,
                -y1 + 0.5,
            ),
            None => (5.0, 5.0, 2.5, 2.5),
        };

        if cursor_x + w > max_x && cursor_x > START_X {
            cursor_x = START_X;
            cursor_y += row_height + GAP;
            row_height = 0.0;
        }

        placements.push((cursor_x + off_x, cursor_y + off_y));
        cursor_x += w + GAP;
        row_height = row_height.max(h);
    }

    placements
}

fn board_outline(width: f64, height: f64) -> Vec<Sexpr> {
    let rect = [
        ((0.0, 0.0), (width, 0.0), "top"),
        ((width, 0.0), (width, height), "right"),
        ((width, height), (0.0, height), "bottom"),
        ((0.0, height), (0.0, 0.0), "left"),
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
                    Sexpr::str(deterministic_id(&format!("pcb:outline:{}", side))),
                ]),
            ])
        })
        .collect()
}

fn footprint_node(
    idx: usize,
    comp: &copperleaf::CompiledComponent,
    at: (f64, f64),
    pin_to_net: &HashMap<(usize, &str), NetIdx>,
    net_to_code: &HashMap<usize, usize>,
    board: &CompiledBoard,
    project_name: &str,
) -> Sexpr {
    let (pads, pin_indices) = fp_geom::pads_from_component_with_indices(comp);
    let extent = fp_geom::pads_extent(&pads);

    let fp_uuid = deterministic_id(&format!("pcb:{}", comp.refdes));
    let fp_name = footprint_ref(comp);
    let seed = format!("pcb:{}", comp.refdes);

    // Text positions relative to the footprint origin.
    let (ref_y, val_y) = match extent {
        Some((x1, y1, _, y2)) => {
            let _ = x1;
            (y1 - 1.52, y2 + 1.52)
        }
        None => (-2.54, 2.54),
    };

    let mut children = vec![
        Sexpr::atom("footprint"),
        Sexpr::str(&fp_name),
        Sexpr::list([Sexpr::atom("layer"), Sexpr::str("F.Cu")]),
        Sexpr::list([Sexpr::atom("locked"), Sexpr::atom("no")]),
        Sexpr::list([Sexpr::atom("uuid"), Sexpr::str(&fp_uuid)]),
        Sexpr::list([
            Sexpr::atom("at"),
            Sexpr::atom(format_grid_float(at.0)),
            Sexpr::atom(format_grid_float(at.1)),
            Sexpr::atom("0"),
        ]),
        // Properties (KiCad 9+ stores Reference/Value as properties).
        // The Reference property is visible on F.SilkS; the Value property
        // is hidden here and re-emitted as fp_text user on F.Fab so it
        // doesn't conflict with the Reference on the same layer.
        footprint_property("Reference", &comp.refdes, 0.0, ref_y, false),
        footprint_property(
            "Value",
            &crate::common::refdes_prefix(&comp.refdes),
            0.0,
            val_y,
            true,
        ),
        // Visible value text on F.Fab using a KiCad variable.
        fp_geom::fp_text("user", "${VALUE}", (0.0, val_y), "F.Fab"),
        // Path linkage to schematic symbol.
        Sexpr::list([Sexpr::atom("path"), Sexpr::str(format!("/{}", fp_uuid))]),
        Sexpr::list([Sexpr::atom("sheetname"), Sexpr::str("/")]),
        Sexpr::list([
            Sexpr::atom("sheetfile"),
            Sexpr::str(format!("{}.kicad_sch", project_name)),
        ]),
        Sexpr::list([
            Sexpr::atom("attr"),
            Sexpr::atom(fp_geom::footprint_attr(&pads)),
        ]),
    ];

    // Outlines (fab, silk, courtyard, pin-1 marker).
    if let Some(ext) = extent {
        for node in fp_geom::outline_sexprs(ext, fp_geom::pin1_pos(&pads), Some(&seed)) {
            children.push(node);
        }
    }

    // Pads with net associations.
    for (pad, pin_index) in pads.iter().zip(pin_indices.iter()) {
        let pad_uuid = deterministic_id(&format!("{}:pad:{}", seed, pad.number));
        let net = pin_index.and_then(|i| {
            let pin = &comp.pins[i];
            pin_to_net.get(&(idx, pin.name())).and_then(|&net_idx| {
                net_to_code
                    .get(&net_idx.0)
                    .map(|&code| (code, board.nets[net_idx.0].name.as_str()))
            })
        });
        children.push(fp_geom::pad_sexpr(pad, Some(&pad_uuid), net));
    }

    // 3D model reference (KLC F9.3; missing files are ignored by KiCad).
    // Use just the filename so it resolves relative to the project directory
    // (the file is copied alongside the project output during emit()).
    let model_path_for_pcb = match comp.meta.model_3d {
        Some(ref path) => Path::new(path)
            .file_name()
            .map(|s| s.to_str().unwrap().to_owned()),
        None if comp.meta.model_3d_data.is_some() => Some(format!("{}.step", comp.refdes)),
        None => None,
    };
    children.push(fp_geom::model_sexpr(
        &fp_name,
        model_path_for_pcb.as_deref(),
        comp.meta.model_3d_offset,
        comp.meta.model_3d_rotation,
    ));

    Sexpr::list(children)
}

/// Hidden footprint property node for KiCad 9+ metadata (Reference, Value, etc.).
fn footprint_property(name: &str, value: &str, x: f64, y: f64, hide: bool) -> Sexpr {
    let prop_uuid = deterministic_id(&format!("pcb:prop:{}:{}", name, value));
    let mut prop = vec![
        Sexpr::atom("property"),
        Sexpr::str(name),
        Sexpr::str(value),
        Sexpr::list([
            Sexpr::atom("at"),
            Sexpr::atom(format_float(x, 2)),
            Sexpr::atom(format_float(y, 2)),
            Sexpr::atom("0"),
        ]),
        Sexpr::list([Sexpr::atom("layer"), Sexpr::str("F.SilkS")]),
    ];
    if hide {
        prop.push(Sexpr::list([Sexpr::atom("hide"), Sexpr::atom("yes")]));
    }
    prop.push(Sexpr::list([Sexpr::atom("uuid"), Sexpr::str(&prop_uuid)]));
    prop.push(Sexpr::list([
        Sexpr::atom("effects"),
        Sexpr::list([
            Sexpr::atom("font"),
            Sexpr::list([Sexpr::atom("size"), Sexpr::atom("1.0"), Sexpr::atom("1.0")]),
            Sexpr::list([Sexpr::atom("thickness"), Sexpr::atom("0.15")]),
        ]),
        Sexpr::list([Sexpr::atom("justify"), Sexpr::atom("left")]),
    ]));
    Sexpr::list(prop)
}

fn general_node() -> Sexpr {
    Sexpr::list([
        Sexpr::atom("general"),
        Sexpr::list([Sexpr::atom("thickness"), Sexpr::atom("1.6")]),
        Sexpr::list([Sexpr::atom("legacy_teardrops"), Sexpr::atom("no")]),
    ])
}

/// Layer table using KiCad's canonical (fixed) layer IDs.
fn layers_node() -> Sexpr {
    Sexpr::list([
        Sexpr::atom("layers"),
        Sexpr::list([Sexpr::atom("0"), Sexpr::str("F.Cu"), Sexpr::atom("signal")]),
        Sexpr::list([Sexpr::atom("31"), Sexpr::str("B.Cu"), Sexpr::atom("signal")]),
        Sexpr::list([
            Sexpr::atom("32"),
            Sexpr::str("B.Adhes"),
            Sexpr::atom("user"),
        ]),
        Sexpr::list([
            Sexpr::atom("33"),
            Sexpr::str("F.Adhes"),
            Sexpr::atom("user"),
        ]),
        Sexpr::list([
            Sexpr::atom("34"),
            Sexpr::str("B.Paste"),
            Sexpr::atom("user"),
        ]),
        Sexpr::list([
            Sexpr::atom("35"),
            Sexpr::str("F.Paste"),
            Sexpr::atom("user"),
        ]),
        Sexpr::list([
            Sexpr::atom("36"),
            Sexpr::str("B.SilkS"),
            Sexpr::atom("user"),
        ]),
        Sexpr::list([
            Sexpr::atom("37"),
            Sexpr::str("F.SilkS"),
            Sexpr::atom("user"),
        ]),
        Sexpr::list([Sexpr::atom("38"), Sexpr::str("B.Mask"), Sexpr::atom("user")]),
        Sexpr::list([Sexpr::atom("39"), Sexpr::str("F.Mask"), Sexpr::atom("user")]),
        Sexpr::list([
            Sexpr::atom("44"),
            Sexpr::str("Edge.Cuts"),
            Sexpr::atom("user"),
        ]),
        Sexpr::list([Sexpr::atom("45"), Sexpr::str("Margin"), Sexpr::atom("user")]),
        Sexpr::list([
            Sexpr::atom("46"),
            Sexpr::str("B.CrtYd"),
            Sexpr::atom("user"),
        ]),
        Sexpr::list([
            Sexpr::atom("47"),
            Sexpr::str("F.CrtYd"),
            Sexpr::atom("user"),
        ]),
        Sexpr::list([Sexpr::atom("48"), Sexpr::str("B.Fab"), Sexpr::atom("user")]),
        Sexpr::list([Sexpr::atom("49"), Sexpr::str("F.Fab"), Sexpr::atom("user")]),
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

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf::{
        CompiledComponent, ComponentMeta, Connection, Net, NetClass, NetIdx, NetKind, Pin, UnitExt,
    };

    fn test_board() -> CompiledBoard {
        CompiledBoard {
            components: vec![CompiledComponent {
                refdes: "U1".into(),
                meta: ComponentMeta::default(),
                pins: vec![
                    Pin::build("VDD")
                        .number("1")
                        .pos(-1.0, 0.0)
                        .width(0.6)
                        .height(1.2)
                        .pad_type("smd")
                        .pwr_fixed(3.3.volt(), 0.1.amp())
                        .pin(),
                    Pin::build("GND")
                        .number("2")
                        .pos(1.0, 0.0)
                        .width(0.6)
                        .height(1.2)
                        .pad_type("smd")
                        .gnd(),
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
    fn pcb_starts_with_kicad_pcb() {
        let out = emit_pcb(&test_board(), "test");
        assert!(out.starts_with("(kicad_pcb"));
        assert!(out.contains("(net_class \"Default\""));
        assert!(out.contains("(footprint"));
    }

    #[test]
    fn pcb_embeds_real_pads() {
        let out = emit_pcb(&test_board(), "test");
        // SMD pad with the declared geometry, not a generic through-hole.
        assert!(out.contains("(pad \"1\" smd rect"), "{}", out);
        assert!(out.contains("(at -1 0)"), "{}", out);
        assert!(out.contains("(size 0.6 1.2)"), "{}", out);
        assert!(out.contains("(attr smd)"), "{}", out);
        // Net attached to pad 1.
        assert!(out.contains("(net 1 \"V3V3\")"), "{}", out);
        // Project-local footprint reference.
        assert!(out.contains("(footprint \"copperleaf:U1\""), "{}", out);
    }

    #[test]
    fn pcb_uses_canonical_layer_ids() {
        let out = emit_pcb(&test_board(), "test");
        assert!(out.contains("(31 \"B.Cu\" signal)"), "{}", out);
        assert!(out.contains("(44 \"Edge.Cuts\" user)"), "{}", out);
        assert!(out.contains("(47 \"F.CrtYd\" user)"), "{}", out);
    }
}
