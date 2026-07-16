use std::path::PathBuf;

use copperleaf::{Diagnostic, Severity};
use copperleaf_backend_kicad::{
    find_symbol, flatten_extends, parse_single_symbol, parse_symbol_lib,
};

use crate::{CliError, UpdateArgs, kindmap::KindMap, manifest};

pub fn run(args: UpdateArgs) -> Result<(), CliError> {
    let kindmap = KindMap::load(args.kind_map.as_deref())?;

    let source = std::fs::read_to_string(&args.part_toml)?;
    let mut manifest = manifest::deserialise(&source)?;

    if let Some(ref path) = args.datasheet {
        manifest = crate::llm::update_from_datasheet(path, &args, &manifest)?;
    }

    let mut diags = Vec::new();

    if let Some(ref symbol_path) = args.symbol {
        let sym_source = std::fs::read_to_string(symbol_path)?;
        let symbols = parse_symbol_lib(&sym_source)?;

        // Resolve lib-id: CLI arg -> existing TOML -> auto-detect from single-symbol file.
        let owned_lib_id;
        let lib_id = match args
            .lib_id
            .as_deref()
            .or(manifest.component.lib_id.as_deref())
        {
            Some(id) => id,
            None => {
                if symbols.len() == 1 {
                    owned_lib_id = symbols[0].lib_id.clone();
                    &owned_lib_id
                } else if symbols.is_empty() {
                    return Err(CliError::Diagnostic(Diagnostic {
                        code: "CLI:NO_SYMBOLS".into(),
                        severity: Severity::Error,
                        message: format!("No symbols found in '{}'", symbol_path),
                        entities: vec![],
                        hint: None,
                    }));
                } else {
                    return Err(CliError::Diagnostic(Diagnostic {
                        code: "CLI:MISSING_LIB_ID".into(),
                        severity: Severity::Error,
                        message: format!(
                            "Multiple symbols found in '{}', --lib-id is required",
                            symbol_path
                        ),
                        entities: symbols.iter().map(|s| s.lib_id.clone()).collect(),
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
            }
        };
        // Idiot check: verify the source matches this part.
        if let Some(ref existing) = manifest.component.lib_id
            && existing != lib_id
        {
            return Err(CliError::Diagnostic(Diagnostic {
                code: "CLI:LIB_ID_MISMATCH".into(),
                severity: Severity::Error,
                message: format!(
                    "Part TOML has lib_id '{}', but source contains '{}'",
                    existing, lib_id
                ),
                entities: vec![existing.clone(), lib_id.to_string()],
                hint: Some("Use --lib-id to override, or update the correct TOML file".into()),
            }));
        }
        let Some(symbol) = find_symbol(&symbols, lib_id) else {
            return Err(CliError::Diagnostic(Diagnostic {
                code: "CLI:SYMBOL_NOT_FOUND".into(),
                severity: Severity::Error,
                message: format!("Symbol '{}' not found in '{}'", lib_id, symbol_path),
                entities: vec![lib_id.to_string()],
                hint: None,
            }));
        };
        // Flatten inheritance so pins from parent symbols (possibly in a
        // different file) are included.  Re-parse the flattened S-expression
        // to extract the complete pin set.
        let flattened = flatten_extends(symbol, &symbols);
        let flat_pins = parse_single_symbol(&flattened)
            .map(|s| s.pins)
            .unwrap_or_else(|| symbol.pins.clone());
        // Idiot check: warn if pin counts don't match.
        if !manifest.pins.is_empty() && flat_pins.len() != manifest.pins.len() {
            diags.push(Diagnostic {
                code: "CLI:PIN_COUNT_MISMATCH".into(),
                severity: Severity::Warning,
                message: format!(
                    "Symbol has {} pins, but part TOML has {}",
                    flat_pins.len(),
                    manifest.pins.len()
                ),
                entities: vec![],
                hint: Some("This may indicate the wrong symbol for this part".into()),
            });
        }
        diags.extend(manifest::merge_symbol(
            &mut manifest,
            &flat_pins,
            &kindmap,
            &args.default_kind,
        ));
        // Inherit datasheet from symbol if the manifest doesn't have one.
        if manifest.component.datasheet.is_none() {
            manifest.component.datasheet = symbol.datasheet.clone();
        }
    }

    if let Some(ref footprint_path) = args.footprint {
        let resolved_lib_id = args
            .lib_id
            .as_deref()
            .or(manifest.component.lib_id.as_deref())
            .unwrap_or_default();
        // Idiot check: verify the source matches this part.
        if let Some(ref existing) = manifest.component.lib_id
            && existing != resolved_lib_id
        {
            return Err(CliError::Diagnostic(Diagnostic {
                code: "CLI:LIB_ID_MISMATCH".into(),
                severity: Severity::Error,
                message: format!(
                    "Part TOML has lib_id '{}', but source contains '{}'",
                    existing, resolved_lib_id
                ),
                entities: vec![existing.clone(), resolved_lib_id.to_string()],
                hint: Some("Use --lib-id to override, or update the correct TOML file".into()),
            }));
        }
        let pads = if std::fs::metadata(footprint_path)?.is_dir() {
            let lib = copperleaf_backend_kicad::parse_footprint_lib(footprint_path)?;
            let Some((_, pads)) = lib.into_iter().find(|(name, _)| name == resolved_lib_id) else {
                return Err(CliError::Diagnostic(Diagnostic {
                    code: "CLI:FOOTPRINT_NOT_FOUND".into(),
                    severity: Severity::Error,
                    message: format!(
                        "Footprint '{}' not found in '{}'",
                        resolved_lib_id, footprint_path
                    ),
                    entities: vec![resolved_lib_id.to_string()],
                    hint: None,
                }));
            };
            pads
        } else {
            copperleaf_backend_kicad::parse_footprint(footprint_path)?
        };
        // Idiot check: warn if pad count doesn't match pin count.
        if !manifest.pins.is_empty() && pads.len() != manifest.pins.len() {
            diags.push(Diagnostic {
                code: "CLI:PAD_COUNT_MISMATCH".into(),
                severity: Severity::Warning,
                message: format!(
                    "Footprint has {} pads, but part TOML has {} pins",
                    pads.len(),
                    manifest.pins.len()
                ),
                entities: vec![],
                hint: Some("This may indicate the wrong footprint for this part".into()),
            });
        }
        diags.extend(manifest::merge_footprint(&mut manifest, &pads));
    }

    let output = manifest::serialise(&manifest);
    let out_path = args
        .out
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(&args.part_toml));

    for d in &diags {
        crate::print_diagnostic(d);
    }

    std::fs::write(&out_path, output)?;
    Ok(())
}
