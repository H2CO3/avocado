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

* `cargo doc --no-deps --open`
* Check out the [`examples/`](https://github.com/H2CO3/avocado/blob/master/examples/) folder
* More docs are coming!

## TODO:

* Fix integer overflow TODO in `bsn.rs`
* Write documentation in `lib.rs` doc comments
* Write integration tests that exercise the library using an actual, running MongoDB database
* Default `Doc::Id` to `ObjectId` and `Query::Output` to `T`, once [#29661](https://github.com/rust-lang/rust/issues/29661) is stabilized
* Auto-derive `Doc` trait; respect Serde renaming when obtaining type name!
