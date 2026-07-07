use std::{io::Write, process::Command};

use copperleaf::{Block, ComponentInst, Design, Limits, Net, Pin, Role, UnitExt};

/// A minimal MCU part used by tests to build designs without the built-in example.
struct SimpleMcu {
    pins: Vec<Pin>,
}

struct SymbolMcu {
    pins: Vec<Pin>,
}

impl SimpleMcu {
    fn new() -> Self {
        Self {
            pins: vec![
                Pin::new(
                    "VDD",
                    Role::PowerIn,
                    Limits::new(1.7.volt(), 3.6.volt(), 0.5.amp()),
                    None,
                ),
                Pin::new(
                    "VSS",
                    Role::Gnd,
                    Limits::new(0.0.volt(), 0.0.volt(), 0.0.amp()),
                    None,
                ),
            ],
        }
    }
}

impl Block for SimpleMcu {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }
}

impl SymbolMcu {
    fn new() -> Self {
        Self {
            pins: vec![
                Pin::new(
                    "VDD",
                    Role::PowerIn,
                    Limits::new(1.7.volt(), 3.6.volt(), 0.5.amp()),
                    None,
                ),
                Pin::new(
                    "VSS",
                    Role::Gnd,
                    Limits::new(0.0.volt(), 0.0.volt(), 0.0.amp()),
                    None,
                ),
            ],
        }
    }
}

impl Block for SymbolMcu {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }
    fn kicad_symbol(&self) -> Option<&str> {
        Some("RP2040:RP2354a")
    }
}

fn cl_bin() -> std::path::PathBuf {
    std::env::var("CARGO_BIN_EXE_cl")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let mut path = std::env::current_dir().unwrap();
            path.push("../../target/debug/cl");
            path
        })
}

fn example_design() -> Design {
    let mut d = Design::default();

    d.add_net(Net::power("VBUS", 5.0.volt()));
    d.add_net(Net::ground());
    d.add_net(Net::power("V3V3", 3.3.volt()));

    d.add_component(ComponentInst::new("U2", SimpleMcu::new()));

    d.connect("U2", "VDD", "V3V3");
    d.connect("U2", "VSS", "GND");

    d
}

#[test]
fn example_design_serializes_and_deserializes() {
    let design = example_design();
    let json = serde_json::to_string_pretty(&design).expect("should serialize design");
    let parsed: copperleaf::Design =
        serde_json::from_str(&json).expect("serialized design should deserialize");

    assert!(!parsed.components.is_empty());
    assert!(!parsed.nets.is_empty());
}

#[test]
fn export_with_design_file_writes_named_outputs() {
    let temp = test_dir("export_named");
    let design_path = temp.join("my_design.json");
    std::fs::write(
        &design_path,
        serde_json::to_string_pretty(&example_design()).unwrap(),
    )
    .unwrap();

    let output = Command::new(cl_bin())
        .arg("export")
        .arg(&design_path)
        .arg("-o")
        .arg(&temp)
        .output()
        .expect("failed to run cl export with file");

    std::fs::remove_file(&design_path).ok();

    assert!(
        output.status.success(),
        "cl export <file> failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let net =
        std::fs::read_to_string(temp.join("my_design.net")).expect("my_design.net should exist");
    assert!(net.starts_with("(export"));

    let sch = std::fs::read_to_string(temp.join("my_design.kicad_sch"))
        .expect("my_design.kicad_sch should exist");
    assert!(sch.starts_with("(kicad_sch"));

    let pcb = std::fs::read_to_string(temp.join("my_design.kicad_pcb"))
        .expect("my_design.kicad_pcb should exist");
    assert!(pcb.starts_with("(kicad_pcb"));

    std::fs::remove_dir_all(&temp).ok();
}

#[test]
fn export_with_missing_file_exits_nonzero() {
    let output = Command::new(cl_bin())
        .arg("export")
        .arg("definitely_does_not_exist.json")
        .output()
        .expect("failed to run cl export");

    assert!(
        !output.status.success(),
        "cl export with missing file should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Error reading design file"),
        "expected clear error message, got: {}",
        stderr
    );
}

