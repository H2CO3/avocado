//! Errors potentially happening while `#[derive]`ing `Doc`.

use std::fmt;
use std::error;
use std::result;
use std::ops::Deref;
use std::num::{ ParseIntError, ParseFloatError };
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use syn::synom::ParseError;

/// Returns an `Err(Error::new(...))` with the given formatted error message.
macro_rules! err_fmt {
    ($($arg:tt)*) => { Err(crate::error::Error::new(format!($($arg)*))) }
}

/// Returns an `Err(Error::new(...))` with the given literal error message.
pub fn err_msg<T>(message: &str) -> Result<T> {
    Err(Error::new(message))
}

/// Convenience type alias for a result that holds a `avocado_derive::Error` value.
pub type Result<T> = result::Result<T, Error>;

/// An error that potentially happens while `#[derive]`ing `Doc`.
#[derive(Debug)]
pub struct Error {
    /// The error message.
    message: String,
    /// The underlying error, if any.
    cause: Option<Box<dyn error::Error>>,
}

impl Error {
    /// Creates an `Error` instance with the specified message.
    pub fn new<T: Into<String>>(message: T) -> Self {
        Error {
            message: message.into(),
            cause: None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.cause {
            Some(ref cause) => write!(f, "{}: {}", self.message, cause),
            None => self.message.fmt(f),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        &self.message
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        self.cause.as_ref().map(Deref::deref)
    }
}

/// A macro for implementing error conversion boilerplate.
macro_rules! impl_error {
    ($($ty:ident => $message:expr;)*) => {$(
        impl From<$ty> for Error {
            fn from(error: $ty) -> Self {
                Error {
                    message: String::from($message),
                    cause: Some(Box::new(error)),
                }
            }
        }
    )*}
}

impl_error! {
    ParseError      => "could not parse derive input";
    Utf8Error       => "byte string is not valid UTF-8";
    FromUtf8Error   => "byte string is not valid UTF-8";
    ParseIntError   => "string does not represent an integer";
    ParseFloatError => "string does not represent a floating-point number";
}
