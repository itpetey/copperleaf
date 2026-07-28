use std::path::PathBuf;

use copperleaf::{Diagnostic, Severity};
use copperleaf_backend_kicad::{
    find_symbol, flatten_extends, parse_single_symbol, parse_symbol_lib,
};

use crate::{CliError, UpdateArgs, discover, kindmap::KindMap, manifest};

pub fn run(args: UpdateArgs) -> Result<(), CliError> {
    // At least one source must be provided.
    if args.symbol.is_none()
        && args.footprint.is_none()
        && args.datasheet.is_none()
        && args.model_3d.is_none()
        && args.dir.is_none()
    {
        return Err(CliError::Diagnostic(Diagnostic {
            code: "CLI:NO_SOURCE".into(),
            severity: Severity::Error,
            message:
                "No source provided — pass --symbol, --footprint, --datasheet, --dir, or --model-3d"
                    .into(),
            entities: vec![],
            hint: None,
        }));
    }

    // If --dir is used, auto-detect lib_id from the directory contents.
    let lib_id = if let Some(ref dir) = args.dir {
        let discovered = discover::discover(std::path::Path::new(dir))?;
        if discovered.symbol.is_none() && discovered.footprint.is_none() {
            return Err(CliError::Diagnostic(Diagnostic {
                code: "CLI:NO_SOURCE".into(),
                severity: Severity::Error,
                message: format!(
                    "No .kicad_sym or .kicad_mod files found in '{}'",
                    dir
                ),
                entities: vec![],
                hint: Some("Place both files in the directory, or use --symbol / --footprint for individual files".into()),
            }));
        }
        discover::resolve_lib_id(args.lib_id.as_deref(), &discovered, None)?
    } else {
        // No --dir: --lib-id is required to locate the part TOML.
        args.lib_id.clone().ok_or_else(|| {
            CliError::Diagnostic(Diagnostic {
                code: "CLI:MISSING_LIB_ID".into(),
                severity: Severity::Error,
                message: "--lib-id is required when not using --dir".into(),
                entities: vec![],
                hint: Some("Use --dir to auto-detect, or provide --lib-id explicitly".into()),
            })
        })?
    };

    // Resolve the part TOML path from --crate + lib_id.
    let root = std::env::current_dir()?;
    let part_toml = root
        .join("parts")
        .join(&args.crate_)
        .join(format!("{}.toml", manifest::toml_stem(&lib_id)));

    if !part_toml.exists() {
        return Err(CliError::Diagnostic(Diagnostic {
            code: "CLI:PART_NOT_FOUND".into(),
            severity: Severity::Error,
            message: format!(
                "Part TOML not found at '{}' (crate={}, lib_id={})",
                part_toml.display(),
                args.crate_,
                lib_id
            ),
            entities: vec![lib_id.clone(), args.crate_.clone()],
            hint: Some("Run `copperleaf new --crate ...` first to create the part".into()),
        }));
    }

    let kindmap = KindMap::load(args.kind_map.as_deref())?;

    let source = std::fs::read_to_string(&part_toml)?;
    let mut manifest = manifest::deserialise(&source)?;

    // ── --dir: auto-discover symbol + footprint ─────────────────────
    if let Some(ref dir) = args.dir {
        let discovered = discover::discover(std::path::Path::new(dir))?;

        if let Some(ref sym_path) = discovered.symbol {
            merge_symbol_source(
                &mut manifest,
                sym_path.to_str().unwrap_or_default(),
                &lib_id,
                &kindmap,
                &args.default_kind,
            )?;
        }

        if let Some(ref fp_path) = discovered.footprint {
            merge_footprint_source(
                &mut manifest,
                fp_path.to_str().unwrap_or_default(),
                &lib_id,
                &args,
            )?;
        }
    }

    // ── Individual sources ──────────────────────────────────────────
    if let Some(ref path) = args.datasheet {
        manifest = crate::llm::update_from_datasheet(path, &args, &manifest)?;
    }

    if let Some(ref symbol_path) = args.symbol {
        merge_symbol_source(
            &mut manifest,
            symbol_path,
            &lib_id,
            &kindmap,
            &args.default_kind,
        )?;
    }

    if let Some(ref footprint_path) = args.footprint {
        merge_footprint_source(&mut manifest, footprint_path, &lib_id, &args)?;
    }

    // Allow --model-3d to override the model path.
    if let Some(ref model_3d) = args.model_3d {
        manifest.component.model_3d = Some(model_3d.clone());
    }

    manifest::embed_model_data(&mut manifest);

    let output = manifest::serialise(&manifest);
    let out_path = args
        .out
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| part_toml.clone());

    std::fs::write(&out_path, output)?;
    Ok(())
}

