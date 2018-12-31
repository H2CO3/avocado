#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate avocado_derive;
extern crate avocado;

use std::marker::PhantomData;
use std::any::TypeId;
use avocado::prelude::*;

/// This could have been a function, but making it a macro results in the
/// error messages pointing to the actual line number of the invocation,
/// which is much better in a test suite.
macro_rules! assert_doc_impl {
    (Doc: $Doc:ty, Id: $Id:ty, name: $name:ident, index: $index:expr) => {
        assert_eq!(<$Doc as Doc>::NAME, stringify!($name));
        assert_eq!(TypeId::of::<<$Doc as Doc>::Id>(), TypeId::of::<$Id>());
        assert_eq!(<$Doc as Doc>::indexes(), $index);
    }
}

#[test]
fn doc_simple() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct Simple {
        _id: Uid<Simple>,
    }

    assert_doc_impl!(Doc: Simple, Id: ObjectId, name: Simple, index: &[]);
}

#[test]
fn doc_simple_with_multiple_fields() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct MultiField {
        _id: Uid<MultiField>,
        name: String,
    }

    assert_doc_impl!(Doc: MultiField, Id: ObjectId, name: MultiField, index: &[]);
}

#[test]
fn doc_renamed_type() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    #[serde(rename = "Renamed")]
    struct Original {
        _id: Uid<Original>,
        other_field: Vec<String>,
    }

    assert_doc_impl!(Doc: Original, Id: ObjectId, name: Renamed, index: &[]);
}

#[test]
fn doc_non_object_id() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    #[id_type = "u64"]
    struct Foo {
        _id: Uid<Foo>,
        stuff: Option<u8>
    }

    assert_doc_impl!(Doc: Foo, Id: u64, name: Foo, index: &[]);
}

#[test]
fn doc_renamed_id_field() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    #[id_type = "String"]
    struct Bar {
        #[serde(rename = "_id")]
        qux: Uid<Bar>,
    }

    assert_doc_impl!(Doc: Bar, Id: String, name: Bar, index: &[]);
}

#[test]
fn doc_rename_all_id_field() {
    /// Since the name `_id` itself is already lower snake case, and Serde's
    /// `rename_all` attribute unconditionally assumes that field names are
    /// lower snake case, there's not much we can test with regards to renaming
    /// a non-lower-snake-case identifier to `_id` (because e.g. renaming `_ID`
    /// to lower snake case will do NOTHING if it's in field name position.)
    ///
    /// We still want to test if it cooperates nicely with plain `rename`,
    /// i.e. if `rename` takes precedence over `rename_all`.
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    #[serde(rename_all = "PascalCase")]
    #[id_type = "u64"]
    struct Renaming {
        lol_foo: String,
        #[serde(rename = "_id")]
        wat_bar: Uid<Renaming>,
        _id: i32, // this will be renamed to `Id` -> no duplicate `_id` fields
    }

    assert_doc_impl!(Doc: Renaming, Id: u64, name: Renaming, index: &[]);
}

/*
/// TODO(H2CO3): Uncomment me occasionally.
#[test]
fn doc_no_id_field() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    #[id_type = "String"]
    #[serde(rename_all = "UPPERCASE")]
    struct Bar {
        _id: Uid<Bar>,
    }

    panic!("This MUST NOT COMPILE: there's no field serialized as `_id`!");
}
 */

