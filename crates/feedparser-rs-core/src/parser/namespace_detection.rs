//! Namespace tag detection utilities
//!
//! This module provides efficient namespace prefix matching for XML elements.
//! Instead of duplicating `is_dc_tag()`, `is_content_tag()`, etc. across parsers,
//! we use a single `NamespacePrefix` abstraction.
//!
//! # Examples
//!
//! ```ignore
//! use feedparser_rs::parser::namespace_detection::namespaces;
//!
//! let tag_name = b"dc:creator";
//! if let Some(element) = namespaces::DC.matches(tag_name) {
//!     assert_eq!(element, "creator");
//! }
//! ```

/// Namespace prefix configuration for efficient tag matching
///
/// This struct stores a namespace prefix (e.g., `"dc:"`) and provides
/// zero-cost matching against tag names. It uses `const fn` construction
/// for compile-time initialization.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // Future use: Will be adopted when consolidating namespace detection
pub struct NamespacePrefix {
    prefix: &'static [u8],
    prefix_len: usize,
}

impl NamespacePrefix {
    /// Creates a new namespace prefix matcher
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use feedparser_rs::parser::namespace_detection::NamespacePrefix;
    ///
    /// const CUSTOM: NamespacePrefix = NamespacePrefix::new("custom:");
    /// ```
    #[must_use]
    #[allow(dead_code)] // Future use
    pub const fn new(prefix: &'static str) -> Self {
        let prefix_bytes = prefix.as_bytes();
        Self {
            prefix: prefix_bytes,
            prefix_len: prefix_bytes.len(),
        }
    }

    /// Check if tag name matches this namespace prefix
    ///
    /// Returns the element name after the prefix if matched, or `None` if
    /// the tag doesn't start with this prefix.
    ///
    /// # Arguments
    ///
    /// * `tag_name` - The full tag name (e.g., `b"dc:creator"`)
    ///
    /// # Returns
    ///
    /// * `Some(element)` - Element name after prefix (e.g., `"creator"`)
    /// * `None` - Tag doesn't match this prefix
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use feedparser_rs::parser::namespace_detection::namespaces;
    ///
    /// assert_eq!(namespaces::DC.matches(b"dc:creator"), Some("creator"));
    /// assert_eq!(namespaces::DC.matches(b"content:encoded"), None);
    /// assert_eq!(namespaces::DC.matches(b"dc:"), Some("")); // Empty element name
    /// ```
    #[inline]
    #[must_use]
    #[allow(dead_code)] // Future use
    pub fn matches<'a>(&self, tag_name: &'a [u8]) -> Option<&'a str> {
        if tag_name.starts_with(self.prefix) {
            std::str::from_utf8(&tag_name[self.prefix_len..]).ok()
        } else {
            None
        }
    }

    /// Returns the prefix string (e.g., `"dc:"`)
    ///
    /// # Safety
    ///
    /// This function uses `unsafe` because `std::str::from_utf8` is not
    /// yet `const fn` stable. The safety invariant is guaranteed by:
    ///
    /// 1. `new()` only accepts `&'static str` (compile-time valid UTF-8)
    /// 2. `as_bytes()` is a reversible, safe transformation
    /// 3. The field is private and immutable - no external mutation
    /// 4. All instances are const-initialized with string literals
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use feedparser_rs::parser::namespace_detection::namespaces;
    ///
    /// assert_eq!(namespaces::DC.prefix(), "dc:");
    /// ```
    #[inline]
    #[must_use]
    #[allow(dead_code)] // Future use
    pub const fn prefix(&self) -> &'static str {
        // SAFETY: prefix is always constructed from &'static str in new(),
        // which guarantees valid UTF-8. The field is private and immutable,
        // so no external code can violate this invariant.
        #[allow(unsafe_code)]
        unsafe {
            std::str::from_utf8_unchecked(self.prefix)
        }
    }
}

