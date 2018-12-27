//! Helper types for making the construction of filter, update, etc. documents
//! a little less stringly-typed.

use std::str;
use std::fmt;
use bson::{ Bson, to_bson };
use serde::{
    ser::{ Serialize, Serializer, SerializeSeq },
    de::{ Deserialize, Deserializer, Visitor, SeqAccess },
};

/// Ordering, for specifying in which order to sort results yielded by a query.
/// ```
/// # #[macro_use]
/// # extern crate bson;
/// # extern crate avocado;
/// #
/// # use avocado::literal::Order;
/// #
/// # fn main() {
/// let sorting = doc! {
///     "_id": Order::Ascending,
///     "zip": Order::Descending,
/// };
/// assert_eq!(sorting, doc!{
///     "_id":  1,
///     "zip": -1,
/// });
/// # }
/// ```
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
/// BSON index documents.
impl From<Order> for Bson {
    fn from(order: Order) -> Self {
        Bson::I32(order as _)
    }
}

/// ```
/// # #[macro_use]
/// # extern crate bson;
/// # extern crate avocado;
/// #
/// # use bson::to_bson;
/// # use avocado::prelude::*;
/// #
/// # fn main() -> AvocadoResult<()> {
/// #
/// assert_eq!(to_bson(&Order::Ascending)?, Bson::from(Order::Ascending));
/// assert_eq!(to_bson(&Order::Descending)?, Bson::from(Order::Descending));
/// #
/// # Ok(())
/// # }
/// ```
impl Serialize for Order {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_i32(*self as _)
    }
}

/// ```
/// # #[macro_use]
/// # extern crate bson;
/// # extern crate avocado;
/// #
/// # use bson::from_bson;
/// # use avocado::prelude::*;
/// #
/// # fn main() -> AvocadoResult<()> {
/// #
/// let asc_i32 = Bson::I32(1);
/// let desc_i64 = Bson::I64(-1);
/// let asc_float = Bson::FloatingPoint(1.0);
///
/// let bad_i32 = Bson::I32(0);
/// let bad_float = Bson::FloatingPoint(-2.0);
/// let bad_type = Bson::from("Ascending");
///
/// assert_eq!(from_bson::<Order>(asc_i32)?, Order::Ascending);
/// assert_eq!(from_bson::<Order>(desc_i64)?, Order::Descending);
/// assert_eq!(from_bson::<Order>(asc_float)?, Order::Ascending);
///
/// assert!(from_bson::<Order>(bad_i32)
///         .unwrap_err()
///         .to_string()
///         .contains("invalid ordering"));
/// assert!(from_bson::<Order>(bad_float)
///         .unwrap_err()
///         .to_string()
///         .contains("invalid ordering"));
/// assert!(from_bson::<Order>(bad_type)
///         .unwrap_err()
///         .to_string()
///         .contains("an integer expressing ordering"));
/// #
/// # Ok(())
/// # }
/// ```
impl<'a> Deserialize<'a> for Order {
    fn deserialize<D: Deserializer<'a>>(de: D) -> Result<Self, D::Error> {
        de.deserialize_i32(OrderVisitor)
    }
}

/// A serde visitor that produces an `Order` from +1 or -1.
struct OrderVisitor;

impl<'a> Visitor<'a> for OrderVisitor {
    type Value = Order;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "an integer expressing ordering: {} or {}",
            Order::Ascending as i32,
            Order::Descending as i32,
        )
    }

    fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
        if v == Order::Ascending as i64 {
            Ok(Order::Ascending)
        } else if v == Order::Descending as i64 {
            Ok(Order::Descending)
        } else {
            Err(E::custom(format!("invalid ordering: {}", v)))
        }
    }

    fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
        if v == Order::Ascending as u64 {
            Ok(Order::Ascending)
        } else {
            Err(E::custom(format!("invalid ordering: {}", v)))
        }
    }

    #[allow(clippy::float_cmp, clippy::cast_lossless, clippy::cast_precision_loss)]
    fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
        if v == Order::Ascending as i32 as f64 {
            Ok(Order::Ascending)
        } else if v == Order::Descending as i32 as f64 {
            Ok(Order::Descending)
        } else {
            Err(E::custom(format!("invalid ordering: {}", v)))
        }
    }
}

/// An index type, applied to a single indexed field.
/// ```
/// # #[macro_use]
/// # extern crate bson;
/// # extern crate avocado;
/// #
/// # use avocado::literal::{ IndexType, Order };
/// #
/// # fn main() {
/// let patient_index = doc!{
///     "description": IndexType::Text,
///     "body.mass": IndexType::Ordered(Order::Ascending),
///     "birth_date.year": IndexType::Ordered(Order::Descending),
///     "address_gps_coords": IndexType::Geo2DSphere,
/// };
/// assert_eq!(patient_index, doc!{
///     "description": "text",
///     "body.mass": 1,
///     "birth_date.year": -1,
///     "address_gps_coords": "2dsphere",
/// });
/// #
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndexType {
    /// An ordered index field.
    Ordered(Order),
    /// A language-specific textual index, most useful for freetext searches.
    Text,
    /// Hashed index for hash-based sharding.
    Hashed,
    /// 2D geospatial index with planar (Euclidean) geometry.
    Geo2D,
    /// 2D geospatial index with spherical geometry.
    Geo2DSphere,
    /// 2D geospatial index optimized for very small areas.
    GeoHaystack,
}

