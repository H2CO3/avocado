//! Convenience extension traits and methods.

use bson::{ Bson, Document, ordered::ValueAccessError };
use crate::error::{ Error, Result };

/// Convenience methods for implementing `transform()` methods in various
/// traits in the [`ops`](ops/index.html) module.
#[allow(clippy::stutter)]
pub trait DocumentExt {
    /// Remove the value corresponding to the given key. Return an error if
    /// no such key-value pair is present in the document.
    fn try_remove(&mut self, key: &str) -> Result<Bson>;

    /// Remove the value corresponding to the given key if it is a `bool`.
    /// Return an error if the key is missing or the value is not a `bool`.
    fn remove_bool(&mut self, key: &str) -> Result<Bson>;

    /// Remove the value corresponding to the given key if it is an `i32`.
    /// Return an error if the key is missing or the value is not an `i32`.
    fn remove_i32(&mut self, key: &str) -> Result<Bson>;

    /// Remove the value corresponding to the given key if it is an `i64`.
    /// Return an error if the key is missing or the value is not an `i64`.
    fn remove_i64(&mut self, key: &str) -> Result<Bson>;

    /// Remove the value corresponding to the given key if it is an `f64`.
    /// Return an error if the key is missing or the value is not an `f64`.
    fn remove_f64(&mut self, key: &str) -> Result<Bson>;

    /// Remove the value corresponding to the given key if it is any number.
    /// Return an error if the key is missing or the value is not numeric.
    fn remove_number(&mut self, key: &str) -> Result<Bson>;

    /// Remove the value corresponding to the given key if it is a string.
    /// Return an error if the key is missing or the value is not a string.
    fn remove_str(&mut self, key: &str) -> Result<Bson>;

    /// Remove the value corresponding to the given key if it is an `Array`.
    /// Return an error if the key is missing or the value is not an `Array`.
    fn remove_array(&mut self, key: &str) -> Result<Bson>;

    /// Remove the value corresponding to the given key if it is a `Document`.
    /// Return an error if the key is missing or the value is not a `Document`.
    fn remove_document(&mut self, key: &str) -> Result<Bson>;

    /// Remove the value corresponding to the given key if it is an `ObjectId`.
    /// Return an error if the key is missing or the value is not an `ObjectId`.
    fn remove_object_id(&mut self, key: &str) -> Result<Bson>;

    /// Remove the value corresponding to the given key if it is a `DateTime`.
    /// Return an error if the key is missing or the value is not a `DateTime`.
    fn remove_datetime(&mut self, key: &str) -> Result<Bson>;

    /// Remove the value corresponding to the given key if it is a `TimeStamp`.
    /// Return an error if the key is missing or the value is not a `TimeStamp`.
    fn remove_timestamp(&mut self, key: &str) -> Result<Bson>;

    /// Remove the value corresponding to the given key if it is a `Binary` of
    /// the `Generic` subtype. Return an error if the key is missing or the
    /// value is not a `Binary` of the `Generic` subtype.
    fn remove_generic_binary(&mut self, key: &str) -> Result<Bson>;
}

impl DocumentExt for Document {
    fn try_remove(&mut self, key: &str) -> Result<Bson> {
        self.remove(key).ok_or_else(|| Error::with_cause(
            format!("key `{}` was not found in the document", key),
            ValueAccessError::NotPresent
        ))
    }

    fn remove_bool(&mut self, key: &str) -> Result<Bson> {
        match self.get_bool(key) {
            Ok(_) => self.try_remove(key),
            Err(cause) => removal_error(key, "bool", cause),
        }
    }

    fn remove_i32(&mut self, key: &str) -> Result<Bson> {
        match self.get_i32(key) {
            Ok(_) => self.try_remove(key),
            Err(cause) => removal_error(key, "i32", cause),
        }
    }

    fn remove_i64(&mut self, key: &str) -> Result<Bson> {
        match self.get_i64(key) {
            Ok(_) => self.try_remove(key),
            Err(cause) => removal_error(key, "i64", cause),
        }
    }

    fn remove_f64(&mut self, key: &str) -> Result<Bson> {
        match self.get_f64(key) {
            Ok(_) => self.try_remove(key),
            Err(cause) => removal_error(key, "f64", cause),
        }
    }

    fn remove_number(&mut self, key: &str) -> Result<Bson> {
        if let Ok(x) = self.remove_i32(key) {
            return Ok(x);
        }
        if let Ok(x) = self.remove_i64(key) {
            return Ok(x);
        }
        if let Ok(x) = self.remove_f64(key) {
            return Ok(x);
        }

        let cause = if self.contains_key(key) {
            ValueAccessError::UnexpectedType
        } else {
            ValueAccessError::NotPresent
        };
        removal_error(key, "numeric", cause)
    }

