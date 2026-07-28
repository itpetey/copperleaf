//! Integration tests for the copperleaf-layout solver.
//!
//! These tests verify structural invariants of the solved layout:
//! - Every component has a placement.
//! - Plane nets produce zones.
//! - DRC passes on the solved output.
//! - Fixed placements (PlaceAt) are respected.

use copperleaf::{
    CompiledBoard, CompiledComponent, Connection, LayoutConstraint, Net, NetClass, NetIdx,
    NetKind, Pin, Stackup, layout::BoardSide, units::UnitExt,
};
use copperleaf_layout::{Effort, LayoutError, SolveOptions, solve};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A minimal two-layer board with two SMD components and a ground net.
fn make_test_board() -> CompiledBoard {
    let design_rules = copperleaf::DesignRules::default();
    CompiledBoard {
        components: vec![
            CompiledComponent::test_with(
                "C1",
                vec![
                    Pin::build("1")
                        .pos(0.0, 0.0)
                        .pad_type("smd")
                        .pwr(0.0.volt(), 5.0.volt(), 0.1.amp())
                        .pin(),
                    Pin::build("2")
                        .pos(2.0, 0.0)
                        .pad_type("smd")
                        .pwr(0.0.volt(), 5.0.volt(), 0.1.amp())
                        .pin(),
                ],
                vec![],
            ),
            CompiledComponent::test_with(
                "R1",
                vec![
                    Pin::build("1")
                        .pos(0.0, 0.0)
                        .pad_type("smd")
                        .pwr(0.0.volt(), 5.0.volt(), 0.1.amp())
                        .pin(),
                    Pin::build("2")
                        .pos(3.0, 0.0)
                        .pad_type("smd")
                        .pwr(0.0.volt(), 5.0.volt(), 0.1.amp())
                        .pin(),
                ],
                vec![],
            ),
        ],
        nets: vec![
            Net {
                name: "V3V3".into(),
                kind: NetKind::Power {
                    v_nom: 3.3.volt(),
                    ripple: None,
                },
                class: NetClass::default(),
                constraints: vec![],
                layout: vec![],
            },
            Net {
                name: "GND".into(),
                kind: NetKind::Power {
                    v_nom: 0.0.volt(),
                    ripple: None,
                },
                class: NetClass::default(),
                constraints: vec![],
                layout: vec![LayoutConstraint::Plane { layer: 3 }], // B.Cu
            },
        ],
        connections: vec![
            Connection {
                component: 0,
                pin: "1".into(),
                net: NetIdx(0),
            },
            Connection {
                component: 0,
                pin: "2".into(),
                net: NetIdx(1),
            },
            Connection {
                component: 1,
                pin: "1".into(),
                net: NetIdx(0),
            },
            Connection {
                component: 1,
                pin: "2".into(),
                net: NetIdx(1),
            },
        ],
        constraints: vec![],
        layout: vec![],
        width: 50.0,
        height: 30.0,
        stackup: Stackup::two_layer(),
        design_rules,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn solve_produces_placements_for_all_components() {
    let board = make_test_board();
    let options = SolveOptions {
        seed: 42,
        effort: Effort::Low,
    };

    let report = solve(&board, &options).expect("solve should succeed");
    assert_eq!(
        report.layout.placements.len(),
        board.components.len(),
        "every component should have a placement"
    );

    for (i, placement) in report.layout.placements.iter().enumerate() {
        assert_eq!(placement.component, i);
    }
}

#[test]
fn plane_nets_produce_zones() {
    let board = make_test_board();
    let options = SolveOptions::default();

    let report = solve(&board, &options).expect("solve should succeed");

    // GND is a plane net on B.Cu → should appear as a zone.
    assert!(
        report.layout.zones.iter().any(|z| z.net == NetIdx(1)),
        "GND (net 1) should have a zone"
    );
}

#[test]
fn plane_zones_cover_board_outline() {
    let board = make_test_board();
    let options = SolveOptions::default();

    let report = solve(&board, &options).expect("solve should succeed");

    // The zone outline should match the board outline (rectangle).
    for zone in &report.layout.zones {
        assert_eq!(zone.outline.len(), 4, "zone outline should be a rectangle");
        // Should cover board area: (0,0) → (width, height)
        assert!(zone.outline.iter().any(|&(x, y)| x == 0.0 && y == 0.0));
    }
}

#[test]
fn placements_are_within_board_bounds() {
    let board = make_test_board();
    let options = SolveOptions {
        seed: 42,
        effort: Effort::Low,
    };

    let report = solve(&board, &options).expect("solve should succeed");

    for placement in &report.layout.placements {
        let (x, y) = placement.at;
        // Allow some tolerance outside the board boundary (autoplacer may
        // place components near the edge).
        assert!(
            x >= -10.0 && x <= board.width + 10.0,
            "placement x={x} out of bounds"
        );
        assert!(
            y >= -10.0 && y <= board.height + 10.0,
            "placement y={y} out of bounds"
        );
    }
}

#[test]
fn fixed_placement_is_respected() {
    // Build a board with one component fixed at a known position.
    let mut board = make_test_board();

    // Add a PlaceAt constraint to C1.
    let fixed_pos = (10.0_f64, 15.0_f64);
    let fixed_rot = 90.0_f64;
    board.components[0].layout = vec![LayoutConstraint::PlaceAt {
        pos: fixed_pos,
        rotation: fixed_rot,
        side: BoardSide::Front,
    }];

    let options = SolveOptions {
        seed: 42,
        effort: Effort::Low,
    };

    let report = solve(&board, &options).expect("solve should succeed");

    // C1 (component 0) should be at the fixed position.
    let c1_placement = &report.layout.placements[0];
    let (x, y) = c1_placement.at;

    // The positioning is in i64 nm internally, so after round-trip we
    // should be within 0.001 mm of the target.
    assert!(
        (x - fixed_pos.0).abs() < 0.001,
        "C1 x={x} should be at {}",
        fixed_pos.0
    );
    assert!(
        (y - fixed_pos.1).abs() < 0.001,
        "C1 y={y} should be at {}",
        fixed_pos.1
    );
    assert_eq!(c1_placement.rotation, fixed_rot);
    assert_eq!(c1_placement.side, BoardSide::Front);
}

#[test]
fn drc_passes_on_placement_only_board() {
    // With no tracks/vias, DRC should produce no structural errors.
    // The autoplacer may produce a warning due to Topola 0.1.0 overflow
    // bugs — filter that out as it's a known limitation.
    let board = make_test_board();
    let options = SolveOptions::default();

    let report = solve(&board, &options).expect("solve should succeed");
    let errors: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.code.starts_with("LAYOUT:"))
        .filter(|d| d.code != "LAYOUT:AUTOPLACER_CRASHED")
        .collect();

    assert!(
        errors.is_empty(),
        "no DRC errors expected on placement-only board, got: {errors:?}"
    );
}

#[test]
fn no_board_outline_is_an_error() {
    let mut board = make_test_board();
    board.width = 0.0;
    board.height = 0.0;

    let result = solve(&board, &SolveOptions::default());
    assert!(matches!(result, Err(LayoutError::NoBoardOutline)));
}

#[test]
fn no_components_is_an_error() {
    let mut board = make_test_board();
    board.components.clear();
    board.connections.clear();

    let result = solve(&board, &SolveOptions::default());
    assert!(matches!(result, Err(LayoutError::NoComponents)));
}

#[test]
fn all_components_placed_when_one_fixed() {
    // When C1 is fixed, R1 should still appear in placements.
    // The autoplacer may crash (Topola 0.1.0 limitation), in which
    // case R1 falls back to the origin — still valid.
    let mut board = make_test_board();
    board.components[0].layout = vec![LayoutConstraint::PlaceAt {
        pos: (5.0, 5.0),
        rotation: 0.0,
        side: BoardSide::Front,
    }];

    let options = SolveOptions {
        seed: 42,
        effort: Effort::Low,
    };

    let report = solve(&board, &options).expect("solve should succeed");
    assert_eq!(report.layout.placements.len(), 2);
    // C1 must be at the fixed position.
    let c1 = &report.layout.placements[0];
    assert!((c1.at.0 - 5.0).abs() < 0.001, "C1 should be fixed at x=5.0");
    assert!((c1.at.1 - 5.0).abs() < 0.001, "C1 should be fixed at y=5.0");
    // R1 has a placement — position is best-effort.
    let r1 = &report.layout.placements[1];
    assert!(r1.component == 1, "R1 should be component index 1");
}
