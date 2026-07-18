//! Connector vendor components
//!
//! Provides parametric connector types that can be instantiated with an
//! arbitrary number of pins at runtime.  Each connector family has a TOML
//! manifest defining mechanical parameters and a single-pin template; the Rust
//! struct repeats that template N times at the given pitch.
//!
//! Fixed-definition parts generated from TOML manifests via
//! `build_component!` coexist alongside parametric types.

use copperleaf_part_macro::build_component;

pub use jst_ph::JstPh;

mod connector;
mod jst_ph;

build_component!("arjm11d7_502_ab_ew2.toml");

build_component!("usb_c_2340901_1.toml");

build_component!("conmhf4_smd_g_t.toml");
