//! A MongoDB collection of a single homogeneous type.

use std::marker::PhantomData;
use std::fmt;
use serde::{ Serialize, Deserialize };
use bson::{ Bson, Document };
use mongodb;
use mongodb::coll::options::IndexModel;
use magnet_schema::BsonSchema;
use error::{ Result, ResultExt };

/// Implemented by top-level (direct collection member) documents only.
/// These types always have an associated top-level name and an `_id` field.
pub trait Doc: BsonSchema + Serialize + for<'de> Deserialize<'de> {
    /// The name of the collection within the database.
    const NAME: &'static str;

    /// Returns the specifications of the indexes created on the collection.
    /// If not provided, returns an empty vector, leading to the collection not
    /// bearing any user-defined indexes. (The `_id` field will still be
    /// indexed, though, as defined by MongoDB.)
    fn indexes() -> Vec<IndexModel> {
        Vec::new()
    }
}

/// Ordering of keys within an index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IndexOrder {
    /// Order smaller values first.
    Ascending,
    /// Order greater values first.
    Descending,
}

/// The default index order is `Ascending`.
impl Default for IndexOrder {
    fn default() -> Self {
        IndexOrder::Ascending
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
/// # use avocado::coll::IndexOrder;
/// #
/// # fn main() {
/// let index = doc! {
///     "_id": IndexOrder::Ascending,
///     "zip": IndexOrder::Descending,
/// };
/// # }
/// ```
impl From<IndexOrder> for Bson {
    fn from(order: IndexOrder) -> Self {
        match order {
            IndexOrder::Ascending  => Bson::I32( 1),
            IndexOrder::Descending => Bson::I32(-1),
        }
    }
}

/// A statically-typed (homogeneous) `MongoDB` collection.
pub struct Collection<T: Doc> {
    /// The backing `MongoDB` collection.
    inner: mongodb::coll::Collection,
    /// Just here so that the type parameter is used.
    _marker: PhantomData<T>,
}

impl<T: Doc> Collection<T> {
    /// Creates indexes on the underlying `MongoDB` collection
    /// according to the given index specifications.
    pub fn create_indexes(&self) -> Result<()> {
        let indexes = T::indexes();
        if indexes.is_empty() {
            Ok(())
        } else {
            self.inner.create_indexes(indexes).map(drop)
                .link(format!("can't create indexes on `{}`", T::NAME))
        }
    }
}

impl<T: Doc> fmt::Debug for Collection<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Collection<{}>", T::NAME)
    }
}
