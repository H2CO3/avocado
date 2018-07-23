//! `Error` and `Result` types arising out of MongoDB operations.
/// TODO(H2CO3): add an `enum ErrorKind` type for structured information?

use std::fmt;
use std::error;
use std::result;
use std::ops::Deref;
use std::borrow::Cow;
use backtrace::Backtrace;
use mongodb;

/// A trait for conveniently propagating errors up the call stack.
pub trait ResultExt<T>: Sized {
    /// If this `Result` is an `Err`, then prepend the specified error
    /// to the front of the list of causes.
    /// TODO(H2CO3): add `kind: ErrorKind` argument for structured information?
    fn link<S: Into<Cow<'static, str>>>(self, message: S) -> Result<T>;
}

/// Type alias for a `Result` containing an Avocado `Error`.
pub type Result<T> = result::Result<T, Error>;

impl<T, E: Into<Error>> ResultExt<T> for result::Result<T, E> {
    fn link<S: Into<Cow<'static, str>>>(self, message: S) -> Result<T> {
        self.map_err(|error| {
            let cause = error.into();
            let message = message.into();
            let backtrace = if cause.backtrace.is_none() {
                Some(Backtrace::new())
            } else {
                None
            };
            let cause = Some(Box::new(cause));
            Error { message, cause, backtrace }
        })
    }
}

/// The central error type for Avocado.
/// TODO(H2CO3): add a `kind: ErrorKind` field for structured information?
#[derive(Debug, Clone)]
pub struct Error {
    /// The human-readable description.
    message: Cow<'static, str>,
    /// The underlying error, if any.
    cause: Option<Box<Error>>,
    /// The backtrace, if any.
    backtrace: Option<Backtrace>,
}

/// TODO(H2CO3): add `fn kind(&self) -> ErrorKind` for structured information?
impl Error {
    /// Creates a new error with the specified message.
    /// TODO(H2CO3): add a `kind: ErrorKind` argument for structured information?
    pub fn new<S: Into<Cow<'static, str>>>(message: S) -> Self {
        Error {
            message: message.into(),
            cause: None,
            backtrace: Some(Backtrace::new()),
        }
    }

    /// Same purpose as of `std::error::Error::cause()`,
    /// but this one doesn't lose type information.
    pub fn reason(&self) -> Option<&Error> {
        self.cause.as_ref().map(Deref::deref)
    }

    /// Returns the deepest possible backtrace, if any.
    pub fn backtrace(&self) -> Option<&Backtrace> {
        self.reason().and_then(Self::backtrace).or(self.backtrace.as_ref())
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
        self.cause.as_ref().map(|c| { let c: &error::Error = &**c; c })
    }
}

impl From<mongodb::Error> for Error {
    /// TODO(H2CO3): add `kind: ErrorKind::MongoDB(error)`
    fn from(error: mongodb::Error) -> Self {
        Self::new(format!("MongoDB error: {}", error))
    }
}
