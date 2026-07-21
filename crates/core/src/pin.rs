use serde::{Deserialize, Serialize};

use crate::units::{Amp, Hertz, Ohm, Qty, Second, UnitExt, Volt};

/// Default drill for through-hole pads, in mm (0.3 in).
pub const DEFAULT_DRILL: f64 = 0.762;
/// Default pad size when no geometry is present, in mm.
pub const DEFAULT_PAD_SIZE: f64 = 1.524;
/// Default layers for through-hole pads.
pub const PTH_LAYERS: &str = "*.Cu *.Mask";
/// Default layers for SMD pads.
pub const SMD_LAYERS: &str = "F.Cu F.Mask F.Paste";

/// Footprint pad type as defined by KiCad.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PadType {
    /// Surface-mount pad.
    #[serde(rename = "smd")]
    Smd,
    /// Plated through-hole pad.
    #[serde(rename = "thru_hole")]
    ThruHole,
    /// Non-plated through-hole pad.
    #[serde(rename = "np_thru_hole")]
    NpThruHole,
    /// Connector pin pad.
    #[serde(rename = "connect")]
    Connect,
}

/// Footprint pad shape as defined by KiCad.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PadShape {
    /// Rectangular pad.
    #[serde(rename = "rect")]
    Rect,
    /// Rounded-rectangle pad (corner radius ratio stored separately).
    #[serde(rename = "roundrect")]
    RoundRect,
    /// Circular pad.
    #[serde(rename = "circle")]
    Circle,
    /// Oval pad.
    #[serde(rename = "oval")]
    Oval,
}

/// Footprint pad geometry, used for both electrical and mechanical pads.
///
/// This is the single pad-geometry representation for the entire workspace.
/// It replaces `MechanicalPad` (core), `PadGeom` (backend), and the ad-hoc
/// pad fields on `PinDef`/`MechanicalDef` (codegen).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pad {
    /// KiCad pad number (e.g. `"1"`, `"A1"`).  Empty string for un-numbered pads.
    #[serde(default)]
    pub number: String,
    /// Position in millimetres relative to the footprint origin.
    pub pos: (f64, f64),
    /// Rotation in degrees.
    #[serde(default)]
    pub rotation: f64,
    /// Pad width in millimetres (X dimension from KiCad `(size W H)`).
    pub width: f64,
    /// Pad height in millimetres (Y dimension from KiCad `(size W H)`).
    pub height: f64,
    /// KiCad pad type.
    pub pad_type: PadType,
    /// Pad shape.
    pub pad_shape: PadShape,
    /// Roundrect corner radius ratio (only meaningful for `RoundRect` shape).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roundrect_rratio: Option<f64>,
    /// Solder mask margin in millimetres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solder_mask_margin: Option<f64>,
    /// Copper layers for this pad, e.g. `"F.Cu F.Mask F.Paste"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layers: Option<String>,
    /// Drill diameter in millimetres (through-hole pads only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drill: Option<f64>,
}

/// Schematic symbol pin graphics.
///
/// Separated from [`Pad`] so that symbol geometry does not leak into
/// footprint pad-dimension calculations.  All values are in millimetres.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SymPin {
    /// Position in the symbol coordinate system (millimetres).
    pub pos: (f64, f64),
    /// Rotation in degrees.
    #[serde(default)]
    pub rotation: f64,
    /// Pin stub length in millimetres (schematic only, not a pad dimension).
    pub length: f64,
}

/// Identifier for a specific pin on a component.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PinId(pub String);

/// Electrical role of a pin used to infer ERC rules and routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    PowerIn,
    PowerOut,
    AnalogIn,
    AnalogOut,
    DigitalIO,
    DiffPos,
    DiffNeg,
    Gnd,
    /// Non-electrical pad (thermal via, paste aperture, mounting hole).
    Passive,
}

/// Absolute electrical limits and nominal voltage for a pin.
#[derive(Clone, Copy, Debug)]
pub struct PowerSpec {
    pub v_min: Qty<Volt>,
    pub v_max: Qty<Volt>,
    pub v_nom: Option<Qty<Volt>>,
    pub i_max: Qty<Amp>,
}

/// Classifies a signal family and integrity expectations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SigKind {
    Generic,
    Usb2Hs,
    Usb3,
    Ddr3,
    PcieGen2,
    Clock,
    AnalogLowNoise,
}

/// Signal integrity specification for a net or pin.
#[derive(Clone, Copy, Debug)]
pub struct SigSpec {
    pub kind: SigKind,
    pub bandwidth: Option<Qty<Hertz>>,
    pub edge_rate: Option<Qty<Second>>,
    pub target_impedance: Option<Qty<Ohm>>,
}

/// A thermal via embedded within a pad (e.g. inside an exposed thermal pad).
#[derive(Clone, Copy, Debug)]
pub struct ThermalVia {
    /// Position relative to the footprint origin, in millimetres.
    pub pos: (f64, f64),
    /// Drill diameter in millimetres.
    pub drill: f64,
    /// Finished pad diameter in millimetres.
    pub size: f64,
}

