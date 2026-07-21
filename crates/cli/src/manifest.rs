//! Helpers for reading, writing, and merging Copperleaf part manifests.

use copperleaf::{Diagnostic, Severity};
use copperleaf_backend_kicad::{PadDef, sym_parser::PinDef as SymPinDef, sym_parser::SymbolDef};
use copperleaf_part_codegen::{
    CodegenError, ComponentMeta, Manifest, MechanicalDef, PinDef, ThermalViaDef, fmt_f64,
    required_fields,
};

use crate::kindmap::{KindEntry, KindMap};

/// Guard against passing a file of the wrong type. If `path` has extension
/// `bad_ext`, return a diagnostic error suggesting `--flag` instead.
pub(crate) fn check_extension(
    path: &str,
    bad_ext: &str,
    code: &str,
    wrong_kind: &str,
    right_kind: &str,
    flag: &str,
) -> Result<(), crate::CliError> {
    if let Some(ext) = std::path::Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        && ext.eq_ignore_ascii_case(bad_ext)
    {
        return Err(crate::CliError::Diagnostic(Diagnostic {
            code: code.into(),
            severity: Severity::Error,
            message: format!(
                "'{}' is {}, not a {} — use {} instead",
                path, wrong_kind, right_kind, flag
            ),
            entities: vec![],
            hint: None,
        }));
    }
    Ok(())
}

/// Read and base64-encode a 3D model file, storing it in the manifest, if a
/// model path is set and data hasn't already been embedded.
pub(crate) fn embed_model_data(manifest: &mut Manifest) {
    if let Some(ref model_path) = manifest.component.model_3d.clone()
        && manifest.component.model_3d_data.is_none()
        && let Ok(bytes) = std::fs::read(model_path)
    {
        use base64::Engine;
        manifest.component.model_3d_data =
            Some(base64::engine::general_purpose::STANDARD.encode(&bytes));
    }
}

