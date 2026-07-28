//! Force-directed placement engine.
//!
//! Components repel each other (like charged particles), components sharing a
//! net attract, fixed components are anchored, and board edges push inward.
//! The algorithm is deterministic for a given input and seed.

use std::collections::HashMap;

use copperleaf::{
    BoardSide, Layout, LayoutConstraint, NetIdx, Placement,
    units::Diagnostic,
};

use crate::{SolveOptions, translate::AdapterInput};

// ---------------------------------------------------------------------------
// Placer state
// ---------------------------------------------------------------------------

/// One component tracked by the placer.
struct Component {
    /// Index into the original `AdapterInput.components`.
    index: usize,
    /// Whether this component has a `PlaceAt` constraint (anchored).
    fixed: bool,
    /// Rotation from `PlaceAt`.
    rotation: f64,
    /// Board side from `PlaceAt`.
    side: BoardSide,
    /// Current position (updated each iteration).
    x: f64,
    y: f64,
    /// Bounding-box half-extents computed from pads, or a default.
    half_w: f64,
    half_h: f64,
}

/// Force-directed placement of components.
///
/// Returns a [`Layout`] with placements for every component plus zone data.
pub fn place(input: &AdapterInput, _options: &SolveOptions) -> (Layout, Vec<Diagnostic>) {
    let diagnostics = Vec::new();

    // Determine board dimensions from the outline.
    let (board_w, board_h) = board_size(&input.outline);

    // Build the set of nets → component indices for attraction.
    let net_to_comps = build_net_map(input);

    // Initialise component state.
    let mut comps: Vec<Component> = input
        .components
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let (fixed, anchor, rotation, side) = fixed_constraint(&c.layout);
            let (half_w, half_h) = component_half_extents(c);
            Component {
                index: i,
                fixed,
                rotation,
                side,
                x: if fixed { anchor.0 } else { 0.0 },
                y: if fixed { anchor.1 } else { 0.0 },
                half_w,
                half_h,
            }
        })
        .collect();

    // Initial placement for non-fixed components: spread along a row.
    let mut next_x = 0.0_f64;
    for comp in comps.iter_mut() {
        if comp.fixed {
            continue;
        }
        comp.x = next_x;
        comp.y = 0.0;
        next_x += comp.half_w * 2.0 + 2.0; // 2 mm gap
        if next_x > board_w - comp.half_w {
            next_x = 0.0;
        }
    }

    // Force-directed iteration.
    let max_iter = 200;
    let mut step = board_w.max(board_h) * 0.1;
    let min_step = 0.001;

    for _iter in 0..max_iter {
        // Compute forces on each non-fixed component.
        let mut forces: Vec<(f64, f64)> = vec![(0.0, 0.0); comps.len()];

        // Repulsion: every pair of components pushes apart.
        for i in 0..comps.len() {
            if comps[i].fixed {
                continue;
            }
            for j in 0..comps.len() {
                if i == j {
                    continue;
                }
                let dx = comps[i].x - comps[j].x;
                let dy = comps[i].y - comps[j].y;
                let dist_sq = dx * dx + dy * dy;
                let min_dist = comps[i].half_w + comps[j].half_w + 1.0; // 1 mm min gap
                let min_dist_sq = min_dist * min_dist;
                if dist_sq < min_dist_sq && dist_sq > 0.0 {
                    let dist = dist_sq.sqrt();
                    let force = (min_dist - dist) / dist * 0.5;
                    forces[i].0 += dx * force;
                    forces[i].1 += dy * force;
                }
            }
        }

        // Attraction: components sharing a net pull toward their centroid.
        for (_net_idx, comp_indices) in &net_to_comps {
            if comp_indices.len() < 2 {
                continue;
            }
            // Compute centroid of component positions on this net.
            let mut cx = 0.0_f64;
            let mut cy = 0.0_f64;
            for &ci in comp_indices {
                cx += comps[ci].x;
                cy += comps[ci].y;
            }
            cx /= comp_indices.len() as f64;
            cy /= comp_indices.len() as f64;

            // Pull each component toward the centroid.
            for &ci in comp_indices {
                if comps[ci].fixed {
                    continue;
                }
                let dx = cx - comps[ci].x;
                let dy = cy - comps[ci].y;
                forces[ci].0 += dx * 0.1;
                forces[ci].1 += dy * 0.1;
            }
        }

        // Board-edge containment: push components inside.
        for comp in comps.iter_mut() {
            if comp.fixed {
                continue;
            }
            let margin = 1.0; // 1 mm from edge
            if comp.x < comp.half_w + margin {
                forces[comp.index].0 += (comp.half_w + margin - comp.x) * 1.0;
            }
            if comp.x > board_w - comp.half_w - margin {
                forces[comp.index].0 -= (comp.x - (board_w - comp.half_w - margin)) * 1.0;
            }
            if comp.y < comp.half_h + margin {
                forces[comp.index].1 += (comp.half_h + margin - comp.y) * 1.0;
            }
            if comp.y > board_h - comp.half_h - margin {
                forces[comp.index].1 -= (comp.y - (board_h - comp.half_h - margin)) * 1.0;
            }
        }

        // Apply forces with damping.
        let mut max_move = 0.0_f64;
        for comp in comps.iter_mut() {
            if comp.fixed {
                continue;
            }
            let (fx, fy) = forces[comp.index];
            let move_x = fx * step;
            let move_y = fy * step;
            // Clamp per-step movement.
            let clamp = step * 2.0;
            let move_x = move_x.clamp(-clamp, clamp);
            let move_y = move_y.clamp(-clamp, clamp);
            comp.x += move_x;
            comp.y += move_y;
            let moved = (move_x * move_x + move_y * move_y).sqrt();
            if moved > max_move {
                max_move = moved;
            }
        }

        step *= 0.95;

        if max_move < min_step {
            break;
        }
    }

    // Build the output Layout.
    let placements: Vec<Placement> = comps
        .iter()
        .map(|c| Placement {
            component: c.index,
            at: (c.x, c.y),
            rotation: c.rotation,
            side: c.side,
        })
        .collect();

    let mut layout = Layout {
        placements,
        tracks: Vec::new(),
        vias: Vec::new(),
        zones: Vec::new(),
    };

    // Generate plane zones from input.
    generate_plane_zones(&mut layout, input);

    (layout, diagnostics)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract board width and height from the outline polygon.
