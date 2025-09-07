//! KiCad backend (placeholder).
//!
//! Provides a minimal text netlist emitter to aid experimentation. The
//! format is not stable and is intended for demos and tests only.

use copperleaf_ir::Design;

/// Emit a toy KiCad-like netlist as a string for the given design.
pub fn emit_netlist_text(design: &Design) -> String {
    let mut s = String::new();
    s.push_str("# KiCad-like netlist (placeholder)\n");
    for net in &design.nets {
        s.push_str(&format!("(net \"{}\")\n", net.name));
    }
    s
}