/// Resolve the lib-id for a symbol source.
///
/// Priority: `args_lib_id` → `manifest_lib_id` (for update mode, pass
/// `None` for new) → auto-detect from a single-symbol file → error.
pub(crate) fn resolve_symbol_lib_id(
    args_lib_id: Option<&str>,
    manifest_lib_id: Option<&str>,
    symbols: &[SymbolDef],
    symbol_path: &str,
) -> Result<String, crate::CliError> {
    // Explicit via CLI arg or existing manifest.
    if let Some(id) = args_lib_id.or(manifest_lib_id) {
        return Ok(id.to_owned());
    }
    // Auto-detect from a single-symbol file.
    if symbols.len() == 1 {
        return Ok(symbols[0].lib_id.clone());
    }
    if symbols.is_empty() {
        return Err(crate::CliError::Diagnostic(Diagnostic {
            code: "CLI:NO_SYMBOLS".into(),
            severity: Severity::Error,
            message: format!("No symbols found in '{}'", symbol_path),
            entities: vec![],
            hint: None,
        }));
    }
    let names: Vec<String> = symbols.iter().map(|s| s.lib_id.clone()).collect();
    Err(crate::CliError::Diagnostic(Diagnostic {
        code: "CLI:MISSING_LIB_ID".into(),
        severity: Severity::Error,
        message: format!(
            "Multiple symbols found in '{}', --lib-id is required",
            symbol_path
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
    }))
}

/// Deserialise a TOML string into a manifest.
pub fn deserialise(input: &str) -> Result<Manifest, CodegenError> {
    toml::from_str(input).map_err(|e| CodegenError::Toml {
        path: "<string>".into(),
        source: e,
    })
}

/// Build a manifest from footprint pads alone.
pub fn manifest_from_footprint(
    pads: &[PadDef],
    component: ComponentMeta,
    default_kind: &str,
) -> Manifest {
    let mut pins = Vec::new();
    let mut mechanical = Vec::new();
    let mut next_num = 1;
    for pad in pads {
        if pad.number.eq_ignore_ascii_case("none")
            || pad.number.is_empty()
            || pad.pad_type == "np_thru_hole"
        {
            mechanical.push(MechanicalDef {
                number: pad.number.clone(),
                pos: pad.pos,
                width: pad.width,
                height: pad.height,
                pad_type: pad.pad_type.clone(),
                pad_shape: pad.shape.clone(),
                roundrect_rratio: pad.roundrect_rratio,
                layers: if pad.layers.is_empty() {
                    None
                } else {
                    Some(pad.layers.clone())
                },
                drill: pad.drill.unwrap_or(0.0),
            });
            continue;
        }
        let num = pin_number(&pad.number, &mut next_num);
        pins.push(PinDef {
            num,
            number: pad.number.clone(),
            name: format!("PAD_{}", pad.number),
            purpose: "Pad".into(),
            notes: String::new(),
            kind: default_kind.into(),
            bw_mhz: None,
            v: None,
            v_min: None,
            v_max: None,
            i: None,
            i_max: None,
            pos: Some(pad.pos),
            rotation: Some(pad.rotation),
            length: Some(pad.width.max(pad.height)),
            nc: None,
            width: Some(pad.width),
            height: Some(pad.height),
            pad_type: if pad.pad_type.is_empty() {
                None
            } else {
                Some(pad.pad_type.clone())
            },
            pad_shape: if pad.shape.is_empty() {
                None
            } else {
                Some(pad.shape.clone())
            },
            roundrect_rratio: pad.roundrect_rratio,
            solder_mask_margin: pad.solder_mask_margin,
            layers: if pad.layers.is_empty() {
                None
            } else {
                Some(pad.layers.clone())
            },
            drill: pad.drill,
            thermal_vias: vec![],
        });
    }
    pins.sort_by_key(|p| p.num);
    Manifest {
        component,
        pins,
        constraints: vec![],
        mechanical,
    }
}

/// Merge footprint pad data into an existing manifest.
///
/// Matched pads update pin geometry in-place. Unmatched pads that are small
/// through-holes inside an existing pad's area are reclassified as thermal
/// vias of that pad. Mechanical pads (np_thru_hole or "None"-numbered) are
/// captured separately. All other unmatched pads produce a warning.
pub fn merge_footprint(existing: &mut Manifest, pads: &[PadDef]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Footprint is authoritative for mechanical pads — replace on every merge.
    existing.mechanical.clear();

    // Clear all thermal vias — they're rebuilt from the footprint each time.
    for pin in &mut existing.pins {
        pin.thermal_vias.clear();
    }

    for pad in pads {
        // Mechanical-only pads (KiCad number "None"/"none", unnamed paste
        // apertures, or np_thru_hole).
        if pad.number.eq_ignore_ascii_case("none")
            || pad.number.is_empty()
            || pad.pad_type == "np_thru_hole"
        {
            existing.mechanical.push(MechanicalDef {
                number: pad.number.clone(),
                pos: pad.pos,
                width: pad.width,
                height: pad.height,
                pad_type: pad.pad_type.clone(),
                pad_shape: pad.shape.clone(),
                roundrect_rratio: pad.roundrect_rratio,
                layers: if pad.layers.is_empty() {
                    None
                } else {
                    Some(pad.layers.clone())
                },
                drill: pad.drill.unwrap_or(0.0),
            });
            continue;
        }
        // Fall back to matching by `num` when `number` is empty (pins created
        // by the `new` command from symbols lack a `number` field).
        if let Some(pin) = existing.pins.iter_mut().find(|p| {
            if p.number == pad.number {
                return true;
            }
            if p.number.is_empty() {
                let Ok(pad_num) = pad.number.parse::<usize>() else {
                    return false;
                };
                return p.num == pad_num;
            }
            false
        }) {
            // Update geometry from pad.
            if pin.number.is_empty() {
                pin.number = pad.number.clone();
            }
            pin.pos = Some(pad.pos);
            pin.rotation = Some(pad.rotation);
            pin.length = Some(pad.width.max(pad.height));
            pin.width = Some(pad.width);
            pin.height = Some(pad.height);
            if !pad.pad_type.is_empty() {
                pin.pad_type = Some(pad.pad_type.clone());
            }
            if !pad.shape.is_empty() {
                pin.pad_shape = Some(pad.shape.clone());
            }
            pin.roundrect_rratio = pad.roundrect_rratio;
            pin.solder_mask_margin = pad.solder_mask_margin;
            if !pad.layers.is_empty() {
                pin.layers = Some(pad.layers.clone());
            }
            pin.drill = pad.drill;
        } else if pad.pad_type == "thru_hole" {
            // This pad is a through-hole not matching any pin. Check if it
            // sits inside any existing pad's bounding box — if so, treat it
            // as a thermal via.
            if let Some(parent) = find_containing_pad(existing, pad) {
                parent.thermal_vias.push(ThermalViaDef {
                    pos: pad.pos,
                    drill: pad.drill.unwrap_or(0.3),
                    size: pad.width.max(pad.height),
                });
                continue;
            }
            diagnostics.push(Diagnostic {
                code: "CLI:UNMATCHED_PAD".into(),
                severity: Severity::Warning,
                message: format!(
                    "Footprint pad {} has no matching pin in the existing TOML",
                    pad.number
                ),
                entities: vec![pad.number.clone()],
                hint: None,
            });
        } else {
            diagnostics.push(Diagnostic {
                code: "CLI:UNMATCHED_PAD".into(),
                severity: Severity::Warning,
                message: format!(
                    "Footprint pad {} has no matching pin in the existing TOML",
                    pad.number
                ),
                entities: vec![pad.number.clone()],
                hint: None,
            });
        }
    }

    diagnostics
}

/// Merge symbol pin data into an existing manifest.
pub fn merge_symbol(
    existing: &mut Manifest,
    symbol: &[SymPinDef],
    kindmap: &KindMap,
    default_kind: &str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Start auto-increment counter past the highest existing num so that
    // auto-assigned numbers never collide with numeric pins.
    let mut next_num = existing.pins.iter().map(|p| p.num).max().unwrap_or(0) + 1;

    for sym_pin in symbol {
        let (entry, fallback) = kindmap.resolve(&sym_pin.name, &sym_pin.pin_type, default_kind);

        // Try to find an existing pin whose stored `number` matches.
        // Fall back to matching by `num` when `number` is empty (footprint-only
        // pins created by the `new` command lack a `number` field).
        let matched = existing.pins.iter_mut().find(|p| {
            if p.number == sym_pin.number {
                return true;
            }
            if p.number.is_empty() {
                let Ok(sym_num) = sym_pin.number.parse::<usize>() else {
                    return false;
                };
                return p.num == sym_num;
            }
            false
        });

        if let Some(pin) = matched {
            // Found by number string — update in place.
            if is_placeholder_name(&pin.name) && !sym_pin.name.is_empty() {
                pin.name = sym_pin.name.clone();
            }
            if pin.kind == default_kind || pin.kind.is_empty() {
                apply_entry(pin, &entry);
            }
        } else {
            let num = pin_number(&sym_pin.number, &mut next_num);
            let mut pin = PinDef {
                num,
                number: sym_pin.number.clone(),
                name: sym_pin.name.clone(),
                purpose: purpose_for_kind(&entry.kind).into(),
                notes: String::new(),
                kind: String::new(),
                bw_mhz: None,
                v: None,
                v_min: None,
                v_max: None,
                i: None,
                i_max: None,
                pos: Some(sym_pin.pos),
                rotation: Some(sym_pin.rotation),
                length: Some(sym_pin.length),
                nc: None,
                width: None,
                height: None,
                pad_type: None,
                pad_shape: None,
                roundrect_rratio: None,
                solder_mask_margin: None,
                layers: None,
                drill: None,
                thermal_vias: vec![],
            };
            apply_entry(&mut pin, &entry);
            existing.pins.push(pin);
            diagnostics.push(Diagnostic {
                code: "CLI:NEW_PIN".into(),
                severity: Severity::Warning,
                message: format!(
                    "Pin {} ({}) from source is not in the existing TOML; appending",
                    sym_pin.number, sym_pin.name
                ),
                entities: vec![sym_pin.number.clone(), sym_pin.name.clone()],
                hint: None,
            });
        }

        if fallback {
            diagnostics.push(Diagnostic {
                code: "CLI:UNKNOWN_PIN_TYPE".into(),
                severity: Severity::Warning,
                message: format!(
                    "Unrecognised pin type '{}' for pin {}; using default kind '{}'",
                    sym_pin.pin_type, sym_pin.name, default_kind
                ),
                entities: vec![sym_pin.name.clone()],
                hint: Some("Provide a --kind-map override if needed".into()),
            });
        }
    }

    existing.pins.sort_by_key(|p| p.num);
    diagnostics
}

/// Parse a KiCad pin/pad number string into a numeric `usize`.
///
/// Purely numeric strings are parsed directly. Non-numeric strings (common in
/// connectors, e.g. `"TD2+"`) get an auto-incrementing number starting at 1
/// so that every pin receives a unique identity.
pub fn pin_number(number: &str, counter: &mut usize) -> usize {
    if let Ok(n) = number.parse::<usize>() {
        n
    } else {
        let n = *counter;
        *counter += 1;
        n
    }
}

/// Serialise a manifest to TOML, with `# TODO` comments for power pins that
/// still need voltage or current limits.
pub fn serialise(manifest: &Manifest) -> String {
    let mut out = String::new();

    out.push_str("[component]\n");
    out.push_str(&format!("name = \"{}\"\n", manifest.component.name));
    out.push_str(&format!("title = \"{}\"\n", manifest.component.title));
    if let Some(desc) = &manifest.component.description {
        out.push_str(&format!("description = \"{}\"\n", desc));
    }
    if let Some(ds) = &manifest.component.datasheet {
        out.push_str(&format!("datasheet = \"{}\"\n", ds));
    }
    if let Some(lib_id) = &manifest.component.lib_id {
        out.push_str(&format!("lib_id = \"{}\"\n", escape_toml_string(lib_id)));
    }
    if let Some(model_3d) = &manifest.component.model_3d {
        out.push_str(&format!(
            "model_3d = \"{}\"\n",
            escape_toml_string(model_3d)
        ));
    }
    if let Some(model_3d_data) = &manifest.component.model_3d_data {
        out.push_str("model_3d_data = \"\"\"\n");
        out.push_str(model_3d_data);
        out.push_str("\"\"\"\n");
    }
    if let Some((x, y, z)) = manifest.component.model_3d_rotation {
        out.push_str(&format!("model_3d_rotation = [{x}, {y}, {z}]\n"));
    }
    if let Some((x, y, z)) = manifest.component.model_3d_offset {
        out.push_str(&format!("model_3d_offset = [{x}, {y}, {z}]\n"));
    }
    out.push('\n');

    for pin in &manifest.pins {
        out.push_str("[[pin]]\n");
        out.push_str(&format!("num = {}\n", pin.num));
        if !pin.number.is_empty() {
            out.push_str(&format!(
                "number = \"{}\"\n",
                escape_toml_string(&pin.number)
            ));
        }
        out.push_str(&format!("name = \"{}\"\n", pin.name));
        if !pin.purpose.is_empty() {
            out.push_str(&format!("purpose = \"{}\"\n", pin.purpose));
        }
        if !pin.notes.is_empty() {
            out.push_str(&format!("notes = \"{}\"\n", escape_toml_string(&pin.notes)));
        }
        out.push_str(&format!("kind = \"{}\"\n", pin.kind));

        if let Some(v) = pin.v {
            out.push_str(&format!("v = {}\n", fmt_f64(v)));
        }
        if let Some(v_min) = pin.v_min {
            out.push_str(&format!("v_min = {}\n", fmt_f64(v_min)));
        }
        if let Some(v_max) = pin.v_max {
            out.push_str(&format!("v_max = {}\n", fmt_f64(v_max)));
        }
        if let Some(i) = pin.i {
            out.push_str(&format!("i = {}\n", fmt_f64(i)));
        }
        if let Some(i_max) = pin.i_max {
            out.push_str(&format!("i_max = {}\n", fmt_f64(i_max)));
        }
        if let Some(bw) = pin.bw_mhz {
            out.push_str(&format!("bw_mhz = {}\n", fmt_f64(bw)));
        }
        if let Some(nc) = pin.nc {
            out.push_str(&format!("nc = {}\n", nc));
        }
        if let Some((x, y)) = pin.pos {
            out.push_str(&format!("pos = [{}, {}]\n", fmt_f64(x), fmt_f64(y)));
        }
        if let Some(r) = pin.rotation {
            out.push_str(&format!("rotation = {}\n", fmt_f64(r)));
        }
        if let Some(l) = pin.length {
            out.push_str(&format!("length = {}\n", fmt_f64(l)));
        }
        if let Some(w) = pin.width {
            out.push_str(&format!("width = {}\n", fmt_f64(w)));
        }
        if let Some(h) = pin.height {
            out.push_str(&format!("height = {}\n", fmt_f64(h)));
        }
        if let Some(ref pt) = pin.pad_type {
            out.push_str(&format!("pad_type = \"{}\"\n", pt));
        }
        if let Some(ref ps) = pin.pad_shape {
            out.push_str(&format!("pad_shape = \"{}\"\n", ps));
        }
        if let Some(rr) = pin.roundrect_rratio {
            out.push_str(&format!("roundrect_rratio = {}\n", fmt_f64(rr)));
        }
        if let Some(smm) = pin.solder_mask_margin {
            out.push_str(&format!("solder_mask_margin = {}\n", fmt_f64(smm)));
        }
        if let Some(ref layers) = pin.layers {
            out.push_str(&format!("layers = \"{}\"\n", escape_toml_string(layers)));
        }
        if let Some(drill) = pin.drill {
            out.push_str(&format!("drill = {}\n", fmt_f64(drill)));
        }
        if !pin.thermal_vias.is_empty() {
            out.push_str("thermal_vias = [\n");
            for via in &pin.thermal_vias {
                out.push_str(&format!(
                    "  {{ pos = [{}, {}], drill = {}, size = {} }},\n",
                    fmt_f64(via.pos.0),
                    fmt_f64(via.pos.1),
                    fmt_f64(via.drill),
                    fmt_f64(via.size)
                ));
            }
            out.push_str("]\n");
        }

        let missing = missing_power_fields(pin);
        if !missing.is_empty() {
            out.push_str(&format!("# TODO: fill {}\n", missing.join(", ")));
        }
        out.push('\n');
    }

    for constraint in &manifest.constraints {
        out.push_str("[[constraint]]\n");
        out.push_str(&format!("type = \"{}\"\n", constraint.ty));
        if let Some(values) = &constraint.values {
            out.push_str("values = [");
            for (i, v) in values.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&format!("\"{}\"", escape_toml_string(v)));
            }
            out.push_str("]\n");
        }
        if let Some(per_pin) = constraint.per_pin {
            out.push_str(&format!("per_pin = {}\n", per_pin));
        }
        if let Some(temp) = &constraint.temp {
            out.push_str(&format!("temp = \"{}\"\n", temp));
        }
        if let Some(group) = &constraint.group {
            out.push_str(&format!("group = \"{}\"\n", group));
        }
        if let Some(skew) = constraint.skew_ps {
            out.push_str(&format!("skew_ps = {}\n", fmt_f64(skew)));
        }
        if let Some(target) = &constraint.target {
            out.push_str(&format!("target = \"{}\"\n", target));
        }
        if let Some(tol) = constraint.tol_pct {
            out.push_str(&format!("tol_pct = {}\n", fmt_f64(tol)));
        }
        if let Some(requires_plane) = constraint.requires_plane {
            out.push_str(&format!("requires_plane = {}\n", requires_plane));
        }
        if let Some(min_width) = &constraint.min_width {
            out.push_str(&format!("min_width = \"{}\"\n", min_width));
        }
        if let Some(clearance) = &constraint.clearance {
            out.push_str(&format!("clearance = \"{}\"\n", clearance));
        }
        if let Some(min) = &constraint.min {
            out.push_str(&format!("min = \"{}\"\n", min));
        }
        if let Some(voltage) = &constraint.voltage {
            out.push_str(&format!("voltage = \"{}\"\n", voltage));
        }
        if let Some(max) = constraint.max {
            out.push_str(&format!("max = {}\n", fmt_f64(max)));
        }
        out.push('\n');
    }

    for mech in &manifest.mechanical {
        out.push_str("[[mechanical]]\n");
        if mech.number != "None" {
            out.push_str(&format!(
                "number = \"{}\"\n",
                escape_toml_string(&mech.number)
            ));
        }
        out.push_str(&format!(
            "pos = [{}, {}]\n",
            fmt_f64(mech.pos.0),
            fmt_f64(mech.pos.1)
        ));
        out.push_str(&format!("width = {}\n", fmt_f64(mech.width)));
        out.push_str(&format!("height = {}\n", fmt_f64(mech.height)));
        out.push_str(&format!("pad_type = \"{}\"\n", mech.pad_type));
        out.push_str(&format!("pad_shape = \"{}\"\n", mech.pad_shape));
        if let Some(rr) = mech.roundrect_rratio {
            out.push_str(&format!("roundrect_rratio = {}\n", fmt_f64(rr)));
        }
        if let Some(ref layers) = mech.layers {
            out.push_str(&format!("layers = \"{}\"\n", escape_toml_string(layers)));
        }
        if mech.drill > 0.0 {
            out.push_str(&format!("drill = {}\n", fmt_f64(mech.drill)));
        }
        out.push('\n');
    }

    // Ensure the file ends with exactly one newline.
    let trimmed = out.trim_end_matches('\n');
    format!("{trimmed}\n")
}

