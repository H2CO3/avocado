//! Lower-level operators of the MongoDB DML.

use std::str;
use std::fmt;
use std::i64;
use std::mem::size_of;
use std::borrow::Cow;
use std::iter::{ FromIterator, DoubleEndedIterator, ExactSizeIterator };
use linked_hash_map::{ self, LinkedHashMap };
use bson::{ Bson, Document };
use serde;
use serde::ser::{ Serialize, Serializer, SerializeSeq, SerializeMap };
use serde::de::{ Deserialize, Deserializer, Visitor, SeqAccess };

/// A top-level filter document consisting of multiple path => filter
/// specifiers and respecting the order of insertion during iteration.
#[cfg_attr(feature = "cargo-clippy", allow(stutter))]
#[derive(Debug, Clone, Default, PartialEq, /* Serialize */)]
pub struct FilterDoc(LinkedHashMap<Cow<'static, str>, Filter>);

impl FilterDoc {
    /// Creates an empty filter document.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty filter document with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        FilterDoc(LinkedHashMap::with_capacity(capacity))
    }

    /// Returns the current capacity of the document.
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Returns the number of entries (key-value pairs) in the document.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if and only if the document contains no entries.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Reserves additional capacity for the document.
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional)
    }

    /// Inserts a key and a value into the document. If the key already
    /// exists, returns the previous value associated with it.
    pub fn insert(&mut self, key: Cow<'static, str>, value: Filter) -> Option<Filter> {
        self.0.insert(key, value)
    }

    /// Returns `true` if and only if the document contains the specified key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    /// Returns a reference to the subquery associated with the key.
    pub fn get(&self, key: &str) -> Option<&Filter> {
        self.0.get(key)
    }

    /// Returns a mutable reference to the subquery associated with the key.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Filter> {
        self.0.get_mut(key)
    }

    /// Removes the subquery associated with the key and returns it.
    pub fn remove(&mut self, key: &str) -> Option<Filter> {
        self.0.remove(key)
    }

    /// Removes all key-value pairs, leaving the document in an empty state.
    pub fn clear(&mut self) {
        self.0.clear()
    }
}

/// TODO(H2CO3): this should be `#[derive]`d, but currently the `bson` crate
/// has a bug and it serializes a newtype struct as a 1-element array, so we
/// must manually delegate to the wrapped hash map.
impl Serialize for FilterDoc {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<K, V> FromIterator<(K, V)> for FilterDoc
    where K: Into<Cow<'static, str>>,
          V: Into<Filter>
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        FilterDoc(iter.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
    }
}

impl<K, V> Extend<(K, V)> for FilterDoc
    where K: Into<Cow<'static, str>>,
          V: Into<Filter>
{
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        self.0.extend(iter.into_iter().map(|(k, v)| (k.into(), v.into())))
    }
}

impl IntoIterator for FilterDoc {
    type Item = (Cow<'static, str>, Filter);
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter())
    }
}

impl<'a> IntoIterator for &'a FilterDoc {
    type Item = (&'a str, &'a Filter);
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter(self.0.iter())
    }
}

impl<'a> IntoIterator for &'a mut FilterDoc {
    type Item = (&'a str, &'a mut Filter);
    type IntoIter = IterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        IterMut(self.0.iter_mut())
    }
}

/// An owning iterator over the entries of a `FilterDoc`.
/// Yields entries in order of insertion.
#[derive(Clone)]
pub struct IntoIter(linked_hash_map::IntoIter<Cow<'static, str>, Filter>);

impl Iterator for IntoIter {
    type Item = (Cow<'static, str>, Filter);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl DoubleEndedIterator for IntoIter {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl ExactSizeIterator for IntoIter {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl fmt::Debug for IntoIter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FilterDoc::IntoIter({} entries)", self.len())
    }
}

/// A borrowing iterator over the entries of a `FilterDoc`.
/// Yields entries in order of insertion.
#[derive(Clone)]
pub struct Iter<'a>(linked_hash_map::Iter<'a, Cow<'static, str>, Filter>);

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a str, &'a Filter);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(k, v)| (k.as_ref(), v))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<'a> DoubleEndedIterator for Iter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(|(k, v)| (k.as_ref(), v))
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a> fmt::Debug for Iter<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FilterDoc::Iter({} entries)", self.len())
    }
}

