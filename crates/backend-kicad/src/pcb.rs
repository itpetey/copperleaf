//! KiCad PCB emitter.

use std::{collections::HashMap, path::Path};

use copperleaf::{
    BoardSide, CompiledBoard, Layout, NetClass, NetIdx, Placement, Stackup, StackupLayer,
};

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
        general_node(&board.stackup),
        kv("paper", "A4"),
        layers_node(&board.stackup),
        setup_node(&board.stackup),
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

/// Emit a KiCad PCB file with an optional solved layout.
///
/// When `layout` is `Some`, placements, rotations, and board side come from
/// the layout instead of auto-placement, and tracks, vias, and zones are
/// emitted as copper elements.  When `None`, behaviour is identical to
/// [`emit_pcb`].
///
/// If `preserved_graphics` is provided, those raw S-expression nodes replace
/// the generated board outline — used by [`crate::KiCad::emit_update`] so that
/// manual board-outline adjustments survive schema changes.
///
/// If `preserved_footprints` and `old_net_names` are provided, footprint
/// S-expressions from the original PCB are re-emitted verbatim with only
/// the net references in pads remapped to the new board.  This preserves
/// footprint-level details (3D models, stroke formatting, external library
/// references, etc.) that copperleaf's emitter does not produce.
pub fn emit_pcb_with_layout(
    board: &CompiledBoard,
    project_name: &str,
    layout: Option<&Layout>,
    preserved_graphics: Option<&[Sexpr]>,
    preserved_footprints: Option<&[Sexpr]>,
    old_net_names: &HashMap<usize, String>,
) -> String {
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
        general_node(&board.stackup),
        kv("paper", "A4"),
        layers_node(&board.stackup),
        setup_node(&board.stackup),
    ];

    for (name, code) in &net_codes {
        children.push(Sexpr::list([
            Sexpr::atom("net"),
            Sexpr::atom(code.to_string()),
            Sexpr::str(name),
        ]));
    }

    children.extend(net_class_nodes(board, &net_codes));

    if let Some(graphics) = preserved_graphics {
        children.extend(graphics.iter().cloned());
    } else {
        children.extend(board_outline(board.width, board.height));
    }

    // Placements: from layout if available, otherwise auto-place.
    if let Some(layout) = layout {
        // Build a lookup from component index to placement.
        let placement_by_component: HashMap<usize, &copperleaf::Placement> =
            layout.placements.iter().map(|p| (p.component, p)).collect();

        // Auto-place positions as fallback for new components without an
        // existing placement.
        let fallback_positions = auto_place(board, board.width);
        let fallback_placements: Vec<Placement> = fallback_positions
            .iter()
            .enumerate()
            .map(|(idx, &at)| Placement {
                component: idx,
                at,
                rotation: 0.0,
                side: BoardSide::Front,
            })
            .collect();

        // If preserved footprints are available, build net-remapping
        // tables and a refdes→footprint lookup.
        let (old_name_to_new, old_code_to_new, refdes_to_fp) =
            if let Some(fps) = preserved_footprints {
                let (on, oc) = build_net_remap(old_net_names, board, &net_codes);
                let r2f: HashMap<String, &Sexpr> = fps
                    .iter()
                    .filter_map(|fp| footprint_refdes(fp).map(|r| (r, fp)))
                    .collect();
                (Some(on), Some(oc), Some(r2f))
            } else {
                (None, None, None)
            };

        for (idx, comp) in board.components.iter().enumerate() {
            // Use a preserved footprint when available; remap its pad nets.
            if let Some(fp) = refdes_to_fp
                .as_ref()
                .and_then(|r2f| r2f.get(comp.refdes.as_str()))
            {
                let mut fp_clone = (*fp).clone();
                remap_footprint_nets(
                    &mut fp_clone,
                    old_name_to_new.as_ref().unwrap(),
                    old_code_to_new.as_ref().unwrap(),
                );
                children.push(fp_clone);
            } else {
                let placement = placement_by_component
                    .get(&idx)
                    .copied()
                    .or_else(|| Some(&fallback_placements[idx]));
                children.push(footprint_node_with_placement(
                    idx,
                    comp,
                    placement,
                    &pin_to_net,
                    &net_to_code,
                    board,
                    project_name,
                ));
            }
        }

        // Emit copper: tracks, vias, zones.
        children.extend(emit_tracks(layout, &net_to_code, board));
        children.extend(emit_vias(layout, &net_to_code, board));
        children.extend(emit_zones(layout, &net_to_code, board));
    } else {
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

/// Build old→new net remapping tables from the parsed old PCB's net name
/// table and the new compiled board.
///
/// Returns:
/// - `old_name_to_new`: old net *name* → (new KiCad code, new net name)
/// - `old_code_to_new`: old net *code* → (new KiCad code, new net name)
fn build_net_remap(
    old_net_names: &HashMap<usize, String>,
    board: &CompiledBoard,
    net_codes: &[(String, usize)],
) -> (
    HashMap<String, (usize, String)>,
    HashMap<usize, (usize, String)>,
) {
    // New net name → (new KiCad code, new net name).
    let name_to_new: HashMap<&str, (usize, &str)> = board
        .nets
        .iter()
        .enumerate()
        .filter_map(|(_idx, net)| {
            let (_, code) = net_codes.iter().find(|(name, _)| *name == net.name)?;
            Some((net.name.as_str(), (*code, net.name.as_str())))
        })
        .collect();

    let mut name_map: HashMap<String, (usize, String)> = HashMap::new();
    let mut code_map: HashMap<usize, (usize, String)> = HashMap::new();

    // Populate from old net code → name → new code mapping.
    for (&old_code, old_name) in old_net_names {
        if let Some(&(new_code, new_name)) = name_to_new.get(old_name.as_str()) {
            let entry = (new_code, new_name.to_string());
            name_map
                .entry(old_name.clone())
                .or_insert_with(|| entry.clone());
            code_map.insert(old_code, entry);
        }
    }

    // Also populate name_map from every net name in the new board, so that
    // old PCBs that reference nets by name (without top-level net-code
    // declarations) still get their pad nets remapped.
    for (&name, &(code, new_name)) in &name_to_new {
        name_map
            .entry(name.to_string())
            .or_insert_with(|| (code, new_name.to_string()));
    }

    (name_map, code_map)
}

fn emit_tracks(
    layout: &Layout,
    net_to_code: &HashMap<usize, usize>,
    board: &CompiledBoard,
) -> Vec<Sexpr> {
    let layer_t = board.stackup.copper_layer_count();
    let mut nodes = Vec::new();
    for track in &layout.tracks {
        let Some(&net_code) = net_to_code.get(&track.net.0) else {
            continue;
        };
        let layer = normalised_layer_name(track.layer, layer_t);
        let width = track.width.as_base() * 1000.0; // m → mm
        let path: Vec<(f64, f64)> = track.path.clone();

        if path.len() < 2 {
            continue;
        }

        for w in path.windows(2) {
            let seg_uuid = deterministic_id(&format!(
                "pcb:segment:{}:{}:{:.3}:{:.3}",
                track.net.0, track.layer, w[0].0, w[0].1
            ));
            nodes.push(Sexpr::list([
                Sexpr::atom("segment"),
                Sexpr::list([
                    Sexpr::atom("start"),
                    Sexpr::atom(format_grid_float(w[0].0)),
                    Sexpr::atom(format_grid_float(w[0].1)),
                ]),
                Sexpr::list([
                    Sexpr::atom("end"),
                    Sexpr::atom(format_grid_float(w[1].0)),
                    Sexpr::atom(format_grid_float(w[1].1)),
                ]),
                Sexpr::list([Sexpr::atom("width"), Sexpr::atom(format_grid_float(width))]),
                Sexpr::list([Sexpr::atom("layer"), Sexpr::str(&layer)]),
                Sexpr::list([Sexpr::atom("net"), Sexpr::atom(net_code.to_string())]),
                Sexpr::list([Sexpr::atom("uuid"), Sexpr::str(&seg_uuid)]),
            ]));
        }
    }
    nodes
}

fn emit_vias(
    layout: &Layout,
    net_to_code: &HashMap<usize, usize>,
    board: &CompiledBoard,
) -> Vec<Sexpr> {
    let mut nodes = Vec::new();
    let layer_t = board.stackup.copper_layer_count();
    for via in &layout.vias {
        let Some(&net_code) = net_to_code.get(&via.net.0) else {
            continue;
        };
        let via_uuid = deterministic_id(&format!(
            "pcb:via:{}:{}:{:.3}:{:.3}",
            via.net.0, via.layers.0, via.at.0, via.at.1
        ));
        let diam = via.diameter.as_base() * 1000.0;
        let drill = via.drill.as_base() * 1000.0;
        let layer_start = normalised_layer_name(via.layers.0, layer_t);
        let layer_end = normalised_layer_name(via.layers.1, layer_t);

        nodes.push(Sexpr::list([
            Sexpr::atom("via"),
            Sexpr::list([
                Sexpr::atom("at"),
                Sexpr::atom(format_grid_float(via.at.0)),
                Sexpr::atom(format_grid_float(via.at.1)),
            ]),
            Sexpr::list([Sexpr::atom("size"), Sexpr::atom(format_grid_float(diam))]),
            Sexpr::list([Sexpr::atom("drill"), Sexpr::atom(format_grid_float(drill))]),
            Sexpr::list([
                Sexpr::atom("layers"),
                Sexpr::str(layer_start),
                Sexpr::str(layer_end),
            ]),
            Sexpr::list([Sexpr::atom("net"), Sexpr::atom(net_code.to_string())]),
            Sexpr::list([Sexpr::atom("uuid"), Sexpr::str(&via_uuid)]),
        ]));
    }
    nodes
}

fn emit_zones(
    layout: &Layout,
    net_to_code: &HashMap<usize, usize>,
    board: &CompiledBoard,
) -> Vec<Sexpr> {
    let mut nodes = Vec::new();
    let layer_t = board.stackup.copper_layer_count();
    for zone in &layout.zones {
        let Some(&net_code) = net_to_code.get(&zone.net.0) else {
            continue;
        };
        let zone_uuid = deterministic_id(&format!("pcb:zone:{}:{}", zone.net.0, zone.layer));
        let layer = normalised_layer_name(zone.layer, layer_t);
        let net_name = &board.nets[zone.net.0].name;

        let mut poly_pts: Vec<Sexpr> = zone
            .outline
            .iter()
            .map(|&(x, y)| {
                Sexpr::list([
                    Sexpr::atom("xy"),
                    Sexpr::atom(format_grid_float(x)),
                    Sexpr::atom(format_grid_float(y)),
                ])
            })
            .collect();

        // Close the polygon.
        if let Some(&first) = zone.outline.first() {
            poly_pts.push(Sexpr::list([
                Sexpr::atom("xy"),
                Sexpr::atom(format_grid_float(first.0)),
                Sexpr::atom(format_grid_float(first.1)),
            ]));
        }

        nodes.push(Sexpr::list([
            Sexpr::atom("zone"),
            Sexpr::list([Sexpr::atom("net"), Sexpr::atom(net_code.to_string())]),
            Sexpr::list([Sexpr::atom("net_name"), Sexpr::str(net_name)]),
            Sexpr::list([Sexpr::atom("layer"), Sexpr::str(layer)]),
            Sexpr::list([Sexpr::atom("uuid"), Sexpr::str(&zone_uuid)]),
            Sexpr::list([
                Sexpr::atom("polygon"),
                Sexpr::list(std::iter::once(Sexpr::atom("pts")).chain(poly_pts)),
            ]),
        ]));
    }
    nodes
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
            Sexpr::atom(format_float(at.0, 6)),
            Sexpr::atom(format_float(at.1, 6)),
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
        for node in fp_geom::outline_sexprs(
            ext,
            fp_geom::pin1_pos(&pads),
            Some(&seed),
            comp.meta.fab_extent,
        ) {
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
    // The .step files live in a models/ subdirectory next to the project.
    let model_path_for_pcb = match comp.meta.model_3d {
        Some(ref path) => Path::new(path)
            .file_name()
            .map(|s| format!("models/{}", s.to_str().unwrap())),
        None if comp.meta.model_3d_data.is_some() => Some(format!("models/{}.step", comp.refdes)),
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

/// Like [`footprint_node`] but takes an optional [`Placement`] for rotation
/// and board side.  Falls back to auto-placement style when `placement` is
/// `None`.
fn footprint_node_with_placement(
    idx: usize,
    comp: &copperleaf::CompiledComponent,
    placement: Option<&copperleaf::Placement>,
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

    let (ref_y, val_y) = match extent {
        Some((x1, y1, _, y2)) => {
            let _ = x1;
            (y1 - 1.52, y2 + 1.52)
        }
        None => (-2.54, 2.54),
    };

    let (at_x, at_y, rotation, layer) = if let Some(p) = placement {
        (
            p.at.0,
            p.at.1,
            p.rotation,
            match p.side {
                BoardSide::Front => "F.Cu",
                BoardSide::Back => "B.Cu",
            },
        )
    } else {
        // Fallback: auto-place position.  This path is not normally hit when a
        // layout is supplied but keeps the function type-safe for the general case.
        (0.0f64, 0.0f64, 0.0f64, "F.Cu")
    };

    let mut children = vec![
        Sexpr::atom("footprint"),
        Sexpr::str(&fp_name),
        Sexpr::list([Sexpr::atom("layer"), Sexpr::str(layer)]),
        Sexpr::list([Sexpr::atom("locked"), Sexpr::atom("no")]),
        Sexpr::list([Sexpr::atom("uuid"), Sexpr::str(&fp_uuid)]),
        Sexpr::list([
            Sexpr::atom("at"),
            Sexpr::atom(format_float(at_x, 6)),
            Sexpr::atom(format_float(at_y, 6)),
            Sexpr::atom(format_grid_float(rotation)),
        ]),
        footprint_property("Reference", &comp.refdes, 0.0, ref_y, false),
        footprint_property(
            "Value",
            &crate::common::refdes_prefix(&comp.refdes),
            0.0,
            val_y,
            true,
        ),
        fp_geom::fp_text("user", "${VALUE}", (0.0, val_y), "F.Fab"),
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

    if let Some(ext) = extent {
        for node in fp_geom::outline_sexprs(
            ext,
            fp_geom::pin1_pos(&pads),
            Some(&seed),
            comp.meta.fab_extent,
        ) {
            children.push(node);
        }
    }

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

    let model_path_for_pcb = match comp.meta.model_3d {
        Some(ref path) => Path::new(path)
            .file_name()
            .map(|s| format!("models/{}", s.to_str().unwrap())),
        None if comp.meta.model_3d_data.is_some() => Some(format!("models/{}.step", comp.refdes)),
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

/// Extract the reference designator from a footprint S-expression.
fn footprint_refdes(fp: &Sexpr) -> Option<String> {
    let Sexpr::List(parts) = fp else {
        return None;
    };
    for child in &parts[1..] {
        let Sexpr::List(props) = child else {
            continue;
        };
        if props.len() >= 3
            && props[0].as_string() == "property"
            && props[1].as_string() == "Reference"
        {
            return Some(props[2].as_string());
        }
    }
    None
}

fn general_node(stackup: &Stackup) -> Sexpr {
    let thickness = format_float(stackup.total_thickness_mm(), 1);
    Sexpr::list([
        Sexpr::atom("general"),
        Sexpr::list([Sexpr::atom("thickness"), Sexpr::atom(&thickness)]),
        Sexpr::list([Sexpr::atom("legacy_teardrops"), Sexpr::atom("no")]),
    ])
}

/// Layer table using KiCad's canonical (fixed) layer IDs
fn layers_node(stackup: &Stackup) -> Sexpr {
    let mut entries: Vec<Sexpr> = vec![Sexpr::atom("layers")];

    // Copper layers
    entries.push(Sexpr::list([
        Sexpr::atom("0"),
        Sexpr::str("F.Cu"),
        Sexpr::atom("signal"),
    ]));
    for i in 1..stackup.copper_layer_count() - 1 {
        entries.push(Sexpr::list([
            Sexpr::atom(i.to_string()),
            Sexpr::str(format!("In{}.Cu", i)),
            Sexpr::atom("signal"),
        ]));
    }
    entries.push(Sexpr::list([
        Sexpr::atom("31"),
        Sexpr::str("B.Cu"),
        Sexpr::atom("signal"),
    ]));

    // Non-copper layers (canonical fixed IDs).
    for &(id, name) in &[
        (32, "B.Adhes"),
        (33, "F.Adhes"),
        (34, "B.Paste"),
        (35, "F.Paste"),
        (36, "B.SilkS"),
        (37, "F.SilkS"),
        (38, "B.Mask"),
        (39, "F.Mask"),
        (44, "Edge.Cuts"),
        (45, "Margin"),
        (46, "B.CrtYd"),
        (47, "F.CrtYd"),
        (48, "B.Fab"),
        (49, "F.Fab"),
    ] {
        entries.push(Sexpr::list([
            Sexpr::atom(id.to_string()),
            Sexpr::str(name),
            Sexpr::atom("user"),
        ]));
    }

    Sexpr::list(entries)
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

    let rules = &board.design_rules;
    let clearance = fmt_mm(rules.min_clearance / 1000.0);
    let track_width = fmt_mm(rules.min_track_width / 1000.0);

    let mut nodes = vec![net_class_node(
        "Default",
        "",
        &clearance,
        &track_width,
        &default_nets,
    )];
    for ((width, clearance), nets) in groups {
        let name = format!("Power_{}mm_{}mm", width, clearance);
        nodes.push(net_class_node(&name, "", &clearance, &width, &nets));
    }
    nodes
}

fn normalised_layer_name(idx: usize, total: usize) -> String {
    match idx {
        0 => "F.Cu".into(),
        i if i == total - 1 => "B.Cu".into(),
        i => format!("In{i}.Cu"),
    }
}

/// Walk a footprint S-expression and update `(net ...)` references inside
/// pads to use the new board's net codes and names.
fn remap_footprint_nets(
    fp: &mut Sexpr,
    old_name_to_new: &HashMap<String, (usize, String)>,
    old_code_to_new: &HashMap<usize, (usize, String)>,
) {
    let Sexpr::List(fp_parts) = fp else {
        return;
    };
    for child in fp_parts.iter_mut() {
        let Sexpr::List(props) = child else {
            continue;
        };
        if props.is_empty() {
            continue;
        }
        // Look for (pad ...) nodes.
        if props[0].as_string() != "pad" {
            continue;
        }
        // Within a pad, find and update the (net ...) node.
        for prop in props.iter_mut() {
            let Sexpr::List(net_props) = prop else {
                continue;
            };
            if net_props.is_empty() || net_props[0].as_string() != "net" {
                continue;
            }
            if net_props.len() < 2 {
                continue;
            }
            remap_net_node(net_props, old_name_to_new, old_code_to_new);
        }
    }
}

/// Update a single `(net ...)` node inside a pad.
fn remap_net_node(
    props: &mut Vec<Sexpr>,
    old_name_to_new: &HashMap<String, (usize, String)>,
    old_code_to_new: &HashMap<usize, (usize, String)>,
) {
    let s = props[1].as_string();
    // Try numeric code first: (net 1) or (net 1 "NAME")
    if let Ok(old_code) = s.parse::<usize>() {
        if let Some(&(new_code, ref new_name)) = old_code_to_new.get(&old_code) {
            *props = vec![
                Sexpr::atom("net"),
                Sexpr::atom(new_code.to_string()),
                Sexpr::str(new_name),
            ];
        }
    } else if let Some(&(new_code, ref new_name)) = old_name_to_new.get(&s) {
        // Try string name: (net "NAME")
        *props = vec![
            Sexpr::atom("net"),
            Sexpr::atom(new_code.to_string()),
            Sexpr::str(new_name),
        ];
    }
}

fn setup_node(stackup: &Stackup) -> Sexpr {
    Sexpr::list([
        Sexpr::atom("setup"),
        Sexpr::list([Sexpr::atom("pad_to_mask_clearance"), Sexpr::atom("0")]),
        stackup_node(stackup),
        Sexpr::list([
            Sexpr::atom("pcbplotparams"),
            Sexpr::list([
                Sexpr::atom("layerselection"),
                Sexpr::atom("0x00010fc_ffffffff"),
            ]),
        ]),
    ])
}

/// Build a single `(layer …)` entry within the stackup.
#[allow(clippy::too_many_arguments)]
fn stackup_layer(
    name: &str,
    layer_type: &str,
    thickness: Option<&str>,
    material: Option<&str>,
    epsilon_r: Option<&str>,
    loss_tangent: Option<&str>,
    _colour: Option<&str>,
) -> Sexpr {
    let mut children: Vec<Sexpr> = vec![
        Sexpr::atom("layer"),
        Sexpr::str(name),
        Sexpr::list([Sexpr::atom("type"), Sexpr::str(layer_type)]),
    ];
    if let Some(t) = thickness {
        children.push(Sexpr::list([Sexpr::atom("thickness"), Sexpr::atom(t)]));
    }
    if let Some(mat) = material {
        children.push(Sexpr::list([Sexpr::atom("material"), Sexpr::str(mat)]));
    }
    if let Some(dk) = epsilon_r {
        children.push(Sexpr::list([Sexpr::atom("epsilon_r"), Sexpr::atom(dk)]));
    }
    if let Some(df) = loss_tangent {
        children.push(Sexpr::list([Sexpr::atom("loss_tangent"), Sexpr::atom(df)]));
    }
    Sexpr::list(children)
}

/// Emit the `(stackup …)` s-expr inside `setup`.
fn stackup_node(stackup: &Stackup) -> Sexpr {
    let mut entries: Vec<Sexpr> = vec![Sexpr::atom("stackup")];

    // Top mask/silk (always present).
    entries.push(stackup_layer(
        "F.SilkS",
        "Top Silk Screen",
        None,
        None,
        None,
        None,
        None,
    ));
    entries.push(stackup_layer(
        "F.Mask",
        "Top Solder Mask",
        None,
        None,
        None,
        None,
        None,
    ));

    let mut dielectric_counter = 1u32;

    let layer_t = stackup.layers.len();
    for (idx, layer) in stackup.layers.iter().enumerate() {
        match layer {
            StackupLayer::Copper { thickness_mm, .. } => {
                let name = normalised_layer_name(idx, layer_t);
                let thickness_str = format_float(*thickness_mm, 3);
                entries.push(stackup_layer(
                    &name,
                    "copper",
                    Some(&thickness_str),
                    None,
                    None,
                    None,
                    None,
                ));
            }
            StackupLayer::Dielectric {
                kind,
                thickness_mm,
                dielectric,
            } => {
                let thickness_str = format_float(*thickness_mm, 3);
                let dk_str = format_float(dielectric.epsilon_r, 3);
                let df_str = format_float(dielectric.loss_tangent, 3);
                entries.push(stackup_layer(
                    &format!("dielectric {}", dielectric_counter),
                    kind,
                    Some(&thickness_str),
                    Some(&dielectric.material),
                    Some(&dk_str),
                    Some(&df_str),
                    None,
                ));
                dielectric_counter += 1;
            }
        }
    }

    // Bottom mask/silk (always present).
    entries.push(stackup_layer(
        "B.Mask",
        "Bottom Solder Mask",
        None,
        None,
        None,
        None,
        None,
    ));
    entries.push(stackup_layer(
        "B.SilkS",
        "Bottom Silk Screen",
        None,
        None,
        None,
        None,
        None,
    ));

    Sexpr::list(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf::{
        CompiledComponent, ComponentMeta, Connection, Layout, Net, NetClass, NetIdx, NetKind, Pin,
        UnitExt,
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
                layout: vec![],
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
                layout: vec![],
            }],
            connections: vec![Connection {
                component: 0,
                pin: "VDD".into(),
                net: NetIdx(0),
            }],
            constraints: vec![],
            layout: vec![],
            width: 100.0,
            height: 80.0,
            stackup: copperleaf::Stackup::two_layer(),
            design_rules: copperleaf::DesignRules::default(),
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

    // ------------------------------------------------------------------
    // Layout-aware emission tests
    // ------------------------------------------------------------------

    /// Build a minimal layout with one placement, one track, one via, and one zone.
    fn test_layout() -> Layout {
        use copperleaf::{BoardSide, NetIdx, Placement, Track, Via, Zone};
        Layout {
            placements: vec![Placement {
                component: 0,
                at: (10.0, 40.0),
                rotation: 90.0,
                side: BoardSide::Front,
            }],
            tracks: vec![Track {
                net: NetIdx(0),
                layer: 0, // F.Cu
                width: 0.25.mm(),
                path: vec![(5.0, 5.0), (15.0, 5.0)],
            }],
            vias: vec![Via {
                net: NetIdx(0),
                at: (15.0, 5.0),
                drill: 0.4.mm(),
                diameter: 0.8.mm(),
                layers: (0, 1), // F.Cu to B.Cu
            }],
            zones: vec![Zone {
                net: NetIdx(0),
                layer: 1, // B.Cu
                outline: vec![(0.0, 0.0), (100.0, 0.0), (100.0, 80.0), (0.0, 80.0)],
            }],
        }
    }

    #[test]
    fn emit_with_layout_includes_placement_rotation() {
        let board = test_board();
        let layout = test_layout();
        let out = emit_pcb_with_layout(&board, "test", Some(&layout), None, None, &HashMap::new());
        // Placement at (10, 40) with rotation 90°.
        assert!(
            out.contains("(at 10 40 90)"),
            "missing rotated placement: {}",
            out
        );
        // Front-side component should be on F.Cu.
        assert!(out.contains("(layer \"F.Cu\")"), "{}", out);
    }

    #[test]
    fn emit_with_layout_includes_segments() {
        let board = test_board();
        let layout = test_layout();
        let out = emit_pcb_with_layout(&board, "test", Some(&layout), None, None, &HashMap::new());
        assert!(out.contains("(segment"), "missing segment: {}", out);
        assert!(out.contains("(start 5 5)"), "{}", out);
        assert!(out.contains("(end 15 5)"), "{}", out);
        assert!(out.contains("(width 0.25)"), "{}", out);
    }

    #[test]
    fn emit_with_layout_includes_vias() {
        let board = test_board();
        let layout = test_layout();
        let out = emit_pcb_with_layout(&board, "test", Some(&layout), None, None, &HashMap::new());
        assert!(out.contains("(via"), "missing via: {}", out);
        assert!(out.contains("(at 15 5)"), "{}", out);
        assert!(out.contains("(size 0.8)"), "{}", out);
        assert!(out.contains("(drill 0.4)"), "{}", out);
    }

    #[test]
    fn emit_with_layout_includes_zones() {
        let board = test_board();
        let layout = test_layout();
        let out = emit_pcb_with_layout(&board, "test", Some(&layout), None, None, &HashMap::new());
        assert!(out.contains("(zone"), "missing zone: {}", out);
        assert!(out.contains("(layer \"B.Cu\")"), "{}", out);
        assert!(out.contains("(polygon"), "{}", out);
    }

    #[test]
    fn emit_without_layout_is_byte_identical_to_emit_pcb() {
        let board = test_board();
        let out1 = emit_pcb(&board, "test");
        let out2 = emit_pcb_with_layout(&board, "test", None, None, None, &HashMap::new());
        assert_eq!(
            out1, out2,
            "no-layout emit_with_layout must match emit_pcb byte-for-byte"
        );
    }

    #[test]
    fn emit_with_empty_layout_produces_no_copper() {
        let board = test_board();
        let empty = Layout {
            placements: vec![],
            tracks: vec![],
            vias: vec![],
            zones: vec![],
        };
        let out = emit_pcb_with_layout(&board, "test", Some(&empty), None, None, &HashMap::new());
        // No copper elements emitted (no segments, vias, or zones).
        assert!(!out.contains("(segment"));
        assert!(!out.contains("(zone"));
        // "via (" — the copper element, not via_dia/via_drill properties.
        assert!(!out.contains("(via "));
        assert!(!out.contains("(via\n"));
    }
}
