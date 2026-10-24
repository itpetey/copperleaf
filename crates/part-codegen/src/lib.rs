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
    #[error("Invalid Rust identifier for {field}: '{value}'")]
    InvalidIdentifier { field: String, value: String },
    #[error("manifest validation failed: {0}")]
    Validation(String),
}

/// Component metadata from the TOML `[component]` table.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
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
    /// Path to a 3D model file (`.step`) for this component's footprint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_3d: Option<String>,
    /// Base64-encoded content of the 3D model file for self-contained
    /// distribution.  When present, this is written to disk during footprint
    /// generation and `model_3d` is used as the reference path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_3d_data: Option<String>,
    /// 3D model rotation (degrees around X, Y, Z axes) to align the model
    /// with the footprint, e.g. `[0.0, 0.0, 90.0]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_3d_rotation: Option<(f64, f64, f64)>,
    /// 3D model offset in millimetres (X, Y, Z) to position the model
    /// relative to the footprint origin, e.g. `[0.0, 0.0, -2.0]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_3d_offset: Option<(f64, f64, f64)>,
    /// Explicit fabrication (body) outline extent as `(x1, y1, x2, y2)` in
    /// millimetres.  When set, this replaces the pad-derived extent for
    /// generating the F.Fab, F.SilkS, and F.CrtYd outlines.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fab_extent: Option<(f64, f64, f64, f64)>,
}

/// Shared electrical fields used by both `PinDef` and the kind-map system.
///
/// Flattened into `PinDef` via `#[serde(flatten)]` so the TOML schema remains
/// flat — no extra nesting.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ElectricalFields {
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
    /// True if the pin is a no-connect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nc: Option<bool>,
}

/// A thermal via that lives inside a pad (e.g. exposed thermal pad).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
    /// Electrical specification fields, flattened into the TOML table.
    #[serde(flatten)]
    pub electrical: ElectricalFields,
    /// Physical position in millimetres, extracted from KiCad symbols/footprints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<(f64, f64)>,
    /// Pin rotation in degrees.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<f64>,
    /// Pin length in millimetres (largest dimension of the pad, for codegen).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<f64>,
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

/// Closed unit enum for manifest quantities.
///
/// Only these units can be rendered, and each maps to one known `UnitExt`
/// constructor. Raw Rust expressions are never accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuantityUnit {
    Volt,
    Millivolt,
    Amp,
    Milliamp,
    Ohm,
    Kiloohm,
    Farad,
    Nanofarad,
    Microfarad,
    Picofarad,
    Henry,
    Hertz,
    Kilohertz,
    Megahertz,
    Meter,
    Millimetre,
    Second,
    Celsius,
}

/// A manifest quantity: a numeric value plus one closed unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quantity {
    pub value: f64,
    pub unit: QuantityUnit,
}

/// A constraint definition from the TOML `[[constraint]]` table.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct ConstraintDef {
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<Quantity>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_pin: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temp: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skew_ps: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tol_pct: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires_plane: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_width: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clearance: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voltage: Option<Quantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

/// Net class definition inside `[layout]`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NetClassDef {
    pub min_width: Quantity,
    pub clearance: Quantity,
}

/// Creepage definition inside `[layout]`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CreepageDef {
    pub min: Quantity,
    pub voltage: Quantity,
}

/// Layout-specific constraint definition from the TOML `[layout]` table.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub net_class: Option<NetClassDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creepage: Option<CreepageDef>,
}

/// A mechanical pad — not an electrical pin — e.g. a mounting hole, fiducial,
/// or paste-only stencil aperture on an exposed pad.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub component: ComponentMeta,
    #[serde(rename = "pin")]
    pub pins: Vec<PinDef>,
    #[serde(rename = "constraint", default)]
    pub constraints: Vec<ConstraintDef>,
    #[serde(default)]
    pub layout: LayoutDef,
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
    layout_constraints: Vec<String>,
    mechanicals: Vec<String>,
    /// Whether any metadata is present (controls the `has_meta` section in the template).
    has_meta: bool,
    /// Library identifier emitted by `symbol()`/`footprint()`, if any.
    symbol_id: Option<String>,
    /// Datasheet URL as a Rust string literal (e.g. `"https://..."`).
    datasheet_lit: Option<String>,
    /// 3D model path as a Rust string literal
    model3d_lit: Option<String>,
    /// Base64-encoded 3D model data as a Rust string literal
    model3d_data_lit: Option<String>,
    /// 3D model rotation as a Rust tuple expression, e.g. `(0.0, 0.0, 90.0)`.
    model3d_rotation_lit: Option<String>,
    /// 3D model offset as a Rust tuple expression, e.g. `(0.0, 0.0, -2.0)`.
    model3d_offset_lit: Option<String>,
    /// Description as a Rust string literal.
    description_lit: Option<String>,
    /// Fab extent as a Rust tuple expression, e.g. `(-3.65, -3.65, 3.65, 3.65)`.
    fab_extent_lit: Option<String>,
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

impl ComponentMeta {
    /// Convert to the core [`copperleaf::ComponentMeta`], mapping `lib_id` to
    /// both `symbol` and `footprint`.
    pub fn to_core_meta(&self) -> copperleaf::ComponentMeta {
        copperleaf::ComponentMeta {
            symbol: self.lib_id.clone(),
            footprint: self.lib_id.clone(),
            datasheet: self.datasheet.clone(),
            description: self.description.clone(),
            model_3d: self.model_3d.clone(),
            model_3d_data: self.model_3d_data.clone(),
            model_3d_rotation: self.model_3d_rotation.unwrap_or((0.0, 0.0, 0.0)),
            model_3d_offset: self.model_3d_offset.unwrap_or((0.0, 0.0, 0.0)),
            fab_extent: self.fab_extent,
            capacitance: None,
            is_bypass: false,
        }
    }
}

