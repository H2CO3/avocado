//! A MongoDB collection of a single homogeneous type.

use std::marker::PhantomData;
use std::fmt;
use bson;
use mongodb;
use mongodb::coll::options::UpdateOptions;
use mongodb::coll::results::UpdateResult;
use cursor::Cursor;
use dsl::*;
use bsn::*;
use error::{ Error, Result, ResultExt };

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
        let options = T::options().read_options;

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
        let options = T::options().read_options;

        self.inner
            .find(filter.into(), options.into())
            .chain("`find_many()` failed")
            .map(Into::into)
    }

    /// Inserts a single document.
    pub fn insert_one(&self, value: &T) -> Result<T::Id> {
        let doc = serialize_document(value)?;
        let write_concern = T::options().write_options.write_concern;

        self.inner
            .insert_one(doc, write_concern)
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
        let options = T::options().write_options;

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

    /// Updates a single document.
    pub fn update_one<Q: Query<T>, U: Update<T>>(&self, query: &Q, update: &U) -> Result<UpdateOneResult> {
        self.update_one_internal(query, update, false)
            .map(|result| UpdateOneResult {
                matched: result.matched_count > 0,
                modified: result.modified_count > 0,
            })
    }

    /// Upserts a single document.
    pub fn upsert_one<Q: Query<T>, U: Upsert<T>>(&self, query: &Q, upsert: &U) -> Result<UpsertOneResult<T>> {
        self.update_one_internal(query, upsert, true)
            .and_then(|result| {
                let matched = result.matched_count > 0;
                let modified = result.modified_count > 0;
                let upserted_id = match result.upserted_id {
                    Some(id) => {
                        Some(bson::from_bson(id)
                             .chain("can't deserialize updated ID")?)
                    }
                    None => None
                };
                Ok(UpsertOneResult { matched, modified, upserted_id })
            })
    }

    /// Updates or upserts a single document.
    fn update_one_internal<Q: Query<T>, U: ToDocument>(
        &self,
        query: &Q,
        update: &U,
        upsert: bool,
    ) -> Result<UpdateResult> {
        let options = UpdateOptions {
            upsert: Some(upsert),
            write_concern: T::options().write_options.write_concern,
        };
        let filter = query.to_document();
        let update = update.to_document();
        let action = if upsert { "upsert" } else { "update" };
        let message = || format!("can't {} document in {}", action, T::NAME);

        self.inner
            .update_one(filter, update, options.into())
            .chain(message())
            .and_then(|result| {
                if let Some(error) = result.write_exception {
                    let error = mongodb::error::Error::from(error);
                    Err(Error::with_cause(message(), error))
                } else {
                    Ok(result)
                }
            })
    }

    /// Updates multiple documents.
    pub fn update_many<Q: Query<T>, U: Update<T>>(&self, query: &Q, update: &U) -> Result<UpdateManyResult> {
        self.update_many_internal(query, update, false)
    }

    /// Upserts multiple documents.
    pub fn upsert_many<Q: Query<T>, U: Upsert<T>>(&self, query: &Q, upsert: &U) -> Result<UpsertManyResult> {
        self.update_many_internal(query, upsert, true)
    }

    /// Updates or upserts multiple documents.
    fn update_many_internal<Q: Query<T>, U: ToDocument>(&self, query: &Q, update: &U, upsert: bool) -> Result<UpdateManyResult> {
        let options = UpdateOptions {
            upsert: Some(upsert),
            write_concern: T::options().write_options.write_concern,
        };
        let filter = query.to_document();
        let update = update.to_document();
        let action = if upsert { "upsert" } else { "update" };
        let message = || format!("can't {} documents in {}", action, T::NAME);

        self.inner
            .update_many(filter, update, options.into())
            .chain(message())
            .and_then(|result| {
                if let Some(error) = result.write_exception {
                    let error = mongodb::error::Error::from(error);
                    Err(Error::with_cause(message(), error))
                } else {
                    let num_matched = i32_to_usize_with_msg(result.matched_count, "# of matched documents")?;
                    let num_modified = i32_to_usize_with_msg(result.modified_count, "# of modified documents")?;
                    Ok(UpdateManyResult { num_matched, num_modified })
                }
            })
    }

    /// Deletes one document. Returns `true` if one was found and deleted.
    pub fn delete_one<Q: Query<T>>(&self, query: &Q) -> Result<bool> {
        let filter = query.to_document();
        let write_concern = T::options().write_options.write_concern;
        let message = || format!("can't delete document from {}", T::NAME);

        self.inner
            .delete_one(filter, write_concern)
            .chain(message())
            .and_then(|result| {
                if let Some(error) = result.write_exception {
                    let error = mongodb::error::Error::from(error);
                    Err(Error::with_cause(message(), error))
                } else {
                    Ok(result.deleted_count > 0)
                }
            })
    }

    /// Deletes many documents. Returns the number of deleted documents.
    pub fn delete_many<Q: Query<T>>(&self, query: &Q) -> Result<usize> {
        let filter = query.to_document();
        let write_concern = T::options().write_options.write_concern;
        let message = || format!("can't delete documents from {}", T::NAME);

        self.inner
            .delete_many(filter, write_concern)
            .chain(message())
            .and_then(|result| {
                if let Some(error) = result.write_exception {
                    let error = mongodb::error::Error::from(error);
                    Err(Error::with_cause(message(), error))
                } else {
                    i32_to_usize_with_msg(result.deleted_count, "# of deleted documents")
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
pub struct UpdateOneResult {
    /// Whether a document matched the query criteria.
    pub matched: bool,
    /// Whether the matched document was actually modified.
    pub modified: bool,
}

/// The outcome of a successful `upsert_one()` operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UpsertOneResult<T: Doc> {
    /// Whether a document matched the query criteria.
    pub matched: bool,
    /// Whether the matched document was actually modified.
    pub modified: bool,
    /// If the document was inserted, this contains its ID.
    pub upserted_id: Option<T::Id>,
}

/// The outcome of a successful `update_many()` or `upsert_many()` operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UpdateManyResult {
    /// The number of documents matched by the query criteria.
    pub num_matched: usize,
    /// The number of documents modified by the update specification.
    pub num_modified: usize,
}

/// An alias for a nicer-looking API.
pub type UpsertManyResult = UpdateManyResult;
