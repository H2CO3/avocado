#[macro_use]
extern crate avocado_derive;
extern crate avocado;
#[macro_use]
extern crate serde_derive;
extern crate serde;

#[derive(Debug, Clone, Serialize, Deserialize, Doc)] //~ ERROR: proc-macro derive panicked
struct Unit; //~| a `Doc` must be a struct with named fields

fn main() {}
