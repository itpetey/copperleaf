//! Golden-file tests: compile one representative board per parts crate and
//! compare the emitted KiCad project files against checked-in snapshots.
//!
//! These tests characterise the current compile + emit behaviour so that
//! refactors can be verified as byte-for-byte no-ops.  When behaviour is
//! *intentionally* changed, regenerate the snapshots with:
//!
//! ```sh
//! COPPERLEAF_BLESS=1 cargo test -p copperleaf-backend-kicad --test golden_board
//! ```

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use copperleaf::{Backend, Board, ComponentHandle, PinRef, Role, UnitExt};
use copperleaf_backend_kicad::KiCad;

const FILES: [&str; 4] = [
    "golden.kicad_pro",
    "golden.kicad_sch",
    "golden.kicad_pcb",
    "golden.net",
];
const PROJECT: &str = "golden";
/// Snapshot of every compiler warning, so warning regressions are reviewed.
const WARNINGS_FILE: &str = "golden.warnings";

/// Wire a component's power and ground pins into nets without hand-written
/// per-part knowledge:
///
/// - all `Gnd` pins are tied into one ground net;
/// - `PowerIn`/`PowerOut` pins are grouped by nominal voltage and tied
///   per-group (flexible pins without `v_nom` are left unconnected — a
///   power net without a voltage source is a compile error);
/// - NC pins and signal pins are left unconnected (ERC warnings only).
fn auto_wire(board: &mut Board, handle: ComponentHandle) {
    let mut groups: BTreeMap<(bool, i64), Vec<&'static str>> = BTreeMap::new();
    let pins = board.components[handle.0].component.pins();
    let identities = copperleaf::resolve_pin_identities(&board.components[handle.0].name, pins);
    for (pin, canonical) in pins.iter().zip(identities.pins.iter()) {
        if pin.nc() || pin.name() == "NC" || pin.name().starts_with("NC_") {
            continue;
        }
        let key = match pin.role() {
            Role::Gnd => (true, 0),
            Role::PowerIn | Role::PowerOut => {
                let Some(v) = pin.power_spec().v_nom else {
                    continue;
                };
                (false, (v.as_base() * 1000.0).round() as i64)
            }
            _ => continue,
        };
        let identity: &'static str = Box::leak(canonical.key.clone().into_boxed_str());
        groups.entry(key).or_default().push(identity);
    }
    for names in groups.values() {
        for pair in names.windows(2) {
            board
                .connect(handle.pin(PinRef(pair[0])), handle.pin(PinRef(pair[1])))
                .expect("auto_wire connect");
        }
    }
}

/// Compile `board`, assert its semantic invariants, emit it with the KiCad
/// backend, and compare all output files and compiler warnings against the
/// snapshots for `name`.
fn check_board(name: &str, board: Board) {
    let report = copperleaf_compile::run(board, &copperleaf_compile::CompileOptions::default())
        .unwrap_or_else(|e| panic!("{name} board failed to compile:\n{e}"));

    assert_board_invariants(name, &report);
    assert!(
        report.validation.is_empty(),
        "{name}: board validation findings:\n{:#?}",
        report.validation
    );

    let dir = tempfile::tempdir().unwrap();
    KiCad::new()
        .with_project_name(PROJECT)
        .emit(dir.path(), &report.board)
        .unwrap();

    let mut goldens = Vec::new();
    for file in FILES {
        let actual = std::fs::read_to_string(dir.path().join(file)).unwrap();
        goldens.push((golden_dir().join(name).join(file), actual));
    }
    goldens.push((
        golden_dir().join(name).join(WARNINGS_FILE),
        warning_snapshot(&report),
    ));
    report_failures(collect_failures(&goldens, blessing_enabled()));

    assert_unique_uuids(name, dir.path());
}

/// Every addressable object in an emitted project must have a distinct UUID
/// while remaining stable across runs.
fn assert_unique_uuids(name: &str, dir: &Path) {
    let mut seen = std::collections::HashMap::new();
    for file in FILES {
        let text = std::fs::read_to_string(dir.join(file)).unwrap();
        for (offset, _) in text.match_indices("(uuid \"") {
            let rest = &text[offset + "(uuid \"".len()..];
            let Some(end) = rest.find('"') else {
                continue;
            };
            let uuid = &rest[..end];
            if let Some(previous) = seen.insert(uuid.to_owned(), file) {
                panic!("{name}: duplicate UUID {uuid} in {file} (also in {previous})");
            }
        }
    }
}

