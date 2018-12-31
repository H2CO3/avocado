//! Strongly-typed unique entity IDs.

use std::{
    str::FromStr,
    borrow::ToOwned,
    cmp::{ PartialEq, Eq, PartialOrd, Ord, Ordering },
    hash::{ Hash, Hasher },
    fmt::{ Debug, Display, Formatter, Result as FmtResult },
};
use serde::{
    ser::{ Serialize, Serializer },
    de::{ Deserialize, Deserializer },
};
use bson::{ Bson, oid::ObjectId };
use crate::{
    doc::Doc,
    error::Error,
};

#[cfg(feature = "schema_validation")]
use magnet_schema::BsonSchema;
#[cfg(feature = "raw_uuid")]
use uuid::Uuid;

/// A newtype wrapper to provide type safety for unique IDs of `Doc`uments
/// that share the same underlying raw ID type.
///
/// It serializes and deserializes transparently as if it were a value of
/// type `<T as Doc>::Id`.
pub struct Uid<T: Doc>(T::Id);

impl<T: Doc> Uid<T> {
    /// Creates a strongly-typed `Uid<T>` from a raw representation.
    pub fn from_raw(raw: T::Id) -> Self {
        Uid(raw)
    }

    /// Converts the strongly-typed `Uid<T>` into its raw representation.
    pub fn into_raw(self) -> T::Id {
        self.0
    }
}

/// Convenience methods for `ObjectId`-valued `Uid`s.
impl<T: Doc<Id = ObjectId>> Uid<T> {
    /// Generates a new `ObjectId`-valued unique ID.
    pub fn new_oid() -> Result<Self, Error> {
        ObjectId::new().map(Uid::from_raw).map_err(Into::into)
    }

    /// Constructs a wrapper around an `ObjectId` represented by raw bytes.
    pub fn from_oid_bytes(bytes: [u8; 12]) -> Self {
        Uid::from_raw(ObjectId::with_bytes(bytes))
    }

    /// Creates an `ObjectID`-valued `Uid` using a 12-byte (24-char)
    /// hexadecimal string.
    pub fn from_oid_str(s: &str) -> Result<Self, Error> {
        ObjectId::with_string(s).map(Uid::from_raw).map_err(Into::into)
    }
}

/// Convenience methods for `Uuid`-valued `Uid`s.
#[cfg(feature = "raw_uuid")]
impl<T: Doc<Id = Uuid>> Uid<T> {
    /// Creates a new random (v4) UUID-backed ID.
    pub fn new_uuid() -> Self {
        Uid::from_raw(Uuid::new_v4())
    }

    /// Creates a `Uid` backed by a `Uuid` of the exact bytes specified.
    pub fn from_uuid_bytes(bytes: [u8; 16]) -> Self {
        Uid::from_raw(Uuid::from_bytes(bytes))
    }

    /// Creates a `Uid` backed by a `Uuid` based on the bytes
    /// supplied, modified so that the result is a valid v4 variant.
    pub fn from_random_uuid_bytes(bytes: [u8; 16]) -> Self {
        Uid::from_raw(Uuid::from_random_bytes(bytes))
    }
}

impl<T: Doc> AsRef<T::Id> for Uid<T> {
    fn as_ref(&self) -> &T::Id {
        &self.0
    }
}

// Note that `Deref<Target=T::Id>` is intentionally not implemented for
// `Uid<T>` because it is often invoked implicitly, enabling (accidental)
// usage like `Uid::<T1>::from_raw(raw_1) == Uid::<T2>::from_raw(raw_2)` if
// `<T1 as Doc>::Id == <T2 as Doc>::Id`. Thus, it would basically neuter the
// type safety gained by using a newtype wrapper.
// In contrast, if the programmer has to write a manual `.into_raw()` or
// `.as_ref()` call, s/he will be reminded that the types don't match up
// and thus there is a potential mistake.

// The following traits are implemented manually in order to relax trait
// bounds and only put requirements on the actually-contained type, `T::Id`,
// instead of the enity type (`T`) itself.

impl<T: Doc> Clone for Uid<T> where T::Id: Clone {
    fn clone(&self) -> Self {
        Uid(self.0.clone())
    }
}

impl<T: Doc> Copy for Uid<T> where T::Id: Copy {}

impl<T: Doc> Default for Uid<T> where T::Id: Default {
    fn default() -> Self {
        Uid(T::Id::default())
    }
}

impl<T: Doc> PartialEq for Uid<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: Doc> Eq for Uid<T> {}

impl<T: Doc> PartialOrd for Uid<T> where T::Id: PartialOrd {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<T: Doc> Ord for Uid<T> where T::Id: Ord {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: Doc> Hash for Uid<T> where T::Id: Hash {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl<T: Doc> Debug for Uid<T> where T::Id: Debug {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter
            .debug_tuple(&format!("Uid<{}>", T::NAME))
            .field(&self.0)
            .finish()
    }
}

impl<T: Doc> Display for Uid<T> where T::Id: Display {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        self.0.fmt(formatter)
    }
}

impl<T: Doc> FromStr for Uid<T> where T::Id: FromStr {
    type Err = <T::Id as FromStr>::Err;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        string.parse().map(Uid::from_raw)
    }
}

impl<T: Doc> Serialize for Uid<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'a, T: Doc> Deserialize<'a> for Uid<T> {
    fn deserialize<D: Deserializer<'a>>(deserializer: D) -> Result<Self, D::Error> {
        T::Id::deserialize(deserializer).map(Uid::from_raw)
    }
}

impl<T: Doc> From<Uid<T>> for Bson where T::Id: Into<Bson> {
    fn from(uid: Uid<T>) -> Self {
        uid.into_raw().into()
    }
}

impl<T: Doc> From<&Uid<T>> for Bson where
    T::Id: ToOwned,
    <T::Id as ToOwned>::Owned: Into<Bson>,
{
    fn from(uid: &Uid<T>) -> Self {
        uid.as_ref().to_owned().into()
    }
}

#[cfg(feature = "schema_validation")]
impl<T: Doc> BsonSchema for Uid<T> where T::Id: BsonSchema {
    fn bson_schema() -> bson::Document {
        T::Id::bson_schema()
    }
}
