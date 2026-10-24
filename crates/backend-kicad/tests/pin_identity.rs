//! Regression: repeated display names identify distinct physical pins.
//!
//! The W5500 exposes six pins named `AVDD` with different pad numbers. A
//! connection must address exactly one of them; the others stay unconnected.

use copperleaf::{Board, BoardView, PinId, PinRef, UnitExt, pin::resolve_pin_identities};

#[test]
fn display_name_lookup_is_rejected_when_it_is_not_a_canonical_identity() {
    let mut board = Board::new("w5500_identity");
    let u1 = board.add("U1", copperleaf_parts_wiznet::W5500::new());
    let r1 = board.add(
        "R1",
        copperleaf_parts_passives::Resistor::new(
            10.0.kohm(),
            copperleaf_parts_passives::footprint::Package::M1608,
        ),
    );

    let error = board
        .connect(u1.pin(PinRef("AVDD")), r1.pin(PinRef("1")))
        .expect_err("display-name lookup must not resolve");
    assert_eq!(error.errors[0].code, "BOARD:INVALID_PIN");
    assert!(error.errors[0].message.contains("AVDD"));
}

/// Return the net name attached to a PCB pad, scanning that pad's block.
fn pad_net(pcb: &str, number: &str) -> Option<String> {
    let start = pcb.find(&format!("(pad \"{number}\""))?;
    let rest = &pcb[start..];
    let end = rest[1..]
        .find("(pad \"")
        .map(|offset| offset + 1)
        .unwrap_or(rest.len());
    let block = &rest[..end];
    let net_start = block.find("(net ")?;
    let net_block = &block[net_start..];
    let end = net_block.find(')')?;
    let quoted = net_block[..end].split_once('"')?.1;
    Some(quoted.split('"').next()?.to_string())
}

#[test]
fn w5500_repeated_avdd_pads_connect_independently() {
    let mut board = Board::new("w5500_identity");
    let u1 = board.add("U1", copperleaf_parts_wiznet::W5500::new());
    let r1 = board.add(
        "R1",
        copperleaf_parts_passives::Resistor::new(
            10.0.kohm(),
            copperleaf_parts_passives::footprint::Package::M1608,
        ),
    );

    // Pad 4 is addressed by its canonical identity, not its display name.
    let avdd_net = board
        .connect(u1.pin(PinRef("4")), r1.pin(PinRef("1")))
        .expect("pad 4 connection should resolve");
    board.set_net_voltage(avdd_net, 3.3.volt());

    let report = copperleaf_compile::run(board, &copperleaf_compile::CompileOptions::default())
        .expect("W5500 identity fixture compiles");

    let w5500 = &report.board.components[0];
    let resolution = resolve_pin_identities("U1", &w5500.pins);
    let avdd: Vec<(usize, &str)> = w5500
        .pins
        .iter()
        .enumerate()
        .filter(|(_, pin)| pin.name() == "AVDD")
        .map(|(index, _pin)| (index, resolution.pins[index].key.as_str()))
        .collect();
    assert!(avdd.len() >= 2, "W5500 fixture needs repeated AVDD pins");
    let (pad4_index, _) = avdd
        .iter()
        .find(|(_, key)| *key == "4")
        .copied()
        .expect("AVDD pad 4");
    let (pad8_index, _) = avdd
        .iter()
        .find(|(_, key)| *key == "8")
        .copied()
        .expect("AVDD pad 8");

    let view = BoardView::new(&report.board);
    assert!(
        view.connected.contains(&(0, pad4_index)),
        "connected AVDD pad 4 must be part of its net"
    );
    assert!(
        !view.connected.contains(&(0, pad8_index)),
        "unconnected AVDD pad 8 must stay unassigned"
    );
    assert!(
        view.net_of.contains_key(&(0, pad4_index)),
        "AVDD pad 4 must resolve to one net"
    );
    assert!(
        !view.net_of.contains_key(&(0, pad8_index)),
        "AVDD pad 8 must not inherit pad 4's net"
    );

    let pad4_connections: Vec<_> = report
        .board
        .connections
        .iter()
        .filter(|conn| conn.component == 0 && conn.pin_index == pad4_index)
        .collect();
    assert_eq!(pad4_connections.len(), 1);
    assert_eq!(pad4_connections[0].pin, w5500.pins[pad4_index].id().clone());
    assert_ne!(w5500.pins[pad4_index].id(), w5500.pins[pad8_index].id());
    assert!(w5500.pins[pad4_index].id() != &PinId("4".into()));
}

#[test]
fn w5500_repeated_pads_agree_across_schematic_netlist_and_pcb() {
    use copperleaf::Backend;

    let mut board = Board::new("w5500_parity");
    let u1 = board.add("U1", copperleaf_parts_wiznet::W5500::new());
    let r1 = board.add(
        "R1",
        copperleaf_parts_passives::Resistor::new(
            10.0.kohm(),
            copperleaf_parts_passives::footprint::Package::M1608,
        ),
    );

    // Two repeated-name AGND pads are connected independently; pad 14 stays
    // unconnected and must not acquire either net.
    let first = board
        .connect(u1.pin(PinRef("3")), r1.pin(PinRef("1")))
        .expect("AGND pad 3 resolves");
    board.set_net_voltage(first, 0.0.volt());
    let second = board
        .connect(u1.pin(PinRef("9")), r1.pin(PinRef("2")))
        .expect("AGND pad 9 resolves");
    board.set_net_voltage(second, 0.0.volt());

    let report = copperleaf_compile::run(board, &copperleaf_compile::CompileOptions::default())
        .expect("parity fixture compiles");

    let dir = tempfile::tempdir().unwrap();
    copperleaf_backend_kicad::KiCad::new()
        .with_project_name("parity")
        .emit(dir.path(), &report.board)
        .expect("emits");

    let netlist = std::fs::read_to_string(dir.path().join("parity.net")).unwrap();
    let pcb = std::fs::read_to_string(dir.path().join("parity.kicad_pcb")).unwrap();
    let schematic = std::fs::read_to_string(dir.path().join("parity.kicad_sch")).unwrap();

    // Netlist: exactly pads 3 and 9 appear as AGND nodes, each as its own node.
    let agnd_nodes: Vec<&str> = netlist
        .match_indices("(pinfunction \"AGND\")")
        .map(|(index, _)| {
            let before = &netlist[..index];
            let pin = before.rfind("(pin \"").expect("node pin");
            let tail = &before[pin + 6..];
            tail.split('"').next().unwrap()
        })
        .collect();
    assert_eq!(agnd_nodes, ["3", "9"], "AGND nodes: {agnd_nodes:?}");
    assert!(
        !netlist.contains("(pin \"14\")"),
        "unconnected AGND pad 14 emitted a node"
    );

    // PCB: connected pads carry a net, the unconnected pad does not.
    assert!(pad_net(&pcb, "3").is_some(), "pad 3 must carry its net");
    assert!(pad_net(&pcb, "9").is_some(), "pad 9 must carry its net");
    assert!(pad_net(&pcb, "14").is_none(), "pad 14 must have no net");

    // Schematic: one wire and label per connected endpoint (U1 pads 3/9 and
    // R1 pads 1/2), with no wire for the unconnected pad.
    let wires = schematic.matches("(wire").count();
    let labels = schematic.matches("(label ").count();
    assert_eq!(wires, 4, "expected one schematic wire per connected pad");
    assert_eq!(labels, 4, "expected one schematic label per connected pad");
}
