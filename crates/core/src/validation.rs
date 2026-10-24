//! Central compiled-board validation.
//!
//! [`validate_compiled_board`] is the single invariant pass at the
//! compiler/backend trust boundary. It reports every independent defect in
//! one result so a malformed board can be corrected or rejected before any
//! emitter writes output.
//!
//! Validation is deliberately separate from [`Board`](crate::Board)
//! construction: intermediate builder states are intentionally incomplete.

use std::collections::{HashMap, HashSet};

use crate::{
    board::{CompiledBoard, CompiledComponent},
    layout::{CopperLayer, LayerSet, Layout, LayoutConstraint},
    pin::{Pad, resolve_mech_pad, resolve_pad, resolve_pin_identities},
    stackup::StackupLayer,
    units::{Diagnostic, Severity},
};

/// Validate a compiled board and, when supplied, its solved layout.
///
/// Every invariant is checked independently: the returned diagnostics report
/// all discovered problems in one pass. An empty result means the board is
/// safe for every backend to emit.
pub fn validate_compiled_board(board: &CompiledBoard, layout: Option<&Layout>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    validate_dimensions(board, &mut diagnostics);
    validate_design_rules(board, &mut diagnostics);
    validate_stackup(board, &mut diagnostics);
    validate_components(board, &mut diagnostics);
    validate_nets(board, &mut diagnostics);
    validate_connections(board, &mut diagnostics);
    validate_directives(board, &mut diagnostics);
    if let Some(layout) = layout {
        validate_layout(board, layout, &mut diagnostics);
    }

    diagnostics
}

fn error(code: &str, message: String, entities: Vec<String>, hint: Option<&str>) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        severity: Severity::Error,
        message,
        entities,
        hint: hint.map(str::to_owned),
    }
}

fn finite(value: f64) -> bool {
    value.is_finite()
}

fn positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn validate_component_pins(component: &CompiledComponent, diagnostics: &mut Vec<Diagnostic>) {
    // Canonical identity diagnostics: missing/conflicting numbers and
    // duplicate component-local identities.
    diagnostics.extend(resolve_pin_identities(&component.refdes, &component.pins).diagnostics);

    let mut ids: HashMap<&str, usize> = HashMap::new();
    for (index, pin) in component.pins.iter().enumerate() {
        if pin.id().0.is_empty() {
            diagnostics.push(error(
                "BOARD:EMPTY_PIN_ID",
                format!(
                    "pin '{}' on '{}' has an empty PinId",
                    pin.name(),
                    component.refdes
                ),
                vec![format!("{}.{}", component.refdes, pin.name())],
                None,
            ));
        } else if let Some(previous) = ids.insert(pin.id().0.as_str(), index) {
            diagnostics.push(error(
                "BOARD:DUPLICATE_PIN_ID",
                format!(
                    "pins {previous} and {index} on '{}' share a PinId",
                    component.refdes
                ),
                vec![format!("{}.{}", component.refdes, pin.name())],
                None,
            ));
        }
    }
}

fn validate_components(board: &CompiledBoard, diagnostics: &mut Vec<Diagnostic>) {
    let mut refdes_seen: HashMap<&str, usize> = HashMap::new();
    for (index, component) in board.components.iter().enumerate() {
        if component.refdes.trim().is_empty() {
            diagnostics.push(error(
                "BOARD:EMPTY_REFDES",
                format!("component at index {index} has an empty refdes"),
                vec![format!("component:{index}")],
                None,
            ));
        } else if let Some(previous) = refdes_seen.insert(&component.refdes, index) {
            diagnostics.push(error(
                "BOARD:DUPLICATE_REFDES",
                format!(
                    "components {previous} and {index} share refdes '{}'",
                    component.refdes
                ),
                vec![
                    format!("{}#{previous}", component.refdes),
                    format!("{}#{index}", component.refdes),
                ],
                None,
            ));
        }

        validate_component_pins(component, diagnostics);

        if let Some(data) = &component.meta.model_3d_data
            && data.trim().is_empty()
        {
            diagnostics.push(error(
                "BOARD:EMPTY_MODEL_DATA",
                format!(
                    "component '{}' declares an empty 3D-model payload",
                    component.refdes
                ),
                vec![format!("{}.model_3d_data", component.refdes)],
                Some("Remove the empty payload or supply encoded model data."),
            ));
        }

        for (pin_index, pin) in component.pins.iter().enumerate() {
            validate_pad(
                &resolve_pad(pin, pin_index),
                &format!("{}.{}", component.refdes, pin.name()),
                diagnostics,
            );
        }
        for mechanical in &component.mechanical {
            validate_pad(
                &resolve_mech_pad(mechanical),
                &format!("{}.mechanical", component.refdes),
                diagnostics,
            );
        }
    }
}