/// Assert semantic invariants that byte-for-byte snapshots cannot express.
fn assert_board_invariants(name: &str, report: &copperleaf_compile::CompileReport) {
    let board = &report.board;
    assert!(
        board.width.is_finite() && board.width > 0.0,
        "{name}: board width must be finite and positive"
    );
    assert!(
        board.height.is_finite() && board.height > 0.0,
        "{name}: board height must be finite and positive"
    );

    let mut refdes = std::collections::HashSet::new();
    let mut pin_count = 0;
    for component in &board.components {
        assert!(
            !component.refdes.is_empty(),
            "{name}: component with empty refdes"
        );
        assert!(
            refdes.insert(component.refdes.as_str()),
            "{name}: duplicate refdes {}",
            component.refdes
        );
        for pin in &component.pins {
            assert!(
                !pin.name().is_empty(),
                "{name}: {} has a pin with an empty name",
                component.refdes
            );
            pin_count += 1;
        }
    }
    assert_eq!(
        report.summary.component_count,
        board.components.len(),
        "{name}: summary component count disagrees with board"
    );
    assert_eq!(
        report.summary.pin_count, pin_count,
        "{name}: summary pin count disagrees with board"
    );
    assert_eq!(
        report.summary.nets.len(),
        board.nets.len(),
        "{name}: summary net count disagrees with board"
    );

    let mut net_names = std::collections::HashSet::new();
    for net in &board.nets {
        assert!(
            !net.name.is_empty(),
            "{name}: net with empty name at a summary index"
        );
        assert!(
            net_names.insert(net.name.as_str()),
            "{name}: duplicate net name {}",
            net.name
        );
    }

    for conn in &board.connections {
        let component = board
            .components
            .get(conn.component)
            .unwrap_or_else(|| panic!("{name}: connection references unknown component"));
        assert!(
            component
                .pins
                .get(conn.pin_index)
                .is_some_and(|pin| pin.id() == &conn.pin),
            "{name}: connection references unknown pin {} on {}",
            conn.pin.0,
            component.refdes
        );
        assert!(
            conn.net.0 < board.nets.len(),
            "{name}: connection references unknown net index {}",
            conn.net.0
        );
    }
}

/// Render all compiler warnings as a stable, reviewable snapshot.
fn warning_snapshot(report: &copperleaf_compile::CompileReport) -> String {
    let mut lines: Vec<String> = report
        .warnings
        .iter()
        .map(|diagnostic| {
            format!(
                "{:?}|{}|{}|{}|{}",
                diagnostic.severity,
                diagnostic.code,
                diagnostic.message,
                diagnostic.entities.join(","),
                diagnostic.hint.as_deref().unwrap_or("")
            )
        })
        .collect();
    lines.sort();
    lines.join("\n") + "\n"
}

/// Returns true only when `COPPERLEAF_BLESS` is exactly `1`.
fn blessing_enabled_from(value: Option<&str>) -> bool {
    value == Some("1")
}

/// Whether snapshots should be written rather than compared.
fn blessing_enabled() -> bool {
    blessing_enabled_from(std::env::var("COPPERLEAF_BLESS").ok().as_deref())
}

/// Compare every golden, or write them when blessing.  All failures are
/// collected so one run reports every missing or stale snapshot.
fn collect_failures(goldens: &[(PathBuf, String)], bless: bool) -> Vec<String> {
    let mut failures = Vec::new();
    for (path, actual) in goldens {
        if bless {
            if let Some(parent) = path.parent()
                && let Err(e) = std::fs::create_dir_all(parent)
            {
                failures.push(format!("cannot create {}: {e}", parent.display()));
                continue;
            }
            if let Err(e) = std::fs::write(path, actual) {
                failures.push(format!("cannot write {}: {e}", path.display()));
            }
            continue;
        }
        match std::fs::read_to_string(path) {
            Ok(expected) if &expected == actual => {}
            Ok(_) => failures.push(format!("golden mismatch: {}", path.display())),
            Err(e) => failures.push(format!("missing golden {}: {e}", path.display())),
        }
    }
    failures
}

