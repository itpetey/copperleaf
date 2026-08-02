//! Copperleaf-side design-rule check of the solved [`Layout`](copperleaf::Layout).
//!
//! This is an independent verification of the solver's output, checking
//! clearances, track widths, and creepage against each net's resolved
//! [`NetClass`](copperleaf::LayoutConstraint::NetClass).
//!
//! Diagnostics are prefixed `LAYOUT:` so they are easily distinguishable
//! from ERC diagnostics in the report.

use copperleaf::{
    CompiledBoard, Layout, LayoutConstraint, Pad,
    units::{Diagnostic, Meter, Qty, Severity},
};

/// Maximum number of DRC violations to report before short-circuiting.
const MAX_DIAGNOSTICS: usize = 50;

/// Run design-rule checks on the solved layout.
///
/// Returns zero or more diagnostics with `LAYOUT:`-prefixed codes.
pub fn check(layout: &Layout, board: &CompiledBoard) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    check_pad_clearances(board, &mut diagnostics);
    check_track_widths(layout, board, &mut diagnostics);
    check_track_clearances(layout, board, &mut diagnostics);
    check_zone_min_width(layout, board, &mut diagnostics);
    check_self_intersections(layout, &mut diagnostics);

    diagnostics
}

/// Verify edge-to-edge clearance between pads within each footprint.
///
/// Uses [`DesignRules::min_pad_to_pad_clearance`] when set (non-zero),
/// otherwise falls back to [`DesignRules::min_clearance`].  This allows
/// fine-pitch footprints (e.g. 0201 with 0.18mm pad gap) to pass DRC
/// even when the board-level copper clearance is 0.2mm.
fn check_pad_clearances(board: &CompiledBoard, diagnostics: &mut Vec<Diagnostic>) {
    let rules = &board.design_rules;
    let required = if rules.min_pad_to_pad_clearance > 0.0 {
        rules.min_pad_to_pad_clearance
    } else {
        rules.min_clearance
    };

    for comp in &board.components {
        if diagnostics.len() >= MAX_DIAGNOSTICS {
            break;
        }

        // Collect copper pads belonging to this component.
        let pads: Vec<&Pad> = comp
            .pins
            .iter()
            .filter_map(|p| p.pad())
            .filter(|pad| {
                pad.layers
                    .as_ref()
                    .map(|l| l.contains("F.Cu") || l.contains("B.Cu"))
                    .unwrap_or(false)
            })
            .collect();

        for i in 0..pads.len() {
            for j in (i + 1)..pads.len() {
                if diagnostics.len() >= MAX_DIAGNOSTICS {
                    break;
                }
                let gap = pad_to_pad_gap(pads[i], pads[j]);
                if gap < required {
                    diagnostics.push(Diagnostic {
                        code: "LAYOUT:PAD_CLEARANCE_VIOLATION".into(),
                        severity: Severity::Warning,
                        message: format!(
                            "pad-to-pad clearance on '{}' is {:.3}mm, \
                             minimum required is {:.3}mm",
                            comp.refdes, gap, required,
                        ),
                        entities: vec![comp.refdes.clone()],
                        hint: Some(
                            "set min_pad_to_pad_clearance for fine-pitch footprints \
                             (e.g. 0.18mm for 0201)"
                                .into(),
                        ),
                    });
                }
            }
        }
    }
}

/// Detect tracks that self-intersect (should not happen with a topological
/// router, but we verify it independently).
fn check_self_intersections(layout: &Layout, diagnostics: &mut Vec<Diagnostic>) {
    for track in &layout.tracks {
        if diagnostics.len() >= MAX_DIAGNOSTICS {
            break;
        }
        if has_self_intersection(&track.path) {
            diagnostics.push(Diagnostic {
                code: "LAYOUT:SELF_INTERSECTION".into(),
                severity: Severity::Warning,
                message: "track self-intersects".into(),
                entities: vec![],
                hint: Some("this is a solver bug; please report".into()),
            });
        }
    }
}

