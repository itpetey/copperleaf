use std::path::PathBuf;

use copperleaf::{Diagnostic, Severity};
use copperleaf_backend_kicad::{find_symbol, parse_symbol_lib};
use copperleaf_part_codegen::{ComponentMeta, Manifest};

use crate::{CliError, NewArgs, discover, kindmap::KindMap, manifest, vendor};

pub fn run(args: NewArgs) -> Result<(), CliError> {
    let kindmap = KindMap::load(args.kind_map.as_deref())?;

    if let Some(ref path) = args.datasheet {
        return run_datasheet(path, &args);
    }

    if let Some(ref dir) = args.dir {
        return run_dir(dir, &args, &kindmap);
    }

    if let Some(ref symbol_path) = args.symbol {
        return run_symbol(symbol_path, &args, &kindmap);
    }

    if let Some(ref footprint_path) = args.footprint {
        return run_footprint(footprint_path, &args, &kindmap);
    }

    Ok(())
}

/// Build a manifest from a single symbol file using a pre-resolved `lib_id`.
fn build_manifest_from_symbol(
    symbol_path: &str,
    lib_id: &str,
    args: &NewArgs,
    kindmap: &KindMap,
    diags: &mut Vec<Diagnostic>,
) -> Result<Manifest, CliError> {
    manifest::check_extension(
        symbol_path,
        "kicad_mod",
        "CLI:FOOTPRINT_AS_SYMBOL",
        "a footprint file",
        "a symbol",
        "--footprint",
    )?;
    let source = std::fs::read_to_string(symbol_path)?;
    let symbols = parse_symbol_lib(&source)?;

    let Some(symbol) = find_symbol(&symbols, lib_id) else {
        return Err(CliError::Diagnostic(Diagnostic {
            code: "CLI:SYMBOL_NOT_FOUND".into(),
            severity: Severity::Error,
            message: format!("Symbol '{}' not found in '{}'", lib_id, symbol_path),
            entities: vec![lib_id.into()],
            hint: None,
        }));
    };

    let mut title = args.title.clone().unwrap_or_else(|| lib_id.to_string());
    // Merge symbol description into the title.
    if let Some(ref desc) = symbol.description
        && let Some(clean) = manifest::clean_description(desc)
    {
        title = format!("{} — {}", title, clean);
    }
    // --description CLI arg still sets the description key explicitly.
    let description = args.description.clone();
    let mut manifest = Manifest {
        component: ComponentMeta {
            name: manifest::struct_name(lib_id),
            title,
            description,
            datasheet: symbol.datasheet.clone(),
            lib_id: Some(lib_id.to_string()),
            model_3d: None,
            model_3d_data: None,
            model_3d_rotation: None,
            model_3d_offset: None,
            fab_extent: None,
        },
        pins: vec![],
        constraints: vec![],
        layout: Default::default(),
        mechanical: vec![],
    };

    diags.extend(manifest::merge_symbol(
        &mut manifest,
        &symbol.pins,
        kindmap,
        &args.default_kind,
    ));

    Ok(manifest)
}

fn run_datasheet(path: &str, args: &NewArgs) -> Result<(), CliError> {
    let manifest = crate::llm::new_from_datasheet(path, args)?;

    let lib_id = args
        .lib_id
        .clone()
        .or_else(|| manifest.component.lib_id.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| manifest.component.name.clone());

    if lib_id.is_empty() {
        return Err(CliError::Diagnostic(Diagnostic {
            code: "CLI:MISSING_LIB_ID".into(),
            severity: Severity::Error,
            message: "Could not determine a part identifier for the output file".into(),
            entities: vec![],
            hint: Some(
                "Provide --lib-id, or make sure the LLM emits component.name or component.lib_id"
                    .into(),
            ),
        }));
    }

    let output = crate::manifest::serialise(&manifest);
    write_output(args, &lib_id, &output, &[])?;
    Ok(())
}