fn apply_entry(pin: &mut PinDef, entry: &KindEntry) {
    pin.kind = entry.kind.clone();
    if let Some(bw) = entry.bw_mhz {
        pin.bw_mhz = pin.bw_mhz.or(Some(bw));
    }
    if let Some(v) = entry.v {
        pin.v = pin.v.or(Some(v));
    }
    if let Some(v_min) = entry.v_min {
        pin.v_min = pin.v_min.or(Some(v_min));
    }
    if let Some(v_max) = entry.v_max {
        pin.v_max = pin.v_max.or(Some(v_max));
    }
    if let Some(i) = entry.i {
        pin.i = pin.i.or(Some(i));
    }
    if let Some(i_max) = entry.i_max {
        pin.i_max = pin.i_max.or(Some(i_max));
    }
    if let Some(nc) = entry.nc {
        pin.nc = pin.nc.or(Some(nc));
    }
}

fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Return a mutable reference to the first pin whose bounding box contains
/// Return `true` if `pos` falls inside the bounding box of a pin.
pub(crate) fn pin_contains_point(pin: &PinDef, pos: (f64, f64)) -> bool {
    let Some((px, py)) = pin.pos else {
        return false;
    };
    let half_w = pin.width.unwrap_or(0.0) / 2.0;
    let half_h = pin.height.or(pin.length).unwrap_or(0.0) / 2.0;
    pos.0 >= px - half_w && pos.0 <= px + half_w && pos.1 >= py - half_h && pos.1 <= py + half_h
}

