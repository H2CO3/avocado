#[macro_use]
extern crate avocado_derive;
extern crate avocado;
#[macro_use]
extern crate serde_derive;
extern crate serde;

#[derive(Debug, Clone, Serialize, Deserialize, Doc)] //~ ERROR proc-macro derive panicked
#[id_type = "u64"] //~| a `Doc` must contain a field serialized as `_id`
struct SkippyThree {
    #[serde(skip)]
    _id: Uid<SkippyThree>,
    #[serde(rename = "_id")]
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    renamed_field: Uid<SkippyThree>,
}

fn main() {}
