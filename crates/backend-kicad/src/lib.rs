//! KiCad backend for Copperleaf.
//!
//! Emits `.kicad_pro`, `.kicad_sch`, `.kicad_pcb`, and `.net` files from a
//! [`CompiledBoard`].  All symbol and footprint geometry is embedded inline,
//! so the output is fully self-contained.

use std::{fs, path::Path};

use base64::Engine as _;
use copperleaf::{Backend, BackendError, CompiledBoard, Layout};

pub use copperleaf::deterministic_id;
pub use fp_emitter::{EmitError, emit_footprint, emit_footprint_to};
pub use fp_parser::{
    PadDef, parse_footprint, parse_footprint_fab_extent, parse_footprint_lib,
    parse_footprint_model, parse_footprint_model_lib,
};
pub use lib_emitter::{emit_footprint_lib, emit_symbol_lib};
pub use pcb_parser::{ParsedPcb, RawPlacement, RawSegment, RawVia, RawZone, parse_pcb};
pub use project::{emit_fp_lib_table, emit_sym_lib_table};
pub use sexpr::{ParseError, Sexpr, kv, parse};
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
pub mod pcb_parser;
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

impl KiCad {
    /// Update an existing PCB file with a new [`CompiledBoard`], preserving
    /// all physical layout (placements, tracks, vias, zones) **and** all
    /// top-level graphic elements (`gr_line`, `gr_arc`, `gr_circle`,
    /// `gr_rect`).
    ///
    /// Reads the existing `.kicad_pcb` from `output_dir` (derived from the
    /// configured project name), parses layout data, remaps nets by name to
    /// the new board, and regenerates all output files.
    ///
    /// Copper elements referencing deleted nets are silently dropped. New
    /// components without an existing placement are auto-placed.
    pub fn emit_update(
        &self,
        output_dir: impl AsRef<Path>,
        board: &CompiledBoard,
    ) -> Result<(), BackendError> {
        let out = output_dir.as_ref();
        let pcb_path = out.join(format!("{}.kicad_pcb", self.project_name));
        let pcb_source = fs::read_to_string(&pcb_path)?;
        let parsed = pcb_parser::parse_pcb(&pcb_source)
            .map_err(|e| BackendError::EmitError(format!("parse existing PCB: {e}")))?;
        let layout = parsed.to_layout(board);
        self.emit_with_graphics(
            output_dir,
            board,
            &layout,
            &parsed.graphics,
            &parsed.footprints,
            &parsed.net_names,
        )
    }

    /// Emit board files with a layout **and** preserved board-level graphics.
    ///
    /// When `graphics` is non-empty those S-expression nodes replace the
    /// generated board outline so that manual edits to the board shape
    /// survive a schema update.
    fn emit_with_graphics(
        &self,
        output_dir: impl AsRef<Path>,
        board: &CompiledBoard,
        layout: &Layout,
        graphics: &[crate::sexpr::Sexpr],
        preserved_footprints: &[crate::sexpr::Sexpr],
        old_net_names: &std::collections::HashMap<usize, String>,
    ) -> Result<(), BackendError> {
        let out = output_dir.as_ref().to_owned();
        fs::create_dir_all(&out)?;

        let pro = project::emit_project(&self.project_name, Some(&board.design_rules));
        fs::write(out.join(format!("{}.kicad_pro", self.project_name)), pro)?;

        let sch = schematic::emit_schematic(board);
        fs::write(out.join(format!("{}.kicad_sch", self.project_name)), sch)?;

        let preserved = if graphics.is_empty() {
            None
        } else {
            Some(graphics)
        };
        let preserved_fps = if preserved_footprints.is_empty() {
            None
        } else {
            Some(preserved_footprints)
        };
        let pcb = pcb::emit_pcb_with_layout(
            board,
            &self.project_name,
            Some(layout),
            preserved,
            preserved_fps,
            old_net_names,
        );
        fs::write(out.join(format!("{}.kicad_pcb", self.project_name)), pcb)?;

        let net = netlist::emit_netlist(board);
        fs::write(out.join(format!("{}.net", self.project_name)), net)?;

        // Write 3D model files into models/ subdir.
        let models_dir = out.join("models");
        for comp in &board.components {
            if let Some(ref data) = comp.meta.model_3d_data {
                fs::create_dir_all(&models_dir)?;
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(data)
                    .map_err(|e| BackendError::EmitError(format!("base64 decode: {e}")))?;
                let fallback = format!("{}.step", comp.refdes);
                let filename = comp
                    .meta
                    .model_3d
                    .as_deref()
                    .and_then(|p| Path::new(p).file_name())
                    .and_then(|s| s.to_str())
                    .unwrap_or(&fallback);
                fs::write(models_dir.join(filename), &bytes)?;
            }
        }

        Ok(())
    }
}

