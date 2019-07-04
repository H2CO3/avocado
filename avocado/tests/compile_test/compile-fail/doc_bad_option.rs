#[macro_use]
extern crate avocado_derive;
extern crate avocado;
#[macro_use]
extern crate serde_derive;
extern crate serde;

use avocado::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, Doc)] //~ ERROR proc-macro derive panicked
#[options(nonexistent_options = "my_options_fn")] //~| no option method named `Doc::nonexistent_options()`
struct MyDoc {
    _id: String,
}

fn my_options_fn() -> FindOptions {
    Default::default()
}
