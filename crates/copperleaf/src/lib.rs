//! Copperleaf facade: unified public API.
//!
//! This crate re-exports types from the `core`, `ir`, `analysis`, `edsl`,
//! `parts`, and `backends` crates so applications can depend on a single crate.
//! Prefer importing from `copperleaf` rather than individual subcrates.
//!
//! Getting started (minimal)
//! -------------------------
//!
//! ```
//! use copperleaf::{erc_voltage_pin_to_net, UnitExt, Net, Pin, Limits, Role};
//!
//! // Define a 3.3 V net and a power input pin that tolerates up to 3.6 V.
//! let v3v3 = Net::power("V3V3", 3.3.volt());
//! let vdd = Pin { name: "VDD".into(), role: Role::PowerIn,
//!     limits: Limits { v_min: 1.7.volt(), v_max: 3.6.volt(), i_max: 0.5.amp() },
//!     sig: None };
//!
//! // ERC: No error because 3.3 V <= 3.6 V max.
//! assert!(erc_voltage_pin_to_net(&v3v3, &vdd).is_none());
//! ```
//!
//! Building a design
//! -----------------
//!
//! ```
//! use copperleaf::{Design, Constraint, NetClass, UnitExt, Net};
//!
//! let mut d = Design::default();
//! let mut v3v3 = Net::power("V3V3", 3.3.volt());
//! v3v3.class = NetClass { min_width: Some(0.3.mm()), clearance: Some(0.2.mm()) };
//! d.add_net(v3v3);
//! d.add_constraint(Constraint::MaxJunction { temp: 85.0.celsius() });
//! ```

pub use copperleaf_analysis::*;
pub use copperleaf_backend_kicad as backend_kicad;
pub use copperleaf_core::*;
pub use copperleaf_edsl::*;
pub use copperleaf_parts as parts;
