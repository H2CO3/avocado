//! `Error` and `Result` types arising out of MongoDB operations.

use std::fmt;
use std::error;
use std::result;
use std::ops::Deref;
use std::borrow::Cow;
use backtrace::Backtrace;
use bson;
use mongodb;

/// Slightly augmented trait for backtrace-able errors.
#[allow(clippy::stutter)]
pub trait ErrorExt: error::Error {
    /// Similar to `std::error::Error::cause()`, but with richer type info.
    fn reason(&self) -> Option<&ErrorExt> {
        None
    }

    /// Returns the deepest possible backtrace, if any.
    fn backtrace(&self) -> Option<&Backtrace> {
        None
    }

    /// Until subtrait coercions are implemented, this helper method
    /// should return the receiver as an `&std::error::Error` trait object.
    fn as_std_error(&self) -> &error::Error;
}

/// A trait for conveniently propagating errors up the call stack.
pub trait ResultExt<T>: Sized {
    /// If this `Result` is an `Err`, then prepend the specified error
    /// to the front of the linked list of causes.
    fn chain<M: ErrMsg>(self, message: M) -> Result<T>;
}

/// Values that can act as or generate an error message.
pub trait ErrMsg: Sized {
    /// Convert the value to an error message.
    fn into_message(self) -> Cow<'static, str>;
}

/// Type alias for a `Result` containing an Avocado `Error`.
pub type Result<T> = result::Result<T, Error>;

impl<T, E> ResultExt<T> for result::Result<T, E> where E: ErrorExt + 'static {
    fn chain<M: ErrMsg>(self, message: M) -> Result<T> {
        self.map_err(|cause| Error::with_cause(message.into_message(), cause))
    }
}

/// Blanket `impl ErrMsg` for string literals.
impl ErrMsg for &'static str {
    fn into_message(self) -> Cow<'static, str> {
        Cow::Borrowed(self)
    }
}

/// Blanket `impl ErrMsg` for error message formatting functions.
impl<F> ErrMsg for F where F: FnOnce() -> String {
    fn into_message(self) -> Cow<'static, str> {
        Cow::Owned(self())
    }
}

/// The central error type for Avocado.
#[derive(Debug)]
pub struct Error {
    /// The human-readable description.
    message: Cow<'static, str>,
    /// The underlying error, if any.
    cause: Option<Box<ErrorExt>>,
    /// The backtrace, if any.
    backtrace: Option<Backtrace>,
}

impl Error {
    /// Creates an error with the specified message, no cause, and a backtrace.
    pub fn new<S>(message: S) -> Self where S: Into<Cow<'static, str>> {
        Error {
            message: message.into(),
            cause: None,
            backtrace: Some(Backtrace::new()),
        }
    }

    /// Creates an error with the specified message and cause. If the cause has
    /// no backtrace, this method will create it and add it to the new instance.
    pub fn with_cause<S, E>(message: S, cause: E) -> Self
        where S: Into<Cow<'static, str>>,
              E: ErrorExt + 'static
    {
        let message = message.into();
        let backtrace = if cause.backtrace().is_none() {
            Some(Backtrace::new())
        } else {
            None
        };
        let cause: Option<Box<ErrorExt>> = Some(Box::new(cause));

        Error { message, cause, backtrace }
    }
}

impl ErrorExt for Error {
    fn reason(&self) -> Option<&ErrorExt> {
        self.cause.as_ref().map(Deref::deref)
    }

    #[allow(clippy::or_fun_call)]
    fn backtrace(&self) -> Option<&Backtrace> {
        self.reason().and_then(ErrorExt::backtrace).or(self.backtrace.as_ref())
    }

    fn as_std_error(&self) -> &error::Error {
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.message)?;

        if let Some(cause) = self.cause.as_ref() {
            write!(f, ", caused by: {}", cause)?
        }

        if let Some(backtrace) = self.backtrace.as_ref() {
            write!(f, "; {:#?}", backtrace)?
        }

        Ok(())
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        &self.message
    }

    fn cause(&self) -> Option<&error::Error> {
        self.reason().map(ErrorExt::as_std_error)
    }
}

/// Implementing `ErrorExt` and `From` boilerplate.
macro_rules! impl_error_type {
    ($ty:path, $message:expr) => {
        impl From<$ty> for Error {
            fn from(error: $ty) -> Self {
                Self::with_cause($message, error)
            }
        }

        impl ErrorExt for $ty {
            fn as_std_error(&self) -> &error::Error {
                self
            }
        }
    }
}

impl_error_type! { serde_json::Error,      "JSON transcoding error" }
impl_error_type! { bson::EncoderError,     "BSON encoding error" }
impl_error_type! { bson::DecoderError,     "BSON decoding error" }
impl_error_type! { bson::ValueAccessError, "missing or ill-typed BSON value" }

impl_error_type! { mongodb::Error,                           "MongoDB error" }
impl_error_type! { mongodb::coll::error::WriteException,     "MongoDB write exception" }
impl_error_type! { mongodb::coll::error::BulkWriteException, "MongoDB bulk write exception" }