#[test]
fn doc_id_partial_skip_allowed() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct SkipSer {
        #[serde(skip_serializing)]
        _id: Uid<SkipSer>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    #[id_type = "String"]
    struct SkipDe {
        #[serde(skip_deserializing)]
        _id: Uid<SkipDe>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    #[id_type = "i32"]
    struct SkipSerIf {
        #[serde(skip_serializing_if = "is_zero")]
        _id: Uid<SkipSerIf>,
    }

    fn is_zero(id: &Uid<SkipSerIf>) -> bool {
        id.into_raw() == 0
    }

    assert_doc_impl!(Doc: SkipSer,   Id: ObjectId, name: SkipSer,   index: &[]);
    assert_doc_impl!(Doc: SkipDe,    Id: String,   name: SkipDe,    index: &[]);
    assert_doc_impl!(Doc: SkipSerIf, Id: i32,      name: SkipSerIf, index: &[]);
}

#[test]
fn doc_non_id_field_skip_allowed() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct SkipNonId {
        _id: Uid<SkipNonId>,
        #[serde(skip)]
        unimportant: Vec<u8>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    #[id_type = "u16"]
    struct SkipSerDeNonId {
        _id: Uid<SkipSerDeNonId>,
        #[serde(skip_deserializing, skip_serializing)]
        dont_care: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    #[id_type = "u32"]
    struct SkipSerDeNonIdMultiAttr {
        _id: Uid<SkipSerDeNonIdMultiAttr>,
        #[serde(skip_serializing)]
        #[serde(skip_deserializing)]
        dont_care_either: Option<String>,
    }

    assert_doc_impl!(Doc: SkipNonId, Id: ObjectId, name: SkipNonId, index: &[]);
    assert_doc_impl!(
        Doc: SkipSerDeNonId,
        Id: u16,
        name: SkipSerDeNonId,
        index: &[]
    );
    assert_doc_impl!(
        Doc: SkipSerDeNonIdMultiAttr,
        Id: u32,
        name: SkipSerDeNonIdMultiAttr,
        index: &[]
    );
}

/*
/// TODO(H2CO3): Uncomment me occasionally.
#[test]
fn doc_id_skipped_1() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    #[id_type = "i64"]
    struct SkippyOne {
        #[serde(skip_serializing, skip_deserializing)]
        _id: Uid<SkippyOne>,
        #[serde(rename = "_id", skip)]
        renamed_field: Uid<SkippyOne>,
    }

    panic!("This MUST NOT COMPILE: all fields serialized as `_id` are skipped!");
}
 */

/*
/// TODO(H2CO3): Uncomment me occasionally.
#[test]
fn doc_id_skipped_2() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    #[id_type = "u64"]
    struct SkippyTwo {
        #[serde(skip)]
        _id: Uid<SkippyTwo>,
        #[serde(rename = "_id", skip_serializing, skip_deserializing)]
        renamed_field: Uid<SkippyTwo>,
    }

    panic!("This MUST NOT COMPILE: all fields serialized as `_id` are skipped!");
}
 */

/*
/// TODO(H2CO3): Uncomment me occasionally.
#[test]
fn doc_id_skipped_3() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    #[id_type = "u64"]
    struct SkippyThree {
        #[serde(skip)]
        _id: Uid<SkippyThree>,
        #[serde(rename = "_id")]
        #[serde(skip_serializing)]
        #[serde(skip_deserializing)]
        renamed_field: Uid<SkippyThree>,
    }

    panic!("This MUST NOT COMPILE: all fields serialized as `_id` are skipped!");
}
 */

/*
/// TODO(H2CO3): Uncomment me occasionally.
#[test]
fn doc_unit_struct() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct Unit;

    panic!("This MUST NOT COMPILE: unit structs are not allowed");
}
 */

/*
/// TODO(H2CO3): Uncomment me occasionally.
#[test]
fn doc_tuple_struct() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct Tuple(String, Vec<u8>);

    panic!("This MUST NOT COMPILE: tuple structs are not allowed");
}
 */

/*
/// TODO(H2CO3): Uncomment me occasionally.
#[test]
fn doc_enum() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    enum Stuff {
        Foo {
            _id: Uid<Stuff>
        },
        Bar {
            _id: Uid<Stuff>
        },
    }

    panic!("This MUST NOT COMPILE: enums are not allowed");
}
 */

/*
/// TODO(H2CO3): Uncomment me occasionally.
#[test]
fn doc_union() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    union Foo {
        signed: i32,
        unsigned: u32,
    }

    panic!("This MUST NOT COMPILE: unions are not allowed");
}
 */

#[test]
fn doc_generic_lifetime_only() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    #[id_type = "u32"]
    struct GenericLifetime<'a> {
        _id: Uid<GenericLifetime<'a>>,
        dummy: PhantomData<&'a ()>,
    }

    assert_doc_impl!(Doc: GenericLifetime, Id: u32, name: GenericLifetime, index: &[]);
}

/*
/// TODO(H2CO3): Uncomment me occasionally.
#[test]
fn doc_generic_type_params() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct GenericType<T> {
        _id: Uid<GenericType<T>>,
        dummy: PhantomData<T>,
    }

    panic!("This MUST NOT COMPILE: generic types can't be `Doc`s");
}
 */

#[test]
fn doc_index() {
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Inner {
        x: i32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    #[index(keys(inner = "hashed"))]
    #[index(keys(rambling = "text"), unique)]
    #[index(
        name = "fluffy",
        unique = false,
        sparse,
        keys(
            _id = "ascending",
            inner = "descending",
        )
    )]
    struct Indexed {
        #[serde(rename = "_id")]
        guid: Uid<Indexed>,
        inner: Inner,
        rambling: String,
    }

    assert_doc_impl!(
        Doc: Indexed,
        Id: ObjectId,
        name: Indexed,
        index: &[
            IndexModel {
                keys: doc!{ "inner": IndexType::Hashed },
                options: Default::default(),
            },
            IndexModel {
                keys: doc!{ "rambling": IndexType::Text },
                options: IndexOptions {
                    unique: Some(true),
                    ..Default::default()
                },
            },
            IndexModel {
                keys: doc!{
                    "_id": IndexType::Ordered(Order::Ascending),
                    "inner": IndexType::Ordered(Order::Descending),
                },
                options: IndexOptions {
                    name: Some(String::from("fluffy")),
                    unique: Some(false),
                    sparse: Some(true),
                    ..Default::default()
                },
            },
        ]
    );
}

#[test]
fn doc_index_options() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    #[id_type = "u32"]
    #[index(
        default_language = "french",
        language_override = "lang",
        min = "-170.3",
        max = 89.5,
        bits = 28,
        bucket_size = 5,
        keys(_id = "ascending")
    )]
    struct Fancy {
        _id: Uid<Fancy>,
    }

    assert_eq!(Fancy::indexes(), [
        IndexModel {
            keys: doc!{
                "_id": IndexType::Ordered(Order::Ascending)
            },
            options: IndexOptions {
                default_language: Some(String::from("french")),
                language_override: Some(String::from("lang")),
                min: Some(-170.3),
                max: Some(89.5),
                bits: Some(28),
                bucket_size: Some(5),
                ..Default::default()
            },
        }
    ]);
}

#[test]
fn doc_index_embedded_paths() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    #[index(
        keys(
            embedded::_id = "ascending",
            embedded::nested::deep = "2d",
        )
    )]
    struct Embedding {
        _id: Uid<Embedding>,
        embedded: Document,
    }

    assert_doc_impl!(
        Doc: Embedding,
        Id: ObjectId,
        name: Embedding,
        index: &[
            IndexModel {
                keys: doc!{
                    "embedded._id": IndexType::Ordered(Order::Ascending),
                    "embedded.nested.deep": IndexType::Geo2D,
                },
                options: Default::default()
            }
        ]
    );
}