impl ElectricalFields {
    /// Merge `other`'s values into `self`, preserving any explicitly-set values.
    /// `kind` is always overwritten; other fields use `or` semantics.
    pub fn merge_from(&mut self, other: &Self) {
        self.kind = other.kind.clone();
        if let Some(bw) = other.bw_mhz {
            self.bw_mhz = self.bw_mhz.or(Some(bw));
        }
        if let Some(v) = other.v {
            self.v = self.v.or(Some(v));
        }
        if let Some(v_min) = other.v_min {
            self.v_min = self.v_min.or(Some(v_min));
        }
        if let Some(v_max) = other.v_max {
            self.v_max = self.v_max.or(Some(v_max));
        }
        if let Some(i) = other.i {
            self.i = self.i.or(Some(i));
        }
        if let Some(i_max) = other.i_max {
            self.i_max = self.i_max.or(Some(i_max));
        }
        if let Some(nc) = other.nc {
            self.nc = self.nc.or(Some(nc));
        }
    }
}

impl QuantityUnit {
    /// The `UnitExt` constructor rendered for this unit.
    pub fn constructor(self) -> &'static str {
        match self {
            Self::Volt => "volt",
            Self::Millivolt => "millivolt",
            Self::Amp => "amp",
            Self::Milliamp => "milliamp",
            Self::Ohm => "ohm",
            Self::Kiloohm => "kohm",
            Self::Farad => "farad",
            Self::Nanofarad => "nf",
            Self::Microfarad => "uf",
            Self::Picofarad => "pf",
            Self::Henry => "henry",
            Self::Hertz => "hz",
            Self::Kilohertz => "khz",
            Self::Megahertz => "mhz",
            Self::Meter => "meter",
            Self::Millimetre => "mm",
            Self::Second => "sec",
            Self::Celsius => "celsius",
        }
    }

    /// Canonical suffix used when serialising a quantity.
    pub fn suffix(self) -> &'static str {
        match self {
            Self::Volt => "V",
            Self::Millivolt => "mV",
            Self::Amp => "A",
            Self::Milliamp => "mA",
            Self::Ohm => "ohm",
            Self::Kiloohm => "kohm",
            Self::Farad => "F",
            Self::Nanofarad => "nF",
            Self::Microfarad => "uF",
            Self::Picofarad => "pF",
            Self::Henry => "H",
            Self::Hertz => "Hz",
            Self::Kilohertz => "kHz",
            Self::Megahertz => "MHz",
            Self::Meter => "m",
            Self::Millimetre => "mm",
            Self::Second => "s",
            Self::Celsius => "C",
        }
    }

    fn from_suffix(suffix: &str) -> Option<Self> {
        Some(match suffix {
            "V" | "v" => Self::Volt,
            "mV" | "mv" => Self::Millivolt,
            "A" | "a" => Self::Amp,
            "mA" | "ma" => Self::Milliamp,
            "ohm" | "Ω" => Self::Ohm,
            "kohm" | "kΩ" => Self::Kiloohm,
            "F" | "f" => Self::Farad,
            "nF" | "nf" => Self::Nanofarad,
            "uF" | "µF" | "uf" => Self::Microfarad,
            "pF" | "pf" => Self::Picofarad,
            "H" | "h" => Self::Henry,
            "Hz" | "hz" => Self::Hertz,
            "kHz" | "khz" => Self::Kilohertz,
            "MHz" | "mhz" => Self::Megahertz,
            "m" => Self::Meter,
            "mm" => Self::Millimetre,
            "s" | "sec" => Self::Second,
            "C" | "°C" | "celsius" => Self::Celsius,
            _ => return None,
        })
    }

    fn from_constructor(method: &str) -> Option<Self> {
        Some(match method {
            "volt" => Self::Volt,
            "millivolt" => Self::Millivolt,
            "amp" => Self::Amp,
            "milliamp" => Self::Milliamp,
            "ohm" => Self::Ohm,
            "kohm" => Self::Kiloohm,
            "farad" => Self::Farad,
            "nf" => Self::Nanofarad,
            "uf" => Self::Microfarad,
            "pf" => Self::Picofarad,
            "henry" => Self::Henry,
            "hz" => Self::Hertz,
            "khz" => Self::Kilohertz,
            "mhz" => Self::Megahertz,
            "meter" => Self::Meter,
            "mm" => Self::Millimetre,
            "sec" => Self::Second,
            "celsius" => Self::Celsius,
            _ => return None,
        })
    }
}

impl Quantity {
    /// Construct a quantity.
    pub const fn new(value: f64, unit: QuantityUnit) -> Self {
        Self { value, unit }
    }

    /// Render the quantity through its known `UnitExt` constructor.
    pub fn render(self) -> String {
        format!("{}.{}()", fmt_f64(self.value), self.unit.constructor())
    }

    /// Canonical suffix form, for serialisation.
    pub fn to_suffix_string(self) -> String {
        format!("{}{}", fmt_f64(self.value), self.unit.suffix())
    }