fn validate_connections(board: &CompiledBoard, diagnostics: &mut Vec<Diagnostic>) {
    for connection in &board.connections {
        let Some(component) = board.components.get(connection.component) else {
            diagnostics.push(error(
                "BOARD:INVALID_CONNECTION",
                format!(
                    "connection references unknown component index {}",
                    connection.component
                ),
                vec![format!("component:{}", connection.component)],
                None,
            ));
            continue;
        };
        match component.pins.get(connection.pin_index) {
            Some(pin) if pin.id() == &connection.pin => {}
            Some(pin) => diagnostics.push(error(
                "BOARD:INVALID_CONNECTION",
                format!(
                    "connection pin index {} on '{}' resolves to a different physical pin",
                    connection.pin_index, component.refdes
                ),
                vec![format!("{}.{}", component.refdes, pin.name())],
                None,
            )),
            None => diagnostics.push(error(
                "BOARD:INVALID_CONNECTION",
                format!(
                    "connection references unknown pin index {} on '{}'",
                    connection.pin_index, component.refdes
                ),
                vec![format!("{}.{}", component.refdes, connection.pin.0)],
                None,
            )),
        }
        if connection.net.0 >= board.nets.len() {
            diagnostics.push(error(
                "BOARD:INVALID_CONNECTION",
                format!(
                    "connection references unknown net index {}",
                    connection.net.0
                ),
                vec![format!("net:{}", connection.net.0)],
                None,
            ));
        }
    }
}

fn validate_design_rules(board: &CompiledBoard, diagnostics: &mut Vec<Diagnostic>) {
    let rules = &board.design_rules;
    let fields = [
        ("min_clearance", rules.min_clearance),
        ("min_pad_to_pad_clearance", rules.min_pad_to_pad_clearance),
        ("min_connection", rules.min_connection),
        ("min_copper_edge_clearance", rules.min_copper_edge_clearance),
        ("min_hole_clearance", rules.min_hole_clearance),
        ("min_hole_to_hole", rules.min_hole_to_hole),
        ("min_silk_clearance", rules.min_silk_clearance),
        ("min_track_width", rules.min_track_width),
        ("min_via_diameter", rules.min_via_diameter),
        ("min_via_drill", rules.min_via_drill),
        ("min_via_annular_width", rules.min_via_annular_width),
        ("min_through_hole_diameter", rules.min_through_hole_diameter),
        ("min_microvia_diameter", rules.min_microvia_diameter),
        ("min_microvia_drill", rules.min_microvia_drill),
        (
            "solder_mask_to_copper_clearance",
            rules.solder_mask_to_copper_clearance,
        ),
        ("min_text_height", rules.min_text_height),
        ("min_text_thickness", rules.min_text_thickness),
    ];
    for (name, value) in fields {
        if !finite(value) || value < 0.0 {
            diagnostics.push(error(
                "BOARD:INVALID_DESIGN_RULE",
                format!("design rule '{name}' must be finite and non-negative, got {value}"),
                vec![format!("design_rule:{name}")],
                None,
            ));
        }
    }
}

fn validate_dimensions(board: &CompiledBoard, diagnostics: &mut Vec<Diagnostic>) {
    if !positive(board.width) {
        diagnostics.push(error(
            "BOARD:INVALID_DIMENSION",
            format!(
                "board width must be finite and positive, got {}",
                board.width
            ),
            vec!["board:width".into()],
            None,
        ));
    }
    if !positive(board.height) {
        diagnostics.push(error(
            "BOARD:INVALID_DIMENSION",
            format!(
                "board height must be finite and positive, got {}",
                board.height
            ),
            vec!["board:height".into()],
            None,
        ));
    }
}

