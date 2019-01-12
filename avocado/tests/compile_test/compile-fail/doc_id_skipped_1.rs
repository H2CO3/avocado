#[macro_use]
extern crate avocado_derive;
extern crate avocado;
#[macro_use]
extern crate serde_derive;
extern crate serde;

#[derive(Debug, Clone, Serialize, Deserialize, Doc)] //~ ERROR proc-macro derive panicked
#[id_type = "i64"] //~| a `Doc` must contain a field serialized as `_id`
struct SkippyOne {
    #[serde(skip_serializing, skip_deserializing)]
    _id: Uid<SkippyOne>,
    #[serde(rename = "_id", skip)]
    renamed_field: Uid<SkippyOne>,
}

fn main() {}
