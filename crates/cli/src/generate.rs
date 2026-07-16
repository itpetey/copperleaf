//! `generate` subcommand — emit KiCad files from part TOML manifests.

use std::path::PathBuf;

use crate::{CliError, GenerateFootprintArgs, GenerateSymbolArgs, manifest};

pub fn footprint(args: GenerateFootprintArgs) -> Result<(), CliError> {
    let source = std::fs::read_to_string(&args.part_toml)?;
    let manifest = manifest::deserialise(&source)?;

    let kicad_mod = copperleaf_backend_kicad::emit_footprint(&manifest);

    let out_path = match args.out {
        Some(ref p) => PathBuf::from(p),
        None => default_out(&args.part_toml, &manifest, "kicad_mod"),
    };

    std::fs::write(&out_path, kicad_mod)?;
    eprintln!("Wrote footprint to {}", out_path.display());
    Ok(())
}

pub fn symbol(args: GenerateSymbolArgs) -> Result<(), CliError> {
    let source = std::fs::read_to_string(&args.part_toml)?;
    let manifest = manifest::deserialise(&source)?;

    let kicad_sym = copperleaf_backend_kicad::emit_symbol(&manifest);

    let out_path = match args.out {
        Some(ref p) => PathBuf::from(p),
        None => default_out(&args.part_toml, &manifest, "kicad_sym"),
    };

    std::fs::write(&out_path, kicad_sym)?;
    eprintln!("Wrote symbol to {}", out_path.display());
    Ok(())
}

fn default_out(
    toml_path: &str,
    manifest: &copperleaf_part_codegen::Manifest,
    extension: &str,
) -> PathBuf {
    let toml_path = PathBuf::from(toml_path);
    let stem = toml_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("part");
    let lib_id = manifest.component.lib_id.as_deref().unwrap_or(stem);
    toml_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join(format!("{}.{}", lib_id, extension))
}
