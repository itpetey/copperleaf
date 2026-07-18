//! Electrical-rule checks (ERC) — pure validation passes.
//!
//! Every function in this module inspects an immutable [`CompiledBoard`] and
//! returns [`Diagnostic`]s.  None of them mutate the board or produce new
//! components — that is the job of the [`synth`](crate::synth) module.
//!
//! [`run_erc`] is the single entry point used by the compilation pipeline;
//! the individual rule functions are kept `pub` so they can be unit-tested
//! in isolation.

use crate::{
    board::CompiledBoard,
    net::NetKind,
    pin::Role,
    units::{Diagnostic, Severity},
};

/// ERC rule: flag DigitalIO/AnalogIn pins with no signal spec and no net connection.
pub fn erc_floating_inputs(board: &CompiledBoard) -> Vec<Diagnostic> {
    let connected = connected_pins(board);
    let mut diags = Vec::new();
    for comp in &board.components {
        for pin in &comp.pins {
            if pin.name() == "NC" || pin.name().starts_with("NC_") {
                continue;
            }
            if matches!(pin.role(), Role::DigitalIO | Role::AnalogIn)
                && pin.sig_spec().is_none()
                && !connected.contains(&(comp.refdes.as_str(), pin.name()))
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
pub fn erc_floating_power_inputs(board: &CompiledBoard) -> Vec<Diagnostic> {
    let connected = connected_pins(board);
    let mut diags = Vec::new();
    for comp in &board.components {
        for pin in &comp.pins {
            if matches!(pin.role(), Role::PowerIn)
                && !connected.contains(&(comp.refdes.as_str(), pin.name()))
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
pub fn erc_nc_pin_connected(board: &CompiledBoard) -> Vec<Diagnostic> {
    let connected = connected_pins(board);
    let mut diags = Vec::new();
    for comp in &board.components {
        for pin in &comp.pins {
            if (pin.name() == "NC" || pin.name().starts_with("NC_"))
                && connected.contains(&(comp.refdes.as_str(), pin.name()))
            {
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
pub fn erc_overvoltage(board: &CompiledBoard) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for comp in &board.components {
        for pin in &comp.pins {
            if !matches!(pin.role(), Role::PowerIn) {
                continue;
            }
            for conn in &board.connections {
                if conn.component == component_index(&comp.refdes, board)
                    && conn.pin == pin.name()
                    && let Some(net) = board.nets.iter().find(|n| n.name == conn.net.0)
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
    }
    diags
}

/// Run every ERC validation pass against a compiled board.
///
/// Returns `(warnings, errors)`.  Errors are fatal and short-circuit the
/// pipeline; warnings are informational and always collected.
pub fn run_erc(board: &CompiledBoard) -> (Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    warnings.extend(erc_floating_inputs(board));
    warnings.extend(erc_floating_power_inputs(board));
    errors.extend(erc_overvoltage(board));
    errors.extend(erc_nc_pin_connected(board));

    (warnings, errors)
}

fn component_index(refdes: &str, board: &CompiledBoard) -> usize {
    board
        .components
        .iter()
        .position(|c| c.refdes == refdes)
        .unwrap_or(usize::MAX)
}

fn connected_pins(board: &CompiledBoard) -> Vec<(&str, &str)> {
    board
        .connections
        .iter()
        .map(|c| {
            let comp = &board.components[c.component];
            (comp.refdes.as_str(), c.pin.as_str())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        board::{CompiledComponent, Connection},
        net::{Constraint, Net, NetClass, NetId},
        pin::Pin,
        units::UnitExt,
    };

    fn make_comp(refdes: &str, pins: Vec<Pin>, constraints: Vec<Constraint>) -> CompiledComponent {
        CompiledComponent {
            refdes: refdes.to_owned(),
            pins,
            constraints,
            symbol: None,
            footprint: None,
            mechanical: vec![],
            datasheet: None,
            description: None,
        }
    }

    #[test]
    fn overvoltage_detected() {
        let board = CompiledBoard {
            components: vec![make_comp(
                "U1",
                vec![Pin::build("VDD").pwr_fixed(3.3.volt(), 0.1.amp()).pin()],
                vec![],
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
                net: NetId("VBUS".into()),
            }],
            constraints: vec![],
        };
        let diags = erc_overvoltage(&board);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "ERC:OVERVOLT");
    }

    #[test]
    fn nc_pin_connected_flags_connected_nc_pin() {
        let board = CompiledBoard {
            components: vec![make_comp("U1", vec![Pin::build("NC").dio()], vec![])],
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
                net: NetId("NET".into()),
            }],
            constraints: vec![],
        };
        let diags = erc_nc_pin_connected(&board);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "ERC:NC_CONNECTED");
    }

    #[test]
    fn floating_input_flags_unconnected_digital_io() {
        let board = CompiledBoard {
            components: vec![make_comp("U1", vec![Pin::build("GPIO").dio()], vec![])],
            nets: vec![],
            connections: vec![],
            constraints: vec![],
        };
        let diags = erc_floating_inputs(&board);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "ERC:FLOATING_INPUT");
    }
}
