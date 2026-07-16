use std::process::Command;

fn copperleaf() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cl"));
    cmd.current_dir(std::env::current_dir().unwrap());
    cmd
}

/// Minimal embedded RP2354A symbol derived from `MCU_RaspberryPi.kicad_sym`.
/// Synthesised so the test does not depend on a temporary external file.
fn minimal_rp2354a_symbol_lib() -> &'static str {
    r#"(kicad_symbol_lib
  (symbol "RP2350A"
    (pin power_in line (at -2.54 45.72 270) (length 3.81) (name "IOVDD") (number "1"))
    (pin bidirectional line (at 25.4 38.1 180) (length 3.81) (name "GPIO0") (number "2"))
    (pin input line (at -25.4 -15.24 0) (length 3.81) (name "XIN") (number "21"))
    (pin output line (at -25.4 -25.4 0) (length 3.81) (name "XOUT") (number "22"))
    (pin power_in line (at 12.7 45.72 270) (length 3.81) (name "DVDD") (number "23"))
    (pin input line (at -25.4 -33.02 0) (length 3.81) (name "SWCLK") (number "24"))
    (pin power_in line (at -10.16 45.72 270) (length 3.81) (name "ADC_AVDD") (number "44"))
    (pin power_in line (at 2.54 45.72 270) (length 3.81) (name "VREG_VIN") (number "49"))
    (pin power_out line (at 7.62 45.72 270) (length 3.81) (name "VREG_VOUT") (number "50"))
    (pin bidirectional line (at -25.4 15.24 0) (length 3.81) (name "USB_DM") (number "51"))
    (pin bidirectional line (at -25.4 -7.62 0) (length 3.81) (name "QSPI_SD3") (number "55"))
    (pin output line (at -25.4 2.54 0) (length 3.81) (name "QSPI_SCLK") (number "56"))
    (pin output line (at -25.4 5.08 0) (length 3.81) (name "QSPI_SS") (number "60"))
    (pin power_in line (at 0 -45.72 90) (length 3.81) (name "GND") (number "61"))
  )
  (symbol "RP2354A"
    (extends "RP2350A")
  )
)"#
}

#[test]
fn new_datasheet_fails_with_stub() {
    let dir = tempfile::tempdir().unwrap();
    let ds = dir.path().join("test.pdf");
    std::fs::write(&ds, "PDF").unwrap();
    let out = dir.path().join("test.toml");

    let status = copperleaf()
        .arg("new")
        .arg("--datasheet")
        .arg(&ds)
        .arg("--out")
        .arg(&out)
        .status()
        .unwrap();
    assert!(!status.success());
    assert!(!out.exists());
}

#[test]
fn new_symbol_codegen_round_trip_emits_physical_calls() {
    let dir = tempfile::tempdir().unwrap();
    let sym = dir.path().join("test.kicad_sym");
    std::fs::write(&sym, sample_symbol_lib()).unwrap();
    let kind_map = dir.path().join("kind_map.toml");
    std::fs::write(
        &kind_map,
        r#"
[by_type]
power_in = { kind = "pwr", v_min = 1.8, v_max = 3.3, i_max = 0.1 }
"#,
    )
    .unwrap();
    let out = dir.path().join("test.toml");

    copperleaf()
        .arg("new")
        .arg("--symbol")
        .arg(&sym)
        .arg("--lib-id")
        .arg("TEST")
        .arg("--kind-map")
        .arg(&kind_map)
        .arg("--out")
        .arg(&out)
        .status()
        .unwrap();

    let rust = copperleaf_part_codegen::generate_component_to_string(&out).unwrap();
    assert!(rust.contains(".pos("));
    assert!(rust.contains(".rotation("));
    assert!(rust.contains(".length("));
}

#[test]
fn new_symbol_generates_toml_with_physical_fields() {
    let dir = tempfile::tempdir().unwrap();
    let sym = dir.path().join("test.kicad_sym");
    std::fs::write(&sym, sample_symbol_lib()).unwrap();
    let out = dir.path().join("test.toml");

    let status = copperleaf()
        .arg("new")
        .arg("--symbol")
        .arg(&sym)
        .arg("--lib-id")
        .arg("TEST")
        .arg("--out")
        .arg(&out)
        .status()
        .unwrap();
    assert!(status.success());

    let toml = std::fs::read_to_string(&out).unwrap();
    assert!(toml.contains("kind = \"pwr\""));
    assert!(toml.contains("kind = \"gnd\""));
    assert!(toml.contains("kind = \"clk\""));
    assert!(toml.contains("bw_mhz = 25.0"));
    assert!(toml.contains("pos = [-5.08, 2.54]"));
    assert!(toml.contains("rotation = 0.0"));
    assert!(toml.contains("length = 2.54"));
}

