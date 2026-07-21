//! Standard SMD footprint land patterns for passive components.
//!
//! Maps imperial codes (e.g. `"0603"`) to physical pad dimensions and positions
//! following the IEC 60062 / JEDEC land-pattern standard used by KiCad's
//! `Resistor_SMD` and `Capacitor_SMD` libraries.
//!
//! # Example
//!
//! ```ignore
//! use footprint::Package;
//!
//! let lp = Package::M0603.land_pattern();
//! assert_eq!(lp.imperial, "0603");
//! assert_eq!(lp.metric, "1608");
//! ```

/// Standard SMD footprint code.
///
/// Variants are named by their metric code (e.g. `M1608` for the 0603/1608
/// package).  Use [`from_code`](Package::from_code) to look up by imperial or
/// metric string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Package {
    /// 0201 (imperial) / 0603 (metric) — 0.6 × 0.3 mm body
    M0603,
    /// 0402 (imperial) / 1005 (metric) — 1.0 × 0.5 mm body
    M1005,
    /// 0603 (imperial) / 1608 (metric) — 1.6 × 0.8 mm body
    M1608,
    /// 0805 (imperial) / 2012 (metric) — 2.0 × 1.25 mm body
    M2012,
    /// 1206 (imperial) / 3216 (metric) — 3.2 × 1.6 mm body
    M3216,
    /// 1210 (imperial) / 3225 (metric) — 3.2 × 2.5 mm body
    M3225,
    /// 1812 (imperial) / 4532 (metric) — 4.5 × 3.2 mm body
    M4532,
    /// 2010 (imperial) / 5025 (metric) — 5.0 × 2.5 mm body
    M5025,
    /// 2512 (imperial) / 6332 (metric) — 6.3 × 3.2 mm body
    M6332,
}

/// Land pattern geometry for a two-pad SMD footprint.
#[derive(Clone, Copy, Debug)]
pub struct LandPattern {
    /// Imperial code string, e.g. `"0603"`.
    pub imperial: &'static str,
    /// Metric code string, e.g. `"1608"`.
    pub metric: &'static str,
    /// Body length (X dimension) in mm.
    pub body_x: f64,
    /// Body width (Y dimension) in mm.
    pub body_y: f64,
    /// Pad width (X dimension) in mm.
    pub pad_w: f64,
    /// Pad height (Y dimension) in mm.
    pub pad_h: f64,
    /// Center-to-center pitch between the two pads in mm.
    pub pitch: f64,
}

impl Package {
    /// Look up a footprint code by its imperial or metric name.
    ///
    /// Accepts e.g. `"0603"`, `"1608"`, `"0402"`, `"1005"`.
    pub fn from_code(s: &str) -> Option<Self> {
        Some(match s {
            "0201" | "0603" => Self::M0603,
            "0402" | "1005" => Self::M1005,
            /* "0603" | */ "1608" => Self::M1608,
            "0805" | "2012" => Self::M2012,
            "1206" | "3216" => Self::M3216,
            "1210" | "3225" => Self::M3225,
            "1812" | "4532" => Self::M4532,
            "2010" | "5025" => Self::M5025,
            "2512" | "6332" => Self::M6332,
            _ => return None,
        })
    }

    /// Resolve the land pattern geometry for this code.
    pub fn land_pattern(self) -> LandPattern {
        match self {
            Self::M0603 => LandPattern {
                imperial: "0201",
                metric: "0603",
                body_x: 0.6,
                body_y: 0.3,
                pad_w: 0.27,
                pad_h: 0.32,
                pitch: 0.32,
            },
            Self::M1005 => LandPattern {
                imperial: "0402",
                metric: "1005",
                body_x: 1.0,
                body_y: 0.5,
                pad_w: 0.5,
                pad_h: 0.65,
                pitch: 0.5,
            },
            Self::M1608 => LandPattern {
                imperial: "0603",
                metric: "1608",
                body_x: 1.6,
                body_y: 0.8,
                pad_w: 0.8,
                pad_h: 0.9,
                pitch: 1.0,
            },
            Self::M2012 => LandPattern {
                imperial: "0805",
                metric: "2012",
                body_x: 2.0,
                body_y: 1.25,
                pad_w: 1.0,
                pad_h: 1.2,
                pitch: 1.3,
            },
            Self::M3216 => LandPattern {
                imperial: "1206",
                metric: "3216",
                body_x: 3.2,
                body_y: 1.6,
                pad_w: 1.5,
                pad_h: 1.6,
                pitch: 2.0,
            },
            Self::M3225 => LandPattern {
                imperial: "1210",
                metric: "3225",
                body_x: 3.2,
                body_y: 2.5,
                pad_w: 1.5,
                pad_h: 2.5,
                pitch: 2.0,
            },
            Self::M4532 => LandPattern {
                imperial: "1812",
                metric: "4532",
                body_x: 4.5,
                body_y: 3.2,
                pad_w: 1.8,
                pad_h: 3.2,
                pitch: 2.8,
            },
            Self::M5025 => LandPattern {
                imperial: "2010",
                metric: "5025",
                body_x: 5.0,
                body_y: 2.5,
                pad_w: 1.6,
                pad_h: 2.5,
                pitch: 3.2,
            },
            Self::M6332 => LandPattern {
                imperial: "2512",
                metric: "6332",
                body_x: 6.3,
                body_y: 3.2,
                pad_w: 2.3,
                pad_h: 3.2,
                pitch: 3.8,
            },
        }
    }

