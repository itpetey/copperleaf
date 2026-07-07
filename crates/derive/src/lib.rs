//! Proc macros for the Copperleaf EDSL.
//!
//! # `#[derive(Component)]`
//!
//! Generates a [`Block`] implementation for a struct with a `pins: Vec<Pin>` field.
//!
//! ## Attributes
//!
//! All attributes live inside `#[component(...)]`:
//!
//! | Attribute | Description |
//! |-----------|-------------|
//! | `symbol` | KiCad symbol reference, e.g. `"MCU:RPi"` |
//! | `symbol_lib_path` | Explicit path to the `.kicad_sym` file (resolved relative to `CARGO_MANIFEST_DIR`) |
//! | `footprint` | Explicit footprint override; takes precedence over the one auto-resolved from the symbol library |
//! | `constraints(...)` | Default constraints for this component |
//!
//! ## Compile-time symbol resolution
//!
//! When `symbol` is specified, the macro searches for the matching `.kicad_sym`
//! file at compile time in the following order:
//!
//! 1. The path given by `symbol_lib_path` (if set).
//! 2. Directories listed in the `KICAD_SYMBOL_DIR` environment variable
//!    (colon-separated on Unix, semicolon-separated on Windows).
//! 3. The project root (`CARGO_MANIFEST_DIR`).
//! 4. Standard KiCad installation paths for the current platform.
//!
//! If found, the symbol's `Footprint` property is extracted and baked into
//! the generated `kicad_footprint()` method. If the symbol file cannot be
//! located, a compile error is emitted. If the file is found but the symbol
//! has no `Footprint` property, a warning is emitted.
//!
//! The explicit `footprint` attribute always takes precedence over the
//! auto-resolved value from the library.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Expr, Ident, Lit, Meta, Type, parse_macro_input};

// ─────────────────────────────────────────────────────────────────────────────
// Derive entry point
// ─────────────────────────────────────────────────────────────────────────────

