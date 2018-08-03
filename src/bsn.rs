//! BSON serialization and deserialization helpers.

use bson;
use bson::{ Bson, Document, ValueAccessError };
use serde::{ Serialize, Deserialize };
use error::{ Error, Result, ResultExt };

/// Methods for dynamically type-checking BSON.
pub trait BsonExt: Sized {
    /// Ensures that the Bson value is a `Document` and unwraps it.
    fn into_doc(self) -> Result<Document>;

    /// Ensures that the BSON value can be interpreted as a boolean,
    /// and performs the conversion.
    fn as_bool(&self) -> Result<bool>;
}

impl BsonExt for Bson {
    fn into_doc(self) -> Result<Document> {
        match self {
            Bson::Document(doc) => Ok(doc),
            value => Err(Error::with_cause(
                format!("expected Document, got {:?}", value.element_type()),
                ValueAccessError::UnexpectedType,
            ))
        }
    }

    #[cfg_attr(feature = "cargo-clippy", allow(float_cmp))]
    fn as_bool(&self) -> Result<bool> {
        match *self {
            Bson::Boolean(b) => Ok(b),
            Bson::I32(0) | Bson::I64(0) => Ok(false),
            Bson::I32(1) | Bson::I64(1) => Ok(true),
            Bson::FloatingPoint(x) if x == 0.0 => Ok(false),
            Bson::FloatingPoint(x) if x == 1.0 => Ok(true),
            _ => Err(Error::new(format!("can't convert {} to Boolean", self))),
        }
    }
}

/// Creates a BSON `Document` out of a serializable value.
/// TODO(H2CO3): validate that the value doesn't contain integers not
/// expressible by `i64`, because the BSON library just casts everything,
/// and overlfowing positive values may end up as negatives in the BSON.
pub fn serialize_document<T: Serialize>(value: &T) -> Result<Document> {
    bson::to_bson(value).chain("BSON serialization error").and_then(Bson::into_doc)
}

/// Creates an array of BSON `Document`s from an array of serializable values.
pub fn serialize_documents<T: Serialize>(values: &[T]) -> Result<Vec<Document>> {
    values.iter().map(serialize_document).collect()
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
