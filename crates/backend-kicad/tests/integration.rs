use std::fs;

use copperleaf_backend_kicad::KiCad;
use copperleaf_model::{
    Backend, Board, Component, ComponentHandle, Constraint, Pin, PinRef, UnitExt,
};

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
        }]
    }
}

fn build_two_component_board(
    src_v: f64,
    load_v_max: f64,
) -> (Board, ComponentHandle, ComponentHandle) {
    let mut board = Board::new();
    let src = board.add("SRC", PwrSource::new(src_v));
    let load = board.add("U1", Load::new(load_v_max));
    board
        .connect(src.pin(PwrSource::VCC), load.pin(Load::VDD))
        .unwrap();
    (board, src, load)
}

#[test]
fn decoupling_caps_appear_in_summary() {
    let mut board = Board::new();
    let src = board.add("SRC", PwrSource::new(3.3));
    let part = board.add("U1", DecoupledPart::new());
    board
        .connect(src.pin(PwrSource::VCC), part.pin(DecoupledPart::VDD))
        .unwrap();

    let report = board.compile().expect("board should compile");
    assert_eq!(report.summary.caps_synthesised.len(), 1);
    assert_eq!(report.summary.caps_synthesised[0].refdes, "C1");
    assert!((report.summary.caps_synthesised[0].value.as_base() - 100e-9).abs() < 1e-18);
    assert_eq!(report.summary.caps_synthesised[0].source_component, "U1");
    assert_eq!(report.summary.caps_synthesised[0].source_pin, "VDD");
}

#[test]
fn emitted_netlist_contains_components_and_nets() {
    let (board, _, _) = build_two_component_board(3.3, 3.6);
    let report = board.compile().unwrap();

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
    let report = board.compile().unwrap();

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
    let err = board
        .compile()
        .expect_err("overvoltage should fail compilation");
    assert!(err.errors.iter().any(|d| d.code == "ERC:OVERVOLT"));
}

#[test]
fn valid_board_compiles_and_emits() {
    let (board, _, _) = build_two_component_board(3.3, 3.6);
    let report = board.compile().expect("valid board should compile");

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
}
