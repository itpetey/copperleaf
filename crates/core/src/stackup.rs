//! PCB layer stackup — defines the physical layer structure of a board.
//!
//! A [`Stackup`] is an ordered sequence of alternating copper and dielectric
//! layers from top to bottom.  The first and last layers are always copper.
//! It is stored in [`CompiledBoard`](crate::CompiledBoard) and emitted by
//! backends (e.g. KiCad) to describe the board's cross‑section.

/// Role of a copper layer in the stackup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerRole {
    /// Signal routing layer.
    Signal,
    /// Power or ground plane.
    Plane,
    /// Mixed signal and plane.
    Mixed,
}

/// Dielectric material properties.
#[derive(Clone, Debug)]
pub struct Dielectric {
    /// Material name, e.g. `"FR4"`.
    pub material: String,
    /// Relative permittivity (dielectric constant, εᵣ / Dk), e.g. `4.5` for FR4.
    pub epsilon_r: f64,
    /// Loss tangent (Df), e.g. `0.02` for FR4.
    pub loss_tangent: f64,
}

/// A single layer in a PCB stackup.
#[derive(Clone, Debug)]
pub enum StackupLayer {
    /// A copper layer (signal, power plane, or mixed).
    Copper {
        /// Display name.
        name: String,
        /// Copper thickness in millimetres (e.g. `0.035` for 1 oz/ft²).
        thickness_mm: f64,
        /// Role of this layer.
        role: LayerRole,
    },
    /// A dielectric layer (core or prepreg).
    Dielectric {
        /// `"core"` or `"prepreg"`.
        kind: String,
        /// Thickness in millimetres.
        thickness_mm: f64,
        /// Material properties.
        dielectric: Dielectric,
    },
}

/// A PCB layer stackup from top to bottom.
///
/// Layers always alternate: copper, dielectric, copper, ..., copper.
/// The stackup must start and end with a copper layer.
#[derive(Clone, Debug, Default)]
pub struct Stackup {
    /// Alternating copper and dielectric layers, top to bottom.
    pub layers: Vec<StackupLayer>,
}

impl LayerRole {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Signal => "signal",
            Self::Plane => "plane",
            Self::Mixed => "mixed",
        }
    }
}

impl Dielectric {
    /// Standard FR‑4 material (εᵣ = 4.5, tan δ = 0.02).
    pub fn fr4() -> Self {
        Self {
            material: "FR4".to_owned(),
            epsilon_r: 4.5,
            loss_tangent: 0.02,
        }
    }

    /// Create a custom dielectric material.
    pub fn new(material: &str, epsilon_r: f64, loss_tangent: f64) -> Self {
        Self {
            material: material.to_owned(),
            epsilon_r,
            loss_tangent,
        }
    }
}

impl StackupLayer {
    /// Convenience constructor for a copper layer.
    pub fn copper(name: &str, thickness_mm: f64, role: LayerRole) -> Self {
        Self::Copper {
            name: name.into(),
            thickness_mm,
            role,
        }
    }

    /// Convenience constructor for a dielectric layer.
    pub fn dielectric(kind: &str, thickness_mm: f64, material: Dielectric) -> Self {
        Self::Dielectric {
            kind: kind.to_owned(),
            thickness_mm,
            dielectric: material,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Copper { name, .. } => name,
            Self::Dielectric { kind, .. } => kind,
        }
    }

    pub fn role(&self) -> &str {
        match self {
            Self::Copper { role, .. } => role.as_str(),
            Self::Dielectric { .. } => "dielectric",
        }
    }
}

impl Stackup {
    /// Standard 2‑layer FR‑4 board (1.6 mm total thickness).
    ///
    /// Layer sequence: top copper → FR4 core → bottom copper.
    pub fn two_layer() -> Self {
        Self {
            layers: vec![
                StackupLayer::copper("F", 0.035, LayerRole::Signal),
                StackupLayer::dielectric("core", 1.53, Dielectric::fr4()),
                StackupLayer::copper("B", 0.035, LayerRole::Signal),
            ],
        }
    }

    /// Standard 4‑layer FR‑4 board.
    ///
    /// Layer sequence: top signal → prepreg → ground plane → core →
    /// power plane → prepreg → bottom signal.
    pub fn four_layer() -> Self {
        Self {
            layers: vec![
                StackupLayer::copper("F", 0.035, LayerRole::Signal),
                StackupLayer::dielectric("prepreg", 0.2, Dielectric::fr4()),
                StackupLayer::copper("Gnd", 0.035, LayerRole::Plane),
                StackupLayer::dielectric("core", 1.13, Dielectric::fr4()),
                StackupLayer::copper("Pwr", 0.035, LayerRole::Plane),
                StackupLayer::dielectric("prepreg", 0.2, Dielectric::fr4()),
                StackupLayer::copper("B", 0.035, LayerRole::Signal),
            ],
        }
    }

    /// Standard 6‑layer FR‑4 board.
    ///
    /// Layer sequence: top signal → prepreg → ground plane → core →
    /// power plane → prepreg → signal → prepreg → ground plane →
    /// prepreg → bottom signal.
    pub fn six_layer() -> Self {
        Self {
            layers: vec![
                StackupLayer::copper("F", 0.035, LayerRole::Signal),
                StackupLayer::dielectric("prepreg", 0.0994, Dielectric::fr4()),
                StackupLayer::copper("Gnd", 0.0152, LayerRole::Plane),
                StackupLayer::dielectric("core", 0.55, Dielectric::fr4()),
                StackupLayer::copper("Pwr", 0.0152, LayerRole::Plane),
                StackupLayer::dielectric("prepreg", 0.1088, Dielectric::fr4()),
                StackupLayer::copper("Sig", 0.0152, LayerRole::Plane),
                StackupLayer::dielectric("core", 0.55, Dielectric::fr4()),
                StackupLayer::copper("Gnd", 0.0152, LayerRole::Plane),
                StackupLayer::dielectric("prepreg", 0.0994, Dielectric::fr4()),
                StackupLayer::copper("B", 0.035, LayerRole::Signal),
            ],
        }
    }