    /// KiCad-style footprint library name for a resistor.
    pub fn resistor_footprint_name(self) -> &'static str {
        match self {
            Self::M0603 => "Resistor_SMD.3dshapes/R_0201_0603Metric",
            Self::M1005 => "Resistor_SMD.3dshapes/R_0402_1005Metric",
            Self::M1608 => "Resistor_SMD.3dshapes/R_0603_1608Metric",
            Self::M2012 => "Resistor_SMD.3dshapes/R_0805_2012Metric",
            Self::M3216 => "Resistor_SMD.3dshapes/R_1206_3216Metric",
            Self::M3225 => "Resistor_SMD.3dshapes/R_1210_3225Metric",
            Self::M4532 => "Resistor_SMD.3dshapes/R_1812_4532Metric",
            Self::M5025 => "Resistor_SMD.3dshapes/R_2010_5025Metric",
            Self::M6332 => "Resistor_SMD.3dshapes/R_2512_6332Metric",
        }
    }

    /// KiCad-style footprint library name for a capacitor.
    pub fn capacitor_footprint_name(self) -> &'static str {
        match self {
            Self::M0603 => "Capacitor_SMD.3dshapes/C_0201_0603Metric",
            Self::M1005 => "Capacitor_SMD.3dshapes/C_0402_1005Metric",
            Self::M1608 => "Capacitor_SMD.3dshapes/C_0603_1608Metric",
            Self::M2012 => "Capacitor_SMD.3dshapes/C_0805_2012Metric",
            Self::M3216 => "Capacitor_SMD.3dshapes/C_1206_3216Metric",
            Self::M3225 => "Capacitor_SMD.3dshapes/C_1210_3225Metric",
            Self::M4532 => "Capacitor_SMD.3dshapes/C_1812_4532Metric",
            Self::M5025 => "Capacitor_SMD.3dshapes/C_2010_5025Metric",
            Self::M6332 => "Capacitor_SMD.3dshapes/C_2512_6332Metric",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_by_imperial_and_metric() {
        assert_eq!(Package::from_code("0603"), Some(Package::M0603));
        assert_eq!(Package::from_code("1608"), Some(Package::M1608));
    }

    #[test]
    fn unknown_code_returns_none() {
        assert_eq!(Package::from_code("9999"), None);
        assert_eq!(Package::from_code(""), None);
    }

    #[test]
    fn land_pattern_pitch_is_positive() {
        for code in [
            Package::M0603,
            Package::M1005,
            Package::M1608,
            Package::M2012,
            Package::M3216,
            Package::M3225,
            Package::M4532,
            Package::M5025,
            Package::M6332,
        ] {
            let lp = code.land_pattern();
            assert!(lp.pitch > 0.0, "{} has non-positive pitch", lp.imperial);
            assert!(lp.pad_w > 0.0);
            assert!(lp.pad_h > 0.0);
            assert!(lp.body_x > 0.0);
            assert!(lp.body_y > 0.0);
        }
    }

    #[test]
    fn footprint_names_are_not_empty() {
        for code in [
            Package::M0603,
            Package::M1005,
            Package::M1608,
            Package::M2012,
            Package::M3216,
            Package::M3225,
            Package::M4532,
            Package::M5025,
            Package::M6332,
        ] {
            assert!(!code.resistor_footprint_name().is_empty());
            assert!(!code.capacitor_footprint_name().is_empty());
        }
    }
}
