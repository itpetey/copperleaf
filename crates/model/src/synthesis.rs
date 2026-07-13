use crate::compiled::{CompiledBoard, CompiledComponent, SynthCap};
use crate::net::Constraint;
use crate::pin::{Pin, PinId, Role};
use crate::units::{Diagnostic, Severity, UnitExt};
use crate::util::deterministic_id;

pub(crate) fn synthesize_decoupling(
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
                    let pin1_id = PinId(deterministic_id(&format!("{}:1", refdes)));
                    let pin2_id = PinId(deterministic_id(&format!("{}:2", refdes)));
                    components.push(CompiledComponent {
                        refdes: refdes.clone(),
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
                    });
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