impl From<IndexType> for Bson {
    fn from(index_type: IndexType) -> Self {
        match index_type {
            IndexType::Ordered(order) => Bson::from(order),
            IndexType::Text           => Bson::from("text"),
            IndexType::Hashed         => Bson::from("hashed"),
            IndexType::Geo2D          => Bson::from("2d"),
            IndexType::Geo2DSphere    => Bson::from("2dsphere"),
            IndexType::GeoHaystack    => Bson::from("geoHaystack"),
        }
    }
}

/// ```
/// # #[macro_use]
/// # extern crate bson;
/// # extern crate avocado;
/// #
/// # use bson::{ to_bson, from_bson };
/// # use avocado::prelude::*;
/// #
/// # fn main() -> AvocadoResult<()> {
/// #
/// let asc = IndexType::Ordered(Order::Ascending);
/// let desc = IndexType::Ordered(Order::Descending);
/// let haystack = IndexType::GeoHaystack;
/// let text = IndexType::Text;
/// let planar_2d = IndexType::Geo2D;
/// let spherical_2d = IndexType::Geo2DSphere;
///
/// assert_eq!(to_bson(&asc)?, Bson::from(asc));
/// assert_eq!(to_bson(&desc)?, Bson::from(desc));
/// assert_eq!(to_bson(&haystack)?, Bson::from(haystack));
/// assert_eq!(to_bson(&text)?, Bson::from(text));
/// assert_eq!(to_bson(&planar_2d)?, Bson::from(planar_2d));
/// assert_eq!(to_bson(&spherical_2d)?, Bson::from(spherical_2d));
/// #
/// # Ok(())
/// # }
/// ```
impl Serialize for IndexType {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        Bson::from(*self).serialize(ser)
    }
}

/// ```
/// # #[macro_use]
/// # extern crate bson;
/// # extern crate avocado;
/// #
/// # use bson::from_bson;
/// # use avocado::prelude::*;
/// #
/// # fn main() -> AvocadoResult<()> {
/// #
/// let asc_i32 = Bson::I32(1);
/// let desc_i64 = Bson::I64(-1);
/// let asc_float = Bson::FloatingPoint(1.0);
/// let text = Bson::from("text");
/// let spherical_2d = Bson::from("2dsphere");
/// let hashed = Bson::from("hashed");
///
/// let bad_i64 = Bson::I64(0);
/// let bad_float = Bson::FloatingPoint(3.14);
/// let bad_str = Bson::from("Ascending");
/// let bad_type = Bson::Boolean(true);
///
/// assert_eq!(from_bson::<IndexType>(asc_i32)?,
///            IndexType::Ordered(Order::Ascending));
/// assert_eq!(from_bson::<IndexType>(desc_i64)?,
///            IndexType::Ordered(Order::Descending));
/// assert_eq!(from_bson::<IndexType>(asc_float)?,
///            IndexType::Ordered(Order::Ascending));
/// assert_eq!(from_bson::<IndexType>(text)?,
///            IndexType::Text);
/// assert_eq!(from_bson::<IndexType>(spherical_2d)?,
///            IndexType::Geo2DSphere);
/// assert_eq!(from_bson::<IndexType>(hashed)?,
///            IndexType::Hashed);
///
/// assert!(from_bson::<IndexType>(bad_i64)
///         .unwrap_err()
///         .to_string()
///         .contains("invalid ordering"));
/// assert!(from_bson::<IndexType>(bad_float)
///         .unwrap_err()
///         .to_string()
///         .contains("invalid ordering"));
/// assert!(from_bson::<IndexType>(bad_str)
///         .unwrap_err()
///         .to_string()
///         .contains("unrecognized index type"));
/// assert!(from_bson::<IndexType>(bad_type)
///         .unwrap_err()
///         .to_string()
///         .contains("an ordering integer or an index type string"));
/// #
/// # Ok(())
/// # }
/// ```
impl<'a> Deserialize<'a> for IndexType {
    fn deserialize<D: Deserializer<'a>>(de: D) -> Result<Self, D::Error> {
        de.deserialize_any(IndexTypeVisitor)
    }
}

/// A serde visitor that produces an `IndexType` from its raw representation,
/// which is either an ordering integer (+/- 1) or an index type string.
struct IndexTypeVisitor;

impl<'a> Visitor<'a> for IndexTypeVisitor {
    type Value = IndexType;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an ordering integer or an index type string")
    }

    fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
        OrderVisitor.visit_i64(v).map(IndexType::Ordered)
    }

    fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
        OrderVisitor.visit_u64(v).map(IndexType::Ordered)
    }

    fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
        OrderVisitor.visit_f64(v).map(IndexType::Ordered)
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(match v {
            "text"        => IndexType::Text,
            "hashed"      => IndexType::Hashed,
            "2d"          => IndexType::Geo2D,
            "2dsphere"    => IndexType::Geo2DSphere,
            "geoHaystack" => IndexType::GeoHaystack,
            _ => Err(E::custom(format!("unrecognized index type: {}", v)))?
        })
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
    /// # use avocado::literal::BsonType;
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
        to_bson(&bson_type).unwrap_or_default()
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
    /// # use avocado::literal::RegexOpts;
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
        to_bson(&options).unwrap_or_default()
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
/// # use avocado::literal::DateTimeType;
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
        to_bson(&ty).unwrap_or_default()
    }
}
