//! Helpers for conveniently specifying additional `Doc` options
//! while still being able to use `#[derive(Doc)]`.

use std::collections::HashMap;
use proc_macro2::{ TokenStream, Span };
use syn::{ Attribute, Ident, Path, PathSegment };
use syn::{ Meta, NestedMeta, MetaNameValue, Lit };
use quote::{ ToTokens, TokenStreamExt };
use crate::error::{ Result, err_msg };

/// This type can tokenize itself in a way that, when quoted inside
/// an `impl Doc for T`, will expand to a bunch of option functions
/// overriding the default options provided by the `Doc` trait.
///
/// The hash map maps names of option functions in the `Doc` trait to
/// pairs of path components of their respective return type and the
/// user-specified path which should be used for implementing said
/// function by means of treating that path as a function itself and
/// emitting a call to it.
#[derive(Debug, Clone)]
pub struct DocOptions(HashMap<String, (&'static [&'static str], Option<Path>)>);

impl DocOptions {
    /// Create an empty `DocOptions` instance.
    fn new() -> Self {
        let all_options: &[(&str, &[&str])] = &[
            (
                "count_options",
                &["mongodb", "coll", "options", "CountOptions"],
            ),
            (
                "distinct_options",
                &["mongodb", "coll", "options", "DistinctOptions"],
            ),
            (
                "aggregate_options",
                &["mongodb", "coll", "options", "AggregateOptions"],
            ),
            (
                "query_options",
                &["mongodb", "coll", "options", "FindOptions"],
            ),
            (
                "insert_options",
                &["mongodb", "coll", "options", "InsertManyOptions"],
            ),
            (
                "delete_options",
                &["mongodb", "common", "WriteConcern"],
            ),
            (
                "update_options",
                &["mongodb", "common", "WriteConcern"],
            ),
            (
                "upsert_options",
                &["mongodb", "common", "WriteConcern"],
            ),
            (
                "find_and_update_options",
                &["mongodb", "coll", "options", "FindOneAndUpdateOptions"],
            ),
        ];

        let hm = all_options
            .iter()
            .map(|&(fn_name, type_path_components)| {
                (fn_name.into(), (type_path_components, None))
            })
            .collect();

        DocOptions(hm)
    }

    /// Create an options descriptor from the `#[options(...)]` attribute.
    pub fn from_attributes(attrs: &[Attribute]) -> Result<Self> {
        let mut options = DocOptions::new();

        let metas = attrs
            .iter()
            .filter_map(Attribute::interpret_meta)
            .filter_map(|meta| match meta {
                Meta::Word(_) | Meta::NameValue(_) => None,
                Meta::List(meta) => if meta.ident == "options" {
                    Some(meta.nested)
                } else {
                    None
                }
            })
            .next();

        if let Some(metas) = metas {
            for meta in metas {
                match meta {
                    NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                        ident,
                        lit: Lit::Str(path_str),
                        ..
                    })) => {
                        let path: Path = path_str.parse()?;
                        let fn_name = ident.to_string();

                        match options.0.get_mut(&fn_name) {
                            Some(&mut (_, ref mut path_ptr)) => {
                                *path_ptr = Some(path);
                            }
                            None => return err_fmt!(
                                "no option method named `Doc::{}()`", fn_name
                            )
                        }
                    },
                    _ => return err_msg(
                        "attribute must have form `#[options(fn_name = \"path\", ...)]`"
                    )
                }
            }
        }

        Ok(options)
    }
}

impl ToTokens for DocOptions {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for (fn_name, &(type_path_components, ref callee_path)) in &self.0 {
            if let Some(ref callee_path) = *callee_path {
                fn_to_tokens(fn_name, type_path_components, callee_path, tokens);
            }
        }
    }
}

/// If a particular function is implemented from within the derive proc-macro,
/// render it here.
fn fn_to_tokens(fn_name: &str, type_path_components: &[&str], callee_path: &Path, tokens: &mut TokenStream) {
    let fn_name = Ident::new(fn_name, Span::call_site());
    let type_path = Path {
        leading_colon: Some(Default::default()),
        segments: type_path_components
            .iter()
            .map(|&name| PathSegment {
                ident: Ident::new(name, Span::call_site()),
                arguments: Default::default(),
            })
            .collect()
    };

    tokens.append_all(quote! {
        fn #fn_name() -> #type_path {
            #callee_path()
        }
    });
}