fn validate_directive(
    board: &CompiledBoard,
    directive: &LayoutConstraint,
    owner: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match directive {
        LayoutConstraint::Plane { layer } => validate_layer(board, *layer, owner, diagnostics),
        LayoutConstraint::Keepout { region, layers } => {
            validate_region(region, owner, diagnostics);
            validate_layer_set(board, layers, owner, diagnostics);
        }
        LayoutConstraint::PlaceAt { pos, rotation, .. } => {
            if !finite(pos.0) || !finite(pos.1) || !finite(*rotation) {
                diagnostics.push(error(
                    "BOARD:INVALID_PLACEMENT",
                    format!("{owner} has a non-finite placement directive"),
                    vec![owner.to_owned()],
                    None,
                ));
            }
        }
        LayoutConstraint::PlaceNear { target, max_radius } => {
            let crate::layout::PlaceTarget::Component(target) = target;
            if *target >= board.components.len() {
                diagnostics.push(error(
                    "BOARD:INVALID_PLACEMENT",
                    format!("{owner} targets unknown component {target}"),
                    vec![owner.to_owned()],
                    None,
                ));
            }
            if !positive(max_radius.as_base()) {
                diagnostics.push(error(
                    "BOARD:INVALID_PLACEMENT",
                    format!("{owner} has a non-positive placement radius"),
                    vec![owner.to_owned()],
                    None,
                ));
            }
        }
        _ => {}
    }
}

fn validate_directives(board: &CompiledBoard, diagnostics: &mut Vec<Diagnostic>) {
    for (component_index, component) in board.components.iter().enumerate() {
        for directive in &component.layout {
            validate_directive(
                board,
                directive,
                &format!("component:{component_index}"),
                diagnostics,
            );
        }
    }
    for (net_index, net) in board.nets.iter().enumerate() {
        for directive in &net.layout {
            validate_directive(board, directive, &format!("net:{net_index}"), diagnostics);
        }
    }
    for directive in &board.layout {
        validate_directive(board, directive, "board", diagnostics);
    }
}

fn validate_layer(
    board: &CompiledBoard,
    layer: CopperLayer,
    owner: &str,
    out: &mut Vec<Diagnostic>,
) {
    if board.stackup.validate_copper_layer(layer).is_err() {
        out.push(error(
            "BOARD:LAYER_OUT_OF_RANGE",
            format!(
                "{owner} references copper ordinal {} outside the stackup",
                layer.0
            ),
            vec![format!("{owner}:layer:{}", layer.0)],
            None,
        ));
    }
}

fn validate_layer_set(
    board: &CompiledBoard,
    layers: &LayerSet,
    owner: &str,
    out: &mut Vec<Diagnostic>,
) {
    if layers.layers.is_empty() {
        out.push(error(
            "BOARD:INVALID_LAYER_SET",
            format!("{owner} has an empty layer set"),
            vec![owner.to_owned()],
            None,
        ));
    }
    for layer in &layers.layers {
        validate_layer(board, *layer, owner, out);
    }
}

