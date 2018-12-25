#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate avocado_derive;
extern crate avocado;

use std::any::TypeId;
use avocado::prelude::*;

/// This could have been a function, but making it a macro results in the
/// error messages pointing to the actual line number of the invocation,
/// which is much better in a test suite.
macro_rules! assert_doc_impl {
    (Doc: $Doc:ident, Id: $Id:ident, name: $name:ident, index: $index:expr) => {
        assert_eq!(<$Doc as Doc>::NAME, stringify!($name));
        assert_eq!(TypeId::of::<<$Doc as Doc>::Id>(), TypeId::of::<$Id>());
        assert_eq!(<$Doc as Doc>::indexes(), $index);
    }
}

#[test]
fn doc_simple() {
    type DocId = ObjectId;

    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct Simple {
        _id: DocId,
    }

    assert_doc_impl!(Doc: Simple, Id: DocId, name: Simple, index: &[]);
}

#[test]
fn doc_simple_with_multiple_fields() {
    type DocId = ObjectId;

    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct MultiField {
        _id: DocId,
        name: String,
    }

    assert_doc_impl!(Doc: MultiField, Id: DocId, name: MultiField, index: &[]);
}

#[test]
fn doc_renamed_type() {
    type DocId = ObjectId;

    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    #[serde(rename = "Renamed")]
    struct Original {
        _id: DocId,
        other_field: Vec<String>,
    }

    assert_doc_impl!(Doc: Original, Id: DocId, name: Renamed, index: &[]);
}

#[test]
fn doc_non_object_id() {
    type DocId = u64;

    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct Foo {
        _id: DocId,
        stuff: Option<u8>
    }

    assert_doc_impl!(Doc: Foo, Id: DocId, name: Foo, index: &[]);
}

#[test]
fn doc_renamed_id_field() {
    type DocId = String;

    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct Bar {
        #[serde(rename = "_id")]
        qux: DocId,
    }

    assert_doc_impl!(Doc: Bar, Id: DocId, name: Bar, index: &[]);
}

#[test]
fn doc_first_id_field_is_used() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct Baz {
        _id: i32,
        #[serde(rename = "_id")]
        second_id: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct Qux {
        #[serde(rename = "_id")]
        first_id: ObjectId,
        _id: u32,
    }

    assert_doc_impl!(Doc: Baz, Id: i32,      name: Baz, index: &[]);
    assert_doc_impl!(Doc: Qux, Id: ObjectId, name: Qux, index: &[]);
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
    struct Renaming {
        lol_foo: String,
        #[serde(rename = "_id")]
        wat_bar: u64,
        _id: i32,
    }

    assert_doc_impl!(Doc: Renaming, Id: u64, name: Renaming, index: &[]);
}

/*
/// TODO(H2CO3): Uncomment me occasionally.
#[test]
fn doc_no_id_field() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    #[serde(rename_all = "UPPERCASE")]
    struct Bar {
        _id: String,
    }

    panic!("This MUST NOT COMPILE: there's no field serialized as `_id`!");
}
 */

#[test]
fn doc_id_partial_skip_allowed() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct SkipSer {
        #[serde(skip_serializing)]
        _id: ObjectId,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct SkipDe {
        #[serde(skip_deserializing)]
        _id: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct SkipSerIf {
        #[serde(skip_serializing_if = "is_zero")]
        _id: i32,
    }

    fn is_zero(id: &i32) -> bool {
        *id == 0
    }

    assert_doc_impl!(Doc: SkipSer,   Id: ObjectId, name: SkipSer,   index: &[]);
    assert_doc_impl!(Doc: SkipDe,    Id: String,   name: SkipDe,    index: &[]);
    assert_doc_impl!(Doc: SkipSerIf, Id: i32,      name: SkipSerIf, index: &[]);
}

#[test]
fn doc_non_id_field_skip_allowed() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct SkipNonId {
        _id: ObjectId,
        #[serde(skip)]
        unimportant: Vec<u8>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct SkipSerDeNonId {
        _id: u16,
        #[serde(skip_deserializing, skip_serializing)]
        dont_care: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct SkipSerDeNonIdMultiAttr {
        _id: u32,
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
    struct SkippyOne {
        #[serde(skip_serializing, skip_deserializing)]
        _id: i64,
        #[serde(rename = "_id", skip)]
        renamed_field: String,
    }

    panic!("This MUST NOT COMPILE: all fields serialized as `_id` are skipped!");
}
 */

/*
/// TODO(H2CO3): Uncomment me occasionally.
#[test]
fn doc_id_skipped_2() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct SkippyTwo {
        #[serde(skip)]
        _id: u64,
        #[serde(rename = "_id", skip_serializing, skip_deserializing)]
        renamed_field: u32,
    }

    panic!("This MUST NOT COMPILE: all fields serialized as `_id` are skipped!");
}
 */

/*
/// TODO(H2CO3): Uncomment me occasionally.
#[test]
fn doc_id_skipped_3() {
    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct SkippyThree {
        #[serde(skip)]
        _id: u64,
        #[serde(rename = "_id")]
        #[serde(skip_serializing)]
        #[serde(skip_deserializing)]
        renamed_field: u32,
    }

    panic!("This MUST NOT COMPILE: all fields serialized as `_id` are skipped!");
}
 */

#[test]
#[ignore]
/// TODO(H2CO3): make it so that `_id: Option<T>` results in `type Id = T;`
fn doc_option_id() {
    use std::option;

    type DocId = ObjectId;

    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct Opt1 {
        _id: Option<DocId>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct Opt2 {
        _id: option::Option<DocId>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct Opt3 {
        _id: core::option::Option<DocId>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Doc)]
    struct Opt4 {
        _id: std::option::Option<DocId>,
    }

    assert_doc_impl!(Doc: Opt1, Id: DocId, name: Opt1, index: &[]);
    assert_doc_impl!(Doc: Opt2, Id: DocId, name: Opt2, index: &[]);
    assert_doc_impl!(Doc: Opt3, Id: DocId, name: Opt3, index: &[]);
    assert_doc_impl!(Doc: Opt4, Id: DocId, name: Opt4, index: &[]);
}

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
            _id: String
        },
        Bar {
            _id: ObjectId,
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
