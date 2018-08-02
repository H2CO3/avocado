//! Traits and types for describing the MongoDB DDL and DML.

use bson::{ Bson, Document };
use serde::{ Serialize, Deserialize };
use mongodb::coll::options::{ FindOptions, CountOptions, AggregateOptions };
use mongodb::coll::options::{ IndexModel, InsertManyOptions };
use magnet_schema::BsonSchema;

/// Implemented by top-level (direct collection member) documents only.
/// These types always have an associated top-level name and an `_id` field.
pub trait Doc: BsonSchema + Serialize + for<'a> Deserialize<'a> {
    /// The type of the unique IDs for the document. A good default choice
    /// is `ObjectId`. TODO(H2CO3): make it default to `ObjectId` (#29661).
    type Id: for <'a> Deserialize<'a>;

    /// The name of the collection within the database.
    const NAME: &'static str;

    /// Returns the specifications of the indexes created on the collection.
    /// If not provided, returns an empty vector, leading to the collection not
    /// bearing any user-defined indexes. (The `_id` field will still be
    /// indexed, though, as defined by MongoDB.)
    fn indexes() -> Vec<IndexModel> {
        Vec::new()
    }

    /// If required, additional read and write options can be provided here.
    /// Returns `<Options as Default>::default()` by default.
    fn options() -> Options {
        Default::default()
    }
}

/// Encapsulates the options for querying collections and inserting into them.
/// TODO(H2CO3): uncomment the derive below once mongodb driver is unfuckenated.
#[derive(Debug, Clone, Default, /* PartialEq */)]
pub struct Options {
    /// Options for reading from (querying) a collection.
    pub find_options: FindOptions,
    /// Options for counting documents in a collection.
    pub count_options: CountOptions,
    /// Options for running an aggregation pipeline.
    pub aggregate_options: AggregateOptions,
    /// Options for writing (inserting/updating/upserting in) a collection.
    pub write_options: InsertManyOptions,
}

/// Ordering, eg. keys within an index, or sorting documents yielded by a query.
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
/// BSON index documents, like this:
///
/// ```
/// # #[macro_use]
/// # extern crate bson;
/// # extern crate avocado;
/// #
/// # use avocado::dsl::Order;
/// #
/// # fn main() {
/// let index = doc! {
///     "_id": Order::Ascending,
///     "zip": Order::Descending,
/// };
/// # }
/// ```
impl From<Order> for Bson {
    fn from(order: Order) -> Self {
        Bson::I32(order as _)
    }
}

/// DSL types which can be converted to a raw BSON document for use with MongoDB.
pub trait ToDocument {
    /// Returns the raw MongoDB DSL BSON representation of this value.
    fn to_document(&self) -> Document;
}

/// DSL types which can be converted to a vector of raw BSON documents for use with MongoDB.
pub trait ToDocuments {
    /// Returns the raw MongoDB DSL BSON representation of this value.
    fn to_documents(&self) -> Vec<Document>;
}

/// A trait marking objects used for querying a collection.
pub trait Query<T: Doc>: ToDocument {
    /// The type of the results obtained by executing the query. Often it's just
    /// the document type, `T`. TODO(H2CO3): make it default to `T` (#29661).
    type Output: for<'a> Deserialize<'a>;
}

/// A trait marking objects used for updating (but not upserting) documents
/// in a collection.
pub trait Update<T: Doc>: ToDocument {}

/// A trait marking objects used for upserting documents in a collection.
pub trait Upsert<T: Doc>: ToDocument {}

/// A trait marking objects used for running an aggregation pipeline.
pub trait Pipeline<T: Doc>: ToDocuments {
    /// The type of the results obtained by running the pipeline.
    type Output: for<'a> Deserialize<'a>;
}
