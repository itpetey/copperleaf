//! The compilation pipeline — turns a [`Board`](copperleaf::Board) into a
//! [`CompileReport`] in a single pass.
//!
//! The pipeline runs three phases in order:
//!
//! 1. **Lowering** — net grouping, name/voltage resolution, and net
//!    classification produce the base [`CompiledBoard`].
//! 2. **Validation** — ERC checks (see [`copperleaf::erc`]) inspect the
//!    lowered board.  Warnings are collected; errors short-circuit.
//! 3. **Generation** — decoupling-capacitor synthesis produces additional
//!    components that are appended to the final board.
//!
//! The returned `CompiledBoard` is constructed exactly once and never rebuilt
//! or mutated afterwards.

use std::collections::{BTreeMap, HashMap};

use copperleaf::{
    CompiledComponent, Farad,
    board::{Board, CompiledBoard, ComponentEntry, Connection, RawNetOverride},
    erc,
    net::{Constraint, Net, NetClass, NetId, NetKind},
    pin::{Pin, RawConnection, Role, SigSpec},
    units::{Diagnostic, Qty, Severity, UnitExt, Volt},
    util::UnionFind,
};
use copperleaf_parts_passives::footprint::Package;

/// Default footprint package for synthesised decoupling capacitors.
const DEFAULT_CAP_FOOTPRINT: Package = Package::M1608;

#[derive(Clone, Debug)]
pub struct NetInfo {
    pub name: String,
    pub kind: NetKind,
    pub pin_count: usize,
}

#[derive(Clone, Debug)]
pub struct CompileSummary {
    pub nets: Vec<NetInfo>,
    pub pin_count: usize,
    pub component_count: usize,
}

#[derive(Clone, Debug)]
pub struct CompileReport {
    pub board: CompiledBoard,
    pub warnings: Vec<Diagnostic>,
    pub summary: CompileSummary,
}

/// Re-exported from `copperleaf` so callers have a single `CompileError` type.
pub use copperleaf::CompileError;