fn validate_layout(board: &CompiledBoard, layout: &Layout, diagnostics: &mut Vec<Diagnostic>) {
    let mut placed = HashSet::new();
    for placement in &layout.placements {
        if placement.component >= board.components.len() {
            diagnostics.push(error(
                "BOARD:INVALID_PLACEMENT",
                format!(
                    "placement references unknown component {}",
                    placement.component
                ),
                vec![format!("component:{}", placement.component)],
                None,
            ));
        } else if !placed.insert(placement.component) {
            diagnostics.push(error(
                "BOARD:INVALID_PLACEMENT",
                format!(
                    "component '{}' is placed more than once",
                    board.components[placement.component].refdes
                ),
                vec![format!("component:{}", placement.component)],
                None,
            ));
        }
        if !finite(placement.at.0) || !finite(placement.at.1) || !finite(placement.rotation) {
            diagnostics.push(error(
                "BOARD:INVALID_PLACEMENT",
                format!(
                    "placement for component {} has non-finite geometry",
                    placement.component
                ),
                vec![format!("component:{}", placement.component)],
                None,
            ));
        }
    }

    for (index, track) in layout.tracks.iter().enumerate() {
        if track.net.0 >= board.nets.len() {
            diagnostics.push(error(
                "BOARD:INVALID_TRACK",
                format!("track {index} references unknown net {}", track.net.0),
                vec![format!("track:{index}")],
                None,
            ));
        }
        validate_layer(board, track.layer, &format!("track:{index}"), diagnostics);
        if track.path.len() < 2 {
            diagnostics.push(error(
                "BOARD:INVALID_TRACK",
                format!("track {index} has fewer than two path points"),
                vec![format!("track:{index}")],
                None,
            ));
        }
        if track.path.iter().any(|(x, y)| !finite(*x) || !finite(*y)) {
            diagnostics.push(error(
                "BOARD:INVALID_TRACK",
                format!("track {index} has non-finite coordinates"),
                vec![format!("track:{index}")],
                None,
            ));
        }
        if !positive(track.width.as_base()) {
            diagnostics.push(error(
                "BOARD:INVALID_TRACK",
                format!("track {index} has a non-positive width"),
                vec![format!("track:{index}")],
                None,
            ));
        }
    }

    for (index, via) in layout.vias.iter().enumerate() {
        if via.net.0 >= board.nets.len() {
            diagnostics.push(error(
                "BOARD:INVALID_VIA",
                format!("via {index} references unknown net {}", via.net.0),
                vec![format!("via:{index}")],
                None,
            ));
        }
        validate_layer(board, via.layers.0, &format!("via:{index}"), diagnostics);
        validate_layer(board, via.layers.1, &format!("via:{index}"), diagnostics);
        if via.layers.0 > via.layers.1 {
            diagnostics.push(error(
                "BOARD:INVALID_VIA",
                format!("via {index} start layer is below its end layer"),
                vec![format!("via:{index}")],
                None,
            ));
        }
        if !finite(via.at.0) || !finite(via.at.1) {
            diagnostics.push(error(
                "BOARD:INVALID_VIA",
                format!("via {index} has non-finite coordinates"),
                vec![format!("via:{index}")],
                None,
            ));
        }
        if !positive(via.drill.as_base()) || !positive(via.diameter.as_base()) {
            diagnostics.push(error(
                "BOARD:INVALID_VIA",
                format!("via {index} has non-positive drill or diameter"),
                vec![format!("via:{index}")],
                None,
            ));
        }
    }

    for (index, zone) in layout.zones.iter().enumerate() {
        if zone.net.0 >= board.nets.len() {
            diagnostics.push(error(
                "BOARD:INVALID_ZONE",
                format!("zone {index} references unknown net {}", zone.net.0),
                vec![format!("zone:{index}")],
                None,
            ));
        }
        validate_layer(board, zone.layer, &format!("zone:{index}"), diagnostics);
        if zone.outline.len() < 3 {
            diagnostics.push(error(
                "BOARD:INVALID_ZONE",
                format!("zone {index} has fewer than three outline points"),
                vec![format!("zone:{index}")],
                None,
            ));
        }
        if zone.outline.iter().any(|(x, y)| !finite(*x) || !finite(*y)) {
            diagnostics.push(error(
                "BOARD:INVALID_ZONE",
                format!("zone {index} has non-finite geometry"),
                vec![format!("zone:{index}")],
                None,
            ));
        }
    }
}

fn validate_nets(board: &CompiledBoard, diagnostics: &mut Vec<Diagnostic>) {
    let mut names: HashMap<&str, usize> = HashMap::new();
    let mut referenced = vec![false; board.nets.len()];
    for connection in &board.connections {
        if connection.net.0 < referenced.len() {
            referenced[connection.net.0] = true;
        }
    }
    for (index, net) in board.nets.iter().enumerate() {
        if net.name.trim().is_empty() {
            diagnostics.push(error(
                "BOARD:EMPTY_NET_NAME",
                format!("net at index {index} has an empty name"),
                vec![format!("net:{index}")],
                None,
            ));
        } else if let Some(previous) = names.insert(&net.name, index) {
            diagnostics.push(error(
                "BOARD:DUPLICATE_NET",
                format!("nets {previous} and {index} share the name '{}'", net.name),
                vec![format!("net:{}", net.name)],
                None,
            ));
        }
        if !referenced[index] {
            diagnostics.push(error(
                "BOARD:ORPHAN_NET",
                format!("net '{}' has no connections", net.name),
                vec![format!("net:{}", net.name)],
                None,
            ));
        }
    }
}

