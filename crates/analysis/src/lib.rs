//! Analysis passes for Copperleaf.
//!
//! Contains ERC (electrical rule check) helpers and a deterministic
//! decoupling-capacitor synthesis pass, plus the [`analyse`] entry point that
//! turns a lowered [`CompiledBoard`] into a full [`CompileReport`].
//!
//! [`CompiledBoard`]: copperleaf_model::CompiledBoard
//! [`CompileReport`]: copperleaf_model::CompileReport

use copperleaf_model::{
    CompileError, CompileReport, CompileSummary, CompiledBoard, CompiledComponent, Constraint,
    Diagnostic, NetInfo, NetKind, Pin, Role, Severity, SynthCap,
};

/// Run every analysis pass against a lowered [`CompiledBoard`] and assemble
/// the final [`CompileReport`].
///
/// Electrical-rule checks are run first: warnings are always collected, errors
/// are fatal and short-circuit the pipeline.  Decoupling synthesis only runs
/// once the board is electrically valid.
pub fn analyse(board: CompiledBoard) -> Result<CompileReport, CompileError> {
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let mut errors: Vec<Diagnostic> = Vec::new();

    warnings.extend(erc_floating_inputs(&board));
    warnings.extend(erc_floating_power_inputs(&board));
    errors.extend(erc_overvoltage(&board));
    errors.extend(erc_nc_pin_connected(&board));

    if !errors.is_empty() {
        return Err(CompileError::new(errors));
    }

    // Synthesis only runs when the board is electrically valid.
    let (synth_components, synth_caps, synth_diags) = synthesise_decoupling(&board);
    warnings.extend(synth_diags);

    let CompiledBoard {
        components,
        nets,
        connections,
        constraints,
    } = board;
    let mut final_components = components;
    final_components.extend(synth_components);

    let final_board = CompiledBoard {
        components: final_components,
        nets,
        connections,
        constraints,
    };

    let summary = build_summary(&final_board, synth_caps);

    Ok(CompileReport {
        board: final_board,
        warnings,
        summary,
    })
}

/// ERC rule: flag DigitalIO/AnalogIn pins with no signal spec and no net connection.
pub fn erc_floating_inputs(board: &CompiledBoard) -> Vec<Diagnostic> {
    let connected = connected_pins(board);
    let mut diags = Vec::new();
    for comp in &board.components {
        for pin in &comp.pins {
            if pin.name() == "NC" || pin.name().starts_with("NC_") {
                continue;
            }
            if matches!(pin.role(), Role::DigitalIO | Role::AnalogIn) && pin.sig_spec().is_none()
                && !connected.contains(&(comp.refdes.as_str(), pin.name())) {
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
                && !connected.contains(&(comp.refdes.as_str(), pin.name())) {
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
                && connected.contains(&(comp.refdes.as_str(), pin.name())) {
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
                if conn.component == component_index(&comp.refdes, board) && conn.pin == pin.name()
                    && let Some(net) = board.nets.iter().find(|n| n.name == conn.net.0)
                        && let NetKind::Power { v_nom, .. } = net.kind
                            && v_nom.as_base() > pin.power_spec().v_max.as_base() + 1e-9 {
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
                                    entities: vec![
                                        format!("{}.{}", comp.refdes, pin.name()),
                                        net.name.clone(),
                                    ],
                                    hint: Some("Use a level shifter or different pin".into()),
                                });
                            }
            }
        }
    }
    diags
}

