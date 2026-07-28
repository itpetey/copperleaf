use copperleaf::{Layout, units::Diagnostic};

/// The result of a successful layout solve.
#[derive(Clone, Debug)]
pub struct LayoutReport {
    /// The solved physical layout (placements and zones).
    pub layout: Layout,
    /// Diagnostics produced during solving and internal DRC.
    pub diagnostics: Vec<Diagnostic>,
}
