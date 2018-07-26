//! BSON serialization and deserialization helpers.

use bson;
use bson::{ Bson, Document };
use serde::{ Serialize, Deserialize };
use error::{ Error, Result, ResultExt };

/// Creates a BSON `Document` out of a serializable value.
/// TODO(H2CO3): validate that the value doesn't contain integers not
/// expressible by `i64`, because the BSON library just casts everything,
/// and overlfowing positive values may end up as negatives in the BSON.
pub fn serialize_document<T: Serialize>(value: &T) -> Result<Document> {
    let bson = bson::to_bson(value).chain("BSON serialization error")?;
    match bson {
        Bson::Document(doc) => Ok(doc),
        _ => Err(Error::new("value didn't encode to a document/object"))?,
    }
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
