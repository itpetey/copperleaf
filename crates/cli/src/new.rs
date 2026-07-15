use std::path::PathBuf;

use copperleaf::{Diagnostic, Severity};
use copperleaf_backend_kicad::{find_symbol, flatten_extends, parse_symbol_lib};
use copperleaf_part_codegen::{ComponentMeta, Manifest};

use crate::{CliError, NewArgs, kindmap::KindMap, manifest, vendor};

pub fn run(args: NewArgs) -> Result<(), CliError> {
    let kindmap = KindMap::load(args.kind_map.as_deref())?;

    if let Some(ref path) = args.datasheet {
        return Err(crate::datasheet_stub(path));
    }

    if let Some(ref symbol_path) = args.symbol {
        return run_symbol(symbol_path, &args, &kindmap);
    }

    if let Some(ref footprint_path) = args.footprint {
        return run_footprint(footprint_path, &args, &kindmap);
    }

    Ok(())
}

fn run_symbol(symbol_path: &str, args: &NewArgs, kindmap: &KindMap) -> Result<(), CliError> {
    let lib_id = args.lib_id.as_deref().ok_or_else(|| {
        CliError::Diagnostic(Diagnostic {
            code: "CLI:MISSING_LIB_ID".into(),
            severity: Severity::Error,
            message: "--lib-id is required when using --symbol".into(),
            entities: vec![],
            hint: Some("Provide the symbol name within the library".into()),
        })
    })?;

    let source = std::fs::read_to_string(symbol_path)?;
    let symbols = parse_symbol_lib(&source)?;
    let Some(symbol) = find_symbol(&symbols, lib_id) else {
        return Err(CliError::Diagnostic(Diagnostic {
            code: "CLI:SYMBOL_NOT_FOUND".into(),
            severity: Severity::Error,
            message: format!("Symbol '{}' not found in '{}'", lib_id, symbol_path),
            entities: vec![lib_id.to_string()],
            hint: None,
        }));
    };

    // Resolve inherited pins; the returned S-expression is available for
    // inspection, and the symbol's `pins` field is already populated by
    // `parse_symbol_lib`.
    let _flattened = flatten_extends(symbol, &symbols);

    let title = args.title.clone().unwrap_or_else(|| lib_id.to_string());
    let description = args.description.clone();
    let mut manifest = Manifest {
        component: ComponentMeta {
            name: struct_name(lib_id),
            title,
            description,
        },
        pins: vec![],
        constraints: vec![],
    };

    let diags = manifest::merge_symbol(&mut manifest, &symbol.pins, kindmap, &args.default_kind);

    let output = manifest::serialise(&manifest);
    write_output(args, lib_id, &output, &diags)?;
    Ok(())
}

fn run_footprint(footprint_path: &str, args: &NewArgs, _kindmap: &KindMap) -> Result<(), CliError> {
    let lib_id = args.lib_id.clone().unwrap_or_default();
    let pads = if std::fs::metadata(footprint_path)?.is_dir() {
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
        pads
    } else {
        copperleaf_backend_kicad::parse_footprint(footprint_path)?
    };

    let title = args.title.clone().unwrap_or_else(|| lib_id.clone());
    let description = args.description.clone();
    let manifest = manifest::manifest_from_footprint(
        &pads,
        ComponentMeta {
            name: struct_name(&lib_id),
            title,
            description,
        },
        &args.default_kind,
    );

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
            .join(toml_filename(lib_id))
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

fn toml_filename(lib_id: &str) -> String {
    let mut out = String::new();
    for ch in lib_id.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("part");
    }
    format!("{}.toml", out)
}

fn struct_name(lib_id: &str) -> String {
    let mut out = String::new();
    let mut first = true;
    for ch in lib_id.chars() {
        if ch.is_ascii_alphanumeric() {
            if first {
                out.push(ch.to_ascii_uppercase());
            } else {
                out.push(ch.to_ascii_lowercase());
            }
            first = false;
        } else {
            first = true;
        }
    }
    if out.is_empty() {
        out.push_str("Part");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_filename_normalises() {
        assert_eq!(toml_filename("RP2354A"), "rp2354a.toml");
        assert_eq!(toml_filename("MM8108-MF15457"), "mm8108_mf15457.toml");
    }

    #[test]
    fn struct_name_normalises() {
        assert_eq!(struct_name("RP2354A"), "Rp2354a");
        assert_eq!(struct_name("MM8108-MF15457"), "Mm8108Mf15457");
    }
}
