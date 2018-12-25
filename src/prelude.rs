//! The Avocado prelude provides re-exports of the most commonly used traits
//! and types for convenience, including ones from crates `bson` and `mongodb`.

pub use crate::{
    db::DatabaseExt,
    coll::Collection,
    doc::Doc,
    ops::*,
    literal::{ Order, BsonType },
    error::Error as AvocadoError,
    error::Result as AvocadoResult,
};
pub use bson::{ Bson, Document, oid::ObjectId, doc, bson };
pub use mongodb::{
    Client, ThreadedClient,
    db::Database,
    coll::options::{ IndexModel, IndexOptions }
};
