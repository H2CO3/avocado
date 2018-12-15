//! Helper types for making the construction of filter, update, etc. documents
//! a little less stringly-typed.

use std::str;
use std::fmt;
use bson::Bson;
use serde::{
    ser::{ Serialize, Serializer, SerializeSeq },
    de::{ Deserialize, Deserializer, Visitor, SeqAccess },
};

/// Ordering, eg. keys within an index, or sorting documents yielded by a query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Order {
    /// Order smaller values first.
    Ascending  =  1,
    /// Order greater values first.
    Descending = -1,
}

/// The default ordering is `Ascending`.
impl Default for Order {
    fn default() -> Self {
        Order::Ascending
    }
}

/// This impl is provided so that you can use these more expressive ordering
/// names instead of the not very clear `1` and `-1` when constructing literal
/// BSON index documents, like this:
///
/// ```
/// # #[macro_use]
/// # extern crate bson;
/// # extern crate avocado;
/// #
/// # use avocado::dsl::literal::Order;
/// #
/// # fn main() {
/// let index = doc! {
///     "_id": Order::Ascending,
///     "zip": Order::Descending,
/// };
/// # }
/// ```
impl From<Order> for Bson {
    fn from(order: Order) -> Self {
        Bson::I32(order as _)
    }
}

bitflags! {
    /// Non-deprecated BSON types. For use with the `$type` operator.
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate bson;
    /// # extern crate avocado;
    /// #
    /// # use avocado::dsl::literal::BsonType;
    /// #
    /// # fn main() {
    /// let queries = bson!([
    ///     { "$type": BsonType::OBJECT_ID },
    ///     { "$type": [ BsonType::STRING, BsonType::default() ] },
    /// ]);
    /// assert_eq!(queries, bson!([{ "$type": "objectId" },
    ///                            { "$type": ["string", "null"] }]));
    /// # }
    /// ```
    pub struct BsonType: u16 {
        /// The `null` value.
        const NULL                  = 0b0000_0000_0000_0001;
        /// `true` or `false`.
        const BOOL                  = 0b0000_0000_0000_0010;
        /// Double-precision floating-point number.
        const DOUBLE                = 0b0000_0000_0000_0100;
        /// 32-bit signed integer.
        const INT                   = 0b0000_0000_0000_1000;
        /// 64-bit signed integer.
        const LONG                  = 0b0000_0000_0001_0000;
        /// 128-bit decimal number.
        const DECIMAL               = 0b0000_0000_0010_0000;
        /// Any of the 4 numeric types (`double`, `int`, `long`, `decimal`).
        const NUMBER                = 0b0000_0000_0011_1100;
        /// `ObjectId`.
        const OBJECT_ID             = 0b0000_0000_0100_0000;
        /// Timestamp.
        const TIMESTAMP             = 0b0000_0000_1000_0000;
        /// Date and time.
        const DATE                  = 0b0000_0001_0000_0000;
        /// String.
        const STRING                = 0b0000_0010_0000_0000;
        /// Regular expression and its matching options.
        const REGEX                 = 0b0000_0100_0000_0000;
        /// Binary data, BLOB.
        const BINARY                = 0b0000_1000_0000_0000;
        /// Array.
        const ARRAY                 = 0b0001_0000_0000_0000;
        /// Document or object.
        const DOCUMENT              = 0b0010_0000_0000_0000;
        /// JavaScript code.
        const JAVASCRIPT            = 0b0100_0000_0000_0000;
        /// JavaScript code with scope.
        const JAVASCRIPT_WITH_SCOPE = 0b1000_0000_0000_0000;
    }
}

/// The default BSON type is `null`.
impl Default for BsonType {
    fn default() -> Self {
        BsonType::NULL
    }
}

/// This is possible because encoding `BsonType` as a `Bson` never actually
/// fails (the in-memory tree serializer always succeeds unless the value
/// being serialized itself provokes an error, which our `BsonType` doesn't.)
impl From<BsonType> for Bson {
    fn from(bson_type: BsonType) -> Self {
        bson::to_bson(&bson_type).unwrap_or_default()
    }
}

/// All distinct BSON type bitflags, along with their string aliases.
static TYPE_NAMES: &[(BsonType, &str)] = &[
    (BsonType::NULL,                  "null"),
    (BsonType::BOOL,                  "bool"),
    (BsonType::DOUBLE,                "double"),
    (BsonType::INT,                   "int"),
    (BsonType::LONG,                  "int"),
    (BsonType::DECIMAL,               "decimal"),
    (BsonType::OBJECT_ID,             "objectId"),
    (BsonType::TIMESTAMP,             "timestamp"),
    (BsonType::DATE,                  "date"),
    (BsonType::STRING,                "string"),
    (BsonType::REGEX,                 "regex"),
    (BsonType::BINARY,                "binData"),
    (BsonType::ARRAY,                 "array"),
    (BsonType::DOCUMENT,              "object"),
    (BsonType::JAVASCRIPT,            "javascript"),
    (BsonType::JAVASCRIPT_WITH_SCOPE, "javascriptWithScope"),
];

impl Serialize for BsonType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::Error;

        match self.bits().count_ones() {
            0 => Err(S::Error::custom("at least one type must be specified")),
            1 => {
                for &(flag, name) in TYPE_NAMES {
                    if self.contains(flag) {
                        return serializer.serialize_str(name);
                    }
                }
                Err(S::Error::custom("found an unexpected flag"))
            }
            n => {
                let mut seq = serializer.serialize_seq(Some(n as usize))?;

                for &(flag, name) in TYPE_NAMES {
                    if self.contains(flag) {
                        seq.serialize_element(name)?;
                    }
                }

                seq.end()
            }
        }
    }
}

