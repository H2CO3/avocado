#[macro_use]
extern crate avocado_derive;
extern crate avocado;
#[macro_use]
extern crate serde_derive;
extern crate serde;

#[derive(Debug, Clone, Serialize, Deserialize, Doc)] //~ ERROR proc-macro derive panicked
enum Stuff { //~| only a `struct` can be a top-level `Doc`; consider wrapping this type in a struct
    Foo {
        _id: Uid<Stuff>
    },
    Bar {
        _id: Uid<Stuff>
    },
}

fn main() {}
