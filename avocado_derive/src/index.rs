//! Types for describing index specifications.

use std::str::FromStr;
use proc_macro2::TokenStream;
use syn::Attribute;
use quote::{ ToTokens, TokenStreamExt };
use crate::{
    error::{ Error, Result, err_msg },
    attr::*,
    meta::*,
};

/// Describes the parts of an index that can be derived using attributes.
#[derive(Debug, Clone, Default)]
pub struct Spec {
    /// The overridden name of the index.
    name: Option<String>,
    /// Whether the index should forbid duplicate values.
    unique: Option<bool>,
    /// Whether this is a sparse index.
    sparse: Option<bool>,
    /// The name of the default language for a text index.
    default_language: Option<String>,
    /// The name of the field specifying the language of the document.
    language_override: Option<String>,
    /// The number of precision bits of the geohash value of `2d` indexes,
    /// in range `[1, 26]`.
    bits: Option<i32>,
    /// The maximal allowed longitude and latitude, in range `[-180, 180]`.
    max: Option<f64>,
    /// The maximal allowed longitude and latitude, in range `[-180, 180]`.
    min: Option<f64>,
    /// Cluster size in units of distance, for geoHaystack. Must be positive.
    bucket_size: Option<i32>,
    /// The actual indexed field names and their type.
    keys: Vec<(String, Type)>,
}

impl Spec {
    /// Attempts to parse an `#[index(...)]` attribute as a `Spec`.
    ///
    /// ### Return value:
    /// * `Ok(None)` if `attribute` is not `#[index(...)]`
    /// * `Ok(Some(Spec))` if `attribute` is a well-formed `#[index(...)]`
    /// * `Err(Error)` if `attribute` is `#[index(...)]` but ill-formed.
    pub fn from_attribute(attr: &Attribute) -> Result<Option<Self>> {
        let meta = match attr.parse_ext_meta() {
            None => return Ok(None),
            Some(meta) => meta,
        };
        let meta = match meta {
            ExtMeta::List(path, _, nested) => {
                if path.into_token_stream().to_string() == "index" {
                    nested
                } else {
                    return Ok(None);
                }
            }
            ExtMeta::Path(path) | ExtMeta::KeyValue(path, ..) => {
                if path.into_token_stream().to_string() == "index" {
                    // index attribute, but malformed
                    err_msg("attribute must be of the form `#[index(...)]`")?
                } else {
                    // none of our business
                    return Ok(None);
                }
            }
        };

        let inner_metas: Vec<_> = meta
            .into_iter()
            .map(|nested| match nested {
                NestedExtMeta::Meta(nested_meta) => Ok(nested_meta),
                NestedExtMeta::Literal(lit) => {
                    err_fmt!("expected a meta item, found literal: {:#?}", lit)
                }
            })
            .collect::<Result<_>>()?;

        Self::from_metas(inner_metas)
    }

    /// Attempts to create a `Spec` from a list of pre-parsed `Meta` items.
    fn from_metas<I>(inner_metas: I) -> Result<Option<Self>>
        where I: IntoIterator<Item=ExtMeta>
    {
        let mut spec = Spec::default();

        for inner_meta in inner_metas {
            let path_str = inner_meta.path_str();

            match inner_meta {
                ExtMeta::Path(_) => match path_str.as_str() {
                    "unique" => spec.unique = Some(true),
                    "sparse" => spec.sparse = Some(true),
                    _ => err_fmt!("bad path attribute: {}", path_str)?
                }
                ExtMeta::KeyValue(_, _, lit) => match path_str.as_str() {
                    "unique" => {
                        spec.unique = value_as_bool(&path_str, &lit)?.into()
                    }
                    "sparse" => {
                        spec.sparse = value_as_bool(&path_str, &lit)?.into()
                    }
                    "name" => {
                        spec.name = lit_value_as_str(&path_str, &lit)?.into()
                    }
                    "min" => spec.min = value_as_f64(&path_str,
                                                     &lit,
                                                     -180.0..=180.0)?.into(),
                    "max" => spec.max = value_as_f64(&path_str,
                                                     &lit,
                                                     -180.0..=180.0)?.into(),
                    "bits" => spec.bits = value_as_i32(&path_str,
                                                       &lit,
                                                       1..=32)?.into(),
                    "bucket_size" => spec.bucket_size = value_as_i32(
                        &path_str,
                        &lit,
                        1..
                    )?.into(),
                    "default_language" => {
                        spec.default_language = lit_value_as_str(
                            &path_str,
                            &lit
                        )?.into()
                    }
                    "language_override" => {
                        spec.language_override = lit_value_as_str(
                            &path_str,
                            &lit
                        )?.into()
                    }
                    _ => err_fmt!("bad name-value attribute: {}", path_str)?
                },
                ExtMeta::List(_, _, list) => match path_str.as_str() {
                    "keys" => {
                        spec.keys = list_into_names_and_values(&path_str, list)?
                    }
                    _ => err_fmt!("bad list attribute: {}", path_str)?
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
        let default_language = self.default_language.as_ref().map(
            |s| quote!(default_language: Some(String::from(#s)),)
        );
        let language_override = self.language_override.as_ref().map(
            |s| quote!(language_override: Some(String::from(#s)),)
        );
        let bucket_size = self.bucket_size.as_ref().map(
            |n| quote!(bucket_size: Some(#n),)
        );
        let bits = self.bits.as_ref().map(|n| quote!(bits: Some(#n),));
        let min = self.min.as_ref().map(|x| quote!(min: Some(#x),));
        let max = self.max.as_ref().map(|x| quote!(max: Some(#x),));
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
                    #min
                    #max
                    #bits
                    #bucket_size
                    #default_language
                    #language_override
                    ..Default::default()
                },
            }
        });
    }
}

/// An index type, applied to a single indexed field.
#[derive(Debug, Clone, Copy)]
enum Type {
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

impl FromStr for Type {
    type Err = Error;

    fn from_str(string: &str) -> Result<Self> {
        Ok(match string {
            "ascending"   => Type::Ascending,
            "descending"  => Type::Descending,
            "text"        => Type::Text,
            "hashed"      => Type::Hashed,
            "2d"          => Type::Geo2D,
            "2dsphere"    => Type::Geo2DSphere,
            "geoHaystack" => Type::GeoHaystack,
            _ => err_fmt!("unknown index type '{}'", string)?
        })
    }
}

impl ToTokens for Type {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match *self {
            Type::Ascending   => 1.to_tokens(tokens),
            Type::Descending  => (-1).to_tokens(tokens),
            Type::Text        => "text".to_tokens(tokens),
            Type::Hashed      => "hashed".to_tokens(tokens),
            Type::Geo2D       => "2d".to_tokens(tokens),
            Type::Geo2DSphere => "2dsphere".to_tokens(tokens),
            Type::GeoHaystack => "geoHaystack".to_tokens(tokens),
        }
    }
}