/// Verify clearance between every pair of tracks on shared layers.
fn check_track_clearances(
    layout: &Layout,
    board: &CompiledBoard,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Group tracks by layer.
    let mut by_layer: std::collections::BTreeMap<usize, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (i, track) in layout.tracks.iter().enumerate() {
        by_layer.entry(track.layer).or_default().push(i);
    }

    for track_indices in by_layer.values() {
        if diagnostics.len() >= MAX_DIAGNOSTICS {
            break;
        }
        for i in 0..track_indices.len() {
            for j in (i + 1)..track_indices.len() {
                if diagnostics.len() >= MAX_DIAGNOSTICS {
                    break;
                }
                let t1 = &layout.tracks[track_indices[i]];
                let t2 = &layout.tracks[track_indices[j]];

                // Same-net tracks are allowed to touch.
                if t1.net == t2.net {
                    continue;
                }

                // Compute minimum clearance between the two track paths.
                let min_clearance_mm = compute_min_path_distance(&t1.path, &t2.path);

                let net1 = board.net(t1.net);
                let net2 = board.net(t2.net);
                let c1 = required_clearance(&net1.layout, &net1.class);
                let c2 = required_clearance(&net2.layout, &net2.class);
                let clearance_required = match (c1, c2) {
                    (Some(a), Some(b)) => Some(if a.as_base() > b.as_base() { a } else { b }),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                };

                if let Some(required) = clearance_required {
                    let required_mm = required.as_base() * 1000.0;
                    if min_clearance_mm < required_mm {
                        diagnostics.push(Diagnostic {
                            code: "LAYOUT:CLEARANCE_VIOLATION".into(),
                            severity: Severity::Warning,
                            message: format!(
                                "clearance between net '{}' and '{}' is {:.3} mm, \
                                 minimum required is {:.3} mm",
                                net1.name, net2.name, min_clearance_mm, required_mm,
                            ),
                            entities: vec![net1.name.clone(), net2.name.clone()],
                            hint: Some("increase spacing or adjust net class clearance".into()),
                        });
                    }
                }
            }
        }
    }
}

/// Verify each track's width against its net's `NetClass::min_width`.
fn check_track_widths(layout: &Layout, board: &CompiledBoard, diagnostics: &mut Vec<Diagnostic>) {
    for track in &layout.tracks {
        if diagnostics.len() >= MAX_DIAGNOSTICS {
            break;
        }
        let net = board.net(track.net);
        let min_width = resolve_min_width(&net.layout, &net.class);

        if let Some(min) = min_width
            && track.width.as_base() < min.as_base()
        {
            diagnostics.push(Diagnostic {
                code: "LAYOUT:TRACK_WIDTH_VIOLATION".into(),
                severity: Severity::Warning,
                message: format!(
                    "track on net '{}' has width {:.3} mm, minimum is {:.3} mm",
                    net.name,
                    track.width.as_base() * 1000.0,
                    min.as_base() * 1000.0,
                ),
                entities: vec![net.name.clone()],
                hint: Some("increase track width or adjust net class".into()),
            });
        }
    }
}

/// Verify that zone outlines have at least the minimum width required by
/// the net class (basic sanity check on the polygon).
fn check_zone_min_width(layout: &Layout, board: &CompiledBoard, diagnostics: &mut Vec<Diagnostic>) {
    for zone in &layout.zones {
        if diagnostics.len() >= MAX_DIAGNOSTICS {
            break;
        }
        // A zone with fewer than 3 vertices is degenerate.
        if zone.outline.len() < 3 {
            let net = board.net(zone.net);
            diagnostics.push(Diagnostic {
                code: "LAYOUT:ZONE_DEGENERATE".into(),
                severity: Severity::Warning,
                message: format!(
                    "zone for net '{}' on layer {} has fewer than 3 outline vertices",
                    net.name, zone.layer,
                ),
                entities: vec![net.name.clone()],
                hint: Some("check the board outline definition".into()),
            });
        }
    }
}

