use std::collections::HashMap;

use crate::{
    Component,
    compiled::{
        CompileError, CompileReport, CompileSummary, CompiledBoard, CompiledComponent, Connection,
        NetInfo,
    },
    erc::{erc_floating_inputs, erc_floating_power_inputs, erc_nc_pin_connected, erc_overvoltage},
    net::{Constraint, Net, NetHandle, NetId, NetKind},
    pin::{Pin, PinHandle, PinId, PinRef, RawConnection, Role, SigSpec},
    synthesis::synthesize_decoupling,
    units::{Diagnostic, Qty, Severity, UnitExt, Volt},
    util::{UnionFind, deterministic_id},
};

/// Handle to a component instance on a [`Board`].
#[derive(Clone, Copy, Debug)]
pub struct ComponentHandle(pub usize);

/// Top level structure representing the PCB being designed.
pub struct Board {
    pub(crate) components: Vec<ComponentEntry>,
    pub(crate) connections: Vec<RawConnection>,
    pub(crate) net_overrides: Vec<RawNetOverride>,
    pub(crate) next_edge: usize,
}

pub(crate) struct ComponentEntry {
    pub(crate) name: String,
    pub(crate) component: Box<dyn Component>,
}

#[derive(Clone, Debug)]
pub(crate) struct RawNetOverride {
    pub(crate) voltage: Option<Qty<Volt>>,
    pub(crate) name: Option<String>,
}

/// Intermediate data structure produced by a union-find pass over the raw
/// connections.  Maps connected pins into equivalence classes (nets).
struct NetGrouping {
    pin_to_node: HashMap<(usize, &'static str), usize>,
    nodes: Vec<(usize, &'static str)>,
    groups: HashMap<usize, Vec<usize>>,
}

impl ComponentHandle {
    /// Create a [`PinHandle`] for a pin on this component.
    pub fn pin(&self, pin: PinRef) -> PinHandle {
        PinHandle {
            component: self.0,
            pin: pin.0,
        }
    }
}

impl Board {
    /// Set an explicit voltage override for a net returned by [`Board::connect`].
    pub fn set_net_voltage(&mut self, handle: NetHandle, v: Qty<Volt>) {
        if let Some(ov) = self.net_overrides.get_mut(handle.edge) {
            ov.voltage = Some(v);
        }
    }

    /// Set an explicit name override for a net returned by [`Board::connect`].
    pub fn set_net_name(&mut self, handle: NetHandle, name: &str) {
        if let Some(ov) = self.net_overrides.get_mut(handle.edge) {
            ov.name = Some(name.to_owned());
        }
    }
}

impl Board {
    /// Creates a new, unpopulated [`Board`].
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            connections: Vec::new(),
            net_overrides: Vec::new(),
            next_edge: 0,
        }
    }

