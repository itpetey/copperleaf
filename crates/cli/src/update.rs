use std::path::PathBuf;

use copperleaf::{Diagnostic, Severity};
use copperleaf_backend_kicad::{find_symbol, flatten_extends, parse_symbol_lib};

use crate::{CliError, UpdateArgs, kindmap::KindMap, manifest};

pub fn run(args: UpdateArgs) -> Result<(), CliError> {
    let kindmap = KindMap::load(args.kind_map.as_deref())?;

    if args.datasheet.is_some() {
        return Err(crate::datasheet_stub(""));
    }

    let source = std::fs::read_to_string(&args.part_toml)?;
    let mut manifest = manifest::deserialise(&source)?;

    let mut diags = Vec::new();

    if let Some(ref symbol_path) = args.symbol {
        let lib_id = args.lib_id.as_deref().ok_or_else(|| {
            CliError::Diagnostic(Diagnostic {
                code: "CLI:MISSING_LIB_ID".into(),
                severity: Severity::Error,
                message: "--lib-id is required when using --symbol".into(),
                entities: vec![],
                hint: Some("Provide the symbol name within the library".into()),
            })
        })?;
        let sym_source = std::fs::read_to_string(symbol_path)?;
        let symbols = parse_symbol_lib(&sym_source)?;
        let Some(symbol) = find_symbol(&symbols, lib_id) else {
            return Err(CliError::Diagnostic(Diagnostic {
                code: "CLI:SYMBOL_NOT_FOUND".into(),
                severity: Severity::Error,
                message: format!("Symbol '{}' not found in '{}'", lib_id, symbol_path),
                entities: vec![lib_id.to_string()],
                hint: None,
            }));
        };
        let _flattened = flatten_extends(symbol, &symbols);
        diags.extend(manifest::merge_symbol(
            &mut manifest,
            &symbol.pins,
            &kindmap,
            &args.default_kind,
        ));
    }

    if let Some(ref footprint_path) = args.footprint {
        let pads = if std::fs::metadata(footprint_path)?.is_dir() {
            let lib_id = args.lib_id.as_deref().unwrap_or_default();
            let lib = copperleaf_backend_kicad::parse_footprint_lib(footprint_path)?;
            let Some((_, pads)) = lib.into_iter().find(|(name, _)| name == lib_id) else {
                return Err(CliError::Diagnostic(Diagnostic {
                    code: "CLI:FOOTPRINT_NOT_FOUND".into(),
                    severity: Severity::Error,
                    message: format!("Footprint '{}' not found in '{}'", lib_id, footprint_path),
                    entities: vec![lib_id.to_string()],
                    hint: None,
                }));
            };
            pads
        } else {
            copperleaf_backend_kicad::parse_footprint(footprint_path)?
        };
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
