//! This crate only contains the `#[derive(Doc)]` proc-macro for Avocado.
//! For documentation, please see the main [`avocado`][1] crate.
//!
//! [1]: https://docs.rs/avocado

#![crate_type = "proc-macro"]
#![doc(html_root_url = "https://docs.rs/avocado_derive/0.5.0")]
#![deny(missing_debug_implementations, missing_copy_implementations,
        trivial_casts, trivial_numeric_casts,
        unsafe_code,
        unstable_features,
        unused_import_braces, unused_qualifications,
        /* missing_docs (https://github.com/rust-lang/rust/issues/42008) */)]
#![allow(clippy::single_match, clippy::match_same_arms, clippy::match_ref_pats,
         clippy::clone_on_ref_ptr, clippy::needless_pass_by_value)]
#![deny(clippy::wrong_pub_self_convention, clippy::used_underscore_binding,
        clippy::stutter, clippy::similar_names, clippy::pub_enum_variant_names,
        clippy::missing_docs_in_private_items,
        clippy::non_ascii_literal, clippy::unicode_not_nfc,
        clippy::result_unwrap_used, clippy::option_unwrap_used,
        clippy::option_map_unwrap_or_else, clippy::option_map_unwrap_or, clippy::filter_map,
        clippy::shadow_unrelated, clippy::shadow_reuse, clippy::shadow_same,
        clippy::int_plus_one, clippy::string_add_assign, clippy::if_not_else,
        clippy::invalid_upcast_comparisons,
        clippy::cast_precision_loss,
        clippy::cast_possible_wrap, clippy::cast_possible_truncation,
        clippy::mutex_integer, clippy::mut_mut, clippy::items_after_statements,
        clippy::print_stdout, clippy::mem_forget, clippy::maybe_infinite_iter)]

#[macro_use]
extern crate quote;
extern crate syn;
extern crate proc_macro;
extern crate proc_macro2;

mod meta;
mod case;
mod error;

use proc_macro::TokenStream;
use syn::{ DeriveInput, Data, Fields, Type, Attribute };
use self::{
    meta::*,
    case::RenameRule,
    error::{ Result, Error },
};

/// The top-level entry point of this proc-macro. Only here to be exported
/// and to handle `Result::Err` return values by `panic!()`ing.
#[proc_macro_derive(Doc, attributes(avocado))]
pub fn derive_avocado_doc(input: TokenStream) -> TokenStream {
    impl_avocado_doc(input).unwrap_or_else(|error| panic!("{}", error))
}

/// Implements `Doc` for the specified type.
fn impl_avocado_doc(input: TokenStream) -> Result<TokenStream> {
    let parsed_ast: DeriveInput = syn::parse(input)?;
    let ty_ident = parsed_ast.ident;
    let ty_name = serde_renamed_ident(&parsed_ast.attrs, ty_ident.to_string())?;

    match parsed_ast.data {
        Data::Struct(s) => {
            let id_ty = type_of_id_field(s.fields, &parsed_ast.attrs)?;
            let ast = quote! {
                impl ::avocado::doc::Doc for #ty_ident {
                    const NAME: &'static str = #ty_name;
                    type Id = #id_ty;
                }
            };
            Ok(ast.into())
        },
        _ => Err(Error::new(
            "Only a `struct` can be a top-level `Doc`; consider wrapping this type in a struct"
        )),
    }
}

/// Returns the collection name based on the the type name,
/// taking Serde renaming into account as well.
fn serde_renamed_ident(attrs: &[Attribute], ident: String) -> Result<String> {
    serde_name_value(attrs, "rename")?
        .as_ref()
        .map_or_else(|| Ok(ident), value_as_str)
}

/// Returns the declared type of the field which serializes as `_id`.
/// If there's no such field, returns an `Err`.
fn type_of_id_field(fields: Fields, attrs: &[Attribute]) -> Result<Type> {
    let named = match fields {
        Fields::Named(fields) => fields.named,
        _ => return Err(Error::new(
            "A `Doc` must be a struct with named fields"
        )),
    };
    let rename_attr = serde_name_value(attrs, "rename_all")?;
    let rename_rule: Option<RenameRule> = match rename_attr {
        None => None,
        Some(kv) => Some(value_as_str(&kv)?.parse()?)
    };

    for field in named {
        let ty = field.ty;
        let attrs = field.attrs;

        // The field isn't inspected if it's never serialized or deserialized.
        if has_serde_word(&attrs, "skip")? {
            continue;
        }
        if
            has_serde_word(&attrs, "skip_serializing")?
            &&
            has_serde_word(&attrs, "skip_deserializing")?
        {
            continue;
        }

        // The original identifier of the field name.
        let ident = match field.ident {
            Some(ident) => ident,
            None => continue,
        };

        // The field name as a string, with the `#[serde(rename_all = "...)]`
        // rule applied to it if present; otherwise, just the original name.
        let rename_all_ident = rename_rule.map_or_else(
            || ident.to_string(),
            |rule| rule.apply_to_field(ident.to_string()),
        );

        // The final field name is the exact name specified in the immediate
        // `#[serde(rename = "...")]` attribute applied directly to the field,
        // or the potentially-`rename_all`'d name, if the former doesn't exist.
        let field_name = serde_renamed_ident(&attrs, rename_all_ident)?;

        if field_name == "_id" {
            return Ok(ty);
        }
    }

    Err(Error::new("A `Doc` must contain a field (de)serialized as `_id`"))
}
