//! Parts-creation CLI for Copperleaf.
//!
//! Provides `new` and `update` commands for creating and enriching part TOML
//! definitions from KiCad symbols and footprints.

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use copperleaf::{Diagnostic, Severity};

mod kindmap;
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
    /// Component title.
    #[arg(long)]
    title: Option<String>,
    /// Component description.
    #[arg(long)]
    description: Option<String>,
    /// Create or use a vendor parts crate.
    #[arg(long)]
    crate_: Option<String>,
}

#[derive(Parser)]
#[command(group = clap::ArgGroup::new("source").required(true).multiple(false))]
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
}

#[derive(Subcommand)]
enum Command {
    /// Create a new part TOML from a source.
    New(NewArgs),
    /// Update an existing part TOML from a source.
    Update(UpdateArgs),
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

pub fn datasheet_stub(_path: &str) -> CliError {
    CliError::Diagnostic(Diagnostic {
        code: "CLI:DATASHEET_STUB".into(),
        severity: Severity::Error,
        message: "LLM-assisted datasheet parsing is a future capability".into(),
        entities: vec![],
        hint: Some("Use --symbol or --footprint instead".into()),
    })
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
    }
}
