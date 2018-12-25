//! BSON serialization and deserialization helpers.

use std::borrow::Borrow;
use serde_json::Value;
use bson;
use bson::{ Bson, Document, ValueAccessError };
use serde::{ Serialize, Deserialize };
use error::{ Error, Result };

/// Methods for dynamically type-checking JSON.
pub trait JsonExt: Sized {
    /// Ensures that this tree of values doesn't contain integers
    /// which are not expressible by `i64` (e.g. too big `u64`s).
    /// Since the `bson` crate just blindly casts integers to `i64`,
    /// the presence of such values would result in over- or underflow
    /// or truncation, leading to potentially hard-to-debug errors.
    /// Incidentally, this is also the reason why we have to do it via
    /// a round-trip through a JSON `Value` and not directly with `Bson`.
    ///
    /// If this check succeeds, `self` is converted into a `Bson` tree.
    /// Preservation of the order of keys in maps is ensured by the
    /// `preserve_order` feature of the `serde_json` crate.
    fn try_into_bson(self) -> Result<Bson>;
}

/// Methods for dynamically type-checking BSON.
pub trait BsonExt: Sized {
    /// Ensures that the BSON value is a `Document` and unwraps it.
    fn try_into_doc(self) -> Result<Document>;

    /// Ensures that the BSON value can be interpreted as a boolean,
    /// and performs the conversion.
    fn try_as_bool(&self) -> Option<bool>;
}

impl JsonExt for Value {
    fn try_into_bson(self) -> Result<Bson> {
        match self {
            // We need the value to be representable by either an `i64` or an `f64`.
            Value::Number(n) => if n.is_i64() || n.is_f64() {
                bson::to_bson(&n).map_err(Into::into)
            } else {
                Err(Error::new(
                    format!("Value `{}` can't be represented in BSON", n)
                ))
            },

            // Check transitively if every element of the array is correct.
            Value::Array(values) => values
                .into_iter()
                .map(JsonExt::try_into_bson)
                .collect::<Result<Vec<_>>>()
                .map(Bson::from),

            // Map keys are always OK because they're strings;
            // therefore, we only need to check the associated values.
            Value::Object(values) => values
                .into_iter()
                .map(|(k, v)| v.try_into_bson().map(|v| (k, v)))
                .collect::<Result<Document>>()
                .map(Bson::from_extended_document),

            // Anything else non-recursive is OK.
            value => Ok(value.into()),
        }
    }
}

impl BsonExt for Bson {
    #[allow(clippy::float_cmp)]
    fn try_as_bool(&self) -> Option<bool> {
        match *self {
            Bson::Boolean(b) => Some(b),
            Bson::I32(0) | Bson::I64(0) => Some(false),
            Bson::I32(1) | Bson::I64(1) => Some(true),
            Bson::FloatingPoint(x) if x == 0.0 => Some(false),
            Bson::FloatingPoint(x) if x == 1.0 => Some(true),
            _ => None,
        }
    }

    fn try_into_doc(self) -> Result<Document> {
        match self {
            Bson::Document(doc) => Ok(doc),
            value => Err(Error::with_cause(
                format!("expected Document, got {:?}", value.element_type()),
                ValueAccessError::UnexpectedType,
            ))
        }
    }
}

/// Creates a BSON `Document` out of a serializable value.
/// ```
/// # #[macro_use]
/// # extern crate bson;
/// # #[macro_use]
/// # extern crate serde_derive;
/// # extern crate avocado;
/// #
/// # use avocado::bsn::serialize_document;
/// # use std::{ u64, i64, i128 };
/// #
/// # fn main() {
/// #
/// #[derive(Serialize)]
/// struct Number { value: u64 };
///
/// #[derive(Serialize)]
/// struct BigNumber { value: i128 };
///
/// let good = Number { value: i64::MAX as u64 };
/// let bad_64 = Number { value: i64::MAX as u64 + 1 };
/// let bad_128 = BigNumber { value: 0 };
/// let bad_nodoc: i64 = 0;
///
/// assert_eq!(
///     serialize_document(&good).unwrap(),
///     doc!{ "value": i64::MAX }
/// );
/// assert!(serialize_document(&bad_64)
///         .unwrap_err()
///         .to_string()
///         .contains("can't be represented in BSON"));
/// assert!(serialize_document(&bad_128)
///         .unwrap_err()
///         .to_string()
///         .contains("i128 is not supported"));
/// assert!(serialize_document(&bad_nodoc)
///         .unwrap_err()
///         .to_string()
///         .contains("expected Document, got Integer64Bit"));
/// #
/// # }
/// ```
pub fn serialize_document<T: Serialize>(value: &T) -> Result<Document> {
    serde_json::to_value(value)
        .map_err(From::from)
        .and_then(JsonExt::try_into_bson)
        .and_then(BsonExt::try_into_doc)
}

