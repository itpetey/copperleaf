//! TOML-to-Rust generator for Copperleaf component definitions.
//!
//! The generator reads one TOML file per component from a directory and emits a
//! single Rust source file containing documented modules for each component.
//! Rendering is driven by a Mustache template so the output format can be
//! changed without recompiling the generator logic.
//!
//! # TOML schema
//!
//! ```toml
//! [component]
//! name = "Mm8108Mf15457"      # Rust struct name
//! title = "..."               # Used for module and struct docs
//! description = "..."         # Optional extra doc paragraph
//! datasheet = "https://..."   # Optional URL to the component datasheet
//!
//! [[pin]]
//! num = 1
//! name = "GND_1"
//! purpose = "Ground"
//! notes = ""                  # Optional, rendered in the pinout table
//! kind = "gnd"                # Selects the PinBuilder expression
//!
//! [[constraint]]
//! type = "Decoupling"
//! values = ["100.0.nf()", "10.0.uf()"]
//! per_pin = false
//! ```
//!
//! Supported `kind` values and required fields:
//!
//! | kind        | required fields                | emitted expression                         |
//! |-------------|--------------------------------|--------------------------------------------|
//! | `gnd`       |                                | `.gnd()`                                   |
//! | `dio`       |                                | `.dio()`                                   |
//! | `analog_in` |                                | `.analog_in()`                             |
//! | `analog_rf` |                                | `.role(Role::AnalogIn).rf_limits().pin()` |
//! | `clk`       | `bw_mhz`                       | `.clk(bw_mhz)`                             |
//! | `spi`       | `bw_mhz`                       | `.spi(bw_mhz)`                             |
//! | `pwr`       | `v_min`, `v_max`, `i_max`      | `.pwr(v_min.volt(), v_max.volt(), i_max.amp()).pin()` |
//! | `pwr_fixed` | `v`, `i`                       | `.pwr_fixed(v.volt(), i.amp()).pin()`      |
//! | `pwr_out`   | `v`, `i`                       | Power output with fixed voltage            |