#[test]
fn export_with_symbol_lib_resolves_pin_positions() {
    let mut design = Design::default();
    design.add_net(Net::power("V3V3", 3.3.volt()));
    design.add_net(Net::ground());
    design.add_component(ComponentInst::new("U1", SymbolMcu::new()));
    design.connect("U1", "VDD", "V3V3");
    design.connect("U1", "VSS", "GND");

    let temp = test_dir("export_symbol_lib");
    let design_path = temp.join("design.json");
    std::fs::write(&design_path, serde_json::to_string_pretty(&design).unwrap()).unwrap();

    let lib_path = temp.join("test.kicad_sym");
    let mut lib_file = std::fs::File::create(&lib_path).unwrap();
    lib_file
        .write_all(
            r#"(kicad_symbol_lib
  (symbol "RP2354a"
    (pin power_in line (at -15.24 5.08 0) (length 2.54) (name "VDD") (number "1"))
    (pin power_in line (at -15.24 -5.08 0) (length 2.54) (name "VSS") (number "2"))
  )
)"#
            .as_bytes(),
        )
        .unwrap();

    let output = Command::new(cl_bin())
        .arg("export")
        .arg(&design_path)
        .arg("-o")
        .arg(&temp)
        .arg("--symbol-lib")
        .arg(&lib_path)
        .output()
        .expect("failed to run cl export --symbol-lib");

    std::fs::remove_file(&design_path).ok();
    std::fs::remove_file(&lib_path).ok();

    assert!(
        output.status.success(),
        "cl export --symbol-lib failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let sch = std::fs::read_to_string(temp.join("design.kicad_sch"))
        .expect("design.kicad_sch should exist");
    assert!(sch.contains("(lib_id \"RP2040:RP2354a\")"));
    assert!(sch.contains("(at -15.24 5.08 0)"));

    std::fs::remove_dir_all(&temp).ok();
}

#[test]
fn export_writes_three_kicad_files() {
    let temp = test_dir("export_basic");
    let design_path = temp.join("design.json");
    std::fs::write(
        &design_path,
        serde_json::to_string_pretty(&example_design()).unwrap(),
    )
    .unwrap();

    let output = Command::new(cl_bin())
        .arg("export")
        .arg(&design_path)
        .arg("-o")
        .arg(&temp)
        .output()
        .expect("failed to run cl export");

    std::fs::remove_file(&design_path).ok();

    assert!(
        output.status.success(),
        "cl export failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let net = std::fs::read_to_string(temp.join("design.net")).expect("design.net should exist");
    assert!(net.starts_with("(export"));
    assert!(net.contains("(components"));
    assert!(net.contains("(nets"));

    let sch = std::fs::read_to_string(temp.join("design.kicad_sch"))
        .expect("design.kicad_sch should exist");
    assert!(sch.starts_with("(kicad_sch"));

    let pcb = std::fs::read_to_string(temp.join("design.kicad_pcb"))
        .expect("design.kicad_pcb should exist");
    assert!(pcb.starts_with("(kicad_pcb"));
    assert!(pcb.contains("(net_class \"Default\""));

    std::fs::remove_dir_all(&temp).ok();
}

fn test_dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!(
        "copperleaf_cli_test_{}_{}_{}",
        name,
        std::process::id(),
        std::thread::current().name().unwrap_or("?"),
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn verify_reports_overvoltage_after_patching_vdd_to_vbus() {
    let mut design = example_design();
    // Patch U2.VDD from V3V3 (3.3 V) to VBUS (5 V) so the ERC overvoltage check
    // produces a result.
    for conn in &mut design.connections {
        if conn.refdes == "U2" && conn.pin == "VDD" {
            conn.net = "VBUS".to_string();
        }
    }

    let temp = test_dir("verify_patch");
    let design_path = temp.join("design.json");
    std::fs::write(&design_path, serde_json::to_string_pretty(&design).unwrap()).unwrap();

    let verify = Command::new(cl_bin())
        .arg("verify")
        .arg(&design_path)
        .output()
        .expect("failed to run cl verify");

    std::fs::remove_dir_all(&temp).ok();

    assert!(
        verify.status.success(),
        "cl verify failed: {}",
        String::from_utf8_lossy(&verify.stderr)
    );

    let stdout = String::from_utf8_lossy(&verify.stdout);
    assert!(
        stdout.contains("ERC:OVERVOLT"),
        "verify output should report overvoltage after patching U2.VDD to VBUS: {}",
        stdout
    );
}
