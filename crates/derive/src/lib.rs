//! Proc macros for the Copperleaf EDSL.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Expr, Lit, Meta, Type, parse_macro_input};

/// Derive `Block` for a struct that contains a `pins: Vec<Pin>` field.
///
/// Optionally annotate the struct with `#[component(symbol = "...")]` to set
/// the `kicad_symbol()` return value, and/or
/// `#[component(constraints(...))]` to specify default constraints.
///
/// # Examples
///
/// ```ignore
/// #[derive(Component)]
/// #[component(symbol = "Connector:PinHeader_2x5")]
/// struct JtagHeader {
///     pins: Vec<Pin>,
/// }
///
/// #[derive(Component)]
/// #[component(
///     symbol = "MCU:RP2354a",
///     constraints(
///         Constraint::Decoupling { values: vec![100.0.nf(), 1.0.uf()], per_pin: true },
///         Constraint::LengthMatch { group: "USB_D".into(), skew_ps: 200.0 },
///     )
/// )]
/// struct MyMcu {
///     pins: Vec<Pin>,
/// }
/// ```
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
    let is_vec_pin = is_vec_pin(ty);
    if !is_vec_pin {
        return compile_error("Field `pins` must be of type `Vec<Pin>`");
    }

    let (symbol, constraints_tokens) = parse_component_attrs(&input.attrs);
    let symbol_expr = match symbol {
        Some(s) => quote! { Some(#s) },
        None => quote! { None },
    };

    let constraints_expr = match constraints_tokens {
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

    let expanded = quote! {
        impl #impl_generics ::copperleaf_ir::Block for #name #ty_generics #where_clause {
            fn pins(&self) -> &[::copperleaf_ir::Pin] {
                &self.pins
            }

            fn constraints(&self) -> ::std::vec::Vec<::copperleaf_ir::Constraint> {
                #constraints_expr
            }

            fn kicad_symbol(&self) -> Option<&str> {
                #symbol_expr
            }
        }
    };

    TokenStream::from(expanded)
}
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

/// Parse `#[component(...)]` attributes, extracting:
/// - `symbol` (optional) — a string literal for the KiCad symbol reference.
/// - `constraints(...)` (optional) — a comma-separated list of `Constraint`
///   expressions. The token stream is passed through as-is into a `vec![]`
///   in the generated code, so users can write arbitrary Rust expressions
///   (including unit extension methods like `100.0.nf()`).
fn parse_component_attrs(attrs: &[Attribute]) -> (Option<String>, Option<proc_macro2::TokenStream>) {
    let mut symbol = None;
    let mut constraints_tokens = None;

    for attr in attrs {
        if !attr.path().is_ident("component") {
            continue;
        }
        let Meta::List(list) = &attr.meta else {
            continue;
        };

        // Parse the inner comma-separated Meta items (e.g. `symbol = "..."`,
        // `constraints(...)`) so we can inspect each one.
        let Ok(nested) = list
            .parse_args_with(syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated)
        else {
            continue;
        };

        for meta in nested {
            match meta {
                Meta::NameValue(nv) if nv.path.is_ident("symbol") => {
                    if let Expr::Lit(expr_lit) = &nv.value {
                        if let Lit::Str(s) = &expr_lit.lit {
                            symbol = Some(s.value());
                        }
                    }
                }
                Meta::List(list) if list.path.is_ident("constraints") => {
                    // Pass the inner token stream through — it'll be spliced
                    // directly into `vec![ ... ]` in the generated code.
                    constraints_tokens = Some(list.tokens);
                }
                _ => {}
            }
        }
    }

    (symbol, constraints_tokens)
}

fn compile_error(msg: &str) -> TokenStream {
    let msg = format!("derive(Component): {}", msg);
    TokenStream::from(quote! {
        compile_error!(#msg);
    })
}
