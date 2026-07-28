/// Errors that can occur during layout solving.
#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    /// The board has no outline (board boundary cannot be determined).
    #[error("board has no outline polygon")]
    NoBoardOutline,
    /// The board has no components to place.
    #[error("board has no components")]
    NoComponents,
    /// Internal error in the solver engine.
    #[error("solver internal error: {0}")]
    Internal(String),
}
