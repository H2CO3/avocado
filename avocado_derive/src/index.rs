//! Types for describing index specifications.

use std::str::FromStr;
use proc_macro2::TokenStream;
use syn::{ Attribute, Meta, NestedMeta, MetaNameValue };
use quote::{ ToTokens, TokenStreamExt };
use crate::{
    error::{ Error, Result, err_msg },
    meta::*,
};

/// Describes the parts of an index that can be described by attributes.
#[derive(Debug, Clone, Default)]
pub struct Spec {
    /// The overridden name of the index.
    name: Option<String>,
    /// Whether the index should forbid duplicate values.
    unique: Option<bool>,
    /// Whether this is a sparse index.
    sparse: Option<bool>,
    /// The actual indexed field names and their type.
    keys: Vec<(String, IndexType)>,
}

impl Spec {
    /// Attempts to parse an `#[index(...)]` attribute as a `Spec`.
    ///
    /// ### Return value:
    /// * `Ok(None)` if `attribute` is not `#[index(...)]`
    /// * `Ok(Some(Spec))` if `attribute` is a well-formed `#[index(...)]`
    /// * `Err(Error)` if `attribute` is `#[index(...)]` but ill-formed.
    pub fn from_attribute(attr: &Attribute) -> Result<Option<Self>> {
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

        let mut spec = Spec::default();

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

    /// Attempts to create an array of `Spec`s from several attributes.
    ///
    /// The implementation could have been simpler:
    ///
    /// `attrs.into_iter().filter_map(Spec::from_attribute).collect()`
    ///
    /// if `Spec::from_attribute()` had returned an `Option<Result<Self>>`
    /// rather than a `Result<Option<Self>>`. Alas, the implementation of
    /// `Spec::from_attribute()` would have been much uglier in that case,
    /// so I decided to pay a (smaller) complexity budget here instead.
    pub fn from_attributes<'a, I>(attrs: I) -> Result<Vec<Spec>>
        where I: IntoIterator<Item=&'a Attribute>
    {
        attrs
            .into_iter()
            .filter_map(|attr| {
                match Spec::from_attribute(attr) {
                    Ok(Some(spec)) => Some(Ok(spec)),
                    Ok(None) => None,
                    Err(error) => Some(Err(error)),
                }
            })
            .collect()
    }
}

impl ToTokens for Spec {
    fn to_tokens(&self, tokens: &mut TokenStream) {
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
    fn to_tokens(&self, tokens: &mut TokenStream) {
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