fn validate_pad(pad: &Pad, owner: &str, diagnostics: &mut Vec<Diagnostic>) {
    if !finite(pad.pos.0) || !finite(pad.pos.1) {
        diagnostics.push(error(
            "BOARD:INVALID_GEOMETRY",
            format!("pad '{owner}' has a non-finite position"),
            vec![owner.to_owned()],
            None,
        ));
    }
    if !positive(pad.width) || !positive(pad.height) {
        diagnostics.push(error(
            "BOARD:INVALID_GEOMETRY",
            format!(
                "pad '{owner}' dimensions must be finite and positive, got {}x{}",
                pad.width, pad.height
            ),
            vec![owner.to_owned()],
            None,
        ));
    }
    if let Some(drill) = pad.drill
        && !positive(drill)
    {
        diagnostics.push(error(
            "BOARD:INVALID_GEOMETRY",
            format!("pad '{owner}' drill must be finite and positive, got {drill}"),
            vec![owner.to_owned()],
            None,
        ));
    }
}

fn validate_region(region: &crate::layout::Region, owner: &str, diagnostics: &mut Vec<Diagnostic>) {
    let points: Vec<(f64, f64)> = match region {
        crate::layout::Region::Rect { x1, y1, x2, y2 } => {
            vec![(*x1, *y1), (*x2, *y2)]
        }
        crate::layout::Region::Circle { cx, cy, r } => {
            if !positive(*r) {
                diagnostics.push(error(
                    "BOARD:INVALID_GEOMETRY",
                    format!("{owner} has a non-positive keepout radius"),
                    vec![owner.to_owned()],
                    None,
                ));
            }
            vec![(*cx, *cy)]
        }
    };
    if points.iter().any(|(x, y)| !finite(*x) || !finite(*y)) {
        diagnostics.push(error(
            "BOARD:INVALID_GEOMETRY",
            format!("{owner} has non-finite keepout geometry"),
            vec![owner.to_owned()],
            None,
        ));
    }
}

