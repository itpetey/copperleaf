//! Directory discovery for KiCad symbol and footprint files.
//!
//! Given a directory path, finds `.kicad_sym` and `.kicad_mod` files (or a
//! `.pretty` footprint library directory).  Also provides helpers for
//! auto-detecting a component's `lib_id` from the discovered files so that
//! `--lib-id` is only needed for disambiguation.

use std::path::{Path, PathBuf};

use copperleaf::{Diagnostic, Severity};
use copperleaf_backend_kicad::{Sexpr, parse, parse_symbol_lib};

/// The result of scanning a directory for KiCad source files.
#[derive(Debug)]
pub struct Discovered {
    /// Path to a `.kicad_sym` symbol library file, if found.
    pub symbol: Option<PathBuf>,
    /// Path to a `.kicad_mod` footprint file or a `.pretty` directory, if found.
    pub footprint: Option<PathBuf>,
}

/// Scan `dir` for KiCad source files.
///
/// Looks for:
/// - The first `.kicad_sym` file (by directory-order).
/// - The first `.kicad_mod` file, or the first `.pretty` sub-directory
///   (footprint library).
pub fn discover(dir: &Path) -> Result<Discovered, std::io::Error> {
    let mut symbol: Option<PathBuf> = None;
    let mut footprint_mod: Option<PathBuf> = None;
    let mut footprint_pretty: Option<PathBuf> = None;

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let ft = entry.file_type()?;

        if ft.is_file() {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if ext.eq_ignore_ascii_case("kicad_sym") && symbol.is_none() {
                    symbol = Some(path);
                } else if ext.eq_ignore_ascii_case("kicad_mod") && footprint_mod.is_none() {
                    footprint_mod = Some(path);
                }
            }
        } else if ft.is_dir()
            && let Some(name) = path.file_name().and_then(|s| s.to_str())
            && name.ends_with(".pretty")
            && footprint_pretty.is_none()
        {
            footprint_pretty = Some(path);
        }
    }

    let footprint = footprint_mod.or(footprint_pretty);

    Ok(Discovered { symbol, footprint })
}

/// Auto-detect a `lib_id` from the discovered files.
///
/// Priority:
/// 1. If `--lib-id` was passed explicitly, use that.
/// 2. If a symbol file was found, resolve the lib-id from it (auto-detect for
///    single-symbol files, error for multi-symbol files without `--lib-id`).
/// 3. If only a footprint file was found, extract the footprint name from the
///    `.kicad_mod` file or from a `.pretty` directory entry.
///
/// Returns an error if no lib-id can be determined.
pub fn resolve_lib_id(
    explicit: Option<&str>,
    discovered: &Discovered,
    symbol_path: Option<&Path>,
) -> Result<String, crate::CliError> {
    // 1. Explicit --lib-id.
    if let Some(id) = explicit {
        return Ok(id.to_owned());
    }

    // 2. From symbol file.
    if let Some(ref sym_path) = discovered.symbol {
        let path = symbol_path.unwrap_or(sym_path);
        let source = std::fs::read_to_string(path)?;
        let symbols = parse_symbol_lib(&source)?;

        if symbols.len() == 1 {
            return Ok(symbols[0].lib_id.clone());
        }
        if symbols.is_empty() {
            return Err(crate::CliError::Diagnostic(Diagnostic {
                code: "CLI:NO_SYMBOLS".into(),
                severity: Severity::Error,
                message: format!("No symbols found in '{}'", path.display()),
                entities: vec![],
                hint: None,
            }));
        }
        // Multiple symbols — need --lib-id.
        let names: Vec<String> = symbols.iter().map(|s| s.lib_id.clone()).collect();
        return Err(crate::CliError::Diagnostic(Diagnostic {
            code: "CLI:MISSING_LIB_ID".into(),
            severity: Severity::Error,
            message: format!(
                "Multiple symbols found in '{}', --lib-id is required",
                path.display()
            ),
            entities: names,
            hint: Some(format!(
                "Available symbols: {}",
                symbols
                    .iter()
                    .map(|s| s.lib_id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }));
    }

    // 3. From footprint file.
    if let Some(ref fp_path) = discovered.footprint {
        return extract_footprint_name(fp_path);
    }

    Err(crate::CliError::Diagnostic(Diagnostic {
        code: "CLI:NO_SOURCE".into(),
        severity: Severity::Error,
        message: "Cannot determine lib_id — no .kicad_sym or .kicad_mod files found".into(),
        entities: vec![],
        hint: Some("Provide --lib-id explicitly".into()),
    }))
}

/// Extract the footprint name from a `.kicad_mod` file.
///
/// The footprint name is the first atom inside the top-level `(footprint …)`
/// list, e.g. `(footprint "QFN-32" …)` → `"QFN-32"`.
fn extract_footprint_name(path: &Path) -> Result<String, crate::CliError> {
    let source = std::fs::read_to_string(path)?;
    let expr = parse(&source)?;
    match &expr {
        Sexpr::List(nodes) if nodes.len() >= 2 => {
            if let Sexpr::Atom(tag) = &nodes[0]
                && tag == "footprint"
            {
                return Ok(nodes[1].as_string());
            }
        }
        _ => {}
    }
    Err(crate::CliError::Diagnostic(Diagnostic {
        code: "CLI:PARSE".into(),
        severity: Severity::Error,
        message: format!("Could not extract footprint name from '{}'", path.display()),
        entities: vec![],
        hint: Some("Ensure the file is a valid KiCad .kicad_mod footprint".into()),
    }))
}
