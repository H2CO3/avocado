//! High-level database operations: query, update, delete, etc.

use std::fmt::Debug;
use serde::Deserialize;
use bson::{ Bson, Document };
use mongodb::common::WriteConcern;
use mongodb::coll::options::{
    FindOptions,
    CountOptions,
    DistinctOptions,
    AggregateOptions,
};
use crate::{
    doc::Doc,
    error::Result,
};

/// A counting-only query.
pub trait Count<T: Doc>: Debug {
    /// Filter for this query. Defaults to an empty filter,
    /// yielding the number of *all* documents in the collection.
    fn filter(&self) -> Document {
        Default::default()
    }

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
        Default::default()
    }

    /// Optional transform applied to each returned raw BSON. Can be used to
    /// adjust the structure of the loosely-typed data so that it fits
    /// what is expected by `<Self::Output as Deserialize>::deserialize()`.
    ///
    /// The default implementation just returns its argument verbatim.
    fn transform(raw: Bson) -> Result<Bson> {
        Ok(raw)
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

    /// Optional transform applied to each returned raw document. Can be used
    /// to adjust the structure of the loosely-typed data so that it fits
    /// what is expected by `<Self::Output as Deserialize>::deserialize()`.
    ///
    /// The default implementation just returns its argument verbatim.
    fn transform(raw: Document) -> Result<Bson> {
        Ok(raw.into())
    }

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

    /// Filter for restricting returned values. Defaults to an empty filter,
    /// resulting in *all* documents of the collection being returned.
    fn filter(&self) -> Document {
        Default::default()
    }

    /// Optional transform applied to each returned raw document. Can be used
    /// to adjust the structure of the loosely-typed data so that it fits
    /// what is expected by `<Self::Output as Deserialize>::deserialize()`.
    ///
    /// The default implementation just returns its argument verbatim.
    fn transform(raw: Document) -> Result<Bson> {
        Ok(raw.into())
    }

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

/////////////////////////////////////////////
// Blanket and convenience implementations //
/////////////////////////////////////////////

impl<T: Doc> Count<T> for Document {
    fn filter(&self) -> Document {
        self.clone()
    }
}

impl<T: Doc> Query<T> for Document {
    type Output = T;

    fn filter(&self) -> Document {
        self.clone()
    }
}

impl<T: Doc> Delete<T> for Document {
    fn filter(&self) -> Document {
        self.clone()
    }
}

impl<T: Doc, Q: Count<T>> Count<T> for &Q {
    fn filter(&self) -> Document {
        (**self).filter()
    }

    fn options() -> CountOptions {
        Q::options()
    }
}

impl<T: Doc, Q: Distinct<T>> Distinct<T> for &Q {
    type Output = Q::Output;

    const FIELD: &'static str = Q::FIELD;

    fn filter(&self) -> Document {
        (**self).filter()
    }

    fn transform(bson: Bson) -> Result<Bson> {
        Q::transform(bson)
    }

    fn options() -> DistinctOptions {
        Q::options()
    }
}

impl<T: Doc, P: Pipeline<T>> Pipeline<T> for &P {
    type Output = P::Output;

    fn stages(&self) -> Vec<Document> {
        (**self).stages()
    }

    fn transform(doc: Document) -> Result<Bson> {
        P::transform(doc)
    }

    fn options() -> AggregateOptions {
        P::options()
    }
}

impl<T: Doc, Q: Query<T>> Query<T> for &Q {
    type Output = Q::Output;

    fn filter(&self) -> Document {
        (**self).filter()
    }

    fn transform(doc: Document) -> Result<Bson> {
        Q::transform(doc)
    }

    fn options() -> FindOptions {
        Q::options()
    }
}

impl<T: Doc, U: Update<T>> Update<T> for &U {
    fn filter(&self) -> Document {
        (**self).filter()
    }

    fn update(&self) -> Document {
        (**self).update()
    }

    fn options() -> WriteConcern {
        U::options()
    }
}

impl<T: Doc, U: Upsert<T>> Upsert<T> for &U {
    fn filter(&self) -> Document {
        (**self).filter()
    }

    fn upsert(&self) -> Document {
        (**self).upsert()
    }

    fn options() -> WriteConcern {
        U::options()
    }
}

impl<T: Doc, Q: Delete<T>> Delete<T> for &Q {
    fn filter(&self) -> Document {
        (**self).filter()
    }

    fn options() -> WriteConcern {
        Q::options()
    }
}
