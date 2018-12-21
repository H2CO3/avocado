//! Blanket and other implementations of the `ops` traits for convenience.

use bson::Document;
use doc::Doc;
use ops::*;

impl<T: Doc> Count<T> for Document {
    fn filter(&self) -> Document {
        self.clone()
    }
}

impl<T: Doc> Query<T> for Document {
    type Output = T;

    fn filter(&self) -> Document {
        self.clone()
    }
}

impl<T: Doc> Delete<T> for Document {
    fn filter(&self) -> Document {
        self.clone()
    }
}