/// Merge symbol data from a file into the manifest.
fn merge_symbol_source(
    manifest: &mut copperleaf_part_codegen::Manifest,
    symbol_path: &str,
    cli_lib_id: &str,
    kindmap: &KindMap,
    default_kind: &str,
) -> Result<(), CliError> {
    manifest::check_extension(
        symbol_path,
        "kicad_mod",
        "CLI:FOOTPRINT_AS_SYMBOL",
        "a footprint file",
        "a symbol",
        "--footprint",
    )?;
    let sym_source = std::fs::read_to_string(symbol_path)?;
    let symbols = parse_symbol_lib(&sym_source)?;

    let lib_id = manifest::resolve_symbol_lib_id(
        Some(cli_lib_id),
        manifest.component.lib_id.as_deref(),
        &symbols,
        symbol_path,
    )?;
    // Idiot check: verify the source matches this part.
    if let Some(ref existing) = manifest.component.lib_id
        && *existing != lib_id
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
    let Some(symbol) = find_symbol(&symbols, &lib_id) else {
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
        crate::print_diagnostic(&Diagnostic {
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
    let diags = manifest::merge_symbol(manifest, &flat_pins, kindmap, default_kind);
    for d in &diags {
        crate::print_diagnostic(d);
    }
    // Inherit datasheet from symbol if the manifest doesn't have one.
    if manifest.component.datasheet.is_none() {
        manifest.component.datasheet = symbol.datasheet.clone();
    }
    // Merge symbol description into title (not stored as its own key).
    // Only do this when the title still matches the bare lib_id — if the
    // user has customised it we leave it alone.
    if let Some(ref desc) = symbol.description
        && let Some(clean) = manifest::clean_description(desc)
    {
        let default_title = manifest
            .component
            .lib_id
            .as_deref()
            .unwrap_or("");
        if manifest.component.title == default_title {
            manifest.component.title = format!("{} — {}", default_title, clean);
        }
    }
    Ok(())
}

/// Merge footprint data from a file into the manifest.
fn merge_footprint_source(
    manifest: &mut copperleaf_part_codegen::Manifest,
    footprint_path: &str,
    cli_lib_id: &str,
    args: &UpdateArgs,
) -> Result<(), CliError> {
    let resolved_lib_id = cli_lib_id;
    // Idiot check: verify the source matches this part.
    if let Some(ref existing) = manifest.component.lib_id
        && *existing != resolved_lib_id
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
        let Some((_, pads)) = lib.into_iter().find(|(name, _)| *name == resolved_lib_id) else {
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
        manifest::check_extension(
            footprint_path,
            "kicad_sym",
            "CLI:SYMBOL_AS_FOOTPRINT",
            "a symbol file",
            "a footprint",
            "--symbol",
        )?;
        copperleaf_backend_kicad::parse_footprint(footprint_path)?
    };
    // Idiot check: warn if pad count doesn't match pin count.
    let electrical_pad_count = pads
        .iter()
        .filter(|p| {
            if p.pad_type.eq_ignore_ascii_case("np_thru_hole")
                || p.number.eq_ignore_ascii_case("none")
                || p.number.is_empty()
            {
                return false;
            }
            if p.pad_type.eq_ignore_ascii_case("thru_hole") {
                return !manifest::is_thermal_via(p, &manifest.pins);
            }
            true
        })
        .count();
    if !manifest.pins.is_empty() && electrical_pad_count != manifest.pins.len() {
        crate::print_diagnostic(&Diagnostic {
            code: "CLI:PAD_COUNT_MISMATCH".into(),
            severity: Severity::Warning,
            message: format!(
                "Footprint has {} electrical pads, but part TOML has {} pins",
                electrical_pad_count,
                manifest.pins.len()
            ),
            entities: vec![],
            hint: Some("This may indicate the wrong footprint for this part".into()),
        });
    }
    let diags = manifest::merge_footprint(manifest, &pads);
    for d in &diags {
        crate::print_diagnostic(d);
    }

    // Extract fabrication body outline from the footprint source.
    {
        let fab_path: std::path::PathBuf = if std::fs::metadata(footprint_path)?.is_dir() {
            std::path::Path::new(footprint_path).join(format!("{}.kicad_mod", resolved_lib_id))
        } else {
            footprint_path.into()
        };
        if let Ok(Some(extent)) =
            copperleaf_backend_kicad::parse_footprint_fab_extent(&fab_path)
        {
            manifest.component.fab_extent = Some(extent);
        }
    }

    // Extract 3D model path from the footprint source, unless overridden
    // by --model-3d.
    if manifest.component.model_3d.is_none() && args.model_3d.is_none() {
        let extracted_model = if std::fs::metadata(footprint_path)?.is_dir() {
            copperleaf_backend_kicad::parse_footprint_model_lib(
                footprint_path,
                resolved_lib_id,
            )?
        } else {
            copperleaf_backend_kicad::parse_footprint_model(footprint_path)?
        };
        manifest.component.model_3d = extracted_model;

        if manifest.component.model_3d.is_none() {
            manifest.component.model_3d = manifest::find_step_file_alongside(footprint_path);
        }
    }

    Ok(())
}
