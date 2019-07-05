//! `Error` and `Result` types arising out of MongoDB operations.

use std::fmt;
use std::error;
use std::result;
use std::ops::Deref;
use std::borrow::Cow;
use bson::ValueAccessError;
use backtrace::Backtrace;
use typemap::{ DebugMap, Key };

/// Slightly augmented trait for backtrace-able errors.
#[allow(clippy::stutter)]
pub trait ErrorExt: error::Error {
    /// Similar to `std::error::Error::source()`, but with richer type info.
    fn reason(&self) -> Option<&(dyn ErrorExt + 'static)> {
        None
    }

    /// Returns the deepest possible backtrace, if any.
    fn backtrace(&self) -> Option<&Backtrace> {
        self.reason().and_then(ErrorExt::backtrace)
    }

    /// Structured error kind.
    fn kind(&self) -> ErrorKind;

    /// Until subtrait coercions are implemented, this helper method
    /// should return the receiver as an `&std::error::Error` trait object.
    fn as_std_error(&self) -> &(dyn error::Error + 'static);
}

/// A trait for conveniently propagating errors up the call stack.
pub trait ResultExt<T>: Sized {
    /// If this `Result` is an `Err`, then prepend the specified error
    /// to the front of the linked list of causes.
    /// ```
    /// # extern crate avocado;
    /// #
    /// # use std::error::Error as StdError;
    /// # use avocado::error::{ Error, ErrorKind, ErrorExt, Result, ResultExt };
    /// #
    /// # fn main() -> Result<()> {
    /// #
    /// let ok: Result<_> = Ok("success!");
    /// let ok_chained = ok.chain("dummy error message")?;
    /// assert_eq!(ok_chained, "success!");
    ///
    /// let err: Result<i32> = Err(Error::new(
    ///     ErrorKind::MongoDbError, "chained cause"
    /// ));
    /// let err_chained = err.chain("top-level message").unwrap_err();
    /// assert_eq!(err_chained.description(), "top-level message");
    /// assert_eq!(err_chained.kind(), ErrorKind::MongoDbError);
    /// #
    /// # Ok(())
    /// # }
    /// ```
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

/// A structured, "machine-readable" error kind.
#[allow(clippy::stutter)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorKind {
    /// There was an error converting between JSON and a strongly-typed value.
    JsonTranscoding,
    /// There was an error converting a strongly-typed value to BSON.
    BsonEncoding,
    /// There was an error converting BSON to a strongly-typed value.
    BsonDecoding,
    /// This numerical value can't be represented in BSON (because,
    /// for example, it exceeds the range of `i64`)
    BsonNumberRepr,
    /// A field with the specified key was not found in the BSON document.
    MissingDocumentField,
    /// A field with the specified key was found in the BSON document,
    /// but it was of an unexpected type.
    IllTypedDocumentField,
    /// One or more ID fields (e.g. `_id` in an entity document or
    /// `inserted_id` in a MongoDB response) could not be found.
    MissingId,
    /// An `ObjectId` could not be generated.
    ObjectIdGeneration,
    /// An error that comes from the MongoDB driver.
    MongoDbError,
    /// An error coming from MongoDB, related to a single write operation.
    MongoDbWriteException,
    /// An error coming from MongoDB, related to a bulk write operation.
    MongoDbBulkWriteException,
    /// An attempt was made to convert a negative integer to a `usize`.
    IntConversionUnderflow,
    /// An attempt was made to convert an integer that is too big to a `usize`.
    IntConversionOverflow,
    /// There was an error in the BSON schema for a type.
    BsonSchema,
}

impl ErrorKind {
    /// Returns a human-readable error description for this kind.
    pub fn as_str(self) -> &'static str {
        use self::ErrorKind::*;

