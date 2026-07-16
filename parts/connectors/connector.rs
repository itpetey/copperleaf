//! Shared connector infrastructure.
//!
//! Parses connector TOML manifests containing a `[connector]` section with
//! mechanical parameters (pitch, pad/drill diameters, housing overhang) and a
//! `[pin_template]` section defining the electrical properties of one pin.
//! Concrete connector structs repeat this template N times at the given pitch.

use copperleaf::{Pin, PowerSpec, Role, UnitExt};
use serde::Deserialize;

/// Connector mechanical parameters.
#[derive(Deserialize)]
#[allow(dead_code)] // fields consumed as footprint emission improves
pub(crate) struct Meta {
    /// Human-readable name, e.g. `"JstPh"`.
    pub name: String,
    /// Display title shown in documentation, e.g. `"JST PH Connector"`.
    pub title: String,
    /// Optional longer description.
    #[serde(default)]
    pub description: Option<String>,
    /// Centre-to-centre spacing between adjacent pins, in mm.
    pub pitch: f64,
    /// Pad diameter, in mm (defaults to 1.524 if omitted).
    #[serde(default = "default_pad_diameter")]
    pub pad_diameter: f64,
    /// Drill diameter, in mm (defaults to 0.762 if omitted).
    #[serde(default = "default_drill_diameter")]
    pub drill_diameter: f64,
    /// How far the plastic housing extends past the outermost pad centres
    /// in the X direction, in mm (defaults to 1.27).
    #[serde(default = "default_housing_overhang_x")]
    pub housing_overhang_x: f64,
    /// How far the plastic housing extends past the pad centres in the
    /// ±Y direction, in mm (defaults to 3.5).
    #[serde(default = "default_housing_overhang_y")]
    pub housing_overhang_y: f64,
}

/// Template for a single pin — its electrical kind and limits.
#[derive(Deserialize)]
pub(crate) struct PinTemplate {
    /// Pin kind: `"dio"`, `"gnd"`, `"pwr"`, `"pwr_fixed"`, `"pwr_out"`,
    /// `"analog_in"`, `"analog_rf"`, `"clk"`, or `"spi"`.
    pub kind: String,
    /// Minimum operating voltage (required for `pwr`).
    #[serde(default)]
    pub v_min: Option<f64>,
    /// Maximum operating voltage (used by `pwr`, `dio`, etc.).
    #[serde(default = "default_v_max")]
    pub v_max: f64,
    /// Maximum current (required for `pwr`).
    #[serde(default)]
    pub i_max: Option<f64>,
    /// Fixed voltage (required for `pwr_fixed`, `pwr_out`).
    #[serde(default)]
    pub v: Option<f64>,
    /// Fixed current (required for `pwr_fixed`, `pwr_out`).
    #[serde(default)]
    pub i: Option<f64>,
    /// Pin rotation in degrees.
    #[serde(default)]
    pub rotation: f64,
    /// Pin length (through-hole depth), in mm.
    #[serde(default = "default_pin_length")]
    pub length: f64,
    /// Bandwidth in MHz (required for `clk`, `spi`).
    #[serde(default)]
    pub bw_mhz: Option<f64>,
}

/// A parsed connector manifest (TOML `[connector]` + `[pin_template]`).
#[derive(Deserialize)]
pub(crate) struct Config {
    pub connector: Meta,
    pub pin_template: PinTemplate,
}

impl PinTemplate {
    /// Build a single `Pin` from this template at the given position.
    pub fn build_pin(&self, name: &str, x: f64, y: f64) -> Pin {
        let mut builder = Pin::build(name).pos(x, y);
        if self.rotation != 0.0 {
            builder = builder.rotation(self.rotation);
        }
        if self.length != 0.0 {
            builder = builder.length(self.length);
        }
        match self.kind.as_str() {
            "gnd" => builder.gnd(),
            "dio" => builder
                .role(Role::DigitalIO)
                .power_spec(PowerSpec {
                    v_min: self.v_min.unwrap_or(0.0).volt(),
                    v_max: self.v_max.volt(),
                    v_nom: None,
                    i_max: self.i_max.unwrap_or(0.02).amp(),
                })
                .pin(),
            "analog_in" => builder.analog_in(),
            "analog_rf" => builder.role(Role::AnalogIn).rf_limits().pin(),
            "clk" => {
                let bw = self.bw_mhz.unwrap_or(10.0);
                builder.clk(bw)
            }
            "spi" => {
                let bw = self.bw_mhz.unwrap_or(10.0);
                builder.spi(bw)
            }
            "pwr" => {
                let vmin = self.v_min.expect("`pwr` pin requires v_min");
                let vmax = self.v_max;
                let imax = self.i_max.expect("`pwr` pin requires i_max");
                builder.pwr(vmin.volt(), vmax.volt(), imax.amp()).pin()
            }
            "pwr_fixed" => {
                let v = self.v.expect("`pwr_fixed` pin requires v");
                let i = self.i.expect("`pwr_fixed` pin requires i");
                builder.pwr_fixed(v.volt(), i.amp()).pin()
            }
            "pwr_out" => {
                let v = self.v.expect("`pwr_out` pin requires v");
                let i = self.i.expect("`pwr_out` pin requires i");
                builder
                    .role(Role::PowerOut)
                    .power_spec(PowerSpec {
                        v_min: v.volt(),
                        v_max: v.volt(),
                        v_nom: Some(v.volt()),
                        i_max: i.amp(),
                    })
                    .pin()
            }
            other => panic!("unknown pin kind '{}'", other),
        }
    }
}

fn default_drill_diameter() -> f64 {
    0.762
}

fn default_housing_overhang_x() -> f64 {
    1.27
}

fn default_housing_overhang_y() -> f64 {
    3.5
}

fn default_pad_diameter() -> f64 {
    1.524
}

fn default_pin_length() -> f64 {
    5.08
}

fn default_v_max() -> f64 {
    100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_jst_ph_toml() {
        let config: Config =
            toml::from_str(include_str!("jst_ph.toml")).expect("jst_ph.toml should parse");
        assert_eq!(config.connector.name, "JstPh");
        assert!((config.connector.pitch - 2.54).abs() < 1e-9);
        assert_eq!(config.pin_template.kind, "dio");
        assert!((config.pin_template.v_max - 100.0).abs() < 1e-9);
        assert!((config.pin_template.i_max.unwrap() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn build_dio_pin_from_template() {
        let t = PinTemplate {
            kind: "dio".into(),
            v_min: Some(0.0),
            v_max: 5.0,
            i_max: Some(1.0),
            v: None,
            i: None,
            rotation: 0.0,
            length: 5.08,
            bw_mhz: None,
        };
        let pin = t.build_pin("1", 0.0, 2.54);
        assert_eq!(pin.name(), "1");
        assert!(matches!(pin.role(), Role::DigitalIO));
        assert!((pin.power_spec().v_max.as_base() - 5.0).abs() < 1e-9);
        assert_eq!(pin.pos(), Some((0.0, 2.54)));
    }

    #[test]
    fn build_pwr_fixed_pin() {
        let t = PinTemplate {
            kind: "pwr_fixed".into(),
            v_min: None,
            v_max: 3.3,
            i_max: None,
            v: Some(3.3),
            i: Some(2.0),
            rotation: 90.0,
            length: 5.08,
            bw_mhz: None,
        };
        let pin = t.build_pin("VBAT", 0.0, 0.0);
        assert!(matches!(pin.role(), Role::PowerIn));
        assert_eq!(pin.rotation(), Some(90.0));
        assert!(pin.decouple());
    }
}
