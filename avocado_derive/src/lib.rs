//! This crate only contains the `#[derive(Doc)]` proc-macro for Avocado.
//! For documentation, please see the main [`avocado`][1] crate.
//!
//! [1]: https://docs.rs/avocado

#![crate_type = "proc-macro"]
#![doc(html_root_url = "https://docs.rs/avocado_derive/0.2.0")]
#![deny(missing_debug_implementations, missing_copy_implementations,
        trivial_casts, trivial_numeric_casts,
        unsafe_code,
        unstable_features,
        anonymous_parameters, bare_trait_objects,
        variant_size_differences,
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
#[macro_use]
extern crate syn;
extern crate proc_macro;
extern crate proc_macro2;

#[macro_use]
mod error;
mod meta;
mod attr;
mod case;
mod index;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{
    DeriveInput, Data, Generics, Fields, Ident,
    Type, Attribute, TypePath, Path, PathSegment,
};
use self::{
    meta::*,
    case::RenameRule,
    index::Spec,
    error::{ Result, err_msg },
};

/// The top-level entry point of this proc-macro. Only here to be exported
/// and to handle `Result::Err` return values by `panic!()`ing.
#[proc_macro_derive(Doc, attributes(avocado, index, id_type))]
pub fn derive_avocado_doc(input: TokenStream) -> TokenStream {
    impl_avocado_doc(input).unwrap_or_else(|error| panic!("{}", error))
}

/// Implements `Doc` for the specified type.
fn impl_avocado_doc(input: TokenStream) -> Result<TokenStream> {
    let parsed_ast: DeriveInput = syn::parse(input)?;
    let ty = parsed_ast.ident;
    let generics = parsed_ast.generics;
    let ty_name = serde_renamed_ident(&parsed_ast.attrs, ty.to_string())?;
    let (impl_gen, ty_gen, where_cls) = generics.split_for_impl();
    let id_ty = raw_id_type(&parsed_ast.attrs)?;
    let indexes = Spec::from_attributes(&parsed_ast.attrs)?;

    ensure_only_lifetime_params(&generics)?;

    match parsed_ast.data {
        Data::Struct(s) => {
            ensure_id_exists_and_unique(s.fields, &parsed_ast.attrs)?;

            let ast = quote! {
                impl #impl_gen ::avocado::doc::Doc for #ty #ty_gen #where_cls {
                    const NAME: &'static str = #ty_name;

                    type Id = #id_ty;

                    fn indexes() -> Vec<::avocado::prelude::IndexModel> {
                        vec![
                            #(#indexes),*
                        ]
                    }
                }
            };
            Ok(ast.into())
        },
        _ => err_msg(
            "only a `struct` can be a top-level `Doc`; consider wrapping this type in a struct"
        ),
    }
}

/// Returns the collection name based on the the type name,
/// taking Serde renaming into account as well.
fn serde_renamed_ident(attrs: &[Attribute], ident: String) -> Result<String> {
    serde_name_value(attrs, "rename")?
        .as_ref()
        .map_or(Ok(ident), value_as_str)
}

/// Returns `true` iff the field has either `#[serde]` attribute `skip` or
/// both `skip_serializing` and `skip_deserializing`.
fn field_is_always_skipped(attrs: &[Attribute]) -> Result<bool> {
    Ok(
        has_serde_word(attrs, "skip")? || (
            has_serde_word(attrs, "skip_serializing")?
            &&
            has_serde_word(attrs, "skip_deserializing")?
        )
    )
}

/// Returns the `Id` associated type, which is the raw backing type of `Uid<T>`,
/// if one has been set using the `#[id_type = "..."]` attribute. Defaults to
/// `ObjectId` if unspecified.
fn raw_id_type(attrs: &[Attribute]) -> Result<Type> {
    literal_value_for_name(attrs, "id_type")
        .map(|maybe_ty| maybe_ty.unwrap_or_else(|| {
            Type::Path(TypePath {
                qself: None,
                path: Path {
                    leading_colon: Some(Default::default()),
                    segments: vec!["avocado", "prelude", "ObjectId"]
                        .into_iter()
                        .map(|name| PathSegment {
                            ident: Ident::new(name, Span::call_site()),
                            arguments: Default::default(),
                        })
                        .collect()
                },
            })
        }))
}

/// Returns an error if there is no field serializing as `_id` or if there
/// are more than 1 of them. (The `_id` field must be unambiguous and unique.)
fn ensure_id_exists_and_unique(fields: Fields, attrs: &[Attribute]) -> Result<()> {
    let named = match fields {
        Fields::Named(fields) => fields.named,
        _ => return err_msg("a `Doc` must be a struct with named fields"),
    };
    let rename_attr = serde_name_value(attrs, "rename_all")?;
    let rename_rule: Option<RenameRule> = match rename_attr {
        None => None,
        Some(kv) => Some(value_as_str(&kv)?.parse()?)
    };
    let mut has_id = false;

    for field in named {
        // The field isn't inspected if it's never serialized or deserialized.
        if field_is_always_skipped(&field.attrs)? {
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
        let field_name = serde_renamed_ident(&field.attrs, rename_all_ident)?;

        if field_name == "_id" {
            if has_id {
                return err_msg("more than one fields serialize as `_id`");
            } else {
                has_id = true;
            }
        }
    }

    if has_id {
        Ok(())
    } else {
        err_msg("a `Doc` must contain a field serialized as `_id`")
    }
}

/// Returns `Ok` if the generics only contain lifetime parameters.
/// Returns `Err` if there are also type and/or const parameters.
fn ensure_only_lifetime_params(generics: &Generics) -> Result<()> {
    let make_error = |param_type| err_fmt!(
        "`Doc` can't be derived for a type that is generic over {} parameters",
        param_type
    );

    if generics.type_params().next().is_some() {
        return make_error("type");
    }
    if generics.const_params().next().is_some() {
        return make_error("const");
    }

    Ok(())
}
