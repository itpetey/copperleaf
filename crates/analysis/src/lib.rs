//! Analysis passes for Copperleaf.
//!
//! Contains early ERC (electrical rule check) helpers and a deterministic
//! decoupling-capacitor synthesis pass. This crate focuses on pure functions
//! over the IR.

use copperleaf_core::{Diagnostic, Farad, Qty, Severity};
use copperleaf_ir::{ComponentRecord, Constraint, Design, Net, NetKind, Pin, Role};
use serde::{Deserialize, Serialize};

/// Trait for analysis passes to expose a stable name.
pub trait CheckPass {
    fn name(&self) -> &str;
}

/// A decoupling capacitor placement emitted by [`synthesize_decoupling`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecouplingCap {
    /// Suggested reference designator (e.g. `C1`).
    pub refdes: String,
    /// Capacitance value.
    pub value: Qty<Farad>,
    /// Power net the capacitor is placed on.
    pub net: String,
    /// Component whose decoupling constraint produced this cap.
    pub source_component: String,
    /// Pin whose connected net determined the placement target.
    pub source_pin: String,
}

/// Result of running decoupling synthesis over a [`Design`].
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DecouplingResult {
    /// Decoupling capacitors placed by the pass.
    pub caps: Vec<DecouplingCap>,
    /// Diagnostics emitted during synthesis.
    pub diagnostics: Vec<Diagnostic>,
}

/// Returns a human-readable multi-line report summarizing the design.
///
/// The report includes graph stats, component list grouped by reference
/// designator prefix, power and signal net summaries, ERC results
/// (overvoltage and floating/NC pins), and decoupling synthesis output.
pub fn report(design: &Design) -> String {
    use std::collections::BTreeMap;

    let mut out = String::new();

    // Header
    out.push_str("Copperleaf Design Report\n");
    out.push_str("========================\n\n");

    // Graph stats
    let (nodes, edges) = design.graph.counts();
    out.push_str(&format!("Graph: {} nodes, {} edges\n", nodes, edges));
    out.push_str(&format!(
        "Components: {}, Nets: {}, Constraints: {}\n\n",
        design.components.len(),
        design.nets.len(),
        design.constraints.len()
    ));

    // Component list grouped by refdes prefix
    out.push_str("Components\n");
    out.push_str("----------\n");
    let mut groups: BTreeMap<String, Vec<&ComponentRecord>> = BTreeMap::new();
    for c in &design.components {
        let prefix: String = c.refdes.chars().take_while(|ch| ch.is_alphabetic()).collect();
        let prefix = if prefix.is_empty() {
            "?".into()
        } else {
            prefix
        };
        groups.entry(prefix).or_default().push(c);
    }
    if groups.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for (prefix, comps) in &groups {
            out.push_str(&format!("  [{}] ({}):\n", prefix, comps.len()));
            for c in comps {
                out.push_str(&format!("    - {} ({} pins)\n", c.refdes, c.pins.len()));
            }
        }
    }
    out.push('\n');

    // Net summary
    out.push_str("Nets\n");
    out.push_str("----\n");
    let mut power_nets: Vec<&Net> = Vec::new();
    let mut signal_nets: Vec<&Net> = Vec::new();
    for n in &design.nets {
        match &n.kind {
            NetKind::Power { .. } => power_nets.push(n),
            NetKind::Signal { .. } => signal_nets.push(n),
        }
    }
    out.push_str(&format!("  Power nets ({}):\n", power_nets.len()));
    if power_nets.is_empty() {
        out.push_str("    (none)\n");
    } else {
        for n in power_nets {
            let v = match &n.kind {
                NetKind::Power { v_nom, .. } => v_nom.as_base(),
                _ => 0.0,
            };
            let pins = design.pins_on_net(&n.name);
            out.push_str(&format!(
                "    - {} ({:.2} V, {} pins)\n",
                n.name,
                v,
                pins.len()
            ));
        }
    }
    out.push_str(&format!("  Signal nets ({}):\n", signal_nets.len()));
    if signal_nets.is_empty() {
        out.push_str("    (none)\n");
    } else {
        for n in signal_nets {
            let pins = design.pins_on_net(&n.name);
            out.push_str(&format!("    - {} ({} pins)\n", n.name, pins.len()));
        }
    }
    out.push('\n');

    // ERC results
    out.push_str("ERC Results\n");
    out.push_str("-----------\n");
    let mut erc_diags: Vec<Diagnostic> = Vec::new();
    for c in &design.components {
        for pin in &c.pins {
            let nets = design.nets_of_pin(&c.refdes, &pin.name);

            // Overvoltage: power-input pin connected to a power net with v_nom > v_max
            if matches!(pin.role, Role::PowerIn) {
                for net_name in &nets {
                    if let Some(net) = design.nets.iter().find(|n| n.name == *net_name) {
                        if let Some(diag) = erc_voltage_pin_to_net(net, pin) {
                            erc_diags.push(diag);
                        }
                    }
                }
            }

            // Floating/NC check: input-ish pins with no net connection
            if matches!(pin.role, Role::PowerIn | Role::AnalogIn | Role::DigitalIO) && nets.is_empty() {
                erc_diags.push(Diagnostic {
                    code: "ERC:FLOATING_PIN".into(),
                    severity: Severity::Warning,
                    message: format!("Pin {}.{} is unconnected", c.refdes, pin.name),
                    entities: vec![format!("{}.{}", c.refdes, pin.name)],
                    hint: Some("Connect the pin to a net or mark it as no-connect".into()),
                });
            }
        }
    }
    if erc_diags.is_empty() {
        out.push_str("  [Info] ERC:OK — no issues detected\n");
    } else {
        for diag in erc_diags {
            out.push_str(&format!(
                "  [{:?}] {} — {}\n",
                diag.severity, diag.code, diag.message
            ));
            if let Some(hint) = diag.hint {
                out.push_str(&format!("         hint: {}\n", hint));
            }
        }
    }
    out.push('\n');

    // Decoupling synthesis
    out.push_str("Decoupling Synthesis\n");
    out.push_str("--------------------\n");
    let decap = synthesize_decoupling(design);
    if decap.caps.is_empty() {
        out.push_str("  [Info] No decoupling capacitors placed\n");
    } else {
        for cap in &decap.caps {
            out.push_str(&format!(
                "  {}: {} F on {} (from {}.{})\n",
                cap.refdes,
                cap.value.as_base(),
                cap.net,
                cap.source_component,
                cap.source_pin
            ));
        }
    }
    for diag in &decap.diagnostics {
        out.push_str(&format!(
            "  [{:?}] {} — {}\n",
            diag.severity, diag.code, diag.message
        ));
    }

    out
}

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
                    entities: vec![pin.name.clone(), net.name.clone()],
                    hint: Some("Use a level shifter or different pin".into()),
                });
            }
            None
        }
        _ => None,
    }
}

