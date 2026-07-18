use std::fs;

use copperleaf::{Backend, Board, Component, ComponentHandle, Constraint, Pin, PinRef, UnitExt};
use copperleaf_backend_kicad::KiCad;
use copperleaf_compile;

struct PwrSource {
    pins: Vec<Pin>,
}

struct Load {
    pins: Vec<Pin>,
}

struct DecoupledPart {
    pins: Vec<Pin>,
}

impl PwrSource {
    const VCC: PinRef = PinRef("VCC");
    fn new(v: f64) -> Self {
        Self {
            pins: vec![Pin::build("VCC").pwr_fixed(v.volt(), 1.0.amp()).pin()],
        }
    }
}

impl Component for PwrSource {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }
}

impl Load {
    const VDD: PinRef = PinRef("VDD");
    fn new(v_max: f64) -> Self {
        Self {
            pins: vec![
                Pin::build("VDD")
                    .pwr(0.0.volt(), v_max.volt(), 0.1.amp())
                    .pin(),
            ],
        }
    }
}

impl Component for Load {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }
}

impl DecoupledPart {
    const VDD: PinRef = PinRef("VDD");
    fn new() -> Self {
        Self {
            pins: vec![Pin::build("VDD").pwr_fixed(3.3.volt(), 0.1.amp()).pin()],
        }
    }
}

impl Component for DecoupledPart {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }
    fn constraints(&self) -> Vec<Constraint> {
        vec![Constraint::Decoupling {
            values: vec![100.0.nf()],
            per_pin: true,
            package: None,
        }]
    }
}

fn build_two_component_board(
    src_v: f64,
    load_v_max: f64,
) -> (Board, ComponentHandle, ComponentHandle) {
    let mut board = Board::new("test");
    let src = board.add("SRC", PwrSource::new(src_v));
    let load = board.add("U1", Load::new(load_v_max));
    board
        .connect(src.pin(PwrSource::VCC), load.pin(Load::VDD))
        .unwrap();
    (board, src, load)
}

#[test]
fn decoupling_caps_have_footprints() {
    let mut board = Board::new("test");
    let src = board.add("SRC", PwrSource::new(3.3));
    let part = board.add("U1", DecoupledPart::new());
    board
        .connect(src.pin(PwrSource::VCC), part.pin(DecoupledPart::VDD))
        .unwrap();

    let report = copperleaf_compile::run(board).expect("board should compile");
    // The synthesised capacitor should appear in the compiled board with a footprint.
    let caps: Vec<_> = report
        .board
        .components
        .iter()
        .filter(|c| {
            c.footprint
                .as_deref()
                .is_some_and(|fp| fp.contains("Capacitor_SMD"))
        })
        .collect();
    assert_eq!(caps.len(), 1);
    assert_eq!(caps[0].refdes, "C1");
    assert_eq!(report.summary.component_count, 3); // SRC + U1 + C1
}

#[test]
fn emitted_netlist_contains_components_and_nets() {
    let (board, _, _) = build_two_component_board(3.3, 3.6);
    let report = copperleaf_compile::run(board).unwrap();

    let dir = tempfile::tempdir().unwrap();
    KiCad::new()
        .with_project_name("test")
        .emit(dir.path().to_str().unwrap(), &report.board)
        .unwrap();

    let net = fs::read_to_string(dir.path().join("test.net")).unwrap();
    assert!(net.contains("(ref \"SRC\")"));
    assert!(net.contains("(ref \"U1\")"));
    assert!(net.contains("(name \"NET_SRC_VCC\")"));
    assert!(net.contains("(pinfunction \"VCC\")"));
    assert!(net.contains("(pinfunction \"VDD\")"));
}

#[test]
fn emitted_schematic_contains_lib_id_and_pin_positions() {
    let (board, _, _) = build_two_component_board(3.3, 3.6);
    let report = copperleaf_compile::run(board).unwrap();

    let dir = tempfile::tempdir().unwrap();
    KiCad::new()
        .with_project_name("test")
        .emit(dir.path().to_str().unwrap(), &report.board)
        .unwrap();

    let sch = fs::read_to_string(dir.path().join("test.kicad_sch")).unwrap();
    assert!(sch.contains("(lib_id \"copperleaf:SRC\")"));
    assert!(sch.contains("(lib_id \"copperleaf:U1\")"));
    assert!(sch.contains("(pin power_in line"));
}

#[test]
fn overvoltage_produces_compile_error() {
    let (board, _, _) = build_two_component_board(5.0, 3.3);
    let err = copperleaf_compile::run(board).expect_err("overvoltage should fail compilation");
    assert!(err.errors.iter().any(|d| d.code == "ERC:OVERVOLT"));
}

#[test]
fn valid_board_compiles_and_emits() {
    let (board, _, _) = build_two_component_board(3.3, 3.6);
    let report = copperleaf_compile::run(board).expect("valid board should compile");

    let dir = tempfile::tempdir().unwrap();
    let backend = KiCad::new().with_project_name("test");
    backend
        .emit(dir.path().to_str().unwrap(), &report.board)
        .unwrap();

    let names: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect();
    assert!(names.contains(&"test.kicad_sch".to_string()));
    assert!(names.contains(&"test.net".to_string()));
    assert!(names.contains(&"symbols".to_string()));
    assert!(names.contains(&"footprints".to_string()));

    // Verify symbols/ directory contains at least one .kicad_sym file.
    let sym_dir = dir.path().join("symbols");
    let sym_files: Vec<_> = fs::read_dir(&sym_dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect();
    assert!(!sym_files.is_empty(), "symbols/ should not be empty");
    assert!(
        sym_files.iter().any(|f| f.ends_with(".kicad_sym")),
        "should contain .kicad_sym files, got: {:?}",
        sym_files
    );

    // Verify footprints/ directory contains at least one .kicad_mod file.
    let fp_dir = dir.path().join("footprints");
    let fp_files: Vec<_> = fs::read_dir(&fp_dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect();
    assert!(!fp_files.is_empty(), "footprints/ should not be empty");
    assert!(
        fp_files.iter().any(|f| f.ends_with(".kicad_mod")),
        "should contain .kicad_mod files, got: {:?}",
        fp_files
    );
}