/// A mutably borrowing iterator over the entries of a `FilterDoc`.
/// Yields entries in order of insertion.
pub struct IterMut<'a>(linked_hash_map::IterMut<'a, Cow<'static, str>, Filter>);

impl<'a> Iterator for IterMut<'a> {
    type Item = (&'a str, &'a mut Filter);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(k, v)| (k.as_ref(), v))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<'a> DoubleEndedIterator for IterMut<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(|(k, v)| (k.as_ref(), v))
    }
}

impl<'a> ExactSizeIterator for IterMut<'a> {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a> fmt::Debug for IterMut<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FilterDoc::IterMut({} entries)", self.len())
    }
}

/// A query/filter condition.
#[derive(Debug, Clone, PartialEq)]
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
    ElemMatch(FilterDoc),
    /// Matches if the field is an array whose length is the given value.
    Size(usize),

    // TODO(H2CO3): implement text search
    // Text(String, Language, TextFlags) -> TextFlags: case sensitive, diacritic sensitive
    // TODO(H2CO3): implement geospatial operators
    // TODO(H2CO3): implement bitwise operators
}

impl Filter {
    /// Serializes a 1-entry map. Helper for the `Serialize` impl.
    fn serialize_map<V: Serialize, S: Serializer>(serializer: S, key: &str, value: V) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(key, &value)?;
        map.end()
    }
}

/// `Filter::from(some_bson_value)` results in `Filter::Value(some_bson_value)`.
impl<T: Into<Bson>> From<T> for Filter {
    fn from(value: T) -> Self {
        Filter::Value(value.into())
    }
}

/// `Filter::from(FilterDoc)` yields a `Filter::Doc(...)`.
impl From<FilterDoc> for Filter {
    fn from(doc: FilterDoc) -> Self {
        Filter::Doc(doc)
    }
}

impl Serialize for Filter {
    #[cfg_attr(feature = "cargo-clippy", allow(cast_possible_truncation))]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use self::Filter::*;

        match *self {
            Value(ref bson) => bson.serialize(serializer),
            Doc(ref doc) => doc.serialize(serializer),

            Eq(ref bson) => Self::serialize_map(serializer, "$eq", bson),
            Ne(ref bson) => Self::serialize_map(serializer, "$ne", bson),
            Gt(ref bson) => Self::serialize_map(serializer, "$gt", bson),
            Lt(ref bson) => Self::serialize_map(serializer, "$lt", bson),
            Gte(ref bson) => Self::serialize_map(serializer, "$gte", bson),
            Lte(ref bson) => Self::serialize_map(serializer, "$lte", bson),
            In(ref array) => Self::serialize_map(serializer, "$in", array),
            Nin(ref array) => Self::serialize_map(serializer, "$nin", array),

            And(ref queries) => Self::serialize_map(serializer, "$and", queries),
            Or(ref queries) => Self::serialize_map(serializer, "$or", queries),
            Nor(ref queries) => Self::serialize_map(serializer, "$nor", queries),
            Not(ref query) => Self::serialize_map(serializer, "$not", query),

            Exists(b) => Self::serialize_map(serializer, "$exists", b as i32),
            Type(types) => Self::serialize_map(serializer, "$type", types),

            JsonSchema(ref doc) => Self::serialize_map(serializer, "$jsonSchema", doc),
            Regex(ref pattern, ref options) => {
                if options.is_empty() {
                    Self::serialize_map(serializer, "$regex", pattern)
                } else {
                    let mut map = serializer.serialize_map(Some(2))?;
                    map.serialize_entry("$regex", pattern)?;
                    map.serialize_entry("$options", options)?;
                    map.end()
                }
            }

            All(ref array) => Self::serialize_map(serializer, "$all", array),
            ElemMatch(ref queries) => Self::serialize_map(serializer, "$elemMatch", queries),
            Size(size) => {
                use serde::ser::Error;

                if size_of::<usize>() >= size_of::<i64>() && size > i64::MAX as usize {
                    Err(S::Error::custom(format!("{{ $size: {} }} overflows i64", size)))
                } else {
                    Self::serialize_map(serializer, "$size", size as i64)
                }
            },
        }
    }
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

