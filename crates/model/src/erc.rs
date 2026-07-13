use crate::{
    compiled::CompiledBoard,
    pin::{RawConnection, Role},
    units::{Diagnostic, Severity},
};

pub(crate) fn erc_floating_inputs(
    board: &CompiledBoard,
    connections: &[RawConnection],
) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for (comp_idx, comp) in board.components.iter().enumerate() {
        for pin in &comp.pins {
            if pin.name() == "NC" || pin.name().starts_with("NC_") {
                continue;
            }
            if matches!(pin.role(), Role::DigitalIO | Role::AnalogIn)
                && pin.sig_spec().is_none()
                && !pin_is_connected(comp_idx, pin.name(), connections)
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

pub(crate) fn erc_floating_power_inputs(
    board: &CompiledBoard,
    connections: &[RawConnection],
) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for (comp_idx, comp) in board.components.iter().enumerate() {
        for pin in &comp.pins {
            if matches!(pin.role(), Role::PowerIn)
                && !pin_is_connected(comp_idx, pin.name(), connections)
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

pub(crate) fn erc_nc_pin_connected(
    board: &CompiledBoard,
    connections: &[RawConnection],
) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for (comp_idx, comp) in board.components.iter().enumerate() {
        for pin in &comp.pins {
            if (pin.name() == "NC" || pin.name().starts_with("NC_"))
                && pin_is_connected(comp_idx, pin.name(), connections)
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

pub(crate) fn erc_overvoltage(board: &CompiledBoard) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for (comp_idx, comp) in board.components.iter().enumerate() {
        for pin in &comp.pins {
            if !matches!(pin.role(), Role::PowerIn) {
                continue;
            }
            for conn in &board.connections {
                if conn.component == comp_idx
                    && conn.pin == pin.name()
                    && let Some(net) = board.nets.iter().find(|n| n.name == conn.net.0)
                    && let crate::net::NetKind::Power { v_nom, .. } = net.kind
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

pub(crate) fn pin_is_connected(comp: usize, pin: &str, connections: &[RawConnection]) -> bool {
    connections.iter().any(|c| {
        (c.from.component == comp && c.from.pin == pin)
            || (c.to.component == comp && c.to.pin == pin)
    })
}
