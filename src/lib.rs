//! Avocado: the strongly-typed MongoDB driver

#![doc(html_root_url = "https://docs.rs/avocado/0.0.3")]
#![deny(missing_debug_implementations, missing_copy_implementations,
        trivial_casts, trivial_numeric_casts,
        unsafe_code,
        unstable_features,
        unused_import_braces, unused_qualifications, missing_docs)]
#![cfg_attr(feature = "cargo-clippy",
            allow(single_match, match_same_arms, match_ref_pats,
                  clone_on_ref_ptr, needless_pass_by_value))]
#![cfg_attr(feature = "cargo-clippy",
            deny(wrong_pub_self_convention, used_underscore_binding,
                 stutter, similar_names, pub_enum_variant_names,
                 missing_docs_in_private_items,
                 non_ascii_literal, unicode_not_nfc,
                 result_unwrap_used, option_unwrap_used,
                 option_map_unwrap_or_else, option_map_unwrap_or, filter_map,
                 shadow_unrelated, shadow_reuse, shadow_same,
                 int_plus_one, string_add_assign, if_not_else,
                 invalid_upcast_comparisons,
                 cast_precision_loss, cast_lossless,
                 cast_possible_wrap, cast_possible_truncation,
                 mutex_integer, mut_mut, items_after_statements,
                 print_stdout, mem_forget, maybe_infinite_iter))]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate bson;
extern crate mongodb_h2co3;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate magnet_schema;
extern crate backtrace;
extern crate linked_hash_map;

pub mod db;
pub mod coll;
pub mod cursor;
pub mod dsl;
pub mod bsn;
pub mod utils;
pub mod error;