#[test]
fn new_symbol_matches_existing_rp2354a_part() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir.parent().unwrap().parent().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let sym = dir.path().join("rp2354a.kicad_sym");
    std::fs::write(&sym, minimal_rp2354a_symbol_lib()).unwrap();
    let out = dir.path().join("rp2354a_generated.toml");

    let output = copperleaf()
        .arg("new")
        .arg("--symbol")
        .arg(&sym)
        .arg("--lib-id")
        .arg("RP2354A")
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success());

    let generated = std::fs::read_to_string(&out).unwrap();
    let existing = std::fs::read_to_string(root.join("parts/raspberrypi/rp2354a.toml")).unwrap();

    let generated_manifest: copperleaf_part_codegen::Manifest = toml::from_str(&generated).unwrap();
    let existing_manifest: copperleaf_part_codegen::Manifest = toml::from_str(&existing).unwrap();

    assert_eq!(generated_manifest.component.name, "Rp2354a");

    for existing_pin in &existing_manifest.pins {
        let Some(generated_pin) = generated_manifest
            .pins
            .iter()
            .find(|p| p.num == existing_pin.num)
        else {
            continue;
        };
        assert_eq!(generated_pin.num, existing_pin.num);
        assert!(!generated_pin.kind.is_empty());
    }
}

#[test]
fn new_symbol_with_crate_scaffolds_vendor_crate() {
    let dir = tempfile::tempdir().unwrap();
    let sym = dir.path().join("test.kicad_sym");
    std::fs::write(&sym, sample_symbol_lib()).unwrap();

    // The CLI expects to run in a workspace root. Create a minimal one.
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\n  \"parts/existing\"\n]\n",
    )
    .unwrap();

    let status = copperleaf()
        .current_dir(dir.path())
        .arg("new")
        .arg("--symbol")
        .arg(&sym)
        .arg("--lib-id")
        .arg("TEST")
        .arg("--crate")
        .arg("testvendor")
        .status()
        .unwrap();
    assert!(status.success());

    let cargo_toml = dir.path().join("parts/testvendor/Cargo.toml");
    assert!(cargo_toml.exists());
    let lib_rs = dir.path().join("parts/testvendor/lib.rs");
    assert!(lib_rs.exists());
    let root_cargo = std::fs::read_to_string(dir.path().join("Cargo.toml")).unwrap();
    assert!(root_cargo.contains("\"parts/testvendor\""));
}

#[test]
fn new_then_update_footprint_preserves_logical_fields() {
    let dir = tempfile::tempdir().unwrap();
    let sym = dir.path().join("test.kicad_sym");
    std::fs::write(&sym, sample_symbol_lib()).unwrap();
    let fp = dir.path().join("test.kicad_mod");
    std::fs::write(&fp, sample_footprint()).unwrap();
    let out = dir.path().join("test.toml");

    copperleaf()
        .arg("new")
        .arg("--symbol")
        .arg(&sym)
        .arg("--lib-id")
        .arg("TEST")
        .arg("--out")
        .arg(&out)
        .status()
        .unwrap();

    copperleaf()
        .arg("update")
        .arg(&out)
        .arg("--footprint")
        .arg(&fp)
        .arg("--lib-id")
        .arg("TEST")
        .status()
        .unwrap();

    let toml = std::fs::read_to_string(&out).unwrap();
    assert!(toml.contains("kind = \"pwr\""));
    assert!(toml.contains("name = \"VDD\""));
    assert!(toml.contains("pos = [-2.0, 1.0]"));
    assert!(toml.contains("rotation = 90.0"));
}

fn sample_footprint() -> &'static str {
    r#"(footprint "TEST"
  (pad "1" smd rect (at -2.0 1.0 90.0) (size 0.5 0.25))
  (pad "2" smd rect (at -2.0 -1.0) (size 0.5 0.25))
  (pad "3" smd rect (at 2.0 1.0 180.0) (size 0.5 0.25))
  (pad "4" smd rect (at 2.0 -1.0) (size 0.5 0.25))
)"#
}

fn sample_symbol_lib() -> &'static str {
    r#"(kicad_symbol_lib
  (symbol "TEST"
    (pin power_in line (at -5.08 2.54 0) (length 2.54) (name "VDD") (number "1"))
    (pin gnd line (at -5.08 -2.54 0) (length 2.54) (name "GND") (number "2"))
    (pin clock line (at 5.08 2.54 180) (length 2.54) (name "CLK") (number "3"))
    (pin input line (at 5.08 0 180) (length 2.54) (name "D0") (number "4"))
  )
)"#
}

fn wrong_symbol_lib() -> &'static str {
    r#"(kicad_symbol_lib
  (symbol "WRONG"
    (pin power_in line (at -5.08 2.54 0) (length 2.54) (name "VDD") (number "1"))
    (pin gnd line (at -5.08 -2.54 0) (length 2.54) (name "GND") (number "2"))
    (pin input line (at 5.08 0 180) (length 2.54) (name "D0") (number "3"))
  )
)"#
}