/// Intermediate data structure produced by a union-find pass over the raw
/// connections.  Maps connected pins into equivalence classes (nets).
struct NetGrouping {
    pin_to_node: HashMap<(usize, &'static str), usize>,
    nodes: Vec<(usize, &'static str)>,
    /// Groups keyed by union-find root.  A `BTreeMap` is used so iteration
    /// order (and therefore net ordering in the compiled board) is
    /// deterministic across processes.
    groups: BTreeMap<usize, Vec<usize>>,
    /// Map from node index to its union-find root.
    roots: Vec<usize>,
}

/// Options controlling the compilation pipeline.
pub struct CompileOptions {
    /// Footprint code used for synthesised decoupling capacitors.
    /// Defaults to [`DEFAULT_CAP_FOOTPRINT`] (0603 / 1608 metric).
    pub decoupling_footprint: Package,
}

impl NetGrouping {
    /// Build the grouping from the given connections.
    fn build(connections: &[RawConnection]) -> Self {
        let mut pin_to_node: HashMap<(usize, &'static str), usize> = HashMap::new();
        let mut nodes: Vec<(usize, &'static str)> = Vec::new();

        for conn in connections {
            for handle in [conn.from, conn.to] {
                if let std::collections::hash_map::Entry::Vacant(e) =
                    pin_to_node.entry((handle.component, handle.pin))
                {
                    e.insert(nodes.len());
                    nodes.push((handle.component, handle.pin));
                }
            }
        }

        let mut uf = UnionFind::new(nodes.len());
        for conn in connections {
            let a = pin_to_node[&(conn.from.component, conn.from.pin)];
            let b = pin_to_node[&(conn.to.component, conn.to.pin)];
            uf.union(a, b);
        }

        let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        let mut roots = Vec::with_capacity(nodes.len());
        for i in 0..nodes.len() {
            let root = uf.find(i);
            roots.push(root);
            groups.entry(root).or_default().push(i);
        }

        Self {
            pin_to_node,
            nodes,
            groups,
            roots,
        }
    }

    /// Return the set of connection-edge indices whose pins belong to the net
    /// identified by `rep`.
    fn edges_for_net(&self, rep: usize, connections: &[RawConnection]) -> Vec<usize> {
        let mut edge_ids = Vec::new();
        for (edge_id, conn) in connections.iter().enumerate() {
            let a = (conn.from.component, conn.from.pin);
            let b = (conn.to.component, conn.to.pin);
            let a_node = self.pin_to_node[&a];
            if self.roots[a_node] == rep {
                edge_ids.push(edge_id);
                continue;
            }
            let b_node = self.pin_to_node[&b];
            if self.roots[b_node] == rep {
                edge_ids.push(edge_id);
            }
        }
        edge_ids
    }
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            decoupling_footprint: DEFAULT_CAP_FOOTPRINT,
        }
    }
}

/// Compile a [`Board`] into a [`CompileReport`].
///
/// This is the entry point for the entire pipeline.  It validates
/// connections first and then runs lowering, ERC, and synthesis.
pub fn run(board: Board, options: &CompileOptions) -> Result<CompileReport, CompileError> {
    // --- Phase 1: Lowering ---
    let compiled = compile_components(&board.components);
    let grouping = NetGrouping::build(&board.connections);

    let mut lowering_errors: Vec<Diagnostic> = Vec::new();

    let (net_names, net_voltages) = resolve_net_overrides(
        &grouping,
        &board.connections,
        &board.net_overrides,
        &board.components,
        &compiled,
        &mut lowering_errors,
    );

    let (nets, connections) = build_nets_and_connections(
        &grouping,
        &net_names,
        &net_voltages,
        &compiled,
        &mut lowering_errors,
    );

    let constraints: Vec<Constraint> = compiled
        .iter()
        .flat_map(|c| c.constraints.clone())
        .collect();

    // Lowering errors (e.g. voltage conflicts) are fatal.
    if !lowering_errors.is_empty() {
        return Err(CompileError::new(lowering_errors));
    }

    let board_struct = CompiledBoard {
        components: compiled,
        nets,
        connections,
        constraints,
        width: board.width(),
        height: board.height(),
    };

    // --- Phase 2: Validation (ERC) ---
    let (mut warnings, errors) = erc::run_erc(&board_struct);
    if !errors.is_empty() {
        return Err(CompileError::new(errors));
    }

    // --- Phase 3: Generation (synthesis) ---
    let (synth_components, synth_diags, synth_connections, synth_net) =
        synthesise_decoupling(&board_struct, options.decoupling_footprint);
    warnings.extend(synth_diags);

    let mut final_components = board_struct.components;
    final_components.extend(synth_components);

    let has_synth_connections = !synth_connections.is_empty();
    let mut final_connections = board_struct.connections;
    final_connections.extend(synth_connections);

    let mut final_nets = board_struct.nets;
    if let Some(net) = synth_net {
        // Only add a fallback ground net if decoupling capacitors were actually
        // placed and reference it.
        if has_synth_connections {
            final_nets.push(net);
        }
    }

    let final_board = CompiledBoard {
        components: final_components,
        nets: final_nets,
        connections: final_connections,
        constraints: board_struct.constraints,
        width: board_struct.width,
        height: board_struct.height,
    };

    let summary = build_summary(&final_board);

    Ok(CompileReport {
        board: final_board,
        warnings,
        summary,
    })
}

/// Turn every group of connected pins into a [`Net`] and a set of
/// [`Connection`]s.  Power nets without a voltage source are flagged as errors.
fn build_nets_and_connections(
    grouping: &NetGrouping,
    net_names: &HashMap<usize, String>,
    net_voltages: &HashMap<usize, Option<Qty<Volt>>>,
    compiled: &[CompiledComponent],
    errors: &mut Vec<Diagnostic>,
) -> (Vec<Net>, Vec<Connection>) {
    let mut nets: Vec<Net> = Vec::new();
    let mut connections: Vec<Connection> = Vec::new();

    for (&rep, members) in &grouping.groups {
        let name = net_names[&rep].clone();
        let voltage = net_voltages[&rep];

        let kind = classify_net(&name, voltage, members, grouping, compiled, errors);

        nets.push(Net {
            name: name.clone(),
            kind,
            class: NetClass::default(),
            constraints: vec![],
        });

        for &node_idx in members {
            let (comp_idx, pin_name) = grouping.nodes[node_idx];
            connections.push(Connection {
                component: comp_idx,
                pin: pin_name.to_owned(),
                net: NetId(name.clone()),
            });
        }
    }

    (nets, connections)
}

/// Build the [`CompileSummary`] from the final board.
fn build_summary(board: &CompiledBoard) -> CompileSummary {
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
        pin_count: board.components.iter().map(|c| c.pins.len()).sum(),
        component_count: board.components.len(),
    }
}

/// Determine whether a net is power or signal and assign its [`NetKind`].
fn classify_net(
    net_name: &str,
    voltage: Option<Qty<Volt>>,
    members: &[usize],
    grouping: &NetGrouping,
    compiled: &[CompiledComponent],
    errors: &mut Vec<Diagnostic>,
) -> NetKind {
    let mut is_power = false;
    let mut sig_spec: Option<SigSpec> = None;

    for &node_idx in members {
        let (comp_idx, pin_name) = grouping.nodes[node_idx];
        let pin = compiled[comp_idx]
            .pins
            .iter()
            .find(|p| p.name() == pin_name)
            .unwrap();

        match pin.role() {
            Role::PowerIn | Role::PowerOut | Role::Gnd => is_power = true,
            _ => {}
        }
        if sig_spec.is_none() && pin.sig_spec().is_some() {
            sig_spec = pin.sig_spec();
        }
    }

    if is_power {
        match voltage {
            Some(v_nom) => NetKind::Power {
                v_nom,
                ripple: None,
            },
            None => {
                errors.push(Diagnostic {
                    code: "NET:NO_VOLTAGE_SOURCE".into(),
                    severity: Severity::Error,
                    message: format!(
                        "power net '{}' has no voltage source; use Board::set_net_voltage()",
                        net_name
                    ),
                    entities: vec![net_name.to_owned()],
                    hint: Some("call Board::set_net_voltage()".into()),
                });
                NetKind::Power {
                    v_nom: 0.0.volt(),
                    ripple: None,
                }
            }
        }
    } else {
        NetKind::Signal {
            spec: sig_spec.unwrap_or_else(SigSpec::control),
        }
    }
}

/// Type-erase [`Component`] trait objects into [`CompiledComponent`]s with
/// deterministic pin IDs.
fn compile_components(entries: &[ComponentEntry]) -> Vec<CompiledComponent> {
    entries
        .iter()
        .map(|entry| CompiledComponent::from_component(&entry.name, entry.component.as_ref()))
        .collect()
}

fn ground_net_name_and_fallback(board: &CompiledBoard) -> (String, Option<Net>) {
    if let Some(n) = board.nets.iter().find(|n| n.name == "GND") {
        return (n.name.clone(), None);
    }
    if let Some(n) = board
        .nets
        .iter()
        .find(|n| matches!(n.kind, NetKind::Power { v_nom, .. } if v_nom.as_base().abs() < 1e-9))
    {
        return (n.name.clone(), None);
    }
    let net = Net {
        name: "GND".into(),
        kind: NetKind::Power {
            v_nom: 0.0.volt(),
            ripple: None,
        },
        class: NetClass::default(),
        constraints: vec![],
    };
    ("GND".into(), Some(net))
}

/// Collect `v_nom` from every power pin on the net.  Returns `None` if no pin
/// provides a nominal voltage, or emits an error on conflict.
///
/// Ground pins are ignored as voltage sources unless the net contains no other
/// `PowerIn`/`PowerOut` pins, so that decoupling capacitors and direct ground
/// ties do not create false conflicts.
fn infer_voltage_from_pins(
    members: &[usize],
    nodes: &[(usize, &'static str)],
    compiled: &[CompiledComponent],
    net_name: &str,
    errors: &mut Vec<Diagnostic>,
) -> Option<Qty<Volt>> {
    let mut inferred: Option<Qty<Volt>> = None;
    let mut ground: Option<Qty<Volt>> = None;

    for &node_idx in members {
        let (comp_idx, pin_name) = nodes[node_idx];
        let comp = &compiled[comp_idx];
        let pin = comp.pins.iter().find(|p| p.name() == pin_name).unwrap();

        let v = pin.power_spec().v_nom;
        if matches!(pin.role(), Role::Gnd) {
            ground = v.or(Some(0.0.volt()));
            continue;
        }

        let Some(v) = v else {
            continue;
        };

        if let Some(existing) = inferred {
            if (existing.as_base() - v.as_base()).abs() > 1e-9 {
                errors.push(Diagnostic {
                    code: "NET:VOLTAGE_CONFLICT".into(),
                    severity: Severity::Error,
                    message: format!(
                        "conflicting v_nom values on net '{}' ({:.2}V vs {:.2}V)",
                        net_name,
                        existing.as_base(),
                        v.as_base()
                    ),
                    entities: vec![net_name.to_owned()],
                    hint: Some("check connected power pins".into()),
                });
            }
        } else {
            inferred = Some(v);
        }
    }

    inferred.or(ground)
}

fn is_ground_net(board: &CompiledBoard, net_name: &str) -> bool {
    board
        .nets
        .iter()
        .find(|n| n.name == net_name)
        .map(|n| matches!(n.kind, NetKind::Power { v_nom, .. } if v_nom.as_base().abs() < 1e-9))
        .unwrap_or(false)
}

/// Create a decoupling capacitor [`CompiledComponent`] with a proper SMD
/// footprint from the passives library.
fn make_capacitor_component(
    refdes: &str,
    value: Qty<Farad>,
    package: Package,
) -> CompiledComponent {
    let cap = copperleaf_parts_passives::Capacitor::decoupling(value, package);
    CompiledComponent::from_component(refdes, &cap)
}

/// Walk the edge overrides for a single net and merge them, detecting
/// conflicting voltage or name values.
fn merge_overrides(
    edge_ids: &[usize],
    overrides: &[RawNetOverride],
    errors: &mut Vec<Diagnostic>,
    rep: usize,
) -> (Option<Qty<Volt>>, Option<String>) {
    let mut explicit_voltage: Option<Qty<Volt>> = None;
    let mut explicit_name: Option<String> = None;

    for &eid in edge_ids {
        if let Some(ov) = overrides.get(eid) {
            if let Some(v) = ov.voltage {
                if let Some(existing) = explicit_voltage {
                    if (existing.as_base() - v.as_base()).abs() > 1e-9 {
                        errors.push(Diagnostic {
                            code: "NET:VOLTAGE_CONFLICT".into(),
                            severity: Severity::Error,
                            message: "conflicting voltage overrides on merged net".into(),
                            entities: vec![format!("net:{}", rep)],
                            hint: Some("resolve the voltage mismatch".into()),
                        });
                    }
                } else {
                    explicit_voltage = Some(v);
                }
            }
            if let Some(name) = &ov.name {
                explicit_name = Some(name.clone());
            }
        }
    }

    (explicit_voltage, explicit_name)
}

#[allow(clippy::too_many_arguments)]
fn place_decoupling_set(
    board: &CompiledBoard,
    comp_idx: usize,
    comp: &CompiledComponent,
    pin: &Pin,
    values: &[Qty<Farad>],
    package: Package,
    gnd_net_name: &str,
    components: &mut Vec<CompiledComponent>,
    connections: &mut Vec<Connection>,
    diagnostics: &mut Vec<Diagnostic>,
    next_c: &mut u32,
) {
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
        return;
    };

    // Skip pins that are tied directly to ground (e.g. VDD_USB in SPI mode).
    if is_ground_net(board, &net) {
        return;
    }

    for value in values {
        let refdes = format!("C{}", *next_c);
        *next_c += 1;

        let comp_idx_in_final = board.components.len() + components.len();
        components.push(make_capacitor_component(&refdes, *value, package));
        connections.push(Connection {
            component: comp_idx_in_final,
            pin: "1".into(),
            net: NetId(net.clone()),
        });
        connections.push(Connection {
            component: comp_idx_in_final,
            pin: "2".into(),
            net: NetId(gnd_net_name.into()),
        });
    }
}

/// Determine the name and voltage for every net by merging explicit overrides
/// with values inferred from connected pins.
///
/// Returns `(net_names, net_voltages)` keyed by the group representative id.
/// Diagnostics are appended to `errors`.
#[allow(clippy::type_complexity)]
fn resolve_net_overrides(
    grouping: &NetGrouping,
    connections: &[RawConnection],
    overrides: &[RawNetOverride],
    components: &[ComponentEntry],
    compiled: &[CompiledComponent],
    errors: &mut Vec<Diagnostic>,
) -> (HashMap<usize, String>, HashMap<usize, Option<Qty<Volt>>>) {
    let mut net_names: HashMap<usize, String> = HashMap::new();
    let mut net_voltages: HashMap<usize, Option<Qty<Volt>>> = HashMap::new();

    for (&rep, members) in &grouping.groups {
        let edge_ids = grouping.edges_for_net(rep, connections);

        // Merge explicit overrides.
        let (explicit_voltage, explicit_name) = merge_overrides(&edge_ids, overrides, errors, rep);

        // Determine net name (override -> auto-generated).
        let name = explicit_name.unwrap_or_else(|| {
            let (comp, pin) = grouping.nodes[rep];
            let comp_name = &components[comp].name;
            format!("NET_{}_{}", comp_name, pin)
        });

        // Infer voltage from connected pin v_nom values.
        let inferred_voltage =
            infer_voltage_from_pins(members, &grouping.nodes, compiled, &name, errors);

        // Override takes precedence over inferred.
        let final_voltage = explicit_voltage.or(inferred_voltage);

        net_names.insert(rep, name);
        net_voltages.insert(rep, final_voltage);
    }

    (net_names, net_voltages)
}

/// Synthesise decoupling capacitors from part-level [`Constraint::Decoupling`] rules.
///
/// Returns `(components, diagnostics, connections, fallback_ground_net)`:
/// - `components` -- new [`CompiledComponent`]s to append to the board.
/// - `diagnostics` -- informational warnings about missing power pins or
///   unconnected power nets, plus an info-level summary.
/// - `connections` -- new connections wiring each synthesised capacitor between
///   its target power net and ground.
/// - `fallback_ground_net` -- a fresh ground net if the board did not already
///   contain one.
///
/// The board itself is never mutated; the caller appends the returned
/// components and connections exactly once when assembling the final `CompiledBoard`.
#[allow(clippy::type_complexity)]
fn synthesise_decoupling(
    board: &CompiledBoard,
    package: Package,
) -> (
    Vec<CompiledComponent>,
    Vec<Diagnostic>,
    Vec<Connection>,
    Option<Net>,
) {
    let mut components = Vec::new();
    let mut diagnostics = Vec::new();
    let mut connections = Vec::new();
    let mut next_c = 1u32;

    let (gnd_net_name, fallback_gnd_net) = ground_net_name_and_fallback(board);

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

            if *per_pin {
                for pin in power_pins {
                    place_decoupling_set(
                        board,
                        comp_idx,
                        comp,
                        pin,
                        values,
                        package,
                        &gnd_net_name,
                        &mut components,
                        &mut connections,
                        &mut diagnostics,
                        &mut next_c,
                    );
                }
            } else {
                // One set of decoupling capacitors per unique power net.
                // A `BTreeMap` keeps iteration (and hence capacitor refdes
                // assignment) deterministic across processes.
                let mut pins_by_net: BTreeMap<String, Vec<&Pin>> = BTreeMap::new();
                for pin in power_pins {
                    let net_name = board
                        .connections
                        .iter()
                        .find(|c| c.component == comp_idx && c.pin == pin.name())
                        .map(|c| c.net.0.clone());
                    if let Some(name) = net_name {
                        pins_by_net.entry(name).or_default().push(pin);
                    } else {
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
                    }
                }
                for (_, pins) in pins_by_net {
                    // Use the first pin on the net as the representative.
                    if let Some(pin) = pins.first() {
                        place_decoupling_set(
                            board,
                            comp_idx,
                            comp,
                            pin,
                            values,
                            package,
                            &gnd_net_name,
                            &mut components,
                            &mut connections,
                            &mut diagnostics,
                            &mut next_c,
                        );
                    }
                }
            }
        }
    }

