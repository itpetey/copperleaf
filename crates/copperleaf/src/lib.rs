//! Copperleaf facade: unified public API.
//!
//! This crate re-exports types from the `core`, `ir`, `analysis`, `edsl`,
//! `parts`, and `backends` crates so applications can depend on a single crate.
//! Prefer importing from `copperleaf` rather than individual subcrates.

pub use copperleaf_analysis::*;
pub use copperleaf_backend_kicad as backend_kicad;
pub use copperleaf_core::*;
pub use copperleaf_edsl::*;
pub use copperleaf_parts as parts;
