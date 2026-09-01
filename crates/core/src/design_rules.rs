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
//! | Profile            | Min track/clearance | Via Ø / Drill | Annular ring |
//! |--------------------|---------------------|---------------|--------------|
//! | [`jlcpcb_6mil`]    | 0.152 mm (6 mil)    | 0.60 / 0.30   | 0.150 mm     |
//! | [`jlcpcb_4mil`]    | 0.102 mm (4 mil)    | 0.45 / 0.20   | 0.125 mm     |
//!
//! [`jlcpcb_6mil`]: DesignRules::jlcpcb_6mil
//! [`jlcpcb_4mil`]: DesignRules::jlcpcb_4mil

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
/// let rules = DesignRules::jlcpcb_6mil();
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
    /// Minimum via drill diameter (mm).
    pub min_via_drill: f64,
    /// Minimum via annular ring width (mm).
    pub min_via_annular_width: f64,
    /// Minimum through-hole drill (hole) diameter (mm).
    ///
    /// KiCad enforces this as a hole-size limit on all plated through
    /// holes — both PTH pads *and* vias — so keep it at or below the
    /// smallest drill used by any footprint on the board.  JLCPCB's
    /// minimum drill is 0.30 mm (2-layer) / 0.20 mm (4+ layer).
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

    /// JLCPCB 6 mil standard process.
    ///
    /// Based on [JLCPCB capabilities](https://jlcpcb.com/capabilities/pcb-capabilities).
    /// Suitable for all layer counts (2, 4, 6).
    /// Minimum track/clearance: 0.152 mm (6 mil).
    /// Minimum via: 0.60 mm pad / 0.30 mm drill.
    /// Minimum through-hole drill: 0.20 mm (JLCPCB 4+ layer minimum;
    /// 2-layer minimum is 0.30 mm).
    pub fn jlcpcb_6mil() -> Self {
        Self {
            min_clearance: 0.152,
            min_pad_to_pad_clearance: 0.0,
            min_connection: 0.152,
            min_copper_edge_clearance: 0.3,
            min_hole_clearance: 0.25,
            min_hole_to_hole: 0.5,
            min_silk_clearance: 0.15,
            min_track_width: 0.152,
            min_via_diameter: 0.60,
            min_via_drill: 0.30,
            min_via_annular_width: 0.15,
            min_through_hole_diameter: 0.20,
            min_microvia_diameter: 0.45,
            min_microvia_drill: 0.20,
            solder_mask_to_copper_clearance: 0.05,
            min_text_height: 1.0,
            min_text_thickness: 0.15,
        }
    }

    /// JLCPCB 4 mil advanced / JLC04161H-7628 process.
    ///
    /// Based on [JLCPCB capabilities](https://jlcpcb.com/capabilities/pcb-capabilities).
    /// Suitable for all layer counts (2, 4, 6).
    /// Minimum track/clearance: 0.102 mm (4 mil).
    /// Minimum via: 0.45 mm pad / 0.20 mm drill.
    /// Minimum through-hole drill: 0.20 mm (JLCPCB 4+ layer minimum).
    pub fn jlcpcb_4mil() -> Self {
        Self {
            min_clearance: 0.102,
            min_pad_to_pad_clearance: 0.0,
            min_connection: 0.102,
            min_copper_edge_clearance: 0.25,
            min_hole_clearance: 0.20,
            min_hole_to_hole: 0.4,
            min_silk_clearance: 0.15,
            min_track_width: 0.102,
            min_via_diameter: 0.45,
            min_via_drill: 0.20,
            min_via_annular_width: 0.125,
            min_through_hole_diameter: 0.20,
            min_microvia_diameter: 0.45,
            min_microvia_drill: 0.20,
            solder_mask_to_copper_clearance: 0.05,
            min_text_height: 1.0,
            min_text_thickness: 0.15,
        }
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
            min_via_drill: 0.2,
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
    fn jlcpcb_6mil_track_and_clearance() {
        let rules = DesignRules::jlcpcb_6mil();
        assert!((rules.min_clearance - 0.152).abs() < 0.001);
        assert!((rules.min_track_width - 0.152).abs() < 0.001);
        assert!((rules.min_connection - 0.152).abs() < 0.001);
    }

    #[test]
    fn jlcpcb_4mil_track_and_clearance() {
        let rules = DesignRules::jlcpcb_4mil();
        assert!((rules.min_clearance - 0.102).abs() < 0.001);
        assert!((rules.min_track_width - 0.102).abs() < 0.001);
        assert!((rules.min_connection - 0.102).abs() < 0.001);
    }

    #[test]
    fn jlcpcb_6mil_via_values() {
        let rules = DesignRules::jlcpcb_6mil();
        assert!(
            (rules.min_via_diameter - 0.60).abs() < 0.01,
            "via diameter should be 0.60 mm"
        );
        assert!(
            (rules.min_via_drill - 0.30).abs() < 0.01,
            "via drill should be 0.30 mm"
        );
        assert!(
            (rules.min_via_annular_width - 0.15).abs() < 0.01,
            "annular width should be 0.15 mm"
        );
    }

    #[test]
    fn jlcpcb_4mil_via_values() {
        let rules = DesignRules::jlcpcb_4mil();
        assert!(
            (rules.min_via_diameter - 0.45).abs() < 0.01,
            "via diameter should be 0.45 mm"
        );
        assert!(
            (rules.min_via_drill - 0.20).abs() < 0.01,
            "via drill should be 0.20 mm"
        );
        assert!(
            (rules.min_via_annular_width - 0.125).abs() < 0.01,
            "annular width should be 0.125 mm"
        );
    }

    #[test]
    fn jlcpcb_4mil_tighter_than_6mil() {
        let r4 = DesignRules::jlcpcb_4mil();
        let r6 = DesignRules::jlcpcb_6mil();
        assert!(r4.min_clearance < r6.min_clearance);
        assert!(r4.min_track_width < r6.min_track_width);
        assert!(r4.min_via_diameter < r6.min_via_diameter);
        assert!(r4.min_via_drill < r6.min_via_drill);
        assert!(r4.min_hole_clearance < r6.min_hole_clearance);
    }

    #[test]
    fn jlcpcb_edge_clearance() {
        assert!((DesignRules::jlcpcb_6mil().min_copper_edge_clearance - 0.3).abs() < 0.01);
        assert!((DesignRules::jlcpcb_4mil().min_copper_edge_clearance - 0.25).abs() < 0.01);
    }

    #[test]
    fn jlcpcb_hole_clearance() {
        assert!((DesignRules::jlcpcb_6mil().min_hole_clearance - 0.25).abs() < 0.01);
        assert!((DesignRules::jlcpcb_4mil().min_hole_clearance - 0.20).abs() < 0.01);
    }

    #[test]
    fn jlcpcb_through_hole_drill_is_manufacturable() {
        // KiCad enforces `min_through_hole_diameter` as a hole-size limit on
        // PTH pads *and* vias.  JLCPCB's minimum drill is 0.20 mm for 4+
        // layers, so the profiles must not reject 0.20 mm holes (e.g. the
        // MM8108 module's corner pins) or flag standard 0.4 mm via drills.
        for rules in [DesignRules::jlcpcb_6mil(), DesignRules::jlcpcb_4mil()] {
            assert!(
                rules.min_through_hole_diameter <= 0.20 + 1e-9,
                "through-hole drill minimum must be <= JLCPCB 0.20 mm 4-layer minimum"
            );
            assert!(
                rules.min_through_hole_diameter >= 0.20 - 1e-9,
                "through-hole drill minimum must not understate JLCPCB capability"
            );
        }
    }
}
