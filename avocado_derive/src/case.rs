//! This code was adapted from Serde's `serde_derive_internals` crate.
//! This was necessary because Serde's `rename_all` attribute does not
//! follow Unicode segmentation rules, and it also ignores non-conventional
//! field and variant names (it assumes that fields are always `snake_case`
//! and that variants are always `UpperCamelCase`). Therefore, using a Unicode
//! segmentation + case conversion crate such as `heck` was not an option.
//!
//! Original license header is reproduced below:

// Copyright 2017 Serde Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::str::FromStr;
use crate::error::{ Error, Result };
use self::RenameRule::*;

/// A renaming convention, as defined by Serde.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenameRule {
    /// Rename direct children to "lowercase" style.
    LowerCase,
    /// Rename direct children to "UPPERCASE" style.
    Uppercase,
    /// Rename direct children to "PascalCase" style, as typically used for enum variants.
    PascalCase,
    /// Rename direct children to "camelCase" style.
    CamelCase,
    /// Rename direct children to "snake_case" style, as commonly used for fields.
    SnakeCase,
    /// Rename direct children to "SCREAMING_SNAKE_CASE" style, as commonly used for constants.
    ScreamingSnakeCase,
    /// Rename direct children to "kebab-case" style.
    KebabCase,
    /// Rename direct children to "SCREAMING-KEBAB-CASE" style.
    ScreamingKebabCase,
}

impl RenameRule {
    /// Returns a string which is the given field name, renamed according
    /// to the rule that is `self`.
    pub fn apply_to_field(self, field: String) -> String {
        match self {
            LowerCase | SnakeCase => field,
            Uppercase => field.to_ascii_uppercase(),
            PascalCase => {
                let mut pascal = String::new();
                let mut capitalize = true;
                for ch in field.chars() {
                    if ch == '_' {
                        capitalize = true;
                    } else if capitalize {
                        pascal.push(ch.to_ascii_uppercase());
                        capitalize = false;
                    } else {
                        pascal.push(ch);
                    }
                }
                pascal
            }
            CamelCase => {
                let pascal = PascalCase.apply_to_field(field);
                pascal[..1].to_ascii_lowercase() + &pascal[1..]
            }
            ScreamingSnakeCase => field.to_ascii_uppercase(),
            KebabCase => field.replace('_', "-"),
            ScreamingKebabCase => ScreamingSnakeCase.apply_to_field(field).replace('_', "-"),
        }
    }
}

impl FromStr for RenameRule {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "lowercase"            => Ok(LowerCase),
            "UPPERCASE"            => Ok(Uppercase),
            "PascalCase"           => Ok(PascalCase),
            "camelCase"            => Ok(CamelCase),
            "snake_case"           => Ok(SnakeCase),
            "SCREAMING_SNAKE_CASE" => Ok(ScreamingSnakeCase),
            "kebab-case"           => Ok(KebabCase),
            "SCREAMING-KEBAB-CASE" => Ok(ScreamingKebabCase),
            _ => err_fmt!("unknown `rename_all` rule: {}", s),
        }
    }
}
