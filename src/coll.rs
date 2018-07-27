//! A MongoDB collection of a single homogeneous type.

use std::marker::PhantomData;
use std::fmt;
use bson;
use mongodb;
use mongodb::common::WriteConcern;
use mongodb::coll::options::{ InsertManyOptions, UpdateOptions };
use cursor::Cursor;
use dsl::*;
use bsn::*;
use error::{ Error, Result, ResultExt };

/// The default, safest `WriteConcern`.
const WRITE_CONCERN: WriteConcern = WriteConcern {
    w: 1, // the default
    w_timeout: 0, // no timeout
    j: true, // wait for journal
    fsync: true, // if no journal, wait for filesystem sync
};

/// Converts an `i32` to a `usize` if the range and value permits.
/// Constructs an error message based on `msg` otherwise.
#[cfg_attr(feature = "cargo-clippy", allow(cast_possible_wrap, cast_possible_truncation, if_same_then_else))]
fn i32_to_usize_with_msg(n: i32, msg: &str) -> Result<usize> {
    use std::usize;
    use std::mem::size_of;

    // XXX: the correctness of this usize -> i32 cast relies on the following:
    // 1. if `sizeof(usize) >= sizeof(i32)`, i.e. 32-bit and wider word size
    //    platforms (the typical), then `i32::MAX` always fits into a `usize`,
    //    therefore the cast `n as usize` is safe as long as `n >= 0`.
    // 2. Otherwise, if `sizeof(usize) < sizeof(i32)`, eg. 16-bit architectures,
    //    then we can safely cast `usize::MAX` to `i32` in order to find out
    //    via comparison whether the actual `i32` value fits dynamically.
    if n < 0 {
        Err(Error::new(format!("{} ({}) is negative", msg, n)))
    } else if size_of::<usize>() >= size_of::<i32>() {
        Ok(n as usize)
    } else if n <= usize::MAX as i32 {
        Ok(n as usize)
    } else {
        Err(Error::new(format!("{} ({}) overflows `usize`", msg, n)))
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
        let filter = query.to_document();
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
        let filter = query.to_document();
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

    /// Updates (or upserts) a single document.
    pub fn update_one<Q, U>(&self, query: &Q, update: &U) -> Result<UpdateOneResult<T>>
        where Q: Query<T>,
              U: Update<T>,
    {
        let options = UpdateOptions {
            upsert: Some(U::UPSERT),
            write_concern: Some(WRITE_CONCERN),
        };
        let filter = query.to_document();
        let update = update.to_document();
        let action = if U::UPSERT { "upsert" } else { "update" };

        self.inner
            .update_one(filter, update, options.into())
            .chain(format!("can't {} documents in {}", action, T::NAME))
            .and_then(|result| {
                if let Some(error) = result.write_exception {
                    let msg = format!("can't {} document in {}", action, T::NAME);
                    let error = mongodb::error::Error::from(error);
                    Err(Error::with_cause(msg, error))
                } else {
                    let upserted_id = match result.upserted_id {
                        Some(id) => bson::from_bson(id).chain("can't deserialize upserted ID")?,
                        None => None,
                    };
                    let num_matched = i32_to_usize_with_msg(result.matched_count, "# of matched documents")?;
                    let num_modified = i32_to_usize_with_msg(result.modified_count, "# of modified documents")?;

                    Ok(UpdateOneResult { upserted_id, num_matched, num_modified })
                }
            })
    }
}

impl<T: Doc> fmt::Debug for Collection<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Collection<{}>", T::NAME)
    }
}

/// The outcome of a successful `update_one()` operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UpdateOneResult<T: Doc> {
    /// the ID that was upserted, if any.
    pub upserted_id: Option<T::Id>,
    /// The number of documents matched by the query criteria.
    pub num_matched: usize,
    /// The number of documents modified by the update specification.
    pub num_modified: usize,
}