    fn remove_str(&mut self, key: &str) -> Result<Bson> {
        match self.get_str(key) {
            Ok(_) => self.try_remove(key),
            Err(cause) => removal_error(key, "string", cause),
        }
    }

    fn remove_array(&mut self, key: &str) -> Result<Bson> {
        match self.get_array(key) {
            Ok(_) => self.try_remove(key),
            Err(cause) => removal_error(key, "array", cause),
        }
    }

    fn remove_document(&mut self, key: &str) -> Result<Bson> {
        match self.get_document(key) {
            Ok(_) => self.try_remove(key),
            Err(cause) => removal_error(key, "document", cause),
        }
    }

    fn remove_object_id(&mut self, key: &str) -> Result<Bson> {
        match self.get_object_id(key) {
            Ok(_) => self.try_remove(key),
            Err(cause) => removal_error(key, "ObjectID", cause),
        }
    }

    fn remove_datetime(&mut self, key: &str) -> Result<Bson> {
        match self.get_utc_datetime(key) {
            Ok(_) => self.try_remove(key),
            Err(cause) => removal_error(key, "DateTime", cause),
        }
    }

    fn remove_timestamp(&mut self, key: &str) -> Result<Bson> {
        match self.get_time_stamp(key) {
            Ok(_) => self.try_remove(key),
            Err(cause) => removal_error(key, "timestamp", cause),
        }
    }

    fn remove_generic_binary(&mut self, key: &str) -> Result<Bson> {
        match self.get_binary_generic(key) {
            Ok(_) => self.try_remove(key),
            Err(cause) => removal_error(key, "generic binary", cause),
        }
    }
}

/// Constructs an error for a missing or ill-typed key-value pair in a Document.
fn removal_error(key: &str, ty: &str, cause: ValueAccessError) -> Result<Bson> {
    Err(Error::with_cause(
        format!("error removing {} value for key `{}`", key, ty),
        cause
    ))
}

#[cfg(test)]
mod tests {
    use bson::{ Bson, oid::ObjectId };
    use super::DocumentExt;
    use crate::error::{ ErrorExt, ErrorKind, Result };

    #[test]
    fn document_ext_works() -> Result<()> {
        let mut d = doc!{
            "string_value": "whatever",
            "i32_value": 42_i32,
            "i64_value": 1337_i64,
            "f64_value": 3.1415926536,
            "bool_value": true,
            "null_value": null,
            "oid_value": ObjectId::new()?,
            "document_value": {
                "foo": "bar",
                "qux": [0],
            },
            "array_value": [-0.00729735257, "stuff", [], { "key": "value" }],
            "number_value": 2.718281829,
        };

        assert_eq!(d.remove_i64("i32_value").unwrap_err().kind(),
                   ErrorKind::IllTypedDocumentField);
        assert_eq!(d.remove_i32("bool_value").unwrap_err().kind(),
                   ErrorKind::IllTypedDocumentField);
        assert_eq!(d.remove_array("document_value").unwrap_err().kind(),
                   ErrorKind::IllTypedDocumentField);
        assert_eq!(d.remove_document("oid_value").unwrap_err().kind(),
                   ErrorKind::IllTypedDocumentField);

        assert_eq!(d.try_remove("null_value").unwrap(), Bson::Null);
        assert_eq!(d.try_remove("null_value").unwrap_err().kind(),
                   ErrorKind::MissingDocumentField);
        assert_eq!(d.try_remove("bogus_value").unwrap_err().kind(),
                   ErrorKind::MissingDocumentField);

        assert_eq!(d.remove_number("number_value").unwrap(),
                   Bson::FloatingPoint(2.718281829));
        assert_eq!(d.remove_i32("i32_value").unwrap(),
                   Bson::I32(42));
        assert_eq!(d.remove_i64("i64_value").unwrap(),
                   Bson::I64(1337));
        assert_eq!(d.remove_array("array_value").unwrap(),
                   bson!([-0.00729735257, "stuff", [], { "key": "value" }]));
        assert_eq!(d.remove_document("document_value").unwrap(),
                   bson!({
                       "foo": "bar",
                       "qux": [0],
                   }));
        assert_eq!(d.remove_document("document_value").unwrap_err().kind(),
                   ErrorKind::MissingDocumentField);
        assert_eq!(d.remove_str("string_value").unwrap(),
                   Bson::from("whatever"));
        assert_eq!(d.remove_bool("bool_value").unwrap(),
                   Bson::Boolean(true));
        assert!(
            d.remove_object_id("oid_value").unwrap().as_object_id().is_some()
        );

        Ok(())
    }
}
