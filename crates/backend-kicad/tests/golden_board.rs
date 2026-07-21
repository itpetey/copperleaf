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

const PROJECT: &str = "golden";
const FILES: [&str; 4] = [
    "golden.kicad_pro",
    "golden.kicad_sch",
    "golden.kicad_pcb",
    "golden.net",
];

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/board")
}

/// Compare `actual` against the golden file at `path`, or overwrite the
/// golden file when running with `COPPERLEAF_BLESS=1`.
fn compare_or_bless(path: &Path, actual: &str) {
    if std::env::var_os("COPPERLEAF_BLESS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(path).unwrap_or_else(|e| {
        panic!(
            "missing golden {}: {e} — run with COPPERLEAF_BLESS=1 to create it",
            path.display()
        )
    });
    assert_eq!(
        expected, actual,
        "golden mismatch: {} — run with COPPERLEAF_BLESS=1 to update",
        path.display()
    );
}

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
    for pin in board.components[handle.0].component.pins() {
        if pin.name() == "NC" || pin.name().starts_with("NC_") {
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
        let name: &'static str = Box::leak(pin.name().to_string().into_boxed_str());
        groups.entry(key).or_default().push(name);
    }
    for names in groups.values() {
        for pair in names.windows(2) {
            board
                .connect(handle.pin(PinRef(pair[0])), handle.pin(PinRef(pair[1])))
                .expect("auto_wire connect");
        }
    }
}

/// Compile `board`, emit it with the KiCad backend, and compare all four
/// output files against the snapshots for `name`.
fn check_board(name: &str, board: Board) {
    let report = copperleaf_compile::run(board, &copperleaf_compile::CompileOptions::default())
        .unwrap_or_else(|e| panic!("{name} board failed to compile:\n{e}"));

    let dir = tempfile::tempdir().unwrap();
    KiCad::new()
        .with_project_name(PROJECT)
        .emit(dir.path(), &report.board)
        .unwrap();

    for file in FILES {
        let actual = std::fs::read_to_string(dir.path().join(file)).unwrap();
        compare_or_bless(&golden_dir().join(name).join(file), &actual);
    }
}

#[test]
fn golden_board_connectors() {
    use copperleaf_parts_connectors::{Arjm11d7502AbEw2, Conmhf4SmdGT, S2bPhSm4TbLfSn, UsbC23409011};
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
        B82472p6152m000, B82472p6222m000, Capacitor, Crystal, Resistor,
        footprint::Package,
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
    let u1 = board.add("U1", copperleaf_parts_texas_instruments::Tps63031dskr::new());
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
