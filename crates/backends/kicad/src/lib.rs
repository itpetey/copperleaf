//! KiCad backend: netlist, schematic, and PCB emitters for the Copperleaf IR.

pub mod common;
pub mod netlist;
pub mod pcb;
pub mod project;
pub mod schematic;
pub mod sexpr;

pub use common::{build_net_codes, fmt_mm, format_float, refdes_prefix};
pub use netlist::emit_netlist;
pub use pcb::emit_pcb;
pub use project::emit_project;
pub use schematic::emit_schematic;
pub use sexpr::{Sexpr, deterministic_uuid, kv};
