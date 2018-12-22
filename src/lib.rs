//! # Avocado: the strongly-typed MongoDB driver
//!
//! This library allows MongoDB users to work directly with statically-typed
//! domain model objects, instead of the dynamically and loosely-typed BSON
//! or JSON representation which is native to Mongo.
//!
//! ### The Prelude
//!
//! Let's get this one out of the way quickly. The most useful and most
//! frequently utilized types from Avocado as well as the `mongodb` and `bson`
//! crates are publicly re-exported under the module [`prelude`](prelude/index.html).
//! Therefore, for most purposes, it's enough to import the library in your
//! code like this:
//!
//! ```rust
//! extern crate avocado;
//!
//! use avocado::prelude::*;
//! ```
//!
//! ### Documents
//!
//! The first step is defining your domain model / entity types. Transcoding
//! them to and from BSON is handled by Serde and the BSON crate.
//!
//! Avocado can handle any top-level entity type with the following properties:
//! * It is `Serialize` and `Deserialize`
//! * It has a serializable and deserializable unique ID which appears under
//!   the key `_id` at the top level
//! * It has a name that is globally unique within the given MongoDB database
//!
//! These constraints are captured by the [`Doc`](doc/trait.Doc.html) trait.
//!
//! Here's an example of how you can `#[derive]` or manually implement `Doc`
//! for your entity types:
//!
//! ```ignore
//! // Automatically, for more convenience and sensible defaults, respecting
//! // Serde renaming conventions
//! #[derive(Debug, Serialize, Deserialize, Doc)]
//! struct Job {
//!     #[serde(rename = "_id")]
//!     pub id: ObjectId,
//!     pub description: String,
//!     pub salary: u32,
//! }
//! ```
//! ```
//! # #[macro_use]
//! # extern crate serde_derive;
//! # extern crate serde;
//! # extern crate avocado;
//! # use avocado::prelude::*;
//! // Manually, for complete flexibility and fine-grained control over indexes
//! // and database operation options
//! #[derive(Debug, Serialize, Deserialize)]
//! struct Product {
//!     #[serde(rename = "_id")]
//!     pub id: ObjectId,
//!     pub name: String,
//!     pub num_employees: usize,
//! }
//!
//! impl Doc for Product {
//!     // Mandatory associated items:
//!     type Id = ObjectId;
//!
//!     const NAME: &'static str = "Product";
//!
//!     // optionally, you can e.g. override the `indexes()` method:
//!     fn indexes() -> Vec<IndexModel> {
//!         vec![
//!             IndexModel {
//!                 keys: doc!{ "name": Order::Ascending },
//!                 options: IndexOptions::default(),
//!             }
//!         ]
//!     }
//! }
//! # fn main() {}
//! ```
//! Note that the model types `Job` and `Product`:
//!   * Implement the `Serialize` and `Deserialize` traits
//!   * Implement the `Debug` trait. This is *not* strictly necessary, however
//!     it is **very strongly** recommended.
//!   * Have a field which is serialized as `_id`. It doesn't matter what the
//!     name of the field is in Rust; here it's `id` but it could have been
//!     anything else, as long as it serializes/deserializes as `_id` in BSON.
//!   * the `Id` associated type is exactly the type of the `_id` field
//!   * the `NAME` associated constant describes and identifies the collection
//!     of values of this type.
//!
//! This trait is also responsible for a couple of other collection-related
//! properties, such as specifying the indexes to be created on this collection,
//! by means of the `indexes()` static method. By default, this returns an
//! empty vector meaning no custom indexes apart from the automatically-created
//! index on the `_id` field.
//!
//! A couple more static methods are also available for customizing the default
//! behavior of the collection when performing various database operations,
//! e.g. querying or insertion. If you don't implement these methods, they
//! return sensible defaults. We'll see more on this later.
//!
//! When `Doc` trait is `#[derive]`d, in which case the `Id` type is bound to
//! the type of whichever field serializes as `_id`. If there's 0 or more than
//! 1 such fields, you will get a compile-time error. The `NAME` constant will
//! be set to the name of the type, respecting the `#[serde(rename = "...")]`
//! attribute at all times.
//!
//! A `#[derive]`d `Doc` trait will not implement the various `..._options()`
//! static methods either, leaving their implementations in the default state.
//! It is planned that in the future, the `indexes()` method will be derived
//! by considering further attributes; however, for now, it also remains at
//! its default implementation.
//!
//! ### Collections and Databases
//!
//! Once we have defined our entity types, we can start storing and retrieving
//! them. For this, we'll need a database of collections, and one collection
//! per entity type.
//!
//! Avocado piggybacks on top of the `mongodb` crate. You connect to a MongoDB
//! client using exactly the same code that you would use if you were using
//! the driver in its "raw" form, and you obtain a named database in exactly
//! the same manner.
//!
//! Once you have a handle to the desired database, you obtain a handle to a
//! collection within that database. This is where the workflow departs from
//! that of the `mongodb` crate: Avocado has its own, strongly-typed, generic
//! `Collection` type. Let's see how these different parts all work together:
//! ```no_run
//! # #[macro_use]
//! # extern crate magnet_derive;
//! # extern crate magnet_schema;
//! # #[macro_use]
//! # extern crate serde_derive;
//! # extern crate serde;
//! # extern crate bson;
//! # extern crate mongodb;
//! # extern crate avocado;
//! # use avocado::prelude::*;
//! #[derive(Debug, Clone, Serialize, Deserialize, BsonSchema)]
//! struct User {
//!     #[serde(rename = "_id")]
//!     id: ObjectId,
//!     legal_name: String,
//! }
//!
//! impl Doc for User {
//!     type Id = ObjectId;
//!     const NAME: &'static str = "User";
//! }
//!
//! # fn main() -> AvocadoResult<()> {
//! // Connect to the server using the underlying mongodb crate.
//! let client = Client::with_uri("mongodb://localhost:27017/")?;
//!
//! // Obtain a database handle, still using the underlying mongodb crate.
//! let db = client.db("avocado_example_db");
//!
//! // Avocado extends database handle types with useful methods which let you
//! // obtain strongly-typed, generic collection handles.
//!
//! // This is how you obtain such a **new, empty** collection without dynamic
//! // schema validation. Note that **this drops and recreates the collection.**
//! let users_novalidate: Collection<User> = db.empty_collection_novalidate()?;
//!
//! // If you also enable the `schema_validation` feature, you can ask for a
//! // collection which always validates inserted documents based on its schema.
//! // Of course, this also **drops and recreates the collection.**
//! let users: Collection<User> = db.empty_collection()?;
//!
//! // If you need to access an **existing collection without emptying it,**
//! // here's how you do it:
//! let users_existing: Collection<User> = db.existing_collection();
//! # Ok(())
//! # }
//! ```
//!
//! ### Operations
//!
//! Once we get hold of a collection, we can finally start performing actual
//! database operations. Some of the most basic ones are:
//!   1. First, we can try and insert some entities.
//!   2. Then, we can update them based on their identity (`_id` field).
//!   3. Finally, we can retrieve them subject to some filtering criteria.
//!
//! Let's see what this looks like in terms of concrete code!
//! ```no_run
//! # extern crate magnet_schema;
//! # #[macro_use]
//! # extern crate serde_derive;
//! # extern crate serde;
//! # extern crate bson;
//! # extern crate mongodb;
//! # extern crate avocado;
//! # use avocado::prelude::*;
//! #
//! # #[derive(Debug, Clone, Serialize, Deserialize)]
//! # struct User {
//! #    #[serde(rename = "_id")]
//! #    id: ObjectId,
//! #    legal_name: String,
//! # }
//! #
//! # impl Doc for User {
//! #     type Id = ObjectId;
//! #     const NAME: &'static str = "User";
//! # }
//! #
//! #
//! # fn main() -> AvocadoResult<()> {
//! # let client = Client::with_uri("mongodb://localhost:27017/")?;
//! # let db = client.db("avocado_example_db");
//! # let users: Collection<User> = db.empty_collection_novalidate()?;
//! #
//! let alice = User {
//!     id: ObjectId::new()?,
//!     legal_name: String::from("Alice Wonderland"),
//! };
//! let bob = User {
//!     id: ObjectId::new()?,
//!     legal_name: String::from("Robert Tables"), // xkcd.com/327
//! };
//! let mut eve = User {
//!     id: ObjectId::new()?,
//!     legal_name: String::from("Eve Sdropper"),
//! };
//!
//! // You can insert a single entity using `Collection::insert_one()`.
//! users.insert_one(&eve)?;
//!
//! // If you have multiple entities, it's more efficient to use
//! // `insert_many()` instead. It will save you precious network round-trips.
//! users.insert_many(vec![&alice, &bob])?;
//!
//! // Update all properties of an entity based on its identity.
//! eve.legal_name = String::from("Eve Adamson");
//! users.replace_entity(&eve)?;
//!
//! // If you want to insert the entity if one with the same ID doesn't exist,
//! // and update its fields if it does already exist, then use `upsert_entity`:
//! users.upsert_entity(&eve)?;
//!
//! // The above two methods constitute a very easy and quick solution to a
//! // common use case, but they aren't very flexible in terms of speciying
//! // finer-grain filter criteria; and setting each field of a large document
//! // may be inefficient too.
//! // So if you are looking for something more flexible or more efficient,
//! // try `update_one()`, `update_many()`, `upsert_one()`, or `upsert_many()`.
//!
//! // Now that we have some data, we can retrieve and filter it:
//! let filter_criteria = doc!{
//!     "legal_name": "Robert Tables",
//! };
//!
//! for result in users.find_many(filter_criteria)? {
//!     let entity = result?;
//!     println!("Found entity: {:#?}", entity);
//! }
//! #
//! # Ok(())
//! # }
//! ```
//!
//! Actually, instead of raw, loosely-typed BSON documents, you can specify
//! more sophisticated, custom objects for the filter criteria. For instance,
//! an example implementation thereof can be found in `examples/basic.rs`.
//!
//! In fact, several traits requiring some sort of filter specification are
//! implemented for `Document`, but you can always make your own. The very
//! purpose of these traits is to make manipulating the database safer and
//! less error-prone by not requiring programmers to write a separate, ad-hoc
//! query document each time they want to perform a query.
//!
//! For this more advanced (and recommended) use case, see the traits in the
//! [`ops` module](ops/index.html) and the corresponding
//! [methods on `Collection`](coll/struct.Collection.html#methods).
//!
//! For using more descriptive names for some constants in filter or update
//! specification documents, and also for preventing certain classes of typos
//! related to the stringly-typed nature of BSON, several "smart literal" types
//! are provided in the [`literal`](literal/index.html) module.