fn two_pin_symbol_lib() -> &'static str {
    r#"(kicad_symbol_lib
  (symbol "TEST"
    (pin power_in line (at -5.08 2.54 0) (length 2.54) (name "VDD") (number "1"))
    (pin gnd line (at -5.08 -2.54 0) (length 2.54) (name "GND") (number "2"))
  )
)"#
}

fn two_pad_footprint() -> &'static str {
    r#"(footprint "TEST"
  (pad "1" smd rect (at -2.0 1.0 90.0) (size 0.5 0.25))
  (pad "2" smd rect (at -2.0 -1.0) (size 0.5 0.25))
)"#
}

#[test]
fn update_symbol_wrong_lib_id_fails() {
    let dir = tempfile::tempdir().unwrap();
    let sym = dir.path().join("test.kicad_sym");
    std::fs::write(&sym, sample_symbol_lib()).unwrap();
    let out = dir.path().join("test.toml");

    copperleaf()
        .arg("new")
        .arg("--symbol")
        .arg(&sym)
        .arg("--lib-id")
        .arg("TEST")
        .arg("--out")
        .arg(&out)
        .status()
        .unwrap();

    let wrong_sym = dir.path().join("wrong.kicad_sym");
    std::fs::write(&wrong_sym, wrong_symbol_lib()).unwrap();

    let output = copperleaf()
        .arg("update")
        .arg(&out)
        .arg("--symbol")
        .arg(&wrong_sym)
        .arg("--lib-id")
        .arg("WRONG")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("CLI:LIB_ID_MISMATCH"));
    assert!(stderr.contains("Part TOML has lib_id 'TEST', but source contains 'WRONG'"));
}

#[test]
fn update_symbol_pin_count_mismatch_warns() {
    let dir = tempfile::tempdir().unwrap();
    let sym = dir.path().join("test.kicad_sym");
    std::fs::write(&sym, sample_symbol_lib()).unwrap();
    let out = dir.path().join("test.toml");

    copperleaf()
        .arg("new")
        .arg("--symbol")
        .arg(&sym)
        .arg("--lib-id")
        .arg("TEST")
        .arg("--out")
        .arg(&out)
        .status()
        .unwrap();

    let two_pin_sym = dir.path().join("two.kicad_sym");
    std::fs::write(&two_pin_sym, two_pin_symbol_lib()).unwrap();

    let output = copperleaf()
        .arg("update")
        .arg(&out)
        .arg("--symbol")
        .arg(&two_pin_sym)
        .arg("--lib-id")
        .arg("TEST")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("CLI:PIN_COUNT_MISMATCH"));
    assert!(stderr.contains("Symbol has 2 pins, but part TOML has 4"));
}

#[test]
fn update_footprint_wrong_lib_id_fails() {
    let dir = tempfile::tempdir().unwrap();
    let sym = dir.path().join("test.kicad_sym");
    std::fs::write(&sym, sample_symbol_lib()).unwrap();
    let fp = dir.path().join("test.kicad_mod");
    std::fs::write(&fp, sample_footprint()).unwrap();
    let out = dir.path().join("test.toml");

    copperleaf()
        .arg("new")
        .arg("--symbol")
        .arg(&sym)
        .arg("--lib-id")
        .arg("TEST")
        .arg("--out")
        .arg(&out)
        .status()
        .unwrap();

    let output = copperleaf()
        .arg("update")
        .arg(&out)
        .arg("--footprint")
        .arg(&fp)
        .arg("--lib-id")
        .arg("WRONG")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("CLI:LIB_ID_MISMATCH"));
    assert!(stderr.contains("Part TOML has lib_id 'TEST', but source contains 'WRONG'"));
}

#[test]
fn update_footprint_pad_count_mismatch_warns() {
    let dir = tempfile::tempdir().unwrap();
    let sym = dir.path().join("test.kicad_sym");
    std::fs::write(&sym, sample_symbol_lib()).unwrap();
    let out = dir.path().join("test.toml");

    copperleaf()
        .arg("new")
        .arg("--symbol")
        .arg(&sym)
        .arg("--lib-id")
        .arg("TEST")
        .arg("--out")
        .arg(&out)
        .status()
        .unwrap();

    let two_pad_fp = dir.path().join("two.kicad_mod");
    std::fs::write(&two_pad_fp, two_pad_footprint()).unwrap();

    let output = copperleaf()
        .arg("update")
        .arg(&out)
        .arg("--footprint")
        .arg(&two_pad_fp)
        .arg("--lib-id")
        .arg("TEST")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("CLI:PAD_COUNT_MISMATCH"));
    assert!(stderr.contains("Footprint has 2 pads, but part TOML has 4 pins"));
}