        match self {
            JsonTranscoding           => "JSON transcoding error",
            BsonEncoding              => "BSON encoding error",
            BsonDecoding              => "BSON decoding error",
            BsonNumberRepr            => "number not i64 nor f64",
            MissingDocumentField      => "document field not found",
            IllTypedDocumentField     => "document field of unexpected type",
            MissingId                 => "missing unique identifier",
            ObjectIdGeneration        => "an ObjectID could not be generated",
            MongoDbError              => "MongoDB error",
            MongoDbWriteException     => "MongoDB write exception",
            MongoDbBulkWriteException => "MongoDB bulk write exception",
            IntConversionUnderflow    => "integer conversion underflowed",
            IntConversionOverflow     => "integer conversion overflowed",
            BsonSchema                => "error in BSON schema",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// The central error type for Avocado.
#[derive(Debug)]
pub struct Error {
    /// The structured, "machine-readable" kind of this error.
    kind: ErrorKind,
    /// The human-readable description.
    message: Cow<'static, str>,
    /// The underlying error, if any.
    cause: Option<Box<dyn ErrorExt>>,
    /// The backtrace, if any.
    backtrace: Option<Backtrace>,
    /// Additional context info, if any.
    context: DebugMap,
}

impl Error {
    /// Creates an error with the specified kind, message, no cause,
    /// and a backtrace.
    /// ```
    /// # extern crate avocado;
    /// #
    /// # use std::error::Error as StdError;
    /// # use avocado::error::{ Error, ErrorKind, ErrorExt };
    /// #
    /// # fn main() {
    /// #
    /// let error = Error::new(ErrorKind::MissingId, "sample error message");
    /// assert_eq!(error.description(), "sample error message");
    /// assert_eq!(error.kind(), ErrorKind::MissingId);
    /// assert!(error.reason().is_none());
    /// assert!(error.backtrace().is_some());
    /// #
    /// # }
    /// ```
    pub fn new<S>(kind: ErrorKind, message: S) -> Self
        where S: Into<Cow<'static, str>>
    {
        Error {
            kind,
            message: message.into(),
            cause: None,
            backtrace: Some(Backtrace::new()),
            context: DebugMap::custom(),
        }
    }

    /// Creates an error with the specified message and cause. If the cause has
    /// no backtrace, this method will create it and add it to the new instance.
    /// ```
    /// # extern crate avocado;
    /// # extern crate bson;
    /// #
    /// # use std::error::Error as StdError;
    /// # use avocado::error::{ Error, ErrorExt };
    /// #
    /// # fn main() {
    /// #
    /// use bson::oid;
    ///
    /// let cause = oid::Error::HostnameError;
    /// assert!(cause.cause().is_none());
    /// assert!(cause.backtrace().is_none());
    ///
    /// let error = Error::with_cause("top-level message", cause);
    /// assert_eq!(error.description(), "top-level message");
    /// assert_eq!(error.cause().unwrap().description(),
    ///            "Failed to retrieve hostname for OID generation.");
    /// assert!(error.backtrace().is_some());
    /// #
    /// # }
    /// ```
    pub fn with_cause<S, E>(message: S, cause: E) -> Self
        where S: Into<Cow<'static, str>>,
              E: ErrorExt + 'static
    {
        let kind = cause.kind();
        let message = message.into();
        let backtrace = if cause.backtrace().is_none() {
            Some(Backtrace::new())
        } else {
            None
        };
        let cause: Option<Box<dyn ErrorExt>> = Some(Box::new(cause));
        let context = DebugMap::custom();

        Error { kind, message, cause, backtrace, context }
    }

    /// Returns additional context info if any.
    pub fn context<K: Key>(&self) -> Option<&K::Value>
        where K::Value: fmt::Debug
    {
        self.context.get::<K>()
    }

    /// Augments the error with additional context info.
    pub fn set_context<K: Key>(&mut self, value: K::Value) -> Option<K::Value>
        where K::Value: fmt::Debug
    {
        self.context.insert::<K>(value)
    }

    /// Builder-style setter for agumenting the error with context info.
    pub fn with_context<K: Key>(mut self, value: K::Value) -> Self
        where K::Value: fmt::Debug
    {
        self.set_context::<K>(value);
        self
    }
}

impl ErrorExt for Error {
    fn reason(&self) -> Option<&(dyn ErrorExt + 'static)> {
        self.cause.as_ref().map(Deref::deref)
    }

    #[allow(clippy::or_fun_call)]
    fn backtrace(&self) -> Option<&Backtrace> {
        self.reason().and_then(ErrorExt::backtrace).or(self.backtrace.as_ref())
    }

    fn kind(&self) -> ErrorKind {
        self.kind
    }

    fn as_std_error(&self) -> &(dyn error::Error + 'static) {
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)?;

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

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.reason().map(ErrorExt::as_std_error)
    }
}

impl From<ValueAccessError> for Error {
    fn from(error: ValueAccessError) -> Self {
        let message = match error {
            ValueAccessError::NotPresent => "missing value for key in Document",
            ValueAccessError::UnexpectedType => "ill-typed value for key in Document",
        };
        Self::with_cause(message, error)
    }
}

impl ErrorExt for ValueAccessError {
    fn kind(&self) -> ErrorKind {
        match *self {
            ValueAccessError::NotPresent => ErrorKind::MissingDocumentField,
            ValueAccessError::UnexpectedType => ErrorKind::IllTypedDocumentField,
        }
    }

    fn as_std_error(&self) -> &(dyn error::Error + 'static) {
        self
    }
}

/// Implementing `ErrorExt` and `From` boilerplate.
macro_rules! impl_error_type {
    ($ty:path, $kind:ident, $message:expr) => {
        impl From<$ty> for Error {
            fn from(error: $ty) -> Self {
                Self::with_cause($message, error)
            }
        }

        impl ErrorExt for $ty {
            fn kind(&self) -> ErrorKind {
                ErrorKind::$kind
            }

            fn as_std_error(&self) -> &(dyn error::Error + 'static) {
                self
            }
        }
    }
}

impl_error_type! { serde_json::Error,  JsonTranscoding,    "JSON transcoding error" }
impl_error_type! { bson::EncoderError, BsonEncoding,       "BSON encoding error" }
impl_error_type! { bson::DecoderError, BsonDecoding,       "BSON decoding error" }
impl_error_type! { bson::oid::Error,   ObjectIdGeneration, "ObjectId generation error" }
impl_error_type! { mongodb::Error,     MongoDbError,       "MongoDB error" }
impl_error_type! {
    mongodb::coll::error::WriteException,
    MongoDbWriteException,
    "MongoDB write exception"
}
impl_error_type! {
    mongodb::coll::error::BulkWriteException,
    MongoDbBulkWriteException,
    "MongoDB bulk write exception"
}