    /// Parse a suffix string or a legacy constructor expression.
    ///
    /// Accepted examples: `"100nF"`, `"0.2mm"`, `"3.3V"`, `"10kohm"`, and
    /// legacy manifest strings such as `"100.0.nf()"` or `"85.0.celsius()"`.
    pub fn parse(input: &str) -> Result<Self, String> {
        let input = input.trim();
        if input.is_empty() {
            return Err("quantity is empty".into());
        }
        if input.contains('(') || input.contains(')') {
            let Some(body) = input.strip_suffix("()") else {
                return Err(format!("unterminated quantity expression '{input}'"));
            };
            let Some((number, method)) = body.rsplit_once('.') else {
                return Err(format!("invalid quantity expression '{input}'"));
            };
            let value: f64 = number
                .trim()
                .parse::<f64>()
                .map_err(|_| format!("invalid numeric value in '{input}'"))?;
            check_finite(value, input)?;
            let method = method.trim();
            let unit = QuantityUnit::from_constructor(method)
                .ok_or_else(|| format!("unknown quantity constructor '{method}' in '{input}'"))?;
            return Ok(Self { value, unit });
        }

        let split = input
            .find(|ch: char| ch.is_alphabetic() || ch == '°' || ch == 'Ω' || ch == 'µ')
            .ok_or_else(|| format!("quantity '{input}' has no unit"))?;
        let number = input[..split].trim();
        let suffix = input[split..].trim();
        let value: f64 = number
            .parse::<f64>()
            .map_err(|_| format!("invalid numeric value in '{input}'"))?;
        check_finite(value, input)?;
        let unit = QuantityUnit::from_suffix(suffix)
            .ok_or_else(|| format!("unknown quantity unit '{suffix}' in '{input}'"))?;
        Ok(Self { value, unit })
    }
}

impl Serialize for Quantity {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_suffix_string())
    }
}

impl<'de> Deserialize<'de> for Quantity {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct QuantityVisitor;
        impl serde::de::Visitor<'_> for QuantityVisitor {
            type Value = Quantity;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a quantity such as \"100nF\" or \"0.2mm\"")
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Quantity, E> {
                Quantity::parse(value).map_err(E::custom)
            }
        }
        deserializer.deserialize_str(QuantityVisitor)
    }
}

pub fn fmt_f64(v: f64) -> String {
    format!("{:?}", v)
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

/// Returns the field names that must be populated for a pin of the given kind.
pub fn required_fields(kind: &str) -> &'static [&'static str] {
    match kind {
        "clk" | "spi" => &["bw_mhz"],
        "pwr" => &["v_min", "v_max", "i_max"],
        "pwr_fixed" | "pwr_out" => &["v", "i"],
        _ => &[],
    }
}