/// Creates an array of `Document`s from an iterator over serializable values.
/// ```
/// # #[macro_use]
/// # extern crate bson;
/// # #[macro_use]
/// # extern crate serde_derive;
/// # extern crate avocado;
/// #
/// # use avocado::bsn::serialize_documents;
/// # use std::{ u64, i64 };
/// #
/// # fn main() {
/// #
/// #[derive(Serialize)]
/// struct Number { value: u64 };
///
/// let good = Number { value: i64::MAX as u64 };
/// let bad = Number { value: i64::MAX as u64 + 1 };
///
/// assert_eq!(serialize_documents::<Number, _>(vec![&good, &good]).unwrap(),
///            vec![
///                doc!{ "value": i64::MAX },
///                doc!{ "value": i64::MAX },
///            ]);
///
/// assert!(serialize_documents::<Number, _>(vec![&good, &bad])
///         .unwrap_err()
///         .to_string()
///         .contains("can't be represented in BSON"));
/// #
/// # }
pub fn serialize_documents<T, I>(values: I) -> Result<Vec<Document>>
    where T: Serialize,
          I: IntoIterator,
          I::Item: Borrow<T>,
{
    values
        .into_iter()
        .map(|val| serialize_document(val.borrow()))
        .collect()
}

/// Creates a single strongly-typed document from loosely-typed BSON.
/// ```
/// # #[macro_use]
/// # extern crate bson;
/// # #[macro_use]
/// # extern crate serde_derive;
/// # extern crate avocado;
/// #
/// # use avocado::bsn::deserialize_document;
/// # use std::{ u64, i64 };
/// #
/// # fn main() {
/// #
/// #[derive(Debug, PartialEq, Eq, Deserialize)]
/// struct Number { value: u64 };
///
/// let good = doc!{ "value": i64::MAX };
/// let bad = doc!{ "value": -1 };
///
/// assert_eq!(
///     deserialize_document::<Number>(good).unwrap(),
///     Number { value: i64::MAX as u64 }
/// );
/// assert!(deserialize_document::<Number>(bad)
///         .unwrap_err()
///         .to_string()
///         .contains("BSON decoding error, caused by: u64"));
/// #
/// # }
/// ```
pub fn deserialize_document<T>(doc: Document) -> Result<T>
    where T: for<'a> Deserialize<'a>
{
    bson::from_bson(doc.into()).map_err(From::from)
}

/// Creates an array of strongly-typed documents from loosely-typed BSON.
/// ```
/// # #[macro_use]
/// # extern crate bson;
/// # #[macro_use]
/// # extern crate serde_derive;
/// # extern crate avocado;
/// #
/// # use avocado::bsn::deserialize_documents;
/// # use std::{ u64, i64 };
/// #
/// # fn main() {
/// #
/// #[derive(Debug, PartialEq, Eq, Deserialize)]
/// struct Number { value: u64 };
///
/// let good_1 = doc!{ "value": 1337 };
/// let good_2 = doc!{ "value": i64::MAX };
/// let good_3 = doc!{ "value": 42 };
/// let bad    = doc!{ "value": i64::MIN };
///
/// assert_eq!(deserialize_documents::<Number>(vec![good_1, good_2]).unwrap(),
///           vec![
///               Number { value: 1337 },
///               Number { value: i64::MAX as u64 },
///           ]);
/// assert!(deserialize_documents::<Number>(vec![good_3, bad])
///         .unwrap_err()
///         .to_string()
///         .contains("BSON decoding error, caused by: u64"));
/// #
/// # }
/// ```
pub fn deserialize_documents<T>(docs: Vec<Document>) -> Result<Vec<T>>
    where T: for<'a> Deserialize<'a>
{
    docs.into_iter().map(deserialize_document).collect()
}
