//! KiCad backend for Copperleaf.
//!
//! Emits `.kicad_pro`, `.kicad_sch`, `.kicad_pcb`, and `.net` files from a
//! [`CompiledBoard`].  All symbol and footprint geometry is embedded inline,
//! so the output is fully self-contained.

use std::{fs, path::Path};

use base64::Engine as _;
use copperleaf::{Backend, BackendError, CompiledBoard};

pub use fp_emitter::{EmitError, emit_footprint, emit_footprint_to};
pub use fp_parser::{
    PadDef, parse_footprint, parse_footprint_lib, parse_footprint_model, parse_footprint_model_lib,
};
pub use lib_emitter::{emit_footprint_lib, emit_symbol_lib};
pub use project::{emit_fp_lib_table, emit_sym_lib_table};
pub use sexpr::{ParseError, Sexpr, deterministic_uuid, kv, parse};
pub use sym_emitter::emit_symbol;
pub use sym_parser::{
    PinDef, SymbolDef, find_symbol, flatten_extends, parse_single_symbol, parse_symbol_lib,
};

pub mod common;
pub mod fp_emitter;
pub mod fp_geom;
pub mod fp_parser;
pub mod lib_emitter;
pub mod netlist;
pub mod pcb;
pub mod project;
pub mod schematic;
pub mod sexpr;
pub mod sym_emitter;
pub mod sym_layout;
pub mod sym_parser;

/// KiCad backend configuration.
#[derive(Clone, Debug, Default)]
pub struct KiCad {
    project_name: String,
}

impl KiCad {
    /// Create a new KiCad backend with the default project name.
    pub fn new() -> Self {
        Self {
            project_name: "copperleaf".into(),
        }
    }

    /// Set the project name used for the `.kicad_pro` filename.
    pub fn with_project_name(mut self, name: impl Into<String>) -> Self {
        self.project_name = name.into();
        self
    }
}

impl Backend for KiCad {
    type Error = BackendError;

    fn emit(&self, output_dir: impl AsRef<Path>, board: &CompiledBoard) -> Result<(), Self::Error> {
        let out = output_dir.as_ref().to_owned();
        fs::create_dir_all(&out)?;

        let pro = project::emit_project(&self.project_name, &[], None);
        fs::write(out.join(format!("{}.kicad_pro", self.project_name)), pro)?;

        let sch = schematic::emit_schematic(board);
        fs::write(out.join(format!("{}.kicad_sch", self.project_name)), sch)?;

        let pcb = pcb::emit_pcb(board, &self.project_name);
        fs::write(out.join(format!("{}.kicad_pcb", self.project_name)), pcb)?;

        let net = netlist::emit_netlist(board);
        fs::write(out.join(format!("{}.net", self.project_name)), net)?;

        // Write 3D model files from embedded base64 data.
        for comp in &board.components {
            if let Some(ref data) = comp.model_3d_data {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(data)
                    .map_err(|e| BackendError::EmitError(format!("base64 decode: {e}")))?;
                // Derive the filename from model_3d, or fall back to <refdes>.step.
                let fallback = format!("{}.step", comp.refdes);
                let filename = comp
                    .model_3d
                    .as_deref()
                    .and_then(|p| Path::new(p).file_name())
                    .and_then(|s| s.to_str())
                    .unwrap_or(&fallback);
                let dst = out.join(filename);
                fs::write(&dst, &bytes)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf::{Board, Component, Pin, PinRef};
    use copperleaf_compile;

    struct TwoPinPart {
        pins: Vec<Pin>,
    }

    impl TwoPinPart {
        const A: PinRef = PinRef("A");
        const B: PinRef = PinRef("B");
        fn new() -> Self {
            Self {
                pins: vec![Pin::build("A").dio(), Pin::build("B").dio()],
            }
        }
    }

    impl Component for TwoPinPart {
        fn pins(&self) -> &[Pin] {
            &self.pins
        }
    }

    #[test]
    fn emits_all_project_files() {
        let mut board = Board::new("test");
        let u1 = board.add("U1", TwoPinPart::new());
        let _ = board.connect(u1.pin(TwoPinPart::A), u1.pin(TwoPinPart::B));
        let report =
            copperleaf_compile::run(board, &copperleaf_compile::CompileOptions::default()).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let backend = KiCad::new().with_project_name("test");
        backend
            .emit(dir.path().to_str().unwrap(), &report.board)
            .unwrap();

        let names: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
            .collect();
        assert!(names.contains(&"test.kicad_pro".to_string()));
        assert!(names.contains(&"test.kicad_sch".to_string()));
        assert!(names.contains(&"test.kicad_pcb".to_string()));
        assert!(names.contains(&"test.net".to_string()));
        // No symbols/ or footprints/ directories — geometry is embedded inline.
        assert!(!names.contains(&"symbols".to_string()));
        assert!(!names.contains(&"footprints".to_string()));
    }
}
