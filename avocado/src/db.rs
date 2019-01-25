//! Represents a MongoDB database.

use mongodb::db::ThreadedDatabase;
use crate::{
    coll::Collection,
    doc::Doc,
    error::{ ErrorKind, Result, ResultExt },
};

#[cfg(feature = "schema_validation")]
use magnet_schema::BsonSchema;
#[cfg(feature = "schema_validation")]
use crate::uid::Uid;

/// Methods augmenting MongoDB `ThreadedDatabase` types.
pub trait DatabaseExt: ThreadedDatabase {
    /// Returns an existing collection without dropping/recreating it.
    fn existing_collection<T: Doc>(&self) -> Collection<T> {
        self.collection(T::NAME).into()
    }

    /// Creates a fresh, empty collection. **Drops any existing collection
    /// with the same name.** Recreates the collection with the `$jsonSchema`
    /// validator based on the `BsonSchema` impl of the document type. Also
    /// creates indexes specified via the `T::indexes()` method.
    #[cfg(feature = "schema_validation")]
    fn empty_collection<T>(&self) -> Result<Collection<T>>
        where T: Doc + BsonSchema,
              Uid<T>: BsonSchema,
    {
        use bson::Bson;
        use mongodb::CommandType;
        use crate::bsn::BsonExt;
        use crate::error::Error;

        self.drop_collection(T::NAME).chain("error dropping collection")?;

        // Add the `_id` field's spec to the top-level document's BSON schema.
        let schema = {
            let mut schema = T::bson_schema();
            let mut properties = schema.remove("properties")
                .ok_or_else(|| Error::new(
                    ErrorKind::MissingDocumentField,
                    format!("no properties in {}::bson_schema()", T::NAME)
                ))
                .and_then(Bson::try_into_doc)?;

            if properties.contains_key("_id") {
                let id_schema = properties.get_document("_id")?;

                if
                    *id_schema != Uid::<T>::bson_schema()
                    &&
                    *id_schema != Option::<Uid<T>>::bson_schema()
                {
                    return Err(Error::new(ErrorKind::BsonSchema, "BSON schema mismatch for _id"));
                }
            } else {
                properties.insert("_id", Uid::<T>::bson_schema());
            }

            schema.insert("properties", properties);
            schema
        };
        let command = doc! {
            "create": T::NAME,
            "validator": { "$jsonSchema": schema },
        };
        let reply = self.command(command, CommandType::CreateCollection, None)?;
        let err = || Error::new(
            ErrorKind::MongoDbError,
            format!("couldn't create {}: {}", T::NAME, reply)
        );
        let success = reply.get("ok").and_then(Bson::try_as_bool).ok_or_else(&err)?;

        if success {
            let coll = self.existing_collection();
            coll.create_indexes()?;
            Ok(coll)
        } else {
            Err(err())
        }
    }

    /// Creates a fresh, empty collection. **Drops any existing collection
    /// with the same name.** Recreates the collection **without** the BSON
    /// schema validator. Also creates indexes specified via the `T::indexes()`
    /// method.
    fn empty_collection_novalidate<T: Doc>(&self) -> Result<Collection<T>> {
        self.drop_collection(T::NAME).chain("error dropping collection")?;
        let coll = self.existing_collection();
        coll.create_indexes()?;
        Ok(coll)
    }
}

impl<T: ThreadedDatabase> DatabaseExt for T {}