/// A logical pin on a component footprint.
///
/// `Pin` carries electrical identity and specification.  Physical data lives
/// in [`Pad`] (footprint geometry) and [`SymPin`] (schematic symbol graphics).
#[derive(Clone, Debug)]
pub struct Pin {
    id: PinId,
    name: String,
    number: Option<String>,
    role: Role,
    power_spec: PowerSpec,
    decouple: bool,
    sig_spec: Option<SigSpec>,
    thermal_vias: Vec<ThermalVia>,
    /// No-connect marker: when `true`, this pin is intentionally unconnected
    /// and is ignored by auto-wire tooling and ERC connectivity checks.
    nc: bool,
    /// Footprint pad geometry; `None` when the pin has no explicit pad data
    /// (fallback defaults are applied by [`resolve_pad`]).
    pad: Option<Pad>,
    /// Schematic symbol pin graphics; `None` when the pin has no explicit
    /// symbol data.
    symbol: Option<SymPin>,
}

pub struct PinBuilder {
    name: String,
    number: Option<String>,
    role: Option<Role>,
    power_spec: Option<PowerSpec>,
    decouple: bool,
    sig_spec: Option<SigSpec>,
    thermal_vias: Vec<ThermalVia>,
    nc: bool,
    // Pad override: set directly via `.pad(Pad)`.
    pad: Option<Pad>,
    // Individual pad fields set by convenience setters.
    pad_pos: Option<(f64, f64)>,
    pad_rotation: Option<f64>,
    pad_width: Option<f64>,
    pad_height: Option<f64>,
    pad_pad_type: Option<PadType>,
    pad_pad_shape: Option<PadShape>,
    pad_roundrect_rratio: Option<f64>,
    pad_solder_mask_margin: Option<f64>,
    pad_layers: Option<String>,
    pad_drill: Option<f64>,
    // Symbol override: set directly via `.symbol(SymPin)`.
    symbol: Option<SymPin>,
    // Individual symbol fields set by convenience setters.
    sym_pos: Option<(f64, f64)>,
    sym_rotation: Option<f64>,
    sym_length: Option<f64>,
}

/// Typed reference to a pin name constant on a component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PinRef(pub &'static str);

/// Handle to a specific pin on a specific component instance.
#[derive(Clone, Copy, Debug)]
pub struct PinHandle {
    pub component: usize,
    pub pin: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct RawConnection {
    pub from: PinHandle,
    pub to: PinHandle,
    pub id: usize,
}

impl PadType {
    /// Return the KiCad string representation of this pad type.
    pub fn as_str(&self) -> &'static str {
        match self {
            PadType::Smd => "smd",
            PadType::ThruHole => "thru_hole",
            PadType::NpThruHole => "np_thru_hole",
            PadType::Connect => "connect",
        }
    }

    /// Parse a KiCad pad-type string.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "smd" => Some(PadType::Smd),
            "thru_hole" => Some(PadType::ThruHole),
            "np_thru_hole" => Some(PadType::NpThruHole),
            "connect" => Some(PadType::Connect),
            _ => None,
        }
    }
}

impl PadShape {
    /// Return the KiCad string representation of this pad shape.
    pub fn as_str(&self) -> &'static str {
        match self {
            PadShape::Rect => "rect",
            PadShape::RoundRect => "roundrect",
            PadShape::Circle => "circle",
            PadShape::Oval => "oval",
        }
    }

    /// Parse a KiCad pad-shape string.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "rect" => Some(PadShape::Rect),
            "roundrect" => Some(PadShape::RoundRect),
            "circle" => Some(PadShape::Circle),
            "oval" => Some(PadShape::Oval),
            _ => None,
        }
    }
}

impl SigSpec {
    pub fn new(
        kind: SigKind,
        bandwidth: Option<Qty<Hertz>>,
        edge_rate: Option<Qty<Second>>,
        target_impedance: Option<Qty<Ohm>>,
    ) -> Self {
        Self {
            kind,
            bandwidth,
            edge_rate,
            target_impedance,
        }
    }

    /// Generic SPI signal with the given bandwidth in MHz and 50 Ω target impedance.
    pub fn spi(bw_mhz: f64) -> Self {
        Self {
            kind: SigKind::Generic,
            bandwidth: Some(bw_mhz.mhz()),
            edge_rate: None,
            target_impedance: Some(50.0.ohm()),
        }
    }

    /// SPI clock signal with the given bandwidth in MHz and 50 Ω target impedance.
    pub fn spi_clk(bw_mhz: f64) -> Self {
        Self {
            kind: SigKind::Clock,
            bandwidth: Some(bw_mhz.mhz()),
            edge_rate: None,
            target_impedance: Some(50.0.ohm()),
        }
    }

    /// Generic control signal with no bandwidth or impedance target.
    pub fn control() -> Self {
        Self {
            kind: SigKind::Generic,
            bandwidth: None,
            edge_rate: None,
            target_impedance: None,
        }
    }

