//! Represents a MongoDB database.

use bson::Bson;
use mongodb::CommandType;
use mongodb::db::ThreadedDatabase;
use magnet_schema::BsonSchema;
use bsn::BsonExt;
use coll::Collection;
use dsl::Doc;
use error::{ Error, Result, ResultExt };

/// Methods augmenting MongoDB `ThreadedDatabase` types.
pub trait DatabaseExt: ThreadedDatabase {
    /// Returns an existing collection without dropping/recreating it.
    fn existing_collection<T: Doc>(&self) -> Collection<T> {
        self.collection(T::NAME).into()
    }

    /// Creates a fresh, empty collection. **Drops any existing collection
    /// with the same name.** Recreates the collection with the `$jsonSchema`
    /// validator based on the `BsonSchema` impl of the document type.
    fn empty_collection<T: Doc>(&self) -> Result<Collection<T>> {
        self.drop_collection(T::NAME).chain("error dropping collection")?;

        // Add the `_id` field's spec to the top-level document's BSON schema.
        let schema = {
            let mut schema = T::bson_schema();
            let mut properties = schema.remove("properties")
                .ok_or_else(|| Error::new(format!("no properties in {}::bson_schema()", T::NAME)))
                .and_then(Bson::into_doc)?;

            if properties.contains_key("_id") {
                if properties.get_document("_id")? != &T::Id::bson_schema() {
                    return Err(Error::new("BSON schema mismatch for _id"));
                }
            } else {
                properties.insert("_id", T::Id::bson_schema());
            }

            schema.insert("properties", properties);
            schema
        };
        let command = doc! {
            "create": T::NAME,
            "validator": { "$jsonSchema": schema },
        };
        let reply = self.command(command, CommandType::CreateCollection, None)?;
        let err = || Error::new(format!("couldn't create {}: {}", T::NAME, reply));
        let success = reply.get("ok").and_then(Bson::as_bool).ok_or_else(&err)?;

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
    /// schema validator.
    fn empty_collection_novalidate<T: Doc>(&self) -> Result<Collection<T>> {
        self.drop_collection(T::NAME).chain("error dropping collection")?;
        let coll = self.existing_collection();
        coll.create_indexes()?;
        Ok(coll)
    }
}

impl<T: ThreadedDatabase> DatabaseExt for T {}