    /// Add a [`Component`] to this board.
    pub fn add<C: Component + 'static>(&mut self, name: &str, component: C) -> ComponentHandle {
        let idx = self.components.len();
        self.components.push(ComponentEntry {
            name: name.to_owned(),
            component: Box::new(component),
        });
        ComponentHandle(idx)
    }

    /// Connect one [`PinHandle`] to another.
    ///
    /// Returns a [`NetHandle`] that can be used to annotate the emerging net.
    pub fn connect(&mut self, from: PinHandle, to: PinHandle) -> Result<NetHandle, CompileError> {
        if let Some(diag) = self.validate_pin(&from) {
            return Err(CompileError::new(vec![diag]));
        }
        if let Some(diag) = self.validate_pin(&to) {
            return Err(CompileError::new(vec![diag]));
        }
        let edge = self.next_edge;
        self.next_edge += 1;
        self.connections.push(RawConnection { from, to });
        self.net_overrides.push(RawNetOverride::default());
        Ok(NetHandle { edge })
    }

    fn validate_pin(&self, handle: &PinHandle) -> Option<Diagnostic> {
        let Some(entry) = self.components.get(handle.component) else {
            return Some(Diagnostic {
                code: "BOARD:INVALID_COMPONENT".into(),
                severity: Severity::Error,
                message: format!("component index {} does not exist", handle.component),
                entities: vec![format!("{}", handle.component)],
                hint: None,
            });
        };
        if entry.component.pin_name(handle.pin).is_none() {
            return Some(Diagnostic {
                code: "BOARD:INVALID_PIN".into(),
                severity: Severity::Error,
                message: format!(
                    "pin '{}' does not exist on component '{}'",
                    handle.pin, entry.name
                ),
                entities: vec![format!("{}.{}", entry.name, handle.pin)],
                hint: None,
            });
        }
        None
    }

    /// Compiles this board design into an electronically correct model for export via
    /// any [`Backend`](crate::Backend).
    pub fn compile(self) -> Result<CompileReport, CompileError> {
        self.validate_connections()?;

        let compiled = compile_components(&self.components);
        let grouping = NetGrouping::build(&self.connections);

        let mut errors: Vec<Diagnostic> = Vec::new();

        let (net_names, net_voltages) = resolve_net_overrides(
            &grouping,
            &self.connections,
            &self.net_overrides,
            &self.components,
            &compiled,
            &mut errors,
        );

        let (nets, connections) = build_nets_and_connections(
            &grouping,
            &net_names,
            &net_voltages,
            &compiled,
            &mut errors,
        );

        let constraints: Vec<Constraint> = compiled
            .iter()
            .flat_map(|c| c.constraints.clone())
            .collect();

        let board = CompiledBoard {
            components: compiled,
            nets,
            connections,
            constraints,
        };

        // Run ERC checks (warnings are always collected; errors are fatal).
        let (mut warnings, erc_errors) = run_erc(&board, &self.connections);
        errors.extend(erc_errors);

        if !errors.is_empty() {
            return Err(CompileError::new(errors));
        }

        // Synthesis only runs when the board is electrically valid.
        let (synth_components, synth_caps, synth_diags) = synthesize_decoupling(&board);
        warnings.extend(synth_diags);

        let mut final_components = board.components;
        final_components.extend(synth_components);

        let final_board = CompiledBoard {
            components: final_components,
            nets: board.nets,
            connections: board.connections,
            constraints: board.constraints,
        };

        let summary = build_summary(&final_board, synth_caps);

        Ok(CompileReport {
            board: final_board,
            warnings,
            summary,
        })
    }

    /// Validate that every [`PinHandle`] in a connection refers to an existing
    /// component and pin.
    fn validate_connections(&self) -> Result<(), CompileError> {
        for conn in &self.connections {
            if let Some(diag) = self.validate_pin(&conn.from) {
                return Err(CompileError::new(vec![diag]));
            }
            if let Some(diag) = self.validate_pin(&conn.to) {
                return Err(CompileError::new(vec![diag]));
            }
        }
        Ok(())
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for RawNetOverride {
    fn default() -> Self {
        Self {
            voltage: None,
            name: None,
        }
    }
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

        let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
        for i in 0..nodes.len() {
            let root = uf.find(i);
            groups.entry(root).or_default().push(i);
        }

        Self {
            pin_to_node,
            nodes,
            groups,
        }
    }

    /// Return the set of connection-edge indices whose pins belong to the net
    /// identified by `rep`.
    fn edges_for_net(&self, rep: usize, connections: &[RawConnection]) -> Vec<usize> {
        let mut edge_ids = Vec::new();
        for (edge_id, conn) in connections.iter().enumerate() {
            let a = (conn.from.component, conn.from.pin);
            let b = (conn.to.component, conn.to.pin);
            let a_rep = self.pin_to_node[&a];
            if a_rep == rep {
                edge_ids.push(edge_id);
                continue;
            }
            let b_rep = self.pin_to_node[&b];
            if b_rep == rep {
                edge_ids.push(edge_id);
            }
        }
        edge_ids
    }
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
            class: crate::net::NetClass::default(),
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

fn build_summary(
    board: &CompiledBoard,
    synth_caps: Vec<crate::compiled::SynthCap>,
) -> CompileSummary {
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
        .map(|entry| {
            let pins: Vec<Pin> = entry
                .component
                .pins()
                .iter()
                .map(|p| {
                    let seed = format!("{}:{}", entry.name, p.name());
                    p.clone().with_id(PinId(deterministic_id(&seed)))
                })
                .collect();
            CompiledComponent {
                refdes: entry.name.clone(),
                pins,
                constraints: entry.component.constraints(),
                symbol: entry.component.symbol().map(|s| s.to_owned()),
                footprint: entry.component.footprint().map(|s| s.to_owned()),
            }
        })
        .collect()
}

/// Collect `v_nom` from every power pin on the net.  Returns `None` if no pin
/// provides a nominal voltage, or emits an error on conflict.
fn infer_voltage_from_pins(
    members: &[usize],
    nodes: &[(usize, &'static str)],
    compiled: &[CompiledComponent],
    net_name: &str,
    errors: &mut Vec<Diagnostic>,
) -> Option<Qty<Volt>> {
    let mut inferred: Option<Qty<Volt>> = None;

    for &node_idx in members {
        let (comp_idx, pin_name) = nodes[node_idx];
        let comp = &compiled[comp_idx];
        let pin = comp.pins.iter().find(|p| p.name() == pin_name).unwrap();

        if let Some(v) = pin.power_spec().v_nom {
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
    }

    inferred
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

/// Determine the name and voltage for every net by merging explicit overrides
/// with values inferred from connected pins.
///
/// Returns `(net_names, net_voltages)` keyed by the group representative id.
/// Diagnostics are appended to `errors`.
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

        // Determine net name (override → auto-generated).
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

/// Run all electrical-rule checks.  Returns `(warnings, errors)`.
fn run_erc(
    board: &CompiledBoard,
    connections: &[RawConnection],
) -> (Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    warnings.extend(erc_floating_inputs(board, connections));
    warnings.extend(erc_floating_power_inputs(board, connections));
    errors.extend(erc_overvoltage(board));
    errors.extend(erc_nc_pin_connected(board, connections));

    (warnings, errors)
}
