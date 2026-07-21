//! Golden-file tests for `cl generate footprint|symbol`: every parts-crate
//! TOML is rendered to a `.kicad_mod` / `.kicad_sym` file and compared
//! against a checked-in snapshot.
//!
//! Regenerate the snapshots after an intentional behaviour change with:
//!
//! ```sh
//! COPPERLEAF_BLESS=1 cargo test -p copperleaf-cli --test golden_generate
//! ```

use std::{
    path::{Path, PathBuf},
    process::Command,
};

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
        expected,
        actual,
        "golden mismatch: {} — run with COPPERLEAF_BLESS=1 to update",
        path.display()
    );
}

fn copperleaf() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cl"))
}

#[test]
fn generate_footprint_matches_goldens() {
    let tomls = parts_tomls();
    assert!(!tomls.is_empty(), "no parts TOMLs found");
    let dir = tempfile::tempdir().unwrap();
    for (vendor, stem, path) in &tomls {
        let out = dir.path().join(format!("{vendor}-{stem}.kicad_mod"));
        run_generate("footprint", path, &out);
        let actual = std::fs::read_to_string(&out).unwrap();
        compare_or_bless(
            &golden_dir().join(vendor).join(format!("{stem}.kicad_mod")),
            &actual,
        );
    }
}

#[test]
fn generate_symbol_matches_goldens() {
    let tomls = parts_tomls();
    assert!(!tomls.is_empty(), "no parts TOMLs found");
    let dir = tempfile::tempdir().unwrap();
    for (vendor, stem, path) in &tomls {
        let out = dir.path().join(format!("{vendor}-{stem}.kicad_sym"));
        run_generate("symbol", path, &out);
        let actual = std::fs::read_to_string(&out).unwrap();
        compare_or_bless(
            &golden_dir().join(vendor).join(format!("{stem}.kicad_sym")),
            &actual,
        );
    }
}

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/generate")
}

fn parts_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parts")
}

/// All parts-crate TOML files, as `(crate_name, file_stem, path)` triples in
/// sorted order.
fn parts_tomls() -> Vec<(String, String, PathBuf)> {
    let mut out = Vec::new();
    let vendors = std::fs::read_dir(parts_dir()).expect("parts directory");
    for vendor in vendors.flatten() {
        let vendor_name = vendor.file_name().to_string_lossy().into_owned();
        let Ok(entries) = std::fs::read_dir(vendor.path()) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }
            // Only component definition TOMLs — skip crate manifests.
            if path.file_name().and_then(|s| s.to_str()) == Some("Cargo.toml") {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            out.push((vendor_name.clone(), stem, path));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    out
}

fn run_generate(kind: &str, toml: &Path, out: &Path) {
    let status = copperleaf()
        .arg("generate")
        .arg(kind)
        .arg(toml)
        .arg("--out")
        .arg(out)
        .status()
        .expect("spawn cl");
    assert!(
        status.success(),
        "cl generate {kind} failed for {}",
        toml.display()
    );
}
