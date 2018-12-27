//! Helper functions for retrieving and parsing meta attributes.

use std::str::FromStr;
use syn::{ Attribute, Meta, MetaList, NestedMeta, MetaNameValue, Lit };
use crate::error::{ Error, Result };

/// Returns the inner, `...` part of the first `#[name(...)]` attribute
/// with the specified name (like `#[serde(rename = "foo")]`).
/// TODO(H2CO3): check for duplicate arguments and bail out with an error
fn meta(attrs: &[Attribute], name: &str, key: &str) -> Option<Meta> {
    attrs.iter().filter_map(|attr| {
        let meta_list = match attr.interpret_meta()? {
            Meta::List(list) => {
                if list.ident == name {
                    list
                } else {
                    return None;
                }
            },
            _ => return None,
        };

        meta_list.nested.into_iter().filter_map(|nested_meta| {
            let meta = match nested_meta {
                NestedMeta::Meta(meta) => meta,
                _ => return None,
            };

            let ident = match meta.clone() {
                Meta::Word(ident) => ident,
                Meta::List(list) => list.ident,
                Meta::NameValue(name_value) => name_value.ident,
            };

            if ident == key {
                Some(meta)
            } else {
                None
            }
        })
        .next()
    })
    .next()
}

/// Search for an attribute, provided that it's a name-value pair.
fn name_value(attrs: &[Attribute], name: &str, key: &str) -> Result<Option<MetaNameValue>> {
    match meta(attrs, name, key) {
        Some(Meta::NameValue(name_value)) => Ok(Some(name_value)),
        Some(_) => {
            err_fmt!("attribute must have form `#[{}({} = \"...\")]`", name, key)
        }
        None => Ok(None),
    }
}

/// Search for an attribute, provided that it's a single word.
fn has_meta_word(attrs: &[Attribute], name: &str, key: &str) -> Result<bool> {
    match meta(attrs, name, key) {
        Some(Meta::Word(_)) => Ok(true),
        Some(_) => {
            err_fmt!("attribute must have form `#[{}({})]`", name, key)
        }
        None => Ok(false),
    }
}

/// Search for a `Serde` attribute, provided that it's a name-value pair.
pub fn serde_name_value(attrs: &[Attribute], key: &str) -> Result<Option<MetaNameValue>> {
    name_value(attrs, "serde", key)
}

/// Search for a `Serde` attribute, provided that it's a single word.
pub fn has_serde_word(attrs: &[Attribute], key: &str) -> Result<bool> {
    has_meta_word(attrs, "serde", key)
}

/// Extracts a boolean value from an attribute value.
/// Returns `Err` if the value is not a `LitBool`.
pub fn value_as_bool(nv: &MetaNameValue) -> Result<bool> {
    match nv.lit {
        Lit::Bool(ref lit) => Ok(lit.value),
        _ => err_fmt!("`value for key `{}` must be a bool", nv.ident.to_string())
    }
}

/// Extracts a string value from an attribute value.
/// Returns `Err` if the value is not a `LitStr` nor a valid UTF-8 `LitByteStr`.
pub fn value_as_str(nv: &MetaNameValue) -> Result<String> {
    match nv.lit {
        Lit::Str(ref string) => Ok(string.value()),
        Lit::ByteStr(ref string) => String::from_utf8(string.value()).map_err(Into::into),
        _ => err_fmt!("`value for key `{}` must be a valid UTF-8 string",
                      nv.ident.to_string())
    }
}

/// Tries to parse a `MetaList` as name-value pairs of the given type.
/// Errors if the list doesn't only contain name-value pairs, if the
/// values aren't strings, or if a value of type `T` couldn't be
/// created by means of `FromStr::from_str()`.
pub fn list_into_names_and_values<T>(list: MetaList) -> Result<Vec<(String, T)>>
    where T: FromStr,
          T::Err: Into<Error>,
{
    let list_name = list.ident;

    list.nested
        .into_iter()
        .map(|nested| match nested {
            NestedMeta::Meta(Meta::NameValue(nv)) => {
                value_as_str(&nv)
                    .and_then(|val_str| {
                        val_str
                            .parse()
                            .map(|value| (nv.ident.to_string(), value))
                            .map_err(Into::into)
                    })
            }
            _ => err_fmt!(
                "attribute `{}` must contain key-value pairs only, not {:#?}",
                list_name.to_string(),
                nested
            )
        })
        .collect()
}
