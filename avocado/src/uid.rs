//! Strongly-typed unique entity IDs.

use std::{
    str::FromStr,
    cmp::{ PartialEq, Eq, PartialOrd, Ord, Ordering },
    hash::{ Hash, Hasher },
    fmt::{ Debug, Display, Formatter, Result as FmtResult },
};
use serde::{
    ser::{ Serialize, Serializer },
    de::{ Deserialize, Deserializer },
};
use crate::doc::Doc;

#[cfg(feature = "schema_validation")]
use magnet_schema::BsonSchema;

/// A newtype wrapper to provide type safety for unique IDs of `Doc`uments
/// that share the same underlying raw ID type.
///
/// It serializes and deserializes transparently as it were a `<T as Doc>::Id`.
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
        if formatter.alternate() {
            write!(formatter, "Uid<{}>({:#})", T::NAME, self.0)
        } else {
            write!(formatter, "Uid<{}>({})", T::NAME, self.0)
        }
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

#[cfg(feature = "schema_validation")]
impl<T: Doc> BsonSchema for Uid<T> where T::Id: BsonSchema {
    fn bson_schema() -> bson::Document {
        T::Id::bson_schema()
    }
}
