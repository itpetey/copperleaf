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
    net::{Constraint, Net, NetClass, NetIdx, NetKind},
    pin::{Pin, PinHandle, RawConnection, Role, SigSpec},
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
    /// Build the grouping from the given connections and single-pin nets.
    fn build(connections: &[RawConnection], single_pin_nets: &[(usize, PinHandle)]) -> Self {
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

        // Add single-pin nets as single-node entries.
        for &(_, pin) in single_pin_nets {
            if let std::collections::hash_map::Entry::Vacant(e) =
                pin_to_node.entry((pin.component, pin.pin))
            {
                e.insert(nodes.len());
                nodes.push((pin.component, pin.pin));
            }
        }

        let mut uf = UnionFind::new(nodes.len());
        for conn in connections {
            let a = pin_to_node[&(conn.from.component, conn.from.pin)];
            let b = pin_to_node[&(conn.to.component, conn.to.pin)];
            uf.union(a, b);
        }
        // Single-pin nets are already isolated (no union needed).

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

    /// Return the set of connection-edge ids whose pins belong to the net
    /// identified by `rep`.
    fn edges_for_net(&self, rep: usize, connections: &[RawConnection]) -> Vec<usize> {
        let mut edge_ids = Vec::new();
        for conn in connections {
            let a = (conn.from.component, conn.from.pin);
            let b = (conn.to.component, conn.to.pin);
            let a_node = self.pin_to_node[&a];
            if self.roots[a_node] == rep {
                edge_ids.push(conn.id);
                continue;
            }
            let b_node = self.pin_to_node[&b];
            if self.roots[b_node] == rep {
                edge_ids.push(conn.id);
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
    let grouping = NetGrouping::build(&board.connections, &board.single_pin_nets);

    let mut lowering_errors: Vec<Diagnostic> = Vec::new();

    // Build override lookups for single-pin nets keyed by (component, pin).
    let single_net_overrides: BTreeMap<(usize, &str), RawNetOverride> = board
        .single_pin_nets
        .iter()
        .map(|&(id, pin)| {
            let ov = board.net_overrides.get(&id).cloned().unwrap_or_default();
            ((pin.component, pin.pin), ov)
        })
        .collect();

    let (nets, connections) = build_nets_and_connections(
        &grouping,
        &board.connections,
        &board.net_overrides,
        &single_net_overrides,
        &board.components,
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
/// [`Connection`]s.  Each net is resolved in one pass via [`resolve_net`].
fn build_nets_and_connections(
    grouping: &NetGrouping,
    connections: &[RawConnection],
    overrides: &BTreeMap<usize, RawNetOverride>,
    single_net_overrides: &BTreeMap<(usize, &str), RawNetOverride>,
    components: &[ComponentEntry],
    compiled: &[CompiledComponent],
    errors: &mut Vec<Diagnostic>,
) -> (Vec<Net>, Vec<Connection>) {
    let mut nets: Vec<Net> = Vec::new();
    let mut out_connections: Vec<Connection> = Vec::new();

    for (&rep, members) in &grouping.groups {
        let (name, kind) = resolve_net(
            rep,
            members,
            grouping,
            connections,
            overrides,
            single_net_overrides,
            components,
            compiled,
            errors,
        );

        nets.push(Net {
            name,
            kind,
            class: NetClass::default(),
            constraints: vec![],
        });
        let net_idx = NetIdx(nets.len() - 1);

        for &node_idx in members {
            let (comp_idx, pin_name) = grouping.nodes[node_idx];
            out_connections.push(Connection {
                component: comp_idx,
                pin: pin_name.to_owned(),
                net: net_idx,
            });
        }
    }

    (nets, out_connections)
}

/// Build the [`CompileSummary`] from the final board.
fn build_summary(board: &CompiledBoard) -> CompileSummary {
    CompileSummary {
        nets: board
            .nets
            .iter()
            .enumerate()
            .map(|(i, n)| NetInfo {
                name: n.name.clone(),
                kind: n.kind.clone(),
                pin_count: board.connections.iter().filter(|c| c.net.0 == i).count(),
            })
            .collect(),
        pin_count: board.components.iter().map(|c| c.pins.len()).sum(),
        component_count: board.components.len(),
    }
}

/// Resolve one net's name and kind in a single pass.
///
/// Precedence, in order:
///   1. explicit override (voltage/name from `NetHandle`)
///   2. power-pin `v_nom` consensus (conflict → `NET:VOLTAGE_CONFLICT`)
///   3. ground fallback (`Gnd`-role pins imply 0 V)
///   4. power net with no voltage → `NET:NO_VOLTAGE_SOURCE` error
#[allow(clippy::too_many_arguments)]
fn resolve_net(
    rep: usize,
    members: &[usize],
    grouping: &NetGrouping,
    connections: &[RawConnection],
    overrides: &BTreeMap<usize, RawNetOverride>,
    single_net_overrides: &BTreeMap<(usize, &str), RawNetOverride>,
    components: &[ComponentEntry],
    compiled: &[CompiledComponent],
    errors: &mut Vec<Diagnostic>,
) -> (String, NetKind) {
    // --- Explicit overrides from the edges that belong to this net ---
    let edge_ids = grouping.edges_for_net(rep, connections);
    let mut explicit_voltage: Option<Qty<Volt>> = None;
    let mut explicit_name: Option<String> = None;
    for &eid in &edge_ids {
        if let Some(ov) = overrides.get(&eid) {
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

    // For single-pin nets (no edges), check the single-net override map.
    if edge_ids.is_empty() && members.len() == 1 {
        let node_idx = members[0];
        let (comp_idx, pin_name) = grouping.nodes[node_idx];
        if let Some(ov) = single_net_overrides.get(&(comp_idx, pin_name)) {
            if let Some(v) = ov.voltage {
                explicit_voltage = Some(v);
            }
            if let Some(name) = &ov.name {
                explicit_name = Some(name.clone());
            }
        }
    }

    // --- Net name (override → auto-generated) ---
    let name = explicit_name.unwrap_or_else(|| {
        let (comp, pin) = grouping.nodes[rep];
        let comp_name = &components[comp].name;
        format!("NET_{}_{}", comp_name, pin)
    });

    // --- Infer voltage from connected pin v_nom values ---
    let mut inferred: Option<Qty<Volt>> = None;
    let mut ground: Option<Qty<Volt>> = None;
    for &node_idx in members {
        let (comp_idx, pin_name) = grouping.nodes[node_idx];
        let comp = &compiled[comp_idx];
        let pin = comp.pins.iter().find(|p| p.name() == pin_name).unwrap();
        let v = pin.power_spec().v_nom;
        if matches!(pin.role(), Role::Gnd) {
            ground = v.or(Some(0.0.volt()));
            continue;
        }
        let Some(v) = v else { continue };
        if let Some(existing) = inferred {
            if (existing.as_base() - v.as_base()).abs() > 1e-9 {
                errors.push(Diagnostic {
                    code: "NET:VOLTAGE_CONFLICT".into(),
                    severity: Severity::Error,
                    message: format!(
                        "conflicting v_nom values on net '{}' ({:.2}V vs {:.2}V)",
                        name,
                        existing.as_base(),
                        v.as_base()
                    ),
                    entities: vec![name.clone()],
                    hint: Some("check connected power pins".into()),
                });
            }
        } else {
            inferred = Some(v);
        }
    }
    // Override takes precedence over inferred; ground is fallback.
    let voltage = explicit_voltage.or(inferred).or(ground);

    // --- Classify: power vs signal ---
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

    let kind = if is_power {
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
                        name
                    ),
                    entities: vec![name.clone()],
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
    };

    (name, kind)
}

/// Type-erase [`Component`] trait objects into [`CompiledComponent`]s with
/// deterministic pin IDs.
fn compile_components(entries: &[ComponentEntry]) -> Vec<CompiledComponent> {
    entries
        .iter()
        .map(|entry| CompiledComponent::from_component(&entry.name, entry.component.as_ref()))
        .collect()
}

fn ground_net_idx_and_fallback(board: &CompiledBoard) -> (NetIdx, Option<Net>) {
    if let Some(idx) = board.find_net("GND") {
        return (idx, None);
    }
    if let Some(idx) = board.nets.iter().position(|n| n.is_ground()) {
        return (NetIdx(idx), None);
    }
    // Fallback: fresh ground net will be appended at current length.
    let idx = NetIdx(board.nets.len());
    let net = Net {
        name: "GND".into(),
        kind: NetKind::Power {
            v_nom: 0.0.volt(),
            ripple: None,
        },
        class: NetClass::default(),
        constraints: vec![],
    };
    (idx, Some(net))
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

#[allow(clippy::too_many_arguments)]
fn place_decoupling_set(
    board: &CompiledBoard,
    comp_idx: usize,
    comp: &CompiledComponent,
    pin: &Pin,
    values: &[Qty<Farad>],
    package: Package,
    gnd_net_idx: NetIdx,
    components: &mut Vec<CompiledComponent>,
    connections: &mut Vec<Connection>,
    diagnostics: &mut Vec<Diagnostic>,
    next_c: &mut u32,
) {
    let net_idx = board
        .connections
        .iter()
        .find(|c| c.component == comp_idx && c.pin == pin.name())
        .map(|c| c.net);

    let Some(net_idx) = net_idx else {
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
    if board.net(net_idx).is_ground() {
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
            net: net_idx,
        });
        connections.push(Connection {
            component: comp_idx_in_final,
            pin: "2".into(),
            net: gnd_net_idx,
        });
    }
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

    let (gnd_net_idx, fallback_gnd_net) = ground_net_idx_and_fallback(board);

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
                        gnd_net_idx,
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
                let mut pins_by_net: BTreeMap<NetIdx, Vec<&Pin>> = BTreeMap::new();
                for pin in power_pins {
                    let net_idx = board
                        .connections
                        .iter()
                        .find(|c| c.component == comp_idx && c.pin == pin.name())
                        .map(|c| c.net);
                    if let Some(idx) = net_idx {
                        pins_by_net.entry(idx).or_default().push(pin);
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
                            gnd_net_idx,
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
    use copperleaf::units::UnitExt;

    #[test]
    fn synthesises_decoupling_caps() {
        let board = CompiledBoard {
            components: vec![CompiledComponent::test_with(
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
                net: NetIdx(0),
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
            components: vec![CompiledComponent::test_with(
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
                net: NetIdx(0),
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
            components: vec![CompiledComponent::test_with(
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
                    net: NetIdx(0),
                },
                Connection {
                    component: 0,
                    pin: "VDD_USB".into(),
                    net: NetIdx(1),
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
            components: vec![CompiledComponent::test_with(
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
                    net: NetIdx(0),
                },
                Connection {
                    component: 0,
                    pin: "VDD".into(),
                    net: NetIdx(1),
                },
            ],
            constraints: vec![],
            width: 100.0,
            height: 80.0,
        };
        let (comps, _, _, _) = synthesise_decoupling(&board, DEFAULT_CAP_FOOTPRINT);
        assert_eq!(comps.len(), 2);
    }

    #[test]
    fn single_pin_net_interleaved_with_connect_preserves_overrides() {
        use copperleaf::PinRef;

        struct TwoPins;
        impl copperleaf::Component for TwoPins {
            fn pins(&self) -> &[Pin] {
                static PINS: std::sync::OnceLock<Vec<Pin>> = std::sync::OnceLock::new();
                PINS.get_or_init(|| {
                    vec![
                        Pin::build("A").pwr_fixed(3.3.volt(), 0.1.amp()).pin(),
                        Pin::build("B").pwr_fixed(3.3.volt(), 0.1.amp()).pin(),
                        Pin::build("C").pwr_fixed(5.0.volt(), 0.1.amp()).pin(),
                    ]
                })
            }
        }
        impl TwoPins {
            const A: PinRef = PinRef("A");
            const B: PinRef = PinRef("B");
            const C: PinRef = PinRef("C");
        }

        let mut board = Board::new("t");
        let u1 = board.add("U1", TwoPins);

        // Interleave: net() before connect() — Phase 4 key scheme must not alias.
        let single = board.net(u1.pin(TwoPins::C)).unwrap();
        board.set_net_name(single, "FIVE_VOLT");

        let pair = board
            .connect(u1.pin(TwoPins::A), u1.pin(TwoPins::B))
            .unwrap();
        board.set_net_name(pair, "PAIR_NET");

        let report = crate::run(board, &CompileOptions::default()).expect("compiles");

        let names: Vec<&str> = report.board.nets.iter().map(|n| n.name.as_str()).collect();
        assert!(
            names.contains(&"FIVE_VOLT"),
            "single-pin net lost its name override: {names:?}"
        );
        assert!(
            names.contains(&"PAIR_NET"),
            "connect net lost its name override: {names:?}"
        );
    }
}
