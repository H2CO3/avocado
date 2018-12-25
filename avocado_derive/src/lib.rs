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
use syn::{ DeriveInput, Data };
use self::error::{ Result, Error };

/// The top-level entry point of this proc-macro. Only here to be exported
/// and to handle `Result::Err` return values by `panic!()`ing.
#[proc_macro_derive(Doc)]
pub fn derive_avocado_doc(input: TokenStream) -> TokenStream {
    impl_avocado_doc(input).unwrap_or_else(|error| panic!("{}", error))
}

/// Implements `Doc` for the specified type.
fn impl_avocado_doc(input: TokenStream) -> Result<TokenStream> {
    let parsed_ast: DeriveInput = syn::parse(input)?;
    let ty = parsed_ast.ident;

    match parsed_ast.data {
        Data::Struct(s) => Ok(TokenStream::new()), // TODO(H2CO3): implement me
        _ => Err(Error::new(
            "Only `struct`s can be top-level `Doc`uments; consider wrapping this type in a struct with an `_id` field"
        )),
    }
}
