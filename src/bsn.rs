//! BSON serialization and deserialization helpers.

use std::borrow::Borrow;
use bson;
use bson::{ Bson, Document, ValueAccessError };
use serde::{ Serialize, Deserialize };
use error::{ Error, Result, ResultExt };

/// Methods for dynamically type-checking BSON.
pub trait BsonExt: Sized {
    /// Ensures that the Bson value is a `Document` and unwraps it.
    fn try_into_doc(self) -> Result<Document>;

    /// Ensures that the BSON value can be interpreted as a boolean,
    /// and performs the conversion.
    fn try_as_bool(&self) -> Option<bool>;
}

impl BsonExt for Bson {
    fn try_into_doc(self) -> Result<Document> {
        match self {
            Bson::Document(doc) => Ok(doc),
            value => Err(Error::with_cause(
                format!("expected Document, got {:?}", value.element_type()),
                ValueAccessError::UnexpectedType,
            ))
        }
    }

    #[cfg_attr(feature = "cargo-clippy", allow(float_cmp))]
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
}

/// Creates a BSON `Document` out of a serializable value.
/// TODO(H2CO3): validate that the value doesn't contain integers not
/// expressible by `i64`, because the BSON library just casts everything,
/// and overlfowing positive values may end up as negatives in the BSON.
pub fn serialize_document<T: Serialize>(value: &T) -> Result<Document> {
    bson::to_bson(value)
        .chain("BSON serialization error")
        .and_then(Bson::try_into_doc)
}

/// Creates an array of BSON `Document`s from an array of serializable values.
pub fn serialize_documents<T, I>(values: I) -> Result<Vec<Document>>
    where T: Serialize,
          I: Iterator,
          I::Item: Borrow<T>,
{
    values
        .into_iter()
        .map(|val| serialize_document(val.borrow()))
        .collect()
}

/// Creates a single strongly-typed document from loosely-typed BSON.
pub fn deserialize_document<T>(doc: Document) -> Result<T>
    where T: for<'a> Deserialize<'a>
{
    bson::from_bson(doc.into()).chain("can't deserialize document from BSON")
}

/// Creates an array of strongly-typed documents from loosely-typed BSON.
pub fn deserialize_documents<T>(docs: Vec<Document>) -> Result<Vec<T>>
    where T: for<'a> Deserialize<'a>
{
    docs.into_iter().map(deserialize_document).collect()
}
