//! Typed, generic wrapper around MongoDB `Cursor`s.

use std::fmt;
use std::iter::FromIterator;
use std::marker::PhantomData;
use serde::Deserialize;
use bson::{ Bson, Document, from_bson };
use crate::error::{ Result, ResultExt };

/// A typed wrapper around the MongoDB `Cursor` type.
pub struct Cursor<T> {
    /// The underlying MongoDB cursor.
    inner: mongodb::cursor::Cursor,
    /// The function applied to each returned `Document` before deserialization.
    transform: fn(Document) -> Result<Bson>,
    /// Just here so that the type parameter is used.
    _marker: PhantomData<T>,
}

impl<T> Cursor<T> where T: for<'a> Deserialize<'a> {
    /// Creates a strongly-typed cursor from an untyped MongoDB cursor
    /// and a transformation function.
    #[doc(hidden)]
    pub fn from_cursor_and_transform(
        inner: mongodb::cursor::Cursor,
        transform: fn(Document) -> Result<Bson>,
    ) -> Self {
        Cursor {
            inner,
            transform,
            _marker: PhantomData,
        }
    }

    /// Reads the remaining documents available in the current batch.
    pub fn next_batch<C: FromIterator<T>>(&mut self) -> Result<C> {
        self.inner
            .drain_current_batch()
            .chain("couldn't retrieve next batch")
            .and_then(|docs| self.transform_and_deserialize_many(docs))
    }

    /// Retrieves the next at most `n` documents.
    pub fn next_n<C: FromIterator<T>>(&mut self, n: usize) -> Result<C> {
        self.inner
            .next_n(n)
            .chain("couldn't retrieve documents")
            .and_then(|docs| self.transform_and_deserialize_many(docs))
    }

    /// Checks whether there are any more documents for the cursor to yield.
    pub fn has_next(&mut self) -> Result<bool> {
        self.inner.has_next().chain("cursor error")
    }

    /// Transforms and tries to deserialize a single document.
    fn transform_and_deserialize_one(&mut self, doc: Document) -> Result<T> {
        (self.transform)(doc).and_then(|b| from_bson(b).map_err(From::from))
    }

    /// Transforms and tries to deserialize a vector of documents.
    fn transform_and_deserialize_many<C>(&mut self, docs: Vec<Document>) -> Result<C>
        where C: FromIterator<T>
    {
        docs.into_iter()
            .map(|doc| self.transform_and_deserialize_one(doc))
            .collect()
    }
}

impl<T> Iterator for Cursor<T> where T: for<'a> Deserialize<'a> {
    type Item = Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|result| {
                result
                    .chain("can't step Cursor")
                    .and_then(|doc| self.transform_and_deserialize_one(doc))
            })
    }
}

impl<T> fmt::Debug for Cursor<T> where T: for<'a> Deserialize<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Cursor").finish()
    }
}