    /// Analog low-noise 50 Ω signal (e.g., RF).
    pub fn rf_50ohm() -> Self {
        Self {
            kind: SigKind::AnalogLowNoise,
            bandwidth: None,
            edge_rate: None,
            target_impedance: Some(50.0.ohm()),
        }
    }
}

impl Pin {
    /// Start building a new [`Pin`].
    pub fn build(name: &str) -> PinBuilder {
        PinBuilder::new(name)
    }

    pub fn with_id(mut self, id: PinId) -> Self {
        self.id = id;
        self
    }

    pub fn id(&self) -> &PinId {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Physical pad/pin number as it appears in the package (e.g. `"1"` or
    /// `"A1"`).  Defaults to the 1-based pin index when unset.
    pub fn number(&self) -> Option<&str> {
        self.number.as_deref()
    }
    pub fn role(&self) -> Role {
        self.role
    }
    pub fn power_spec(&self) -> &PowerSpec {
        &self.power_spec
    }
    pub fn decouple(&self) -> bool {
        self.decouple
    }
    pub fn sig_spec(&self) -> Option<SigSpec> {
        self.sig_spec
    }
    /// Footprint pad geometry, if present.
    pub fn pad(&self) -> Option<&Pad> {
        self.pad.as_ref()
    }
    /// Schematic symbol pin graphics, if present.
    pub fn symbol(&self) -> Option<&SymPin> {
        self.symbol.as_ref()
    }
    /// Thermal vias embedded within this pad.
    pub fn thermal_vias(&self) -> &[ThermalVia] {
        &self.thermal_vias
    }
    /// Whether this pin is intentionally unconnected (no-connect).
    pub fn nc(&self) -> bool {
        self.nc
    }

    // ── Backward-compatibility accessors (delegate to pad/symbol) ──

    /// Footprint pad position in millimetres.
    pub fn pos(&self) -> Option<(f64, f64)> {
        self.pad.as_ref().map(|p| p.pos)
    }
    /// Footprint pad rotation in degrees.
    pub fn rotation(&self) -> Option<f64> {
        self.pad.as_ref().map(|p| p.rotation)
    }
    /// Symbol pin stub length in mils.
    pub fn length(&self) -> Option<f64> {
        self.symbol.as_ref().map(|s| s.length)
    }
    /// Pad width in millimetres (X dimension from KiCad `(size W H)`).
    pub fn width(&self) -> Option<f64> {
        self.pad.as_ref().map(|p| p.width)
    }
    /// Pad height in millimetres (Y dimension from KiCad `(size W H)`).
    pub fn height(&self) -> Option<f64> {
        self.pad.as_ref().map(|p| p.height)
    }
    /// KiCad pad type, as a string.
    pub fn pad_type(&self) -> Option<&str> {
        self.pad.as_ref().map(|p| p.pad_type.as_str())
    }
    /// Pad shape, as a string.
    pub fn pad_shape(&self) -> Option<&str> {
        self.pad.as_ref().map(|p| p.pad_shape.as_str())
    }
    /// Roundrect corner radius ratio (only meaningful for `roundrect` pads).
    pub fn roundrect_rratio(&self) -> Option<f64> {
        self.pad.as_ref().and_then(|p| p.roundrect_rratio)
    }
    /// Solder mask margin in millimetres.
    pub fn solder_mask_margin(&self) -> Option<f64> {
        self.pad.as_ref().and_then(|p| p.solder_mask_margin)
    }
    /// Copper layers for this pad, e.g. `"F.Cu F.Mask F.Paste"`.
    pub fn layers(&self) -> Option<&str> {
        self.pad.as_ref().and_then(|p| p.layers.as_deref())
    }
    /// Drill diameter in millimetres (thru-hole pads only).
    pub fn drill(&self) -> Option<f64> {
        self.pad.as_ref().and_then(|p| p.drill)
    }
}

impl PinBuilder {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            number: None,
            role: None,
            power_spec: None,
            decouple: false,
            sig_spec: None,
            thermal_vias: Vec::new(),
            nc: false,
            pad: None,
            pad_pos: None,
            pad_rotation: None,
            pad_width: None,
            pad_height: None,
            pad_pad_type: None,
            pad_pad_shape: None,
            pad_roundrect_rratio: None,
            pad_solder_mask_margin: None,
            pad_layers: None,
            pad_drill: None,
            symbol: None,
            sym_pos: None,
            sym_rotation: None,
            sym_length: None,
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_owned();
        self
    }

    /// Physical pad/pin number as it appears in the package (e.g. `"1"` or
    /// `"A1"`).
    pub fn number(mut self, number: &str) -> Self {
        self.number = Some(number.to_owned());
        self
    }

    pub fn role(mut self, role: Role) -> Self {
        self.role = Some(role);
        self
    }

    pub fn power_spec(mut self, p: PowerSpec) -> Self {
        self.power_spec = Some(p);
        self
    }

    pub fn decouple(mut self, decouple: bool) -> Self {
        self.decouple = decouple;
        self
    }

    pub fn sig_spec(mut self, spec: SigSpec) -> Self {
        self.sig_spec = Some(spec);
        self
    }

    /// Mark this pin as no-connect.
    pub fn nc(mut self, nc: bool) -> Self {
        self.nc = nc;
        self
    }

    // ── Footprint pad geometry ──

    /// Set the full footprint pad geometry.  Overrides any individual
    /// convenience-setters called before this point.
    pub fn pad(mut self, pad: Pad) -> Self {
        self.pad = Some(pad);
        self
    }

    /// Footprint pad position in millimetres.
    pub fn pos(mut self, x: f64, y: f64) -> Self {
        self.pad_pos = Some((x, y));
        self
    }

    /// Footprint pad rotation in degrees.
    pub fn rotation(mut self, deg: f64) -> Self {
        self.pad_rotation = Some(deg);
        self
    }

    /// Pad width in millimetres (X dimension from KiCad `(size W H)`).
    pub fn width(mut self, mm: f64) -> Self {
        self.pad_width = Some(mm);
        self
    }

    /// Pad height in millimetres (Y dimension from KiCad `(size W H)`).
    pub fn height(mut self, mm: f64) -> Self {
        self.pad_height = Some(mm);
        self
    }

    /// KiCad pad type.
    pub fn pad_type(mut self, pad_type: &str) -> Self {
        let pt = match pad_type {
            "smd" => PadType::Smd,
            "thru_hole" => PadType::ThruHole,
            "np_thru_hole" => PadType::NpThruHole,
            "connect" => PadType::Connect,
            other => panic!("unknown pad type: {other}"),
        };
        self.pad_pad_type = Some(pt);
        self
    }

    /// Pad shape.
    pub fn pad_shape(mut self, shape: &str) -> Self {
        let ps = match shape {
            "rect" => PadShape::Rect,
            "roundrect" => PadShape::RoundRect,
            "circle" => PadShape::Circle,
            "oval" => PadShape::Oval,
            other => panic!("unknown pad shape: {other}"),
        };
        self.pad_pad_shape = Some(ps);
        self
    }

    /// Roundrect corner radius ratio (only for `roundrect` pads).
    pub fn roundrect_rratio(mut self, ratio: f64) -> Self {
        self.pad_roundrect_rratio = Some(ratio);
        self
    }

    /// Solder mask margin in millimetres.
    pub fn solder_mask_margin(mut self, mm: f64) -> Self {
        self.pad_solder_mask_margin = Some(mm);
        self
    }

    /// Copper layers for this pad, e.g. `"F.Cu F.Mask F.Paste"`.
    pub fn layers(mut self, layers: &str) -> Self {
        self.pad_layers = Some(layers.to_owned());
        self
    }

    /// Drill diameter in millimetres (thru-hole pads only).
    pub fn drill(mut self, mm: f64) -> Self {
        self.pad_drill = Some(mm);
        self
    }

    // ── Schematic symbol pin graphics ──

    /// Set the full schematic symbol pin graphics.  Overrides any individual
    /// convenience-setters called before this point.
    pub fn symbol(mut self, sym: SymPin) -> Self {
        self.symbol = Some(sym);
        self
    }

    /// Symbol pin position in mils.
    pub fn sym_pos(mut self, x: f64, y: f64) -> Self {
        self.sym_pos = Some((x, y));
        self
    }

    /// Symbol pin rotation in degrees.
    pub fn sym_rotation(mut self, deg: f64) -> Self {
        self.sym_rotation = Some(deg);
        self
    }

    /// Pin stub length in mils (schematic symbol graphics).
    pub fn length(mut self, mm: f64) -> Self {
        self.sym_length = Some(mm);
        self
    }

    // ── Thermal vias ──

    /// Add a thermal via embedded within this pad.
    pub fn thermal_via(mut self, pos: (f64, f64), drill: f64, size: f64) -> Self {
        self.thermal_vias.push(ThermalVia { pos, drill, size });
        self
    }

    // ── Electrical short-hands ──

    /// Fixed-voltage power input: `v_nom = v_min = v_max = v`, `decouple = true`.
    pub fn pwr_fixed(mut self, v: Qty<Volt>, i: Qty<Amp>) -> Self {
        self.role = Some(Role::PowerIn);
        self.power_spec = Some(PowerSpec {
            v_min: v,
            v_max: v,
            v_nom: Some(v),
            i_max: i,
        });
        self.decouple = true;
        self
    }

    /// Flexible power input: `v_nom = None`, `decouple = true`.
    pub fn pwr(mut self, v_min: Qty<Volt>, v_max: Qty<Volt>, i: Qty<Amp>) -> Self {
        self.role = Some(Role::PowerIn);
        self.power_spec = Some(PowerSpec {
            v_min,
            v_max,
            v_nom: None,
            i_max: i,
        });
        self.decouple = true;
        self
    }

    /// Chainable override to set `v_nom` on a flexible pin.
    pub fn nominal(mut self, v: Qty<Volt>) -> Self {
        if let Some(ref mut p) = self.power_spec {
            p.v_nom = Some(v);
        }
        self
    }

    pub fn digital_limits(mut self) -> Self {
        self.power_spec = Some(PowerSpec {
            v_min: 0.0.volt(),
            v_max: 3.6.volt(),
            v_nom: None,
            i_max: 0.02.amp(),
        });
        self
    }

    pub fn rf_limits(mut self) -> Self {
        self.power_spec = Some(PowerSpec {
            v_min: 0.0.volt(),
            v_max: 1.2.volt(),
            v_nom: None,
            i_max: 1.0.amp(),
        });
        self
    }

    /// Creates a new digital I/O [`Pin`].
    pub fn dio(mut self) -> Pin {
        self.role = Some(Role::DigitalIO);
        self.digital_limits().pin()
    }

    /// Creates a new digital I/O [`Pin`] for SPI.
    pub fn spi(mut self, bw_mhz: f64) -> Pin {
        self.role = Some(Role::DigitalIO);
        self.sig_spec = Some(SigSpec::spi(bw_mhz));
        self.digital_limits().pin()
    }

    /// Creates a new digital clock signal [`Pin`].
    pub fn clk(mut self, bw_mhz: f64) -> Pin {
        self.role = Some(Role::DigitalIO);
        self.sig_spec = Some(SigSpec::spi_clk(bw_mhz));
        self.digital_limits().pin()
    }

    /// Creates a new ground [`Pin`].
    pub fn gnd(mut self) -> Pin {
        self.role = Some(Role::Gnd);
        self.power_spec = Some(PowerSpec {
            v_min: 0.0.volt(),
            v_max: 0.0.volt(),
            v_nom: Some(0.0.volt()),
            i_max: 100.0.amp(),
        });
        self.pin()
    }

    /// Creates a new analogue input [`Pin`].
    pub fn analog_in(mut self) -> Pin {
        self.role = Some(Role::AnalogIn);
        self.digital_limits().pin()
    }

    /// Returns a [`Pin`] with the settings configured with this builder.
    ///
    /// # Panics
    ///
    /// This method will panic if `role` or `power_spec` is not set.
    pub fn pin(self) -> Pin {
        let pad = self.pad.clone().or_else(|| self.build_pad());
        let symbol = self.symbol.or_else(|| self.build_sym());
        Pin {
            id: PinId(String::new()),
            name: self.name,
            number: self.number,
            role: self.role.unwrap(),
            power_spec: self.power_spec.unwrap(),
            decouple: self.decouple,
            sig_spec: self.sig_spec,
            thermal_vias: self.thermal_vias,
            nc: self.nc,
            pad,
            symbol,
        }
    }

    /// Build a [`Pad`] from the individual pad option fields, if any are set.
    fn build_pad(&self) -> Option<Pad> {
        let has_any = self.pad_pos.is_some()
            || self.pad_rotation.is_some()
            || self.pad_width.is_some()
            || self.pad_height.is_some()
            || self.pad_pad_type.is_some()
            || self.pad_pad_shape.is_some()
            || self.pad_roundrect_rratio.is_some()
            || self.pad_solder_mask_margin.is_some()
            || self.pad_layers.is_some()
            || self.pad_drill.is_some();
        if !has_any {
            return None;
        }
        Some(Pad {
            number: self.number.clone().unwrap_or_default(),
            pos: self.pad_pos.unwrap_or((0.0, 0.0)),
            rotation: self.pad_rotation.unwrap_or(0.0),
            width: self.pad_width.unwrap_or(0.0),
            height: self.pad_height.unwrap_or(0.0),
            pad_type: self.pad_pad_type.unwrap_or(PadType::Smd),
            pad_shape: self.pad_pad_shape.unwrap_or(PadShape::Rect),
            roundrect_rratio: self.pad_roundrect_rratio,
            solder_mask_margin: self.pad_solder_mask_margin,
            layers: self.pad_layers.clone(),
            drill: self.pad_drill,
        })
    }

    /// Build a [`SymPin`] from the individual symbol option fields, if any are set.
    fn build_sym(&self) -> Option<SymPin> {
        let has_any =
            self.sym_pos.is_some() || self.sym_rotation.is_some() || self.sym_length.is_some();
        if !has_any {
            return None;
        }
        Some(SymPin {
            pos: self.sym_pos.unwrap_or((0.0, 0.0)),
            rotation: self.sym_rotation.unwrap_or(0.0),
            length: self.sym_length.unwrap_or(0.0),
        })
    }
}

/// Auto-layout position for pins without explicit pad positions: a single
/// horizontal row at 2.54 mm pitch with pad 1 at the origin (KLC F7.2).
pub fn auto_pad_pos(index: usize) -> (f64, f64) {
    (index as f64 * 2.54, 0.0)
}

/// Normalise footprint pad anchor positions per KLC:
///
/// - Footprints with any SMD pad are recentred so the pad bounding box is
///   centred on the origin (KLC F6.2).
/// - Pure through-hole footprints with explicit positions are translated so
///   pad 1 sits at the origin (KLC F7.2).
/// - Fully automatic footprints (all pads on the 2.54 mm auto-row grid
///   starting from origin) are left untouched.
pub fn normalise_anchor(pads: &mut [Pad]) {
    if pads.is_empty() {
        return;
    }

    // Check if every pad follows the pure auto-row pattern (i * 2.54, 0.0).
    // If they do, pad 1 is already at the origin and no normalisation is needed.
    let all_auto = pads.iter().all(|p| (p.pos.1 * 1000.0).round() == 0.0)
        && pads
            .iter()
            .enumerate()
            .all(|(i, p)| ((p.pos.0 - auto_pad_pos(i).0) * 1000.0).round() == 0.0);
    if all_auto {
        return;
    }

    let any_smd = pads.iter().any(|p| p.pad_type == PadType::Smd);
    if any_smd {
        // Recentre on the pad bounding box.
        if let Some((x1, y1, x2, y2)) = pad_extent(pads) {
            let cx = (x1 + x2) / 2.0;
            let cy = (y1 + y2) / 2.0;
            for p in pads.iter_mut() {
                p.pos.0 -= cx;
                p.pos.1 -= cy;
            }
        }
    } else if let Some(anchor) = pads.first().map(|p| p.pos) {
        // Through-hole: first pad (the first electrical pin) at the origin.
        for p in pads.iter_mut() {
            p.pos.0 -= anchor.0;
            p.pos.1 -= anchor.1;
        }
    }
}

/// Compute the axis-aligned bounding box of a set of pads.
///
/// Returns `(min_x, min_y, max_x, max_y)` or `None` if the slice is empty.
pub fn pad_extent(pads: &[Pad]) -> Option<(f64, f64, f64, f64)> {
    if pads.is_empty() {
        return None;
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in pads {
        let hw = p.width / 2.0;
        let hh = p.height / 2.0;
        min_x = min_x.min(p.pos.0 - hw);
        min_y = min_y.min(p.pos.1 - hh);
        max_x = max_x.max(p.pos.0 + hw);
        max_y = max_y.max(p.pos.1 + hh);
    }
    Some((min_x, min_y, max_x, max_y))
}

/// Resolve a mechanical pad to fully-populated geometry for emission.
///
/// Mechanical pads receive:
/// - Number normalisation: `"None"` → empty string.
/// - Layer default: `"*.Cu *.Mask"` when not set.
/// - Drill: values ≤ 0 normalised to `None`.
pub fn resolve_mech_pad(pad: &Pad) -> Pad {
    let number = if pad.number.eq_ignore_ascii_case("none") {
        String::new()
    } else {
        pad.number.clone()
    };
    let layers = pad.layers.clone().unwrap_or_else(|| PTH_LAYERS.to_string());
    let drill = pad.drill.filter(|&d| d > 0.0);

    Pad {
        number,
        layers: Some(layers),
        drill,
        ..pad.clone()
    }
}

/// Resolve a pin's pad to fully-populated geometry for emission.
///
/// This function owns D2's pad-defaulting rules.  The board-emission path
/// calls it directly; the CLI `generate` path has an equivalent
/// `resolve_pin_def_pad` in `backend-kicad` that implements the same rules
/// against the TOML data model (see design D2).
///
/// # Defaults applied (in order of precedence)
///
/// 1. **pad_type** — explicit `pad.pad_type` wins; otherwise SMD iff the pin
///    has an explicit pad position, else through-hole.
/// 2. **pos** — explicit `pad.pos` wins; otherwise falls back to
///    [`auto_pad_pos`] (2.54 mm pitch row).
/// 3. **width / height** — explicit `pad.width`/`pad.height` > 0 wins;
///    otherwise falls back to [`SymPin::length`]; otherwise
///    [`DEFAULT_PAD_SIZE`].
/// 4. **layers** — explicit `pad.layers` wins; otherwise [`PTH_LAYERS`] for
///    through-hole, [`SMD_LAYERS`] for SMD.
/// 5. **drill** — explicit `pad.drill` wins; otherwise [`DEFAULT_DRILL`] for
///    through-hole, `None` for SMD.
/// 6. **pad_shape** — explicit `pad.pad_shape` wins; otherwise: for
///    auto-generated through-hole rows, pad 1 is [`PadShape::Rect`] and the
///    rest are [`PadShape::Circle`] (KLC F7.3); for all other cases,
///    [`PadShape::Rect`].
/// 7. **number** — explicit `pad.number` (non-empty) wins; otherwise
///    falls back to `pin.number()` or the 1-based pad index.
pub fn resolve_pad(pin: &Pin, index: usize) -> Pad {
    // A pin has an explicit position when its pad carries non-default geometry
    // (pos ≠ (0,0), width > 0, height > 0, or rotation ≠ 0).  A pad that was
    // auto-created by `build_pad` with only pad_type set, for example, should
    // NOT be treated as explicitly positioned — the auto-row rules must apply.
    let has_explicit_pos = pin.pad().is_some_and(|p| {
        p.pos != (0.0, 0.0) || p.width > 0.0 || p.height > 0.0 || p.rotation != 0.0
    });
    let has_explicit_pad = pin.pad().is_some();

    // 1. pad_type
    let is_through_hole = if let Some(pad) = pin.pad() {
        matches!(pad.pad_type, PadType::ThruHole | PadType::NpThruHole)
    } else {
        // Default: SMD if explicit position, else through-hole.
        !has_explicit_pos
    };
    let pad_type = if has_explicit_pad {
        pin.pad().unwrap().pad_type
    } else if has_explicit_pos {
        PadType::Smd
    } else {
        PadType::ThruHole
    };

    // 2. pos
    let pos = pin.pos().unwrap_or_else(|| auto_pad_pos(index));

    // 3. width / height
    let sym_len = pin.symbol().map(|s| s.length);
    let explicit_w = pin
        .pad()
        .and_then(|p| if p.width > 0.0 { Some(p.width) } else { None });
    let explicit_h = pin
        .pad()
        .and_then(|p| if p.height > 0.0 { Some(p.height) } else { None });
    let width = explicit_w.or(sym_len).unwrap_or(DEFAULT_PAD_SIZE);
    let height = explicit_h.or(sym_len).unwrap_or(DEFAULT_PAD_SIZE);

    // 4. layers
    let layers = pin.pad().and_then(|p| p.layers.clone()).unwrap_or_else(|| {
        if is_through_hole {
            PTH_LAYERS.to_string()
        } else {
            SMD_LAYERS.to_string()
        }
    });

    // 5. drill
    let drill = pin.pad().and_then(|p| p.drill).or({
        if is_through_hole {
            Some(DEFAULT_DRILL)
        } else {
            None
        }
    });

    // 6. pad_shape
    let pad_shape = pin.pad().map(|p| p.pad_shape).unwrap_or_else(|| {
        // KLC F7.3: auto TH rows: pad 1 rect, rest circle.
        if !has_explicit_pos && is_through_hole && index > 0 {
            PadShape::Circle
        } else {
            PadShape::Rect
        }
    });

    // 7. number
    let number = pin
        .pad()
        .and_then(|p| {
            if p.number.is_empty() {
                None
            } else {
                Some(p.number.clone())
            }
        })
        .or_else(|| pin.number().map(str::to_owned))
        .unwrap_or_else(|| (index + 1).to_string());

    let rotation = pin.pad().map(|p| p.rotation).unwrap_or(0.0);
    let roundrect_rratio = pin.pad().and_then(|p| p.roundrect_rratio);
    let solder_mask_margin = pin.pad().and_then(|p| p.solder_mask_margin);

    Pad {
        number,
        pos,
        rotation,
        width,
        height,
        pad_type,
        pad_shape,
        roundrect_rratio,
        solder_mask_margin,
        layers: Some(layers),
        drill,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UnitExt;

    // ── resolve_pad tests ──

    #[test]
    fn resolve_pad_uses_explicit_values() {
        let pin = Pin::build("1")
            .pos(3.0, 4.0)
            .width(1.0)
            .height(0.5)
            .pad_type("smd")
            .pad_shape("roundrect")
            .roundrect_rratio(0.25)
            .layers("F.Cu F.Mask F.Paste")
            .drill(0.0)
            .solder_mask_margin(0.1)
            .pwr_fixed(3.3.volt(), 0.1.amp())
            .pin();
        let pad = resolve_pad(&pin, 0);
        assert_eq!(pad.pos, (3.0, 4.0));
        assert_eq!(pad.width, 1.0);
        assert_eq!(pad.height, 0.5);
        assert_eq!(pad.pad_type, PadType::Smd);
        assert_eq!(pad.pad_shape, PadShape::RoundRect);
        assert_eq!(pad.roundrect_rratio, Some(0.25));
        assert_eq!(pad.solder_mask_margin, Some(0.1));
        assert_eq!(pad.number, "1");
    }

    #[test]
    fn resolve_pad_defaults_pad_type_smd_when_pos_set() {
        let pin = Pin::build("1").pos(1.0, 0.0).dio();
        let pad = resolve_pad(&pin, 0);
        assert_eq!(pad.pad_type, PadType::Smd);
    }

    #[test]
    fn resolve_pad_defaults_pad_type_th_when_no_pos() {
        let pin = Pin::build("1").dio();
        let pad = resolve_pad(&pin, 0);
        assert_eq!(pad.pad_type, PadType::ThruHole);
    }

    #[test]
    fn resolve_pad_defaults_drill_for_th() {
        let pin = Pin::build("1").dio();
        let pad = resolve_pad(&pin, 0);
        assert_eq!(pad.drill, Some(DEFAULT_DRILL));
    }

    #[test]
    fn resolve_pad_no_drill_for_smd() {
        let pin = Pin::build("1").pos(1.0, 0.0).dio();
        let pad = resolve_pad(&pin, 0);
        assert_eq!(pad.drill, None);
    }

    #[test]
    fn resolve_pad_defaults_layers() {
        let th = Pin::build("1").dio();
        assert_eq!(resolve_pad(&th, 0).layers.as_deref(), Some(PTH_LAYERS));

        let smd = Pin::build("1").pos(1.0, 0.0).dio();
        assert_eq!(resolve_pad(&smd, 0).layers.as_deref(), Some(SMD_LAYERS));
    }

    #[test]
    fn resolve_pad_shape_default_auto_row() {
        let p1 = Pin::build("1").dio();
        let p2 = Pin::build("2").dio();
        let p3 = Pin::build("3").dio();
        assert_eq!(resolve_pad(&p1, 0).pad_shape, PadShape::Rect);
        assert_eq!(resolve_pad(&p2, 1).pad_shape, PadShape::Circle);
        assert_eq!(resolve_pad(&p3, 2).pad_shape, PadShape::Circle);
    }

    #[test]
    fn resolve_pad_falls_back_width_height_to_length() {
        let pin = Pin::build("1").length(2.0).dio();
        let pad = resolve_pad(&pin, 0);
        assert_eq!(pad.width, 2.0);
        assert_eq!(pad.height, 2.0);
    }

    #[test]
    fn resolve_pad_falls_back_size_to_default() {
        let pin = Pin::build("1").dio();
        let pad = resolve_pad(&pin, 0);
        assert_eq!(pad.width, DEFAULT_PAD_SIZE);
        assert_eq!(pad.height, DEFAULT_PAD_SIZE);
    }

    #[test]
    fn resolve_pad_defaults_number_to_index() {
        let pin = Pin::build("A").dio();
        let pad = resolve_pad(&pin, 2);
        assert_eq!(pad.number, "3");
    }

    #[test]
    fn resolve_pad_uses_explicit_number() {
        let pin = Pin::build("A").number("7").dio();
        let pad = resolve_pad(&pin, 0);
        assert_eq!(pad.number, "7");
    }

    #[test]
    fn resolve_pad_auto_row_positions() {
        let pads: Vec<_> = (0..3)
            .map(|i| resolve_pad(&Pin::build("1").dio(), i))
            .collect();
        assert_eq!(pads[0].pos, (0.0, 0.0));
        assert_eq!(pads[1].pos, (2.54, 0.0));
        assert_eq!(pads[2].pos, (5.08, 0.0));
    }

    // ── resolve_mech_pad tests ──

    #[test]
    fn resolve_mech_pad_normalises_none_number() {
        let mech = Pad {
            number: "None".into(),
            pos: (1.0, 2.0),
            width: 3.0,
            height: 3.0,
            pad_type: PadType::NpThruHole,
            pad_shape: PadShape::Circle,
            ..default_pad()
        };
        let resolved = resolve_mech_pad(&mech);
        assert_eq!(resolved.number, "");
    }

    #[test]
    fn resolve_mech_pad_defaults_layers() {
        let mech = Pad {
            layers: None,
            ..default_pad()
        };
        let resolved = resolve_mech_pad(&mech);
        assert_eq!(resolved.layers.as_deref(), Some(PTH_LAYERS));
    }

    #[test]
    fn resolve_mech_pad_normalises_zero_drill() {
        let mech = Pad {
            drill: Some(0.0),
            ..default_pad()
        };
        let resolved = resolve_mech_pad(&mech);
        assert_eq!(resolved.drill, None);
    }

    // ── normalise_anchor tests ──

    #[test]
    fn normalise_anchor_auto_row_untouched() {
        let mut pads: Vec<Pad> = (0..3)
            .map(|i| resolve_pad(&Pin::build("1").dio(), i))
            .collect();
        let original: Vec<_> = pads.iter().map(|p| p.pos).collect();
        normalise_anchor(&mut pads);
        for (i, p) in pads.iter().enumerate() {
            assert_eq!(p.pos, original[i]);
        }
    }

    #[test]
    fn normalise_anchor_smd_centred() {
        let mut pads = vec![
            resolve_pad(&Pin::build("1").pos(1.0, 1.0).dio(), 0),
            resolve_pad(&Pin::build("2").pos(3.0, 3.0).dio(), 1),
        ];
        normalise_anchor(&mut pads);
        let (x1, y1, x2, y2) = pad_extent(&pads).unwrap();
        assert!((x1 + x2).abs() < 1e-9, "SMD anchor not centred: {x1}..{x2}");
        assert!((y1 + y2).abs() < 1e-9, "SMD anchor not centred: {y1}..{y2}");
    }

    #[test]
    fn normalise_anchor_th_on_pad1() {
        let mut pads = vec![
            resolve_pad(
                &Pin::build("1").pos(5.0, 3.0).pad_type("thru_hole").dio(),
                0,
            ),
            resolve_pad(
                &Pin::build("2").pos(7.54, 3.0).pad_type("thru_hole").dio(),
                1,
            ),
        ];
        normalise_anchor(&mut pads);
        assert_eq!(pads[0].pos, (0.0, 0.0));
        assert_eq!(pads[1].pos, (2.54, 0.0));
    }

    fn default_pad() -> Pad {
        Pad {
            number: String::new(),
            pos: (0.0, 0.0),
            rotation: 0.0,
            width: 1.0,
            height: 1.0,
            pad_type: PadType::Smd,
            pad_shape: PadShape::Rect,
            roundrect_rratio: None,
            solder_mask_margin: None,
            layers: None,
            drill: None,
        }
    }
}
