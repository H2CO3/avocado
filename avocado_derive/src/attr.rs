//! This is mostly lifted from [`syn`](https://docs.rs/syn/0.14.9/src/syn/attr.rs.html#101-259)
//! and slightly adapted so that paths in key-value attributes can be parsed too.

use syn::{
    Attribute, Path, PathSegment, Lit, LitBool, Ident,
    token::Paren,
    punctuated::Punctuated,
};
use quote::ToTokens;
use proc_macro2::{ Delimiter, Spacing, TokenTree, TokenStream };

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

impl ExtMeta {
    /// Returns a reference to the path of this meta item.
    pub fn path(&self) -> &Path {
        match *self {
            ExtMeta::Path(ref path) => path,
            ExtMeta::List(ref path, ..) => path,
            ExtMeta::KeyValue(ref path, ..) => path,
        }
    }

    /// Returns the path of this meta item as a string.
    pub fn path_str(&self) -> String {
        self.path().colon_sep_str()
    }
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

/// Provides convenience helper methods on `Path`.
pub trait PathExt {
    /// Returns the colon-separated string representation of the path.
    fn colon_sep_str(&self) -> String;

    /// Returns the dot-separated string representation of the path.
    fn dot_sep_str(&self) -> String;
}

impl PathExt for Path {
    fn colon_sep_str(&self) -> String {
        let mut ts = TokenStream::new();
        self.to_tokens(&mut ts);
        ts.to_string()
    }

    fn dot_sep_str(&self) -> String {
        let idents: Vec<_> = self.segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();

        idents.join(".")
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

        let tts: Vec<_> = self.tts.clone().into_iter().collect();

        meta_from_path_and_token_trees(path, &tts)
    }
}

/// Parses a *single* `ExtMeta` from a list of token trees.
fn meta_from_path_and_token_trees(path: &Path, tts: &[TokenTree]) -> Option<ExtMeta> {
    if tts.is_empty() {
        return Some(ExtMeta::Path(path.clone()));
    }

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

/// Converts a path and a token tree to a `MetaList` if possible.
fn extract_meta_list(path: Path, tt: &TokenTree) -> Option<ExtMeta> {
    let g = match *tt {
        TokenTree::Group(ref g) => g,
        _ => return None,
    };

    if g.delimiter() != Delimiter::Parenthesis {
        return None;
    }

    let tokens: Vec<_> = g.stream().clone().into_iter().collect();
    let nested = list_of_nested_meta_items_from_tokens(&tokens)?;

    Some(ExtMeta::List(path, Paren(g.span()), nested))
}

/// Converts a path, an equal sign, and a token tree to a
/// `MetaNameValue` if possible.
fn extract_name_value(path: Path, eq: &TokenTree, lit: &TokenTree) -> Option<ExtMeta> {
    let eq_punct = match *eq {
        TokenTree::Punct(ref o) => o,
        _ => return None,
    };

    if eq_punct.spacing() != Spacing::Alone {
        return None;
    }
    if eq_punct.as_char() != '=' {
        return None;
    }

    match *lit {
        TokenTree::Literal(ref l) if !l.to_string().starts_with('/') => {
            Some(ExtMeta::KeyValue(
                path,
                Token![=]([eq.span()]),
                Lit::new(l.clone()),
            ))
        }
        TokenTree::Ident(ref v) => match &v.to_string()[..] {
            v @ "true" | v @ "false" => Some(ExtMeta::KeyValue(
                path,
                Token![=]([eq.span()]),
                Lit::Bool(LitBool {
                    value: v == "true",
                    span: lit.span(),
                }),
            )),
            _ => None,
        },
        _ => None,
    }
}

/// Converts a list of consecutive token trees to a nested meta (a `Meta`
/// or a `Lit`).
///
/// `tts` must contain either a single literal, or a path followed by:
/// * an optional `=` and a literal; or
/// * a parenthesized list.
///
/// That is, the input token tree must be pre-sliced, beacuse its size will
/// be used by `meta_from_path_and_token_trees()` to decide what kind of
/// meta to parse it to.
fn nested_meta_item_from_tokens(tts: &[TokenTree]) -> Option<NestedExtMeta> {
    match *tts.first()? {
        TokenTree::Literal(ref lit) => {
            if tts.len() == 1 && !lit.to_string().starts_with('/') {
                Some(NestedExtMeta::Literal(Lit::new(lit.clone())))
            } else {
                None
            }
        }
        TokenTree::Ident(_) => {
            let (path, rest) = path_from_prefix_of_token_trees(tts)?;

            meta_from_path_and_token_trees(&path, rest).map(NestedExtMeta::Meta)
        }
        _ => None
    }
}

/// Helper for `extract_meta_list()`. The argument `tts` is the list of
/// token trees *inside* the parentheses, but *without* the enclosing
/// parenthesis tokens.
fn list_of_nested_meta_items_from_tokens(
    mut tts: &[TokenTree],
) -> Option<Punctuated<NestedExtMeta, Token![,]>> {
    let mut nested_meta_items = Punctuated::new();

    loop {
        let mut comma = None;
        let mut i = 0;

        while i < tts.len() {
            if let TokenTree::Punct(ref op) = tts[i] {
                if op.as_char() == ',' && op.spacing() == Spacing::Alone {
                    comma = Some(Token![,]([op.span()]));
                    i += 1;
                    break;
                }
            }

            i += 1;
        }

        let until_next_comma = if comma.is_some() {
            &tts[..i - 1]
        } else {
            &tts[..i]
        };

        tts = &tts[i..];

        if until_next_comma.is_empty() {
            if comma.is_some() {
                break None; // TODO(H2CO3): is this indeed correct?
            } else {
                break Some(nested_meta_items);
            }
        }

        let nested = nested_meta_item_from_tokens(until_next_comma)?;
        nested_meta_items.push_value(nested);

        if let Some(comma) = comma {
            nested_meta_items.push_punct(comma);
        }
    }
}

/// Parses the prefix of a bunch of token trees as a `Path`, and returns
/// it with the rest of the (unparsed) token tree postfix.
fn path_from_prefix_of_token_trees(tts: &[TokenTree]) -> Option<(Path, &[TokenTree])> {
    let mut i = 0;
    let mut path = Path {
        leading_colon: None,
        segments: Punctuated::new(),
    };

    while i < tts.len() {
        if let TokenTree::Ident(ref ident) = tts[i] {
            path.segments.push(PathSegment {
                ident: Ident::new(&ident.to_string(), ident.span()),
                arguments: Default::default(),
            });

            if let (
                Some(&TokenTree::Punct(ref op_left)),
                Some(&TokenTree::Punct(ref op_right)),
            ) = (tts.get(i + 1), tts.get(i + 2)) {
                if op_left.as_char() == ':'
                    && op_left.spacing() == Spacing::Joint
                    && op_right.as_char() == ':'
                    && op_right.spacing() == Spacing::Alone
                {
                    i += 3;
                    continue;
                }
            }
        } else {
            break;
        }

        i += 1;
    }

    if path.segments.is_empty() {
        None
    } else {
        Some((path, &tts[i..]))
    }
}