fn board_size(outline: &[(f64, f64)]) -> (f64, f64) {
    let max_x = outline.iter().map(|p| p.0).fold(0.0_f64, f64::max);
    let max_y = outline.iter().map(|p| p.1).fold(0.0_f64, f64::max);
    (max_x, max_y)
}

/// Build a map from net index to the set of component indices connected
/// to that net.
fn build_net_map(input: &AdapterInput) -> HashMap<usize, Vec<usize>> {
    let mut map: HashMap<usize, Vec<usize>> = HashMap::new();
    for (comp_idx, comp) in input.components.iter().enumerate() {
        for pad in &comp.pads {
            if let Some(net) = pad.net {
                map.entry(net.0).or_default().push(comp_idx);
            }
        }
    }
    // Deduplicate component indices per net.
    for indices in map.values_mut() {
        indices.sort();
        indices.dedup();
    }
    map
}

/// Extract fixed-placement constraint from a component's layout directives.
/// Returns `(is_fixed, anchor_pos, rotation, side)`.
fn fixed_constraint(layout: &[LayoutConstraint]) -> (bool, (f64, f64), f64, BoardSide) {
    for c in layout {
        if let LayoutConstraint::PlaceAt { pos, rotation, side } = c {
            return (true, *pos, *rotation, *side);
        }
    }
    (false, (0.0, 0.0), 0.0, BoardSide::Front)
}

/// Compute approximate half-extents of a component from its pad positions.
fn component_half_extents(comp: &crate::translate::ComponentInfo) -> (f64, f64) {
    if comp.pads.is_empty() {
        return (2.0, 2.0); // default: 4×4 mm
    }

    let (min_x, min_y, max_x, max_y) = comp.pads.iter().fold(
        (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
        |(min_x, min_y, max_x, max_y), pad| {
            let x1 = pad.pos.0 - pad.width / 2.0;
            let y1 = pad.pos.1 - pad.height / 2.0;
            let x2 = pad.pos.0 + pad.width / 2.0;
            let y2 = pad.pos.1 + pad.height / 2.0;
            (
                min_x.min(x1),
                min_y.min(y1),
                max_x.max(x2),
                max_y.max(y2),
            )
        },
    );

    let w = ((max_x - min_x) / 2.0).max(1.0); // at least 1 mm half-width
    let h = ((max_y - min_y) / 2.0).max(1.0);
    (w, h)
}

/// Generate plane zones from input net data.
fn generate_plane_zones(layout: &mut Layout, input: &AdapterInput) {
    for (net_idx, net_info) in input.nets.iter().enumerate() {
        let layer = net_info.layout.iter().find_map(|c| {
            if let LayoutConstraint::Plane { layer } = c {
                Some(*layer)
            } else {
                None
            }
        });

        if let Some(layer) = layer {
            layout.zones.push(copperleaf::Zone {
                net: NetIdx(net_idx),
                layer,
                outline: input.outline.clone(),
            });
        }
    }
}