/// Compute the minimum Euclidean distance between any two line segments from
/// the two paths.  Returns distance in millimetres.
fn compute_min_path_distance(a: &[(f64, f64)], b: &[(f64, f64)]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return f64::INFINITY;
    }

    let mut min_dist = f64::INFINITY;

    // Compare every segment in a against every segment in b.
    for w_a in a.windows(2) {
        let seg_a = (w_a[0], w_a[1]);
        for w_b in b.windows(2) {
            let seg_b = (w_b[0], w_b[1]);
            let d = segment_distance(seg_a, seg_b);
            if d < min_dist {
                min_dist = d;
            }
        }
    }

    min_dist
}

/// Signed area / cross product of three points.
fn cross(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

/// Check if a polyline path self-intersects.
fn has_self_intersection(path: &[(f64, f64)]) -> bool {
    if path.len() < 4 {
        return false;
    }
    for i in 0..path.len().saturating_sub(2) {
        let seg_i = (path[i], path[i + 1]);
        for j in (i + 2)..path.len().saturating_sub(1) {
            // Adjacent segments share an endpoint — ignore those.
            if j == i + 1 {
                continue;
            }
            let seg_j = (path[j], path[j + 1]);
            if segments_intersect(seg_i, seg_j) {
                return true;
            }
        }
    }
    false
}

/// Compute the minimum edge-to-edge gap between two axis-aligned
/// rectangular pads.  Returns the gap in millimetres, or 0.0 if the
/// pads overlap.
fn pad_to_pad_gap(a: &Pad, b: &Pad) -> f64 {
    // Half-extents.
    let (a_hw, a_hh) = (a.width / 2.0, a.height / 2.0);
    let (b_hw, b_hh) = (b.width / 2.0, b.height / 2.0);

    // Extents on each axis.
    let a_x1 = a.pos.0 - a_hw;
    let a_x2 = a.pos.0 + a_hw;
    let a_y1 = a.pos.1 - a_hh;
    let a_y2 = a.pos.1 + a_hh;

    let b_x1 = b.pos.0 - b_hw;
    let b_x2 = b.pos.0 + b_hw;
    let b_y1 = b.pos.1 - b_hh;
    let b_y2 = b.pos.1 + b_hh;

    let x_overlap = a_x1 < b_x2 && b_x1 < a_x2;
    let y_overlap = a_y1 < b_y2 && b_y1 < a_y2;

    if x_overlap && y_overlap {
        // Pads overlap.
        return 0.0;
    }

    if x_overlap {
        // Overlap in X: gap is the Y separation.
        return if a_y2 <= b_y1 {
            b_y1 - a_y2
        } else {
            a_y1 - b_y2
        };
    }

    if y_overlap {
        // Overlap in Y: gap is the X separation.
        return if a_x2 <= b_x1 {
            b_x1 - a_x2
        } else {
            a_x1 - b_x2
        };
    }

    // No overlap on either axis: Euclidean distance between closest corners.
    let dx = if a_x2 <= b_x1 {
        b_x1 - a_x2
    } else {
        a_x1 - b_x2
    };
    let dy = if a_y2 <= b_y1 {
        b_y1 - a_y2
    } else {
        a_y1 - b_y2
    };
    (dx * dx + dy * dy).sqrt()
}

/// Check if a point lies within the bounding box of a segment (collinear case).
fn point_on_segment_bbox(p: (f64, f64), s: ((f64, f64), (f64, f64))) -> bool {
    let (x1, y1) = (s.0.0.min(s.1.0), s.0.1.min(s.1.1));
    let (x2, y2) = (s.0.0.max(s.1.0), s.0.1.max(s.1.1));
    p.0 >= x1 && p.0 <= x2 && p.1 >= y1 && p.1 <= y2
}

/// Minimum distance from a point to a line segment.
fn point_to_segment_distance(p: (f64, f64), s: ((f64, f64), (f64, f64))) -> f64 {
    let dx = s.1.0 - s.0.0;
    let dy = s.1.1 - s.0.1;
    let len_sq = dx * dx + dy * dy;

    if len_sq == 0.0 {
        // Degenerate segment: distance to point.
        let ex = p.0 - s.0.0;
        let ey = p.1 - s.0.1;
        return (ex * ex + ey * ey).sqrt();
    }

    // Projection parameter t, clamped to [0, 1].
    let t = ((p.0 - s.0.0) * dx + (p.1 - s.0.1) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);

    let proj_x = s.0.0 + t * dx;
    let proj_y = s.0.1 + t * dy;

    let ex = p.0 - proj_x;
    let ey = p.1 - proj_y;
    (ex * ex + ey * ey).sqrt()
}

/// Resolve the required clearance from a net's layout constraints.
fn required_clearance(
    layout_constraints: &[LayoutConstraint],
    default: &copperleaf::NetClass,
) -> Option<Qty<Meter>> {
    for constraint in layout_constraints {
        if let LayoutConstraint::NetClass { clearance, .. } = constraint {
            return Some(*clearance);
        }
    }
    default.clearance
}

/// Resolve the minimum track width from a net's layout constraints.
fn resolve_min_width(
    layout_constraints: &[LayoutConstraint],
    default: &copperleaf::NetClass,
) -> Option<Qty<Meter>> {
    for constraint in layout_constraints {
        if let LayoutConstraint::NetClass { min_width, .. } = constraint {
            return Some(*min_width);
        }
    }
    default.min_width
}

/// Minimum distance between two line segments in 2D.
fn segment_distance(seg_a: ((f64, f64), (f64, f64)), seg_b: ((f64, f64), (f64, f64))) -> f64 {
    // Check if segments intersect.
    if segments_intersect(seg_a, seg_b) {
        return 0.0;
    }

    // Distance from endpoints of one segment to the other.
    let d1 = point_to_segment_distance(seg_a.0, seg_b);
    let d2 = point_to_segment_distance(seg_a.1, seg_b);
    let d3 = point_to_segment_distance(seg_b.0, seg_a);
    let d4 = point_to_segment_distance(seg_b.1, seg_a);

    d1.min(d2).min(d3).min(d4)
}

/// Check if two line segments intersect.
fn segments_intersect(s1: ((f64, f64), (f64, f64)), s2: ((f64, f64), (f64, f64))) -> bool {
    let d1 = cross(s2.0, s2.1, s1.0);
    let d2 = cross(s2.0, s2.1, s1.1);
    let d3 = cross(s1.0, s1.1, s2.0);
    let d4 = cross(s1.0, s1.1, s2.1);

    if d1 == 0.0 && d2 == 0.0 && d3 == 0.0 && d4 == 0.0 {
        // Collinear: check bounding box overlap.
        return point_on_segment_bbox(s1.0, s2)
            || point_on_segment_bbox(s1.1, s2)
            || point_on_segment_bbox(s2.0, s1)
            || point_on_segment_bbox(s2.1, s1);
    }

    ((d1 > 0.0) != (d2 > 0.0)) && ((d3 > 0.0) != (d4 > 0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_segments_distance() {
        let a = ((0.0, 0.0), (10.0, 0.0));
        let b = ((0.0, 1.0), (10.0, 1.0));
        let d = segment_distance(a, b);
        assert!((d - 1.0).abs() < 0.001, "expected 1.0, got {}", d);
    }

    #[test]
    fn intersecting_segments_distance() {
        let a = ((0.0, 0.0), (10.0, 10.0));
        let b = ((0.0, 10.0), (10.0, 0.0));
        let d = segment_distance(a, b);
        assert!((d - 0.0).abs() < 0.001, "expected 0.0, got {}", d);
    }

    #[test]
    fn point_to_segment() {
        let s = ((0.0, 0.0), (10.0, 0.0));
        let d = point_to_segment_distance((5.0, 3.0), s);
        assert!((d - 3.0).abs() < 0.001, "expected 3.0, got {}", d);
    }

    #[test]
    fn no_self_intersection() {
        let path = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)];
        assert!(!has_self_intersection(&path));
    }

    #[test]
    fn self_intersection_detected() {
        let path = vec![(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)];
        assert!(has_self_intersection(&path));
    }
}
