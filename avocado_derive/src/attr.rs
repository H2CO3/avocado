//! This is mostly lifted from [`syn`](https://docs.rs/syn/0.14.9/src/syn/attr.rs.html#101-259)
//! and slightly adapted so that paths in key-value attributes can be parsed too.

use syn::{
    Attribute, Path, Lit, LitBool,
    token::Paren,
    punctuated::Punctuated,
};
use proc_macro2::{ Delimiter, Spacing, TokenTree };

/// Loosely mirrors `syn::Meta`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExtMeta {
    /// A single-path attribute.
    Path(Path),
    /// A named list of nested attributes.
    List(Path, Paren, Punctuated<NestedExtMeta, Token![,]>),
    /// A key-value pair within an attribute, like `feature = "nightly"`.
    KeyValue(Path, Token![=], Lit),
}

/// The equivalent of `syn::NestedMeta`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NestedExtMeta {
    /// A structured meta item, like the `Copy` in `#[derive(Copy)]` which
    /// would be a nested `Meta::Word`.
    Meta(ExtMeta),
    /// A Rust literal, like the `"new_name"` in `#[rename("new_name")]`.
    Literal(Lit),
}

impl From<ExtMeta> for NestedExtMeta {
    fn from(meta: ExtMeta) -> Self {
        NestedExtMeta::Meta(meta)
    }
}

impl From<Lit> for NestedExtMeta {
    fn from(lit: Lit) -> Self {
        NestedExtMeta::Literal(lit)
    }
}

/// Provides the `parse_ext_meta()` method.
pub trait AttributeExt {
    /// Parses the attribute like `interpret_meta()` but a bit smarter:
    /// this method also accepts paths (as opposed to single identifiers)
    /// in key-value pairs.
    fn parse_ext_meta(&self) -> Option<ExtMeta>;
}

impl AttributeExt for Attribute {
    fn parse_ext_meta(&self) -> Option<ExtMeta> {
        let path = if self.path.segments.is_empty() {
            return None;
        } else {
            &self.path
        };

        if self.tts.is_empty() {
            return Some(ExtMeta::Path(path.clone()));
        }

        let tts: Vec<_> = self.tts.clone().into_iter().collect();

        if tts.len() == 1 {
            if let Some(meta) = extract_meta_list(path.clone(), &tts[0]) {
                return Some(meta);
            }
        }

        if tts.len() == 2 {
            if let Some(meta) = extract_name_value(path.clone(), &tts[0], &tts[1]) {
                return Some(meta);
            }
        }

        None
    }
}

/// Converts an identifier and a token tree to a `MetaList` if possible.
fn extract_meta_list(path: Path, tt: &TokenTree) -> Option<ExtMeta> {
    let g = match *tt {
        TokenTree::Group(ref g) => g,
        _ => return None,
    };

    if g.delimiter() != Delimiter::Parenthesis {
        return None;
    }

    let tokens: Vec<_> = g.stream().clone().into_iter().collect();
    let nested = match list_of_nested_meta_items_from_tokens(&tokens) {
        Some(n) => n,
        None => return None,
    };

    Some(ExtMeta::List(path, Paren(g.span()), nested))
}

/// Converts an identifier, an equal sign, and a token tree to a
/// `MetaNameValue` if possible.
fn extract_name_value(path: Path, a: &TokenTree, b: &TokenTree) -> Option<ExtMeta> {
    let a_punct = match *a {
        TokenTree::Punct(ref o) => o,
        _ => return None,
    };

    if a_punct.spacing() != Spacing::Alone {
        return None;
    }
    if a_punct.as_char() != '=' {
        return None;
    }

    match *b {
        TokenTree::Literal(ref l) if !l.to_string().starts_with('/') => {
            Some(ExtMeta::KeyValue(
                path,
                Token![=]([a_punct.span()]),
                Lit::new(l.clone()),
            ))
        }
        TokenTree::Ident(ref v) => match &v.to_string()[..] {
            v @ "true" | v @ "false" => Some(ExtMeta::KeyValue(
                path,
                Token![=]([a.span()]),
                Lit::Bool(LitBool {
                    value: v == "true",
                    span: b.span(),
                }),
            )),
            _ => None,
        },
        _ => None,
    }
}

/// Converts a list of consecutive token trees to a nested meta (a `Meta`
/// or a `Lit`), also returning the rest of the still unparsed token trees.
fn nested_meta_item_from_tokens(tts: &[TokenTree]) -> Option<(NestedExtMeta, &[TokenTree])> {
    match *tts.first()? {
        TokenTree::Literal(ref lit) => {
            if lit.to_string().starts_with('/') {
                None
            } else {
                let lit = Lit::new(lit.clone());
                Some((NestedExtMeta::Literal(lit), &tts[1..]))
            }
        }
        TokenTree::Ident(ref ident) => {
            if tts.len() >= 3 {
                if let Some(meta) = extract_name_value(
                    ident.clone().into(),
                    &tts[1], &tts[2]
                ) {
                    return Some((NestedExtMeta::Meta(meta), &tts[3..]));
                }
            }

            if tts.len() >= 2 {
                if let Some(meta) = extract_meta_list(ident.clone().into(), &tts[1]) {
                    return Some((NestedExtMeta::Meta(meta), &tts[2..]));
                }
            }

            Some((ExtMeta::Path(ident.clone().into()).into(), &tts[1..]))
        }
        _ => None
    }
}

/// Helper for `extract_meta_list()`.
fn list_of_nested_meta_items_from_tokens(
    mut tts: &[TokenTree],
) -> Option<Punctuated<NestedExtMeta, Token![,]>> {
    let mut nested_meta_items = Punctuated::new();
    let mut first = true;

    while !tts.is_empty() {
        let prev_comma = if first {
            first = false;
            None
        } else if let TokenTree::Punct(ref op) = tts[0] {
            if op.spacing() != Spacing::Alone {
                return None;
            }
            if op.as_char() != ',' {
                return None;
            }

            let tok = Token![,]([op.span()]);

            tts = &tts[1..];

            if tts.is_empty() {
                break;
            }

            Some(tok)
        } else {
            return None;
        };
        let (nested, rest) = match nested_meta_item_from_tokens(tts) {
            Some(pair) => pair,
            None => return None,
        };
        if let Some(comma) = prev_comma {
            nested_meta_items.push_punct(comma);
        }

        nested_meta_items.push_value(nested);
        tts = rest;
    }

    Some(nested_meta_items)
}
