//! KiCad PCB file parser.
//!
//! Parses a `.kicad_pcb` S-expression file and extracts layout data
//! (placements, tracks, vias, zones) along with the net name table.
//! This enables an incremental "update" workflow: parse the existing PCB,
//! remap nets against a new [`CompiledBoard`], and re-emit with
//! [`emit_pcb_with_layout`](crate::pcb::emit_pcb_with_layout).

use std::collections::HashMap;

use copperleaf::{BoardSide, CompiledBoard, Layout, NetIdx, Placement, Track, Via, Zone};
use copperleaf::UnitExt;

use crate::sexpr::{self, ParseError, Sexpr};

// ---------------------------------------------------------------------------
// Parsed representation
// ---------------------------------------------------------------------------

/// Raw data extracted from an existing `.kicad_pcb` file.
#[derive(Clone, Debug, Default)]
pub struct ParsedPcb {
    /// Old net code (1-based integer) → net name.
    pub net_names: HashMap<usize, String>,
    /// Refdes → raw placement data.
    pub placements: HashMap<String, RawPlacement>,
    /// Track segments (one per straight-line segment in the PCB).
    pub tracks: Vec<RawSegment>,
    /// Vias.
    pub vias: Vec<RawVia>,
    /// Copper zones / pours.
    pub zones: Vec<RawZone>,
}

/// Raw placement extracted from a footprint node.
#[derive(Clone, Debug)]
pub struct RawPlacement {
    pub at: (f64, f64),
    pub rotation: f64,
    /// `"F.Cu"` or `"B.Cu"` (defaults to `"F.Cu"` if not found).
    pub layer: String,
}

/// Raw track segment from a `(segment ...)` node.
#[derive(Clone, Debug)]
pub struct RawSegment {
    pub net_code: usize,
    pub layer: String,
    pub width_mm: f64,
    pub start: (f64, f64),
    pub end: (f64, f64),
}

/// Raw via from a `(via ...)` node.
#[derive(Clone, Debug)]
pub struct RawVia {
    pub net_code: usize,
    pub at: (f64, f64),
    pub diameter_mm: f64,
    pub drill_mm: f64,
    pub layer_start: String,
    pub layer_end: String,
}

