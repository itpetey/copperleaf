use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <definitions-dir> <output-file>", args[0]);
        process::exit(1);
    }

    if let Err(e) = copperleaf_part_codegen::generate(&args[1], &args[2]) {
        eprintln!("codegen failed: {e}");
        process::exit(1);
    }
}
