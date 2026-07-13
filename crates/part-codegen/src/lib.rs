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
//!
//! [[pin]]
//! num = 1
//! name = "GND_1"
//! purpose = "Ground"
//! notes = ""                  # Optional, rendered in the pinout table
//! kind = "gnd"                # Selects the PinBuilder expression
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

use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

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
    Mustache(#[from] mustache::Error),
}

const DEFAULT_TEMPLATE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/component.mustache");

#[derive(Debug, Deserialize)]
struct Manifest {
    component: ComponentMeta,
    #[serde(rename = "pin")]
    pins: Vec<PinDef>,
}

#[derive(Debug, Deserialize)]
struct ComponentMeta {
    /// Rust struct name for the component (PascalCase).
    name: String,
    /// Short human-readable title used in module docs.
    title: String,
    /// Optional shorter description used for the struct doc comment.
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PinDef {
    /// Physical pin number.
    num: usize,
    /// Pin name as it appears on the schematic and in `PinRef` constants.
    name: String,
    /// Short purpose summary for the documentation table.
    purpose: String,
    /// Additional notes rendered in the documentation table.
    #[serde(default)]
    notes: String,
    /// Pin kind selecting the builder expression to emit.
    kind: String,
    /// Bandwidth in MHz for clock and SPI pins.
    #[serde(default)]
    bw_mhz: Option<f64>,
    /// Fixed voltage for `pwr_fixed` pins.
    #[serde(default)]
    v: Option<f64>,
    /// Minimum voltage for flexible power pins.
    #[serde(default)]
    v_min: Option<f64>,
    /// Maximum voltage for flexible power pins.
    #[serde(default)]
    v_max: Option<f64>,
    /// Current for `pwr_fixed` pins.
    #[serde(default)]
    i: Option<f64>,
    /// Maximum current for flexible power pins.
    #[serde(default)]
    i_max: Option<f64>,
}

#[derive(Serialize)]
struct TemplateData {
    title: String,
    description: Option<String>,
    struct_doc: String,
    struct_name: String,
    module_name: String,
    pins: Vec<PinRow>,
    constants: Vec<ConstantRow>,
    builders: Vec<String>,
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

    let template = load_template(DEFAULT_TEMPLATE)?;
    let mut code = String::new();
    code.push_str("// Generated by copperleaf-parts-codegen. Do not edit by hand.\n\n");

    for path in entries {
        let component = render_component_file(&path, &template)?;
        code.push_str(&component);
        code.push('\n');
    }

    fs::write(output_file, code)?;
    Ok(())
}

/// Generate Rust code for a single component TOML file and return it as a
/// string. This is the entry point used by the `build_component!` proc macro.
pub fn generate_component_to_string(toml_path: impl AsRef<Path>) -> Result<String, CodegenError> {
    let template = load_template(DEFAULT_TEMPLATE)?;
    render_component_file(toml_path.as_ref(), &template)
}

fn load_template(path: &str) -> Result<mustache::Template, CodegenError> {
    let source = fs::read_to_string(path)?;
    Ok(mustache::compile_str(&source)?)
}

fn render_component_file(
    path: &Path,
    template: &mustache::Template,
) -> Result<String, CodegenError> {
    let module_name = module_name(path)?;
    let source = fs::read_to_string(path)?;
    let manifest: Manifest = toml::from_str(&source).map_err(|e| CodegenError::Toml {
        path: path.display().to_string(),
        source: e,
    })?;
    render_component(&module_name, &manifest, template)
}

fn render_component(
    module_name: &str,
    manifest: &Manifest,
    template: &mustache::Template,
) -> Result<String, CodegenError> {
    let struct_name = &manifest.component.name;
    let title = &manifest.component.title;

    let mut seen: HashSet<&str> = HashSet::new();
    let mut constants = Vec::new();
    let mut builders = Vec::new();
    let mut pin_rows = Vec::new();

    for pin in &manifest.pins {
        if seen.insert(&pin.name) {
            constants.push(ConstantRow {
                name: const_name(&pin.name),
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

    let data = TemplateData {
        title: title.clone(),
        description: manifest.component.description.clone(),
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
    };

    let mut rendered = Vec::new();
    template.render(&mut rendered, &data)?;
    Ok(String::from_utf8(rendered).expect("template produced invalid UTF-8"))
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
    if out.is_empty() {
        out.push_str("PIN");
    }
    out
}

fn builder_expr(pin: &PinDef) -> Result<String, CodegenError> {
    let base = format!("Pin::build({:?})", pin.name);
    let missing = |field: &str| {
        Err(CodegenError::MissingField {
            name: pin.name.clone(),
            kind: pin.kind.clone(),
            field: field.to_string(),
        })
    };
    match pin.kind.as_str() {
        "gnd" => Ok(format!("{}.gnd()", base)),
        "dio" => Ok(format!("{}.dio()", base)),
        "analog_in" => Ok(format!("{}.analog_in()", base)),
        "analog_rf" => Ok(format!("{}.role(Role::AnalogIn).rf_limits().pin()", base)),
        "clk" => {
            let Some(bw) = pin.bw_mhz else {
                return missing("bw_mhz");
            };
            Ok(format!("{}.clk({})", base, fmt_f64(bw)))
        }
        "spi" => {
            let Some(bw) = pin.bw_mhz else {
                return missing("bw_mhz");
            };
            Ok(format!("{}.spi({})", base, fmt_f64(bw)))
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
                "{}.pwr({}.volt(), {}.volt(), {}.amp()).pin()",
                base,
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
                "{}.pwr_fixed({}.volt(), {}.amp()).pin()",
                base,
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

fn fmt_f64(v: f64) -> String {
    format!("{:?}", v)
}