/// Find the first pin in the manifest whose bounding box contains `pad`'s
/// centre point, or `None` if no pin contains it.
fn find_containing_pad<'a>(manifest: &'a mut Manifest, pad: &PadDef) -> Option<&'a mut PinDef> {
    manifest
        .pins
        .iter_mut()
        .find(|pin| pin_contains_point(pin, pad.pos))
}

fn is_placeholder_name(name: &str) -> bool {
    name.starts_with("PAD_")
}

fn missing_power_fields(pin: &PinDef) -> Vec<&'static str> {
    required_fields(&pin.kind)
        .iter()
        .filter(|&&field| match field {
            "bw_mhz" => pin.bw_mhz.is_none(),
            "v_min" => pin.v_min.is_none(),
            "v_max" => pin.v_max.is_none(),
            "i_max" => pin.i_max.is_none(),
            "v" => pin.v.is_none(),
            "i" => pin.i.is_none(),
            _ => false,
        })
        .copied()
        .collect()
}

fn purpose_for_kind(kind: &str) -> &'static str {
    match kind {
        "gnd" => "Ground",
        "pwr" | "pwr_fixed" | "pwr_out" => "Supply",
        _ => "I/O",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf_part_codegen::PinDef as CodegenPinDef;

    fn make_manifest() -> Manifest {
        Manifest {
            component: ComponentMeta {
                name: "Test".into(),
                title: "Test".into(),
                description: None,
                datasheet: None,
                lib_id: None,
                model_3d: None,
                model_3d_data: None,
                model_3d_rotation: None,
                model_3d_offset: None,
            },
            pins: vec![CodegenPinDef {
                num: 1,
                number: String::new(),
                name: "PAD_1".into(),
                purpose: "Pad".into(),
                notes: String::new(),
                kind: "dio".into(),
                bw_mhz: None,
                v: None,
                v_min: None,
                v_max: None,
                i: None,
                i_max: None,
                pos: None,
                rotation: None,
                length: None,
                nc: None,
                width: None,
                height: None,
                pad_type: None,
                pad_shape: None,
                roundrect_rratio: None,
                solder_mask_margin: None,
                layers: None,
                drill: None,
                thermal_vias: vec![],
            }],
            constraints: vec![],

            mechanical: vec![],
        }
    }

    #[test]
    fn serialise_deserialise_round_trip() {
        let mut manifest = make_manifest();
        manifest.pins[0].pos = Some((1.0, 2.0));
        manifest.pins[0].rotation = Some(90.0);
        manifest.pins[0].length = Some(2.54);
        let toml = serialise(&manifest);
        let parsed = deserialise(&toml).unwrap();
        assert_eq!(parsed.pins.len(), 1);
        assert_eq!(parsed.pins[0].pos, Some((1.0, 2.0)));
        assert_eq!(parsed.pins[0].rotation, Some(90.0));
        assert_eq!(parsed.pins[0].length, Some(2.54));
    }

    #[test]
    fn merge_symbol_replaces_placeholder_name() {
        let mut manifest = make_manifest();
        manifest.pins[0].number = "1".into();
        let sym = vec![SymPinDef {
            name: "VDD".into(),
            number: "1".into(),
            pos: (0.0, 0.0),
            rotation: 0.0,
            pin_type: "power_in".into(),
            length: 2.54,
        }];
        let kindmap = KindMap::load(None).unwrap();
        let diags = merge_symbol(&mut manifest, &sym, &kindmap, "dio");
        assert!(diags.is_empty(), "{:?}", diags);
        assert_eq!(manifest.pins[0].name, "VDD");
        assert_eq!(manifest.pins[0].kind, "pwr");
    }

    #[test]
    fn merge_symbol_preserves_manual_voltage() {
        let mut manifest = make_manifest();
        manifest.pins[0].number = "1".into();
        manifest.pins[0].name = "VDD".into();
        manifest.pins[0].kind = "pwr".into();
        manifest.pins[0].v_min = Some(1.8);
        manifest.pins[0].v_max = Some(3.3);
        manifest.pins[0].i_max = Some(0.1);
        let sym = vec![SymPinDef {
            name: "VDD".into(),
            number: "1".into(),
            pos: (0.0, 0.0),
            rotation: 0.0,
            pin_type: "power_in".into(),
            length: 2.54,
        }];
        let kindmap = KindMap::load(None).unwrap();
        merge_symbol(&mut manifest, &sym, &kindmap, "dio");
        assert_eq!(manifest.pins[0].v_min, Some(1.8));
        assert_eq!(manifest.pins[0].v_max, Some(3.3));
        assert_eq!(manifest.pins[0].i_max, Some(0.1));
    }

    #[test]
    fn merge_footprint_sets_pos_without_clobbering_kind() {
        let mut manifest = make_manifest();
        manifest.pins[0].number = "1".into();
        manifest.pins[0].name = "VDD".into();
        manifest.pins[0].kind = "pwr".into();
        let pads = vec![PadDef {
            number: "1".into(),
            pos: (10.0, 20.0),
            rotation: 45.0,
            width: 0.5,
            height: 0.25,
            pad_type: "smd".into(),
            shape: "rect".into(),
            roundrect_rratio: None,
            solder_mask_margin: None,
            layers: "F.Cu F.Mask F.Paste".into(),
            drill: None,
        }];
        let diags = merge_footprint(&mut manifest, &pads);
        assert!(diags.is_empty());
        assert_eq!(manifest.pins[0].kind, "pwr");
        assert_eq!(manifest.pins[0].pos, Some((10.0, 20.0)));
        assert_eq!(manifest.pins[0].rotation, Some(45.0));
        assert_eq!(manifest.pins[0].length, Some(0.5));
    }

    #[test]
    fn pin_number_parses_numeric() {
        let mut c = 1;
        assert_eq!(pin_number("1", &mut c), 1);
        assert_eq!(pin_number("42", &mut c), 42);
        assert_eq!(c, 1); // counter unchanged for numeric pins
    }

    #[test]
    fn pin_number_auto_increments_non_numeric() {
        let mut c = 1;
        assert_eq!(pin_number("TD2+", &mut c), 1);
        assert_eq!(pin_number("TD3-", &mut c), 2);
        assert_eq!(pin_number("RD2+", &mut c), 3);
        assert_eq!(c, 4);
    }

    #[test]
    fn merge_symbol_auto_increments_stringy_pins() {
        let mut manifest = Manifest {
            component: ComponentMeta {
                name: "Test".into(),
                title: "Test".into(),
                description: None,
                datasheet: None,
                lib_id: None,
                model_3d: None,
                model_3d_data: None,
                model_3d_rotation: None,
                model_3d_offset: None,
            },
            pins: vec![],
            constraints: vec![],

            mechanical: vec![],
        };
        let sym = vec![
            SymPinDef {
                name: "TD2+".into(),
                number: "TD2+".into(),
                pos: (0.0, 0.0),
                rotation: 0.0,
                pin_type: "input".into(),
                length: 2.54,
            },
            SymPinDef {
                name: "TD3-".into(),
                number: "TD3-".into(),
                pos: (0.0, 2.54),
                rotation: 0.0,
                pin_type: "input".into(),
                length: 2.54,
            },
            SymPinDef {
                name: "RD2+".into(),
                number: "RD2+".into(),
                pos: (0.0, 5.08),
                rotation: 0.0,
                pin_type: "output".into(),
                length: 2.54,
            },
        ];
        let kindmap = KindMap::load(None).unwrap();
        let diags = merge_symbol(&mut manifest, &sym, &kindmap, "dio");
        assert_eq!(manifest.pins.len(), 3, "all three pins should be present");
        assert_eq!(manifest.pins[0].num, 1);
        assert_eq!(manifest.pins[0].number, "TD2+");
        assert_eq!(manifest.pins[0].name, "TD2+");
        assert_eq!(manifest.pins[1].num, 2);
        assert_eq!(manifest.pins[1].number, "TD3-");
        assert_eq!(manifest.pins[1].name, "TD3-");
        assert_eq!(manifest.pins[2].num, 3);
        assert_eq!(manifest.pins[2].number, "RD2+");
        assert_eq!(manifest.pins[2].name, "RD2+");
        // All three should have produced NEW_PIN diagnostics.
        let new_pin_diags: Vec<_> = diags.iter().filter(|d| d.code == "CLI:NEW_PIN").collect();
        assert_eq!(new_pin_diags.len(), 3);

        // Re-merging the same symbol should be idempotent.
        let diags2 = merge_symbol(&mut manifest, &sym, &kindmap, "dio");
        assert_eq!(
            manifest.pins.len(),
            3,
            "re-merge should not add duplicate pins"
        );
        let new_pin_diags2: Vec<_> = diags2.iter().filter(|d| d.code == "CLI:NEW_PIN").collect();
        assert!(
            new_pin_diags2.is_empty(),
            "re-merge should produce no NEW_PIN diagnostics"
        );
    }
}
