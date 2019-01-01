# Avocado, the strongly-typed MongoDB driver

[![Avocado on crates.io](https://img.shields.io/crates/v/avocado.svg)](https://crates.io/crates/avocado)
[![Avocado on docs.rs](https://docs.rs/avocado/badge.svg)](https://docs.rs/avocado)
[![Avocado Download](https://img.shields.io/crates/d/avocado.svg)](https://crates.io/crates/avocado)
[![Avocado License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/H2CO3/avocado/blob/master/LICENSE.txt)
[![Lines of Code](https://tokei.rs/b1/github/H2CO3/avocado)](https://github.com/Aaronepower/tokei)
[![Twitter](https://img.shields.io/badge/twitter-@H2CO3_iOS-blue.svg?style=flat&colorB=64A5DE&label=Twitter)](http://twitter.com/H2CO3_iOS)

[![goto counter](https://img.shields.io/github/search/H2CO3/avocado/goto.svg)](https://github.com/H2CO3/avocado/search?q=goto)
[![unsafe counter](https://img.shields.io/github/search/H2CO3/avocado/unsafe.svg)](https://github.com/H2CO3/avocado/search?q=unsafe)
[![fuck counter](https://img.shields.io/github/search/H2CO3/avocado/fuck.svg)](https://github.com/H2CO3/avocado/search?q=fuck)

## Usage

* See the [online documentation](https://docs.rs/avocado) above, or open it locally:
* `cargo doc --features schema_validation --open`
* Check out the [`examples/`](https://github.com/H2CO3/avocado/blob/master/examples/) folder
* The `schema_validation` feature can be enabled, in which case the `DatabaseExt::empty_collection()` method becomes available. If a collection is created using this method, it will add a JSON schema validation pass and specify the schema as generated by [`magnet`](https://github.com/H2CO3/magnet).

    **This can potentially be slow if you are performing many insertions into a collection of a complex type. However, it dynamically ensures that other users/drivers can't put malformed data in the collection.** Therefore it's probably more useful if you or somebody else are accessing a database from outside the Avocado driver too. It's also great for debugging Avocado itself.

## TODO:

* In `tests/ops.rs`, in macro `implement_tests!`, use the `?` Kleene operator around the return type of test functions, once it's stabilized (in Rust 1.32)
* Auto-derive `Doc` trait
	* Check existence of first segment of index key paths (?)
* Default `Doc::Id` to `ObjectId` and `Query::Output` to `T`, once [#29661](https://github.com/rust-lang/rust/issues/29661) is stabilized
* Make `Error` more structured, e.g. introduce an `ErrorKind` to match on, and a method for transitively retrieving it (throughout the cause chain)