use std::{
    borrow::Cow,
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use mustache2::render::{RenderManager, SourceCache, provider::SourceProvider};
use serde::{Deserialize, Serialize, Serializer};

const DEFAULT_TEMPLATE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/component.mustache");
const TEMPLATE_KEY: &str = "component";

/// A [`SourceProvider`] backed by an owned template string.
struct TemplateProvider {
    source: String,
}

/// Holds a loaded template and its renderer.
struct MustacheRenderer(RenderManager<'static, TemplateProvider>);

/// Error type for the part code generator.
#[derive(Debug, thiserror::Error)]
pub enum CodegenError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error in {path}: {source}")]
    Toml {
        path: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("Invalid component file name: {0}")]
    InvalidFileName(String),
    #[error("Pin '{name}' (kind '{kind}') is missing required field '{field}'")]
    MissingField {
        name: String,
        kind: String,
        field: String,
    },
    #[error("Unknown pin kind '{kind}' for pin '{name}'")]
    UnknownKind { name: String, kind: String },
    #[error("Template error: {0}")]
    Mustache(#[from] mustache2::Error),
    #[error("Constraint '{ty}' is missing required field '{field}'")]
    MissingConstraintField { ty: String, field: String },
    #[error("Unknown constraint type '{ty}'")]
    UnknownConstraint { ty: String },
}

/// Component metadata from the TOML `[component]` table.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ComponentMeta {
    /// Rust struct name for the component (PascalCase).
    pub name: String,
    /// Short human-readable title used in module docs.
    pub title: String,
    /// Optional shorter description used for the struct doc comment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional URL pointing to the component datasheet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datasheet: Option<String>,
    /// Library identifier for the symbol/footprint within the source file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lib_id: Option<String>,
}

/// A thermal via that lives inside a pad (e.g. exposed thermal pad).
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ThermalViaDef {
    /// Position relative to the footprint origin, in millimetres.
    pub pos: (f64, f64),
    /// Drill diameter in millimetres.
    pub drill: f64,
    /// Finished pad diameter in millimetres.
    pub size: f64,
}

/// A single pin definition from the TOML `[[pin]]` table.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PinDef {
    /// Physical pin number (auto-assigned for non-numeric KiCad pin numbers).
    pub num: usize,
    /// Original KiCad pin number string (e.g. `"1"` or `"TD2+"`).
    /// Preserved so that non-numeric pin numbers can be matched across runs.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub number: String,
    /// Pin name as it appears on the schematic and in `PinRef` constants.
    pub name: String,
    /// Short purpose summary for the documentation table.
    pub purpose: String,
    /// Additional notes rendered in the documentation table.
    #[serde(default)]
    pub notes: String,
    /// Pin kind selecting the builder expression to emit.
    pub kind: String,
    /// Bandwidth in MHz for clock and SPI pins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bw_mhz: Option<f64>,
    /// Fixed voltage for `pwr_fixed` pins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v: Option<f64>,
    /// Minimum voltage for flexible power pins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v_min: Option<f64>,
    /// Maximum voltage for flexible power pins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v_max: Option<f64>,
    /// Current for `pwr_fixed` pins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub i: Option<f64>,
    /// Maximum current for flexible power pins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub i_max: Option<f64>,
    /// Physical position in millimetres, extracted from KiCad symbols/footprints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<(f64, f64)>,
    /// Pin rotation in degrees.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<f64>,
    /// Pin length in millimetres (largest dimension of the pad, for codegen).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<f64>,
    /// True if the pin is a no-connect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nc: Option<bool>,
    // ── pad geometry (from footprint) ──
    /// Pad width in millimetres (X dimension from KiCad `(size W H)`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    /// Pad height in millimetres (Y dimension from KiCad `(size W H)`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    /// KiCad pad type: `smd`, `thru_hole`, `connect`, or `np_thru_hole`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pad_type: Option<String>,
    /// Pad shape: `rect`, `roundrect`, `circle`, `oval`, `custom`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pad_shape: Option<String>,
    /// Roundrect corner radius ratio (only meaningful for `roundrect`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roundrect_rratio: Option<f64>,
    /// Solder mask margin in millimetres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solder_mask_margin: Option<f64>,
    /// Copper layers for this pad, e.g. `"F.Cu F.Mask F.Paste"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layers: Option<String>,
    /// Drill diameter in millimetres (thru_hole pads only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drill: Option<f64>,
    /// Thermal vias embedded within this pad.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub thermal_vias: Vec<ThermalViaDef>,
}

/// A constraint definition from the TOML `[[constraint]]` table.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct ConstraintDef {
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_pin: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temp: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skew_ps: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tol_pct: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires_plane: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_width: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clearance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voltage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

/// A mechanical pad — not an electrical pin — e.g. a mounting hole, fiducial,
/// or paste-only stencil aperture on an exposed pad.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MechanicalDef {
    /// KiCad pad number. `"None"` for mounting holes / fiducials, `""` for
    /// unnamed pads (e.g. paste stencil apertures).
    #[serde(default = "default_mech_number")]
    pub number: String,
    /// Position in millimetres.
    pub pos: (f64, f64),
    /// Pad width in millimetres (X dimension).
    pub width: f64,
    /// Pad height in millimetres (Y dimension).
    pub height: f64,
    /// KiCad pad type: `np_thru_hole`, `thru_hole`, `smd`.
    #[serde(default = "default_mech_pad_type")]
    pub pad_type: String,
    /// Pad shape: `circle`, `rect`, `oval`, `roundrect`.
    #[serde(default = "default_mech_shape")]
    pub pad_shape: String,
    /// Roundrect corner radius ratio (only for `roundrect` shape).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roundrect_rratio: Option<f64>,
    /// Copper layers, e.g. `"*.Cu *.Mask"` or `"F.Paste"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layers: Option<String>,
    /// Drill diameter in millimetres.
    #[serde(default)]
    pub drill: f64,
}

