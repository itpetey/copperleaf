//! KiCad backend for Copperleaf.
//!
//! Emits `.kicad_pro`, `.kicad_sch`, `.kicad_pcb`, and `.net` files from a
//! [`CompiledBoard`], as well as standalone symbol and footprint library
//! files in `symbols/` and `footprints/` subdirectories.

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

use copperleaf::{Backend, BackendError, CompiledBoard};

pub use fp_emitter::emit_footprint;
pub use fp_parser::{PadDef, parse_footprint, parse_footprint_lib};
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

        // ── group components by symbol library nickname ───────────
        let mut symbol_lib_nicks: Vec<String> = Vec::new();
        let mut lib_groups: HashMap<String, Vec<&copperleaf::CompiledComponent>> = HashMap::new();
        for comp in &board.components {
            let lib_name = common::symbol_lib_nick(comp);
            lib_groups.entry(lib_name).or_default().push(comp);
        }
        for lib_name in lib_groups.keys() {
            if !symbol_lib_nicks.contains(lib_name) {
                symbol_lib_nicks.push(lib_name.clone());
            }
        }

        // ── collect project-local footprints (deduplicated by name) ──
        // Components referencing an external `lib:footprint` are skipped.
        let mut footprint_names: Vec<String> = Vec::new();
        let mut fp_dedup: HashSet<String> = HashSet::new();
        for comp in &board.components {
            if let Some(fp_name) = common::local_footprint_name(comp)
                && fp_dedup.insert(fp_name.clone())
            {
                footprint_names.push(fp_name);
            }
        }
        let has_local_footprints = !footprint_names.is_empty();

        // ── write project file (with library registrations) ───────
        let pro = project::emit_project(
            &self.project_name,
            &symbol_lib_nicks,
            has_local_footprints.then_some(common::PROJECT_LIB),
        );
        fs::write(out.join(format!("{}.kicad_pro", self.project_name)), pro)?;

        // Library table files (standard KiCad mechanism, works in all versions).
        if !symbol_lib_nicks.is_empty() {
            let sym_table = project::emit_sym_lib_table(&symbol_lib_nicks);
            fs::write(out.join("sym-lib-table"), sym_table)?;
        }
        if has_local_footprints {
            let fp_table = project::emit_fp_lib_table(common::PROJECT_LIB);
            fs::write(out.join("fp-lib-table"), fp_table)?;
        }

        // ── schematic, pcb, netlist ───────────────────────────────
        let sch = schematic::emit_schematic(board);
        fs::write(out.join(format!("{}.kicad_sch", self.project_name)), sch)?;

        let pcb = pcb::emit_pcb(board, &self.project_name);
        fs::write(out.join(format!("{}.kicad_pcb", self.project_name)), pcb)?;

        let net = netlist::emit_netlist(board);
        fs::write(out.join(format!("{}.net", self.project_name)), net)?;

        // ── symbol library files ──────────────────────────────────
        let sym_dir = out.join("symbols");
        fs::create_dir_all(&sym_dir)?;

        for (lib_name, comps) in &lib_groups {
            let content = lib_emitter::emit_symbol_lib(comps, lib_name);
            fs::write(sym_dir.join(format!("{}.kicad_sym", lib_name)), content)?;
        }

        // ── footprint library files (project-local only) ──────────
        if has_local_footprints {
            let fp_dir = out.join("footprints");
            fs::create_dir_all(&fp_dir)?;

            for fp_name in &footprint_names {
                let comp = board
                    .components
                    .iter()
                    .find(|c| common::local_footprint_name(c).as_deref() == Some(fp_name))
                    .unwrap();
                let content = lib_emitter::emit_footprint_lib(comp, fp_name);
                fs::write(fp_dir.join(format!("{}.kicad_mod", fp_name)), content)?;
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
        let report = copperleaf_compile::run(board).unwrap();

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
        assert!(names.contains(&"symbols".to_string()));
        assert!(names.contains(&"footprints".to_string()));

        // Verify symbols/ directory contains at least one .kicad_sym file.
        let sym_dir = dir.path().join("symbols");
        let sym_files: Vec<_> = fs::read_dir(&sym_dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
            .collect();
        assert!(
            !sym_files.is_empty(),
            "symbols/ directory should not be empty"
        );
        assert!(
            sym_files.iter().any(|f| f.ends_with(".kicad_sym")),
            "should contain a .kicad_sym file, got: {:?}",
            sym_files
        );

        // Verify footprints/ directory contains at least one .kicad_mod file.
        let fp_dir = dir.path().join("footprints");
        let fp_files: Vec<_> = fs::read_dir(&fp_dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
            .collect();
        assert!(
            !fp_files.is_empty(),
            "footprints/ directory should not be empty"
        );
        assert!(
            fp_files.iter().any(|f| f.ends_with(".kicad_mod")),
            "should contain a .kicad_mod file, got: {:?}",
            fp_files
        );
    }
}