/// Synthesise decoupling capacitors from part-level [`Constraint::Decoupling`] rules.
pub fn synthesise_decoupling(
    board: &CompiledBoard,
) -> (Vec<CompiledComponent>, Vec<SynthCap>, Vec<Diagnostic>) {
    let mut components = Vec::new();
    let mut caps = Vec::new();
    let mut diagnostics = Vec::new();
    let mut next_c = 1u32;

    for (comp_idx, comp) in board.components.iter().enumerate() {
        for constraint in &comp.constraints {
            let Constraint::Decoupling { values, per_pin } = constraint else {
                continue;
            };

            let power_pins: Vec<&Pin> = comp
                .pins
                .iter()
                .filter(|p| matches!(p.role(), Role::PowerIn))
                .collect();

            if power_pins.is_empty() {
                diagnostics.push(Diagnostic {
                    code: "DECOUPLE:NO_PWR_PIN".into(),
                    severity: Severity::Warning,
                    message: format!(
                        "{} has a decoupling constraint but no power-input pins",
                        comp.refdes
                    ),
                    entities: vec![comp.refdes.clone()],
                    hint: Some("add a PowerIn pin to the part definition".into()),
                });
                continue;
            }

            let target_pins: Vec<&Pin> = if *per_pin {
                power_pins.clone()
            } else {
                vec![power_pins[0]]
            };

            for pin in target_pins {
                let net_name = board
                    .connections
                    .iter()
                    .find(|c| c.component == comp_idx && c.pin == pin.name())
                    .map(|c| c.net.0.clone());

                let Some(net) = net_name else {
                    diagnostics.push(Diagnostic {
                        code: "DECOUPLE:UNCONNECTED".into(),
                        severity: Severity::Warning,
                        message: format!(
                            "power pin {}.{} is not connected to a net",
                            comp.refdes,
                            pin.name()
                        ),
                        entities: vec![format!("{}.{}", comp.refdes, pin.name())],
                        hint: Some("connect the pin to a power net".into()),
                    });
                    continue;
                };

                for value in values {
                    let refdes = format!("C{}", next_c);
                    next_c += 1;
                    components.push(make_capacitor_component(&refdes));
                    caps.push(SynthCap {
                        refdes: refdes.clone(),
                        value: *value,
                        net: net.clone(),
                        source_component: comp.refdes.clone(),
                        source_pin: pin.name().to_owned(),
                    });
                }
            }
        }
    }

    if !caps.is_empty() {
        diagnostics.push(Diagnostic {
            code: "DECOUPLE:SUMMARY".into(),
            severity: Severity::Info,
            message: format!("placed {} decoupling capacitor(s)", caps.len()),
            entities: caps.iter().map(|c| c.refdes.clone()).collect(),
            hint: None,
        });
    }

    (components, caps, diagnostics)
}

/// Build the [`CompileSummary`] from the final [`CompiledBoard`] and the
/// decoupling capacitors synthesised during the pipeline.
fn build_summary(board: &CompiledBoard, synth_caps: Vec<SynthCap>) -> CompileSummary {
    CompileSummary {
        nets: board
            .nets
            .iter()
            .map(|n| NetInfo {
                name: n.name.clone(),
                kind: n.kind.clone(),
                pin_count: board
                    .connections
                    .iter()
                    .filter(|c| c.net.0 == n.name)
                    .count(),
            })
            .collect(),
        caps_synthesised: synth_caps,
        pin_count: board.components.iter().map(|c| c.pins.len()).sum(),
        component_count: board.components.len(),
    }
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

fn make_capacitor_component(refdes: &str) -> CompiledComponent {
    use copperleaf_model::{PinId, UnitExt, deterministic_id};
    let pin1_id = PinId(deterministic_id(&format!("{}:1", refdes)));
    let pin2_id = PinId(deterministic_id(&format!("{}:2", refdes)));
    CompiledComponent {
        refdes: refdes.to_owned(),
        pins: vec![
            Pin::build("1")
                .pwr_fixed(50.0.volt(), 0.1.amp())
                .decouple(false)
                .pin()
                .with_id(pin1_id),
            Pin::build("2").gnd().with_id(pin2_id),
        ],
        constraints: vec![],
        symbol: None,
        footprint: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf_model::{Connection, Net, NetClass, NetId, Pin, UnitExt};

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-15
    }

    fn make_comp(refdes: &str, pins: Vec<Pin>, constraints: Vec<Constraint>) -> CompiledComponent {
        CompiledComponent {
            refdes: refdes.to_owned(),
            pins,
            constraints,
            symbol: None,
            footprint: None,
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
    fn synthesises_decoupling_caps() {
        let board = CompiledBoard {
            components: vec![make_comp(
                "U1",
                vec![Pin::build("VIN").pwr_fixed(3.3.volt(), 1.0.amp()).pin()],
                vec![Constraint::Decoupling {
                    values: vec![100.0.nf(), 1.0.uf()],
                    per_pin: true,
                }],
            )],
            nets: vec![Net {
                name: "V3V3".into(),
                kind: NetKind::Power {
                    v_nom: 3.3.volt(),
                    ripple: None,
                },
                class: NetClass::default(),
                constraints: vec![],
            }],
            connections: vec![Connection {
                component: 0,
                pin: "VIN".into(),
                net: NetId("V3V3".into()),
            }],
            constraints: vec![],
        };
        let (comps, caps, diags) = synthesise_decoupling(&board);
        assert_eq!(caps.len(), 2);
        assert_eq!(caps[0].refdes, "C1");
        assert_eq!(caps[0].net, "V3V3");
        assert_eq!(caps[0].source_component, "U1");
        assert_eq!(caps[0].source_pin, "VIN");
        assert!(approx_eq(caps[0].value.as_base(), 100e-9));
        assert_eq!(comps.len(), 2);
        assert!(diags.iter().any(|d| d.code == "DECOUPLE:SUMMARY"));
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
