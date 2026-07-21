use std::path::PathBuf;

use copperleaf::{Diagnostic, Severity};
use copperleaf_backend_kicad::{
    find_symbol, flatten_extends, parse_single_symbol, parse_symbol_lib,
};

use crate::{CliError, UpdateArgs, kindmap::KindMap, manifest};

pub fn run(args: UpdateArgs) -> Result<(), CliError> {
    // At least one source must be provided.
    if args.symbol.is_none()
        && args.footprint.is_none()
        && args.datasheet.is_none()
        && args.model_3d.is_none()
    {
        return Err(CliError::Diagnostic(Diagnostic {
            code: "CLI:NO_SOURCE".into(),
            severity: Severity::Error,
            message: "No source provided — pass --symbol, --footprint, --datasheet, or --model-3d"
                .into(),
            entities: vec![],
            hint: None,
        }));
    }

    let kindmap = KindMap::load(args.kind_map.as_deref())?;

    let source = std::fs::read_to_string(&args.part_toml)?;
    let mut manifest = manifest::deserialise(&source)?;

    if let Some(ref path) = args.datasheet {
        manifest = crate::llm::update_from_datasheet(path, &args, &manifest)?;
    }

    let mut diags = Vec::new();

    if let Some(ref symbol_path) = args.symbol {
        // Guard against accidentally passing a footprint file as a symbol.
        if let Some(ext) = std::path::Path::new(symbol_path)
            .extension()
            .and_then(|s| s.to_str())
        {
            if ext.eq_ignore_ascii_case("kicad_mod") {
                return Err(CliError::Diagnostic(Diagnostic {
                    code: "CLI:FOOTPRINT_AS_SYMBOL".into(),
                    severity: Severity::Error,
                    message: format!(
                        "'{}' is a footprint file, not a symbol — use --footprint instead",
                        symbol_path
                    ),
                    entities: vec![],
                    hint: None,
                }));
            }
        }
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
            .unwrap_or_default()
            .to_owned();
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
            // Guard against accidentally passing a symbol file as a footprint.
            if let Some(ext) = std::path::Path::new(footprint_path)
                .extension()
                .and_then(|s| s.to_str())
            {
                if ext.eq_ignore_ascii_case("kicad_sym") {
                    return Err(CliError::Diagnostic(Diagnostic {
                        code: "CLI:SYMBOL_AS_FOOTPRINT".into(),
                        severity: Severity::Error,
                        message: format!(
                            "'{}' is a symbol file, not a footprint — use --symbol instead",
                            footprint_path
                        ),
                        entities: vec![],
                        hint: None,
                    }));
                }
            }
            copperleaf_backend_kicad::parse_footprint(footprint_path)?
        };
        // Idiot check: warn if pad count doesn't match pin count.
        // Exclude mechanical pads (np_thru_hole, "None"-numbered, unnamed
        // paste-only stencil apertures, and thru_hole thermal vias that sit
        // inside an existing pad's bounding box).
        let electrical_pad_count = pads
            .iter()
            .filter(|p| {
                if p.pad_type.eq_ignore_ascii_case("np_thru_hole")
                    || p.number.eq_ignore_ascii_case("none")
                    || p.number.is_empty()
                {
                    return false;
                }
                // Thru-hole pads inside an existing pad are thermal vias.
                if p.pad_type.eq_ignore_ascii_case("thru_hole") {
                    return !is_thermal_via(p, &manifest.pins);
                }
                true
            })
            .count();
        if !manifest.pins.is_empty() && electrical_pad_count != manifest.pins.len() {
            diags.push(Diagnostic {
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
        diags.extend(manifest::merge_footprint(&mut manifest, &pads));

        // Extract 3D model path from the footprint source, unless overridden
        // by --model-3d.
        if manifest.component.model_3d.is_none() && args.model_3d.is_none() {
            let extracted_model = if std::fs::metadata(footprint_path)?.is_dir() {
                copperleaf_backend_kicad::parse_footprint_model_lib(
                    footprint_path,
                    &resolved_lib_id,
                )?
            } else {
                copperleaf_backend_kicad::parse_footprint_model(footprint_path)?
            };
            manifest.component.model_3d = extracted_model;

            // If no model found in the footprint S-expression, look for a
            // .step file alongside the footprint file.
            if manifest.component.model_3d.is_none() {
                manifest.component.model_3d = find_step_file_alongside(footprint_path);
            }
        }
    }

    // Allow --model-3d to override the model path.
    if let Some(ref model_3d) = args.model_3d {
        manifest.component.model_3d = Some(model_3d.clone());
    }

    // If we have a model path but no embedded data, read and embed the file.
    if let Some(ref model_path) = manifest.component.model_3d.clone() {
        if manifest.component.model_3d_data.is_none() {
            if let Ok(bytes) = std::fs::read(model_path) {
                use base64::Engine;
                manifest.component.model_3d_data =
                    Some(base64::engine::general_purpose::STANDARD.encode(&bytes));
            }
        }
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

/// Look for a `.step` file in the same directory as `footprint_path`.
///
/// If the footprint path is a directory (`.pretty` library), searches for any
/// `.step` file inside it.  Returns the first match as a `Some(String)`, or
/// `None` if nothing is found.
pub(crate) fn find_step_file_alongside(footprint_path: &str) -> Option<String> {
    let path = std::path::Path::new(footprint_path);
    let dir = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()?.to_path_buf()
    };

    for entry in std::fs::read_dir(&dir).ok()? {
        let entry = entry.ok()?;
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) == Some("step") {
            return p.to_str().map(|s| s.to_string());
        }
    }
    None
}

/// Return `true` if `pad` is a thru-hole that sits inside any existing pin's
/// bounding box (i.e. it is a thermal via, not an electrical pad).
fn is_thermal_via(
    pad: &copperleaf_backend_kicad::PadDef,
    pins: &[copperleaf_part_codegen::PinDef],
) -> bool {
    for pin in pins {
        // Skip pins that correspond to the same pad number — a pad is not a
        // thermal via of itself.
        if pin.number == pad.number {
            continue;
        }
        let Some((px, py)) = pin.pos else { continue };
        let half_w = pin.width.unwrap_or(0.0) / 2.0;
        let half_h = pin.height.or(pin.length).unwrap_or(0.0) / 2.0;
        if pad.pos.0 >= px - half_w
            && pad.pos.0 <= px + half_w
            && pad.pos.1 >= py - half_h
            && pad.pos.1 <= py + half_h
        {
            return true;
        }
    }
    false
}