/// Validates a manifest and returns diagnostics for any problems found.
pub fn validate(manifest: &Manifest) -> Vec<copperleaf::Diagnostic> {
    use copperleaf::{Diagnostic, Severity};
    let mut diags = Vec::new();
    let mut seen_identities: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for pin in &manifest.pins {
        // Physical pad identities must be unique; display names may repeat.
        let identity = pin_identity(pin);
        if let Some(previous) = seen_identities.insert(identity.clone(), pin.num) {
            diags.push(Diagnostic {
                code: "VALIDATE:DUPLICATE_PAD".into(),
                severity: Severity::Error,
                message: format!(
                    "Pins {} and {} both resolve to physical identity '{}'",
                    previous, pin.num, identity
                ),
                entities: vec![pin.name.clone(), identity.clone()],
                hint: Some("Give every electrical pin a distinct pad number".into()),
            });
        }

        // Required fields per kind.
        let mut missing = None;
        for &field in required_fields(&pin.electrical.kind) {
            let ok = match field {
                "bw_mhz" => pin.electrical.bw_mhz.is_some(),
                "v_min" => pin.electrical.v_min.is_some(),
                "v_max" => pin.electrical.v_max.is_some(),
                "i_max" => pin.electrical.i_max.is_some(),
                "v" => pin.electrical.v.is_some(),
                "i" => pin.electrical.i.is_some(),
                _ => true,
            };
            if !ok {
                missing = Some(field);
                break;
            }
        }
        if let Some(field) = missing {
            diags.push(Diagnostic {
                code: "VALIDATE:MISSING_FIELD".into(),
                severity: Severity::Error,
                message: format!(
                    "Pin '{}' (kind '{}') is missing required field '{}'",
                    pin.name, pin.electrical.kind, field
                ),
                entities: vec![pin.name.clone()],
                hint: None,
            });
        }

        // Unresolved power pins.
        if pin.electrical.kind == "pwr"
            && (pin.electrical.v_min.is_none()
                || pin.electrical.v_max.is_none()
                || pin.electrical.i_max.is_none())
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

/// Validate a manifest and return `Err` when any error diagnostic is present.
pub fn validate_or_error(manifest: &Manifest) -> Result<(), CodegenError> {
    validate_errors(manifest, false)
}

/// Structural validation for draft manifests read or written by the CLI.
/// Missing electrical fields are allowed in a draft; duplicate physical
/// identities and unknown fields are not.
pub fn validate_structure_or_error(manifest: &Manifest) -> Result<(), CodegenError> {
    validate_errors(manifest, true)
}

fn builder_expr(pin: &PinDef) -> Result<String, CodegenError> {
    let base = format!("Pin::build({})", rust_string(&pin.name));
    let suffix = physical_suffix(pin);
    let missing = |field: &str| {
        Err(CodegenError::MissingField {
            name: pin.name.clone(),
            kind: pin.electrical.kind.clone(),
            field: field.to_string(),
        })
    };
    match pin.electrical.kind.as_str() {
        "gnd" => Ok(format!("{}{}.gnd()", base, suffix)),
        "dio" => Ok(format!("{}{}.dio()", base, suffix)),
        "analog_in" => Ok(format!("{}{}.analog_in()", base, suffix)),
        "analog_rf" => Ok(format!(
            "{}{}.role(Role::AnalogIn).rf_limits().pin()",
            base, suffix
        )),
        "clk" => {
            let Some(bw) = pin.electrical.bw_mhz else {
                return missing("bw_mhz");
            };
            Ok(format!("{}{}.clk({})", base, suffix, fmt_f64(bw)))
        }
        "spi" => {
            let Some(bw) = pin.electrical.bw_mhz else {
                return missing("bw_mhz");
            };
            Ok(format!("{}{}.spi({})", base, suffix, fmt_f64(bw)))
        }
        "pwr" => {
            let Some(vmin) = pin.electrical.v_min else {
                return missing("v_min");
            };
            let Some(vmax) = pin.electrical.v_max else {
                return missing("v_max");
            };
            let Some(imax) = pin.electrical.i_max else {
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
            let Some(v) = pin.electrical.v else {
                return missing("v");
            };
            let Some(i) = pin.electrical.i else {
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
            let Some(v) = pin.electrical.v else {
                return missing("v");
            };
            let Some(i) = pin.electrical.i else {
                return missing("i");
            };
            Ok(format!(
                "Pin::build({}){}.role(Role::PowerOut).power_spec(PowerSpec {{ v_min: {}.volt(), v_max: {}.volt(), v_nom: Some({}.volt()), i_max: {}.amp() }}).pin()",
                rust_string(&pin.name),
                suffix,
                fmt_f64(v),
                fmt_f64(v),
                fmt_f64(v),
                fmt_f64(i)
            ))
        }
        _ => Err(CodegenError::UnknownKind {
            name: pin.name.clone(),
            kind: pin.electrical.kind.clone(),
        }),
    }
}

fn check_finite(value: f64, input: &str) -> Result<(), String> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(format!("quantity '{input}' is not finite"))
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

/// Build the generated pin constants.
///
/// A unique display name keeps a readable constant (`IOVDD`). A repeated
/// display name gains a sanitised physical suffix (`AVDD_4`, `AVDD_8`) so
/// every physical pin remains independently addressable. If two sanitised
/// suffixes collide, a deterministic ordinal is appended.
fn constant_rows(pins: &[PinDef]) -> Result<Vec<ConstantRow>, CodegenError> {
    let mut name_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for pin in pins {
        *name_counts.entry(pin.name.as_str()).or_default() += 1;
    }

    let mut seen = HashSet::new();
    let mut rows = Vec::with_capacity(pins.len());
    for pin in pins {
        let identity = pin_identity(pin);
        let base = const_name(&pin.name);
        let repeated = name_counts.get(pin.name.as_str()).copied().unwrap_or(0) > 1;
        let mut name = if repeated {
            format!("{}_{}", base, identity_suffix(&identity))
        } else {
            base
        };
        if !seen.insert(name.clone()) {
            // A repeated name whose physical identities sanitise to the same
            // suffix (for example `A+` and `A-`) needs a deterministic ordinal.
            let mut ordinal = pin.num;
            loop {
                let candidate = format!("{}_{}", name, ordinal);
                if seen.insert(candidate.clone()) {
                    name = candidate;
                    break;
                }
                ordinal += 1;
            }
        }
        validate_ident("pin constant", &name)?;
        rows.push(ConstantRow {
            name,
            pin_name: rust_string(&identity),
        });
    }
    Ok(rows)
}

fn constraint_expr(c: &ConstraintDef) -> Result<String, CodegenError> {
    let missing = |field: &str| {
        Err(CodegenError::MissingConstraintField {
            ty: c.ty.clone(),
            field: field.to_string(),
        })
    };
    match c.ty.as_str() {
        "NetClass" => Err(CodegenError::UnknownConstraint {
            ty: "NetClass (moved to [layout] table — use `[layout] net_class = { min_width = \"…\", clearance = \"…\" }`)".into(),
        }),
        "Creepage" => Err(CodegenError::UnknownConstraint {
            ty: "Creepage (moved to [layout] table — use `[layout] creepage = { min = \"…\", voltage = \"…\" }`)".into(),
        }),
        "Decoupling" => {
            let Some(values) = &c.values else {
                return missing("values");
            };
            let values_expr = values
                .iter()
                .map(|quantity| quantity.render())
                .collect::<Vec<_>>()
                .join(", ");
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
            Ok(format!("Constraint::MaxJunction {{ temp: {} }}", temp.render()))
        }
        "LengthMatch" => {
            let Some(group) = &c.group else {
                return missing("group");
            };
            let Some(skew_ps) = c.skew_ps else {
                return missing("skew_ps");
            };
            Ok(format!(
                "Constraint::LengthMatch {{ group: {}.into(), skew_ps: {} }}",
                rust_string(group),
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

/// Strip characters that would break out of a `///` doc comment.
fn doc_text(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch == '\n' || ch == '\r' { ' ' } else { ch })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Turn a physical identity into a Rust-identifier-safe suffix.
fn identity_suffix(identity: &str) -> String {
    let mut out = String::new();
    for ch in identity.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "PIN".to_string()
    } else {
        trimmed.to_string()
    }
}

fn is_rust_keyword(value: &str) -> bool {
    matches!(
        value,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "try"
            | "union"
    )
}

/// Generate `LayoutConstraint::*` expressions from the `[layout]` table.
fn layout_constraint_exprs(layout: &LayoutDef) -> Vec<String> {
    let mut exprs = Vec::new();
    if let Some(ref nc) = layout.net_class {
        exprs.push(format!(
            "copperleaf::LayoutConstraint::NetClass {{ min_width: {}, clearance: {} }}",
            nc.min_width.render(),
            nc.clearance.render()
        ));
    }
    if let Some(ref cr) = layout.creepage {
        exprs.push(format!(
            "copperleaf::LayoutConstraint::Creepage {{ min: {}, voltage: {} }}",
            cr.min.render(),
            cr.voltage.render()
        ));
    }
    exprs
}

fn load_template(path: &str) -> Result<MustacheRenderer, CodegenError> {
    let source = fs::read_to_string(path)?;
    let cache: &'static SourceCache = Box::leak(Box::new(SourceCache::default()));
    let provider = TemplateProvider { source };
    let manager = RenderManager::new(provider, cache);
    Ok(MustacheRenderer(manager))
}

/// Build the Rust expression constructing a [`copperleaf::Pad`].
fn mechanical_expr(mech: &MechanicalDef) -> String {
    let rratio = match mech.roundrect_rratio {
        Some(rr) => format!("Some({})", fmt_f64(rr)),
        None => "None".to_string(),
    };
    let layers = match &mech.layers {
        Some(l) => format!("Some({}.into())", rust_string(l)),
        None => "None".to_string(),
    };
    let drill = if mech.drill > 0.0 {
        format!("Some({})", fmt_f64(mech.drill))
    } else {
        "None".to_string()
    };
    let pad_type = match mech.pad_type.as_str() {
        "smd" => "PadType::Smd",
        "thru_hole" => "PadType::ThruHole",
        "np_thru_hole" => "PadType::NpThruHole",
        "connect" => "PadType::Connect",
        other => panic!("unknown pad type: {other}"),
    };
    let pad_shape = match mech.pad_shape.as_str() {
        "rect" => "PadShape::Rect",
        "roundrect" => "PadShape::RoundRect",
        "circle" => "PadShape::Circle",
        "oval" => "PadShape::Oval",
        other => panic!("unknown pad shape: {other}"),
    };
    let number = if mech.number.eq_ignore_ascii_case("none") {
        "String::new()".to_string()
    } else {
        format!("{}.into()", rust_string(&mech.number))
    };
    format!(
        "copperleaf::Pad {{ number: {}, pos: ({}, {}), rotation: 0.0, width: {}, height: {}, pad_type: copperleaf::{}, pad_shape: copperleaf::{}, roundrect_rratio: {}, solder_mask_margin: None, layers: {}, drill: {} }}",
        number,
        fmt_f64(mech.pos.0),
        fmt_f64(mech.pos.1),
        fmt_f64(mech.width),
        fmt_f64(mech.height),
        pad_type,
        pad_shape,
        rratio,
        layers,
        drill,
    )
}

fn module_name(path: &Path) -> Result<String, CodegenError> {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| CodegenError::InvalidFileName(path.display().to_string()))?;
    if stem.is_empty() {
        return Err(CodegenError::InvalidFileName(stem.to_string()));
    }
    if !stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(CodegenError::InvalidFileName(stem.to_string()));
    }
    // Stems starting with a digit need a prefix to be valid Rust identifiers.
    let name = if !stem.chars().next().unwrap().is_ascii_alphabetic() {
        format!("comp_{}", stem)
    } else {
        stem.to_string()
    };
    validate_ident("module name", &name)?;
    Ok(name)
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
    suffix.push_str(&format!(".number({})", rust_string(&number)));
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
        suffix.push_str(&format!(".pad_type({})", rust_string(t)));
    }
    if let Some(ref s) = pin.pad_shape {
        suffix.push_str(&format!(".pad_shape({})", rust_string(s)));
    }
    if let Some(rr) = pin.roundrect_rratio {
        suffix.push_str(&format!(".roundrect_rratio({})", fmt_f64(rr)));
    }
    if let Some(smm) = pin.solder_mask_margin {
        suffix.push_str(&format!(".solder_mask_margin({})", fmt_f64(smm)));
    }
    if let Some(ref l) = pin.layers {
        suffix.push_str(&format!(".layers({})", rust_string(l)));
    }
    if let Some(d) = pin.drill {
        suffix.push_str(&format!(".drill({})", fmt_f64(d)));
    }
    if pin.electrical.nc == Some(true) {
        suffix.push_str(".nc(true)");
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

/// Canonical physical identity for a manifest pin: the explicit KiCad pin
/// number when present, otherwise its numeric ordinal.
fn pin_identity(pin: &PinDef) -> String {
    if pin.number.is_empty() {
        pin.num.to_string()
    } else {
        pin.number.clone()
    }
}

fn render_component(
    module_name: &str,
    manifest: &Manifest,
    renderer: &mut MustacheRenderer,
) -> Result<String, CodegenError> {
    let raw_name = &manifest.component.name;
    let struct_name = if raw_name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("Comp{}", raw_name)
    } else {
        raw_name.clone()
    };
    validate_ident("component name", &struct_name)?;
    let title = doc_text(&manifest.component.title);
    let description_doc = manifest.component.description.as_deref().map(doc_text);
    let struct_doc = manifest
        .component
        .description
        .as_deref()
        .map(doc_text)
        .unwrap_or_else(|| title.clone());
    let datasheet_doc = manifest.component.datasheet.as_deref().map(doc_text);

    let constants = constant_rows(&manifest.pins)?;
    let mut builders = Vec::new();
    let mut pin_rows = Vec::new();

    for pin in &manifest.pins {
        builders.push(builder_expr(pin)?);
        let name = doc_text(&pin.name);
        let purpose = doc_text(&pin.purpose);
        let notes = doc_text(&pin.notes);
        pin_rows.push(PinRow {
            num: pin.num,
            name: name.clone(),
            purpose: purpose.clone(),
            notes: notes.clone(),
            row: format!(
                "{:<3} | {:<8} | {:<11} | {:<21}",
                pin.num, name, purpose, notes
            ),
        });
    }

    let constraints: Result<Vec<String>, CodegenError> =
        manifest.constraints.iter().map(constraint_expr).collect();

    let layout_constraints = layout_constraint_exprs(&manifest.layout);

    let mechanicals: Vec<String> = manifest.mechanical.iter().map(mechanical_expr).collect();

    let has_meta = manifest.component.lib_id.is_some()
        || manifest.component.datasheet.is_some()
        || manifest.component.description.is_some()
        || manifest.component.model_3d.is_some()
        || manifest.component.model_3d_data.is_some()
        || manifest.component.model_3d_rotation.is_some()
        || manifest.component.model_3d_offset.is_some()
        || manifest.component.fab_extent.is_some();

    let data = TemplateData {
        title: title.clone(),
        description: description_doc.clone(),
        datasheet: datasheet_doc.clone(),
        struct_doc,
        struct_name: struct_name.clone(),
        module_name: module_name.to_string(),
        pins: pin_rows,
        constants,
        builders,
        constraints: constraints?,
        layout_constraints,
        mechanicals,
        has_meta,
        symbol_id: manifest.component.lib_id.as_deref().map(rust_string),
        datasheet_lit: manifest.component.datasheet.as_deref().map(rust_string),
        model3d_lit: manifest.component.model_3d.as_deref().map(rust_string),
        model3d_data_lit: manifest
            .component
            .model_3d_data
            .as_deref()
            .map(str::trim)
            .filter(|data| !data.is_empty())
            .map(rust_string),
        model3d_rotation_lit: manifest
            .component
            .model_3d_rotation
            .map(|(x, y, z)| format!("({x:?}, {y:?}, {z:?})")),
        model3d_offset_lit: manifest
            .component
            .model_3d_offset
            .map(|(x, y, z)| format!("({x:?}, {y:?}, {z:?})")),
        description_lit: manifest.component.description.as_deref().map(rust_string),
        fab_extent_lit: manifest
            .component
            .fab_extent
            .map(|(x1, y1, x2, y2)| format!("({x1:?}, {y1:?}, {x2:?}, {y2:?})")),
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
    validate_or_error(&manifest)?;
    render_component(&module_name, &manifest, renderer)
}

/// Render a Rust string literal through `proc-macro2`, which escapes quotes,
/// backslashes, control characters, and newlines correctly.
fn rust_string(value: &str) -> String {
    proc_macro2::Literal::string(value).to_string()
}

fn validate_errors(manifest: &Manifest, structural_only: bool) -> Result<(), CodegenError> {
    let errors: Vec<String> = validate(manifest)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == copperleaf::Severity::Error)
        .filter(|diagnostic| !structural_only || diagnostic.code == "VALIDATE:DUPLICATE_PAD")
        .map(|diagnostic| {
            if diagnostic.entities.is_empty() {
                format!("{}: {}", diagnostic.code, diagnostic.message)
            } else {
                format!(
                    "{}: {} ({})",
                    diagnostic.code,
                    diagnostic.message,
                    diagnostic.entities.join(", ")
                )
            }
        })
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CodegenError::Validation(errors.join("\n")))
    }
}

/// Validate a generated Rust identifier before it reaches the template.
fn validate_ident(field: &str, value: &str) -> Result<(), CodegenError> {
    let mut chars = value.chars();
    let valid = chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        && !is_rust_keyword(value);
    if valid {
        Ok(())
    } else {
        Err(CodegenError::InvalidIdentifier {
            field: field.to_string(),
            value: value.to_string(),
        })
    }
}

#[cfg(test)]
mod quantity_tests {
    use super::*;

    fn q(value: f64, unit: QuantityUnit) -> Quantity {
        Quantity::new(value, unit)
    }

    #[test]
    fn parses_suffix_quantities() {
        assert_eq!(
            Quantity::parse("100nF").unwrap(),
            q(100.0, QuantityUnit::Nanofarad)
        );
        assert_eq!(
            Quantity::parse("0.2mm").unwrap(),
            q(0.2, QuantityUnit::Millimetre)
        );
        assert_eq!(Quantity::parse("3.3V").unwrap(), q(3.3, QuantityUnit::Volt));
        assert_eq!(
            Quantity::parse("10kohm").unwrap(),
            q(10.0, QuantityUnit::Kiloohm)
        );
        assert_eq!(
            Quantity::parse("85C").unwrap(),
            q(85.0, QuantityUnit::Celsius)
        );
    }

    #[test]
    fn parses_legacy_constructor_expressions() {
        assert_eq!(
            Quantity::parse("100.0.nf()").unwrap(),
            q(100.0, QuantityUnit::Nanofarad)
        );
        assert_eq!(
            Quantity::parse("85.0.celsius()").unwrap(),
            q(85.0, QuantityUnit::Celsius)
        );
        assert_eq!(
            Quantity::parse("0.2.mm()").unwrap(),
            q(0.2, QuantityUnit::Millimetre)
        );
    }

    #[test]
    fn rejects_expressions_and_unknown_units() {
        for invalid in [
            "std::process::exit(1)",
            "100furlong",
            "1.0.unknown()",
            "",
            "not-a-number mm",
            "NaNmm",
            "infV",
        ] {
            assert!(
                Quantity::parse(invalid).is_err(),
                "{invalid} should be rejected"
            );
        }
    }

    #[test]
    fn renders_only_known_constructors() {
        assert_eq!(q(100.0, QuantityUnit::Nanofarad).render(), "100.0.nf()");
        assert_eq!(q(0.2, QuantityUnit::Millimetre).render(), "0.2.mm()");
        assert_eq!(q(3.3, QuantityUnit::Volt).render(), "3.3.volt()");
        assert_eq!(q(85.0, QuantityUnit::Celsius).render(), "85.0.celsius()");
        assert_eq!(q(10.0, QuantityUnit::Kiloohm).render(), "10.0.kohm()");
    }

    #[test]
    fn serialises_to_suffix_and_round_trips() {
        #[derive(Serialize, Deserialize)]
        struct Wrapper {
            quantity: Quantity,
        }
        for quantity in [
            q(100.0, QuantityUnit::Nanofarad),
            q(0.2, QuantityUnit::Millimetre),
            q(3.3, QuantityUnit::Volt),
        ] {
            let text = toml::to_string(&Wrapper { quantity }).unwrap();
            let parsed: Wrapper = toml::from_str(&text).unwrap();
            assert_eq!(parsed.quantity, quantity);
        }
    }

    #[test]
    fn invalid_quantity_expression_is_rejected_by_deserialisation() {
        #[derive(Deserialize)]
        struct Wrapper {
            #[allow(dead_code)]
            quantity: Quantity,
        }
        let result = toml::from_str::<Wrapper>("quantity = \"std::process::exit(1)\"");
        assert!(result.is_err(), "raw Rust expressions must not deserialise");
        assert!(Quantity::parse("std::process::exit(1)").is_err());
    }

    #[test]
    fn every_part_manifest_uses_typed_quantities() {
        let parts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parts");
        let mut count = 0;
        for vendor in std::fs::read_dir(&parts).unwrap() {
            let vendor = vendor.unwrap();
            let Ok(entries) = std::fs::read_dir(vendor.path()) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("toml")
                    || path.file_name().and_then(|s| s.to_str()) == Some("Cargo.toml")
                {
                    continue;
                }
                let text = std::fs::read_to_string(&path).unwrap();
                let manifest: Manifest = toml::from_str(&text)
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
                // A full re-serialisation must also succeed with typed values.
                toml::to_string(&manifest)
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
                count += 1;
            }
        }
        assert!(count >= 27, "expected every part manifest, found {count}");
    }
}

#[cfg(test)]
mod security_tests {
    use super::*;

    fn malicious_manifest() -> Manifest {
        Manifest {
            component: ComponentMeta {
                name: "Evil".into(),
                title: "evil\npub fn injected() {}".into(),
                description: Some("\"; std::process::exit(1); //".into()),
                datasheet: Some("https://example.test/\n*/ fn injected() {}".into()),
                lib_id: Some("Evil\"\n".into()),
                model_3d: Some("\"; //\n".into()),
                model_3d_data: None,
                model_3d_rotation: None,
                model_3d_offset: None,
                fab_extent: None,
            },
            pins: vec![PinDef {
                num: 1,
                number: "1".into(),
                name: "A\"; std::process::exit(1); //".into(),
                purpose: "purpose\"; //".into(),
                notes: "notes\n fn injected() {}".into(),
                electrical: ElectricalFields {
                    kind: "dio".into(),
                    ..Default::default()
                },
                pos: None,
                rotation: None,
                length: None,
                width: None,
                height: None,
                pad_type: Some("smd\"; //".into()),
                pad_shape: None,
                roundrect_rratio: None,
                solder_mask_margin: None,
                layers: Some("F.Cu\"; //".into()),
                drill: None,
                thermal_vias: vec![],
            }],
            constraints: vec![],
            layout: Default::default(),
            mechanical: vec![],
        }
    }

    #[test]
    fn malicious_strings_remain_escaped_literals() {
        let mut renderer = load_template(DEFAULT_TEMPLATE).unwrap();
        let source = render_component("evil", &malicious_manifest(), &mut renderer).unwrap();

        // The generated file must still be syntactically valid Rust.
        syn::parse_file(&source).expect("generated source must parse");

        // Injection attempts stay inside string literals or doc text.
        assert!(source.contains("\\\""), "quotes must be escaped: {source}");
        assert!(!source.contains("\n        fn injected()"), "{source}");
        // Doc text is flattened onto a single comment line, so injected text
        // cannot start a real item.
        assert!(
            !source
                .lines()
                .any(|line| line.trim_start().starts_with("pub fn injected")),
            "{source}"
        );

        // The pin constant itself is sanitised to a valid identifier.
        let constant = source
            .lines()
            .find(|line| line.contains("PinRef"))
            .expect("pin constant");
        let name = constant
            .trim()
            .strip_prefix("pub const ")
            .and_then(|rest| rest.split(':').next())
            .expect("constant name");
        validate_ident("pin constant", name).unwrap();
    }

    #[test]
    fn invalid_component_identifier_is_rejected() {
        let mut manifest = malicious_manifest();
        manifest.component.name = "Bad-Name".into();
        let mut renderer = load_template(DEFAULT_TEMPLATE).unwrap();
        let error = render_component("bad", &manifest, &mut renderer).unwrap_err();
        assert!(matches!(error, CodegenError::InvalidIdentifier { .. }));
    }

    #[test]
    fn invalid_fields_are_rejected_by_identifier_validation() {
        for value in ["", "1bad", "bad-name", "struct", "has space", "emoji😀"] {
            assert!(
                validate_ident("field", value).is_err(),
                "{value} must be rejected"
            );
        }
        for value in ["OK", "_ok", "OK_1"] {
            validate_ident("field", value).unwrap();
        }
    }
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
            electrical: ElectricalFields {
                kind: "dio".into(),
                ..Default::default()
            },
            pos: Some((101.6, 12.7)),
            rotation: Some(90.0),
            length: Some(2.54),
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

    fn pin_def(num: usize, number: &str, name: &str) -> PinDef {
        PinDef {
            num,
            number: number.into(),
            name: name.into(),
            purpose: "Test".into(),
            notes: String::new(),
            electrical: ElectricalFields {
                kind: "dio".into(),
                ..Default::default()
            },
            pos: None,
            rotation: None,
            length: None,
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
    fn unique_names_keep_readable_constants() {
        let rows = constant_rows(&[pin_def(12, "12", "IOVDD")]).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "IOVDD");
        assert_eq!(rows[0].pin_name, "\"12\"");
    }

    #[test]
    fn repeated_names_gain_physical_suffixes() {
        let rows = constant_rows(&[
            pin_def(1, "4", "AVDD"),
            pin_def(2, "8", "AVDD"),
            pin_def(3, "11", "AVDD"),
        ])
        .unwrap();
        let names: Vec<_> = rows.iter().map(|row| row.name.as_str()).collect();
        let identities: Vec<_> = rows.iter().map(|row| row.pin_name.as_str()).collect();
        assert_eq!(names, ["AVDD_4", "AVDD_8", "AVDD_11"]);
        assert_eq!(identities, ["\"4\"", "\"8\"", "\"11\""]);
    }

    #[test]
    fn non_numeric_pad_numbers_are_sanitised() {
        let rows =
            constant_rows(&[pin_def(1, "TD1+", "SHIELD"), pin_def(2, "TD2+", "SHIELD")]).unwrap();
        assert_eq!(rows[0].name, "SHIELD_TD1");
        assert_eq!(rows[1].name, "SHIELD_TD2");
        assert_eq!(rows[0].pin_name, "\"TD1+\"");
        assert_eq!(rows[1].pin_name, "\"TD2+\"");
    }

    #[test]
    fn suffix_collisions_append_an_ordinal() {
        let rows =
            constant_rows(&[pin_def(1, "A+", "SHIELD"), pin_def(2, "A-", "SHIELD")]).unwrap();
        assert_eq!(rows[0].name, "SHIELD_A");
        assert_eq!(rows[1].name, "SHIELD_A_2");
        assert!(rows.iter().all(|row| {
            row.name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        }));
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
                model_3d: None,
                model_3d_data: None,
                model_3d_rotation: None,
                model_3d_offset: None,
                fab_extent: None,
            },
            pins: vec![PinDef {
                num: 1,
                number: String::new(),
                name: "VDD".into(),
                purpose: "Supply".into(),
                notes: String::new(),
                electrical: ElectricalFields {
                    kind: "pwr".into(),
                    ..Default::default()
                },
                pos: None,
                rotation: None,
                length: None,
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
            layout: Default::default(),
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
                model_3d: None,
                model_3d_data: None,
                model_3d_rotation: None,
                model_3d_offset: None,
                fab_extent: None,
            },
            pins: vec![PinDef {
                num: 1,
                number: String::new(),
                name: "VDD".into(),
                purpose: "Supply".into(),
                notes: String::new(),
                electrical: ElectricalFields {
                    kind: "pwr".into(),
                    v_min: Some(1.8),
                    v_max: Some(3.3),
                    i_max: Some(0.1),
                    ..Default::default()
                },
                pos: None,
                rotation: None,
                length: None,
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
            layout: Default::default(),
            mechanical: vec![],
        };
        let diags = validate(&manifest);
        assert!(!diags.iter().any(|d| d.code == "VALIDATE:UNRESOLVED_POWER"));
    }
}

#[cfg(test)]
mod validation_entry_tests {
    use super::*;

    fn minimal_manifest() -> Manifest {
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
                fab_extent: None,
            },
            pins: vec![PinDef {
                num: 1,
                number: "1".into(),
                name: "IO".into(),
                purpose: "IO".into(),
                notes: String::new(),
                electrical: ElectricalFields {
                    kind: "dio".into(),
                    ..Default::default()
                },
                pos: None,
                rotation: None,
                length: None,
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
            layout: Default::default(),
            mechanical: vec![],
        }
    }

    #[test]
    fn unknown_fields_are_rejected_by_name() {
        let cases = [
            (
                "[component]\nname = \"Test\"\ntitle = \"Test\"\nbogus_field = 1\n",
                "bogus_field",
            ),
            (
                "[component]\nname = \"Test\"\ntitle = \"Test\"\n[[pin]]\nnum = 1\nname = \"IO\"\npurpose = \"IO\"\nkind = \"dio\"\nmystery = true\n",
                "mystery",
            ),
        ];
        for (toml, field) in cases {
            let error = toml::from_str::<Manifest>(toml).unwrap_err().to_string();
            assert!(error.contains("unknown field"), "{error}");
            assert!(error.contains(field), "{error}");
        }
    }

    #[test]
    fn duplicate_pad_numbers_are_rejected_but_repeated_names_are_allowed() {
        let manifest = minimal_manifest();
        validate_or_error(&manifest).unwrap();

        let mut repeated = minimal_manifest();
        repeated.pins.push(PinDef {
            num: 2,
            number: "2".into(),
            name: "IO".into(),
            ..repeated.pins[0].clone()
        });
        validate_or_error(&repeated).expect("repeated display names are valid");

        let mut duplicate = minimal_manifest();
        duplicate.pins.push(PinDef {
            num: 2,
            number: "1".into(),
            name: "OTHER".into(),
            ..duplicate.pins[0].clone()
        });
        let error = validate_or_error(&duplicate).unwrap_err().to_string();
        assert!(error.contains("VALIDATE:DUPLICATE_PAD"), "{error}");
        assert!(error.contains('1'), "{error}");
    }

    #[test]
    fn codegen_rejects_invalid_manifest_without_writing_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.toml");
        std::fs::write(
            &path,
            "[component]\nname = \"Bad-Name\"\ntitle = \"Bad\"\n[[pin]]\nnum = 1\nname = \"IO\"\npurpose = \"IO\"\nkind = \"dio\"\n",
        )
        .unwrap();
        let before = std::fs::read_dir(dir.path()).unwrap().count();
        let error = generate_component_to_string(&path).unwrap_err();
        assert!(
            matches!(error, CodegenError::InvalidIdentifier { .. }),
            "{error}"
        );
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), before);
    }
}
