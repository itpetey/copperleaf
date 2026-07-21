use std::path::Path;

// Re-export all public types at crate root for backward compatibility.
pub use board::{
    Board, BoardView, CompiledBoard, CompiledComponent, ComponentEntry, ComponentHandle,
    Connection, RawNetOverride,
};
pub use net::{Constraint, Net, NetClass, NetHandle, NetIdx, NetKind};
pub use pin::{
    DEFAULT_DRILL, DEFAULT_PAD_SIZE, PTH_LAYERS, Pad, PadShape, PadType, Pin, PinBuilder,
    PinHandle, PinId, PinRef, PowerSpec, RawConnection, Role, SMD_LAYERS, SigKind, SigSpec, SymPin,
    ThermalVia, auto_pad_pos, normalise_anchor, pad_extent, resolve_mech_pad, resolve_pad,
};
pub use units::{
    Amp, Celsius, Diagnostic, Farad, Henry, Hertz, Meter, Ohm, Qty, Second, Severity, UnitExt, Volt,
};
pub use util::deterministic_id;

pub mod board;
pub mod erc;
pub mod helpers;
pub mod net;
pub mod pin;
pub mod units;
pub mod util;

/// Trait implemented by backends that emit a [`CompiledBoard`] to a target format.
pub trait Backend {
    type Error;
    fn emit(&self, output_dir: impl AsRef<Path>, board: &CompiledBoard) -> Result<(), Self::Error>;
}

/// Represents a single part (e.g. a resistor, chip, etc.) on a PCB.
pub trait Component {
    /// Retrieves all [`Pin`]s attached to the [`Component`].
    fn pins(&self) -> &[Pin];

    /// Retrieves a [`Pin`] from this [`Component`] by ID, if it exists.
    fn pin(&self, id: PinId) -> Option<&Pin> {
        self.pins().iter().find(|p| *p.id() == id)
    }

    /// Retrieves a [`Pin`] by its name from this [`Component`], if it exists.
    fn pin_name(&self, name: &str) -> Option<&Pin> {
        self.pins().iter().find(|p| p.name() == name)
    }

    /// Non-electrical metadata: symbol/footprint identifiers, datasheet,
    /// description, 3D model data.
    fn meta(&self) -> &ComponentMeta {
        ComponentMeta::EMPTY
    }

    /// Constraints declared by this component for synthesis and analysis.
    fn constraints(&self) -> Vec<Constraint> {
        vec![]
    }

    /// Mechanical (non-electrical) pads belonging to this component's
    /// footprint — mounting holes, fiducials, paste apertures, etc.
    fn mechanical(&self) -> &[Pad] {
        &[]
    }
}

/// Non-electrical metadata for a component — symbol/footprint library identifiers,
/// datasheet URL, description, and 3D model data.
#[derive(Clone, Debug, Default)]
pub struct ComponentMeta {
    /// Symbol library identifier (e.g. `"RP2354A"` or `"MCU_RaspberryPi:RP2354A"`).
    pub symbol: Option<String>,
    /// Footprint name. Names without a `:` are project-local.
    pub footprint: Option<String>,
    /// Datasheet URL.
    pub datasheet: Option<String>,
    /// Human-readable description.
    pub description: Option<String>,
    /// Path to a 3D model file (`.step` / `.stp`).
    pub model_3d: Option<String>,
    /// Base64-encoded 3D model content, decoded during emit.
    pub model_3d_data: Option<String>,
    /// 3D model rotation in degrees (x, y, z).
    pub model_3d_rotation: (f64, f64, f64),
    /// 3D model offset in millimetres (x, y, z) relative to the footprint origin.
    pub model_3d_offset: (f64, f64, f64),
}

/// Common backend errors.
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("backend I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("backend emit error: {0}")]
    EmitError(String),
}

/// Error returned when board compilation fails.
#[derive(Clone, Debug, thiserror::Error)]
pub struct CompileError {
    pub errors: Vec<Diagnostic>,
}

impl ComponentMeta {
    /// An empty metadata value — all fields `None` / zero.
    pub const EMPTY: &Self = &ComponentMeta {
        symbol: None,
        footprint: None,
        datasheet: None,
        description: None,
        model_3d: None,
        model_3d_data: None,
        model_3d_rotation: (0.0, 0.0, 0.0),
        model_3d_offset: (0.0, 0.0, 0.0),
    };
}

