//! PCB design rules — board-level physical constraints for manufacturing.
//!
//! [`DesignRules`] captures the minimum clearances, track widths, annular
//! ring sizes, and other constraints required by PCB manufacturers (e.g.
//! JLCPCB).  These are emitted into the KiCad project file's `rules` section
//! and validated by tools like Quilter.
//!
//! # Manufacturer profiles
//!
//! Convenience constructors mirror JLCPCB capabilities:
//!
//! | Profile               | Min track/clearance | Layers      |
//! |-----------------------|---------------------|-------------|
//! | [`jlcpcb_2layer`]     | 5 mil (0.127 mm)    | 2           |
//! | [`jlcpcb_4layer`]     | 3.5 mil (0.09 mm)   | 4           |
//! | [`jlcpcb_6layer`]     | 3.5 mil (0.09 mm)   | 6           |
//!
//! [`jlcpcb_2layer`]: DesignRules::jlcpcb_2layer
//! [`jlcpcb_4layer`]: DesignRules::jlcpcb_4layer
//! [`jlcpcb_6layer`]: DesignRules::jlcpcb_6layer

/// Board-level design rules for manufacturing constraints.
///
/// All values are in **millimetres**.  `Default` provides conservative
/// generic values suitable for most 2‑layer prototyping.
///
/// # Example
///
/// ```ignore
/// use copperleaf::DesignRules;
///
/// let rules = DesignRules::jlcpcb_2layer();
/// assert!(rules.min_clearance > 0.0, "Quilter requires positive clearance");
/// board.set_design_rules(rules);
/// ```
#[derive(Clone, Debug)]
pub struct DesignRules {
    /// Minimum clearance between any two copper features (mm).
    pub min_clearance: f64,
    /// Minimum edge-to-edge clearance between pads within the same
    /// footprint (mm).  When `0.0`, falls back to [`min_clearance`].
    ///
    /// This can be set smaller than `min_clearance` to accommodate
    /// fine-pitch footprints like 0201 (0603 metric) whose IPC‑7351
    /// pad gap is only 0.18mm.
    ///
    /// [`min_clearance`]: DesignRules::min_clearance
    pub min_pad_to_pad_clearance: f64,
    /// Minimum connection width (mm).
    pub min_connection: f64,
    /// Minimum clearance from copper to board edge (mm).
    pub min_copper_edge_clearance: f64,
    /// Minimum hole clearance — track/via to hole (mm).
    pub min_hole_clearance: f64,
    /// Minimum hole-to-hole spacing (mm).
    pub min_hole_to_hole: f64,
    /// Minimum silkscreen clearance to other features (mm).
    pub min_silk_clearance: f64,
    /// Minimum track width (mm).
    pub min_track_width: f64,
    /// Minimum via outer diameter (mm).
    pub min_via_diameter: f64,
    /// Minimum via annular ring width (mm).
    pub min_via_annular_width: f64,
    /// Minimum through-hole pad outer diameter (mm).
    pub min_through_hole_diameter: f64,
    /// Minimum microvia outer diameter (mm).
    pub min_microvia_diameter: f64,
    /// Minimum microvia drill diameter (mm).
    pub min_microvia_drill: f64,
    /// Solder mask to copper clearance (mm).
    pub solder_mask_to_copper_clearance: f64,
    /// Minimum on-board text height (mm).
    pub min_text_height: f64,
    /// Minimum on-board text stroke thickness (mm).
    pub min_text_thickness: f64,
}

impl DesignRules {
    // --- Manufacturer profiles ---

    /// JLCPCB 2‑layer board (1 oz outer copper).
    ///
    /// Based on [JLCPCB capabilities](https://jlcpcb.com/capabilities/pcb-capabilities).
    /// Minimum track/clearance: 5 mil (0.127 mm).
    pub fn jlcpcb_2layer() -> Self {
        Self {
            min_clearance: 0.127,
            min_pad_to_pad_clearance: 0.0,
            min_connection: 0.127,
            min_copper_edge_clearance: 0.3,
            min_hole_clearance: 0.254,
            min_hole_to_hole: 0.5,
            min_silk_clearance: 0.15,
            min_track_width: 0.127,
            min_via_diameter: 0.5,
            min_via_annular_width: 0.075,
            min_through_hole_diameter: 0.3,
            min_microvia_diameter: 0.45,
            min_microvia_drill: 0.2,
            solder_mask_to_copper_clearance: 0.05,
            min_text_height: 1.0,
            min_text_thickness: 0.15,
        }
    }

