//! Generic type abstractions for DRY principle
//!
//! This module provides reusable generic types that eliminate code duplication
//! across feed types and parsing logic.

use std::fmt::Debug;

/// A field with an optional detailed representation
///
/// Many feed fields follow the pattern of having a simple value (like a string)
/// and an optional detailed struct with additional metadata. This generic wrapper
/// captures that pattern.
///
/// # Type Parameters
///
/// * `V` - The simple value type (usually `String`)
/// * `D` - The detailed representation type (e.g., `TextConstruct`, `Person`)
///
/// # Examples
///
/// ```
/// use feedparser_rs::types::generics::DetailedField;
///
/// // Simple value only
/// let title: DetailedField<String, ()> = DetailedField::from_value("My Title".to_string());
/// assert_eq!(title.value(), "My Title");
/// assert!(title.detail().is_none());
///
/// // With detail
/// let title = DetailedField::with_detail("My Title".to_string(), "extra info");
/// assert_eq!(title.detail(), Some(&"extra info"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct DetailedField<V, D> {
    value: V,
    detail: Option<D>,
}

impl<V, D> DetailedField<V, D> {
    /// Create a field with only a simple value
    #[inline]
    #[must_use]
    pub const fn from_value(value: V) -> Self {
        Self {
            value,
            detail: None,
        }
    }

    /// Create a field with both value and detail
    #[inline]
    #[must_use]
    pub const fn with_detail(value: V, detail: D) -> Self {
        Self {
            value,
            detail: Some(detail),
        }
    }

    /// Get reference to the simple value
    #[inline]
    #[must_use]
    pub const fn value(&self) -> &V {
        &self.value
    }

    /// Get mutable reference to the simple value
    #[inline]
    pub const fn value_mut(&mut self) -> &mut V {
        &mut self.value
    }

    /// Get reference to the detail if present
    #[inline]
    #[must_use]
    pub const fn detail(&self) -> Option<&D> {
        self.detail.as_ref()
    }

    /// Get mutable reference to the detail if present
    #[inline]
    pub const fn detail_mut(&mut self) -> Option<&mut D> {
        self.detail.as_mut()
    }

    /// Set the detail
    #[inline]
    pub fn set_detail(&mut self, detail: D) {
        self.detail = Some(detail);
    }

    /// Take the detail, leaving None in its place
    #[inline]
    pub const fn take_detail(&mut self) -> Option<D> {
        self.detail.take()
    }

    /// Convert into a tuple of (value, `Option<detail>`)
    #[inline]
    #[must_use]
    pub fn into_parts(self) -> (V, Option<D>) {
        (self.value, self.detail)
    }
}

impl<V: Default, D> Default for DetailedField<V, D> {
    fn default() -> Self {
        Self {
            value: V::default(),
            detail: None,
        }
    }
}

impl<V, D> From<V> for DetailedField<V, D> {
    fn from(value: V) -> Self {
        Self::from_value(value)
    }
}

impl<V, D> From<(V, D)> for DetailedField<V, D> {
    fn from((value, detail): (V, D)) -> Self {
        Self::with_detail(value, detail)
    }
}

/// Extension trait for collections with size limits
///
/// Provides methods for safely adding items to collections while respecting
/// configured limits, which is essential for `DoS` protection.
///
/// # Examples
///
/// ```
/// use feedparser_rs::types::LimitedCollectionExt;
///
/// let mut vec = Vec::new();
/// assert!(vec.try_push_limited("first", 2));
/// assert!(vec.try_push_limited("second", 2));
/// assert!(!vec.try_push_limited("third", 2)); // Exceeds limit
/// assert_eq!(vec.len(), 2);
/// ```
pub trait LimitedCollectionExt<T> {
    /// Try to push an item if the collection is below the limit
    ///
    /// Returns `true` if the item was added, `false` if limit was reached.
    fn try_push_limited(&mut self, item: T, limit: usize) -> bool;

    /// Check if the collection has reached its limit
    fn is_at_limit(&self, limit: usize) -> bool;

    /// Get remaining capacity before reaching the limit
    fn remaining_capacity(&self, limit: usize) -> usize;
}

impl<T> LimitedCollectionExt<T> for Vec<T> {
    #[inline]
    fn try_push_limited(&mut self, item: T, limit: usize) -> bool {
        if self.len() < limit {
            self.push(item);
            true
        } else {
            false
        }
    }

    #[inline]
    fn is_at_limit(&self, limit: usize) -> bool {
        self.len() >= limit
    }

