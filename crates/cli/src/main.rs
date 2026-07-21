//! Parts-creation CLI for Copperleaf.
//!
//! Provides `new`, `update`, and `generate` commands for creating, enriching,
//! and exporting part definitions.

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use copperleaf::{Diagnostic, Severity};

mod generate;
mod kindmap;
mod llm;
mod manifest;
mod new;
mod update;
mod vendor;

/// CLI error type wrapping either a structured diagnostic or a low-level I/O error.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("{0:?}")]
    Diagnostic(Diagnostic),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("codegen error: {0}")]
    Codegen(#[from] copperleaf_part_codegen::CodegenError),
    #[error("parse error: {0}")]
    Parse(#[from] copperleaf_backend_kicad::ParseError),
    #[error("emit error: {0}")]
    Emit(#[from] copperleaf_backend_kicad::fp_emitter::EmitError),
}

#[derive(Parser)]
#[command(group = clap::ArgGroup::new("source").required(true).multiple(false))]
struct NewArgs {
    /// KiCad symbol library file.
    #[arg(long, group = "source")]
    symbol: Option<String>,
    /// KiCad footprint file or .pretty directory.
    #[arg(long, group = "source")]
    footprint: Option<String>,
    /// Datasheet PDF file (not yet supported).
    #[arg(long, group = "source")]
    datasheet: Option<String>,
    /// Library identifier within the source file.
    #[arg(long)]
    lib_id: Option<String>,
    /// Output file path.
    #[arg(long, short)]
    out: Option<String>,
    /// TOML file overriding the built-in pin type → kind map.
    #[arg(long)]
    kind_map: Option<String>,
    /// Default kind for unrecognised pin types.
    #[arg(long, default_value = "dio")]
    default_kind: String,
    /// Path to a 3D model (.step) file for the footprint.
    #[arg(long)]
    model_3d: Option<String>,
    /// Component title.
    #[arg(long)]
    title: Option<String>,
    /// Component description.
    #[arg(long)]
    description: Option<String>,
    /// LLM model for datasheet processing (provider/model format).
    #[arg(long, default_value = "opencode/big-pickle")]
    model: String,
    /// Create or use a vendor parts crate.
    #[arg(long)]
    crate_: Option<String>,
}

#[derive(Parser)]
#[command(group = clap::ArgGroup::new("source").multiple(false))]
struct UpdateArgs {
    /// Existing part TOML file.
    part_toml: String,
    /// KiCad symbol library file.
    #[arg(long, group = "source")]
    symbol: Option<String>,
    /// KiCad footprint file or .pretty directory.
    #[arg(long, group = "source")]
    footprint: Option<String>,
    /// Datasheet PDF file (not yet supported).
    #[arg(long, group = "source")]
    datasheet: Option<String>,
    /// Library identifier within the source file.
    #[arg(long)]
    lib_id: Option<String>,
    /// Output file path (defaults to overwriting the input).
    #[arg(long, short)]
    out: Option<String>,
    /// TOML file overriding the built-in pin type → kind map.
    #[arg(long)]
    kind_map: Option<String>,
    /// Default kind for unrecognised pin types.
    #[arg(long, default_value = "dio")]
    default_kind: String,
    /// Path to a 3D model (.step) file for the footprint.
    #[arg(long)]
    model_3d: Option<String>,
    /// LLM model for datasheet processing (provider/model format).
    #[arg(long, default_value = "opencode/big-pickle")]
    model: String,
}

/// Arguments for `generate footprint`.
#[derive(Parser)]
struct GenerateFootprintArgs {
    /// Part TOML file.
    part_toml: String,
    /// Output file path (defaults to <lib_id>.kicad_mod beside the TOML).
    #[arg(long, short)]
    out: Option<String>,
}

/// Arguments for `generate symbol`.
#[derive(Parser)]
struct GenerateSymbolArgs {
    /// Part TOML file.
    part_toml: String,
    /// Output file path (defaults to <lib_id>.kicad_sym beside the TOML).
    #[arg(long, short)]
    out: Option<String>,
}

#[derive(Subcommand)]
enum GenerateCommand {
    /// Generate a .kicad_mod footprint file from a part TOML.
    Footprint(GenerateFootprintArgs),
    /// Generate a .kicad_sym symbol library file from a part TOML.
    Symbol(GenerateSymbolArgs),
}

#[derive(Subcommand)]
enum Command {
    /// Create a new part TOML from a source.
    New(NewArgs),
    /// Update an existing part TOML from a source.
    Update(UpdateArgs),
    /// Generate output files from a part TOML.
    #[command(subcommand)]
    Generate(GenerateCommand),
}

#[derive(Parser)]
#[command(name = "copperleaf")]
#[command(version, about = "Parts-creation CLI for Copperleaf")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => Ok(()),
        Err(CliError::Diagnostic(d)) => {
            print_diagnostic(&d);
            bail!(CliError::Diagnostic(d))
        }
        Err(e) => bail!(e),
    }
}

pub fn print_diagnostic(d: &Diagnostic) {
    match d.severity {
        Severity::Info => eprint!("info"),
        Severity::Warning => eprint!("warning"),
        Severity::Error => eprint!("error"),
    }
    eprint!("[{}]", d.code);
    if !d.entities.is_empty() {
        eprint!(" {}", d.entities.join(", "));
    }
    eprintln!(": {}", d.message);
    if let Some(hint) = &d.hint {
        eprintln!("  hint: {}", hint);
    }
}

fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Command::New(args) => new::run(args),
        Command::Update(args) => update::run(args),
        Command::Generate(cmd) => match cmd {
            GenerateCommand::Footprint(args) => generate::footprint(args),
            GenerateCommand::Symbol(args) => generate::symbol(args),
        },
    }
}
