#[macro_use]
extern crate avocado_derive;
extern crate avocado;
#[macro_use]
extern crate serde_derive;
extern crate serde;

#[derive(Debug, Clone, Serialize, Deserialize, Doc)] //~ ERROR proc-macro derive panicked
struct GenericType<T> { //~| `Doc` can't be derived for a type that is generic over type parameters
    _id: Uid<GenericType<T>>,
    dummy: PhantomData<T>,
}

fn main() {}