fn validate_stackup(board: &CompiledBoard, diagnostics: &mut Vec<Diagnostic>) {
    let stackup = &board.stackup;
    if stackup.layers.is_empty() {
        diagnostics.push(error(
            "BOARD:INVALID_STACKUP",
            "stackup has no layers".into(),
            vec!["stackup".into()],
            Some("Configure at least one copper layer."),
        ));
        return;
    }
    if !matches!(stackup.layers.first(), Some(StackupLayer::Copper { .. })) {
        diagnostics.push(error(
            "BOARD:INVALID_STACKUP",
            "stackup must start with a copper layer".into(),
            vec!["stackup:first".into()],
            None,
        ));
    }
    if !matches!(stackup.layers.last(), Some(StackupLayer::Copper { .. })) {
        diagnostics.push(error(
            "BOARD:INVALID_STACKUP",
            "stackup must end with a copper layer".into(),
            vec!["stackup:last".into()],
            None,
        ));
    }
    for (index, layer) in stackup.layers.iter().enumerate() {
        let is_copper = matches!(layer, StackupLayer::Copper { .. });
        if is_copper != (index % 2 == 0) {
            diagnostics.push(error(
                "BOARD:INVALID_STACKUP",
                format!(
                    "stackup layer {index} is {} but must alternate copper/dielectric",
                    if is_copper { "copper" } else { "dielectric" }
                ),
                vec![format!("stackup:{index}")],
                None,
            ));
        }
        let thickness = match layer {
            StackupLayer::Copper { thickness_mm, .. } => *thickness_mm,
            StackupLayer::Dielectric { thickness_mm, .. } => *thickness_mm,
        };
        if !positive(thickness) {
            diagnostics.push(error(
                "BOARD:INVALID_STACKUP",
                format!("stackup layer {index} has non-positive thickness {thickness}"),
                vec![format!("stackup:{index}")],
                None,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        board::{CompiledComponent, Connection},
        layout::{BoardSide, Placement, Track, Via, Zone},
        net::{Net, NetIdx},
        pin::{Pad, PadShape, PadType, Pin, PinId},
        units::UnitExt,
    };

    fn component() -> CompiledComponent {
        let mut component = CompiledComponent::test(
            "U1",
            vec![
                Pin::build("VDD")
                    .number("1")
                    .pwr_fixed(3.3.volt(), 0.1.amp())
                    .pin(),
            ],
        );
        component.pins[0] = component.pins[0]
            .clone()
            .with_id(PinId(crate::deterministic_id("U1:1")));
        component
    }

    fn valid_board() -> CompiledBoard {
        let component = component();
        CompiledBoard {
            connections: vec![Connection {
                component: 0,
                pin: component.pins[0].id().clone(),
                pin_index: 0,
                net: NetIdx(0),
            }],
            components: vec![component],
            nets: vec![Net::power("V3V3", 3.3.volt())],
            constraints: vec![],
            layout: vec![],
            width: 100.0,
            height: 80.0,
            stackup: crate::Stackup::two_layer(),
            design_rules: crate::DesignRules::default(),
        }
    }

    fn codes(board: &CompiledBoard, layout: Option<&Layout>) -> Vec<String> {
        validate_compiled_board(board, layout)
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect()
    }

    #[test]
    fn valid_board_has_no_findings() {
        assert!(validate_compiled_board(&valid_board(), None).is_empty());
    }

    #[test]
    fn duplicate_refdes_is_reported() {
        let mut board = valid_board();
        let mut duplicate = component();
        duplicate.pins[0] = duplicate.pins[0]
            .clone()
            .with_id(PinId(crate::deterministic_id("U2:1")));
        board.components.push(duplicate);
        assert!(codes(&board, None).contains(&"BOARD:DUPLICATE_REFDES".into()));
    }

    #[test]
    fn empty_refdes_is_reported() {
        let mut board = valid_board();
        board.components[0].refdes.clear();
        assert!(codes(&board, None).contains(&"BOARD:EMPTY_REFDES".into()));
    }

    #[test]
    fn duplicate_physical_identity_is_reported() {
        let mut board = valid_board();
        let second = Pin::build("VSS").number("1").gnd();
        board.components[0]
            .pins
            .push(second.with_id(PinId("second".into())));
        assert!(codes(&board, None).contains(&"PIN:DUPLICATE_IDENTITY".into()));
    }

    #[test]
    fn duplicate_pin_id_is_reported() {
        let mut board = valid_board();
        let id = board.components[0].pins[0].id().clone();
        board.components[0]
            .pins
            .push(Pin::build("GND").number("2").gnd().with_id(id));
        assert!(codes(&board, None).contains(&"BOARD:DUPLICATE_PIN_ID".into()));
    }

    #[test]
    fn net_ownership_problems_are_reported() {
        let mut board = valid_board();
        board.nets.push(board.nets[0].clone());
        assert!(codes(&board, None).contains(&"BOARD:DUPLICATE_NET".into()));

        let mut empty = valid_board();
        empty.nets[0].name.clear();
        assert!(codes(&empty, None).contains(&"BOARD:EMPTY_NET_NAME".into()));

        let mut orphan = valid_board();
        orphan.connections.clear();
        assert!(codes(&orphan, None).contains(&"BOARD:ORPHAN_NET".into()));
    }

    #[test]
    fn invalid_connection_is_reported() {
        let mut board = valid_board();
        board.connections[0].net = NetIdx(99);
        assert!(codes(&board, None).contains(&"BOARD:INVALID_CONNECTION".into()));

        let mut board = valid_board();
        board.connections[0].pin_index = 99;
        assert!(codes(&board, None).contains(&"BOARD:INVALID_CONNECTION".into()));
    }

    #[test]
    fn invalid_dimension_is_reported() {
        let mut board = valid_board();
        board.width = f64::NAN;
        board.height = -1.0;
        let found = codes(&board, None);
        assert_eq!(
            found
                .iter()
                .filter(|code| *code == "BOARD:INVALID_DIMENSION")
                .count(),
            2
        );
    }

    #[test]
    fn invalid_design_rule_is_reported() {
        let mut board = valid_board();
        board.design_rules.min_track_width = f64::INFINITY;
        assert!(codes(&board, None).contains(&"BOARD:INVALID_DESIGN_RULE".into()));
    }

    #[test]
    fn invalid_stackup_is_reported() {
        let mut board = valid_board();
        board.stackup = crate::Stackup::default();
        assert!(codes(&board, None).contains(&"BOARD:INVALID_STACKUP".into()));

        let mut unordered = valid_board();
        unordered.stackup.layers.swap(0, 1);
        assert!(codes(&unordered, None).contains(&"BOARD:INVALID_STACKUP".into()));
    }

    #[test]
    fn invalid_pad_geometry_is_reported() {
        let mut board = valid_board();
        board.components[0].pins[0] = Pin::build("VDD")
            .number("1")
            .pad(Pad {
                number: "1".into(),
                pos: (f64::NAN, 0.0),
                rotation: 0.0,
                width: 0.0,
                height: 1.0,
                pad_type: PadType::Smd,
                pad_shape: PadShape::Rect,
                roundrect_rratio: None,
                solder_mask_margin: None,
                layers: None,
                drill: None,
            })
            .pwr_fixed(3.3.volt(), 0.1.amp())
            .pin()
            .with_id(PinId(crate::deterministic_id("U1:1")));
        let found = codes(&board, None);
        assert!(found.iter().any(|code| code == "BOARD:INVALID_GEOMETRY"));
    }

    fn valid_layout() -> Layout {
        Layout {
            placements: vec![Placement {
                component: 0,
                at: (10.0, 10.0),
                rotation: 0.0,
                side: BoardSide::Front,
            }],
            tracks: vec![Track {
                net: NetIdx(0),
                layer: CopperLayer(0),
                width: 0.25.mm(),
                path: vec![(0.0, 0.0), (1.0, 1.0)],
            }],
            vias: vec![Via {
                net: NetIdx(0),
                at: (1.0, 1.0),
                drill: 0.3.mm(),
                diameter: 0.6.mm(),
                layers: (CopperLayer(0), CopperLayer(1)),
            }],
            zones: vec![Zone {
                net: NetIdx(0),
                layer: CopperLayer(1),
                outline: vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)],
            }],
        }
    }

    #[test]
    fn valid_layout_has_no_findings() {
        let board = valid_board();
        let layout = valid_layout();
        assert!(validate_compiled_board(&board, Some(&layout)).is_empty());
    }

    #[test]
    fn out_of_range_layer_is_reported() {
        let board = valid_board();
        let mut layout = valid_layout();
        layout.tracks[0].layer = CopperLayer(7);
        assert!(codes(&board, Some(&layout)).contains(&"BOARD:LAYER_OUT_OF_RANGE".into()));

        let mut board = valid_board();
        board.nets[0].layout.push(LayoutConstraint::Plane {
            layer: CopperLayer(7),
        });
        assert!(codes(&board, None).contains(&"BOARD:LAYER_OUT_OF_RANGE".into()));
    }

    #[test]
    fn invalid_placement_is_reported() {
        let board = valid_board();
        let mut layout = valid_layout();
        layout.placements.push(layout.placements[0].clone());
        assert!(codes(&board, Some(&layout)).contains(&"BOARD:INVALID_PLACEMENT".into()));

        let mut layout = valid_layout();
        layout.placements[0].component = 99;
        assert!(codes(&board, Some(&layout)).contains(&"BOARD:INVALID_PLACEMENT".into()));

        let mut layout = valid_layout();
        layout.placements[0].at = (f64::NAN, 0.0);
        assert!(codes(&board, Some(&layout)).contains(&"BOARD:INVALID_PLACEMENT".into()));
    }

    #[test]
    fn invalid_track_is_reported() {
        let board = valid_board();
        let mut layout = valid_layout();
        layout.tracks[0].path.truncate(1);
        layout.tracks[0].width = 0.0.mm();
        assert!(codes(&board, Some(&layout)).contains(&"BOARD:INVALID_TRACK".into()));
    }

    #[test]
    fn invalid_via_is_reported() {
        let board = valid_board();
        let mut layout = valid_layout();
        layout.vias[0].layers = (CopperLayer(1), CopperLayer(0));
        layout.vias[0].drill = 0.0.mm();
        assert!(codes(&board, Some(&layout)).contains(&"BOARD:INVALID_VIA".into()));
    }

    #[test]
    fn invalid_zone_is_reported() {
        let board = valid_board();
        let mut layout = valid_layout();
        layout.zones[0].outline.truncate(2);
        assert!(codes(&board, Some(&layout)).contains(&"BOARD:INVALID_ZONE".into()));
    }

    #[test]
    fn multiple_independent_defects_are_reported_together() {
        let mut board = valid_board();
        let mut duplicate = component();
        duplicate.pins[0] = duplicate.pins[0]
            .clone()
            .with_id(PinId(crate::deterministic_id("U2:1")));
        board.components.push(duplicate);
        board.width = f64::NAN;
        board.connections[0].net = NetIdx(99);
        let found = codes(&board, None);
        assert!(found.contains(&"BOARD:DUPLICATE_REFDES".into()));
        assert!(found.contains(&"BOARD:INVALID_DIMENSION".into()));
        assert!(found.contains(&"BOARD:INVALID_CONNECTION".into()));
    }
}
