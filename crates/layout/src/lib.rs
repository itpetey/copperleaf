//! Layout solving for [`CompiledBoard`]s.
//!
//! This crate provides a deterministic force-directed autoplacer that
//! produces a [`Layout`](copperleaf::Layout) with component placements and
//! plane zones. The public entry point is [`solve()`].
//!
//! Autorouting is not yet included. Route nets manually in KiCad or with
//! an external autorouter.
//!
//! # Quick start
//!
//! ```ignore
//! use copperleaf_layout::{solve, SolveOptions, Effort};
//!
//! let board = /* compile a Board */;
//! let options = SolveOptions { seed: 42, effort: Effort::Medium };
//! let report = solve(&board, &options)?;
//!
//! // Pass the layout to a backend for emission:
//! backend.emit_with_layout("output/", &board, &report.layout)?;
//! ```
//!
//! After emission, open the `.kicad_pcb` in KiCad and refill zones
//! (`Edit` → `Fill All Zones`, or `kicad-cli pcb refill` for CI) to
//! finalise plane pours.
//!
//! # Module layout
//!
//! - [`translate`] — `CompiledBoard` → placer input model.
//! - [`placer`] — force-directed placement engine (no external deps).
//! - [`drc`] — copperleaf-side design-rule check of the solved layout.
//! - [`report`] — [`LayoutReport`] output type.
//! - [`error`] — [`LayoutError`] error type.

use copperleaf::CompiledBoard;

mod drc;
pub mod error;
mod placer;
pub mod report;
mod translate;

pub use error::LayoutError;
pub use report::LayoutReport;

/// Solver effort level (controls placement iteration count).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effort {
    /// Fast — fewer iterations.
    Low,
    /// Balanced — default.
    Medium,
    /// Thorough — more iterations.
    High,
}

/// Options controlling the layout solver.
#[derive(Clone, Copy, Debug)]
pub struct SolveOptions {
    /// Random seed (reserved for future use; the placer is deterministic).
    pub seed: u64,
    /// Placement effort level.
    pub effort: Effort,
}

impl Default for SolveOptions {
    fn default() -> Self {
        Self {
            seed: 42,
            effort: Effort::Medium,
        }
    }
}

/// Solve placement for a compiled board.
///
/// Returns a [`LayoutReport`] containing the solved [`Layout`](copperleaf::Layout)
/// with placements and plane zones, plus any diagnostics.
///
/// # Errors
///
/// Returns [`LayoutError`] only for fundamental problems (no board outline,
/// no components).
pub fn solve(board: &CompiledBoard, options: &SolveOptions) -> Result<LayoutReport, LayoutError> {
    // 1. Translate CompiledBoard → placer input model.
    let input = translate::translate_board(board)?;

    // 2. Run force-directed placement.
    let (layout, diagnostics) = placer::place(&input, options);

    // 3. Run copperleaf-side DRC.
    let mut all_diagnostics = diagnostics;
    let drc_diags = drc::check(&layout, board);
    all_diagnostics.extend(drc_diags);

    Ok(LayoutReport {
        layout,
        diagnostics: all_diagnostics,
    })
}