impl<'a> Deserialize<'a> for BsonType {
    fn deserialize<D: Deserializer<'a>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(BsonTypeVisitor)
    }
}

/// A `Visitor` for converting a BSON type alias or an array thereof to a `BsonType` bitflag.
#[derive(Debug, Clone, Copy)]
struct BsonTypeVisitor;

impl BsonTypeVisitor {
    /// Attempts to convert a BSON type alias to a `BsonType` bitflag.
    fn bitflag_for_name<E: serde::de::Error>(name: &str) -> Result<BsonType, E> {
        match TYPE_NAMES.iter().find(|&&(_, n)| n == name) {
            Some(&(flag, _)) => Ok(flag),
            None => Err(E::custom(format!("unknown BSON type alias: '{}'", name))),
        }
    }
}

impl<'a> Visitor<'a> for BsonTypeVisitor {
    type Value = BsonType;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a BSON type alias string or an array of BSON type alias strings")
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Self::bitflag_for_name(value)
    }

    fn visit_seq<A: SeqAccess<'a>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut flags = BsonType::empty();

        while let Some(name) = seq.next_element()? {
            flags |= Self::bitflag_for_name(name)?;
        }

        Ok(flags)
    }
}

bitflags! {
    /// Options for matching text against a regular expression.
    /// Useful with the `$regex` operator. E.g.:
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate bson;
    /// # extern crate avocado;
    /// #
    /// # use avocado::dsl::literal::RegexOpts;
    /// #
    /// # fn main() {
    /// let query = doc!{
    ///     "name": {
    ///         "$regex": "^Foo",
    ///         "$options": RegexOpts::LINE_ANCHOR | RegexOpts::IGNORE_CASE,
    ///     },
    ///     "address": {
    ///         "$regex": ".* street$",
    ///         "$options": RegexOpts::default(),
    ///     },
    /// };
    /// assert_eq!(query, doc!{
    ///     "name": {
    ///         "$regex": "^Foo",
    ///         "$options": "im",
    ///     },
    ///     "address": {
    ///         "$regex": ".* street$",
    ///         "$options": "",
    ///     },
    /// });
    /// # }
    /// ```
    #[derive(Default)]
    pub struct RegexOpts: u8 {
        /// Case insensitive matching.
        const IGNORE_CASE = 0b0000_0001;
        /// `^` and `$` match the beginning and the end of lines, not the whole string.
        const LINE_ANCHOR = 0b0000_0010;
        /// "extended" syntax, allows embedded whitespace and `#`-comments
        const EXTENDED    = 0b0000_0100;
        /// The `.` character matches newlines too.
        const DOT_NEWLINE = 0b0000_1000;
    }
}

/// See the explanation for `BsonType` as to why this impl is possible.
impl From<RegexOpts> for Bson {
    fn from(options: RegexOpts) -> Self {
        bson::to_bson(&options).unwrap_or_default()
    }
}

/// Bitflags for each regex option, along with its letter representation.
static OPTION_LETTERS: &[(RegexOpts, u8)] = &[
    (RegexOpts::IGNORE_CASE, b'i'),
    (RegexOpts::LINE_ANCHOR, b'm'),
    (RegexOpts::EXTENDED,    b'x'),
    (RegexOpts::DOT_NEWLINE, b's'),
];

impl Serialize for RegexOpts {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use std::mem::size_of;
        use serde::ser::Error;

        // can't have more than this many distinct bits/flags
        let mut letters = [0; size_of::<Self>() * 8];
        let mut n = 0;

        for &(option, letter) in OPTION_LETTERS {
            if self.contains(option) {
                letters[n] = letter;
                n += 1;
            }
        }

        let s = str::from_utf8(&letters[..n]).map_err(S::Error::custom)?;

        serializer.serialize_str(s)
    }
}

impl<'a> Deserialize<'a> for RegexOpts {
    fn deserialize<D: Deserializer<'a>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(RegexOptsVisitor)
    }
}

/// A visitor for deserializing `RegexOpts`.
#[derive(Debug, Clone, Copy)]
struct RegexOptsVisitor;

impl<'a> Visitor<'a> for RegexOptsVisitor {
    type Value = RegexOpts;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string containing one of [imxs]")
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        let mut options = RegexOpts::empty();

        for byte in value.bytes() {
            match OPTION_LETTERS.iter().find(|&&(_, b)| b == byte) {
                Some(&(option, _)) => options |= option,
                None => return Err(E::custom(format!("unexpected regex option: '{}'", byte as char))),
            }
        }

        Ok(options)
    }
}

/// Tells the `$currentDate` operator which type it should use for setting its fields.
///
/// ```
/// # #[macro_use]
/// # extern crate bson;
/// # extern crate avocado;
/// #
/// # use avocado::dsl::literal::DateTimeType;
/// #
/// # fn main() {
/// let update = doc!{
///     "$currentDate": {
///         "date": DateTimeType::Date,
///         "time": DateTimeType::Timestamp,
///     }
/// };
/// assert_eq!(update, doc!{
///     "$currentDate": {
///         "date": "date",
///         "time": "timestamp",
///     }
/// });
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DateTimeType {
    /// the `$currentDate` operator will set the field to a timestamp value.
    Timestamp,
    /// the `$currentDate` operator will set the field to a `Date` value.
    Date,
}

/// See the explanation for `BsonType` as to why this impl is possible.
impl From<DateTimeType> for Bson {
    fn from(ty: DateTimeType) -> Self {
        bson::to_bson(&ty).unwrap_or_default()
    }
}
