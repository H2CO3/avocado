//! Types describing elementary update (but not upsert) operations.

use std::borrow::Cow;
use std::collections::{ BTreeSet, BTreeMap };
use bson::Document;

/// An update specification: either field-value pairs, or update operators.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "cargo-clippy", allow(stutter))]
#[serde(untagged)]
pub enum UpdateSpec {
    /// The set of possible update operators. This has to be the first variant
    /// for correctly deserializing, because `Document` is too general and if
    /// it was the first, it would successfully deserialize from `Modify` too.
    Modify(Modification),
    /// Field-value pairs to set during the update.
    Replace(Document),
}

/// The set of possible update operators.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Modification {
    /// Replaces the value of given fields with the specified values.
    #[serde(rename = "$set", default, skip_serializing_if = "Document::is_empty")]
    pub set: Document,
    /// Removes the specified fiels from documents. In arrays, only sets
    /// the element to `null` but does not remove it.
    #[serde(rename = "$unset", default, with = "serde_unset")]
    pub unset: BTreeSet<Cow<'static, str>>,
    /// Sets the value of the given fiels to the current date.
    #[serde(rename = "$currentDate", default, skip_serializing_if = "BTreeMap::is_empty")]
    pub set_current_date: BTreeMap<Cow<'static, str>, DateTimeType>,
    /// Renames the given fields using the new name specified as the value.
    #[serde(rename = "$rename", default, skip_serializing_if = "BTreeMap::is_empty")]
    pub rename: BTreeMap<Cow<'static, str>, Cow<'static, str>>,
    /// Increments the specified fields by the given amount, which may be negative.
    #[serde(rename = "$inc", default, skip_serializing_if = "Document::is_empty")]
    pub inc: Document,
    /// Multiplies the specified fields by the given factor.
    #[serde(rename = "$mul", default, skip_serializing_if = "Document::is_empty")]
    pub mul: Document,
    /// Sets the value of each field only if the specified value is *less*
    /// than the already-existing value of the respective field.
    #[serde(rename = "$min", default, skip_serializing_if = "Document::is_empty")]
    pub min: Document,
    /// Sets the value of each field only if the specified value is
    /// *greater* than the already-existing value of the respective field.
    #[serde(rename = "$max", default, skip_serializing_if = "Document::is_empty")]
    pub max: Document,
}

/// Tells the `$currentDate` operator which type it should use for setting its fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DateTimeType {
    /// the `$currentDate` operator will set the field to a timestamp value.
    Timestamp,
    /// the `$currentDate` operator will set the field to a `Date` value.
    Date,
}

/// Helper module for serializing and deserializing the path arguments of the
/// `$unset` operator in the correct format.
mod serde_unset {
    use std::borrow::Cow;
    use std::collections::BTreeSet;
    use bson::Document;
    use serde::ser::{ Serializer, SerializeMap };
    use serde::de::{ Deserialize, Deserializer };

    /// Serializes a set of paths / field names as a dummy document (with null values)
    pub fn serialize<S: Serializer>(paths: &BTreeSet<Cow<'static, str>>, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(paths.len()))?;
        for path in paths {
            map.serialize_entry(path, &())?;
        }
        map.end()
    }

    /// Deserializes a document and returns its keys, ignoring the values.
    pub fn deserialize<'a, D: Deserializer<'a>>(deserializer: D) -> Result<BTreeSet<Cow<'static, str>>, D::Error> {
        Document::deserialize(deserializer).map(|doc| doc.into_iter().map(|(key, _)| key.into()).collect())
    }
}
