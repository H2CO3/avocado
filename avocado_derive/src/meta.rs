//! Helper functions for retrieving and parsing meta attributes.

use std::str;
use std::str::FromStr;
use std::i32;
use std::ops::RangeBounds;
use std::fmt::Debug;
use syn::{ Attribute, Meta, MetaList, NestedMeta, MetaNameValue, Lit };
use syn::synom::Synom;
use crate::{
    attr::{ ExtMeta, NestedExtMeta, PathExt },
    error::{ Error, Result },
};

/// Utilities for working with ranges.
pub trait RangeBoundsExt<T>: RangeBounds<T> {
    /// Replicates the still-unstable `RangeBounds::contains()` method.
    fn contains_value<U>(&self, value: &U) -> bool
        where T: PartialOrd<U>,
              U: PartialOrd<T> + ?Sized,
    {
        use std::ops::Bound::*;

        (match self.start_bound() {
            Included(ref start) => *start <= value,
            Excluded(ref start) => *start < value,
            Unbounded => true,
        })
        &&
        (match self.end_bound() {
            Included(ref end) => value <= *end,
            Excluded(ref end) => value < *end,
            Unbounded => true,
        })
    }
}

impl<T, R: RangeBounds<T>> RangeBoundsExt<T> for R {}

/// Returns the inner, `...` part of the first `#[name(...)]` attribute
/// with the specified name (like `#[serde(rename = "foo")]`).
/// TODO(H2CO3): check for duplicate arguments and bail out with an error
fn meta(attrs: &[Attribute], name: &str, key: &str) -> Option<Meta> {
    attrs.iter().find_map(|attr| {
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

        meta_list.nested.into_iter().find_map(|nested_meta| {
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
    })
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
pub fn value_as_bool(key: &str, lit: &Lit) -> Result<bool> {
    match *lit {
        Lit::Bool(ref lit) => Ok(lit.value),
        _ => err_fmt!("value for key `{}` must be a bool", key)
    }
}

/// Extracts a string value from an attribute value.
/// Returns `Err` if the value is not a `LitStr` nor a valid UTF-8 `LitByteStr`.
pub fn value_as_str(nv: &MetaNameValue) -> Result<String> {
    match nv.lit {
        Lit::Str(ref string) => Ok(string.value()),
        Lit::ByteStr(ref string) => {
            String::from_utf8(string.value()).map_err(Into::into)
        }
        _ => err_fmt!("value for key `{}` must be a valid UTF-8 string",
                      nv.ident.to_string())
    }
}

/// Similar to `value_as_str()`, but for `ExtMeta`-related usage.
pub fn lit_value_as_str(key: &str, lit: &Lit) -> Result<String> {
    match *lit {
        Lit::Str(ref string) => Ok(string.value()),
        Lit::ByteStr(ref string) => {
            String::from_utf8(string.value()).map_err(Into::into)
        }
        _ => err_fmt!("value for key `{}` must be a valid UTF-8 string", key)
    }
}

/// Extracts an `i32` value from an attribute value.
/// Ensures that the resulting value is contained in the specified `range`.
///
/// Accepts string-valued attributes as well because that is currently the
/// only way to specify a negative number.
#[allow(clippy::cast_possible_truncation)]
pub fn value_as_i32<R>(key: &str, lit: &Lit, range: R) -> Result<i32>
    where R: Debug + RangeBoundsExt<i32>
{
    let value = match *lit {
        Lit::Int(ref lit) => {
            let v = lit.value();
            if v <= i32::MAX as u64 {
                v as i32
            } else {
                err_fmt!("integer value `{}` for key `{}` overflows i32", v, key)?
            }
        }
        Lit::Str(ref lit) => lit.value().parse()?,
        Lit::ByteStr(ref lit) => str::from_utf8(&lit.value())?.parse()?,
        _ => return err_fmt!("value for key `{}` must be an i32", key)
    };

    if range.contains_value(&value) {
        Ok(value)
    } else {
        err_fmt!("value `{}` for key `{}` exceeds range {:?}",
                 value, key, range)
    }
}

/// Extracts an `f64` value from an attribute value.
/// Ensures that the resulting value is contained in the specified `range`.
///
/// Accepts string-valued attributes as well because that is currently the
/// only way to specify a negative number.
#[allow(clippy::cast_precision_loss)]
pub fn value_as_f64<R>(key: &str, lit: &Lit, range: R) -> Result<f64>
    where R: Debug + RangeBoundsExt<f64>
{
    let value = match *lit {
        Lit::Float(ref lit) => lit.value(),
        Lit::Int(ref lit) => lit.value() as f64,
        Lit::Str(ref lit) => lit.value().parse()?,
        Lit::ByteStr(ref lit) => str::from_utf8(&lit.value())?.parse()?,
        _ => return err_fmt!("value for key `{}` must be an f64", key)
    };

    if range.contains_value(&value) {
        Ok(value)
    } else {
        err_fmt!("value `{}` for key `{}` exceeds range {:?}",
                 value, key, range)
    }
}

/// Tries to parse a list of `NestedExtMeta` as name-value pairs of the given
/// type. Errors if the list doesn't only contain name-value pairs, if the
/// values aren't strings, or if a value of type `T` couldn't be
/// created by means of `FromStr::from_str()`.
pub fn list_into_names_and_values<T, I>(outer_name: &str, list: I) -> Result<Vec<(String, T)>>
    where T: FromStr,
          T::Err: Into<Error>,
          I: IntoIterator<Item = NestedExtMeta>,
{
    list.into_iter()
        .map(|nested| match nested {
            NestedExtMeta::Meta(ExtMeta::KeyValue(path, _, literal)) => {
                let val_str = match literal {
                    Lit::Str(ref s) => s.value(),
                    Lit::ByteStr(ref s) => String::from_utf8(s.value())?,
                    _ => return err_fmt!(
                        "value for key `{}` must be a valid UTF-8 string",
                        path.colon_sep_str()
                    )
                };
                val_str
                    .parse()
                    .map_err(Into::into)
                    .map(|value| (path.dot_sep_str(), value))
            }
            _ => err_fmt!(
                "attribute `{}` must contain key-value pairs only, not {:#?}",
                outer_name,
                nested
            )
        })
        .collect()
}

/// Extracts the literal value of a top-level name-value pair of the given name.
pub fn literal_value_for_name<T: Synom>(attrs: &[Attribute], name: &str) -> Result<Option<T>> {
    attrs
        .iter()
        .find_map(|attr| match attr.interpret_meta()? {
            Meta::NameValue(nv) => {
                if nv.ident == name {
                    value_as_str(&nv)
                        .and_then(|s| syn::parse_str(&s).map_err(Into::into))
                        .into()
                } else {
                    None
                }
            }
            Meta::Word(ident) | Meta::List(MetaList { ident, .. }) => {
                if ident == name {
                    Some(
                        err_fmt!("attribute must have form `#[{} = ...]`", name)
                    )
                } else {
                    None
                }
            }
        })
        .map_or(Ok(None), |result| result.map(Some))
}