    if !components.is_empty() {
        diagnostics.push(Diagnostic {
            code: "DECOUPLE:SUMMARY".into(),
            severity: Severity::Info,
            message: format!("placed {} decoupling capacitor(s)", components.len()),
            entities: components.iter().map(|c| c.refdes.clone()).collect(),
            hint: None,
        });
    }

    (components, diagnostics, connections, fallback_gnd_net)
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf::{ComponentMeta, units::UnitExt};

    fn make_comp(refdes: &str, pins: Vec<Pin>, constraints: Vec<Constraint>) -> CompiledComponent {
        CompiledComponent {
            refdes: refdes.to_owned(),
            meta: ComponentMeta::default(),
            pins,
            mechanical: vec![],
            constraints,
        }
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
            width: 100.0,
            height: 80.0,
        };
        let (comps, diags, conns, fallback) = synthesise_decoupling(&board, DEFAULT_CAP_FOOTPRINT);
        assert_eq!(comps.len(), 2);
        assert_eq!(comps[0].refdes, "C1");
        assert_eq!(conns.len(), 4);
        assert!(fallback.is_some());
        assert!(diags.iter().any(|d| d.code == "DECOUPLE:SUMMARY"));
    }

    #[test]
    fn synthesised_caps_have_footprints() {
        let board = CompiledBoard {
            components: vec![make_comp(
                "U1",
                vec![Pin::build("VIN").pwr_fixed(3.3.volt(), 1.0.amp()).pin()],
                vec![Constraint::Decoupling {
                    values: vec![100.0.nf()],
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
            width: 100.0,
            height: 80.0,
        };
        let (comps, _, _, _) = synthesise_decoupling(&board, DEFAULT_CAP_FOOTPRINT);
        assert_eq!(comps.len(), 1);
        let fp = comps[0]
            .meta
            .footprint
            .as_ref()
            .expect("cap should have a footprint");
        assert!(
            fp.contains("Capacitor_SMD"),
            "footprint should be a KiCad capacitor: {fp}"
        );
    }

    #[test]
    fn skips_ground_tied_power_pin() {
        let board = CompiledBoard {
            components: vec![make_comp(
                "U1",
                vec![
                    Pin::build("VDD").pwr_fixed(3.3.volt(), 1.0.amp()).pin(),
                    Pin::build("VDD_USB")
                        .pwr(3.0.volt(), 3.6.volt(), 0.1.amp())
                        .pin(),
                ],
                vec![Constraint::Decoupling {
                    values: vec![100.0.nf()],
                    per_pin: false,
                }],
            )],
            nets: vec![
                Net {
                    name: "V3V3".into(),
                    kind: NetKind::Power {
                        v_nom: 3.3.volt(),
                        ripple: None,
                    },
                    class: NetClass::default(),
                    constraints: vec![],
                },
                Net {
                    name: "GND".into(),
                    kind: NetKind::Power {
                        v_nom: 0.0.volt(),
                        ripple: None,
                    },
                    class: NetClass::default(),
                    constraints: vec![],
                },
            ],
            connections: vec![
                Connection {
                    component: 0,
                    pin: "VDD".into(),
                    net: NetId("V3V3".into()),
                },
                Connection {
                    component: 0,
                    pin: "VDD_USB".into(),
                    net: NetId("GND".into()),
                },
            ],
            constraints: vec![],
            width: 100.0,
            height: 80.0,
        };
        let (comps, _, _, _) = synthesise_decoupling(&board, DEFAULT_CAP_FOOTPRINT);
        assert_eq!(comps.len(), 1);
    }

    #[test]
    fn groups_per_net_when_not_per_pin() {
        let board = CompiledBoard {
            components: vec![make_comp(
                "U1",
                vec![
                    Pin::build("AVDD").pwr_fixed(3.3.volt(), 0.1.amp()).pin(),
                    Pin::build("VDD").pwr_fixed(3.3.volt(), 0.1.amp()).pin(),
                ],
                vec![Constraint::Decoupling {
                    values: vec![100.0.nf()],
                    per_pin: false,
                }],
            )],
            nets: vec![
                Net {
                    name: "AVDD".into(),
                    kind: NetKind::Power {
                        v_nom: 3.3.volt(),
                        ripple: None,
                    },
                    class: NetClass::default(),
                    constraints: vec![],
                },
                Net {
                    name: "VDD".into(),
                    kind: NetKind::Power {
                        v_nom: 3.3.volt(),
                        ripple: None,
                    },
                    class: NetClass::default(),
                    constraints: vec![],
                },
                Net {
                    name: "GND".into(),
                    kind: NetKind::Power {
                        v_nom: 0.0.volt(),
                        ripple: None,
                    },
                    class: NetClass::default(),
                    constraints: vec![],
                },
            ],
            connections: vec![
                Connection {
                    component: 0,
                    pin: "AVDD".into(),
                    net: NetId("AVDD".into()),
                },
                Connection {
                    component: 0,
                    pin: "VDD".into(),
                    net: NetId("VDD".into()),
                },
            ],
            constraints: vec![],
            width: 100.0,
            height: 80.0,
        };
        let (comps, _, _, _) = synthesise_decoupling(&board, DEFAULT_CAP_FOOTPRINT);
        assert_eq!(comps.len(), 2);
    }
}
