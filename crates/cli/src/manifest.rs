//! Helpers for reading, writing, and merging Copperleaf part manifests.

use copperleaf::{Diagnostic, Severity};
use copperleaf_backend_kicad::{PadDef, sym_parser::PinDef as SymPinDef};
use copperleaf_part_codegen::{CodegenError, ComponentMeta, Manifest, PinDef};

use crate::kindmap::{KindEntry, KindMap};

/// Deserialise a TOML string into a manifest.
pub fn deserialise(input: &str) -> Result<Manifest, CodegenError> {
    toml::from_str(input).map_err(|e| CodegenError::Toml {
        path: "<string>".into(),
        source: e,
    })
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
    out.push('\n');

    for pin in &manifest.pins {
        out.push_str("[[pin]]\n");
        out.push_str(&format!("num = {}\n", pin.num));
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

    out
}

fn missing_power_fields(pin: &PinDef) -> Vec<&'static str> {
    match pin.kind.as_str() {
        "pwr" => {
            let mut missing = Vec::new();
            if pin.v_min.is_none() {
                missing.push("v_min");
            }
            if pin.v_max.is_none() {
                missing.push("v_max");
            }
            if pin.i_max.is_none() {
                missing.push("i_max");
            }
            missing
        }
        "pwr_fixed" | "pwr_out" => {
            let mut missing = Vec::new();
            if pin.v.is_none() {
                missing.push("v");
            }
            if pin.i.is_none() {
                missing.push("i");
            }
            missing
        }
        _ => vec![],
    }
}

fn fmt_f64(v: f64) -> String {
    format!("{:?}", v)
}

fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn purpose_for_kind(kind: &str) -> &'static str {
    match kind {
        "gnd" => "Ground",
        "pwr" | "pwr_fixed" | "pwr_out" => "Supply",
        _ => "I/O",
    }
}

fn is_placeholder_name(name: &str) -> bool {
    name.starts_with("PAD_")
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

/// Merge symbol pin data into an existing manifest.
pub fn merge_symbol(
    existing: &mut Manifest,
    symbol: &[SymPinDef],
    kindmap: &KindMap,
    default_kind: &str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for sym_pin in symbol {
        let num = sym_pin.number.parse::<usize>().unwrap_or(0);
        let (entry, fallback) = kindmap.resolve(&sym_pin.name, &sym_pin.pin_type, default_kind);

        if let Some(pin) = existing.pins.iter_mut().find(|p| p.num == num) {
            if is_placeholder_name(&pin.name) && !sym_pin.name.is_empty() {
                pin.name = sym_pin.name.clone();
            }
            if pin.kind == default_kind || pin.kind.is_empty() {
                apply_entry(pin, &entry);
            }
        } else {
            let mut pin = PinDef {
                num,
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
            };
            apply_entry(&mut pin, &entry);
            existing.pins.push(pin);
            diagnostics.push(Diagnostic {
                code: "CLI:NEW_PIN".into(),
                severity: Severity::Warning,
                message: format!(
                    "Pin {} ({}) from source is not in the existing TOML; appending",
                    num, sym_pin.name
                ),
                entities: vec![format!("{}", num), sym_pin.name.clone()],
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

/// Merge footprint pad data into an existing manifest.
pub fn merge_footprint(existing: &mut Manifest, pads: &[PadDef]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for pad in pads {
        let num = pad.number.parse::<usize>().unwrap_or(0);
        if let Some(pin) = existing.pins.iter_mut().find(|p| p.num == num) {
            pin.pos = Some(pad.pos);
            pin.rotation = Some(pad.rotation);
            pin.length = Some(pad.width.max(pad.height));
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

/// Build a manifest from footprint pads alone.
pub fn manifest_from_footprint(
    pads: &[PadDef],
    component: ComponentMeta,
    default_kind: &str,
) -> Manifest {
    let mut pins = Vec::new();
    for pad in pads {
        let num = pad.number.parse::<usize>().unwrap_or(0);
        pins.push(PinDef {
            num,
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
        });
    }
    pins.sort_by_key(|p| p.num);
    Manifest {
        component,
        pins,
        constraints: vec![],
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
            },
            pins: vec![CodegenPinDef {
                num: 1,
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
            }],
            constraints: vec![],
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
        manifest.pins[0].name = "VDD".into();
        manifest.pins[0].kind = "pwr".into();
        let pads = vec![PadDef {
            number: "1".into(),
            pos: (10.0, 20.0),
            rotation: 45.0,
            width: 0.5,
            height: 0.25,
            pad_type: "smd".into(),
        }];
        let diags = merge_footprint(&mut manifest, &pads);
        assert!(diags.is_empty());
        assert_eq!(manifest.pins[0].kind, "pwr");
        assert_eq!(manifest.pins[0].pos, Some((10.0, 20.0)));
        assert_eq!(manifest.pins[0].rotation, Some(45.0));
        assert_eq!(manifest.pins[0].length, Some(0.5));
    }
}