impl CompileError {
    pub fn new(errors: Vec<Diagnostic>) -> Self {
        Self { errors }
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for e in &self.errors {
            writeln!(f, "[{:?}] {} — {}", e.severity, e.code, e.message)?;
            if let Some(hint) = &e.hint {
                writeln!(f, "         hint: {}", hint)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_id_is_stable() {
        let a = deterministic_id("U1:VDD");
        let b = deterministic_id("U1:VDD");
        assert_eq!(a, b);
        assert_ne!(a, deterministic_id("U1:GND"));
    }

    #[test]
    fn pin_id_is_string_newtype() {
        let id = PinId(deterministic_id("seed"));
        assert_eq!(id.0.len(), 36);
    }

    #[test]
    fn pwr_fixed_sets_all_fields() {
        let p = Pin::build("DVDD").pwr_fixed(1.1.volt(), 0.1.amp()).pin();
        assert!(matches!(p.role(), Role::PowerIn));
        assert!(p.decouple());
        assert!((p.power_spec().v_nom.unwrap().as_base() - 1.1).abs() < 1e-9);
        assert!((p.power_spec().v_min.as_base() - 1.1).abs() < 1e-9);
        assert!((p.power_spec().v_max.as_base() - 1.1).abs() < 1e-9);
    }

    #[test]
    fn pwr_leaves_v_nom_none() {
        let p = Pin::build("IOVDD")
            .pwr(1.8.volt(), 3.3.volt(), 0.1.amp())
            .pin();
        assert!(p.power_spec().v_nom.is_none());
    }

    #[test]
    fn nominal_chain_sets_v_nom() {
        let p = Pin::build("VBAT")
            .pwr(3.0.volt(), 3.6.volt(), 0.3.amp())
            .nominal(3.3.volt())
            .pin();
        assert!((p.power_spec().v_nom.unwrap().as_base() - 3.3).abs() < 1e-9);
    }

    #[test]
    fn physical_fields_round_trip() {
        let p = Pin::build("GPIO")
            .pos(1.0, 2.0)
            .rotation(90.0)
            .length(2.54)
            .dio();
        assert_eq!(p.pos(), Some((1.0, 2.0)));
        assert_eq!(p.rotation(), Some(90.0));
        assert_eq!(p.length(), Some(2.54));
    }

    #[test]
    fn pin_ref_and_handle() {
        pub const TEST_PIN: PinRef = PinRef("TEST");
        let handle = ComponentHandle(3).pin(TEST_PIN);
        assert_eq!(handle.component, 3);
        assert_eq!(handle.pin, "TEST");
    }

    struct TestPart;

    impl Component for TestPart {
        fn pins(&self) -> &[Pin] {
            static PINS: std::sync::OnceLock<Vec<Pin>> = std::sync::OnceLock::new();
            PINS.get_or_init(|| vec![Pin::build("VDD").pwr_fixed(3.3.volt(), 0.1.amp()).pin()])
        }
    }

    #[test]
    fn board_add_returns_handle() {
        let mut board = Board::new("test");
        let h = board.add("U1", TestPart);
        assert_eq!(h.0, 0);
    }

    #[test]
    fn board_connect_records_connection() {
        struct TwoPins;
        impl Component for TwoPins {
            fn pins(&self) -> &[Pin] {
                static PINS: std::sync::OnceLock<Vec<Pin>> = std::sync::OnceLock::new();
                PINS.get_or_init(|| {
                    vec![
                        Pin::build("A").pwr_fixed(3.3.volt(), 0.1.amp()).pin(),
                        Pin::build("B").pwr_fixed(3.3.volt(), 0.1.amp()).pin(),
                    ]
                })
            }
        }
        impl TwoPins {
            pub const A: PinRef = PinRef("A");
            pub const B: PinRef = PinRef("B");
        }

        let mut board = Board::new("test");
        let u1 = board.add("U1", TwoPins);
        let _ = board.connect(u1.pin(TwoPins::A), u1.pin(TwoPins::B));
        assert_eq!(board.connections.len(), 1);
    }

    #[test]
    fn empty_board_has_no_components_or_nets() {
        let board = Board::new("test");
        assert_eq!(board.components.len(), 0);
        assert!(board.connections.is_empty());
    }

    #[test]
    fn component_with_pins_has_constants_and_roles() {
        struct PinPart {
            pins: Vec<Pin>,
        }

        impl PinPart {
            pub const VDD: PinRef = PinRef("VDD");
            pub const GND: PinRef = PinRef("GND");
            pub const IO: PinRef = PinRef("IO");

            pub fn new() -> Self {
                Self {
                    pins: vec![
                        Pin::build("VDD").pwr_fixed(3.3.volt(), 0.1.amp()).pin(),
                        Pin::build("GND").gnd(),
                        Pin::build("IO").dio(),
                        Pin::build("VDD").pwr_fixed(3.3.volt(), 0.1.amp()).pin(),
                    ],
                }
            }
        }

        impl Component for PinPart {
            fn pins(&self) -> &[Pin] {
                &self.pins
            }
        }

        let part = PinPart::new();
        assert_eq!(part.pins().len(), 4);
        assert_eq!(PinPart::VDD.0, "VDD");
        assert_eq!(PinPart::GND.0, "GND");
        assert_eq!(PinPart::IO.0, "IO");
        assert!(matches!(part.pins()[0].role(), Role::PowerIn));
        assert!(matches!(part.pins()[1].role(), Role::Gnd));
        assert!(matches!(part.pins()[2].role(), Role::DigitalIO));
        assert!(matches!(part.pins()[3].role(), Role::PowerIn));
        assert_eq!(part.pins()[3].name(), "VDD");
    }
}