    /// JLCPCB 4‑layer board (0.5 oz inner, 1 oz outer copper).
    ///
    /// Minimum track/clearance: 3.5 mil (0.09 mm) on inner and outer layers.
    /// Via hole minimum is relaxed to 0.2 mm (vs. 0.3 mm for 2‑layer).
    pub fn jlcpcb_4layer() -> Self {
        Self {
            min_clearance: 0.09,
            min_pad_to_pad_clearance: 0.0,
            min_connection: 0.09,
            min_copper_edge_clearance: 0.3,
            min_hole_clearance: 0.254,
            min_hole_to_hole: 0.5,
            min_silk_clearance: 0.15,
            min_track_width: 0.09,
            min_via_diameter: 0.45,
            min_via_annular_width: 0.075,
            min_through_hole_diameter: 0.2,
            min_microvia_diameter: 0.45,
            min_microvia_drill: 0.15,
            solder_mask_to_copper_clearance: 0.05,
            min_text_height: 1.0,
            min_text_thickness: 0.15,
        }
    }

    /// JLCPCB 6‑layer board (same minimums as 4‑layer).
    pub fn jlcpcb_6layer() -> Self {
        Self::jlcpcb_4layer()
    }
}

impl Default for DesignRules {
    /// Conservative defaults suitable for most 2‑layer prototyping.
    ///
    /// These values are loose enough to work with most manufacturers
    /// while still passing basic DRC checks.  The critical value for
    /// Quilter compatibility is `min_clearance = 0.2`.
    fn default() -> Self {
        Self {
            min_clearance: 0.2,
            min_pad_to_pad_clearance: 0.0,
            min_connection: 0.0,
            min_copper_edge_clearance: 0.5,
            min_hole_clearance: 0.25,
            min_hole_to_hole: 0.25,
            min_silk_clearance: 0.0,
            min_track_width: 0.2,
            min_via_diameter: 0.5,
            min_via_annular_width: 0.1,
            min_through_hole_diameter: 0.3,
            min_microvia_diameter: 0.2,
            min_microvia_drill: 0.1,
            solder_mask_to_copper_clearance: 0.0,
            min_text_height: 0.8,
            min_text_thickness: 0.08,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_positive_clearance() {
        let rules = DesignRules::default();
        assert!(
            rules.min_clearance > 0.0,
            "Quilter requires min_clearance > 0"
        );
    }

    #[test]
    fn jlcpcb_2layer_min_clearance_is_5mil() {
        let rules = DesignRules::jlcpcb_2layer();
        assert!((rules.min_clearance - 0.127).abs() < 0.001);
        assert!((rules.min_track_width - 0.127).abs() < 0.001);
    }

    #[test]
    fn jlcpcb_4layer_min_clearance_is_3_5mil() {
        let rules = DesignRules::jlcpcb_4layer();
        assert!((rules.min_clearance - 0.09).abs() < 0.001);
        assert!((rules.min_track_width - 0.09).abs() < 0.001);
    }

    #[test]
    fn jlcpcb_6layer_equals_4layer() {
        let r4 = DesignRules::jlcpcb_4layer();
        let r6 = DesignRules::jlcpcb_6layer();
        // Compare field-by-field since we don't derive PartialEq.
        assert!((r4.min_clearance - r6.min_clearance).abs() < 1e-9);
        assert!((r4.min_track_width - r6.min_track_width).abs() < 1e-9);
        assert!((r4.min_via_diameter - r6.min_via_diameter).abs() < 1e-9);
    }

    #[test]
    fn jlcpcb_edge_clearance() {
        for rules in [DesignRules::jlcpcb_2layer(), DesignRules::jlcpcb_4layer()] {
            assert!(
                (rules.min_copper_edge_clearance - 0.3).abs() < 0.01,
                "edge clearance should be ~0.3 mm"
            );
        }
    }
}
