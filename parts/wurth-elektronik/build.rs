//! Rebuild this crate when any part TOML changes.
fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "toml")
                && let Some(name) = path.file_name()
            {
                println!("cargo:rerun-if-changed={}", name.to_string_lossy());
            }
        }
    }
}
