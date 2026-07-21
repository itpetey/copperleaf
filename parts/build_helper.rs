/// Shared build helper for parts crates.
///
/// The `build_component!` proc-macro reads `.toml` files at compile time via
/// `std::fs::read_to_string`, but cargo does not track those file reads
/// automatically.  This script emits `cargo:rerun-if-changed` directives for
/// every TOML file in the crate root so editing a part manifest triggers a
/// recompile.
///
/// Included by every `parts/*/build.rs`.

use std::{env, fs, path::Path};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dir = Path::new(&manifest_dir);

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "toml")
                && let Some(name) = path.file_name()
            {
                println!("cargo:rerun-if-changed={}", name.to_string_lossy());
            }
        }
    }
}
