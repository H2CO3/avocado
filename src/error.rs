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
#[cfg_attr(feature = "cargo-clippy", allow(stutter))]
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
    fn chain<S>(self, message: S) -> Result<T> where S: Into<Cow<'static, str>>;
}

/// Type alias for a `Result` containing an Avocado `Error`.
pub type Result<T> = result::Result<T, Error>;

impl<T, E> ResultExt<T> for result::Result<T, E> where E: ErrorExt + 'static {
    fn chain<S>(self, message: S) -> Result<T> where S: Into<Cow<'static, str>> {
        self.map_err(|cause| {
            let message = message.into();
            let backtrace = if cause.backtrace().is_none() {
                Some(Backtrace::new())
            } else {
                None
            };
            let cause: Option<Box<ErrorExt>> = Some(Box::new(cause));
            Error { message, cause, backtrace }
        })
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

    #[cfg_attr(feature = "cargo-clippy", allow(or_fun_call))]
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

impl From<mongodb::Error> for Error {
    fn from(error: mongodb::Error) -> Self {
        Self::with_cause("MongoDB error", error)
    }
}

impl ErrorExt for mongodb::Error {
    fn as_std_error(&self) -> &error::Error {
        self
    }
}

impl From<bson::EncoderError> for Error {
    fn from(error: bson::EncoderError) -> Self {
        Self::with_cause("BSON encoding error", error)
    }
}

impl ErrorExt for bson::EncoderError {
    fn as_std_error(&self) -> &error::Error {
        self
    }
}

impl From<bson::DecoderError> for Error {
    fn from(error: bson::DecoderError) -> Self {
        Self::with_cause("BSON decoding error", error)
    }
}

impl ErrorExt for bson::DecoderError {
    fn as_std_error(&self) -> &error::Error {
        self
    }
}

impl From<mongodb::coll::error::WriteException> for Error {
    fn from(error: mongodb::coll::error::WriteException) -> Self {
        Self::with_cause("MongoDB write exception", error)
    }
}

impl ErrorExt for mongodb::coll::error::WriteException {
    fn as_std_error(&self) -> &error::Error {
        self
    }
}

impl From<mongodb::coll::error::BulkWriteException> for Error {
    fn from(error: mongodb::coll::error::BulkWriteException) -> Self {
        Self::with_cause("MongoDB bulk write exception", error)
    }
}

impl ErrorExt for mongodb::coll::error::BulkWriteException {
    fn as_std_error(&self) -> &error::Error {
        self
    }
}
