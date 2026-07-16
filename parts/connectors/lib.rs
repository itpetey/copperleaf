//! Connector vendor components
//!
//! Provides parametric connector types that can be instantiated with an
//! arbitrary number of pins at runtime.  Each connector family has a TOML
//! manifest defining mechanical parameters and a single-pin template; the Rust
//! struct repeats that template N times at the given pitch.
//!
//! Fixed-definition parts generated from TOML manifests via
//! `build_component!` coexist alongside parametric types.

use copperleaf::{Component, Pin};
use copperleaf_part_macro::build_component;

mod connector;

/// Standard JST PH through-hole wire-to-board connector.
///
/// Configuration is read from `jst_ph.toml` at construction time (embedded at
/// compile time via `include_str!`).  Pins are numbered `"1"` through `"N"` and
/// placed at 2.54 mm pitch along the Y-axis.
///
/// # Examples
///
/// ```ignore
/// use copperleaf::Board;
/// use copperleaf_parts_connectors::JstPh;
///
/// let mut board = Board::new();
/// let j1 = board.add("J1", JstPh::new(4));
/// ```
pub struct JstPh {
    pins: Vec<Pin>,
}

impl JstPh {
    /// Create a new JST PH connector with the given number of pins.
    ///
    /// # Panics
    ///
    /// Panics if `num_pins < 2`.
    pub fn new(num_pins: usize) -> Self {
        assert!(num_pins >= 2, "JST PH connector requires at least 2 pins");
        let config: connector::Config =
            toml::from_str(include_str!("jst_ph.toml")).expect("jst_ph.toml is valid");
        let pitch = config.connector.pitch;
        let pins: Vec<Pin> = (0..num_pins)
            .map(|i| {
                let name = format!("{}", i + 1);
                let y = i as f64 * pitch;
                config.pin_template.build_pin(&name, 0.0, y)
            })
            .collect();
        Self { pins }
    }

    /// The pin pitch in millimetres.
    pub fn pitch(&self) -> f64 {
        let config: connector::Config = toml::from_str(include_str!("jst_ph.toml")).unwrap();
        config.connector.pitch
    }

    /// Total number of pins on this connector.
    pub fn num_pins(&self) -> usize {
        self.pins.len()
    }
}

impl Component for JstPh {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }
}

build_component!("arjm11d7_502_ab_ew2.toml");

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf::Role;

    #[test]
    fn jst_ph_two_pins() {
        let j = JstPh::new(2);
        assert_eq!(j.num_pins(), 2);
        assert_eq!(j.pins()[0].name(), "1");
        assert_eq!(j.pins()[1].name(), "2");
        assert!(matches!(j.pins()[0].role(), Role::DigitalIO));
        assert!((j.pitch() - 2.54).abs() < 1e-9);
    }

    #[test]
    fn jst_ph_four_pins_positions() {
        let j = JstPh::new(4);
        assert_eq!(j.num_pins(), 4);
        assert_eq!(j.pins()[0].pos(), Some((0.0, 0.0)));
        assert_eq!(j.pins()[1].pos(), Some((0.0, 2.54)));
        assert_eq!(j.pins()[2].pos(), Some((0.0, 5.08)));
        assert_eq!(j.pins()[3].pos(), Some((0.0, 7.62)));
    }

    #[test]
    fn jst_ph_uses_toml_voltage_and_current() {
        let j = JstPh::new(3);
        for pin in j.pins() {
            assert!((pin.power_spec().v_max.as_base() - 100.0).abs() < 1e-9);
            assert!((pin.power_spec().i_max.as_base() - 2.0).abs() < 1e-9);
        }
    }
}
