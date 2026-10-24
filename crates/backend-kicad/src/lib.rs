//! KiCad backend for Copperleaf.
//!
//! Emits `.kicad_pro`, `.kicad_sch`, `.kicad_pcb`, and `.net` files from a
//! [`CompiledBoard`].  All symbol and footprint geometry is embedded inline,
//! so the output is fully self-contained.

use std::{fs, path::Path};

use base64::Engine as _;
use copperleaf::{
    Backend, BackendError, CompiledBoard, Layout, Severity, safe_join, validate_compiled_board,
};

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
    /// Update schematic and project files from a new [`CompiledBoard`]
    /// without touching the existing `.kicad_pcb`.
    ///
    /// Emits `.kicad_pro`, `.kicad_sch`, `.net`, and 3D model files.  The
    /// PCB layout is left alone so that manual edits are not overwritten.
    pub fn emit_update(
        &self,
        output_dir: impl AsRef<Path>,
        board: &CompiledBoard,
    ) -> Result<(), BackendError> {
        ensure_valid(board, None)?;
        let out = output_dir.as_ref().to_owned();
        validate_output_names(&out, board, &self.project_name)?;
        fs::create_dir_all(&out)?;

        let pro = project::emit_project(&self.project_name, Some(&board.design_rules));
        write_contained(
            out.as_path(),
            &format!("{}.kicad_pro", self.project_name),
            &pro,
        )?;

        let sch = schematic::emit_schematic(board);
        write_contained(
            out.as_path(),
            &format!("{}.kicad_sch", self.project_name),
            &sch,
        )?;

        let net = netlist::emit_netlist(board);
        write_contained(out.as_path(), &format!("{}.net", self.project_name), &net)?;

        emit_3d_models(&out, board)?;

        Ok(())
    }
}

impl Backend for KiCad {
    type Error = BackendError;

    fn emit(&self, output_dir: impl AsRef<Path>, board: &CompiledBoard) -> Result<(), Self::Error> {
        ensure_valid(board, None)?;
        let out = output_dir.as_ref().to_owned();
        validate_output_names(&out, board, &self.project_name)?;
        fs::create_dir_all(&out)?;

        let pro = project::emit_project(&self.project_name, Some(&board.design_rules));
        write_contained(
            out.as_path(),
            &format!("{}.kicad_pro", self.project_name),
            &pro,
        )?;

        let sch = schematic::emit_schematic(board);
        write_contained(
            out.as_path(),
            &format!("{}.kicad_sch", self.project_name),
            &sch,
        )?;

        let pcb = pcb::emit_pcb(board, &self.project_name);
        write_contained(
            out.as_path(),
            &format!("{}.kicad_pcb", self.project_name),
            &pcb,
        )?;

        let net = netlist::emit_netlist(board);
        write_contained(out.as_path(), &format!("{}.net", self.project_name), &net)?;

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
        ensure_valid(board, Some(layout))?;
        let out = output_dir.as_ref().to_owned();
        validate_output_names(&out, board, &self.project_name)?;
        fs::create_dir_all(&out)?;

        let pro = project::emit_project(&self.project_name, Some(&board.design_rules));
        write_contained(
            out.as_path(),
            &format!("{}.kicad_pro", self.project_name),
            &pro,
        )?;

        let sch = schematic::emit_schematic(board);
        write_contained(
            out.as_path(),
            &format!("{}.kicad_sch", self.project_name),
            &sch,
        )?;

        let pcb = pcb::emit_pcb_with_layout(
            board,
            &self.project_name,
            Some(layout),
            None,
            None,
            &std::collections::HashMap::new(),
        );
        write_contained(
            out.as_path(),
            &format!("{}.kicad_pcb", self.project_name),
            &pcb,
        )?;

        let net = netlist::emit_netlist(board);
        write_contained(out.as_path(), &format!("{}.net", self.project_name), &net)?;

        emit_local_footprints(&out, board)?;
        emit_3d_models(&out, board)?;

        Ok(())
    }
}

/// Join one untrusted filename beneath a trusted output directory.
fn contained(root: &Path, component: &str) -> Result<std::path::PathBuf, BackendError> {
    safe_join(root, component)
        .map_err(|error| BackendError::EmitError(format!("path containment: {error}")))
}

/// Write 3D model files from embedded base64 data into the `models/` subdir.
fn emit_3d_models(out: &Path, board: &CompiledBoard) -> Result<(), BackendError> {
    let models_dir = out.join("models");
    for comp in &board.components {
        if let Some(ref data) = comp.meta.model_3d_data {
            if data.trim().is_empty() {
                continue;
            }
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(data)
                .map_err(|e| BackendError::EmitError(format!("base64 decode: {e}")))?;
            if bytes.is_empty() {
                continue;
            }
            fs::create_dir_all(&models_dir)?;
            let fallback = format!("{}.step", comp.refdes);
            let filename = comp
                .meta
                .model_3d
                .as_deref()
                .and_then(|p| Path::new(p).file_name())
                .and_then(|s| s.to_str())
                .unwrap_or(&fallback);
            let model_path = contained(&models_dir, filename)?;
            fs::write(model_path, &bytes)?;
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
            write_contained(&footprints_dir, &format!("{}.kicad_mod", fp_name), &content)?;
        }
    }

    if has_any {
        let fp_lib_table = project::emit_fp_lib_table("copperleaf");
        write_contained(out, "fp-lib-table", &fp_lib_table)?;
    }

    Ok(())
}