/// Fail with every collected golden problem at once.
fn report_failures(failures: Vec<String>) {
    if !failures.is_empty() {
        panic!(
            "{} golden failure(s) — run with COPPERLEAF_BLESS=1 to create or update:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}

#[test]
fn golden_board_connectors() {
    use copperleaf_parts_connectors::{
        Arjm11d7502AbEw2, Conmhf4SmdGT, S2bPhSm4TbLfSn, UsbC23409011,
    };
    let mut board = Board::new(PROJECT);
    let j1 = board.add("J1", Arjm11d7502AbEw2::new());
    auto_wire(&mut board, j1);
    let j2 = board.add("J2", Conmhf4SmdGT::new());
    auto_wire(&mut board, j2);
    let j3 = board.add("J3", S2bPhSm4TbLfSn::new());
    auto_wire(&mut board, j3);
    let j4 = board.add("J4", UsbC23409011::new());
    auto_wire(&mut board, j4);
    check_board("connectors", board);
}

#[test]
fn golden_board_microchip() {
    let mut board = Board::new(PROJECT);
    let u1 = board.add("U1", copperleaf_parts_microchip::Mcp73831t2atiOt::new());
    auto_wire(&mut board, u1);
    check_board("microchip", board);
}

#[test]
fn golden_board_morsemicro() {
    let mut board = Board::new(PROJECT);
    let u1 = board.add("U1", copperleaf_parts_morsemicro::Mm8108Mf15457::new());
    auto_wire(&mut board, u1);
    check_board("morsemicro", board);
}

#[test]
fn golden_board_passives() {
    use copperleaf_parts_passives::{
        B82472p6152m000, B82472p6222m000, Capacitor, Crystal, Resistor, footprint::Package,
    };
    let mut board = Board::new(PROJECT);
    let c1 = board.add("C1", Capacitor::new(100.0.nf(), Package::M1608));
    auto_wire(&mut board, c1);
    let r1 = board.add("R1", Resistor::new(10.0.kohm(), Package::M1608));
    auto_wire(&mut board, r1);
    let y1 = board.add("Y1", Crystal::new(25.0.mhz()));
    auto_wire(&mut board, y1);
    let l1 = board.add("L1", B82472p6152m000::new());
    auto_wire(&mut board, l1);
    let l2 = board.add("L2", B82472p6222m000::new());
    auto_wire(&mut board, l2);
    check_board("passives", board);
}

#[test]
fn golden_board_raspberrypi() {
    let mut board = Board::new(PROJECT);
    let u1 = board.add("U1", copperleaf_parts_raspberrypi::Rp2354a::new());
    auto_wire(&mut board, u1);
    check_board("raspberrypi", board);
}

#[test]
fn golden_board_texas_instruments() {
    let mut board = Board::new(PROJECT);
    let u1 = board.add(
        "U1",
        copperleaf_parts_texas_instruments::Tps63031dskr::new(),
    );
    auto_wire(&mut board, u1);
    check_board("texas-instruments", board);
}

#[test]
fn golden_board_wiznet() {
    let mut board = Board::new(PROJECT);
    let u1 = board.add("U1", copperleaf_parts_wiznet::W5500::new());
    auto_wire(&mut board, u1);
    check_board("wiznet", board);
}

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/board")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blessing_requires_exact_one() {
        assert!(blessing_enabled_from(Some("1")));
        assert!(!blessing_enabled_from(Some("0")));
        assert!(!blessing_enabled_from(Some("true")));
        assert!(!blessing_enabled_from(None));
    }

    #[test]
    fn all_missing_goldens_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        let goldens: Vec<_> = ["golden.kicad_pro", "golden.kicad_sch", "golden.warnings"]
            .iter()
            .map(|name| (dir.path().join(name), String::new()))
            .collect();
        let failures = collect_failures(&goldens, false);
        assert_eq!(failures.len(), 3, "{failures:?}");
        for failure in failures {
            assert!(failure.starts_with("missing golden "), "{failure}");
        }
    }
}