/// Process a directory, auto-discovering `.kicad_sym` and `.kicad_mod` files.
/// The `lib_id` is auto-detected from the directory contents unless `--lib-id`
/// is provided explicitly.
fn run_dir(dir: &str, args: &NewArgs, kindmap: &KindMap) -> Result<(), CliError> {
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

    // Resolve lib_id once — explicit arg takes priority, else auto-detect.
    let lib_id = discover::resolve_lib_id(args.lib_id.as_deref(), &discovered, None)?;

    let mut diags = Vec::new();

    // ── Symbol ──────────────────────────────────────────────────────
    let mut manifest = if let Some(ref sym_path) = discovered.symbol {
        let sym_path_str = sym_path.to_str().unwrap_or_default();
        build_manifest_from_symbol(sym_path_str, &lib_id, args, kindmap, &mut diags)?
    } else {
        // No symbol — seed with an empty manifest; footprint will fill it in.
        let title = args.title.clone().unwrap_or_else(|| lib_id.clone());
        let description = args.description.clone();
        Manifest {
            component: ComponentMeta {
                name: manifest::struct_name(&lib_id),
                title,
                description,
                datasheet: None,
                lib_id: Some(lib_id.clone()),
                model_3d: None,
                model_3d_data: None,
                model_3d_rotation: None,
                model_3d_offset: None,
                fab_extent: None,
            },
            pins: vec![],
            constraints: vec![],
            layout: Default::default(),
            mechanical: vec![],
        }
    };

    // ── Footprint ───────────────────────────────────────────────────
    if let Some(ref fp_path) = discovered.footprint {
        let fp_path_str = fp_path.to_str().unwrap_or_default();

        let (pads, extracted_model) = if std::fs::metadata(fp_path)?.is_dir() {
            let lib = copperleaf_backend_kicad::parse_footprint_lib(fp_path_str)?;
            let Some((_, pads)) = lib.into_iter().find(|(name, _)| *name == lib_id) else {
                return Err(CliError::Diagnostic(Diagnostic {
                    code: "CLI:FOOTPRINT_NOT_FOUND".into(),
                    severity: Severity::Error,
                    message: format!("Footprint '{}' not found in '{}'", lib_id, fp_path_str),
                    entities: vec![lib_id.clone()],
                    hint: None,
                }));
            };
            let model = copperleaf_backend_kicad::parse_footprint_model_lib(fp_path_str, &lib_id)?;
            (pads, model)
        } else {
            manifest::check_extension(
                fp_path_str,
                "kicad_sym",
                "CLI:SYMBOL_AS_FOOTPRINT",
                "a symbol file",
                "a footprint",
                "--symbol",
            )?;
            let model = copperleaf_backend_kicad::parse_footprint_model(fp_path_str)?;
            (
                copperleaf_backend_kicad::parse_footprint(fp_path_str)?,
                model,
            )
        };

        let model_3d = args
            .model_3d
            .clone()
            .or(extracted_model)
            .or_else(|| manifest::find_step_file_alongside(fp_path_str));

        if model_3d.is_some() && manifest.component.model_3d.is_none() {
            manifest.component.model_3d = model_3d;
        }

        if manifest.pins.is_empty() {
            // No symbol was processed — build footprint-only manifest.
            let title = args.title.clone().unwrap_or_else(|| lib_id.clone());
            let description = args.description.clone();
            let mut m = manifest::manifest_from_footprint(
                &pads,
                ComponentMeta {
                    name: manifest::struct_name(&lib_id),
                    title,
                    description,
                    datasheet: None,
                    lib_id: Some(lib_id.clone()),
                    model_3d: manifest.component.model_3d.clone(),
                    model_3d_data: None,
                    model_3d_rotation: None,
                    model_3d_offset: None,
                    fab_extent: None,
                },
                &args.default_kind,
            );
            manifest::embed_model_data(&mut m);
            let output = manifest::serialise(&m);
            write_output(args, &lib_id, &output, &diags)?;
            return Ok(());
        }

        // Merge footprint into existing symbol manifest.
        diags.extend(manifest::merge_footprint(&mut manifest, &pads));
    }

    // ── Finalise ────────────────────────────────────────────────────
    manifest::embed_model_data(&mut manifest);
    let output = manifest::serialise(&manifest);
    write_output(args, &lib_id, &output, &diags)?;
    Ok(())
}

