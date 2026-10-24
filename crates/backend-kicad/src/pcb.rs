//! KiCad PCB emitter.

use std::{collections::HashMap, path::Path};

use copperleaf::{
    BoardSide, CompiledBoard, CopperLayer, Layout, NetClass, NetIdx, Placement, Stackup,
    StackupLayer,
};

use crate::{
    common::{build_net_codes, fmt_mm, footprint_ref, format_float, format_grid_float},
    deterministic_id, fp_geom,
    sexpr::{Sexpr, kv},
};

pub type NetRemapping = (
    HashMap<String, (usize, String)>,
    HashMap<usize, (usize, String)>,
);

/// Emit a KiCad S-expression PCB file for the given compiled board.
pub fn emit_pcb(board: &CompiledBoard, project_name: &str) -> String {
    let net_codes = build_net_codes(board);
    let net_to_code: HashMap<usize, usize> = net_codes
        .iter()
        .enumerate()
        .map(|(idx, (_, code))| (idx, *code))
        .collect();

    let pin_to_net: HashMap<(usize, usize), NetIdx> = board
        .connections
        .iter()
        .map(|c| ((c.component, c.pin_index), c.net))
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

    let pin_to_net: HashMap<(usize, usize), NetIdx> = board
        .connections
        .iter()
        .map(|c| ((c.component, c.pin_index), c.net))
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
) -> NetRemapping {
    // New net name → (new KiCad code, new net name).
    let name_to_new: HashMap<&str, (usize, &str)> = board
        .nets
        .iter()
        .filter_map(|net| {
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
    for (track_ordinal, track) in layout.tracks.iter().enumerate() {
        let Some(&net_code) = net_to_code.get(&track.net.0) else {
            continue;
        };
        let layer = normalised_layer_name(track.layer, layer_t);
        let width = track.width.as_base() * 1000.0; // m → mm
        let path: Vec<(f64, f64)> = track.path.clone();

        if path.len() < 2 {
            continue;
        }

        for (segment_ordinal, w) in path.windows(2).enumerate() {
            let seg_uuid = deterministic_id(&format!(
                "pcb:segment:{}:{}:{}:{}:{:.6}:{:.6}:{:.6}:{:.6}",
                track.net.0,
                track.layer.0,
                track_ordinal,
                segment_ordinal,
                w[0].0,
                w[0].1,
                w[1].0,
                w[1].1
            ));
            nodes.push(Sexpr::list([
                Sexpr::atom("segment"),
                Sexpr::list([
                    Sexpr::atom("start"),
                    Sexpr::atom(format_float(w[0].0, 6)),
                    Sexpr::atom(format_float(w[0].1, 6)),
                ]),
                Sexpr::list([
                    Sexpr::atom("end"),
                    Sexpr::atom(format_float(w[1].0, 6)),
                    Sexpr::atom(format_float(w[1].1, 6)),
                ]),
                Sexpr::list([Sexpr::atom("width"), Sexpr::atom(format_float(width, 6))]),
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
    for (via_ordinal, via) in layout.vias.iter().enumerate() {
        let Some(&net_code) = net_to_code.get(&via.net.0) else {
            continue;
        };
        let via_uuid = deterministic_id(&format!(
            "pcb:via:{}:{}:{}:{}:{:.6}:{:.6}",
            via.net.0, via.layers.0.0, via.layers.1.0, via_ordinal, via.at.0, via.at.1
        ));
        let diam = via.diameter.as_base() * 1000.0;
        let drill = via.drill.as_base() * 1000.0;
        let layer_start = normalised_layer_name(via.layers.0, layer_t);
        let layer_end = normalised_layer_name(via.layers.1, layer_t);

        nodes.push(Sexpr::list([
            Sexpr::atom("via"),
            Sexpr::list([
                Sexpr::atom("at"),
                Sexpr::atom(format_float(via.at.0, 6)),
                Sexpr::atom(format_float(via.at.1, 6)),
            ]),
            Sexpr::list([Sexpr::atom("size"), Sexpr::atom(format_float(diam, 6))]),
            Sexpr::list([Sexpr::atom("drill"), Sexpr::atom(format_float(drill, 6))]),
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
    for (zone_ordinal, zone) in layout.zones.iter().enumerate() {
        let Some(&net_code) = net_to_code.get(&zone.net.0) else {
            continue;
        };
        let zone_uuid = deterministic_id(&format!(
            "pcb:zone:{}:{}:{}",
            zone.net.0, zone.layer.0, zone_ordinal
        ));
        let layer = normalised_layer_name(zone.layer, layer_t);
        let net_name = &board.nets[zone.net.0].name;

        let mut poly_pts: Vec<Sexpr> = zone
            .outline
            .iter()
            .map(|&(x, y)| {
                Sexpr::list([
                    Sexpr::atom("xy"),
                    Sexpr::atom(format_float(x, 6)),
                    Sexpr::atom(format_float(y, 6)),
                ])
            })
            .collect();

        // Close the polygon.
        if let Some(&first) = zone.outline.first() {
            poly_pts.push(Sexpr::list([
                Sexpr::atom("xy"),
                Sexpr::atom(format_float(first.0, 6)),
                Sexpr::atom(format_float(first.1, 6)),
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
    pin_to_net: &HashMap<(usize, usize), NetIdx>,
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
        footprint_property(
            "Reference",
            &comp.refdes,
            0.0,
            ref_y,
            false,
            &comp.refdes,
            0,
        ),
        footprint_property(
            "Value",
            &crate::common::refdes_prefix(&comp.refdes),
            0.0,
            val_y,
            true,
            &comp.refdes,
            1,
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
    for (pad_ordinal, (pad, pin_index)) in pads.iter().zip(pin_indices.iter()).enumerate() {
        let pad_uuid = deterministic_id(&format!("{}:pad:{pad_ordinal}:{}", seed, pad.number));
        let net = pin_index.and_then(|i| {
            pin_to_net.get(&(idx, i)).and_then(|&net_idx| {
                net_to_code
                    .get(&net_idx.0)
                    .map(|&code| (code, board.nets[net_idx.0].name.as_str()))
            })
        });
        children.push(fp_geom::pad_sexpr(pad, Some(&pad_uuid), net));
    }

    // 3D model reference (KLC F9.3; missing files are ignored by KiCad).
    // The .step files live in a models/ subdirectory next to the project.
    let model_path_for_pcb = if comp
        .meta
        .model_3d_data
        .as_deref()
        .is_some_and(|data| !data.trim().is_empty())
    {
        // Embedded payloads are written next to the project.
        match comp.meta.model_3d.as_deref() {
            Some(path) => Path::new(path)
                .file_name()
                .map(|name| format!("models/{}", name.to_str().unwrap())),
            None => Some(format!("models/{}.step", comp.refdes)),
        }
    } else {
        // A library reference is already a complete model path.
        comp.meta.model_3d.clone()
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
    pin_to_net: &HashMap<(usize, usize), NetIdx>,
    net_to_code: &HashMap<usize, usize>,
    board: &CompiledBoard,
    project_name: &str,
) -> Sexpr {
    let (mut pads, pin_indices) = fp_geom::pads_from_component_with_indices(comp);
    let is_back = placement.is_some_and(|p| p.side == BoardSide::Back);
    if is_back {
        for pad in &mut pads {
            mirror_pad(pad);
        }
    }
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
        footprint_property(
            "Reference",
            &comp.refdes,
            0.0,
            ref_y,
            false,
            &comp.refdes,
            0,
        ),
        footprint_property(
            "Value",
            &crate::common::refdes_prefix(&comp.refdes),
            0.0,
            val_y,
            true,
            &comp.refdes,
            1,
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

    if is_back {
        for child in &mut children {
            if let Sexpr::List(fields) = child
                && let Some(Sexpr::Atom(head)) = fields.first()
                && matches!(head.as_str(), "property" | "fp_text")
            {
                mirror_footprint_node(child);
            }
        }
    }

    if let Some(ext) = extent {
        for mut node in fp_geom::outline_sexprs(
            ext,
            fp_geom::pin1_pos(&pads),
            Some(&seed),
            comp.meta.fab_extent,
        ) {
            if is_back {
                mirror_footprint_node(&mut node);
            }
            children.push(node);
        }
    }

    for (pad_ordinal, (pad, pin_index)) in pads.iter().zip(pin_indices.iter()).enumerate() {
        let pad_uuid = deterministic_id(&format!("{}:pad:{pad_ordinal}:{}", seed, pad.number));
        let net = pin_index.and_then(|i| {
            pin_to_net.get(&(idx, i)).and_then(|&net_idx| {
                net_to_code
                    .get(&net_idx.0)
                    .map(|&code| (code, board.nets[net_idx.0].name.as_str()))
            })
        });
        children.push(fp_geom::pad_sexpr(pad, Some(&pad_uuid), net));
    }

    let model_path_for_pcb = if comp
        .meta
        .model_3d_data
        .as_deref()
        .is_some_and(|data| !data.trim().is_empty())
    {
        // Embedded payloads are written next to the project.
        match comp.meta.model_3d.as_deref() {
            Some(path) => Path::new(path)
                .file_name()
                .map(|name| format!("models/{}", name.to_str().unwrap())),
            None => Some(format!("models/{}.step", comp.refdes)),
        }
    } else {
        // A library reference is already a complete model path.
        comp.meta.model_3d.clone()
    };
    let mut model = fp_geom::model_sexpr(
        &fp_name,
        model_path_for_pcb.as_deref(),
        comp.meta.model_3d_offset,
        comp.meta.model_3d_rotation,
    );
    if is_back {
        mirror_footprint_node(&mut model);
    }
    children.push(model);

    Sexpr::list(children)
}

/// Hidden footprint property node for KiCad 9+ metadata (Reference, Value, etc.).
fn footprint_property(
    name: &str,
    value: &str,
    x: f64,
    y: f64,
    hide: bool,
    owner: &str,
    ordinal: usize,
) -> Sexpr {
    let prop_uuid = deterministic_id(&format!("pcb:prop:{owner}:{name}:{value}:{ordinal}"));
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

fn mirror_coordinate(parts: &mut [Sexpr], index: usize) {
    if let Some(atom) = parts.get_mut(index) {
        negate_atom(atom);
    }
}

/// Apply the single back-side geometric transform to a footprint child node.
fn mirror_footprint_node(node: &mut Sexpr) {
    let Sexpr::List(parts) = node else {
        return;
    };
    let head = match parts.first() {
        Some(Sexpr::Atom(head)) => head.clone(),
        _ => return,
    };
    match head.as_str() {
        "fp_line" | "fp_rect" | "fp_arc" | "fp_poly" => {
            for part in parts.iter_mut().skip(1) {
                if let Sexpr::List(fields) = part
                    && let Some(Sexpr::Atom(field)) = fields.first()
                    && matches!(field.as_str(), "start" | "end" | "xy")
                {
                    mirror_coordinate(fields, 1);
                }
            }
        }
        "fp_circle" => {
            for part in parts.iter_mut().skip(1) {
                if let Sexpr::List(fields) = part
                    && let Some(Sexpr::Atom(field)) = fields.first()
                    && matches!(field.as_str(), "center" | "end")
                {
                    mirror_coordinate(fields, 1);
                }
            }
        }
        "fp_text" | "property" => {
            for part in parts.iter_mut().skip(1) {
                if let Sexpr::List(fields) = part
                    && let Some(Sexpr::Atom(field)) = fields.first()
                    && field == "at"
                {
                    mirror_coordinate(fields, 1);
                    // Rotate the opposite way on the mirrored side.
                    if fields.len() > 3 {
                        negate_atom(&mut fields[3]);
                    }
                }
            }
            // Back-side text is mirrored so it reads correctly through the board.
            for part in parts.iter_mut().skip(1) {
                if let Sexpr::List(fields) = part
                    && let Some(Sexpr::Atom(field)) = fields.first()
                    && field == "effects"
                {
                    fields.push(Sexpr::list([Sexpr::atom("justify"), Sexpr::atom("mirror")]));
                }
            }
        }
        "model" => {
            for part in parts.iter_mut().skip(1) {
                if let Sexpr::List(fields) = part
                    && let Some(Sexpr::Atom(field)) = fields.first()
                    && field == "offset"
                {
                    for xyz in fields.iter_mut().skip(1) {
                        if let Sexpr::List(values) = xyz
                            && let Some(Sexpr::Atom(kind)) = values.first()
                            && kind == "xyz"
                        {
                            mirror_coordinate(values, 1);
                        }
                    }
                } else if let Sexpr::List(fields) = part
                    && let Some(Sexpr::Atom(field)) = fields.first()
                    && field == "rotate"
                {
                    for xyz in fields.iter_mut().skip(1) {
                        if let Sexpr::List(values) = xyz
                            && let Some(Sexpr::Atom(kind)) = values.first()
                            && kind == "xyz"
                        {
                            negate_atom(&mut values[2]);
                            negate_atom(&mut values[3]);
                        }
                    }
                }
            }
        }
        _ => {}
    }
    mirror_layers(node);
}

fn mirror_layer_atom(atom: &mut String) {
    let quoted = atom.starts_with('"') && atom.ends_with('"');
    let mapped = mirrored_layer_name(atom.trim_matches('"'));
    *atom = if quoted {
        format!("\"{mapped}\"")
    } else {
        mapped.to_string()
    };
}

fn mirror_layers(node: &mut Sexpr) {
    let Sexpr::List(parts) = node else {
        return;
    };
    let head = match parts.first() {
        Some(Sexpr::Atom(head)) => head.clone(),
        _ => return,
    };
    match head.as_str() {
        "layer" => {
            if let Some(Sexpr::Atom(layer)) = parts.get_mut(1) {
                mirror_layer_atom(layer);
            }
        }
        "layers" => {
            for part in parts.iter_mut().skip(1) {
                if let Sexpr::Atom(layer) = part {
                    mirror_layer_atom(layer);
                }
            }
        }
        _ => {
            for part in parts.iter_mut().skip(1) {
                mirror_layers(part);
            }
        }
    }
}

/// Mirror a pad across the footprint Y axis for back-side emission.
fn mirror_pad(pad: &mut copperleaf::Pad) {
    pad.pos.0 = -pad.pos.0;
    pad.rotation = -pad.rotation;
    if let Some(layers) = &pad.layers {
        pad.layers = Some(mirrored_layer_field(layers));
    }
}

fn mirrored_layer_field(field: &str) -> String {
    field
        .split_whitespace()
        .map(mirrored_layer_name)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Map one footprint-side layer name to its mirror.
fn mirrored_layer_name(name: &str) -> &str {
    match name {
        "F.Cu" => "B.Cu",
        "B.Cu" => "F.Cu",
        "F.Mask" => "B.Mask",
        "B.Mask" => "F.Mask",
        "F.Paste" => "B.Paste",
        "B.Paste" => "F.Paste",
        "F.SilkS" => "B.SilkS",
        "B.SilkS" => "F.SilkS",
        "F.Fab" => "B.Fab",
        "B.Fab" => "F.Fab",
        "F.CrtYd" => "B.CrtYd",
        "B.CrtYd" => "F.CrtYd",
        // Wildcards such as `*.Cu` and `*.Mask` are side-independent.
        other => other,
    }
}

fn negate_atom(atom: &mut Sexpr) {
    if let Sexpr::Atom(text) = atom
        && let Ok(value) = text.parse::<f64>()
    {
        let mirrored = if value == 0.0 { 0.0 } else { -value };
        *text = format_float(mirrored, 4);
    }
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

fn normalised_layer_name(layer: CopperLayer, total: usize) -> String {
    match layer.0 {
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
        // Within a pad, find and update the (net ...) node. A net that no
        // longer exists is removed entirely so a reused numeric code cannot
        // alias old copper onto a different net.
        let mut clear_net = false;
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
            match remap_net_node(net_props, old_name_to_new, old_code_to_new) {
                Some(updated) => *net_props = updated,
                None => clear_net = true,
            }
        }
        if clear_net {
            props.retain(|prop| {
                !matches!(prop, Sexpr::List(fields) if fields.first().is_some_and(|head| head.as_string() == "net"))
            });
        }
    }
}

/// Update a single `(net ...)` node inside a pad.
///
/// Returns `Some(updated)` when the old net maps to a net in the new board,
/// and `None` when the old net no longer exists.
fn remap_net_node(
    props: &[Sexpr],
    old_name_to_new: &HashMap<String, (usize, String)>,
    old_code_to_new: &HashMap<usize, (usize, String)>,
) -> Option<Vec<Sexpr>> {
    let s = props[1].as_string();
    let mapped = if let Ok(old_code) = s.parse::<usize>() {
        old_code_to_new.get(&old_code).cloned()
    } else {
        old_name_to_new.get(&s).cloned()
    };
    mapped.map(|(new_code, new_name)| {
        vec![
            Sexpr::atom("net"),
            Sexpr::atom(new_code.to_string()),
            Sexpr::str(new_name),
        ]
    })
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

    // Number copper layers by their copper ordinal (0 = F.Cu, 1 = In1.Cu, …)
    // so inner layers line up with the `(layers …)` table, which uses the
    // same numbering.  Using the raw position in `stackup.layers` would
    // include dielectrics in the count (`In2.Cu`, `In4.Cu` for 4-layer).
    let mut copper_counter = 0usize;
    let copper_total = stackup.copper_layer_count();
    for layer in stackup.layers.iter() {
        match layer {
            StackupLayer::Copper { thickness_mm, .. } => {
                let name = normalised_layer_name(CopperLayer::new(copper_counter), copper_total);
                copper_counter += 1;
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
                pin: copperleaf::PinId("VDD".into()),
                pin_index: 0,
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

    #[test]
    fn four_layer_stackup_uses_copper_ordinals() {
        let mut board = test_board();
        board.stackup = copperleaf::Stackup::four_layer();
        let out = emit_pcb(&board, "test");
        // Inner coppers must be numbered by copper ordinal, matching the
        // `(layers …)` table (In1.Cu, In2.Cu) — not by raw position in the
        // layer list, which would produce In2.Cu/In4.Cu.
        assert!(out.contains("(layer \"In1.Cu\""), "{}", out);
        assert!(out.contains("(layer \"In2.Cu\""), "{}", out);
        assert!(!out.contains("\"In4.Cu\""), "{}", out);
        // Dielectrics keep their own sequential numbering.
        assert!(out.contains("(layer \"dielectric 1\""), "{}", out);
        assert!(out.contains("(layer \"dielectric 2\""), "{}", out);
        assert!(out.contains("(layer \"dielectric 3\""), "{}", out);
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
                layer: copperleaf::CopperLayer(0), // F.Cu
                width: 0.25.mm(),
                path: vec![(5.0, 5.0), (15.0, 5.0)],
            }],
            vias: vec![Via {
                net: NetIdx(0),
                at: (15.0, 5.0),
                drill: 0.4.mm(),
                diameter: 0.8.mm(),
                layers: (copperleaf::CopperLayer(0), copperleaf::CopperLayer(1)), // F.Cu to B.Cu
            }],
            zones: vec![Zone {
                net: NetIdx(0),
                layer: copperleaf::CopperLayer(1), // B.Cu
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

    #[test]
    fn copper_geometry_keeps_six_decimal_millimetre_precision() {
        let board = test_board();
        let mut layout = test_layout();
        layout.tracks[0].path = vec![(1.234567, 2.345678), (3.456789, 4.567891)];
        layout.tracks[0].width = 0.123456.mm();
        let out = emit_pcb_with_layout(&board, "test", Some(&layout), None, None, &HashMap::new());
        assert!(out.contains("(start 1.234567 2.345678)"), "{out}");
        assert!(out.contains("(end 3.456789 4.567891)"), "{out}");
        assert!(out.contains("(width 0.123456)"), "{out}");
    }

    #[test]
    fn back_side_footprint_uses_only_back_side_layers() {
        use copperleaf::BoardSide;

        let board = test_board();
        let mut layout = test_layout();
        layout.placements[0].side = BoardSide::Back;
        let out = emit_pcb_with_layout(&board, "test", Some(&layout), None, None, &HashMap::new());
        let footprint = out
            .split_once("(footprint")
            .map(|(_, rest)| rest)
            .and_then(|rest| rest.split("\n  (segment").next())
            .expect("footprint emitted");

        assert!(footprint.contains("(layer \"B.Cu\")"), "{footprint}");
        assert!(
            footprint.contains("(layers B.Cu B.Mask B.Paste)"),
            "back-side pad layers missing: {footprint}"
        );
        assert!(footprint.contains("B.SilkS"), "{footprint}");
        assert!(footprint.contains("B.Fab"), "{footprint}");
        assert!(footprint.contains("B.CrtYd"), "{footprint}");
        assert!(footprint.contains("(justify mirror)"), "{footprint}");
        for front in ["F.Cu", "F.Mask", "F.Paste", "F.SilkS", "F.Fab", "F.CrtYd"] {
            assert!(
                !footprint.contains(front),
                "front-side layer {front} survived in a back-side footprint:\n{footprint}"
            );
        }
    }

    fn preserved_footprint(net_code: usize, net_name: &str) -> Sexpr {
        crate::sexpr::parse(&format!(
            "(footprint \"X\" (layer \"F.Cu\") (property \"Reference\" \"U1\") (pad \"1\" smd rect (at 0 0) (size 1 1) (layers F.Cu) (net {net_code} \"{net_name}\")))"
        ))
        .unwrap()
    }

    fn emitted_preserved_pad(board: &CompiledBoard, old_name: &str, old_code: usize) -> String {
        let layout = test_layout();
        let fp = preserved_footprint(old_code, old_name);
        let mut old_names = HashMap::new();
        old_names.insert(old_code, old_name.to_string());
        let out = emit_pcb_with_layout(board, "test", Some(&layout), None, Some(&[fp]), &old_names);
        out.split_once("(footprint \"X\"")
            .map(|(_, rest)| rest)
            .and_then(|rest| rest.split("(segment").next())
            .expect("preserved footprint")
            .to_string()
    }

    #[test]
    fn deleted_net_is_cleared_from_preserved_pad() {
        let block = emitted_preserved_pad(&test_board(), "OLD_NET", 5);
        assert!(
            !block.contains("(net"),
            "deleted net survived on a preserved pad: {block}"
        );
    }

    #[test]
    fn reused_net_code_does_not_alias_old_copper() {
        // Old code 1 was OLD_NET; the new board assigns code 1 to V3V3.
        let block = emitted_preserved_pad(&test_board(), "OLD_NET", 1);
        assert!(
            !block.contains("(net"),
            "reused net code aliased old copper: {block}"
        );
        assert!(!block.contains("V3V3"), "{block}");
    }

    #[test]
    fn renamed_net_preserves_preserved_pad_association() {
        // The old code maps by name, so the pad survives with the new code.
        let block = emitted_preserved_pad(&test_board(), "V3V3", 7);
        assert!(block.contains("(net 1 \"V3V3\")"), "{block}");
    }

    #[test]
    fn canonical_ordinals_map_to_kicad_copper_layers() {
        // Two-layer: ordinal 1 is the back, and no inner layer exists.
        assert_eq!(normalised_layer_name(CopperLayer(0), 2), "F.Cu");
        assert_eq!(normalised_layer_name(CopperLayer(1), 2), "B.Cu");

        // Four-layer: ordinal 1 and 2 are inner copper, 3 is the back.
        assert_eq!(normalised_layer_name(CopperLayer(0), 4), "F.Cu");
        assert_eq!(normalised_layer_name(CopperLayer(1), 4), "In1.Cu");
        assert_eq!(normalised_layer_name(CopperLayer(2), 4), "In2.Cu");
        assert_eq!(normalised_layer_name(CopperLayer(3), 4), "B.Cu");
    }
}