/// Common namespace prefixes used in RSS/Atom feeds
///
/// These constants provide efficient namespace detection across all parsers.
/// Each constant uses `const fn` construction for zero runtime cost.
///
/// # Available Namespaces
///
/// - `DC` - Dublin Core (`dc:`)
/// - `CONTENT` - RSS Content Module (`content:`)
/// - `MEDIA` - Media RSS (`media:`)
/// - `ITUNES` - iTunes Podcast (`itunes:`)
/// - `PODCAST` - Podcast 2.0 (`podcast:`)
///
/// # Examples
///
/// ```ignore
/// use feedparser_rs::parser::namespace_detection::namespaces;
///
/// let tag = b"itunes:author";
/// if let Some(element) = namespaces::ITUNES.matches(tag) {
///     println!("iTunes element: {element}");
/// }
/// ```
pub mod namespaces {
    use super::NamespacePrefix;

    /// Dublin Core namespace prefix (`dc:`)
    ///
    /// Common elements: `creator`, `publisher`, `rights`, `date`, `identifier`
    #[allow(dead_code)] // Future use
    pub const DC: NamespacePrefix = NamespacePrefix::new("dc:");

    /// RSS Content Module namespace prefix (`content:`)
    ///
    /// Common elements: `encoded`
    #[allow(dead_code)] // Future use
    pub const CONTENT: NamespacePrefix = NamespacePrefix::new("content:");

    /// Media RSS namespace prefix (`media:`)
    ///
    /// Common elements: `content`, `thumbnail`, `description`, `keywords`
    #[allow(dead_code)] // Future use
    pub const MEDIA: NamespacePrefix = NamespacePrefix::new("media:");

    /// iTunes Podcast namespace prefix (`itunes:`)
    ///
    /// Common elements: `author`, `summary`, `explicit`, `category`, `image`
    #[allow(dead_code)] // Future use
    pub const ITUNES: NamespacePrefix = NamespacePrefix::new("itunes:");

    /// Podcast 2.0 namespace prefix (`podcast:`)
    ///
    /// Common elements: `transcript`, `chapters`, `soundbite`, `person`
    #[allow(dead_code)] // Future use
    pub const PODCAST: NamespacePrefix = NamespacePrefix::new("podcast:");

    /// Atom Threading Extensions namespace prefix (`thr:`)
    ///
    /// Common elements: `in-reply-to`, `total`
    #[allow(dead_code)] // Future use
    pub const THR: NamespacePrefix = NamespacePrefix::new("thr:");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_prefix_matches() {
        assert_eq!(namespaces::DC.matches(b"dc:creator"), Some("creator"));
        assert_eq!(namespaces::DC.matches(b"dc:publisher"), Some("publisher"));
        assert_eq!(
            namespaces::CONTENT.matches(b"content:encoded"),
            Some("encoded")
        );
        assert_eq!(
            namespaces::MEDIA.matches(b"media:thumbnail"),
            Some("thumbnail")
        );
    }

    #[test]
    fn test_namespace_prefix_no_match() {
        assert_eq!(namespaces::DC.matches(b"content:encoded"), None);
        assert_eq!(namespaces::CONTENT.matches(b"dc:creator"), None);
        assert_eq!(namespaces::MEDIA.matches(b"itunes:author"), None);
    }

    #[test]
    fn test_namespace_prefix_empty_element() {
        // Edge case: prefix matches but no element name
        assert_eq!(namespaces::DC.matches(b"dc:"), Some(""));
    }

    #[test]
    fn test_namespace_prefix_invalid_utf8() {
        // Invalid UTF-8 after prefix should return None
        let invalid = b"dc:\xFF\xFE";
        assert_eq!(namespaces::DC.matches(invalid), None);
    }

    #[test]
    fn test_namespace_prefix_getter() {
        assert_eq!(namespaces::DC.prefix(), "dc:");
        assert_eq!(namespaces::CONTENT.prefix(), "content:");
        assert_eq!(namespaces::MEDIA.prefix(), "media:");
        assert_eq!(namespaces::ITUNES.prefix(), "itunes:");
        assert_eq!(namespaces::PODCAST.prefix(), "podcast:");
    }

    #[test]
    fn test_custom_namespace() {
        const CUSTOM: NamespacePrefix = NamespacePrefix::new("custom:");
        assert_eq!(CUSTOM.matches(b"custom:field"), Some("field"));
        assert_eq!(CUSTOM.matches(b"other:field"), None);
    }
}
