//! Type-safe document specific to DSL sub-operations (e.g. filter, update, etc.)

use std::fmt;
use std::borrow::Cow;
use linked_hash_map::{ self, LinkedHashMap };
use std::iter::{ FromIterator, DoubleEndedIterator, ExactSizeIterator };

/// A top-level DSL document consisting of multiple path => sub-operation
/// specifiers and respecting the order of insertion during iteration.
#[cfg_attr(feature = "cargo-clippy", allow(stutter))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Document<T>(LinkedHashMap<Cow<'static, str>, T>);

impl<T> Document<T> {
    /// Creates an empty document.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty document with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Document(LinkedHashMap::with_capacity(capacity))
    }

    /// Returns the current capacity of the document.
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Returns the number of entries (key-value pairs) in the document.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if and only if the document contains no entries.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Reserves additional capacity for the document.
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional)
    }

    /// Inserts a key and a value into the document. If the key already
    /// exists, returns the previous value associated with it.
    pub fn insert(&mut self, key: Cow<'static, str>, value: T) -> Option<T> {
        self.0.insert(key, value)
    }

    /// Returns `true` if and only if the document contains the specified key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    /// Returns a reference to the subquery associated with the key.
    pub fn get(&self, key: &str) -> Option<&T> {
        self.0.get(key)
    }

    /// Returns a mutable reference to the subquery associated with the key.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut T> {
        self.0.get_mut(key)
    }

    /// Removes the subquery associated with the key and returns it.
    pub fn remove(&mut self, key: &str) -> Option<T> {
        self.0.remove(key)
    }

    /// Removes all key-value pairs, leaving the document in an empty state.
    pub fn clear(&mut self) {
        self.0.clear()
    }
}

impl<T> Default for Document<T> {
    fn default() -> Self {
        Document(LinkedHashMap::new())
    }
}

impl<T, K, V> FromIterator<(K, V)> for Document<T>
    where K: Into<Cow<'static, str>>,
          V: Into<T>
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Document(iter.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
    }
}

impl<T, K, V> Extend<(K, V)> for Document<T>
    where K: Into<Cow<'static, str>>,
          V: Into<T>
{
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        self.0.extend(iter.into_iter().map(|(k, v)| (k.into(), v.into())))
    }
}

impl<T> IntoIterator for Document<T> {
    type Item = (Cow<'static, str>, T);
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter())
    }
}

impl<'a, T> IntoIterator for &'a Document<T> {
    type Item = (&'a str, &'a T);
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        Iter(self.0.iter())
    }
}

impl<'a, T> IntoIterator for &'a mut Document<T> {
    type Item = (&'a str, &'a mut T);
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        IterMut(self.0.iter_mut())
    }
}

/// An owning iterator over the entries of a `dsl::Document`.
/// Yields entries in order of insertion.
#[derive(Clone)]
pub struct IntoIter<T>(linked_hash_map::IntoIter<Cow<'static, str>, T>);

impl<T> Iterator for IntoIter<T> {
    type Item = (Cow<'static, str>, T);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<T> fmt::Debug for IntoIter<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "dsl::doc::Document::IntoIter({} entries)", self.len())
    }
}

/// A borrowing iterator over the entries of a `dsl::Document`.
/// Yields entries in order of insertion.
#[derive(Clone)]
pub struct Iter<'a, T: 'a>(linked_hash_map::Iter<'a, Cow<'static, str>, T>);

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (&'a str, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(k, v)| (k.as_ref(), v))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(|(k, v)| (k.as_ref(), v))
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a, T> fmt::Debug for Iter<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "dsl::doc::Document::Iter({} entries)", self.len())
    }
}

/// A mutably borrowing iterator over the entries of a `dsl::Document`.
/// Yields entries in order of insertion.
pub struct IterMut<'a, T: 'a>(linked_hash_map::IterMut<'a, Cow<'static, str>, T>);

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (&'a str, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(k, v)| (k.as_ref(), v))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<'a, T> DoubleEndedIterator for IterMut<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(|(k, v)| (k.as_ref(), v))
    }
}

impl<'a, T> ExactSizeIterator for IterMut<'a, T> {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a, T> fmt::Debug for IterMut<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "dsl::doc::Document::IterMut({} entries)", self.len())
    }
}

/// Helper for creating `dsl::Document`s.
#[macro_export]
#[doc(hidden)]
macro_rules! __avocado_dsl_doc {
    ($($path:tt: $value:expr),*) => ({
        let mut doc = $crate::dsl::doc::Document::with_capacity(
            __avocado_dsl_doc_count_elements!($($path),*)
        );
        $(
            doc.insert($path.into(), $value.into());
        )*
        doc
    });
    ($($path:tt: $value:expr,)*) => {
        __avocado_dsl_doc!{ $($path: $value),* }
    }
}

/// Helper for `__avocado_dsl_doc!` that counts the number of elements in a sequence.
#[macro_export]
#[doc(hidden)]
macro_rules! __avocado_dsl_doc_count_elements {
    () => (0);
    ($first:tt $(, $rest:tt)*) => {
        1 + __avocado_dsl_doc_count_elements!($($rest),*)
    }
}
