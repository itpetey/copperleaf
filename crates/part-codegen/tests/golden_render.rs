//! Golden-file tests: every parts-crate TOML is rendered to Rust source via
//! [`generate_component_to_string`] and compared against a checked-in
//! snapshot, so template/expression changes are verified as no-ops.
//!
//! Regenerate the snapshots after an intentional change with:
//!
//! ```sh
//! COPPERLEAF_BLESS=1 cargo test -p copperleaf-part-codegen --test golden_render
//! ```

use std::path::{Path, PathBuf};

use copperleaf_part_codegen::generate_component_to_string;

fn parts_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parts")
}

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/render")
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

/// Generated sources embed base64-encoded 3D models inline, which would make
/// golden files tens of megabytes.  Replace any quoted string literal longer
/// than 1 KiB with a compact `<elided:LEN:FNV1A>` marker — the snapshot still
/// characterises the payload's length and content, but stays small.
fn elide_long_literals(rendered: &str) -> String {
    const THRESHOLD: usize = 1024;
    let mut out = String::with_capacity(rendered.len());
    let mut chars = rendered.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '"' {
            out.push(c);
            continue;
        }
        // Collect the literal body (the generator only emits simple escapes).
        let mut body = String::new();
        let mut escaped = false;
        for c in chars.by_ref() {
            if escaped {
                body.push(c);
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                break;
            } else {
                body.push(c);
            }
        }
        if body.len() > THRESHOLD {
            out.push_str(&format!("\"<elided:{}:{:016x}>\"", body.len(), fnv1a(&body)));
        } else {
            out.push('"');
            out.push_str(&body);
            out.push('"');
        }
    }
    out
}

fn fnv1a(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[test]
fn rendered_components_match_goldens() {
    let tomls = parts_tomls();
    assert!(!tomls.is_empty(), "no parts TOMLs found");
    for (vendor, stem, path) in &tomls {
        let rendered = generate_component_to_string(path)
            .unwrap_or_else(|e| panic!("failed to render {}: {e}", path.display()));
        compare_or_bless(
            &golden_dir().join(vendor).join(format!("{stem}.rs")),
            &elide_long_literals(&rendered),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elide_leaves_short_literals_alone() {
        assert_eq!(elide_long_literals("fn x() { \"abc\" }"), "fn x() { \"abc\" }");
    }

    #[test]
    fn elide_replaces_long_literals_deterministically() {
        let long = format!("\"{}\"", "A".repeat(2048));
        let a = elide_long_literals(&long);
        assert!(a.contains("<elided:2048:"), "{a}");
        assert_eq!(a, elide_long_literals(&long));
    }
}
