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

fn copperleaf() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cl"))
}

#[test]
fn generate_footprint_matches_goldens() {
    let tomls = parts_tomls();
    assert!(!tomls.is_empty(), "no parts TOMLs found");
    let dir = tempfile::tempdir().unwrap();
    let mut goldens = Vec::new();
    for (vendor, stem, path) in &tomls {
        let out = dir.path().join(format!("{vendor}-{stem}.kicad_mod"));
        run_generate("footprint", path, &out);
        let actual = std::fs::read_to_string(&out).unwrap();
        goldens.push((
            golden_dir().join(vendor).join(format!("{stem}.kicad_mod")),
            actual,
        ));
    }
    report_failures(collect_failures(&goldens, blessing_enabled()));
}

#[test]
fn generate_symbol_matches_goldens() {
    let tomls = parts_tomls();
    assert!(!tomls.is_empty(), "no parts TOMLs found");
    let dir = tempfile::tempdir().unwrap();
    let mut goldens = Vec::new();
    for (vendor, stem, path) in &tomls {
        let out = dir.path().join(format!("{vendor}-{stem}.kicad_sym"));
        run_generate("symbol", path, &out);
        let actual = std::fs::read_to_string(&out).unwrap();
        goldens.push((
            golden_dir().join(vendor).join(format!("{stem}.kicad_sym")),
            actual,
        ));
    }
    report_failures(collect_failures(&goldens, blessing_enabled()));
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
        let goldens: Vec<_> = ["a", "b", "c"]
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
