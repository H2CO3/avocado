//! Traits and types for describing the MongoDB DDL and DML.

use serde::{ Serialize, Deserialize };
use mongodb::{
    common::WriteConcern,
    coll::options::{
        IndexModel,
        FindOptions,
        CountOptions,
        DistinctOptions,
        AggregateOptions,
        InsertManyOptions,
    },
};
use magnet_schema::BsonSchema;

pub mod ops;
pub mod literal;

/// Implemented by top-level (direct collection member) documents only.
/// These types always have an associated top-level name and an `_id` field.
pub trait Doc: BsonSchema + Serialize + for<'a> Deserialize<'a> {
    /// The type of the unique IDs for the document. A good default choice
    /// is `ObjectId`. TODO(H2CO3): make it default to `ObjectId` (#29661).
    type Id: BsonSchema + Serialize + for <'a> Deserialize<'a>;

    /// The name of the collection within the database.
    const NAME: &'static str;

    /// Returns the specifications of the indexes created on the collection.
    /// If not provided, returns an empty vector, leading to the collection not
    /// bearing any user-defined indexes. (The `_id` field will still be
    /// indexed, though, as defined by MongoDB.)
    fn indexes() -> Vec<IndexModel> {
        Vec::new()
    }

    /// Options for a count-only query.
    fn count_options() -> CountOptions {
        Default::default()
    }

    /// Options for a `distinct` query.
    fn distinct_options() -> DistinctOptions {
        Default::default()
    }

    /// Aggregation pipeline options.
    fn aggregate_options() -> AggregateOptions {
        Default::default()
    }

    /// Options for a regular query.
    fn query_options() -> FindOptions {
        Default::default()
    }

    /// Options for single and batch insertions.
    fn insert_options() -> InsertManyOptions {
        Default::default()
    }

    /// Options for a delete operation.
    fn delete_options() -> WriteConcern {
        Default::default()
    }

    /// Options for a (strictly non-upsert) update operation.
    fn update_options() -> WriteConcern {
        Default::default()
    }

    /// Options for upserting.
    fn upsert_options() -> WriteConcern {
        Default::default()
    }
}
