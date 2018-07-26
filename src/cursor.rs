//! Typed, generic wrapper around MongoDB `Cursor`s.

use std::i32;
use std::fmt;
use std::iter::Iterator;
use std::marker::PhantomData;
use serde::Deserialize;
use mongodb;
use bsn::*;
use error::{ Error, Result, ResultExt };

/// A typed wrapper around the MongoDB `Cursor` type.
pub struct Cursor<T> where T: for<'a> Deserialize<'a> {
    /// The underlying MongoDB cursor.
    inner: mongodb::cursor::Cursor,
    /// Just here so that the type parameter is used.
    _marker: PhantomData<T>,
}

impl<T> Cursor<T> where T: for<'a> Deserialize<'a> {
    /// Reads the remaining documents available in the current batch.
    pub fn next_batch(&mut self) -> Result<Vec<T>> {
        self.inner
            .drain_current_batch()
            .chain("couldn't retrieve next batch")
            .and_then(deserialize_documents)
    }

    /// Retrieves the next at most `n` documents.
    #[cfg_attr(feature = "cargo-clippy", allow(cast_possible_wrap, cast_possible_truncation))]
    pub fn next_n(&mut self, n: usize) -> Result<Vec<T>> {
        if n > i32::MAX as usize {
            let msg = format!("can't return {} documents at once; max {} allowed", n, i32::MAX);
            return Err(Error::new(msg));
        }

        self.inner
            .next_n(n as i32)
            .chain("couldn't retrieve documents")
            .and_then(deserialize_documents)
    }

    /// Checks whether there are any more documents for the cursor to yield.
    pub fn has_next(&mut self) -> Result<bool> {
        self.inner.has_next().chain("cursor error")
    }
}

#[doc(hidden)]
impl<T> From<mongodb::cursor::Cursor> for Cursor<T> where T: for<'a> Deserialize<'a> {
    fn from(cursor: mongodb::cursor::Cursor) -> Self {
        Cursor {
            inner: cursor,
            _marker: PhantomData,
        }
    }
}

impl<T> Iterator for Cursor<T> where T: for<'a> Deserialize<'a> {
    type Item = Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|r| r.chain("can't step Cursor").and_then(deserialize_document))
    }
}

impl<T> fmt::Debug for Cursor<T> where T: for<'a> Deserialize<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Cursor").finish()
    }
}
