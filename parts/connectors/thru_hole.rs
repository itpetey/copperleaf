//! Rectangular grid of plated through-hole pads for headers and test points.

use copperleaf::{Component, Pin, PinRef, PowerSpec, Role, UnitExt};

/// A rectangular grid of plated through-hole pads.
///
/// Useful for pin headers, test-point arrays, or any row/column
/// arrangement of solderable holes.
///
/// Pad numbering is **column-major**: pin 1 is (row 1, col 1),
/// pin 2 is (row 2, col 1), pin 3 is (row 1, col 2), and so on.
/// This matches the physical layout of right-angle box headers where
/// adjacent pins on the same column pair are consecutive numbers.
pub struct ThruHoleHeader {
    pins: Vec<Pin>,
    rows: usize,
    cols: usize,
}

impl ThruHoleHeader {
    /// Create a new through-hole header grid.
    ///
    /// - `rows` / `cols` — grid dimensions (e.g. 2, 20 for a 2×20 GPIO header)
    /// - `drill` — hole diameter in mm (1.0 is standard for 2.54 mm headers)
    /// - `pad` — annular ring outer diameter in mm (1.7 typical)
    /// - `pitch` — centre-to-centre spacing in mm (2.54 standard)
    pub fn new(rows: usize, cols: usize, drill: f64, pad: f64, pitch: f64) -> Self {
        let mut pins = Vec::with_capacity(rows * cols);
        for col in 0..cols {
            for row in 0..rows {
                let x = col as f64 * pitch;
                let y = row as f64 * pitch;
                let name = format!("{}_{}", row + 1, col + 1);
                let number = (col * rows + row + 1).to_string();
                pins.push(
                    Pin::build(&name)
                        .number(&number)
                        .pad_type("thru_hole")
                        .pad_shape("circle")
                        .drill(drill)
                        .width(pad)
                        .height(pad)
                        .pos(x, y)
                        .layers("*.Cu *.Mask")
                        .role(Role::Passive)
                        .power_spec(PowerSpec {
                            v_min: 0.0.volt(),
                            v_max: 0.0.volt(),
                            v_nom: None,
                            i_max: 1.0.amp(),
                        })
                        .pin(),
                );
            }
        }
        Self { pins, rows, cols }
    }

    /// Get a [`PinRef`] for the hole at the given 1-indexed `(row, col)`.
    ///
    /// Uses `Box::leak` so the ref has `'static` lifetime — acceptable
    /// for board-level design code where the number of unique names is
    /// small and fixed at build time.
    pub fn pin_ref(row: usize, col: usize) -> PinRef {
        let s: &'static str = Box::leak(format!("{}_{}", row, col).into_boxed_str());
        PinRef(s)
    }

    /// Number of rows in the grid.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Number of columns in the grid.
    pub fn cols(&self) -> usize {
        self.cols
    }
}

impl Component for ThruHoleHeader {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thru_hole_header_pin_count_matches_grid() {
        let hdr = ThruHoleHeader::new(2, 20, 1.0, 1.7, 2.54);
        assert_eq!(hdr.pins().len(), 40);
        assert_eq!(hdr.rows(), 2);
        assert_eq!(hdr.cols(), 20);
    }

    #[test]
    fn thru_hole_header_pin_names_are_row_col() {
        let hdr = ThruHoleHeader::new(2, 3, 1.0, 1.7, 2.54);
        let names: Vec<_> = hdr.pins().iter().map(|p| p.name()).collect();
        assert_eq!(names, vec!["1_1", "2_1", "1_2", "2_2", "1_3", "2_3",]);
    }

    #[test]
    fn thru_hole_header_pin_numbers_are_sequential() {
        let hdr = ThruHoleHeader::new(2, 3, 1.0, 1.7, 2.54);
        let numbers: Vec<_> = hdr.pins().iter().map(|p| p.number().unwrap()).collect();
        assert_eq!(numbers, vec!["1", "2", "3", "4", "5", "6"]);
    }

    #[test]
    fn thru_hole_header_pads_are_through_hole_with_drill() {
        let hdr = ThruHoleHeader::new(1, 1, 0.8, 1.6, 2.54);
        let pad = hdr.pins()[0].pad().unwrap();
        assert_eq!(pad.pad_type.as_str(), "thru_hole");
        assert_eq!(pad.drill, Some(0.8));
        assert_eq!(pad.width, 1.6);
        assert_eq!(pad.height, 1.6);
    }

    #[test]
    fn thru_hole_header_positions_on_pitch_grid() {
        let hdr = ThruHoleHeader::new(2, 3, 1.0, 1.7, 2.54);
        assert_eq!(hdr.pins()[0].pos(), Some((0.0, 0.0)));
        assert_eq!(hdr.pins()[1].pos(), Some((0.0, 2.54)));
        assert_eq!(hdr.pins()[2].pos(), Some((2.54, 0.0)));
        assert_eq!(hdr.pins()[5].pos(), Some((5.08, 2.54)));
    }

    #[test]
    fn thru_hole_header_pin_ref_leaks_static_str() {
        let r1 = ThruHoleHeader::pin_ref(1, 1);
        let r2 = ThruHoleHeader::pin_ref(2, 3);
        assert_eq!(r1.0, "1_1");
        assert_eq!(r2.0, "2_3");
    }

    #[test]
    fn thru_hole_header_pin_ref_works_with_board() {
        use copperleaf::Board;
        let mut board = Board::new("test");
        let hdr = board.add("J1", ThruHoleHeader::new(2, 2, 1.0, 1.7, 2.54));
        let _ = hdr.pin(ThruHoleHeader::pin_ref(1, 1));
        let _ = hdr.pin(ThruHoleHeader::pin_ref(2, 2));
    }
}
