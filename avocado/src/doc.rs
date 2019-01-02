//! A document is a direct member of a collection.

use std::borrow::Cow;
use std::cell::{ Cell, RefCell };
use std::sync::{ Mutex, RwLock };
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

/// Implemented by top-level (direct collection member) documents only.
/// These types always have an associated top-level name and an `_id` field.
pub trait Doc: Serialize + for<'a> Deserialize<'a> {
    /// The type of the unique IDs for the document. A good default choice
    /// is `ObjectId`. TODO(H2CO3): make it default to `ObjectId` (#29661).
    type Id: Eq + Serialize + for <'a> Deserialize<'a>;

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

/// Wrappers and single-element containers of documents implement `Doc` too for
/// reasons of convenience. This macro helps forward methods to the wrapped type.
macro_rules! implement_doc {
    ($($ty:ident < $($lt:lifetime,)* T: Doc $(+ $posbound:ident)* $(+ ?$negbound:ident)* >,)*) => {$(
        impl<$($lt,)* T: Doc $(+ $posbound)* $(+ ?$negbound)*> Doc for $ty<$($lt,)* T> {
            type Id = <T as Doc>::Id;

            const NAME: &'static str = <T as Doc>::NAME;

            fn indexes() -> Vec<IndexModel> {
                <T as Doc>::indexes()
            }

            fn count_options() -> CountOptions {
                <T as Doc>::count_options()
            }

            fn distinct_options() -> DistinctOptions {
                <T as Doc>::distinct_options()
            }

            fn aggregate_options() -> AggregateOptions {
                <T as Doc>::aggregate_options()
            }

            fn query_options() -> FindOptions {
                <T as Doc>::query_options()
            }

            fn insert_options() -> InsertManyOptions {
                <T as Doc>::insert_options()
            }

            fn delete_options() -> WriteConcern {
                <T as Doc>::delete_options()
            }

            fn update_options() -> WriteConcern {
                <T as Doc>::update_options()
            }

            fn upsert_options() -> WriteConcern {
                <T as Doc>::upsert_options()
            }
        }
    )*}
}

implement_doc!{
    Box<T: Doc + ?Sized>,
    Cow<'a, T: Doc + Clone + ?Sized>,

    Cell<T: Doc + Copy>,
    RefCell<T: Doc + ?Sized>,

    Mutex<T: Doc + ?Sized>,
    RwLock<T: Doc + ?Sized>,
}
