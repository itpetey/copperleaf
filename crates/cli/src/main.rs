use std::{fs, path::PathBuf};

use clap::{Parser, Subcommand};

use copperleaf::{Design, Role, backend_kicad, erc_voltage_pin_to_net, synthesize_decoupling};

#[derive(Parser)]
#[command(name = "cl", version, about = "Copperleaf circuit design CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// ERC verification
    Verify {
        /// Design JSON file
        design: PathBuf,
    },
    /// Export KiCad files (netlist, schematic, PCB)
    Export {
        /// Design JSON file
        design: PathBuf,
        /// Output directory (defaults to current directory)
        #[arg(short = 'o', long, default_value = ".")]
        output: PathBuf,
        /// Path to a KiCad symbol library (.kicad_sym) used to resolve pin positions
        #[arg(long = "symbol-lib")]
        symbol_lib: Option<PathBuf>,
    },
    /// Synthesize decoupling capacitors
    Decouple {
        /// Design JSON file
        design: PathBuf,
    },
    /// Generate a text report
    Report {
        /// Design JSON file
        design: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Verify { design } => cmd_verify(&design),
        Commands::Export {
            design,
            output,
            symbol_lib,
        } => cmd_export(&design, &output, symbol_lib.as_ref()),
        Commands::Decouple { design } => cmd_decouple(&design),
        Commands::Report { design } => cmd_report(&design),
    }
}

fn cmd_export(design: &PathBuf, out_dir: &PathBuf, symbol_lib: Option<&PathBuf>) {
    let mut d = load_design(design);
    let _ = fs::create_dir_all(out_dir);

    let base = design
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("design")
        .to_string();

    let write_file = |ext: &str, content: &str| {
        let out_path = out_dir.join(format!("{}.{}", base, ext));
        if let Err(e) = fs::write(&out_path, content) {
            eprintln!("Error writing {}: {}", out_path.display(), e);
            std::process::exit(1);
        }
        println!("Wrote {}", out_path.display());
    };

    // Resolve symbols using component-level library paths, falling back to
    // --symbol-lib if provided (which may be None if not specified).
    let fallback = symbol_lib.and_then(|p| p.to_str());
    backend_kicad::resolve_symbols(&mut d, fallback);
    for diag in &d.diagnostics {
        eprintln!("[{:?}] {} — {}", diag.severity, diag.code, diag.message);
    }

    write_file("net", &backend_kicad::emit_netlist(&d));
    write_file("kicad_sch", &backend_kicad::emit_schematic(&d));
    write_file("kicad_pcb", &backend_kicad::emit_pcb(&d));
    write_file("kicad_pro", &backend_kicad::emit_project(&base));
}

fn cmd_decouple(design: &PathBuf) {
    let d = load_design(design);
    let result = synthesize_decoupling(&d);
    if result.caps.is_empty() {
        println!("[Info] DECOUPLE: no capacitors placed");
    } else {
        for cap in &result.caps {
            println!(
                "  {}: {} F on {} (from {}.{})",
                cap.refdes,
                cap.value.as_base(),
                cap.net,
                cap.source_component,
                cap.source_pin,
            );
        }
    }
    for diag in &result.diagnostics {
        println!("[{:?}] {} — {}", diag.severity, diag.code, diag.message);
    }
}

fn cmd_report(design: &PathBuf) {
    let d = load_design(design);
    println!("{}", copperleaf::report(&d));
}

fn cmd_verify(design: &PathBuf) {
    let d = load_design(design);
    let mut issues = false;
    for c in &d.components {
        for pin in &c.pins {
            if !matches!(pin.role, Role::PowerIn) {
                continue;
            }
            for net_name in d.nets_of_pin(&c.refdes, &pin.name) {
                if let Some(net) = d.nets.iter().find(|n| n.name == net_name)
                    && let Some(diag) = erc_voltage_pin_to_net(net, pin)
                {
                    println!("[{:?}] {} — {}", diag.severity, diag.code, diag.message);
                    issues = true;
                }
            }
        }
    }
    if !issues {
        println!("[Info] ERC:OK — no overvoltage detected");
    }
}

fn load_design(path: &PathBuf) -> Design {
    let data = match fs::read_to_string(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error reading design file '{}': {}", path.display(), e);
            std::process::exit(1);
        }
    };
    match serde_json::from_str(&data) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error parsing design file '{}': {}", path.display(), e);
            std::process::exit(1);
        }
    }
}