/// The complete TOML manifest for a component.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub component: ComponentMeta,
    #[serde(rename = "pin")]
    pub pins: Vec<PinDef>,
    #[serde(rename = "constraint", default)]
    pub constraints: Vec<ConstraintDef>,
    /// Mechanical-only pads (mounting holes, fiducials, etc.) that are not
    /// electrical pins.
    #[serde(rename = "mechanical", default, skip_serializing_if = "Vec::is_empty")]
    pub mechanical: Vec<MechanicalDef>,
}

#[derive(Serialize)]
struct PinRow {
    num: usize,
    name: String,
    purpose: String,
    notes: String,
    row: String,
}

#[derive(Serialize)]
struct ConstantRow {
    name: String,
    pin_name: String,
}

#[derive(Serialize)]
struct TemplateData {
    title: String,
    description: Option<String>,
    datasheet: Option<String>,
    struct_doc: String,
    struct_name: String,
    module_name: String,
    pins: Vec<PinRow>,
    constants: Vec<ConstantRow>,
    builders: Vec<String>,
    constraints: Vec<String>,
    mechanicals: Vec<String>,
    /// Library identifier emitted by `symbol()`/`footprint()`, if any.
    symbol_id: Option<String>,
    /// Datasheet URL as a Rust string literal (e.g. `"https://..."`).
    datasheet_lit: Option<String>,
    /// Description as a Rust string literal.
    description_lit: Option<String>,
}

impl SourceProvider for TemplateProvider {
    type Key = &'static str;

    fn get_src(&mut self, key: &Self::Key) -> Result<String, Cow<'static, str>> {
        if *key == TEMPLATE_KEY {
            Ok(self.source.clone())
        } else {
            Err("unknown template key".into())
        }
    }

    fn resolve_partial(&self, _key: &Self::Key, name: &str) -> Self::Key {
        name.to_string().leak()
    }

    fn display_key(key: &'_ Self::Key) -> Cow<'_, str> {
        Cow::Borrowed(*key)
    }
}

impl Serialize for CodegenError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

/// Generate a Rust source file from all `*.toml` files in `definitions_dir`.
pub fn generate(
    definitions_dir: impl AsRef<Path>,
    output_file: impl AsRef<Path>,
) -> Result<(), CodegenError> {
    let definitions_dir = definitions_dir.as_ref();
    let output_file = output_file.as_ref();

    let mut entries: Vec<PathBuf> = fs::read_dir(definitions_dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("toml"))
        .collect();
    entries.sort();

    let mut renderer = load_template(DEFAULT_TEMPLATE)?;
    let mut code = String::new();
    code.push_str("// Generated by copperleaf-parts-codegen. Do not edit by hand.\n\n");

    for path in entries {
        let component = render_component_file(&path, &mut renderer)?;
        code.push_str(&component);
        code.push('\n');
    }

    fs::write(output_file, code)?;
    Ok(())
}

/// Generate Rust code for a single component TOML file and return it as a
/// string. This is the entry point used by the `build_component!` proc macro.
pub fn generate_component_to_string(toml_path: impl AsRef<Path>) -> Result<String, CodegenError> {
    let mut renderer = load_template(DEFAULT_TEMPLATE)?;
    render_component_file(toml_path.as_ref(), &mut renderer)
}

