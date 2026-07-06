use std::process::Command;

fn cl_bin() -> std::path::PathBuf {
    std::env::var("CARGO_BIN_EXE_cl")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let mut path = std::env::current_dir().unwrap();
            path.push("../../target/debug/cl");
            path
        })
}

#[test]
fn emit_output_round_trips_through_json() {
    let output = Command::new(cl_bin())
        .arg("emit")
        .output()
        .expect("failed to run cl emit");

    assert!(
        output.status.success(),
        "cl emit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let design: copperleaf::Design =
        serde_json::from_str(&stdout).expect("emit output should be valid Design JSON");

    // The example design has components, nets, and an empty connections array.
    assert!(!design.components.is_empty());
    assert!(!design.nets.is_empty());
}

#[test]
fn verify_runs_on_emitted_design_with_patched_connections() {
    let emit = Command::new(cl_bin())
        .arg("emit")
        .output()
        .expect("failed to run cl emit");
    assert!(emit.status.success());

    let mut design: serde_json::Value =
        serde_json::from_slice(&emit.stdout).expect("emit output should be valid JSON");

    // The emitted example design already contains connections. Patch U2.VDD from
    // V3V3 (3.3 V) to VBUS (5 V) so the ERC overvoltage check produces a result.
    if let Some(connections) = design.get_mut("connections")
        && let Some(arr) = connections.as_array_mut()
    {
        for conn in arr.iter_mut() {
            if conn["refdes"] == "U2" && conn["pin"] == "VDD" {
                conn["net"] = serde_json::json!("VBUS");
            }
        }
    }

    let temp_dir = std::env::temp_dir();
    let design_path = temp_dir.join(format!(
        "copperleaf_cli_verify_test_{}.json",
        std::process::id()
    ));
    std::fs::write(&design_path, serde_json::to_string_pretty(&design).unwrap()).unwrap();

    let verify = Command::new(cl_bin())
        .arg("verify")
        .arg(&design_path)
        .output()
        .expect("failed to run cl verify");

    std::fs::remove_file(&design_path).ok();

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
