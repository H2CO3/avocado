//! The Avocado prelude provides re-exports of the most commonly used traits
//! and types for convenience, including ones from crates `bson` and `mongodb`.

pub use crate::{
    db::DatabaseExt,
    coll::Collection,
    doc::Doc,
    uid::Uid,
    ops::*,
    literal::{ IndexType, Order, BsonType },
    error::Error as AvocadoError,
    error::ErrorKind as AvocadoErrorKind,
    error::Result as AvocadoResult,
};
pub use bson::{ Bson, Document, oid::ObjectId, doc, bson };
pub use mongodb::{
    Client, ThreadedClient,
    db::Database,
    coll::options::{
        IndexModel, IndexOptions, FindOptions,
        FindOneAndUpdateOptions, ReturnDocument,
    },
};
