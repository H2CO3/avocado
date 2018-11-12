//! High-level database operations: query, update, delete, etc.

use std::fmt::Debug;
use serde::Deserialize;
use bson::Document;
use mongodb_h2co3::common::WriteConcern;
use mongodb_h2co3::coll::options::{
    FindOptions,
    CountOptions,
    DistinctOptions,
    AggregateOptions,
};
use super::Doc;
use super::filter::FilterDoc;
use super::update::UpdateSpec;

/// A counting-only query.
pub trait Count<T: Doc>: Debug {
    /// Filter for this query.
    fn filter(&self) -> FilterDoc;

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
    fn filter(&self) -> FilterDoc {
        Default::default()
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
    fn filter(&self) -> FilterDoc;

    /// Options for this query.
    fn options() -> FindOptions {
        T::query_options()
    }
}

/// An update (but not an upsert) operation.
pub trait Update<T: Doc>: Debug {
    /// Filter for restricting documents to update.
    fn filter(&self) -> FilterDoc;

    /// The update to perform on matching documents.
    fn update(&self) -> UpdateSpec;

    /// Options for this update operation.
    fn options() -> WriteConcern {
        T::update_options()
    }
}

/// An upsert (update or insert) operation.
pub trait Upsert<T: Doc>: Debug {
    /// Filter for restricting documents to upsert.
    fn filter(&self) -> FilterDoc;

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
    fn filter(&self) -> FilterDoc;

    /// Writing options for this deletion operation.
    fn options() -> WriteConcern {
        T::delete_options()
    }
}
