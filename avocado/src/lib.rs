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
//! #[macro_use]
//! extern crate avocado_derive;
//! extern crate avocado;
//!
//! use avocado::prelude::*;
//! #
//! # fn main() {}
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
//!   the key `_id` at the top level. **The corresponding field of the `struct`
//!   must be of type `Uid<T>` or `Option<Uid<T>>`,** where `T` is the document
//!   type itself (what would be `Self` in a trait).
//!
//!   If the `_id` field is an `Option<Uid<T>>`, it must be marked with
//!   `#[serde(skip_serializing_if = "Option::is_none")]`, because `null` IDs
//!   won't be able to be returned via `insert_one`, for example (since they
//!   don't deserialize successfully as a `Uid<T>`).
//!
//! * It has a name that is globally unique within the given MongoDB database
//!
//! These constraints are captured by the [`Doc`](doc/trait.Doc.html) trait.
//!
//! Here's an example of how you can `#[derive]` or manually implement `Doc`
//! for your entity types:
//!
//! ```
//! # #[macro_use]
//! # extern crate serde_derive;
//! # #[macro_use]
//! # extern crate avocado_derive;
//! # extern crate avocado;
//! #
//! # use avocado::prelude::*;
//! #
//! // Automatically, for more convenience and sensible defaults, respecting
//! // Serde renaming conventions
//! #[derive(Debug, Serialize, Deserialize, Doc)]
//! struct Job {
//!     #[serde(rename = "_id")]
//!     pub id: Uid<Job>,
//!     pub description: String,
//!     pub salary: u32,
//! }
//! #
//! # fn main() {}
//! ```
//! ```
//! # #[macro_use]
//! # extern crate serde_derive;
//! # extern crate serde;
//! # extern crate avocado;
//! #
//! # use avocado::prelude::*;
//! #
//! // Manually, for complete flexibility and fine-grained control over indexes
//! // and database operation options
//! #[derive(Debug, Serialize, Deserialize)]
//! struct Product {
//!     #[serde(rename = "_id")]
//!     pub id: Uid<Product>,
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
//!     fn id(&self) -> Option<&Uid<Self>> {
//!         Some(&self.id)
//!     }
//!
//!     fn set_id(&mut self, id: Uid<Self>) {
//!         self.id = id;
//!     }
//!
//!     // optionally, you can e.g. override the `indexes()` method:
//!     fn indexes() -> Vec<IndexModel> {
//!         vec![
//!             IndexModel {
//!                 keys: doc!{
//!                     "name": IndexType::Ordered(Order::Ascending),
//!                 },
//!                 options: IndexOptions::default(),
//!             }
//!         ]
//!     }
//! }
//! #
//! # fn main() {}
//! ```
//! Note that the model types `Job` and `Product`:
//!   * Implement the `Serialize` and `Deserialize` traits
//!   * Implement the `Debug` trait. This is *not* strictly necessary, however
//!     it is **very strongly** recommended.
//!   * Have a field which is serialized as `_id`. It doesn't matter what the
//!     name of the field is in Rust; here it's `id` but it could have been
//!     anything else, as long as it serializes/deserializes as `_id` in BSON.
//!   * the `Job::Id` associated type is the underlying raw type of `Uid<Job>`,
//!     and the same holds for `Product`. When deriving `Doc`, it is controlled
//!     by the `#[id_type = "..."]` attribute on the struct declaration. If you
//!     don't specify this attribute, the raw ID type will default to `ObjectId`.
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
//! When the `Doc` trait is `#[derive]`d, the `Id` type is bound to the type
//! of whichever field serializes as `_id`. If there's 0 or more than 1 such
//! fields, you will get a compile-time error. The `NAME` constant will
//! be set to the name of the type, respecting the `#[serde(rename = "...")]`
//! attribute at all times.
//!
//! A `#[derive]`d `Doc` trait will not implement the various `..._options()`
//! static methods either, leaving their implementations in the default state.
//!
//! ### Deriving `Doc` with indexes
//!
//! The `#[index(...)]` attribute can be applied to a type several times in
//! order to generate index specifications and implement the `Doc::indexes()`
//! static method. An example is provided below:
//!
//! ```
//! # #[macro_use]
//! # extern crate serde_derive;
//! # #[macro_use]
//! # extern crate avocado_derive;
//! # extern crate avocado;
//! #
//! # use avocado::prelude::*;
//! #
//! #[derive(Debug, Serialize, Deserialize)]
//! struct NaiveDate {
//!     year: u32,
//!     month: u32,
//!     day: u32,
//! }
//!
//! #[derive(Debug, Serialize, Deserialize, Doc)]
//! #[id_type = "u64"]
//! #[index(keys(name = "ascending"))]
//! #[index(
//!     unique,
//!     sparse = false,
//!     name = "establishment_index",
//!     min = "-129.5",
//!     bits = 26,
//!     keys(
//!         established::year  = "descending",
//!         established::month = "ascending",
//!     )
//! )]
//! #[index(keys(geolocation_lng_lat = "2dsphere"))]
//! struct Department {
//!     #[serde(rename = "_id")]
//!     guid: Uid<Department>,
//!     name: Option<String>,
//!     established: NaiveDate,
//!     employees: Vec<ObjectId>,
//!     geolocation_lng_lat: [f32; 2],
//! }
//! #
//! # fn main() {
//!
//! assert_eq!(Department::indexes(), &[
//!     IndexModel {
//!         keys: doc!{
//!             "name": IndexType::Ordered(Order::Ascending),
//!         },
//!         options: IndexOptions::default(),
//!     },
//!     IndexModel {
//!         keys: doc!{
//!             "established.year":  IndexType::Ordered(Order::Descending),
//!             "established.month": IndexType::Ordered(Order::Ascending),
//!         },
//!         options: IndexOptions {
//!             unique: Some(true),
//!             sparse: Some(false),
//!             name: Some(String::from("establishment_index")),
//!             bits: Some(26),
//!             min: Some(-129.5),
//!             ..Default::default()
//!         },
//!     },
//!     IndexModel {
//!         keys: doc!{
//!             "geolocation_lng_lat": IndexType::Geo2DSphere,
//!         },
//!         options: IndexOptions::default(),
//!     },
//! ]);
//! #
//! # }
//! ```
//!
//! This demonstrates the usage of the `index` attribute. To sum up:
//! * Fields to be indexed are given as path-value pairs in the `keys`
//!   sub-attribute. The paths specify the field names whereas the values
//!   describe the type of index that should be created.
//!   * Multi-component paths, such as `foo::bar::qux`, can be used to index
//!     a field of an embedded document or array. This is equivalent with
//!     MongoDB's "dot notation", e.g. the above example translates to the
//!     key `"foo.bar.qux"` in the resuling BSON document.
//!   * If a path (field name) occurs multiple times in the key list, the
//!     last occurrence will overwrite any previous ones.
//!   * The correctness of indexed field names/paths, i.e. the fact that they
//!     indeed exist in the `Doc`ument type, is **not currently enforced.**
//!     This is to allow indexes to be created on fields that only exist
//!     dynamically, e.g. a `HashMap` which is `#[serde(flatten)]`ed into
//!     its containing `struct` type.
//!
//!     In the future, this behavior will be improved: the existence of the
//!     first segment of each field name will be enforced by default. Only the
//!     first segment is checked because further segments, referring to embedded
//!     documents/arrays, can't be checked, as the derive macro doesn't receive
//!     type information, so it only knows about the field names of the type
//!     it is being applied to. It will then be possible for individual fields
//!     to opt out of this constraint, e.g. using a `dynamic` attribute.
//!   * The possible values of the index type are:
//!     * `ascending`
//!     * `descending`
//!     * `text`
//!     * `hashed`
//!     * `2d`
//!     * `2dsphere`
//!     * `geoHaystack`
//! * Additional, optional configuration attributes can be specified, such as
//!   `unique`, `sparse` or `name`. The `name` attribute must be string-valued.
//!   The `unique` and `sparse` switches are either boolean-valued key-value
//!   pairs, or bare words. Specifying a bare word is equivalent with setting
//!   it to `true`, e.g. `unique` is the same as `unique = true`.
//! * The rest of the supported options are:
//!   * `max = 85.0` &mdash; maximal longitude/latitude for `2d` indexes.
//!     This must be a floating-point number in the range `[-180, +180]`.
//!     Use a string to specify negative values.
//!   * `min = "-129.5"` &mdash; minimal allowed longitude/latitude.
//!   * `bits = 26` &mdash; number of bits to set precision of a `2d` index.
//!     Must be an integer between 1 and 32, inclusive.
//!   * `bucket_size` &mdash; grouping granularity of `GeoHaystack` indexes.
//!     Must be a strictly positive integer.
//!   * `default_language = "french"` &mdash; default language of a text index.
//!   * `language_override = "lang"` &mdash; field name that indicates the
//!     language of a document.
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
//! # #[macro_use]
//! # extern crate avocado_derive;
//! # extern crate avocado;
//! # extern crate bson;
//! # extern crate mongodb;
//! #
//! # use avocado::prelude::*;
//! #
//! #[derive(Debug, Clone, Serialize, Deserialize, BsonSchema, Doc)]
//! struct User {
//!     #[serde(rename = "_id")]
//!     id: Uid<User>,
//!     legal_name: String,
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
//! // It also creates any indexes specified in the `Doc::indexes()` method.
//! let users_novalidate: Collection<User> = db.empty_collection_novalidate()?;
//!
//! // If you also enable the `schema_validation` feature, you can ask for a
//! // collection which always validates inserted documents based on its schema.
//! // Of course, this also **drops and recreates the collection,** and
//! // it also creates any indexes specified in the `Doc::indexes()` method.
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
//! # #[macro_use]
//! # extern crate avocado_derive;
//! # extern crate avocado;
//! #
//! # use avocado::prelude::*;
//! #
//! # #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
//! # struct User {
//! #    #[serde(rename = "_id")]
//! #    id: Uid<User>,
//! #    legal_name: String,
//! # }
//! #
//! # fn main() -> AvocadoResult<()> {
//! # let client = Client::with_uri("mongodb://localhost:27017/")?;
//! # let db = client.db("avocado_example_db");
//! # let users: Collection<User> = db.empty_collection_novalidate()?;
//! #
//! let alice = User {
//!     id: Uid::new_oid()?,
//!     legal_name: String::from("Alice Wonderland"),
//! };
//! let bob = User {
//!     id: Uid::new_oid()?,
//!     legal_name: String::from("Robert Tables"), // xkcd.com/327
//! };
//! let mut eve = User {
//!     id: Uid::new_oid()?,
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
//!
//! A short example:
//!
//! ```no_run
//! # #[macro_use]
//! # extern crate serde_derive;
//! # #[macro_use]
//! # extern crate avocado_derive;
//! # extern crate avocado;
//! #
//! # use avocado::prelude::*;
//! #
//! #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
//! struct Recipe {
//!     #[serde(rename = "_id")]
//!     id: Uid<Recipe>,
//!     ingredients: Vec<String>,
//!     description: String,
//! }
//!
//! #[derive(Debug, Clone)]
//! struct AddIngredient<'a> {
//!     recipe_id: &'a Uid<Recipe>,
//!     ingredient: &'a str,
//! }
//!
//! impl<'a> Update<Recipe> for AddIngredient<'a> {
//!     fn filter(&self) -> Document {
//!         doc!{ "_id": self.recipe_id }
//!     }
//!
//!     fn update(&self) -> Document {
//!         doc!{
//!             "$push": {
//!                 "ingredients": self.ingredient
//!             }
//!         }
//!     }
//! }
//!
//! #[derive(Debug, Clone, Copy)]
//! struct GetDescription<'a> {
//!     recipe_id: &'a Uid<Recipe>,
//! }
//!
//! impl<'a> Query<Recipe> for GetDescription<'a> {
//!     type Output = String;
//!
//!     fn filter(&self) -> Document {
//!         doc!{ "_id": self.recipe_id }
//!     }
//!
//!     fn transform(mut raw: Document) -> AvocadoResult<Bson> {
//!         raw.remove("description").ok_or_else(|| AvocadoError::new(
//!             AvocadoErrorKind::MissingDocumentField,
//!             "no field `description` in entity `Recipe`"
//!         ))
//!     }
//!
//!     fn options() -> FindOptions {
//!         FindOptions {
//!             projection: Some(doc!{
//!                 "_id": false,
//!                 "description": true,
//!             }),
//!             ..Default::default()
//!         }
//!     }
//! }
//!
//! # fn main() -> AvocadoResult<()> {
//! #
//! let client = Client::with_uri("mongodb://localhost:27017/")?;
//! let db = client.db("avocado_example_db");
//! let recipes: Collection<Recipe> = db.empty_collection_novalidate()?;
//!
//! // Create a new `Recipe` entity and save it to the database.
//! let r = Recipe {
//!     id: Uid::new_oid()?,
//!     ingredients: vec![String::from("cream"), String::from("sugar")],
//!     description: String::from("mix 'em all together"),
//! };
//! recipes.insert_one(&r)?;
//!
//! // Add an extra ingredient to it.
//! let u = AddIngredient {
//!     recipe_id: &r.id,
//!     ingredient: "strawberries",
//! };
//! recipes.update_one(&u)?;
//!
//! // Retrieve its description in case we already forgot it.
//! let q = GetDescription { recipe_id: &r.id };
//! let description = recipes.find_one(q)?;
//! assert_eq!(description.as_ref(), Some(&r.description));
//! #
//! # Ok(())
//! # }
//! ```
//!
//! ### Preventing NoSQL Injection
//!
//! Basically any database technology is subject to the hazard of DDL/DML
//! (query, modification, and administrative) **injection attacks** if not
//! enough care is taken.
//!
//! In the case of traditional relational DB engines, the use of untrusted
//! (e.g. user-supplied) text in formatted / templated SQL strings, and thus
//! the concatenation of potentially arbitrary executable code with what was
//! intended by the programmer, is the most common source of these security
//! bugs.
//!
//! This is usually mitigated by the use of "prepared statements", meaning
//! that SQL statements are precompiled without any user input, while external
//! values/arguments are marked by special placeholder syntax. Then, for the
//! actual execution of a precompiled statement, parameters are structurally
//! bound to each placeholder in the statement, i.e. by supplying typed values
//! to the DB engine **after** parsing, without textually pasting them together
//! with the query script.
//!
//! Several NoSQL databases, including MongoDB, use a more structured query
//! interface. (In fact, MongoDB queries are almost like the programmer writes
//! plain syntax trees by hand.) This gets rid of **some** of the textual
//! injection attempts. However, in a loosely-typed environment, supplying a
//! query with arbitrary untrusted input can still lead to injection. For
//! example, if one is directly working with the loosely-typed "value tree"
//! representation of JSON, a malicious user might supply a MongoDB query
//! operator document where the programmer was expecting a plain string.
//! An example of this mistake can be found [here](https://ckarande.gitbooks.io/owasp-nodegoat-tutorial/content/tutorial/a1_-_sql_and_nosql_injection.html#2-nosql-injection).
//!
//! Avocado tries to counter these problems by encouraging the **use of static
//! types in queries** as well as domain models. Therefore, any time you are
//! are handling untrusted input, you should build strongly-typed query and/or
//! update objects implementing the [`Query`](ops/trait.Query.html),
//! [`Update`](ops/trait.Update.html), [`Upsert`](ops/trait.Upsert.html),
//! [`Delete`](ops/trait.Delete.html), etc. traits from the ops module,
//! instead of using what is effectively dynamic typing with raw BSON or JSON.
//!
//! Ideally, the "no raw JSON/BSON" rule should be applied **transitively**
//! in (recursive) data structures: no struct or tuple fields, enum variants,
//! map keys, map/set/array values, etc., nor any substructures threof should
//! contain untyped data.
//!
//! ### Crate Features
//!
//! * `schema_validation` (default): enables MongoDB-flavored JSON schema
//!   validation via the `magnet_schema` crate.
//! * `raw_uuid` (default): augments the [`Uid`](uid/struct.Uid.html) type
//!   with convenience methods for working with UUID-based entity/document IDs.

#![doc(html_root_url = "https://docs.rs/avocado/0.2.0")]
#![deny(missing_debug_implementations, missing_copy_implementations,
        trivial_casts, trivial_numeric_casts,
        unsafe_code,
        unstable_features,
        anonymous_parameters, bare_trait_objects,
        variant_size_differences,
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
#[cfg(feature = "raw_uuid")]
extern crate uuid;

pub mod db;
pub mod coll;
pub mod cursor;
pub mod doc;
pub mod uid;
pub mod ops;
pub mod literal;
pub mod error;
pub mod prelude;

mod bsn;
mod utils;
