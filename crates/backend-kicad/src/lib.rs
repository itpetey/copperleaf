//! KiCad backend for Copperleaf.
//!
//! Emits `.kicad_pro`, `.kicad_sch`, `.kicad_pcb`, and `.net` files from a
//! [`CompiledBoard`].

use std::{fs, path::PathBuf};

use copperleaf::{Backend, BackendError, CompiledBoard};

pub use fp_parser::{PadDef, parse_footprint, parse_footprint_lib};
pub use sexpr::{ParseError, Sexpr, deterministic_uuid, kv, parse};
pub use sym_parser::{
    PinDef, SymbolDef, find_symbol, flatten_extends, parse_single_symbol, parse_symbol_lib,
};

pub mod common;
pub mod fp_parser;
pub mod netlist;
pub mod pcb;
pub mod project;
pub mod schematic;
pub mod sexpr;
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

    fn emit(&self, output_dir: &str, board: &CompiledBoard) -> Result<(), Self::Error> {
        let out = PathBuf::from(output_dir);
        fs::create_dir_all(&out)?;

        let pro = project::emit_project(&self.project_name);
        fs::write(out.join(format!("{}.kicad_pro", self.project_name)), pro)?;

        let sch = schematic::emit_schematic(board);
        fs::write(out.join(format!("{}.kicad_sch", self.project_name)), sch)?;

        let pcb = pcb::emit_pcb(board);
        fs::write(out.join(format!("{}.kicad_pcb", self.project_name)), pcb)?;

        let net = netlist::emit_netlist(board);
        fs::write(out.join(format!("{}.net", self.project_name)), net)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use copperleaf::{Board, Component, Pin, PinRef};

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
        let mut board = Board::new();
        let u1 = board.add("U1", TwoPinPart::new());
        let _ = board.connect(u1.pin(TwoPinPart::A), u1.pin(TwoPinPart::B));
        let report = board.compile().unwrap();

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
    }
}
