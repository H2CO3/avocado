//! A MongoDB collection of a single homogeneous type.

use std::marker::PhantomData;
use std::fmt;
use serde::{ Serialize, Deserialize };
use bson;
use bson::Bson;
use mongodb;
use mongodb::common::WriteConcern;
use mongodb::coll::options::{ FindOptions, InsertManyOptions, IndexModel };
use magnet_schema::BsonSchema;
use cursor::Cursor;
use bsn::*;
use error::{ Error, Result, ResultExt };

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
}

/// A trait marking objects used for querying a collection.
pub trait Query<T: Doc>: Serialize {
    /// The type of the results obtained by executing the query. Often it's just
    /// the document type, `T`. TODO(H2CO3): make it default to `T` (#29661).
    type Output: for<'a> Deserialize<'a>;

    /// If required, additional options can be provided here.
    /// Returns the `<FindOptions as Default>::default()` by default.
    fn options() -> FindOptions {
        Default::default()
    }
}

/// Ordering of keys within an index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IndexOrder {
    /// Order smaller values first.
    Ascending  =  1,
    /// Order greater values first.
    Descending = -1,
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
        Bson::I32(order as _)
    }
}

/// The default, safest `WriteConcern`.
const WRITE_CONCERN: WriteConcern = WriteConcern {
    w: 1, // the default
    w_timeout: 0, // no timeout
    j: true, // wait for journal
    fsync: true, // if no journal, wait for filesystem sync
};

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
            self.inner
                .create_indexes(indexes)
                .map(drop)
                .chain(format!("can't create indexes on `{}`", T::NAME))
        }
    }

    /// Deletes the collection.
    pub fn drop(&self) -> Result<()> {
        self.inner.drop().map_err(Into::into)
    }

    /// Retrieves a single document satisfying the query, if one exists.
    pub fn find_one<Q: Query<T>>(&self, query: &Q) -> Result<Option<Q::Output>> {
        let filter = serialize_document(query)?;
        let options = Q::options();

        // This uses `impl Deserialize for Option<T> where T: Deserialize`
        // and the fact that in MongoDB, top-level documents are always
        // `Document`s and never `Null`.
        self.inner
            .find_one(filter.into(), options.into())
            .chain("`find_one()` failed")
            .and_then(|opt| opt.map_or(Ok(None), deserialize_document))
    }

    /// Retrieves all documents satisfying the query.
    pub fn find_many<Q: Query<T>>(&self, query: &Q) -> Result<Cursor<Q::Output>> {
        let filter = serialize_document(query)?;
        let options = Q::options();

        self.inner
            .find(filter.into(), options.into())
            .chain("`find_many()` failed")
            .map(Into::into)
    }

    /// Inserts a single document.
    pub fn insert_one(&self, value: &T) -> Result<T::Id> {
        let doc = serialize_document(value)?;

        self.inner
            .insert_one(doc, WRITE_CONCERN.into())
            .chain(format!("can't insert document into {}", T::NAME))
            .and_then(|result| {
                if let Some(error) = result.write_exception {
                    let msg = format!("can't insert document into {}", T::NAME);
                    let error = mongodb::error::Error::from(error);
                    Err(Error::with_cause(msg, error))
                } else if let Some(id) = result.inserted_id {
                    bson::from_bson(id).chain("can't deserialize document ID")
                } else {
                    let msg = format!("can't insert document into {}: missing inserted ID", T::NAME);
                    Err(Error::new(msg))
                }
            })
    }

    /// Inserts many documents.
    pub fn insert_many(&self, values: &[T]) -> Result<Vec<T::Id>> {
        let docs = serialize_documents(values)?;
        let options = InsertManyOptions {
            ordered: Some(true),
            write_concern: Some(WRITE_CONCERN),
        };

        self.inner
            .insert_many(docs, options.into())
            .chain(format!("can't insert documents into {}", T::NAME))
            .and_then(|result| {
                if let Some(error) = result.bulk_write_exception {
                    let msg = format!("can't insert documents into {}", T::NAME);
                    let error = mongodb::error::Error::from(error);
                    Err(Error::with_cause(msg, error))
                } else if let Some(ids) = result.inserted_ids {
                    let ids = ids
                        .into_iter()
                        .map(|(_, v)| bson::from_bson(v).chain("can't deserialize document IDs"))
                        .collect::<Result<Vec<_>>>()?;
                    let n_docs = values.len();
                    let n_ids = ids.len();

                    if n_ids == n_docs {
                        Ok(ids)
                    } else {
                        let msg = format!("{} documents given, but {} IDs returned", n_docs, n_ids);
                        Err(Error::new(msg))
                    }
                } else {
                    let msg = format!("can't insert documents into {}: missing inserted IDs", T::NAME);
                    Err(Error::new(msg))
                }
            })
    }
}

impl<T: Doc> fmt::Debug for Collection<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Collection<{}>", T::NAME)
    }
}