/// Reject invalid boards before writing any file.
fn ensure_valid(board: &CompiledBoard, layout: Option<&Layout>) -> Result<(), BackendError> {
    let findings = validate_compiled_board(board, layout);
    let errors: Vec<String> = findings
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| format!("[{}] {}", diagnostic.code, diagnostic.message))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(BackendError::EmitError(format!(
            "board validation failed:\n{}",
            errors.join("\n")
        )))
    }
}

/// Validate every filename that emission plans to write before any file is
/// created, so a traversal attempt cannot leave partial output behind.
fn validate_output_names(
    out: &Path,
    board: &CompiledBoard,
    project_name: &str,
) -> Result<(), BackendError> {
    for extension in ["kicad_pro", "kicad_sch", "kicad_pcb", "net"] {
        let _ = contained(out, &format!("{project_name}.{extension}"))?;
    }
    let _ = contained(out, "fp-lib-table")?;

    let footprints_dir = out.join("footprints");
    let models_dir = out.join("models");
    for comp in &board.components {
        let fp_ref = common::footprint_ref(comp);
        if let Some(fp_name) = fp_ref.strip_prefix("copperleaf:") {
            let _ = contained(&footprints_dir, &format!("{fp_name}.kicad_mod"))?;
        }
        if comp
            .meta
            .model_3d_data
            .as_deref()
            .is_some_and(|data| !data.trim().is_empty())
        {
            let filename = comp
                .meta
                .model_3d
                .as_deref()
                .and_then(|path| Path::new(path).file_name())
                .and_then(|name| name.to_str())
                .map(str::to_owned)
                .unwrap_or_else(|| format!("{}.step", comp.refdes));
            let _ = contained(&models_dir, &filename)?;
        }
    }
    Ok(())
}

/// Write one output file, refusing names that escape the destination.
fn write_contained(root: &Path, component: &str, contents: &str) -> Result<(), BackendError> {
    fs::write(contained(root, component)?, contents)?;
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
        const A: PinRef = PinRef("1");
        const B: PinRef = PinRef("2");
        fn new() -> Self {
            Self {
                pins: vec![
                    Pin::build("A").number("1").dio(),
                    Pin::build("B").number("2").dio(),
                ],
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

    fn empty_valid_board() -> CompiledBoard {
        CompiledBoard {
            components: vec![copperleaf::CompiledComponent::test("U1", vec![])],
            nets: vec![],
            connections: vec![],
            constraints: vec![],
            layout: vec![],
            width: 100.0,
            height: 80.0,
            stackup: copperleaf::Stackup::two_layer(),
            design_rules: copperleaf::DesignRules::default(),
        }
    }

    #[test]
    fn traversal_in_project_name_writes_nothing() {
        let board = empty_valid_board();
        let dir = tempfile::tempdir().unwrap();
        let result = KiCad::new()
            .with_project_name("../escape")
            .emit(dir.path(), &board);
        assert!(result.is_err(), "traversal must be rejected");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn absolute_project_name_writes_nothing() {
        let board = empty_valid_board();
        let dir = tempfile::tempdir().unwrap();
        let result = KiCad::new()
            .with_project_name("/tmp/escape")
            .emit(dir.path(), &board);
        assert!(result.is_err(), "absolute path must be rejected");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn manifest_footprint_traversal_writes_nothing() {
        let mut board = empty_valid_board();
        board.components[0].meta.footprint = Some("copperleaf:../escape".into());
        let dir = tempfile::tempdir().unwrap();
        let result = KiCad::new().emit(dir.path(), &board);
        assert!(result.is_err(), "footprint traversal must be rejected");
        assert_eq!(
            std::fs::read_dir(dir.path()).unwrap().count(),
            0,
            "no partial output may be written"
        );
    }

    #[test]
    fn invalid_board_cannot_be_emitted() {
        let board = CompiledBoard {
            components: vec![
                copperleaf::CompiledComponent::test("U1", vec![]),
                copperleaf::CompiledComponent::test("U1", vec![]),
            ],
            nets: vec![],
            connections: vec![],
            constraints: vec![],
            layout: vec![],
            width: 100.0,
            height: 80.0,
            stackup: copperleaf::Stackup::two_layer(),
            design_rules: copperleaf::DesignRules::default(),
        };
        let dir = tempfile::tempdir().unwrap();
        let result = KiCad::new().emit(dir.path(), &board);
        assert!(result.is_err(), "invalid board must not emit");
        assert_eq!(
            fs::read_dir(dir.path()).unwrap().count(),
            0,
            "validation must fail before any file is written"
        );
    }
}