/// Validates a manifest and returns diagnostics for any problems found.
pub fn validate(manifest: &Manifest) -> Vec<copperleaf::Diagnostic> {
    use copperleaf::{Diagnostic, Severity};
    let mut diags = Vec::new();
    let mut seen_names: HashSet<&str> = HashSet::new();

    for pin in &manifest.pins {
        // Duplicate pin name check.
        if !seen_names.insert(&pin.name) {
            diags.push(Diagnostic {
                code: "VALIDATE:DUPLICATE_PIN_NAME".into(),
                severity: Severity::Error,
                message: format!("Duplicate pin name '{}'", pin.name),
                entities: vec![pin.name.clone()],
                hint: Some("Pin names must be unique within a component".into()),
            });
        }

        // Required fields per kind.
        let missing = match pin.kind.as_str() {
            "clk" | "spi" => pin.bw_mhz.is_none().then_some("bw_mhz"),
            "pwr" => {
                if pin.v_min.is_none() {
                    Some("v_min")
                } else if pin.v_max.is_none() {
                    Some("v_max")
                } else if pin.i_max.is_none() {
                    Some("i_max")
                } else {
                    None
                }
            }
            "pwr_fixed" | "pwr_out" => {
                if pin.v.is_none() {
                    Some("v")
                } else if pin.i.is_none() {
                    Some("i")
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(field) = missing {
            diags.push(Diagnostic {
                code: "VALIDATE:MISSING_FIELD".into(),
                severity: Severity::Error,
                message: format!(
                    "Pin '{}' (kind '{}') is missing required field '{}'",
                    pin.name, pin.kind, field
                ),
                entities: vec![pin.name.clone()],
                hint: None,
            });
        }

        // Unresolved power pins.
        if pin.kind == "pwr" && (pin.v_min.is_none() || pin.v_max.is_none() || pin.i_max.is_none())
        {
            diags.push(Diagnostic {
                code: "VALIDATE:UNRESOLVED_POWER".into(),
                severity: Severity::Warning,
                message: format!("Power pin '{}' is missing voltage/current limits", pin.name),
                entities: vec![pin.name.clone()],
                hint: Some("Add v_min, v_max, and i_max".into()),
            });
        }

        // Pin-name-to-const sanity: names that start with a digit need the PIN_ prefix.
        if !pin.name.is_empty() {
            let first = pin.name.chars().next().unwrap();
            if first.is_ascii_digit() && !pin.name.starts_with("PIN_") {
                diags.push(Diagnostic {
                    code: "VALIDATE:NUMERIC_PIN_NAME".into(),
                    severity: Severity::Warning,
                    message: format!(
                        "Pin name '{}' starts with a digit and will be prefixed with PIN_",
                        pin.name
                    ),
                    entities: vec![pin.name.clone()],
                    hint: Some("Rename the pin or accept the PIN_ prefix".into()),
                });
            }
        }
    }

    diags
}

fn builder_expr(pin: &PinDef) -> Result<String, CodegenError> {
    let base = format!("Pin::build({:?})", pin.name);
    let suffix = physical_suffix(pin);
    let missing = |field: &str| {
        Err(CodegenError::MissingField {
            name: pin.name.clone(),
            kind: pin.kind.clone(),
            field: field.to_string(),
        })
    };
    match pin.kind.as_str() {
        "gnd" => Ok(format!("{}{}.gnd()", base, suffix)),
        "dio" => Ok(format!("{}{}.dio()", base, suffix)),
        "analog_in" => Ok(format!("{}{}.analog_in()", base, suffix)),
        "analog_rf" => Ok(format!(
            "{}{}.role(Role::AnalogIn).rf_limits().pin()",
            base, suffix
        )),
        "clk" => {
            let Some(bw) = pin.bw_mhz else {
                return missing("bw_mhz");
            };
            Ok(format!("{}{}.clk({})", base, suffix, fmt_f64(bw)))
        }
        "spi" => {
            let Some(bw) = pin.bw_mhz else {
                return missing("bw_mhz");
            };
            Ok(format!("{}{}.spi({})", base, suffix, fmt_f64(bw)))
        }
        "pwr" => {
            let Some(vmin) = pin.v_min else {
                return missing("v_min");
            };
            let Some(vmax) = pin.v_max else {
                return missing("v_max");
            };
            let Some(imax) = pin.i_max else {
                return missing("i_max");
            };
            Ok(format!(
                "{}{}.pwr({}.volt(), {}.volt(), {}.amp()).pin()",
                base,
                suffix,
                fmt_f64(vmin),
                fmt_f64(vmax),
                fmt_f64(imax)
            ))
        }
        "pwr_fixed" => {
            let Some(v) = pin.v else {
                return missing("v");
            };
            let Some(i) = pin.i else {
                return missing("i");
            };
            Ok(format!(
                "{}{}.pwr_fixed({}.volt(), {}.amp()).pin()",
                base,
                suffix,
                fmt_f64(v),
                fmt_f64(i)
            ))
        }
        "pwr_out" => {
            let Some(v) = pin.v else {
                return missing("v");
            };
            let Some(i) = pin.i else {
                return missing("i");
            };
            Ok(format!(
                "Pin::build({:?}){}.role(Role::PowerOut).power_spec(PowerSpec {{ v_min: {}.volt(), v_max: {}.volt(), v_nom: Some({}.volt()), i_max: {}.amp() }}).pin()",
                pin.name,
                suffix,
                fmt_f64(v),
                fmt_f64(v),
                fmt_f64(v),
                fmt_f64(i)
            ))
        }
        _ => Err(CodegenError::UnknownKind {
            name: pin.name.clone(),
            kind: pin.kind.clone(),
        }),
    }
}

fn const_name(pin_name: &str) -> String {
    let mut out = String::new();
    let mut first = true;
    for ch in pin_name.chars() {
        if first && ch.is_ascii_digit() {
            out.push_str("PIN_");
        }
        first = false;
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() || out.chars().all(|c| c == '_') {
        out.push_str("PIN");
    }
    out
}

fn constraint_expr(c: &ConstraintDef) -> Result<String, CodegenError> {
    let missing = |field: &str| {
        Err(CodegenError::MissingConstraintField {
            ty: c.ty.clone(),
            field: field.to_string(),
        })
    };
    match c.ty.as_str() {
        "Decoupling" => {
            let Some(values) = &c.values else {
                return missing("values");
            };
            let values_expr = values.join(", ");
            let per_pin = c.per_pin.unwrap_or(false);
            Ok(format!(
                "Constraint::Decoupling {{ values: vec![{}], per_pin: {} }}",
                values_expr, per_pin
            ))
        }
        "MaxJunction" => {
            let Some(temp) = &c.temp else {
                return missing("temp");
            };
            Ok(format!("Constraint::MaxJunction {{ temp: {} }}", temp))
        }
        "LengthMatch" => {
            let Some(group) = &c.group else {
                return missing("group");
            };
            let Some(skew_ps) = c.skew_ps else {
                return missing("skew_ps");
            };
            Ok(format!(
                "Constraint::LengthMatch {{ group: {:?}.into(), skew_ps: {} }}",
                group,
                fmt_f64(skew_ps)
            ))
        }
        _ => Err(CodegenError::UnknownConstraint { ty: c.ty.clone() }),
    }
}

fn default_mech_number() -> String {
    "None".into()
}

fn default_mech_pad_type() -> String {
    "np_thru_hole".into()
}

fn default_mech_shape() -> String {
    "circle".into()
}

fn fmt_f64(v: f64) -> String {
    format!("{:?}", v)
}

fn load_template(path: &str) -> Result<MustacheRenderer, CodegenError> {
    let source = fs::read_to_string(path)?;
    let cache: &'static SourceCache = Box::leak(Box::new(SourceCache::default()));
    let provider = TemplateProvider { source };
    let manager = RenderManager::new(provider, cache);
    Ok(MustacheRenderer(manager))
}

/// Build the Rust expression constructing a [`copperleaf::MechanicalPad`].
fn mechanical_expr(mech: &MechanicalDef) -> String {
    let rratio = match mech.roundrect_rratio {
        Some(rr) => format!("Some({})", fmt_f64(rr)),
        None => "None".to_string(),
    };
    let layers = match &mech.layers {
        Some(l) => format!("Some({:?}.into())", l),
        None => "None".to_string(),
    };
    format!(
        "copperleaf::MechanicalPad {{ number: {:?}.into(), pos: ({}, {}), width: {}, height: {}, pad_type: {:?}.into(), pad_shape: {:?}.into(), roundrect_rratio: {}, layers: {}, drill: {} }}",
        mech.number,
        fmt_f64(mech.pos.0),
        fmt_f64(mech.pos.1),
        fmt_f64(mech.width),
        fmt_f64(mech.height),
        mech.pad_type,
        mech.pad_shape,
        rratio,
        layers,
        fmt_f64(mech.drill),
    )
}

fn module_name(path: &Path) -> Result<String, CodegenError> {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| CodegenError::InvalidFileName(path.display().to_string()))?;
    if stem.is_empty() || !stem.chars().next().unwrap().is_ascii_alphabetic() {
        return Err(CodegenError::InvalidFileName(stem.to_string()));
    }
    if !stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(CodegenError::InvalidFileName(stem.to_string()));
    }
    Ok(stem.to_string())
}

fn physical_suffix(pin: &PinDef) -> String {
    let mut suffix = String::new();
    // Physical pad number: the original KiCad pin number when present,
    // otherwise the auto-assigned physical pin number.
    let number = if pin.number.is_empty() {
        pin.num.to_string()
    } else {
        pin.number.clone()
    };
    suffix.push_str(&format!(".number({:?})", number));
    if let Some((x, y)) = pin.pos {
        suffix.push_str(&format!(".pos({}, {})", fmt_f64(x), fmt_f64(y)));
    }
    if let Some(r) = pin.rotation {
        suffix.push_str(&format!(".rotation({})", fmt_f64(r)));
    }
    if let Some(l) = pin.length {
        suffix.push_str(&format!(".length({})", fmt_f64(l)));
    }
    if let Some(w) = pin.width {
        suffix.push_str(&format!(".width({})", fmt_f64(w)));
    }
    if let Some(h) = pin.height {
        suffix.push_str(&format!(".height({})", fmt_f64(h)));
    }
    if let Some(ref t) = pin.pad_type {
        suffix.push_str(&format!(".pad_type({:?})", t));
    }
    if let Some(ref s) = pin.pad_shape {
        suffix.push_str(&format!(".pad_shape({:?})", s));
    }
    if let Some(rr) = pin.roundrect_rratio {
        suffix.push_str(&format!(".roundrect_rratio({})", fmt_f64(rr)));
    }
    if let Some(smm) = pin.solder_mask_margin {
        suffix.push_str(&format!(".solder_mask_margin({})", fmt_f64(smm)));
    }
    if let Some(ref l) = pin.layers {
        suffix.push_str(&format!(".layers({:?})", l));
    }
    if let Some(d) = pin.drill {
        suffix.push_str(&format!(".drill({})", fmt_f64(d)));
    }
    for via in &pin.thermal_vias {
        suffix.push_str(&format!(
            ".thermal_via(({}, {}), {}, {})",
            fmt_f64(via.pos.0),
            fmt_f64(via.pos.1),
            fmt_f64(via.drill),
            fmt_f64(via.size)
        ));
    }
    suffix
}

fn render_component(
    module_name: &str,
    manifest: &Manifest,
    renderer: &mut MustacheRenderer,
) -> Result<String, CodegenError> {
    let struct_name = &manifest.component.name;
    let title = &manifest.component.title;

    let mut seen_names: HashSet<&str> = HashSet::new();
    let mut seen_consts: HashSet<String> = HashSet::new();
    let mut constants = Vec::new();
    let mut builders = Vec::new();
    let mut pin_rows = Vec::new();

    for pin in &manifest.pins {
        if seen_names.insert(&pin.name) {
            let base = const_name(&pin.name);
            let name = if seen_consts.insert(base.clone()) {
                base
            } else {
                // Disambiguate with pin number when names collide
                // (e.g. TD1+ and TD1- both map to TD1_).
                format!("{}_{}", base, pin.num)
            };
            constants.push(ConstantRow {
                name,
                pin_name: pin.name.clone(),
            });
        }
        builders.push(builder_expr(pin)?);
        pin_rows.push(PinRow {
            num: pin.num,
            name: pin.name.clone(),
            purpose: pin.purpose.clone(),
            notes: pin.notes.clone(),
            row: format!(
                "{:<3} | {:<8} | {:<11} | {:<21}",
                pin.num, pin.name, pin.purpose, pin.notes
            ),
        });
    }

    let constraints: Result<Vec<String>, CodegenError> =
        manifest.constraints.iter().map(constraint_expr).collect();

    let mechanicals: Vec<String> = manifest.mechanical.iter().map(mechanical_expr).collect();

    let data = TemplateData {
        title: title.clone(),
        description: manifest.component.description.clone(),
        datasheet: manifest.component.datasheet.clone(),
        struct_doc: manifest
            .component
            .description
            .as_deref()
            .unwrap_or(title)
            .to_string(),
        struct_name: struct_name.clone(),
        module_name: module_name.to_string(),
        pins: pin_rows,
        constants,
        builders,
        constraints: constraints?,
        mechanicals,
        symbol_id: manifest.component.lib_id.clone(),
        datasheet_lit: manifest
            .component
            .datasheet
            .as_ref()
            .map(|s| format!("{s:?}")),
        description_lit: manifest
            .component
            .description
            .as_ref()
            .map(|s| format!("{s:?}")),
    };

    let mut out = String::new();
    renderer.0.render_serde_to(&mut out, TEMPLATE_KEY, &data)?;
    Ok(out)
}

fn render_component_file(
    path: &Path,
    renderer: &mut MustacheRenderer,
) -> Result<String, CodegenError> {
    let module_name = module_name(path)?;
    let source = fs::read_to_string(path)?;
    let manifest: Manifest = toml::from_str(&source).map_err(|e| CodegenError::Toml {
        path: path.display().to_string(),
        source: e,
    })?;
    render_component(&module_name, &manifest, renderer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin_with_physical() -> PinDef {
        PinDef {
            num: 1,
            number: String::new(),
            name: "TXN".into(),
            purpose: "Test".into(),
            notes: String::new(),
            kind: "dio".into(),
            bw_mhz: None,
            v: None,
            v_min: None,
            v_max: None,
            i: None,
            i_max: None,
            pos: Some((101.6, 12.7)),
            rotation: Some(90.0),
            length: Some(2.54),
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
        }
    }

    #[test]
    fn pin_def_physical_round_trip() {
        let pin = pin_with_physical();
        let toml = toml::to_string(&pin).unwrap();
        let parsed: PinDef = toml::from_str(&toml).unwrap();
        assert_eq!(parsed.pos, Some((101.6, 12.7)));
        assert_eq!(parsed.rotation, Some(90.0));
        assert_eq!(parsed.length, Some(2.54));
    }

    #[test]
    fn builder_expr_emits_physical_fields() {
        let pin = pin_with_physical();
        let expr = builder_expr(&pin).unwrap();
        assert!(expr.contains(".pos(101.6, 12.7)"), "{}", expr);
        assert!(expr.contains(".rotation(90.0)"), "{}", expr);
        assert!(expr.contains(".length(2.54)"), "{}", expr);
    }

    #[test]
    fn validate_flags_unresolved_power() {
        let manifest = Manifest {
            component: ComponentMeta {
                name: "Test".into(),
                title: "Test".into(),
                description: None,
                datasheet: None,
                lib_id: None,
            },
            pins: vec![PinDef {
                num: 1,
                number: String::new(),
                name: "VDD".into(),
                purpose: "Supply".into(),
                notes: String::new(),
                kind: "pwr".into(),
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
        };
        let diags = validate(&manifest);
        assert!(diags.iter().any(|d| d.code == "VALIDATE:UNRESOLVED_POWER"));
    }

    #[test]
    fn validate_passes_complete_power() {
        let manifest = Manifest {
            component: ComponentMeta {
                name: "Test".into(),
                title: "Test".into(),
                description: None,
                datasheet: None,
                lib_id: None,
            },
            pins: vec![PinDef {
                num: 1,
                number: String::new(),
                name: "VDD".into(),
                purpose: "Supply".into(),
                notes: String::new(),
                kind: "pwr".into(),
                bw_mhz: None,
                v: None,
                v_min: Some(1.8),
                v_max: Some(3.3),
                i: None,
                i_max: Some(0.1),
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
        };
        let diags = validate(&manifest);
        assert!(!diags.iter().any(|d| d.code == "VALIDATE:UNRESOLVED_POWER"));
    }
}
