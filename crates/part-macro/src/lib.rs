//! Proc-macro front-end for the Copperleaf component generator.
//!
//! `build_component!("definitions/part.toml")` reads a TOML definition at
//! compile time and expands into a documented Rust module for that component.

use proc_macro::TokenStream;
use std::path::PathBuf;
use std::str::FromStr as _;

/// Read a component TOML file and emit its generated Rust module.
///
/// The path is resolved relative to `CARGO_MANIFEST_DIR` of the crate invoking
/// the macro.
#[proc_macro]
pub fn build_component(input: TokenStream) -> TokenStream {
    let lit: syn::LitStr =
        syn::parse(input).expect("build_component! expects a single string literal path");

    let base = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("failed to get current directory"));
    let path = base.join(lit.value());

    let generated = copperleaf_part_codegen::generate_component_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to generate component from {}: {e}", path.display()));

    proc_macro2::TokenStream::from_str(&generated)
        .unwrap_or_else(|e| panic!("generated component code is invalid Rust: {e}"))
        .into()
}
