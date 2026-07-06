//! Proc macros for the Copperleaf EDSL.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Expr, Lit, Meta, Type, parse_macro_input};

/// Derive `Block` for a struct that contains a `pins: Vec<Pin>` field.
///
/// Optionally annotate the struct with `#[component(symbol = "...")]` to set
/// the `kicad_symbol()` return value.
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

    let symbol = parse_component_symbol(&input.attrs);
    let symbol_expr = match symbol {
        Some(s) => quote! { Some(#s) },
        None => quote! { None },
    };

    let expanded = quote! {
        impl #impl_generics ::copperleaf_ir::Block for #name #ty_generics #where_clause {
            fn pins(&self) -> &[::copperleaf_ir::Pin] {
                &self.pins
            }

            fn constraints(&self) -> ::std::vec::Vec<::copperleaf_ir::Constraint> {
                ::std::vec::Vec::new()
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

fn parse_component_symbol(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("component") {
            continue;
        }
        let Meta::List(list) = &attr.meta else {
            continue;
        };
        let nested = list
            .parse_args_with(syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated)
            .ok()?;
        for meta in nested {
            let Meta::NameValue(nv) = meta else {
                continue;
            };
            if !nv.path.is_ident("symbol") {
                continue;
            }
            let Expr::Lit(expr_lit) = &nv.value else {
                continue;
            };
            let Lit::Str(s) = &expr_lit.lit else {
                continue;
            };
            return Some(s.value());
        }
    }
    None
}

fn compile_error(msg: &str) -> TokenStream {
    let msg = format!("derive(Component): {}", msg);
    TokenStream::from(quote! {
        compile_error!(#msg);
    })
}