    /// Total board thickness in millimetres (sum of all layer thicknesses).
    pub fn total_thickness_mm(&self) -> f64 {
        self.layers
            .iter()
            .map(|l| match l {
                StackupLayer::Copper { thickness_mm, .. } => *thickness_mm,
                StackupLayer::Dielectric { thickness_mm, .. } => *thickness_mm,
            })
            .sum()
    }

    /// The number of copper layers in this stackup.
    pub fn copper_layer_count(&self) -> usize {
        self.layers
            .iter()
            .filter(|l| matches!(l, StackupLayer::Copper { .. }))
            .count()
    }

    /// Returns the copper layer at the given canonical ordinal, if it exists.
    ///
    /// Ordinal `0` is the front copper layer; the last valid ordinal is the
    /// back copper layer.
    pub fn copper_layer(&self, layer: crate::layout::CopperLayer) -> Option<&StackupLayer> {
        if layer.0 >= self.copper_layer_count() {
            return None;
        }
        self.layers.get(layer.0 * 2)
    }

    /// Reject a canonical copper ordinal that this stackup does not define.
    pub fn validate_copper_layer(
        &self,
        layer: crate::layout::CopperLayer,
    ) -> Result<(), crate::units::Diagnostic> {
        if self.copper_layer(layer).is_some() {
            return Ok(());
        }
        Err(crate::units::Diagnostic {
            code: "BOARD:LAYER_OUT_OF_RANGE".into(),
            severity: crate::units::Severity::Error,
            message: format!(
                "copper layer ordinal {} is outside the stackup's {} copper layer(s)",
                layer.0,
                self.copper_layer_count()
            ),
            entities: vec![format!("layer:{}", layer.0)],
            hint: Some(
                "Use a zero-based copper ordinal between 0 and the last stackup copper layer."
                    .into(),
            ),
        })
    }

    /// Returns the nth copper layer, where i=1 is the *second* copper layer.
    ///
    /// # Panics
    ///
    /// This method panics when `i` is out of bounds.
    pub fn nth_copper(&self, i: usize) -> &StackupLayer {
        let i = i * 2;
        &self.layers[i]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_layer_default_has_correct_thickness() {
        let s = Stackup::two_layer();
        let t = s.total_thickness_mm();
        // 0.035 + 1.53 + 0.035 ≈ 1.6
        assert!((t - 1.6).abs() < 0.01, "expected ~1.6 mm, got {}", t);
    }

    #[test]
    fn two_layer_has_three_entries() {
        let s = Stackup::two_layer();
        assert_eq!(s.layers.len(), 3);
    }

    #[test]
    fn four_layer_has_seven_entries() {
        let s = Stackup::four_layer();
        assert_eq!(s.layers.len(), 7);
    }

    #[test]
    fn copper_layer_count() {
        assert_eq!(Stackup::two_layer().copper_layer_count(), 2);
        assert_eq!(Stackup::four_layer().copper_layer_count(), 4);
    }

    #[test]
    fn default_stackup_is_empty() {
        let s = Stackup::default();
        assert!(s.layers.is_empty());
    }

    #[test]
    fn two_layer_starts_and_ends_with_copper() {
        let s = Stackup::two_layer();
        assert!(matches!(
            s.layers.first(),
            Some(StackupLayer::Copper { .. })
        ));
        assert!(matches!(s.layers.last(), Some(StackupLayer::Copper { .. })));
    }

    #[test]
    fn four_layer_alternates() {
        let s = Stackup::four_layer();
        for (i, layer) in s.layers.iter().enumerate() {
            if i % 2 == 0 {
                assert!(matches!(layer, StackupLayer::Copper { .. }));
            } else {
                assert!(matches!(layer, StackupLayer::Dielectric { .. }));
            }
        }
    }

    #[test]
    fn canonical_copper_ordinals_are_bounded_by_the_stackup() {
        use crate::layout::CopperLayer;

        let two = Stackup::two_layer();
        assert!(CopperLayer(0).is_valid_in(&two));
        assert!(
            CopperLayer(1).is_valid_in(&two),
            "two-layer back copper is ordinal 1"
        );
        assert!(!CopperLayer(2).is_valid_in(&two));

        let four = Stackup::four_layer();
        assert!(CopperLayer(0).is_valid_in(&four));
        assert!(CopperLayer(1).is_valid_in(&four));
        assert!(CopperLayer(2).is_valid_in(&four));
        assert!(CopperLayer(3).is_valid_in(&four));
        assert!(!CopperLayer(4).is_valid_in(&four));
        assert_eq!(
            four.copper_layer(CopperLayer(1)).map(StackupLayer::name),
            Some("Gnd")
        );
        assert!(four.copper_layer(CopperLayer(4)).is_none());

        let error = four
            .validate_copper_layer(CopperLayer(4))
            .expect_err("ordinal 4 is outside a four-layer stackup");
        assert_eq!(error.code, "BOARD:LAYER_OUT_OF_RANGE");
        assert!(four.validate_copper_layer(CopperLayer(3)).is_ok());
    }
}
