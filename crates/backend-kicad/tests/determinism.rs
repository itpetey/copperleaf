//! Cross-process output determinism check.
//!
//! std `HashMap` iteration order is randomised per process, so any pipeline
//! stage that iterates a `HashMap` can produce differently-ordered output on
//! each run.  This test compiles and emits the same board in two spawned
//! processes (each with a fresh random state) and byte-diffs every emitted
//! file.
//!
//! The board is deliberately broad: multiple power nets, a name and voltage
//! override, signal nets, decoupling-capacitor synthesis across two nets,
//! thermal vias, and a mechanical pad.

use std::{path::Path, process::Command};

use copperleaf::{
    Backend, Board, Component, Constraint, Pad, PadShape, PadType, Pin, PinRef, UnitExt,
};
use copperleaf_backend_kicad::KiCad;

/// When set, the test runs in child mode: emit the board into this directory.
const ENV_OUT_DIR: &str = "COPPERLEAF_DETERMINISM_OUT";
const FILES: [&str; 4] = ["det.kicad_pro", "det.kicad_sch", "det.kicad_pcb", "det.net"];
const PROJECT: &str = "det";
const TEST_NAME: &str = "emit_is_byte_identical_across_processes";

struct PwrSource {
    pins: Vec<Pin>,
}

struct Mcu {
    pins: Vec<Pin>,
    mechanical: Vec<Pad>,
}

struct Slave {
    pins: Vec<Pin>,
}

impl PwrSource {
    const V3V3: PinRef = PinRef("VCC3V3");
    const V1V8: PinRef = PinRef("VCC1V8");
    const GND: PinRef = PinRef("GND");

    fn new() -> Self {
        Self {
            pins: vec![
                Pin::build("VCC3V3").pwr_fixed(3.3.volt(), 1.0.amp()).pin(),
                Pin::build("VCC1V8").pwr_fixed(1.8.volt(), 0.5.amp()).pin(),
                Pin::build("GND").gnd(),
            ],
        }
    }
}

impl Component for PwrSource {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }
}

impl Mcu {
    const VDD: PinRef = PinRef("VDD");
    const VBAT: PinRef = PinRef("VBAT");
    const GND: PinRef = PinRef("GND");
    const EP: PinRef = PinRef("EP");
    const SCK: PinRef = PinRef("SCK");
    const MOSI: PinRef = PinRef("MOSI");

    fn new() -> Self {
        Self {
            pins: vec![
                Pin::build("VDD").pwr_fixed(3.3.volt(), 0.3.amp()).pin(),
                Pin::build("VBAT")
                    .pwr(1.5.volt(), 3.6.volt(), 0.3.amp())
                    .pin(),
                Pin::build("GND").gnd(),
                Pin::build("EP")
                    .thermal_via((0.35, 0.0), 0.2, 0.3)
                    .thermal_via((-0.35, 0.0), 0.2, 0.3)
                    .gnd(),
                Pin::build("SCK").clk(25.0),
                Pin::build("MOSI").spi(25.0),
            ],
            mechanical: vec![Pad {
                number: "None".into(),
                pos: (0.0, 0.0),
                rotation: 0.0,
                width: 1.2,
                height: 1.2,
                pad_type: PadType::NpThruHole,
                pad_shape: PadShape::Circle,
                roundrect_rratio: None,
                layers: None,
                drill: Some(1.2),
                solder_mask_margin: None,
            }],
        }
    }
}

impl Component for Mcu {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }

    fn mechanical(&self) -> &[Pad] {
        &self.mechanical
    }

    fn constraints(&self) -> Vec<Constraint> {
        vec![Constraint::Decoupling {
            values: vec![100.0.nf()],
            per_pin: false,
        }]
    }
}

impl Slave {
    const SCK: PinRef = PinRef("SCK");
    const MOSI: PinRef = PinRef("MOSI");

    fn new() -> Self {
        Self {
            pins: vec![Pin::build("SCK").dio(), Pin::build("MOSI").dio()],
        }
    }
}

impl Component for Slave {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }
}

fn build_board() -> Board {
    let mut board = Board::new(PROJECT);
    let src = board.add("SRC", PwrSource::new());
    let u1 = board.add("U1", Mcu::new());
    let u2 = board.add("U2", Slave::new());

    let v3v3 = board
        .connect(src.pin(PwrSource::V3V3), u1.pin(Mcu::VDD))
        .unwrap();
    board.set_net_name(v3v3, "V3V3");
    let v1v8 = board
        .connect(src.pin(PwrSource::V1V8), u1.pin(Mcu::VBAT))
        .unwrap();
    board.set_net_voltage(v1v8, 1.8.volt());
    board
        .connect(src.pin(PwrSource::GND), u1.pin(Mcu::GND))
        .unwrap();
    board
        .connect(src.pin(PwrSource::GND), u1.pin(Mcu::EP))
        .unwrap();
    board.connect(u1.pin(Mcu::SCK), u2.pin(Slave::SCK)).unwrap();
    board
        .connect(u1.pin(Mcu::MOSI), u2.pin(Slave::MOSI))
        .unwrap();
    board
}

fn emit_board(out_dir: &Path) {
    let report = copperleaf_compile::run(
        build_board(),
        &copperleaf_compile::CompileOptions::default(),
    )
    .expect("board should compile");
    KiCad::new()
        .with_project_name(PROJECT)
        .emit(out_dir, &report.board)
        .expect("emit should succeed");
}

#[test]
fn emit_is_byte_identical_across_processes() {
    // Child mode: emit into the directory provided by the parent and stop.
    if let Ok(dir) = std::env::var(ENV_OUT_DIR) {
        emit_board(Path::new(&dir));
        return;
    }

    let exe = std::env::current_exe().expect("test executable path");
    let run_child = || {
        let dir = tempfile::tempdir().expect("tempdir");
        let status = Command::new(&exe)
            .arg(TEST_NAME)
            .arg("--exact")
            .env(ENV_OUT_DIR, dir.path())
            .status()
            .expect("spawn child test process");
        assert!(status.success(), "child emit process failed");
        dir
    };

    let a = run_child();
    let b = run_child();

    for name in FILES {
        let fa = std::fs::read(a.path().join(name))
            .unwrap_or_else(|e| panic!("child A did not write {name}: {e}"));
        let fb = std::fs::read(b.path().join(name))
            .unwrap_or_else(|e| panic!("child B did not write {name}: {e}"));
        assert!(!fa.is_empty(), "{name} is empty");
        assert_eq!(
            fa, fb,
            "{name} differs between processes — a HashMap is being iterated somewhere"
        );
    }
}