#![doc(html_root_url = "https://docs.rs/avocado/0.0.5")]
#![deny(missing_debug_implementations, missing_copy_implementations,
        trivial_casts, trivial_numeric_casts,
        unsafe_code,
        unstable_features,
        unused_import_braces, unused_qualifications, missing_docs)]
#![allow(clippy::single_match, clippy::match_same_arms, clippy::match_ref_pats,
         clippy::clone_on_ref_ptr, clippy::needless_pass_by_value)]
#![deny(clippy::wrong_pub_self_convention, clippy::used_underscore_binding,
        clippy::stutter, clippy::similar_names, clippy::pub_enum_variant_names,
        clippy::missing_docs_in_private_items,
        clippy::non_ascii_literal, clippy::unicode_not_nfc,
        clippy::result_unwrap_used, clippy::option_unwrap_used,
        clippy::option_map_unwrap_or_else, clippy::option_map_unwrap_or,
        clippy::filter_map,
        clippy::shadow_unrelated, clippy::shadow_reuse, clippy::shadow_same,
        clippy::int_plus_one, clippy::string_add_assign, clippy::if_not_else,
        clippy::invalid_upcast_comparisons,
        clippy::cast_precision_loss, clippy::cast_lossless,
        clippy::cast_possible_wrap, clippy::cast_possible_truncation,
        clippy::mutex_integer, clippy::mut_mut, clippy::items_after_statements,
        clippy::print_stdout, clippy::mem_forget, clippy::maybe_infinite_iter)]

#[macro_use]
extern crate bitflags;
extern crate mongodb;
#[macro_use]
extern crate bson;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate backtrace;

#[cfg(feature = "schema_validation")]
extern crate magnet_schema;

pub mod db;
pub mod coll;
pub mod cursor;
pub mod doc;
pub mod ops;
pub mod literal;
pub mod bsn;
pub mod utils;
pub mod error;
pub mod prelude;
