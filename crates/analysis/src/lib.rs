//! Analysis passes for Copperleaf.
//!
//! Contains early ERC (electrical rule check) helpers and traits for future
//! verification passes. This crate focuses on pure functions over the IR.

use copperleaf_core::{Diagnostic, Severity};
use copperleaf_ir::{Net, NetKind, Pin};

/// Simple ERC: flag when a pin's maximum voltage is below a power net's nominal voltage.
///
/// Returns a [`Diagnostic`] describing the violation when overvoltage is detected,
/// otherwise `None`.
pub fn erc_voltage_pin_to_net(net: &Net, pin: &Pin) -> Option<Diagnostic> {
    match net.kind {
        NetKind::Power { v_nom, .. } => {
            if v_nom.as_base() > pin.limits.v_max.as_base() + 1e-9 {
                return Some(Diagnostic {
                    code: "ERC:OVERVOLT".into(),
                    severity: Severity::Error,
                    message: format!(
                        "Pin {} max {:.2}V, connected to {:.2}V net",
                        pin.name,
                        pin.limits.v_max.as_base(),
                        v_nom.as_base()
                    ),
                    entities: vec![pin.name.into(), net.name.clone()],
                    hint: Some("Use a level shifter or different pin".into()),
                });
            }
            None
        }
        _ => None,
    }
}

// Placeholder for a constraint registry trait
/// Trait for analysis passes to expose a stable name.
pub trait CheckPass {
    fn name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf_core::UnitExt;
    use copperleaf_ir::{Limits, Net, Pin, Role};

    #[test]
    fn overvoltage_detected() {
        let net = Net::power("VBUS", 5.0.volt());
        let pin = Pin {
            name: "VDD",
            role: Role::PowerIn,
            limits: Limits {
                v_min: 1.7.volt(),
                v_max: 3.6.volt(),
                i_max: 0.1.amp(),
            },
            sig: None,
        };
        assert!(erc_voltage_pin_to_net(&net, &pin).is_some());
    }
}
