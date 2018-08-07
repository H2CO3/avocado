//! Lower-level operators of the MongoDB DML.

use std::borrow::Cow;
use bson::{ Bson, Document };

/// A query/filter condition.
#[derive(Debug, Clone, PartialEq)]
pub enum Filter {
    /// Matches if the field has the given value.
    Value(Bson),
    /// A sub-query of multiple path => filter specifiers.
    Multi(Vec<(Cow<'static, str>, Filter)>),

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