/// Derive `Block` for a struct that contains a `pins: Vec<Pin>` field.
#[proc_macro_derive(Component, attributes(component))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let Data::Struct(data) = &input.data else {
        return compile_error("Component can only be derived for structs");
    };

    let fields: Vec<_> = match &data.fields {
        syn::Fields::Named(f) => f.named.iter().collect(),
        syn::Fields::Unnamed(_) => {
            return compile_error("Component does not support tuple structs");
        }
        syn::Fields::Unit => {
            return compile_error("Component does not support unit structs");
        }
    };

    let pins_field = fields
        .iter()
        .find(|f| f.ident.as_ref().map(|i| i == "pins").unwrap_or(false));

    let Some(pins_field) = pins_field else {
        return compile_error("Component struct must have a field named `pins`");
    };

    let ty = &pins_field.ty;
    if !is_vec_pin(ty) {
        return compile_error("Field `pins` must be of type `Vec<Pin>`");
    }

    // ── Parse attributes and resolve symbol library at compile time ──────

    let mut errors: Vec<String> = Vec::new();
    let attrs = parse_component_attrs(&input.attrs, name, &mut errors);

    // ── Generate the Block implementation ────────────────────────────────

    let symbol_expr = match &attrs.symbol {
        Some(s) => quote! { ::core::option::Option::Some(#s) },
        None => quote! { ::core::option::Option::None },
    };

    let symbol_lib_path_expr = match &attrs.symbol_lib_path {
        Some(p) => quote! { ::core::option::Option::Some(#p) },
        None => quote! { ::core::option::Option::None },
    };

    let footprint_expr = match &attrs.footprint {
        Some(fp) => quote! { ::core::option::Option::Some(#fp) },
        None => quote! { ::core::option::Option::None },
    };

    let constraints_expr = match &attrs.constraints_tokens {
        Some(tokens) => {
            quote! {
                {
                    use ::copperleaf_core::UnitExt;
                    ::std::vec![ #tokens ]
                }
            }
        }
        None => quote! { ::std::vec::Vec::new() },
    };

    let block_impl = quote! {
        impl #impl_generics ::copperleaf_ir::Block for #name #ty_generics #where_clause {
            fn pins(&self) -> &[::copperleaf_ir::Pin] {
                &self.pins
            }

            fn constraints(&self) -> ::std::vec::Vec<::copperleaf_ir::Constraint> {
                #constraints_expr
            }

            fn kicad_symbol(&self) -> ::core::option::Option<&str> {
                #symbol_expr
            }

            fn kicad_symbol_lib_path(&self) -> ::core::option::Option<&str> {
                #symbol_lib_path_expr
            }

            fn kicad_footprint(&self) -> ::core::option::Option<&str> {
                #footprint_expr
            }
        }
    };

    // ── Emit compile_error! for any errors collected during resolution ───

    let error_tokens = errors
        .iter()
        .fold(proc_macro2::TokenStream::new(), |mut acc, msg| {
            acc.extend(quote! { compile_error!(#msg); });
            acc
        });

    TokenStream::from(quote! {
        #block_impl
        #error_tokens
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Attribute parsing
// ─────────────────────────────────────────────────────────────────────────────

/// Parsed `#[component(...)]` attributes.
struct ComponentAttrs {
    /// `symbol = "Lib:Name"` — the KiCad symbol reference.
    symbol: Option<String>,
    /// Absolute path to the `.kicad_sym` file, resolved at compile time.
    symbol_lib_path: Option<String>,
    /// `footprint = "Pkg:Footprint"` — explicit override (auto-resolved if absent).
    footprint: Option<String>,
    /// Token stream inside `constraints(...)`.
    constraints_tokens: Option<proc_macro2::TokenStream>,
}

/// Parse `#[component(...)]` attributes, resolving the symbol library at
/// compile time when `symbol` is specified. Errors are appended to `errors`.
fn parse_component_attrs(
    attrs: &[Attribute],
    component_name: &Ident,
    errors: &mut Vec<String>,
) -> ComponentAttrs {
    // Step 1: extract raw string values from the attribute.
    let mut raw_symbol: Option<String> = None;
    let mut raw_lib_path: Option<String> = None;
    let mut raw_footprint: Option<String> = None;
    let mut constraints_tokens: Option<proc_macro2::TokenStream> = None;

    for attr in attrs {
        if !attr.path().is_ident("component") {
            continue;
        }
        let Meta::List(list) = &attr.meta else {
            continue;
        };

        let Ok(nested) = list
            .parse_args_with(syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated)
        else {
            continue;
        };

        for meta in nested {
            match meta {
                Meta::NameValue(nv) if nv.path.is_ident("symbol") => {
                    if let Expr::Lit(expr_lit) = &nv.value
                        && let Lit::Str(s) = &expr_lit.lit
                    {
                        raw_symbol = Some(s.value());
                    }
                }
                Meta::NameValue(nv) if nv.path.is_ident("symbol_lib_path") => {
                    if let Expr::Lit(expr_lit) = &nv.value
                        && let Lit::Str(s) = &expr_lit.lit
                    {
                        raw_lib_path = Some(s.value());
                    }
                }
                Meta::NameValue(nv) if nv.path.is_ident("footprint") => {
                    if let Expr::Lit(expr_lit) = &nv.value
                        && let Lit::Str(s) = &expr_lit.lit
                    {
                        raw_footprint = Some(s.value());
                    }
                }
                Meta::List(list) if list.path.is_ident("constraints") => {
                    constraints_tokens = Some(list.tokens);
                }
                _ => {}
            }
        }
    }

    // Step 2: if `symbol` is present, try to auto-resolve the library and
    // extract the footprint.
    let (resolved_lib_path, auto_footprint) = if let Some(ref symbol) = raw_symbol {
        let (lib_name, sym_name) = split_symbol_ref(symbol);
        resolve_symbol_at_compile_time(
            lib_name,
            sym_name,
            raw_lib_path.as_deref(),
            component_name,
            errors,
        )
    } else {
        (None, None)
    };

    // The explicit `footprint` attribute takes precedence over auto-resolved.
    let footprint = raw_footprint.or(auto_footprint);

    ComponentAttrs {
        symbol: raw_symbol,
        symbol_lib_path: resolved_lib_path,
        footprint,
        constraints_tokens,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Symbol reference helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Split a symbol reference like `"MCU_RaspberryPi:RP2354A"` into
/// `("MCU_RaspberryPi", "RP2354A")`. If there is no colon, the whole string
/// is treated as both library name and symbol name.
fn split_symbol_ref(symbol: &str) -> (&str, &str) {
    if let Some((lib, sym)) = symbol.split_once(':') {
        (lib, sym)
    } else {
        (symbol, symbol)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Compile-time .kicad_sym resolution
// ─────────────────────────────────────────────────────────────────────────────

/// Try to locate and parse the `.kicad_sym` file for the given library/symbol
/// pair at compile time.
///
/// Returns `(resolved_lib_path, auto_footprint)`.
///
/// * `resolved_lib_path` — the absolute path to the `.kicad_sym` file (so the
///   runtime `resolve_symbols` can read pin positions).
/// * `auto_footprint` — the `Footprint` property value parsed from the symbol,
///   if present.
fn resolve_symbol_at_compile_time(
    lib_name: &str,
    sym_name: &str,
    explicit_path: Option<&str>,
    component_name: &Ident,
    errors: &mut Vec<String>,
) -> (Option<String>, Option<String>) {
    let file_path = match find_sym_file(lib_name, explicit_path, component_name, errors) {
        Some(p) => p,
        None => return (None, None),
    };

    let footprint = extract_footprint_from_file(&file_path, sym_name, component_name, errors);

    (Some(file_path), footprint)
}

/// Search for a `.kicad_sym` file matching `lib_name` in various locations.
///
/// Search order:
/// 1. Explicit `symbol_lib_path` attribute (resolved relative to CARGO_MANIFEST_DIR).
/// 2. `KICAD_SYMBOL_DIR` environment variable.
/// 3. `CARGO_MANIFEST_DIR` (project root).
/// 4. Platform-specific standard KiCad installation paths.
fn find_sym_file(
    lib_name: &str,
    explicit_path: Option<&str>,
    component_name: &Ident,
    errors: &mut Vec<String>,
) -> Option<String> {
    let file_name = format!("{}.kicad_sym", lib_name);

    // 1. Explicit path from attribute.
    if let Some(path) = explicit_path {
        let p = resolve_path(path);
        if p.is_file() {
            return Some(p.to_string_lossy().to_string());
        }
        errors.push(format!(
            "symbol_lib_path \"{}\" not found for component `{}` (resolved to `{}`)",
            path,
            component_name,
            p.display(),
        ));
        return None;
    }

    // 2. KICAD_SYMBOL_DIR environment variable.
    if let Ok(dirs) = env::var("KICAD_SYMBOL_DIR") {
        for dir in env::split_paths(&dirs) {
            let candidate = dir.join(&file_name);
            if candidate.is_file() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }

    // 3. CARGO_MANIFEST_DIR (project root).
    if let Ok(manifest) = env::var("CARGO_MANIFEST_DIR") {
        let candidate = Path::new(&manifest).join(&file_name);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }

    // 4. Platform-specific standard KiCad paths.
    for dir in default_kicad_sym_dirs() {
        let candidate = dir.join(&file_name);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }

    // Not found anywhere. When no explicit `symbol_lib_path` was provided,
    // this is a soft failure — we warn and continue without auto-resolution.
    // The runtime `resolve_symbols` may still pick it up via `--symbol-lib`.
    eprintln!(
        "warning: symbol library `{}.kicad_sym` not found for component `{}`.\n\
         Pin positions and footprint will not be auto-resolved at compile time.\n\
         Either place the file in the project root, set `KICAD_SYMBOL_DIR`, \
         or add `symbol_lib_path` to the `#[component(...)]` attribute.",
        lib_name, component_name,
    );
    None
}

/// Return a list of well-known KiCad symbol directories for the current platform.
fn default_kicad_sym_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // macOS (KiCad app bundle)
    if cfg!(target_os = "macos") {
        dirs.push(PathBuf::from(
            "/Applications/KiCad.app/Contents/SharedSupport/symbols",
        ));
        dirs.push(PathBuf::from(
            "/opt/homebrew/Caskroom/kicad/10.0.4/KiCad/KiCad.app/Contents/SharedSupport/symbols",
        ));
        if let Ok(home) = env::var("HOME") {
            dirs.push(
                Path::new(&home)
                    .join("Library")
                    .join("Application Support")
                    .join("kicad")
                    .join("symbols"),
            );
        }
    }

    // Linux
    if cfg!(target_os = "linux") {
        dirs.push(PathBuf::from("/usr/share/kicad/symbols"));
        dirs.push(PathBuf::from("/usr/local/share/kicad/symbols"));
        if let Ok(home) = env::var("HOME") {
            for ver in ["8.0", "7.0"] {
                dirs.push(
                    Path::new(&home)
                        .join(".local")
                        .join("share")
                        .join("kicad")
                        .join(ver)
                        .join("symbols"),
                );
            }
        }
    }

    // Windows
    if cfg!(target_os = "windows") {
        dirs.push(PathBuf::from(r"C:\Program Files\KiCad\share\kicad\symbols"));
        if let Ok(program_files) = env::var("ProgramFiles") {
            dirs.push(
                Path::new(&program_files)
                    .join("KiCad")
                    .join("share")
                    .join("kicad")
                    .join("symbols"),
            );
        }
        if let Ok(appdata) = env::var("APPDATA") {
            dirs.push(Path::new(&appdata).join("kicad").join("symbols"));
        }
    }

    dirs
}

/// Resolve a potentially-relative path against `CARGO_MANIFEST_DIR`.
fn resolve_path(path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else if let Ok(manifest) = env::var("CARGO_MANIFEST_DIR") {
        Path::new(&manifest).join(p)
    } else {
        p.to_path_buf()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// .kicad_sym footprint extraction
// ─────────────────────────────────────────────────────────────────────────────

/// Extract the `Footprint` property value for a symbol named `sym_name` from
/// the given `.kicad_sym` file.
///
/// Uses simple paren-matching to find the symbol definition, then searches for
/// `(property "Footprint" "VALUE" ...)` inside it.
///
/// Emits a compile warning (via eprintln) if the symbol is found but has no
/// Footprint property.
fn extract_footprint_from_file(
    file_path: &str,
    sym_name: &str,
    component_name: &Ident,
    errors: &mut Vec<String>,
) -> Option<String> {
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            errors.push(format!("cannot read symbol library `{}`: {}", file_path, e,));
            return None;
        }
    };

    // Find the opening of `(symbol "sym_name" ...)`.
    let search_for = format!("(symbol \"{}\"", sym_name);
    let Some(start) = content.find(&search_for) else {
        errors.push(format!(
            "symbol `{}` not found in library `{}`",
            sym_name, file_path,
        ));
        return None;
    };

    // Track paren nesting to find the matching closing paren.
    let mut depth = 0u32;
    let mut in_string = false;
    let mut end = content.len();
    for (i, ch) in content[start..].char_indices() {
        let abs_i = start + i;
        if ch == '"' {
            in_string = !in_string;
        }
        if !in_string {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        end = abs_i + 1; // include the )
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    let symbol_body = &content[start..end];

    // Search for (property "Footprint" "VALUE") inside the symbol body.
    let fp_search = r#"(property "Footprint" ""#;
    if let Some(fp_start) = symbol_body.find(fp_search) {
        let after_key = &symbol_body[fp_start + fp_search.len()..];
        if let Some(fp_end) = after_key.find('"') {
            let value = &after_key[..fp_end];
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }

    // Symbol found but no Footprint property.
    eprintln!(
        "warning: symbol `{}` in `{}` (component `{}`) has no `Footprint` property.\n\
         Consider adding one or specifying `footprint` explicitly in `#[component(...)]`.",
        sym_name, file_path, component_name,
    );
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Type-checking helpers
// ─────────────────────────────────────────────────────────────────────────────

fn is_vec_pin(ty: &Type) -> bool {
    let Type::Path(type_path) = ty else {
        return false;
    };
    let segments: Vec<_> = type_path.path.segments.iter().collect();
    if segments.len() != 1 || segments[0].ident != "Vec" {
        return false;
    }
    let syn::PathArguments::AngleBracketed(args) = &segments[0].arguments else {
        return false;
    };
    if args.args.len() != 1 {
        return false;
    }
    let syn::GenericArgument::Type(Type::Path(inner)) = &args.args[0] else {
        return false;
    };
    let inner_segments: Vec<_> = inner.path.segments.iter().collect();
    inner_segments.len() == 1 && inner_segments[0].ident == "Pin"
}

// ─────────────────────────────────────────────────────────────────────────────
// Error helpers
// ─────────────────────────────────────────────────────────────────────────────

fn compile_error(msg: &str) -> TokenStream {
    let msg = format!("derive(Component): {}", msg);
    TokenStream::from(quote! {
        compile_error!(#msg);
    })
}