/// Synthesize decoupling capacitors from part-level [`Constraint::Decoupling`] rules.
///
/// For each component carrying a `Decoupling` constraint, the pass places the
/// specified capacitor values on the power net connected to the component's
/// `PowerIn` pins. When `per_pin` is `true`, caps are placed independently for
/// each power-input pin; otherwise a single set is placed on the first
/// power-input pin's net.
///
/// The pass is deterministic: components, pins, and cap values are iterated in
/// insertion order, and reference designators are assigned as `C1`, `C2`, …
/// in that order. When a pin connects to multiple nets (itself a warning),
/// nets are sorted alphabetically and the first is used.
pub fn synthesize_decoupling(design: &Design) -> DecouplingResult {
    let mut caps = Vec::new();
    let mut diagnostics = Vec::new();
    let mut next_c = 1u32;

    for component in &design.components {
        for constraint in &component.constraints {
            let Constraint::Decoupling { values, per_pin } = constraint else {
                continue;
            };

            let power_pins: Vec<&Pin> = component
                .pins
                .iter()
                .filter(|p| matches!(p.role, Role::PowerIn))
                .collect();

            if power_pins.is_empty() {
                diagnostics.push(Diagnostic {
                    code: "DECOUPLE:NO_PWR_PIN".into(),
                    severity: Severity::Warning,
                    message: format!(
                        "{} has a decoupling constraint but no power-input pins",
                        component.refdes
                    ),
                    entities: vec![component.refdes.clone()],
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
                let mut nets = design.nets_of_pin(&component.refdes, &pin.name);
                nets.sort();

                let Some(net) = nets.first() else {
                    diagnostics.push(Diagnostic {
                        code: "DECOUPLE:UNCONNECTED".into(),
                        severity: Severity::Warning,
                        message: format!(
                            "power pin {}.{} is not connected to a net",
                            component.refdes, pin.name
                        ),
                        entities: vec![format!("{}.{}", component.refdes, pin.name)],
                        hint: Some("connect the pin to a power net".into()),
                    });
                    continue;
                };

                if nets.len() > 1 {
                    diagnostics.push(Diagnostic {
                        code: "DECOUPLE:MULTI_NET".into(),
                        severity: Severity::Warning,
                        message: format!(
                            "pin {}.{} connects to {} nets; placing on {}",
                            component.refdes,
                            pin.name,
                            nets.len(),
                            net
                        ),
                        entities: vec![format!("{}.{}", component.refdes, pin.name)],
                        hint: Some("check for shorted power nets".into()),
                    });
                }

                for value in values {
                    caps.push(DecouplingCap {
                        refdes: format!("C{}", next_c),
                        value: *value,
                        net: net.clone(),
                        source_component: component.refdes.clone(),
                        source_pin: pin.name.clone(),
                    });
                    next_c += 1;
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

    DecouplingResult { caps, diagnostics }
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf_core::UnitExt;
    use copperleaf_ir::{ComponentInst, ComponentRecord, Limits, Net, Pin, Role};
    use copperleaf_parts::Buck;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-15
    }

    fn build_connected_buck_design() -> Design {
        let mut d = Design::default();
        d.add_net(Net::power("VBUS", 5.0.volt()));
        d.add_net(Net::ground());

        let buck = Buck::new("MPM3610", 3.3.volt(), 2.0.amp());
        let u1 = ComponentInst::new("U1", buck);
        d.add_component(&u1);
        d.connect("U1", "VIN", "VBUS");
        d.connect("U1", "GND", "GND");
        d
    }

    #[test]
    fn overvoltage_detected() {
        let net = Net::power("VBUS", 5.0.volt());
        let pin = Pin {
            name: "VDD".into(),
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

    #[test]
    fn synthesizes_decoupling_caps_for_connected_buck() {
        let d = build_connected_buck_design();
        let result = synthesize_decoupling(&d);

        // Buck carries Decoupling { values: [100nF, 1µF], per_pin: true }.
        // VIN (PowerIn) is connected to VBUS → 2 caps on VBUS.
        assert_eq!(result.caps.len(), 2);
        assert_eq!(result.caps[0].refdes, "C1");
        assert_eq!(result.caps[0].net, "VBUS");
        assert_eq!(result.caps[0].source_component, "U1");
        assert_eq!(result.caps[0].source_pin, "VIN");
        assert!(approx_eq(result.caps[0].value.as_base(), 100e-9));
        assert_eq!(result.caps[1].refdes, "C2");
        assert_eq!(result.caps[1].net, "VBUS");
        assert!(approx_eq(result.caps[1].value.as_base(), 1e-6));
    }

    #[test]
    fn warns_when_power_pin_is_unconnected() {
        let mut d = Design::default();
        d.add_net(Net::power("VBUS", 5.0.volt()));
        d.add_net(Net::ground());

        let buck = Buck::new("MPM3610", 3.3.volt(), 2.0.amp());
        let u1 = ComponentInst::new("U1", buck);
        d.add_component(&u1);
        // VIN deliberately left unconnected

        let result = synthesize_decoupling(&d);
        assert!(result.caps.is_empty());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == "DECOUPLE:UNCONNECTED")
        );
    }

    #[test]
    fn produces_nothing_for_components_without_decoupling() {
        let mut d = Design::default();
        d.add_net(Net::power("V3V3", 3.3.volt()));
        d.components.push(ComponentRecord {
            refdes: "U2".into(),
            pins: vec![Pin {
                name: "VDD".into(),
                role: Role::PowerIn,
                limits: Limits::new(1.7.volt(), 3.6.volt(), 0.5.amp()),
                sig: None,
            }],
            constraints: vec![],
        });
        d.connect("U2", "VDD", "V3V3");

        let result = synthesize_decoupling(&d);
        assert!(result.caps.is_empty());
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn handles_per_pin_false() {
        let mut d = Design::default();
        d.add_net(Net::power("VBUS", 5.0.volt()));
        d.components.push(ComponentRecord {
            refdes: "U3".into(),
            pins: vec![
                Pin {
                    name: "VIN".into(),
                    role: Role::PowerIn,
                    limits: Limits::new(3.0.volt(), 24.0.volt(), 3.0.amp()),
                    sig: None,
                },
                Pin {
                    name: "EN".into(),
                    role: Role::PowerIn,
                    limits: Limits::new(0.0.volt(), 24.0.volt(), 0.1.amp()),
                    sig: None,
                },
            ],
            constraints: vec![Constraint::Decoupling {
                values: vec![10.0.uf()],
                per_pin: false,
            }],
        });
        d.connect("U3", "VIN", "VBUS");
        d.connect("U3", "EN", "VBUS");

        let result = synthesize_decoupling(&d);
        // per_pin: false → only first power pin (VIN) gets caps
        assert_eq!(result.caps.len(), 1);
        assert_eq!(result.caps[0].source_pin, "VIN");
        assert_eq!(result.caps[0].net, "VBUS");
        assert!(approx_eq(result.caps[0].value.as_base(), 10e-6));
    }

    #[test]
    fn warns_when_pin_connects_to_multiple_nets() {
        let mut d = Design::default();
        d.add_net(Net::power("VBUS", 5.0.volt()));
        d.add_net(Net::power("VCC", 5.0.volt()));
        d.components.push(ComponentRecord {
            refdes: "U4".into(),
            pins: vec![Pin {
                name: "VIN".into(),
                role: Role::PowerIn,
                limits: Limits::new(3.0.volt(), 24.0.volt(), 3.0.amp()),
                sig: None,
            }],
            constraints: vec![Constraint::Decoupling {
                values: vec![100.0.nf()],
                per_pin: true,
            }],
        });
        d.connect("U4", "VIN", "VBUS");
        d.connect("U4", "VIN", "VCC");

        let result = synthesize_decoupling(&d);
        assert_eq!(result.caps.len(), 1);
        // sorted alphabetically: VBUS < VCC
        assert_eq!(result.caps[0].net, "VBUS");
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == "DECOUPLE:MULTI_NET")
        );
    }

    #[test]
    fn synthesis_is_deterministic() {
        let d = build_connected_buck_design();
        let first = synthesize_decoupling(&d);
        let second = synthesize_decoupling(&d);

        assert_eq!(first.caps.len(), second.caps.len());
        for (a, b) in first.caps.iter().zip(second.caps.iter()) {
            assert_eq!(a.refdes, b.refdes);
            assert_eq!(a.net, b.net);
            assert_eq!(a.source_component, b.source_component);
            assert_eq!(a.source_pin, b.source_pin);
            assert!(approx_eq(a.value.as_base(), b.value.as_base()));
        }
    }

    #[test]
    fn summary_diagnostic_reports_cap_count() {
        let d = build_connected_buck_design();
        let result = synthesize_decoupling(&d);
        let summary = result
            .diagnostics
            .iter()
            .find(|d| d.code == "DECOUPLE:SUMMARY")
            .expect("summary diagnostic exists");
        assert_eq!(summary.severity, Severity::Info);
        assert!(summary.message.contains("2"));
        assert_eq!(summary.entities, vec!["C1", "C2"]);
    }

    #[test]
    fn report_contains_expected_sections() {
        let d = build_connected_buck_design();
        let r = report(&d);
        assert!(r.contains("Copperleaf Design Report"));
        assert!(r.contains("Graph:"));
        assert!(r.contains("Components"));
        assert!(r.contains("Nets"));
        assert!(r.contains("ERC Results"));
        assert!(r.contains("Decoupling Synthesis"));
        assert!(r.contains("U1"));
        assert!(r.contains("VBUS"));
        assert!(r.contains("C1"));
    }
}
