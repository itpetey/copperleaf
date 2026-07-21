//! Electrical-rule checks (ERC) — pure validation passes.
//!
//! Every function in this module inspects an immutable [`CompiledBoard`] and
//! returns [`Diagnostic`]s.  None of them mutate the board or produce new
//! components — that is the job of the compilation pipeline in
//! [`copperleaf_compile`].
//!
//! [`run_erc`] is the single entry point used by the compilation pipeline;
//! the individual rule functions are kept `pub` so they can be unit-tested
//! in isolation.

use crate::{
    board::{BoardView, CompiledBoard},
    net::NetKind,
    pin::{Pin, Role},
    units::{Diagnostic, Severity},
};

/// Returns `true` if the pin is intentionally no-connect.
///
/// Checks the semantic `nc` flag first; falls back to name-prefix matching
/// for hand-written parts that use `NC`/`NC_*` naming convention.
fn is_nc_pin(pin: &Pin) -> bool {
    pin.nc() || pin.name() == "NC" || pin.name().starts_with("NC_")
}

/// ERC rule: flag DigitalIO/AnalogIn pins with no signal spec and no net connection.
pub fn erc_floating_inputs(view: &BoardView) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for (comp_idx, comp) in view.board.components.iter().enumerate() {
        for (pin_idx, pin) in comp.pins.iter().enumerate() {
            if is_nc_pin(pin) {
                continue;
            }
            if matches!(pin.role(), Role::DigitalIO | Role::AnalogIn)
                && pin.sig_spec().is_none()
                && !view.connected.contains(&(comp_idx, pin_idx))
            {
                diags.push(Diagnostic {
                    code: "ERC:FLOATING_INPUT".into(),
                    severity: Severity::Warning,
                    message: format!("Input pin {}.{} is floating", comp.refdes, pin.name()),
                    entities: vec![format!("{}.{}", comp.refdes, pin.name())],
                    hint: Some("Connect the pin or assign a signal specification".into()),
                });
            }
        }
    }
    diags
}

/// ERC rule: flag PowerIn pins with no net connection.
pub fn erc_floating_power_inputs(view: &BoardView) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for (comp_idx, comp) in view.board.components.iter().enumerate() {
        for (pin_idx, pin) in comp.pins.iter().enumerate() {
            if matches!(pin.role(), Role::PowerIn) && !view.connected.contains(&(comp_idx, pin_idx))
            {
                diags.push(Diagnostic {
                    code: "ERC:FLOATING_POWER_INPUT".into(),
                    severity: Severity::Warning,
                    message: format!(
                        "Power input pin {}.{} is unconnected",
                        comp.refdes,
                        pin.name()
                    ),
                    entities: vec![format!("{}.{}", comp.refdes, pin.name())],
                    hint: Some("Connect the pin to a power net".into()),
                });
            }
        }
    }
    diags
}

/// ERC rule: flag NC pins that are connected to a net.
pub fn erc_nc_pin_connected(view: &BoardView) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for (comp_idx, comp) in view.board.components.iter().enumerate() {
        for (pin_idx, pin) in comp.pins.iter().enumerate() {
            if is_nc_pin(pin) && view.connected.contains(&(comp_idx, pin_idx)) {
                diags.push(Diagnostic {
                    code: "ERC:NC_CONNECTED".into(),
                    severity: Severity::Error,
                    message: format!(
                        "NC pin {}.{} is connected to a net",
                        comp.refdes,
                        pin.name()
                    ),
                    entities: vec![format!("{}.{}", comp.refdes, pin.name())],
                    hint: Some("Leave no-connect pins unconnected".into()),
                });
            }
        }
    }
    diags
}

/// ERC rule: flag PowerIn pins connected to a net with voltage exceeding v_max.
pub fn erc_overvoltage(view: &BoardView) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for (comp_idx, comp) in view.board.components.iter().enumerate() {
        for (pin_idx, pin) in comp.pins.iter().enumerate() {
            if !matches!(pin.role(), Role::PowerIn) {
                continue;
            }
            if let Some(&net_idx) = view.net_of.get(&(comp_idx, pin_idx))
                && let net = view.board.net(net_idx)
                && let NetKind::Power { v_nom, .. } = net.kind
                && v_nom.as_base() > pin.power_spec().v_max.as_base() + 1e-9
            {
                diags.push(Diagnostic {
                    code: "ERC:OVERVOLT".into(),
                    severity: Severity::Error,
                    message: format!(
                        "Pin {}.{} max {:.2}V, connected to {:.2}V net {}",
                        comp.refdes,
                        pin.name(),
                        pin.power_spec().v_max.as_base(),
                        v_nom.as_base(),
                        net.name
                    ),
                    entities: vec![format!("{}.{}", comp.refdes, pin.name()), net.name.clone()],
                    hint: Some("Use a level shifter or different pin".into()),
                });
            }
        }
    }
    diags
}

/// Run every ERC validation pass against a compiled board.
///
/// Returns `(warnings, errors)`.  Errors are fatal and short-circuit the
/// pipeline; warnings are informational and always collected.
pub fn run_erc(board: &CompiledBoard) -> (Vec<Diagnostic>, Vec<Diagnostic>) {
    let view = BoardView::new(board);
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    warnings.extend(erc_floating_inputs(&view));
    warnings.extend(erc_floating_power_inputs(&view));
    errors.extend(erc_overvoltage(&view));
    errors.extend(erc_nc_pin_connected(&view));

    (warnings, errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        board::{BoardView, CompiledComponent, Connection},
        net::{Net, NetClass, NetIdx},
        pin::Pin,
        units::UnitExt,
    };

    #[test]
    fn overvoltage_detected() {
        let board = CompiledBoard {
            components: vec![CompiledComponent::test(
                "U1",
                vec![Pin::build("VDD").pwr_fixed(3.3.volt(), 0.1.amp()).pin()],
            )],
            nets: vec![Net {
                name: "VBUS".into(),
                kind: NetKind::Power {
                    v_nom: 5.0.volt(),
                    ripple: None,
                },
                class: NetClass::default(),
                constraints: vec![],
            }],
            connections: vec![Connection {
                component: 0,
                pin: "VDD".into(),
                net: NetIdx(0),
            }],
            constraints: vec![],
            width: 100.0,
            height: 80.0,
        };
        let view = BoardView::new(&board);
        let diags = erc_overvoltage(&view);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "ERC:OVERVOLT");
    }

    #[test]
    fn nc_pin_connected_flags_connected_nc_pin() {
        let board = CompiledBoard {
            components: vec![CompiledComponent::test("U1", vec![Pin::build("NC").dio()])],
            nets: vec![Net {
                name: "NET".into(),
                kind: NetKind::Power {
                    v_nom: 3.3.volt(),
                    ripple: None,
                },
                class: NetClass::default(),
                constraints: vec![],
            }],
            connections: vec![Connection {
                component: 0,
                pin: "NC".into(),
                net: NetIdx(0),
            }],
            constraints: vec![],
            width: 100.0,
            height: 80.0,
        };
        let view = BoardView::new(&board);
        let diags = erc_nc_pin_connected(&view);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "ERC:NC_CONNECTED");
    }

    #[test]
    fn floating_input_flags_unconnected_digital_io() {
        let board = CompiledBoard {
            components: vec![CompiledComponent::test(
                "U1",
                vec![Pin::build("GPIO").dio()],
            )],
            nets: vec![],
            connections: vec![],
            constraints: vec![],
            width: 100.0,
            height: 80.0,
        };
        let view = BoardView::new(&board);
        let diags = erc_floating_inputs(&view);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "ERC:FLOATING_INPUT");
    }
}
