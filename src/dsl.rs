//! Traits and types for describing the MongoDB DDL and DML.

use std::fmt::Debug;
use bson::{ Bson, Document };
use serde::{ Serialize, Deserialize };
use mongodb::common::WriteConcern;
use mongodb::coll::options::IndexModel;
use mongodb::coll::options::{ FindOptions, CountOptions, DistinctOptions, AggregateOptions, InsertManyOptions };
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

/// A counting-only query.
pub trait Count<T: Doc>: Debug {
    /// Filter for this query.
    fn filter(&self) -> Document;

    /// Options for this query.
    fn options() -> CountOptions {
        T::count_options()
    }
}

/// A query for returning the distinct values of a field.
pub trait Distinct<T: Doc>: Debug {
    /// The type of the field of which the distinct values will be returned.
    type Output: for<'a> Deserialize<'a>;

    /// The name of the field of which the distinct values will be returned.
    const FIELD: &'static str;

    /// Optional filter restricting which values are taken into account.
    /// Defaults to no filtering.
    fn filter(&self) -> Document {
        Document::new()
    }

    /// Options for this query.
    fn options() -> DistinctOptions {
        T::distinct_options()
    }
}

/// An aggregation pipeline.
pub trait Pipeline<T: Doc>: Debug {
    /// The type of the values obtained by running this pipeline.
    type Output: for<'a> Deserialize<'a>;

    /// The stages of the aggregation pipeline.
    fn stages(&self) -> Vec<Document>;

    /// Options for this pipeline.
    fn options() -> AggregateOptions {
        T::aggregate_options()
    }
}

/// A regular query (`find_one()` or `find_many()`) operation.
pub trait Query<T: Doc>: Debug {
    /// The type of the results obtained by executing the query. Often it's just
    /// the document type, `T`. TODO(H2CO3): make it default to `T` (#29661).
    type Output: for<'a> Deserialize<'a>;

    /// Filter for restricting returned values.
    fn filter(&self) -> Document;

    /// Options for this query.
    fn options() -> FindOptions {
        T::query_options()
    }
}

/// An update (but not an upsert) operation.
pub trait Update<T: Doc>: Debug {
    /// Filter for restricting documents to update.
    fn filter(&self) -> Document;

    /// The update to perform on matching documents.
    fn update(&self) -> Document;

    /// Options for this update operation.
    fn options() -> WriteConcern {
        T::update_options()
    }
}

/// An upsert (update or insert) operation.
pub trait Upsert<T: Doc>: Debug {
    /// Filter for restricting documents to upsert.
    fn filter(&self) -> Document;

    /// The upsert to perform on matching documents.
    fn upsert(&self) -> Document;

    /// Options for this upsert operation.
    fn options() -> WriteConcern {
        T::upsert_options()
    }
}

/// A deletion / removal operation.
pub trait Delete<T: Doc>: Debug {
    /// Filter for restricting documents to delete.
    fn filter(&self) -> Document;

    /// Writing options for this deletion operation.
    fn options() -> WriteConcern {
        T::delete_options()
    }
}
