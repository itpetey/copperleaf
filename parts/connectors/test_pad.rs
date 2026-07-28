//! Single SMD test pad for pogo-pin or probe contact.

use copperleaf::{Component, Pin, PinRef, PowerSpec, Role, UnitExt};

/// A single SMD test pad for pogo-pin or probe contact.
///
/// Typical use cases:
/// - SWD debug headers (SWCLK, SWDIO, RUN, BOOTSEL, GND)
/// - Production programming fixtures
/// - Factory test points
///
/// Default pad size is 1.5 mm × 1.5 mm, suitable for standard
/// pogo pins (e.g. Mill-Max 0906 series).
#[derive(Clone, Debug)]
pub struct TestPad {
    pins: Vec<Pin>,
}

impl TestPad {
    pub const PAD: PinRef = PinRef("1");

    /// Create a single SMD test pad with the given width and height in mm.
    ///
    /// The pad is centred at the origin, suitable for pogo-pin contact.
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            pins: vec![Pin::build("1")
                .role(Role::Passive)
                .power_spec(PowerSpec {
                    v_min: 0.0.volt(),
                    v_max: 5.0.volt(),
                    v_nom: None,
                    i_max: 1.0.amp(),
                })
                .pos(0.0, 0.0)
                .width(width)
                .height(height)
                .pad_type("smd")
                .pad_shape("circle")
                .layers("F.Cu F.Mask")
                .pin()],
        }
    }

    /// Create a 1.5 mm × 1.5 mm test pad (standard pogo-pin size).
    pub fn pogo() -> Self {
        Self::new(1.5, 1.5)
    }
}

impl Component for TestPad {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pad_pogo_has_single_passive_pin() {
        let tp = TestPad::pogo();
        assert_eq!(tp.pins().len(), 1);
        assert!(matches!(tp.pins()[0].role(), Role::Passive));
        assert_eq!(tp.pins()[0].name(), "1");
    }

    #[test]
    fn test_pad_pogo_has_correct_dimensions() {
        let tp = TestPad::pogo();
        assert_eq!(tp.pins()[0].width(), Some(1.5));
        assert_eq!(tp.pins()[0].height(), Some(1.5));
        assert_eq!(tp.pins()[0].pad_type(), Some("smd"));
        assert_eq!(tp.pins()[0].pos(), Some((0.0, 0.0)));
    }

    #[test]
    fn test_pad_custom_size() {
        let tp = TestPad::new(2.0, 1.0);
        assert_eq!(tp.pins()[0].width(), Some(2.0));
        assert_eq!(tp.pins()[0].height(), Some(1.0));
    }

    #[test]
    fn test_pad_constant_is_accessible() {
        assert_eq!(TestPad::PAD.0, "1");
    }

    #[test]
    fn test_pad_works_with_board() {
        use copperleaf::Board;
        let mut board = Board::new("test");
        let tp = board.add("TP1", TestPad::pogo());
        let _ = tp.pin(TestPad::PAD);
    }
}