    #[inline]
    #[allow(dead_code)]
    fn remaining_capacity(&self, limit: usize) -> usize {
        limit.saturating_sub(self.len())
    }
}

/// Trait for types that can be built from XML attributes
///
/// Implement this trait for structs that are parsed from XML element attributes,
/// providing a consistent interface for attribute extraction with limit validation.
pub trait FromAttributes: Sized {
    /// Parse from XML attributes with limit validation
    ///
    /// # Arguments
    ///
    /// * `attrs` - Iterator over XML attributes
    /// * `max_attr_length` - Maximum allowed attribute value length
    ///
    /// # Returns
    ///
    /// * `Some(Self)` - Successfully parsed struct
    /// * `None` - Required attributes missing or validation failed
    fn from_attributes<'a, I>(attrs: I, max_attr_length: usize) -> Option<Self>
    where
        I: Iterator<Item = quick_xml::events::attributes::Attribute<'a>>;
}

/// Generic trait for parsing types from various sources using GAT
///
/// This trait provides a unified interface for constructing types from
/// different data sources (JSON values, XML elements, etc.) using
/// Generic Associated Types (GAT) for flexible lifetime handling.
///
/// # Examples
///
/// ```
/// use feedparser_rs::types::generics::ParseFrom;
/// use feedparser_rs::types::Person;
/// use serde_json::json;
///
/// let json = json!({"name": "John Doe", "url": "https://example.com"});
/// let person = Person::parse_from(&json);
/// assert!(person.is_some());
/// assert_eq!(person.unwrap().name.as_deref(), Some("John Doe"));
/// ```
pub trait ParseFrom<Source>: Sized {
    /// Parse from the given source
    ///
    /// # Arguments
    ///
    /// * `source` - The source data to parse from
    ///
    /// # Returns
    ///
    /// * `Some(Self)` - Successfully parsed instance
    /// * `None` - Failed to parse (missing required fields, wrong type, etc.)
    fn parse_from(source: Source) -> Option<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detailed_field_from_value() {
        let field: DetailedField<String, i32> = DetailedField::from_value("test".to_string());
        assert_eq!(field.value(), "test");
        assert!(field.detail().is_none());
    }

    #[test]
    fn test_detailed_field_with_detail() {
        let field = DetailedField::with_detail("test".to_string(), 42);
        assert_eq!(field.value(), "test");
        assert_eq!(field.detail(), Some(&42));
    }

    #[test]
    fn test_detailed_field_from_tuple() {
        let field: DetailedField<String, i32> = ("test".to_string(), 42).into();
        assert_eq!(field.value(), "test");
        assert_eq!(field.detail(), Some(&42));
    }

    #[test]
    fn test_detailed_field_into_parts() {
        let field = DetailedField::with_detail("test".to_string(), 42);
        let (value, detail) = field.into_parts();
        assert_eq!(value, "test");
        assert_eq!(detail, Some(42));
    }

    #[test]
    fn test_limited_collection_try_push() {
        let mut vec: Vec<i32> = Vec::new();
        assert!(vec.try_push_limited(1, 3));
        assert!(vec.try_push_limited(2, 3));
        assert!(vec.try_push_limited(3, 3));
        assert!(!vec.try_push_limited(4, 3));
        assert_eq!(vec.len(), 3);
    }

    #[test]
    fn test_limited_collection_is_at_limit() {
        let mut vec: Vec<i32> = Vec::new();
        assert!(!vec.is_at_limit(2));
        vec.push(1);
        assert!(!vec.is_at_limit(2));
        vec.push(2);
        assert!(vec.is_at_limit(2));
    }

    #[test]
    fn test_limited_collection_remaining_capacity() {
        let mut vec: Vec<i32> = Vec::new();
        assert_eq!(vec.remaining_capacity(5), 5);
        vec.push(1);
        vec.push(2);
        assert_eq!(vec.remaining_capacity(5), 3);
    }

    #[test]
    fn test_detailed_field_default() {
        let field: DetailedField<String, i32> = DetailedField::default();
        assert_eq!(field.value(), "");
        assert!(field.detail().is_none());
    }

    #[test]
    fn test_detailed_field_mutability() {
        let mut field = DetailedField::from_value("original".to_string());
        *field.value_mut() = "modified".to_string();
        assert_eq!(field.value(), "modified");

        field.set_detail(100);
        assert_eq!(field.detail(), Some(&100));

        let taken = field.take_detail();
        assert_eq!(taken, Some(100));
        assert!(field.detail().is_none());
    }
}
