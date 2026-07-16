//! Built-in and user-provided mapping from KiCad pin types to Copperleaf kinds.

use std::collections::HashMap;

use serde::Deserialize;

use crate::CliError;

/// A kind mapping entry, mirroring the optional fields on `PinDef` that a
/// mapping can set.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct KindEntry {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bw_mhz: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v_min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v_max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub i: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub i_max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nc: Option<bool>,
}

/// Mapping from KiCad pin types/names to Copperleaf kinds.
#[derive(Clone, Debug, Default)]
pub struct KindMap {
    by_type: HashMap<String, KindEntry>,
    by_name: HashMap<String, KindEntry>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct KindMapFile {
    #[serde(default)]
    by_type: HashMap<String, KindEntry>,
    #[serde(default)]
    by_name: HashMap<String, KindEntry>,
}

impl KindMap {
    /// Load a kind map, optionally from a TOML file, with built-in defaults.
    pub fn load(path: Option<&str>) -> Result<Self, CliError> {
        let mut map = KindMap::default();
        map.populate_builtins();
        if let Some(path) = path {
            let source = std::fs::read_to_string(path)?;
            let file: KindMapFile = toml::from_str(&source)?;
            for (k, v) in file.by_type {
                map.by_type.insert(k, v);
            }
            for (k, v) in file.by_name {
                map.by_name.insert(k, v);
            }
        }
        Ok(map)
    }

    /// Resolve the kind for a pin.
    ///
    /// Returns the matching entry and a boolean that is `true` when the
    /// built-in fallback to `default_kind` was used for an unrecognised pin
    /// type.
    pub fn resolve(&self, name: &str, pin_type: &str, default_kind: &str) -> (KindEntry, bool) {
        if let Some(entry) = self.by_name.get(name) {
            return (entry.clone(), false);
        }
        if let Some(entry) = self.by_type.get(pin_type) {
            return (entry.clone(), false);
        }
        if let Some(entry) = self.builtin(pin_type) {
            return (entry, false);
        }
        (
            KindEntry {
                kind: default_kind.into(),
                ..Default::default()
            },
            true,
        )
    }

    fn populate_builtins(&mut self) {
        for (ty, entry) in builtin_entries() {
            self.by_type.entry(ty.into()).or_insert(entry);
        }
        for (name, entry) in builtin_name_entries() {
            self.by_name.entry(name.into()).or_insert(entry);
        }
    }

    fn builtin(&self, pin_type: &str) -> Option<KindEntry> {
        builtin_entries()
            .iter()
            .find(|(ty, _)| *ty == pin_type)
            .map(|(_, e)| e.clone())
    }
}

/// Built-in name-based overrides for common pin names that KiCad classifies
/// as `power_in` but should map to `gnd` in Copperleaf.
fn builtin_name_entries() -> Vec<(&'static str, KindEntry)> {
    let gnd = KindEntry {
        kind: "gnd".into(),
        ..Default::default()
    };
    vec![("GND", gnd.clone()), ("VSS", gnd.clone()), ("PGND", gnd)]
}

fn builtin_entries() -> Vec<(&'static str, KindEntry)> {
    vec![
        (
            "power_in",
            KindEntry {
                kind: "pwr".into(),
                ..Default::default()
            },
        ),
        (
            "power",
            KindEntry {
                kind: "pwr".into(),
                ..Default::default()
            },
        ),
        (
            "power_out",
            KindEntry {
                kind: "pwr_fixed".into(),
                ..Default::default()
            },
        ),
        (
            "gnd",
            KindEntry {
                kind: "gnd".into(),
                ..Default::default()
            },
        ),
        (
            "ground",
            KindEntry {
                kind: "gnd".into(),
                ..Default::default()
            },
        ),
        (
            "passive",
            KindEntry {
                kind: "dio".into(),
                ..Default::default()
            },
        ),
        (
            "unspecified",
            KindEntry {
                kind: "dio".into(),
                ..Default::default()
            },
        ),
        (
            "input",
            KindEntry {
                kind: "dio".into(),
                ..Default::default()
            },
        ),
        (
            "output",
            KindEntry {
                kind: "dio".into(),
                ..Default::default()
            },
        ),
        (
            "bidirectional",
            KindEntry {
                kind: "dio".into(),
                ..Default::default()
            },
        ),
        (
            "3state",
            KindEntry {
                kind: "dio".into(),
                ..Default::default()
            },
        ),
        (
            "open_collector",
            KindEntry {
                kind: "dio".into(),
                ..Default::default()
            },
        ),
        (
            "open_emitter",
            KindEntry {
                kind: "dio".into(),
                ..Default::default()
            },
        ),
        (
            "free",
            KindEntry {
                kind: "dio".into(),
                ..Default::default()
            },
        ),
        (
            "clock",
            KindEntry {
                kind: "clk".into(),
                bw_mhz: Some(25.0),
                ..Default::default()
            },
        ),
        (
            "no_connect",
            KindEntry {
                kind: "dio".into(),
                nc: Some(true),
                ..Default::default()
            },
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_in_maps_to_pwr() {
        let map = KindMap::load(None).unwrap();
        let (entry, fallback) = map.resolve("VDD", "power_in", "dio");
        assert_eq!(entry.kind, "pwr");
        assert!(!fallback);
    }

    #[test]
    fn gnd_maps_to_gnd() {
        let map = KindMap::load(None).unwrap();
        let (entry, fallback) = map.resolve("GND", "gnd", "dio");
        assert_eq!(entry.kind, "gnd");
        assert!(!fallback);
    }

    #[test]
    fn gnd_name_overrides_power_in_type() {
        let map = KindMap::load(None).unwrap();
        let (entry, fallback) = map.resolve("GND", "power_in", "dio");
        assert_eq!(entry.kind, "gnd");
        assert!(!fallback);
    }

    #[test]
    fn clock_maps_to_clk_with_default_bw() {
        let map = KindMap::load(None).unwrap();
        let (entry, fallback) = map.resolve("CLK", "clock", "dio");
        assert_eq!(entry.kind, "clk");
        assert_eq!(entry.bw_mhz, Some(25.0));
        assert!(!fallback);
    }

    #[test]
    fn unknown_type_falls_back_to_default() {
        let map = KindMap::load(None).unwrap();
        let (entry, fallback) = map.resolve("X", "non_existent_pin_type", "dio");
        assert_eq!(entry.kind, "dio");
        assert!(fallback);
    }

    #[test]
    fn by_name_overrides_by_type() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("kind_map.toml");
        std::fs::write(
            &path,
            r#"
[by_type]
power_out = { kind = "pwr" }

[by_name]
"1V2O" = { kind = "pwr_fixed", v = 1.2, i = 0.01 }
"#,
        )
        .unwrap();
        let map = KindMap::load(Some(path.to_str().unwrap())).unwrap();
        let (entry, fallback) = map.resolve("1V2O", "power_out", "dio");
        assert_eq!(entry.kind, "pwr_fixed");
        assert_eq!(entry.v, Some(1.2));
        assert_eq!(entry.i, Some(0.01));
        assert!(!fallback);
    }

    #[test]
    fn by_type_override_takes_precedence_over_builtin() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("kind_map.toml");
        std::fs::write(
            &path,
            r#"
[by_type]
clock = { kind = "clk", bw_mhz = 100.0 }
"#,
        )
        .unwrap();
        let map = KindMap::load(Some(path.to_str().unwrap())).unwrap();
        let (entry, _) = map.resolve("CLK", "clock", "dio");
        assert_eq!(entry.bw_mhz, Some(100.0));
    }
}