impl Backend for KiCad {
    type Error = BackendError;

    fn emit(&self, output_dir: impl AsRef<Path>, board: &CompiledBoard) -> Result<(), Self::Error> {
        let out = output_dir.as_ref().to_owned();
        fs::create_dir_all(&out)?;

        let pro = project::emit_project(&self.project_name, Some(&board.design_rules));
        fs::write(out.join(format!("{}.kicad_pro", self.project_name)), pro)?;

        let sch = schematic::emit_schematic(board);
        fs::write(out.join(format!("{}.kicad_sch", self.project_name)), sch)?;

        let pcb = pcb::emit_pcb(board, &self.project_name);
        fs::write(out.join(format!("{}.kicad_pcb", self.project_name)), pcb)?;

        let net = netlist::emit_netlist(board);
        fs::write(out.join(format!("{}.net", self.project_name)), net)?;

        emit_local_footprints(&out, board)?;
        emit_3d_models(&out, board)?;

        Ok(())
    }

    fn emit_with_layout(
        &self,
        output_dir: impl AsRef<Path>,
        board: &CompiledBoard,
        layout: &Layout,
    ) -> Result<(), Self::Error> {
        let out = output_dir.as_ref().to_owned();
        fs::create_dir_all(&out)?;

        let pro = project::emit_project(&self.project_name, Some(&board.design_rules));
        fs::write(out.join(format!("{}.kicad_pro", self.project_name)), pro)?;

        let sch = schematic::emit_schematic(board);
        fs::write(out.join(format!("{}.kicad_sch", self.project_name)), sch)?;

        let pcb = pcb::emit_pcb_with_layout(
            board,
            &self.project_name,
            Some(layout),
            None,
            None,
            &std::collections::HashMap::new(),
        );
        fs::write(out.join(format!("{}.kicad_pcb", self.project_name)), pcb)?;

        let net = netlist::emit_netlist(board);
        fs::write(out.join(format!("{}.net", self.project_name)), net)?;

        emit_local_footprints(&out, board)?;
        emit_3d_models(&out, board)?;

        Ok(())
    }
}

/// Write 3D model files from embedded base64 data into the `models/` subdir.
fn emit_3d_models(out: &Path, board: &CompiledBoard) -> Result<(), BackendError> {
    let models_dir = out.join("models");
    for comp in &board.components {
        if let Some(ref data) = comp.meta.model_3d_data {
            fs::create_dir_all(&models_dir)?;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(data)
                .map_err(|e| BackendError::EmitError(format!("base64 decode: {e}")))?;
            let fallback = format!("{}.step", comp.refdes);
            let filename = comp
                .meta
                .model_3d
                .as_deref()
                .and_then(|p| Path::new(p).file_name())
                .and_then(|s| s.to_str())
                .unwrap_or(&fallback);
            fs::write(models_dir.join(filename), &bytes)?;
        }
    }
    Ok(())
}

/// Emit `.kicad_mod` files for components that reference the project-local
/// `copperleaf` footprint library and write the accompanying `fp-lib-table`.
fn emit_local_footprints(out: &Path, board: &CompiledBoard) -> Result<(), BackendError> {
    use std::collections::BTreeSet;

    let mut emitted: BTreeSet<String> = BTreeSet::new();
    let mut has_any = false;

    for comp in &board.components {
        let fp_ref = common::footprint_ref(comp);
        // Only emit for the `copperleaf:` library prefix — KiCad system
        // libraries (containing `/`) are resolved by KiCad itself.
        let Some(fp_name) = fp_ref.strip_prefix("copperleaf:") else {
            continue;
        };
        has_any = true;
        if emitted.insert(fp_name.to_string()) {
            let content = lib_emitter::emit_footprint_lib(comp, fp_name);
            let footprints_dir = out.join("footprints");
            fs::create_dir_all(&footprints_dir)?;
            fs::write(
                footprints_dir.join(format!("{}.kicad_mod", fp_name)),
                content,
            )?;
        }
    }

    if has_any {
        let fp_lib_table = project::emit_fp_lib_table("copperleaf");
        fs::write(out.join("fp-lib-table"), fp_lib_table)?;
    }

    Ok(())
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
        // The test part has no explicit footprint, so the backend falls
        // back to `copperleaf:U1`, emits the footprint under footprints/,
        // and writes fp-lib-table.
        assert!(!names.contains(&"symbols".to_string()));
        assert!(names.contains(&"footprints".to_string()));
    }
}
