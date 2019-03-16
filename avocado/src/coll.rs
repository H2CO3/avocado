//! A MongoDB collection of a single homogeneous type.

use std::borrow::Borrow;
use std::marker::PhantomData;
use std::iter::FromIterator;
use std::fmt;
use serde::Deserialize;
use bson::{ Document, from_bson };
use mongodb::coll::options::{
    UpdateOptions,
    FindOneAndDeleteOptions,
    FindOneAndUpdateOptions,
    ReturnDocument,
};
use mongodb::coll::results::UpdateResult;
use crate::{
    cursor::Cursor,
    doc::Doc,
    uid::Uid,
    ops::*,
    bsn::*,
    utils::*,
    error::{ Error, ErrorKind::MissingId, Result, ResultExt },
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
                .chain(|| format!("can't create indexes on {}", T::NAME))
        }
    }

    /// Deletes the collection.
    pub fn drop(&self) -> Result<()> {
        self.inner.drop().map_err(Into::into)
    }

    /// Returns the number of documents matching the query criteria.
    pub fn count<Q: Count<T>>(&self, query: Q) -> Result<usize> {
        self.inner
            .count(query.filter().into(), Q::options().into())
            .chain(|| format!("error in {}::count({:#?})", T::NAME, query))
            .and_then(|n| int_to_usize_with_msg(n, "# of counted documents"))
    }

    /// Returns the distinct values of a certain field.
    pub fn distinct<Q, C>(&self, query: Q) -> Result<C>
        where Q: Distinct<T>,
              C: FromIterator<Q::Output>,
    {
        self.inner
            .distinct(Q::FIELD, query.filter().into(), Q::options().into())
            .chain(|| format!("error in {}::distinct({:#?})", T::NAME, query))
            .and_then(|values| {
                values
                    .into_iter()
                    .map(|b| from_bson(Q::transform(b)?).chain(|| format!(
                        "can't deserialize {}::{}", T::NAME, Q::FIELD
                    )))
                    .collect()
            })
    }

    /// Runs an aggregation pipeline.
    pub fn aggregate<P: Pipeline<T>>(&self, pipeline: P) -> Result<Cursor<P::Output>> {
        self.inner
            .aggregate(pipeline.stages(), P::options().into())
            .chain(|| format!("error in {}::aggregate({:#?})", T::NAME, pipeline))
            .map(|crs| Cursor::from_cursor_and_transform(crs, P::transform))
    }

    /// Retrieves a single document satisfying the query, if one exists.
    pub fn find_one<Q: Query<T>>(&self, query: Q) -> Result<Option<Q::Output>> {
        // This uses `impl Deserialize for Option<T> where T: Deserialize`
        // and the fact that in MongoDB, top-level documents are always
        // `Document`s and never `Null`.
        self.inner
            .find_one(query.filter().into(), Q::options().into())
            .chain(|| format!("error in {}::find_one({:#?})", T::NAME, query))
            .and_then(|opt| opt.map_or(Ok(None), |doc| {
                let transformed = Q::transform(doc)?;
                from_bson(transformed).map_err(From::from)
            }))
    }

    /// Retrieves all documents satisfying the query.
    pub fn find_many<Q: Query<T>>(&self, query: Q) -> Result<Cursor<Q::Output>> {
        self.inner
            .find(query.filter().into(), Q::options().into())
            .chain(|| format!("error in {}::find_many({:#?})", T::NAME, query))
            .map(|crs| Cursor::from_cursor_and_transform(crs, Q::transform))
    }

    /// Inserts a single document.
    pub fn insert_one(&self, entity: &T) -> Result<Uid<T>> {
        let doc = serialize_document(entity)?;
        let write_concern = T::insert_options().write_concern;
        let message = || format!("error in {}::insert_one()", T::NAME);

        self.inner
            .insert_one(doc, write_concern)
            .chain(&message)
            .and_then(|result| {
                if let Some(error) = result.write_exception {
                    Err(Error::with_cause(message(), error))
                } else if let Some(id) = result.inserted_id {
                    from_bson(id).chain(
                        || format!("can't deserialize ID for {}", T::NAME)
                    )
                } else {
                    Err(Error::new(MissingId, message() + ": missing `inserted_id`"))
                }
            })
    }

    /// Inserts many documents.
    pub fn insert_many<I>(&self, entities: I) -> Result<Vec<Uid<T>>>
        where I: IntoIterator,
              I::Item: Borrow<T>,
              I::IntoIter: ExactSizeIterator,
    {
        let values = entities.into_iter();
        let n_docs = values.len();
        let docs = serialize_documents(values)?;
        let options = T::insert_options();
        let message = || format!("error in {}::insert_many()", T::NAME);

        // MongoDB complains if you try to insert 0 documents, but that's silly.
        if n_docs == 0 {
            return Ok(Vec::new());
        }

        self.inner
            .insert_many(docs, options.into())
            .chain(&message)
            .and_then(|result| {
                if let Some(error) = result.bulk_write_exception {
                    Err(Error::with_cause(message(), error))
                } else if let Some(ids) = result.inserted_ids {
                    let ids = ids
                        .into_iter()
                        .map(|(_, v)| from_bson(v).chain(
                            || format!("can't deserialize IDs for {}", T::NAME)
                        ))
                        .collect::<Result<Vec<_>>>()?;

                    if ids.len() == n_docs {
                        Ok(ids)
                    } else {
                        let msg = format!("{}: {} documents given, but {} IDs returned",
                                          message(), n_docs, ids.len());
                        Err(Error::new(MissingId, msg))
                    }
                } else {
                    Err(Error::new(MissingId, message() + ": missing `inserted_ids`"))
                }
            })
    }

    /// Convenience method for updating a single document based on identity (its
    /// `_id` field), setting all fields to the values supplied by `entity`.
    ///
    /// This doesn't add a new document if none with the specified `_id` exists.
    pub fn replace_entity(&self, entity: &T) -> Result<UpdateOneResult> where T: fmt::Debug {
        self.update_entity_internal(entity, false)
            .and_then(UpdateOneResult::from_raw)
    }

    /// Convenience method for updating a single document based on identity (its
    /// `_id` field), setting all fields to the values supplied by `entity`.
    ///
    /// This method adds a new document if none with the specified `_id` exists.
    pub fn upsert_entity(&self, entity: &T) -> Result<UpsertOneResult<Uid<T>>> where T: fmt::Debug {
        self.update_entity_internal(entity, true)
            .and_then(UpsertOneResult::from_raw)
    }

    /// Helper for the `{...}_entity` convenience methods above.
    fn update_entity_internal(&self, entity: &T, upsert: bool) -> Result<UpdateResult>
        where T: fmt::Debug
    {
        let mut document = serialize_document(entity)?;
        let id = document.remove("_id").ok_or_else(
            || Error::new(MissingId, format!("No `_id` in entity of type {}", T::NAME))
        )?;
        let filter = doc!{ "_id": id };
        let options = UpdateOptions {
            upsert: upsert.into(),
            write_concern: T::update_options().into(),
        };
        let message = || format!("error in {}::{}_entity({:#?})",
                                 T::NAME,
                                 if upsert { "upsert" } else { "replace" },
                                 entity);

        self.inner
            .replace_one(filter, document, options.into())
            .chain(&message)
            .and_then(|result| {
                if let Some(error) = result.write_exception {
                    Err(Error::with_cause(message(), error))
                } else {
                    Ok(result)
                }
            })
    }

    /// Updates a single document.
    ///
    /// This method only works with update operators (with field names starting
    /// with `$`), i.e. it does **not** replace entire documents.
    pub fn update_one<U: Update<T>>(&self, update: U) -> Result<UpdateOneResult> {
        let filter = update.filter();
        let change = update.update();
        let options = UpdateOptions {
            upsert: Some(false),
            write_concern: U::options().into(),
        };
        let message = || format!("error in {}::update_one({:#?})", T::NAME, update);

        self.update_one_internal(filter, change, options, &message)
            .and_then(UpdateOneResult::from_raw)
    }

    /// Upserts a single document.
    ///
    /// This method only works with update operators (with field names starting
    /// with `$`), i.e. it does **not** replace entire documents.
    pub fn upsert_one<U: Upsert<T>>(&self, upsert: U) -> Result<UpsertOneResult<Uid<T>>> {
        let filter = upsert.filter();
        let change = upsert.upsert();
        let options = UpdateOptions {
            upsert: Some(true),
            write_concern: U::options().into(),
        };
        let message = || format!("error in {}::upsert_one({:#?})", T::NAME, upsert);

        self.update_one_internal(filter, change, options, &message)
            .and_then(UpsertOneResult::from_raw)
    }

    /// Updates or upserts a single document.
    ///
    /// This method only works with update operators (with field names starting
    /// with `$`), i.e. it does **not** replace entire documents.
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
    ///
    /// This method only works with update operators (with field names starting
    /// with `$`), i.e. it does **not** replace entire documents.
    pub fn update_many<U: Update<T>>(&self, update: U) -> Result<UpdateManyResult> {
        let filter = update.filter();
        let change = update.update();
        let options = UpdateOptions {
            upsert: Some(false),
            write_concern: U::options().into(),
        };
        let message = || format!("error in {}::update_many({:#?})", T::NAME, update);
        self.update_many_internal(filter, change, options, &message)
    }

    /// Upserts multiple documents (updates many or inserts one if none found).
    ///
    /// This method only works with update operators (with field names starting
    /// with `$`), i.e. it does **not** replace entire documents.
    pub fn upsert_many<U: Upsert<T>>(&self, upsert: U) -> Result<UpsertManyResult> {
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
    ///
    /// This method only works with update operators (with field names starting
    /// with `$`), i.e. it does **not** replace entire documents.
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

    /// Convenience method for deleting a single entity based on its identity
    /// (the `_id` field). Returns `true` if it was found and deleted.
    pub fn delete_entity(&self, entity: &T) -> Result<bool> where T: fmt::Debug {
        let id = entity.id().ok_or_else(
            || Error::new(MissingId, format!("No `_id` in entity of type {}", T::NAME))
        )?;
        let id_bson = bson::to_bson(id)?;

        self.delete_one(doc!{ "_id": id_bson }).chain(
            || format!("error in {}::delete_entity({:#?})", T::NAME, entity)
        )
    }

    /// Convenience method for deleting entities based on their identity
    /// (the `_id` fields). Returns the number of deleted documents.
    pub fn delete_entities<I>(&self, entities: I) -> Result<usize>
        where I: IntoIterator,
              I::Item: Borrow<T>,
              T: fmt::Debug,
    {
        let ids: Vec<_> = entities
            .into_iter()
            .map(|item| {
                let entity = item.borrow();
                let id = entity.id().ok_or_else(|| Error::new(
                    MissingId,
                    format!("No `_id` in entity to delete: {:#?}", entity)
                ))?;
                bson::to_bson(id).map_err(From::from)
            })
            .collect::<Result<_>>()?;

        let criterion = doc!{
            "_id": {
                "$in": ids
            }
        };

        self.delete_many(criterion).chain(
            || format!("error in {}::delete_entities(...)", T::NAME)
        )
    }

    /// Deletes one document. Returns `true` if one was found and deleted.
    pub fn delete_one<Q: Delete<T>>(&self, query: Q) -> Result<bool> {
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
    pub fn delete_many<Q: Delete<T>>(&self, query: Q) -> Result<usize> {
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

    /// Deletes a single document based on the query criteria,
    /// returning it if it was found.
    pub fn find_one_and_delete<Q: Query<T>>(&self, query: Q) -> Result<Option<Q::Output>> {
        let query_options = Q::options();
        let find_delete_options = FindOneAndDeleteOptions {
            max_time_ms: query_options.max_time_ms,
            projection: query_options.projection,
            sort: query_options.sort,
            write_concern: None, // TODO(H2CO3): do something intelligent here
        };

        self.inner
            .find_one_and_delete(query.filter(), find_delete_options.into())
            .chain(|| format!(
                "error in {}::find_one_and_delete({:#?})", T::NAME, query
            ))
            .and_then(|opt| match opt {
                Some(document) => {
                    let transformed = Q::transform(document)?;
                    from_bson(transformed).map_err(From::from)
                }
                None => Ok(None)
            })
    }

    /// Replaces a single document based on the query criteria.
    /// Returns the original document if found.
    ///
    /// This method does **not** provide an option for returning the updated
    /// document, since it already **requires** the presence of a replacement.
    pub fn find_one_and_replace<Q: Query<T>>(&self, query: Q, replacement: &T) -> Result<Option<Q::Output>>
        where T: fmt::Debug
    {
        let query_options = Q::options();
        let find_replace_options = FindOneAndUpdateOptions {
            return_document: Some(ReturnDocument::Before),
            max_time_ms: query_options.max_time_ms,
            projection: query_options.projection,
            sort: query_options.sort,
            upsert: Some(false),
            ..Default::default()
        };
        let filter = query.filter();
        let doc = serialize_document(replacement)?;

        self.inner
            .find_one_and_replace(filter, doc, find_replace_options.into())
            .chain(|| format!(
                "error in {}::find_one_and_replace({:#?}, {:#?})",
                T::NAME, query, replacement
            ))
            .and_then(|opt| match opt {
                Some(document) => {
                    let transformed = Q::transform(document)?;
                    from_bson(transformed).map_err(From::from)
                }
                None => Ok(None)
            })
    }

    /// Finds a single document based on query criteria and updates it.
    ///
    /// For convenience reasons, unlike others, **this API is NOT split into
    /// separate update and upsert functions.** The options returned by the
    /// `update` argument decide whether an update or an upsert happens.
    pub fn find_one_and_update<U: FindAndUpdate<T>>(&self, update: U) -> Result<Option<U::Output>> {
        let filter = update.filter();
        let change = update.update();
        let options = U::options();

        self.inner
            .find_one_and_update(filter, change, options.into())
            .chain(|| format!(
                "error in {}::find_one_and_update({:#?})", T::NAME, update
            ))
            .and_then(|opt| match opt {
                Some(document) => {
                    let transformed = U::transform(document)?;
                    from_bson(transformed).map_err(From::from)
                }
                None => Ok(None)
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

impl UpdateOneResult {
    /// Converts a MongoDB `UpdateResult` to an Avocado `UpdateOneResult`.
    fn from_raw(result: UpdateResult) -> Result<Self> {
        if let Some(error) = result.write_exception {
            Err(Error::with_cause("couldn't perform single update", error))
        } else {
            Ok(UpdateOneResult {
                matched: result.matched_count > 0,
                modified: result.modified_count > 0,
            })
        }
    }
}

/// The outcome of a successful `upsert_one()` operation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UpsertOneResult<Id> {
    /// Whether a document matched the query criteria.
    pub matched: bool,
    /// Whether the matched document was actually modified.
    pub modified: bool,
    /// If the document was inserted, this contains its ID.
    pub upserted_id: Option<Id>,
}

impl<Id: for<'a> Deserialize<'a>> UpsertOneResult<Id> {
    /// Converts a MongoDB `UpdateResult` to an Avocado `UpsertOneResult`.
    fn from_raw(result: UpdateResult) -> Result<Self> {
        let matched = result.matched_count > 0;
        let modified = result.modified_count > 0;
        let upserted_id = match result.upserted_id {
            Some(bson) => {
                let mut doc = bson.try_into_doc()?;
                let id_bson = doc.remove("_id").ok_or_else(
                    || Error::new(MissingId, "no `_id` found in `WriteResult.upserted`")
                )?;
                let id = from_bson(id_bson).chain("can't deserialize upserted ID")?;
                Some(id)
            }
            None => None
        };

        if let Some(error) = result.write_exception {
            Err(Error::with_cause("couldn't perform single upsert", error))
        } else {
            Ok(UpsertOneResult { matched, modified, upserted_id })
        }
    }
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
