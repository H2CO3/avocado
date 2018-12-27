//! This crate only contains the `#[derive(Doc)]` proc-macro for Avocado.
//! For documentation, please see the main [`avocado`][1] crate.
//!
//! [1]: https://docs.rs/avocado

#![crate_type = "proc-macro"]
#![doc(html_root_url = "https://docs.rs/avocado_derive/0.0.5")]
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

#[macro_use]
mod error;
mod meta;
mod case;

use std::str::FromStr;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{
    DeriveInput, Data, Generics, Fields, Type,
    Attribute, Meta, NestedMeta, MetaNameValue,
};
use quote::{ ToTokens, TokenStreamExt };
use self::{
    meta::*,
    case::RenameRule,
    error::{ Result, Error, err_msg },
};

/// The top-level entry point of this proc-macro. Only here to be exported
/// and to handle `Result::Err` return values by `panic!()`ing.
#[proc_macro_derive(Doc, attributes(avocado, index))]
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

    ensure_only_lifetime_params(&generics)?;

    match parsed_ast.data {
        Data::Struct(s) => {
            let id_ty = type_of_id_field(s.fields, &parsed_ast.attrs)?;
            let indexes = IndexSpec::from_attributes(&parsed_ast.attrs)?;
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
        .map_or_else(|| Ok(ident), value_as_str)
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

/// Returns the declared type of the field which serializes as `_id`.
/// If there's no such field, returns an `Err`.
fn type_of_id_field(fields: Fields, attrs: &[Attribute]) -> Result<Type> {
    let named = match fields {
        Fields::Named(fields) => fields.named,
        _ => return err_msg("a `Doc` must be a struct with named fields"),
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
        if field_is_always_skipped(&attrs)? {
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

    err_msg("a `Doc` must contain a field (de)serialized as `_id`")
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

/// Describes the parts of an index that can be described by attributes.
#[derive(Debug, Clone, Default)]
struct IndexSpec {
    /// The overridden name of the index.
    name: Option<String>,
    /// Whether the index should forbid duplicate values.
    unique: Option<bool>,
    /// Whether this is a sparse index.
    sparse: Option<bool>,
    /// The actual indexed field names and their type.
    keys: Vec<(String, IndexType)>,
}

impl IndexSpec {
    /// Attempts to parse an `#[index(...)]` attribute as an `IndexSpec`.
    ///
    /// ### Return value:
    /// * `Ok(None)` if `attribute` is not `#[index(...)]`
    /// * `Ok(Some(IndexSpec))` if `attribute` is a well-formed `#[index(...)]`
    /// * `Err(Error)` if `attribute` is `#[index(...)]` but ill-formed.
    fn new(attr: &Attribute) -> Result<Option<Self>> {
        let meta = match attr.interpret_meta() {
            None => return Ok(None),
            Some(meta) => meta,
        };
        let meta = match meta {
            Meta::List(list) => {
                if list.ident == "index" {
                    list
                } else {
                    return Ok(None);
                }
            }
            Meta::Word(ident) | Meta::NameValue(MetaNameValue { ident, .. }) => {
                if ident == "index" {
                    // index attribute, but malformed
                    return err_msg("attribute must be of the form `#[index(...)]`");
                } else {
                    // none of our business
                    return Ok(None);
                }
            }
        };

        let inner_metas: Vec<_> = meta.nested
            .into_iter()
            .map(|nested| match nested {
                NestedMeta::Meta(nested_meta) => Ok(nested_meta),
                NestedMeta::Literal(lit) => {
                    err_fmt!("expected a meta item, found literal: {:#?}", lit)
                }
            })
            .collect::<Result<_>>()?;

        let mut spec = IndexSpec::default();

        for inner_meta in inner_metas {
            match inner_meta {
                Meta::Word(ident) => match ident.to_string().as_str() {
                    "unique" => spec.unique = Some(true),
                    "sparse" => spec.sparse = Some(true),
                    word => return err_fmt!("bad single-word attribute: {}", word)
                },
                Meta::NameValue(nv) => match nv.ident.to_string().as_str() {
                    "unique" => spec.unique = value_as_bool(&nv)?.into(),
                    "sparse" => spec.sparse = value_as_bool(&nv)?.into(),
                    "name" => spec.name = value_as_str(&nv)?.into(),
                    name => return err_fmt!("bad name-value attribute: {}", name)
                },
                Meta::List(list) => match list.ident.to_string().as_str() {
                    "keys" => spec.keys = list_into_names_and_values(list)?,
                    name => return err_fmt!("bad list attribute: {}", name)
                }
            }
        }

        if spec.keys.is_empty() {
            err_msg("at least one field must be specified for indexing")
        } else {
            Ok(Some(spec))
        }
    }

    /// Attempts to create an array of `IndexSpec`s from several attributes.
    ///
    /// The implementation could have been simpler:
    ///
    /// `attrs.into_iter().filter_map(IndexSpec::new).collect()`
    ///
    /// if `IndexSpec::new()` had returned an `Option<Result<Self>>`
    /// rather than a `Result<Option<Self>>`. Alas, the implementation
    /// of `IndexSpec::new()` would have been much uglier in that case,
    /// so I decided to pay a (smaller) complexity budget here instead.
    fn from_attributes<'a, I>(attrs: I) -> Result<Vec<IndexSpec>>
        where I: IntoIterator<Item=&'a Attribute>
    {
        attrs
            .into_iter()
            .filter_map(|attr| {
                match IndexSpec::new(attr) {
                    Ok(Some(spec)) => Some(Ok(spec)),
                    Ok(None) => None,
                    Err(error) => Some(Err(error)),
                }
            })
            .collect()
    }
}

impl ToTokens for IndexSpec {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let unique = self.unique.as_ref().map(|b| quote!(unique: Some(#b),));
        let sparse = self.sparse.as_ref().map(|b| quote!(sparse: Some(#b),));
        let name = self.name.as_ref().map(
            |s| quote!(name: Some(String::from(#s)),)
        );
        let fields = self.keys.iter().map(|&(ref field, _)| field);
        let types  = self.keys.iter().map(|&(_, ty)| ty);

        tokens.append_all(quote!{
            ::avocado::prelude::IndexModel {
                keys: {
                    let mut avocado_keys = ::avocado::prelude::Document::new();
                    #(avocado_keys.insert(#fields, #types);)*
                    avocado_keys
                },
                options: ::avocado::prelude::IndexOptions {
                    #name
                    #unique
                    #sparse
                    ..Default::default()
                },
            }
        });
    }
}

/// An index type, applied to a single indexed field.
#[derive(Debug, Clone, Copy)]
enum IndexType {
    /// An ordered, ascending index field.
    Ascending,
    /// An ordered, descending index field.
    Descending,
    /// A language-specific textual index, most useful for freetext searches.
    Text,
    /// Hashed index for hash-based sharding.
    Hashed,
    /// 2D geospatial index with planar (Euclidean) geometry.
    Geo2D,
    /// 2D geospatial index with spherical geometry.
    Geo2DSphere,
    /// 2D geospatial index optimized for very small areas.
    GeoHaystack,
}

impl FromStr for IndexType {
    type Err = Error;

    fn from_str(string: &str) -> Result<Self> {
        Ok(match string {
            "ascending"   => IndexType::Ascending,
            "descending"  => IndexType::Descending,
            "text"        => IndexType::Text,
            "hashed"      => IndexType::Hashed,
            "2d"          => IndexType::Geo2D,
            "2dsphere"    => IndexType::Geo2DSphere,
            "geoHaystack" => IndexType::GeoHaystack,
            _ => return err_fmt!("unknown index type '{}'", string)
        })
    }
}

impl ToTokens for IndexType {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match *self {
            IndexType::Ascending   => 1.to_tokens(tokens),
            IndexType::Descending  => (-1).to_tokens(tokens),
            IndexType::Text        => "text".to_tokens(tokens),
            IndexType::Hashed      => "hashed".to_tokens(tokens),
            IndexType::Geo2D       => "2d".to_tokens(tokens),
            IndexType::Geo2DSphere => "2dsphere".to_tokens(tokens),
            IndexType::GeoHaystack => "geoHaystack".to_tokens(tokens),
        }
    }
}
