//! Common utility functions and types.

use crate::error::{ Error, Result };

/// Converts an `i8`, `i16`, `i32` or `i64` to a `usize` if the range and
/// the value permits. Constructs an error message based on `msg` otherwise.
/// ```
/// # extern crate avocado;
/// #
/// # use std::{ u32, i64 };
/// # use avocado::utils::int_to_usize_with_msg;
/// # use avocado::error::Result;
/// #
/// # fn main() -> Result<()> {
/// #
/// assert!(int_to_usize_with_msg(-1 as i32, "example value")
///         .unwrap_err()
///         .to_string()
///         .contains("example value (-1) is negative"));
///
/// assert_eq!(
///     int_to_usize_with_msg(1, "example value")?,
///     1
/// );
///
/// let platform_dependent = int_to_usize_with_msg(i64::MAX, "example value");
/// if cfg!(target_pointer_width =  "8") ||
///    cfg!(target_pointer_width = "16") ||
///    cfg!(target_pointer_width = "32") {
///     assert!(platform_dependent
///             .unwrap_err()
///             .to_string()
///             .contains("overflows usize"));
/// } else if cfg!(target_pointer_width =  "64") ||
///           cfg!(target_pointer_width = "128") {
///     assert_eq!(platform_dependent?, i64::MAX as usize);
/// } else {
///     panic!("exotic pointer width, can't assume correct result");
/// }
/// #
/// # Ok(())
/// # }
/// ```
#[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation, clippy::if_same_then_else)]
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