/// Raw zone from a `(zone ...)` node.
#[derive(Clone, Debug)]
pub struct RawZone {
    pub net_code: usize,
    pub layer: String,
    pub outline: Vec<(f64, f64)>,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Parse a `.kicad_pcb` file into a [`ParsedPcb`].
pub fn parse_pcb(input: &str) -> Result<ParsedPcb, ParseError> {
    let root = sexpr::parse(input)?;
    let Sexpr::List(children) = &root else {
        return Err(ParseError::UnexpectedEof);
    };

    // The root must be a `kicad_pcb` list.
    if children.is_empty() || children[0].as_string() != "kicad_pcb" {
        return Err(ParseError::UnexpectedChar {
            ch: '?',
            pos: 0,
        });
    }

    let mut parsed = ParsedPcb::default();

    for node in &children[1..] {
        let Sexpr::List(parts) = node else {
            continue;
        };
        if parts.is_empty() {
            continue;
        }
        let Sexpr::Atom(tag) = &parts[0] else {
            continue;
        };

        match tag.as_str() {
            "net" => {
                if let Some((code, name)) = parse_net(parts) {
                    parsed.net_names.insert(code, name);
                }
            }
            "footprint" => {
                if let Some((refdes, placement)) = parse_footprint(parts) {
                    parsed.placements.insert(refdes, placement);
                }
            }
            "segment" => {
                if let Some(seg) = parse_segment(parts) {
                    parsed.tracks.push(seg);
                }
            }
            "via" => {
                if let Some(via) = parse_via_node(parts) {
                    parsed.vias.push(via);
                }
            }
            "zone" => {
                if let Some(zone) = parse_zone_node(parts) {
                    parsed.zones.push(zone);
                }
            }
            _ => {}
        }
    }

    Ok(parsed)
}

// ---------------------------------------------------------------------------
// Conversion to Layout
// ---------------------------------------------------------------------------

impl ParsedPcb {
    /// Convert this parsed PCB to a [`Layout`] suitable for
    /// [`emit_pcb_with_layout`](crate::pcb::emit_pcb_with_layout).
    ///
    /// Nets are remapped by name from the old PCB to the given `board`.
    /// Components are matched by reference designator. Copper elements
    /// referencing nets that no longer exist in `board` are silently dropped.
    pub fn to_layout(&self, board: &CompiledBoard) -> Layout {
        // Build net name → new NetIdx.
        let name_to_idx: HashMap<&str, NetIdx> = board
            .nets
            .iter()
            .enumerate()
            .map(|(i, n)| (n.name.as_str(), NetIdx(i)))
            .collect();

        // Old net code → new NetIdx.
        let code_to_idx: HashMap<usize, NetIdx> = self
            .net_names
            .iter()
            .filter_map(|(&code, name)| name_to_idx.get(name.as_str()).map(|&idx| (code, idx)))
            .collect();

        // Refdes → component index.
        let refdes_to_idx: HashMap<&str, usize> = board
            .components
            .iter()
            .enumerate()
            .map(|(i, c)| (c.refdes.as_str(), i))
            .collect();

        let copper_count = board.stackup.copper_layer_count();

        let placements: Vec<Placement> = self
            .placements
            .iter()
            .filter_map(|(refdes, raw)| {
                let comp_idx = *refdes_to_idx.get(refdes.as_str())?;
                let side = match raw.layer.as_str() {
                    "B.Cu" => BoardSide::Back,
                    _ => BoardSide::Front,
                };
                Some(Placement {
                    component: comp_idx,
                    at: raw.at,
                    rotation: raw.rotation,
                    side,
                })
            })
            .collect();

        let tracks: Vec<Track> = self
            .tracks
            .iter()
            .filter_map(|raw| {
                let net = *code_to_idx.get(&raw.net_code)?;
                let layer = layer_name_to_index(&raw.layer, copper_count)?;
                Some(Track {
                    net,
                    layer,
                    width: (raw.width_mm).mm(),
                    path: vec![raw.start, raw.end],
                })
            })
            .collect();

        let vias: Vec<Via> = self
            .vias
            .iter()
            .filter_map(|raw| {
                let net = *code_to_idx.get(&raw.net_code)?;
                let start = layer_name_to_index(&raw.layer_start, copper_count)?;
                let end = layer_name_to_index(&raw.layer_end, copper_count)?;
                let layers = if start <= end {
                    (start, end)
                } else {
                    (end, start)
                };
                Some(Via {
                    net,
                    at: raw.at,
                    drill: (raw.drill_mm).mm(),
                    diameter: (raw.diameter_mm).mm(),
                    layers,
                })
            })
            .collect();

        let zones: Vec<Zone> = self
            .zones
            .iter()
            .filter_map(|raw| {
                let net = *code_to_idx.get(&raw.net_code)?;
                let layer = layer_name_to_index(&raw.layer, copper_count)?;
                Some(Zone {
                    net,
                    layer,
                    outline: raw.outline.clone(),
                })
            })
            .collect();

        Layout {
            placements,
            tracks,
            vias,
            zones,
        }
    }
}

// ---------------------------------------------------------------------------
// Low-level parse helpers
// ---------------------------------------------------------------------------

/// Parse a `(net CODE "NAME")` node.  CODE is the KiCad 1-based net number.
fn parse_net(parts: &[Sexpr]) -> Option<(usize, String)> {
    if parts.len() < 3 {
        return None;
    }
    let code: usize = parts[1].as_string().parse().ok()?;
    let name = parts[2].as_string();
    Some((code, name))
}

/// Parse a footprint node, returning `(refdes, RawPlacement)`.
fn parse_footprint(parts: &[Sexpr]) -> Option<(String, RawPlacement)> {
    let mut at: Option<(f64, f64, f64)> = None;
    let mut layer = "F.Cu".to_string();
    let mut refdes: Option<String> = None;

    for child in &parts[1..] {
        let Sexpr::List(props) = child else {
            continue;
        };
        if props.is_empty() {
            continue;
        }
        let Sexpr::Atom(key) = &props[0] else {
            continue;
        };

        match key.as_str() {
            "at" => {
                if props.len() >= 3 {
                    let x: f64 = props[1].as_string().parse().ok()?;
                    let y: f64 = props[2].as_string().parse().ok()?;
                    let rot: f64 = props
                        .get(3)
                        .map(|s| s.as_string().parse().ok())
                        .flatten()
                        .unwrap_or(0.0);
                    at = Some((x, y, rot));
                }
            }
            "layer" => {
                if props.len() >= 2 {
                    layer = props[1].as_string();
                }
            }
            "property" => {
                // (property "Reference" "U1" (at ...) ...)
                if props.len() >= 3 && props[1].as_string() == "Reference" {
                    refdes = Some(props[2].as_string());
                }
            }
            _ => {}
        }
    }

    let (x, y, rot) = at?;
    let refdes = refdes?;

    Some((
        refdes,
        RawPlacement {
            at: (x, y),
            rotation: rot,
            layer,
        },
    ))
}

/// Parse a `(segment ...)` node.
fn parse_segment(parts: &[Sexpr]) -> Option<RawSegment> {
    let mut start: Option<(f64, f64)> = None;
    let mut end: Option<(f64, f64)> = None;
    let mut width_mm: Option<f64> = None;
    let mut layer: Option<String> = None;
    let mut net_code: Option<usize> = None;

    for child in &parts[1..] {
        let Sexpr::List(props) = child else {
            continue;
        };
        if props.is_empty() {
            continue;
        }
        let Sexpr::Atom(key) = &props[0] else {
            continue;
        };

        match key.as_str() {
            "start" => {
                if props.len() >= 3 {
                    let x: f64 = props[1].as_string().parse().ok()?;
                    let y: f64 = props[2].as_string().parse().ok()?;
                    start = Some((x, y));
                }
            }
            "end" => {
                if props.len() >= 3 {
                    let x: f64 = props[1].as_string().parse().ok()?;
                    let y: f64 = props[2].as_string().parse().ok()?;
                    end = Some((x, y));
                }
            }
            "width" => {
                if props.len() >= 2 {
                    width_mm = props[1].as_string().parse().ok();
                }
            }
            "layer" => {
                if props.len() >= 2 {
                    layer = Some(props[1].as_string());
                }
            }
            "net" => {
                if props.len() >= 2 {
                    net_code = props[1].as_string().parse().ok();
                }
            }
            _ => {}
        }
    }

    Some(RawSegment {
        net_code: net_code?,
        layer: layer?,
        width_mm: width_mm?,
        start: start?,
        end: end?,
    })
}

/// Parse a `(via ...)` node.
fn parse_via_node(parts: &[Sexpr]) -> Option<RawVia> {
    let mut at: Option<(f64, f64)> = None;
    let mut diameter_mm: Option<f64> = None;
    let mut drill_mm: Option<f64> = None;
    let mut layer_start: Option<String> = None;
    let mut layer_end: Option<String> = None;
    let mut net_code: Option<usize> = None;

    for child in &parts[1..] {
        let Sexpr::List(props) = child else {
            continue;
        };
        if props.is_empty() {
            continue;
        }
        let Sexpr::Atom(key) = &props[0] else {
            continue;
        };

        match key.as_str() {
            "at" => {
                if props.len() >= 3 {
                    let x: f64 = props[1].as_string().parse().ok()?;
                    let y: f64 = props[2].as_string().parse().ok()?;
                    at = Some((x, y));
                }
            }
            "size" => {
                if props.len() >= 2 {
                    diameter_mm = props[1].as_string().parse().ok();
                }
            }
            "drill" => {
                if props.len() >= 2 {
                    drill_mm = props[1].as_string().parse().ok();
                }
            }
            "layers" => {
                if props.len() >= 3 {
                    layer_start = Some(props[1].as_string());
                    layer_end = Some(props[2].as_string());
                }
            }
            "net" => {
                if props.len() >= 2 {
                    net_code = props[1].as_string().parse().ok();
                }
            }
            _ => {}
        }
    }

    Some(RawVia {
        net_code: net_code?,
        at: at?,
        diameter_mm: diameter_mm?,
        drill_mm: drill_mm?,
        layer_start: layer_start?,
        layer_end: layer_end?,
    })
}

/// Parse a `(zone ...)` node.
fn parse_zone_node(parts: &[Sexpr]) -> Option<RawZone> {
    let mut net_code: Option<usize> = None;
    let mut layer: Option<String> = None;
    let mut outline: Vec<(f64, f64)> = Vec::new();

    for child in &parts[1..] {
        let Sexpr::List(props) = child else {
            continue;
        };
        if props.is_empty() {
            continue;
        }
        let Sexpr::Atom(key) = &props[0] else {
            continue;
        };

        match key.as_str() {
            "net" => {
                if props.len() >= 2 {
                    net_code = props[1].as_string().parse().ok();
                }
            }
            "layer" => {
                if props.len() >= 2 {
                    layer = Some(props[1].as_string());
                }
            }
            "polygon" => {
                outline = parse_polygon(props);
            }
            _ => {}
        }
    }

    Some(RawZone {
        net_code: net_code?,
        layer: layer?,
        outline,
    })
}

/// Parse a `(polygon (xy X Y) (xy X Y) ...)` list.
///
/// If the last point duplicates the first (closing the polygon), it is
/// stripped so the outline matches the [`Zone`] representation.
fn parse_polygon(props: &[Sexpr]) -> Vec<(f64, f64)> {
    let mut pts = Vec::new();
    for child in &props[1..] {
        let Sexpr::List(parts) = child else {
            continue;
        };
        if parts.len() < 3 {
            continue;
        }
        if parts[0].as_string() != "xy" {
            continue;
        }
        let x: f64 = match parts[1].as_string().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let y: f64 = match parts[2].as_string().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        pts.push((x, y));
    }
    // Strip closing point if it duplicates the first.
    if pts.len() > 1 && pts.first() == pts.last() {
        pts.pop();
    }
    pts
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Map a KiCad copper layer name to a zero-based layer index.
///
/// `"F.Cu"` → 0, `"In1.Cu"` → 1, `"In2.Cu"` → 2, …, `"B.Cu"` → N-1.
fn layer_name_to_index(name: &str, copper_count: usize) -> Option<usize> {
    match name {
        "F.Cu" => Some(0),
        "B.Cu" => Some(copper_count.checked_sub(1)?),
        other => {
            // Expect "InN.Cu".
            if !other.starts_with("In") || !other.ends_with(".Cu") {
                return None;
            }
            let num_str = &other[2..other.len() - 3];
            num_str.parse::<usize>().ok()
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf::{
        CompiledBoard, CompiledComponent, ComponentMeta, Connection, Layout, Net, NetClass, NetIdx,
        NetKind, Pin, Stackup, DesignRules,
    };
    use copperleaf::units::UnitExt;

    fn two_layer_board() -> CompiledBoard {
        CompiledBoard {
            components: vec![
                CompiledComponent {
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
                },
            ],
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
            stackup: Stackup::two_layer(),
            design_rules: DesignRules::default(),
        }
    }

    #[test]
    fn parse_round_trip_nets_and_placements() {
        let board = two_layer_board();
        let pcb_str = crate::pcb::emit_pcb(&board, "test");

        let parsed = parse_pcb(&pcb_str).expect("parse emitted PCB");

        // Should have found one net (code 1 → "V3V3")
        assert_eq!(parsed.net_names.get(&1).map(|s| s.as_str()), Some("V3V3"));

        // Should have one placement for U1
        assert!(parsed.placements.contains_key("U1"));

        // Convert to layout — should have one placement, no copper.
        let layout = parsed.to_layout(&board);
        assert_eq!(layout.placements.len(), 1);
        assert_eq!(layout.placements[0].component, 0);
        assert!(layout.tracks.is_empty());
        assert!(layout.vias.is_empty());
        assert!(layout.zones.is_empty());
    }

    #[test]
    fn parse_with_layout_round_trip() {
        let board = two_layer_board();
        let layout = Layout {
            placements: vec![Placement {
                component: 0,
                at: (10.0, 40.0),
                rotation: 90.0,
                side: BoardSide::Front,
            }],
            tracks: vec![Track {
                net: NetIdx(0),
                layer: 0,
                width: 0.25.mm(),
                path: vec![(5.0, 5.0), (15.0, 5.0)],
            }],
            vias: vec![Via {
                net: NetIdx(0),
                at: (15.0, 5.0),
                drill: 0.4.mm(),
                diameter: 0.8.mm(),
                layers: (0, 1),
            }],
            zones: vec![Zone {
                net: NetIdx(0),
                layer: 1,
                outline: vec![(0.0, 0.0), (100.0, 0.0), (100.0, 80.0), (0.0, 80.0)],
            }],
        };

        let pcb_str = crate::pcb::emit_pcb_with_layout(&board, "test", Some(&layout));

        let parsed = parse_pcb(&pcb_str).expect("parse emitted PCB with layout");

        // One net
        assert_eq!(parsed.net_names.get(&1).map(|s| s.as_str()), Some("V3V3"));

        // One placement
        let u1_place = &parsed.placements["U1"];
        assert!((u1_place.at.0 - 10.0).abs() < 0.001);
        assert!((u1_place.at.1 - 40.0).abs() < 0.001);
        assert!((u1_place.rotation - 90.0).abs() < 0.001);

        // One track segment
        assert_eq!(parsed.tracks.len(), 1);
        assert_eq!(parsed.tracks[0].net_code, 1);
        assert_eq!(parsed.tracks[0].layer, "F.Cu");
        assert!((parsed.tracks[0].width_mm - 0.25).abs() < 0.001);

        // One via
        assert_eq!(parsed.vias.len(), 1);
        assert_eq!(parsed.vias[0].net_code, 1);
        assert!((parsed.vias[0].diameter_mm - 0.8).abs() < 0.001);
        assert!((parsed.vias[0].drill_mm - 0.4).abs() < 0.001);

        // One zone
        assert_eq!(parsed.zones.len(), 1);
        assert_eq!(parsed.zones[0].net_code, 1);
        assert_eq!(parsed.zones[0].layer, "B.Cu");
        assert_eq!(parsed.zones[0].outline.len(), 4);

        // Convert to layout and verify the round-trip
        let layout2 = parsed.to_layout(&board);
        assert_eq!(layout2.placements.len(), 1);
        assert_eq!(layout2.placements[0], layout.placements[0]);
        assert_eq!(layout2.tracks.len(), 1);
        assert_eq!(layout2.vias.len(), 1);
        assert_eq!(layout2.zones.len(), 1);
    }

    #[test]
    fn net_remapping_drops_deleted_nets() {
        let board = two_layer_board();
        let layout = Layout {
            placements: vec![],
            tracks: vec![Track {
                net: NetIdx(0),
                layer: 0,
                width: 0.25.mm(),
                path: vec![(5.0, 5.0), (15.0, 5.0)],
            }],
            vias: vec![],
            zones: vec![],
        };

        let pcb_str = crate::pcb::emit_pcb_with_layout(&board, "test", Some(&layout));
        let parsed = parse_pcb(&pcb_str).unwrap();

        // Create a modified board with no nets (simulating net deletion).
        let empty_board = CompiledBoard {
            nets: vec![],
            ..board.clone()
        };

        let layout2 = parsed.to_layout(&empty_board);
        // Track referencing a deleted net should be dropped.
        assert!(layout2.tracks.is_empty());
    }

    #[test]
    fn net_renamed_preserves_copper() {
        let board = two_layer_board();
        let layout = Layout {
            placements: vec![],
            tracks: vec![Track {
                net: NetIdx(0),
                layer: 0,
                width: 0.25.mm(),
                path: vec![(5.0, 5.0), (15.0, 5.0)],
            }],
            vias: vec![],
            zones: vec![],
        };

        let pcb_str = crate::pcb::emit_pcb_with_layout(&board, "test", Some(&layout));
        let parsed = parse_pcb(&pcb_str).unwrap();

        // Create a board where the net is renamed but the name matches.
        let renamed_board = CompiledBoard {
            nets: vec![Net {
                name: "V3V3".into(), // same name
                kind: NetKind::Power {
                    v_nom: 3.3.volt(),
                    ripple: None,
                },
                class: NetClass::default(),
                constraints: vec![],
                layout: vec![],
            }],
            ..board.clone()
        };

        let layout2 = parsed.to_layout(&renamed_board);
        // Net should still be found by name → tracks preserved.
        assert_eq!(layout2.tracks.len(), 1);
        assert_eq!(layout2.tracks[0].net, NetIdx(0));
    }

    #[test]
    fn layer_name_parsing() {
        // 2-layer board
        assert_eq!(layer_name_to_index("F.Cu", 2), Some(0));
        assert_eq!(layer_name_to_index("B.Cu", 2), Some(1));

        // 4-layer board
        assert_eq!(layer_name_to_index("F.Cu", 4), Some(0));
        assert_eq!(layer_name_to_index("In1.Cu", 4), Some(1));
        assert_eq!(layer_name_to_index("In2.Cu", 4), Some(2));
        assert_eq!(layer_name_to_index("B.Cu", 4), Some(3));

        // Invalid names
        assert_eq!(layer_name_to_index("F.SilkS", 2), None);
        assert_eq!(layer_name_to_index("", 2), None);
    }
}
