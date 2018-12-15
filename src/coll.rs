//! A MongoDB collection of a single homogeneous type.

use std::marker::PhantomData;
use std::fmt;
use bson;
use bson::Document;
use mongodb;
use mongodb::coll::options::UpdateOptions;
use mongodb::coll::results::UpdateResult;
use cursor::Cursor;
use doc::Doc;
use ops::*;
use bsn::*;
use utils::*;
use error::{ Error, Result, ResultExt };

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
                .chain(|| format!("can't create indexes on {}", T::NAME))
        }
    }

    /// Deletes the collection.
    pub fn drop(&self) -> Result<()> {
        self.inner.drop().map_err(Into::into)
    }

    /// Returns the number of documents matching the query criteria.
    pub fn count<Q: Count<T>>(&self, query: &Q) -> Result<usize> {
        self.inner
            .count(query.filter().into(), Q::options().into())
            .chain(|| format!("error in {}::count({:#?})", T::NAME, query))
            .and_then(|n| int_to_usize_with_msg(n, "# of counted documents"))
    }

    /// Returns the distinct values of a certain field.
    pub fn distinct<Q: Distinct<T>>(&self, query: &Q) -> Result<Vec<Q::Output>> {
        self.inner
            .distinct(Q::FIELD, query.filter().into(), Q::options().into())
            .chain(|| format!("error in {}::distinct({:#?})", T::NAME, query))
            .and_then(|values| {
                values
                    .into_iter()
                    .map(|b| {
                        bson::from_bson(b)
                            .chain(|| format!("can't deserialize {}::{}", T::NAME, Q::FIELD))
                    })
                    .collect()
            })
    }

    /// Runs an aggregation pipeline.
    pub fn aggregate<P: Pipeline<T>>(&self, pipeline: &P) -> Result<Cursor<P::Output>> {
        self.inner
            .aggregate(pipeline.stages(), P::options().into())
            .chain(|| format!("error in {}::aggregate({:#?})", T::NAME, pipeline))
            .map(Into::into)
    }

    /// Retrieves a single document satisfying the query, if one exists.
    pub fn find_one<Q: Query<T>>(&self, query: &Q) -> Result<Option<Q::Output>> {
        // This uses `impl Deserialize for Option<T> where T: Deserialize`
        // and the fact that in MongoDB, top-level documents are always
        // `Document`s and never `Null`.
        self.inner
            .find_one(query.filter().into(), Q::options().into())
            .chain(|| format!("error in {}::find_one({:#?})", T::NAME, query))
            .and_then(|opt| opt.map_or(Ok(None), deserialize_document))
    }

    /// Retrieves all documents satisfying the query.
    pub fn find_many<Q: Query<T>>(&self, query: &Q) -> Result<Cursor<Q::Output>> {
        self.inner
            .find(query.filter().into(), Q::options().into())
            .chain(|| format!("error in {}::find_many({:#?})", T::NAME, query))
            .map(Into::into)
    }

    /// Inserts a single document.
    pub fn insert_one(&self, value: &T) -> Result<T::Id> {
        let doc = serialize_document(value)?;
        let write_concern = T::insert_options().write_concern;
        let message = || format!("error in {}::insert_one()", T::NAME);

        self.inner
            .insert_one(doc, write_concern)
            .chain(&message)
            .and_then(|result| {
                if let Some(error) = result.write_exception {
                    Err(Error::with_cause(message(), error))
                } else if let Some(id) = result.inserted_id {
                    bson::from_bson(id).chain(
                        || format!("can't deserialize ID for {}", T::NAME)
                    )
                } else {
                    Err(Error::new(message() + ": missing `inserted_id`"))
                }
            })
    }

    /// Inserts many documents.
    pub fn insert_many(&self, values: &[T]) -> Result<Vec<T::Id>> {
        let docs = serialize_documents(values)?;
        let options = T::insert_options();
        let message = || format!("error in {}::insert_many()", T::NAME);

        self.inner
            .insert_many(docs, options.into())
            .chain(&message)
            .and_then(|result| {
                if let Some(error) = result.bulk_write_exception {
                    Err(Error::with_cause(message(), error))
                } else if let Some(ids) = result.inserted_ids {
                    let ids = ids
                        .into_iter()
                        .map(|(_, v)| {
                            bson::from_bson(v)
                                .chain(|| format!("can't deserialize IDs for {}", T::NAME))
                        })
                        .collect::<Result<Vec<_>>>()?;

                    let n_docs = values.len();
                    let n_ids = ids.len();

                    if n_ids == n_docs {
                        Ok(ids)
                    } else {
                        let msg = format!("{}: {} documents given, but {} IDs returned",
                                          message(), n_docs, n_ids);
                        Err(Error::new(msg))
                    }
                } else {
                    Err(Error::new(message() + ": missing `inserted_ids`"))
                }
            })
    }

    /// Updates a single document.
    pub fn update_one<U: Update<T>>(&self, update: &U) -> Result<UpdateOneResult> {
        let filter = update.filter();
        let change = update.update();
        let options = UpdateOptions {
            upsert: Some(false),
            write_concern: U::options().into(),
        };
        let message = || format!("error in {}::update_one({:#?})", T::NAME, update);

        self.update_one_internal(filter, change, options, &message)
            .map(|result| UpdateOneResult {
                matched: result.matched_count > 0,
                modified: result.modified_count > 0,
            })
    }

    /// Upserts a single document.
    pub fn upsert_one<U: Upsert<T>>(&self, upsert: &U) -> Result<UpsertOneResult<T>> {
        let filter = upsert.filter();
        let change = upsert.upsert();
        let options = UpdateOptions {
            upsert: Some(true),
            write_concern: U::options().into(),
        };
        let message = || format!("error in {}::upsert_one({:#?})", T::NAME, upsert);

        self.update_one_internal(filter, change, options, &message)
            .and_then(|result| {
                let matched = result.matched_count > 0;
                let modified = result.modified_count > 0;
                let upserted_id = match result.upserted_id {
                    Some(id) => Some(
                        bson::from_bson(id).chain(
                            || message() + ": can't deserialize upserted ID"
                        )?
                    ),
                    None => None,
                };
                Ok(UpsertOneResult { matched, modified, upserted_id })
            })
    }

    /// Updates or upserts a single document.
    fn update_one_internal<F: Copy + FnOnce() -> String>(
        &self,
        filter: Document,
        change: Document,
        options: UpdateOptions,
        message: F,
    ) -> Result<UpdateResult> {
        self.inner
            .update_one(filter, change, options.into())
            .chain(message)
            .and_then(|result| {
                if let Some(error) = result.write_exception {
                    Err(Error::with_cause(message(), error))
                } else {
                    Ok(result)
                }
            })
    }

    /// Updates multiple documents.
    pub fn update_many<U: Update<T>>(&self, update: &U) -> Result<UpdateManyResult> {
        let filter = update.filter();
        let change = update.update();
        let options = UpdateOptions {
            upsert: Some(false),
            write_concern: U::options().into(),
        };
        let message = || format!("error in {}::update_many({:#?})", T::NAME, update);
        self.update_many_internal(filter, change, options, &message)
    }

    /// Upserts multiple documents.
    pub fn upsert_many<U: Upsert<T>>(&self, upsert: &U) -> Result<UpsertManyResult> {
        let filter = upsert.filter();
        let change = upsert.upsert();
        let options = UpdateOptions {
            upsert: Some(true),
            write_concern: U::options().into(),
        };
        let message = || format!("error in {}::upsert_many({:#?})", T::NAME, upsert);
        self.update_many_internal(filter, change, options, &message)
    }

    /// Updates or upserts multiple documents.
    fn update_many_internal<F: Copy + FnOnce() -> String>(
        &self,
        filter: Document,
        change: Document,
        options: UpdateOptions,
        message: F,
    ) -> Result<UpdateManyResult> {
        self.inner
            .update_many(filter, change, options.into())
            .chain(message)
            .and_then(|result| {
                if let Some(error) = result.write_exception {
                    Err(Error::with_cause(message(), error))
                } else {
                    let num_matched = int_to_usize_with_msg(result.matched_count, "# of matched documents")?;
                    let num_modified = int_to_usize_with_msg(result.modified_count, "# of modified documents")?;
                    Ok(UpdateManyResult { num_matched, num_modified })
                }
            })
    }

    /// Deletes one document. Returns `true` if one was found and deleted.
    pub fn delete_one<Q: Delete<T>>(&self, query: &Q) -> Result<bool> {
        let message = || format!("error in {}::delete_one({:#?})", T::NAME, query);
        self.inner
            .delete_one(query.filter(), Q::options().into())
            .chain(&message)
            .and_then(|result| {
                if let Some(error) = result.write_exception {
                    Err(Error::with_cause(message(), error))
                } else {
                    Ok(result.deleted_count > 0)
                }
            })
    }

    /// Deletes many documents. Returns the number of deleted documents.
    pub fn delete_many<Q: Delete<T>>(&self, query: &Q) -> Result<usize> {
        let message = || format!("error in {}::delete_many({:#?})", T::NAME, query);
        self.inner
            .delete_many(query.filter(), Q::options().into())
            .chain(&message)
            .and_then(|result| {
                if let Some(error) = result.write_exception {
                    Err(Error::with_cause(message(), error))
                } else {
                    int_to_usize_with_msg(result.deleted_count, "# of deleted documents")
                }
            })
    }
}

impl<T: Doc> fmt::Debug for Collection<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Collection<{}>", T::NAME)
    }
}

#[doc(hidden)]
impl<T: Doc> From<mongodb::coll::Collection> for Collection<T> {
    fn from(collection: mongodb::coll::Collection) -> Self {
        Collection {
            inner: collection,
            _marker: PhantomData,
        }
    }
}

/// The outcome of a successful `update_one()` operation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UpdateOneResult {
    /// Whether a document matched the query criteria.
    pub matched: bool,
    /// Whether the matched document was actually modified.
    pub modified: bool,
}

/// The outcome of a successful `upsert_one()` operation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UpsertOneResult<T: Doc> {
    /// Whether a document matched the query criteria.
    pub matched: bool,
    /// Whether the matched document was actually modified.
    pub modified: bool,
    /// If the document was inserted, this contains its ID.
    pub upserted_id: Option<T::Id>,
}

/// The outcome of a successful `update_many()` or `upsert_many()` operation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UpdateManyResult {
    /// The number of documents matched by the query criteria.
    pub num_matched: usize,
    /// The number of documents modified by the update specification.
    pub num_modified: usize,
}

/// An alias for a nicer-looking API.
pub type UpsertManyResult = UpdateManyResult;
