//! Lower-level operators of the MongoDB DML.

use std::str;
use std::fmt;
use std::mem::size_of;
use std::borrow::Cow;
use linked_hash_map::LinkedHashMap;
use bson::{ Bson, Document };
use serde;
use serde::ser::{ Serialize, Serializer, SerializeSeq };
use serde::de::{ Deserialize, Deserializer, Visitor, SeqAccess };

/// A top-level filter document consisting of multiple path => filter specifiers
type FilterDoc = LinkedHashMap<Cow<'static, str>, Filter>;

/// A query/filter condition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Filter {
    /// Matches if the field has the given value.
    Value(Bson),
    /// A sub-query of multiple path => filter specifiers.
    Doc(FilterDoc),

    /// Matches if the field is equal to the given value.
    Eq(Bson),
    /// Matches if the field is not equal to the given value.
    Ne(Bson),
    /// Matches if the field is greater than the given value.
    Gt(Bson),
    /// Matches if the field is less than the given value.
    Lt(Bson),
    /// Matches if the field is greater than or equal to the given value.
    Gte(Bson),
    /// Matches if the field is less than or equal to the given value.
    Lte(Bson),
    /// Matches if the value of field is any of the specified values.
    In(Vec<Bson>),
    /// Matches if the value of field is none of the specified values.
    Nin(Vec<Bson>),

    /// Matches if the field satisfies all of the specified subqueries.
    And(Vec<Filter>),
    /// Matches if the field satisfies any of the specified subqueries.
    Or(Vec<Filter>),
    /// Matches if the field does not satisfy the specified subquery.
    Not(Box<Filter>),
    /// Matches if the field satisfies none of the specified subqueries.
    Nor(Vec<Filter>),

    /// If the argument is `true`, matches if the field exists in the enclosing
    /// document. If it is `false`, then matches if the field does not exist.
    Exists(bool),
    /// Matches if the type of the field is any of the specified types.
    Type(BsonType),

    // TODO(H2CO3): implement Expr
    // Expr(...),
    /// Matches if the value of the field satisfies the given JSON schema.
    JsonSchema(Document),
    /// Matches if the field is a string satisfying the given regular expression.
    Regex(Cow<'static, str>, RegexOpts),
    /// Matches if the field is an array containing all the specified values.
    All(Vec<Bson>),
    /// Matches if the field is an array containing at least one element that
    /// matches all of the specified subqueries.
    ElemMatch(Vec<Filter>),
    /// Matches if the field is an array whose length is the given value.
    Size(usize),

    // TODO(H2CO3): implement text search
    // Text(String, Language, TextFlags) -> TextFlags: case sensitive, diacritic sensitive
    // TODO(H2CO3): implement geospatial operators
    // TODO(H2CO3): implement bitwise operators
}

bitflags! {
    /// Non-deprecated BSON types.
    #[derive(Default)]
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

/// Bitflags for each regex option, along with its letter representation.
static OPTION_LETTERS: &[(RegexOpts, u8)] = &[
    (RegexOpts::IGNORE_CASE, b'i'),
    (RegexOpts::LINE_ANCHOR, b'm'),
    (RegexOpts::EXTENDED,    b'x'),
    (RegexOpts::DOT_NEWLINE, b's'),
];


impl Serialize for RegexOpts {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
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

        let s = str::from_utf8(&letters).map_err(S::Error::custom)?;

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