/// Convenience macro for constructing a `Filter`.
///
/// ## Example:
///
/// ```
/// # #[macro_use] extern crate avocado;
/// #
/// # use avocado::dsl::filter::*;
/// # use avocado::dsl::filter::Filter::*;
/// #
/// # fn main() {
/// let repo_filter = filter! {
///     name: regex("^Avocado.*$"),
///     author.username: "H2CO3",
///     release_date: filter! {
///         year: 2018,
///     },
///     stargazers: Type(BsonType::ARRAY),
///     commits: And(vec![gte(42), lte(43)]),
///     downloads: ne(1337) // trailing comma is allowed but optional
/// };
/// # }
/// ```
#[macro_export]
macro_rules! filter {
    ($($first:ident $(.$rest:ident)*: $value:expr,)*) => ({
        let mut doc = $crate::dsl::filter::FilterDoc::new();
        $(
            doc.insert(
                concat!(stringify!($first), $(".", stringify!($rest))*).into(),
                $value.into()
            );
        )*
        doc
    });
    ($($first:ident $(.$rest:ident)*: $value:expr),*) => {
        filter!{ $($first $(.$rest)*: $value,)* }
    };
}

/// Helper macro for implementing the generic convenience "constructor"
/// functions that make it possible to create `Filter`s from values
/// without always calling `.into()`.
macro_rules! impl_filter_ctor {
    ($($function:ident -> $variant:ident;)*) => {
        impl_filter_ctor_internal!(
            $(
                $function: concat!("$", stringify!($function))
                =>
                $variant: stringify!($variant);
            )*
        );
    }
}

/// Helper for the above helper. Helperception! Necessary only because
/// stringifying identifiers and interpolating them into docstrings is hard.
macro_rules! impl_filter_ctor_internal {
    ($($function:ident: $fn_name:expr => $variant:ident: $var_name:expr;)*) => ($(
        #[doc = "Convenience helper function for constructing a `"]
        #[doc = $fn_name]
        #[doc = "` filter without needing to write `"]
        #[doc = $var_name]
        #[doc = "(value.into())` explicitly."]
        pub fn $function<T: Into<Bson>>(value: T) -> Filter {
            Filter::$variant(value.into())
        }
    )*)
}

impl_filter_ctor! {
    eq  -> Eq;
    ne  -> Ne;
    gt  -> Gt;
    lt  -> Lt;
    gte -> Gte;
    lte -> Lte;
}

/// Convenience helper function for constructing a `$regex` filter from a
/// string-like value and no options.
pub fn regex<S: Into<Cow<'static, str>>>(pattern: S) -> Filter {
    regex_opts(pattern, RegexOpts::empty())
}

/// Convenience helper function for constructing a `$regex` filter from a
/// string-like value and the specified options.
pub fn regex_opts<S: Into<Cow<'static, str>>>(pattern: S, options: RegexOpts) -> Filter {
    Filter::Regex(pattern.into(), options)
}

#[cfg(test)]
mod tests {
    extern crate serde_json;

    #[test]
    fn test_filter_macro() {
        use super::*;
        use super::Filter::*;

        let repo_filter = filter! {
            name: regex("^Avocado.*$"),
            author.username: "H2CO3",
            release_date: filter! {
                year: 2018,
            },
            stargazers: Type(BsonType::ARRAY),
            commits: And(vec![gte(42), lte(43)]),
            downloads: ne(1337)
        };
        println!("{}", serde_json::to_string_pretty(&repo_filter).unwrap());
    }
}
