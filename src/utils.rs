//! Common utility functions and types.

use error::{ Error, Result };

/// Converts an `i8`, `i16`, `i32` or `i64` to a `usize` if the range and
/// the value permits. Constructs an error message based on `msg` otherwise.
#[cfg_attr(feature = "cargo-clippy", allow(cast_possible_wrap, cast_possible_truncation, if_same_then_else))]
pub fn int_to_usize_with_msg<T: Into<i64>>(x: T, msg: &str) -> Result<usize> {
    use std::usize;
    use std::mem::size_of;

    let n: i64 = x.into();

    // XXX: the correctness of this usize -> i64 cast relies on the following:
    // 1. if `sizeof(usize) >= sizeof(i64)`, i.e. 64-bit and wider word size
    //    platforms (the typical), then `i64::MAX` always fits into a `usize`,
    //    therefore the cast `n as usize` is safe as long as `n >= 0`.
    // 2. Otherwise, if `sizeof(usize) < sizeof(i64)`, eg. 32-bit architectures,
    //    then we can safely cast `usize::MAX` to `i64` in order to find out
    //    via comparison whether the actual `i64` value fits dynamically.
    if n < 0 {
        Err(Error::new(format!("{} ({}) is negative", msg, n)))
    } else if size_of::<usize>() >= size_of::<i64>() {
        Ok(n as usize)
    } else if n <= usize::MAX as i64 {
        Ok(n as usize)
    } else {
        Err(Error::new(format!("{} ({}) overflows `usize`", msg, n)))
    }
}