fn run_footprint(footprint_path: &str, args: &NewArgs, _kindmap: &KindMap) -> Result<(), CliError> {
    manifest::check_extension(
        footprint_path,
        "kicad_sym",
        "CLI:SYMBOL_AS_FOOTPRINT",
        "a symbol file",
        "a footprint",
        "--symbol",
    )?;
    let lib_id = args.lib_id.clone().unwrap_or_default();
    let (pads, extracted_model) = if std::fs::metadata(footprint_path)?.is_dir() {
        let lib = copperleaf_backend_kicad::parse_footprint_lib(footprint_path)?;
        let Some((_, pads)) = lib.into_iter().find(|(name, _)| name == &lib_id) else {
            return Err(CliError::Diagnostic(Diagnostic {
                code: "CLI:FOOTPRINT_NOT_FOUND".into(),
                severity: Severity::Error,
                message: format!("Footprint '{}' not found in '{}'", lib_id, footprint_path),
                entities: vec![lib_id.clone()],
                hint: None,
            }));
        };
        let model = copperleaf_backend_kicad::parse_footprint_model_lib(footprint_path, &lib_id)?;
        (pads, model)
    } else {
        let model = copperleaf_backend_kicad::parse_footprint_model(footprint_path)?;
        (
            copperleaf_backend_kicad::parse_footprint(footprint_path)?,
            model,
        )
    };

    let model_3d = args
        .model_3d
        .clone()
        .or(extracted_model)
        .or_else(|| manifest::find_step_file_alongside(footprint_path));
    let title = args.title.clone().unwrap_or_else(|| lib_id.clone());
    let description = args.description.clone();
    let mut manifest = manifest::manifest_from_footprint(
        &pads,
        ComponentMeta {
            name: manifest::struct_name(&lib_id),
            title,
            description,
            datasheet: None,
            lib_id: Some(lib_id.clone()),
            model_3d,
            model_3d_data: None,
            model_3d_rotation: None,
            model_3d_offset: None,
            fab_extent: None,
        },
        &args.default_kind,
    );

    manifest::embed_model_data(&mut manifest);

    let output = manifest::serialise(&manifest);
    let diags = vec![Diagnostic {
        code: "CLI:ANON_PAD_NAMES".into(),
        severity: Severity::Warning,
        message: "Pin names were synthesised from pad numbers".into(),
        entities: vec![],
        hint: Some("Run update --symbol to replace placeholder names".into()),
    }];
    write_output(args, &lib_id, &output, &diags)?;
    Ok(())
}

fn run_symbol(symbol_path: &str, args: &NewArgs, kindmap: &KindMap) -> Result<(), CliError> {
    manifest::check_extension(
        symbol_path,
        "kicad_mod",
        "CLI:FOOTPRINT_AS_SYMBOL",
        "a footprint file",
        "a symbol",
        "--footprint",
    )?;
    let source = std::fs::read_to_string(symbol_path)?;
    let symbols = parse_symbol_lib(&source)?;

    let lib_id =
        manifest::resolve_symbol_lib_id(args.lib_id.as_deref(), None, &symbols, symbol_path)?;

    let Some(symbol) = find_symbol(&symbols, &lib_id) else {
        return Err(CliError::Diagnostic(Diagnostic {
            code: "CLI:SYMBOL_NOT_FOUND".into(),
            severity: Severity::Error,
            message: format!("Symbol '{}' not found in '{}'", lib_id, symbol_path),
            entities: vec![lib_id.clone()],
            hint: None,
        }));
    };

    let mut title = args.title.clone().unwrap_or_else(|| lib_id.to_string());
    // Merge symbol description into the title.
    if let Some(ref desc) = symbol.description
        && let Some(clean) = manifest::clean_description(desc)
    {
        title = format!("{} — {}", title, clean);
    }
    // --description CLI arg still sets the description key explicitly.
    let description = args.description.clone();
    let mut manifest = Manifest {
        component: ComponentMeta {
            name: manifest::struct_name(&lib_id),
            title,
            description,
            datasheet: symbol.datasheet.clone(),
            lib_id: Some(lib_id.to_string()),
            model_3d: None,
            model_3d_data: None,
            model_3d_rotation: None,
            model_3d_offset: None,
            fab_extent: None,
        },
        pins: vec![],
        constraints: vec![],
        layout: Default::default(),
        mechanical: vec![],
    };

    let diags = manifest::merge_symbol(&mut manifest, &symbol.pins, kindmap, &args.default_kind);

    let output = manifest::serialise(&manifest);
    write_output(args, &lib_id, &output, &diags)?;
    Ok(())
}

fn write_output(
    args: &NewArgs,
    lib_id: &str,
    output: &str,
    diags: &[Diagnostic],
) -> Result<(), CliError> {
    for d in diags {
        crate::print_diagnostic(d);
    }

    let out_path = if let Some(path) = &args.out {
        PathBuf::from(path)
    } else if let Some(vendor) = &args.crate_ {
        let root = std::env::current_dir()?;
        vendor::scaffold(&root, vendor, lib_id)?;
        PathBuf::from("parts")
            .join(vendor)
            .join(format!("{}.toml", manifest::toml_stem(lib_id)))
    } else {
        print!("{}", output);
        return Ok(());
    };

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&out_path, output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_stem_normalises() {
        assert_eq!(manifest::toml_stem("RP2354A"), "rp2354a");
        assert_eq!(manifest::toml_stem("MM8108-MF15457"), "mm8108_mf15457");
    }

    #[test]
    fn struct_name_normalises() {
        assert_eq!(manifest::struct_name("RP2354A"), "Rp2354a");
        assert_eq!(manifest::struct_name("MM8108-MF15457"), "Mm8108Mf15457");
    }
}
