//! Parser limits to prevent `DoS` attacks and excessive memory usage

/// Parser limits for protecting against denial-of-service attacks
///
/// These limits prevent malicious or malformed feeds from causing excessive
/// memory allocation, deep recursion, or other resource exhaustion issues.
///
/// # Examples
///
/// ```
/// use feedparser_rs::ParserLimits;
///
/// let limits = ParserLimits::default();
/// assert_eq!(limits.max_entries, 10_000);
///
/// // Custom limits for restricted environments
/// let strict = ParserLimits {
///     max_entries: 1_000,
///     max_feed_size_bytes: 10 * 1024 * 1024, // 10MB
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParserLimits {
    /// Maximum number of entries/items in a feed
    ///
    /// Prevents memory exhaustion from feeds with millions of items.
    /// Typical feeds have 10-100 entries, large feeds may have up to 1000.
    ///
    /// Default: 10,000 entries
    pub max_entries: usize,

    /// Maximum number of links per feed (channel-level)
    ///
    /// Prevents link bombing attacks.
    ///
    /// Default: 100 links
    pub max_links_per_feed: usize,

    /// Maximum number of links per entry
    ///
    /// Prevents link bombing in individual entries.
    ///
    /// Default: 50 links
    pub max_links_per_entry: usize,

    /// Maximum number of authors per feed or entry
    ///
    /// Default: 20 authors
    pub max_authors: usize,

    /// Maximum number of contributors per feed or entry
    ///
    /// Default: 20 contributors
    pub max_contributors: usize,

    /// Maximum number of tags/categories per feed or entry
    ///
    /// Default: 100 tags
    pub max_tags: usize,

    /// Maximum number of content blocks per entry
    ///
    /// Atom feeds can have multiple content elements.
    ///
    /// Default: 10 content blocks
    pub max_content_blocks: usize,

    /// Maximum number of enclosures per entry
    ///
    /// Podcast feeds typically have 1 enclosure per episode.
    ///
    /// Default: 20 enclosures
    pub max_enclosures: usize,

    /// Maximum number of XML namespaces
    ///
    /// Prevents namespace pollution attacks.
    ///
    /// Default: 100 namespaces
    pub max_namespaces: usize,

    /// Maximum XML nesting depth
    ///
    /// Prevents stack overflow from deeply nested XML.
    ///
    /// Default: 100 levels
    pub max_nesting_depth: usize,

    /// Maximum text field length in bytes
    ///
    /// Prevents excessive memory from huge title/description fields.
    ///
    /// Default: 10 MB
    pub max_text_length: usize,

    /// Maximum total feed size in bytes
    ///
    /// The entire feed must fit within this limit.
    ///
    /// Default: 100 MB
    pub max_feed_size_bytes: usize,

    /// Maximum attribute value length in bytes
    ///
    /// XML attributes should be reasonably sized.
    ///
    /// Default: 64 KB
    pub max_attribute_length: usize,

    /// Maximum number of podcast soundbites per entry
    ///
    /// Podcast 2.0 soundbite elements for shareable clips.
    ///
    /// Default: 10 soundbites
    pub max_podcast_soundbites: usize,

    /// Maximum number of podcast transcripts per entry
    ///
    /// Podcast 2.0 transcript elements.
    ///
    /// Default: 20 transcripts
    pub max_podcast_transcripts: usize,

    /// Maximum number of podcast funding elements per feed
    ///
    /// Podcast 2.0 funding elements for donation links.
    ///
    /// Default: 20 funding elements
    pub max_podcast_funding: usize,

    /// Maximum number of podcast person elements per entry
    ///
    /// Podcast 2.0 person elements for hosts, guests, etc.
    ///
    /// Default: 50 persons
    pub max_podcast_persons: usize,

    /// Maximum number of podcast value recipients per feed
    ///
    /// Podcast 2.0 value recipients for payment splitting.
    /// Prevents `DoS` from feeds with excessive recipient lists.
    ///
    /// Default: 20 recipients
    pub max_value_recipients: usize,
}

impl Default for ParserLimits {
    /// Creates default parser limits suitable for general use
    ///
    /// These defaults are conservative and should work for most feeds,
    /// including large podcast feeds and news aggregators.
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            max_links_per_feed: 100,
            max_links_per_entry: 50,
            max_authors: 20,
            max_contributors: 20,
            max_tags: 100,
            max_content_blocks: 10,
            max_enclosures: 20,
            max_namespaces: 100,
            max_nesting_depth: 100,
            max_text_length: 10 * 1024 * 1024,      // 10 MB
            max_feed_size_bytes: 100 * 1024 * 1024, // 100 MB
            max_attribute_length: 64 * 1024,        // 64 KB
            max_podcast_soundbites: 10,
            max_podcast_transcripts: 20,
            max_podcast_funding: 20,
            max_podcast_persons: 50,
            max_value_recipients: 20,
        }
    }
}

impl ParserLimits {
    /// Creates strict limits for resource-constrained environments
    ///
    /// Use this for embedded systems or when parsing untrusted feeds
    /// with minimal resources.
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::ParserLimits;
    ///
    /// let limits = ParserLimits::strict();
    /// assert_eq!(limits.max_entries, 1_000);
    /// ```
    #[must_use]
    pub const fn strict() -> Self {
        Self {
            max_entries: 1_000,
            max_links_per_feed: 20,
            max_links_per_entry: 10,
            max_authors: 5,
            max_contributors: 5,
            max_tags: 20,
            max_content_blocks: 3,
            max_enclosures: 5,
            max_namespaces: 20,
            max_nesting_depth: 50,
            max_text_length: 1024 * 1024,          // 1 MB
            max_feed_size_bytes: 10 * 1024 * 1024, // 10 MB
            max_attribute_length: 8 * 1024,        // 8 KB
            max_podcast_soundbites: 5,
            max_podcast_transcripts: 5,
            max_podcast_funding: 5,
            max_podcast_persons: 10,
            max_value_recipients: 5,
        }
    }

    /// Creates permissive limits for trusted feeds
    ///
    /// Use this only for feeds from trusted sources where you expect
    /// large data volumes (e.g., feed archives).
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::ParserLimits;
    ///
    /// let limits = ParserLimits::permissive();
    /// assert_eq!(limits.max_entries, 100_000);
    /// ```
    #[must_use]
    pub const fn permissive() -> Self {
        Self {
            max_entries: 100_000,
            max_links_per_feed: 500,
            max_links_per_entry: 200,
            max_authors: 100,
            max_contributors: 100,
            max_tags: 500,
            max_content_blocks: 50,
            max_enclosures: 100,
            max_namespaces: 500,
            max_nesting_depth: 200,
            max_text_length: 50 * 1024 * 1024,      // 50 MB
            max_feed_size_bytes: 500 * 1024 * 1024, // 500 MB
            max_attribute_length: 256 * 1024,       // 256 KB
            max_podcast_soundbites: 50,
            max_podcast_transcripts: 100,
            max_podcast_funding: 50,
            max_podcast_persons: 200,
            max_value_recipients: 50,
        }
    }

    /// Validates that a feed size is within limits
    ///
    /// Call this before starting to parse a feed.
    ///
    /// # Errors
    ///
    /// Returns an error if the feed exceeds `max_feed_size_bytes`.
    pub const fn check_feed_size(&self, size: usize) -> Result<(), LimitError> {
        if size > self.max_feed_size_bytes {
            Err(LimitError::FeedTooLarge {
                size,
                max: self.max_feed_size_bytes,
            })
        } else {
            Ok(())
        }
    }

    /// Validates that a collection size is within limits
    ///
    /// Use this during parsing to check collection sizes.
    ///
    /// # Errors
    ///
    /// Returns an error if the collection size exceeds the specified limit.
    pub const fn check_collection_size(
        &self,
        current: usize,
        limit: usize,
        name: &'static str,
    ) -> Result<(), LimitError> {
        if current >= limit {
            Err(LimitError::CollectionTooLarge {
                name,
                size: current,
                max: limit,
            })
        } else {
            Ok(())
        }
    }

    /// Validates XML nesting depth
    ///
    /// # Errors
    ///
    /// Returns an error if nesting depth exceeds `max_nesting_depth`.
    pub const fn check_nesting_depth(&self, depth: usize) -> Result<(), LimitError> {
        if depth > self.max_nesting_depth {
            Err(LimitError::NestingTooDeep {
                depth,
                max: self.max_nesting_depth,
            })
        } else {
            Ok(())
        }
    }

    /// Validates text field length
    ///
    /// # Errors
    ///
    /// Returns an error if text length exceeds `max_text_length`.
    pub const fn check_text_length(&self, length: usize) -> Result<(), LimitError> {
        if length > self.max_text_length {
            Err(LimitError::TextTooLong {
                length,
                max: self.max_text_length,
            })
        } else {
            Ok(())
        }
    }
}

/// Errors that occur when parser limits are exceeded
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[allow(missing_docs)] // Fields are self-explanatory from error messages
pub enum LimitError {
    /// Feed size exceeds maximum allowed
    #[error("Feed size ({size} bytes) exceeds maximum ({max} bytes)")]
    FeedTooLarge { size: usize, max: usize },

    /// Collection (entries, links, etc.) has too many items
    #[error("Collection '{name}' has {size} items, exceeds maximum ({max})")]
    CollectionTooLarge {
        name: &'static str,
        size: usize,
        max: usize,
    },

    /// XML nesting is too deep
    #[error("XML nesting depth ({depth}) exceeds maximum ({max})")]
    NestingTooDeep { depth: usize, max: usize },

    /// Text field is too long
    #[error("Text field length ({length} bytes) exceeds maximum ({max} bytes)")]
    TextTooLong { length: usize, max: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limits() {
        let limits = ParserLimits::default();
        assert_eq!(limits.max_entries, 10_000);
        assert_eq!(limits.max_feed_size_bytes, 100 * 1024 * 1024);
    }

    #[test]
    fn test_strict_limits() {
        let limits = ParserLimits::strict();
        assert_eq!(limits.max_entries, 1_000);
        assert!(limits.max_entries < ParserLimits::default().max_entries);
    }

    #[test]
    fn test_permissive_limits() {
        let limits = ParserLimits::permissive();
        assert_eq!(limits.max_entries, 100_000);
        assert!(limits.max_entries > ParserLimits::default().max_entries);
    }

    #[test]
    fn test_check_feed_size_ok() {
        let limits = ParserLimits::default();
        assert!(limits.check_feed_size(1024).is_ok());
    }

    #[test]
    fn test_check_feed_size_too_large() {
        let limits = ParserLimits::default();
        let result = limits.check_feed_size(200 * 1024 * 1024);
        assert!(result.is_err());
        assert!(matches!(result, Err(LimitError::FeedTooLarge { .. })));
    }

    #[test]
    fn test_check_collection_size_ok() {
        let limits = ParserLimits::default();
        assert!(
            limits
                .check_collection_size(50, limits.max_entries, "entries")
                .is_ok()
        );
    }

    #[test]
    fn test_check_collection_size_too_large() {
        let limits = ParserLimits::default();
        let result = limits.check_collection_size(10_001, limits.max_entries, "entries");
        assert!(result.is_err());
        assert!(matches!(result, Err(LimitError::CollectionTooLarge { .. })));
    }

    #[test]
    fn test_check_nesting_depth_ok() {
        let limits = ParserLimits::default();
        assert!(limits.check_nesting_depth(50).is_ok());
    }

    #[test]
    fn test_check_nesting_depth_too_deep() {
        let limits = ParserLimits::default();
        let result = limits.check_nesting_depth(101);
        assert!(result.is_err());
        assert!(matches!(result, Err(LimitError::NestingTooDeep { .. })));
    }

    #[test]
    fn test_check_text_length_ok() {
        let limits = ParserLimits::default();
        assert!(limits.check_text_length(1024).is_ok());
    }

    #[test]
    fn test_check_text_length_too_long() {
        let limits = ParserLimits::default();
        let result = limits.check_text_length(20 * 1024 * 1024);
        assert!(result.is_err());
        assert!(matches!(result, Err(LimitError::TextTooLong { .. })));
    }

    #[test]
    fn test_limit_error_display() {
        let err = LimitError::FeedTooLarge {
            size: 200_000_000,
            max: 100_000_000,
        };
        let msg = err.to_string();
        assert!(msg.contains("200000000"));
        assert!(msg.contains("100000000"));
    }

    #[test]
    fn test_max_value_recipients_default() {
        let limits = ParserLimits::default();
        assert_eq!(limits.max_value_recipients, 20);
    }

    #[test]
    fn test_max_value_recipients_strict() {
        let limits = ParserLimits::strict();
        assert_eq!(limits.max_value_recipients, 5);
        assert!(limits.max_value_recipients < ParserLimits::default().max_value_recipients);
    }

    #[test]
    fn test_max_value_recipients_permissive() {
        let limits = ParserLimits::permissive();
        assert_eq!(limits.max_value_recipients, 50);
        assert!(limits.max_value_recipients > ParserLimits::default().max_value_recipients);
    }

    #[test]
    fn test_value_recipients_limit_enforcement() {
        let limits = ParserLimits::default();

        // Within limit
        assert!(
            limits
                .check_collection_size(19, limits.max_value_recipients, "value_recipients")
                .is_ok()
        );

        // At limit
        assert!(
            limits
                .check_collection_size(20, limits.max_value_recipients, "value_recipients")
                .is_err()
        );

        // Exceeds limit
        let result =
            limits.check_collection_size(21, limits.max_value_recipients, "value_recipients");
        assert!(result.is_err());
        assert!(matches!(result, Err(LimitError::CollectionTooLarge { .. })));
    }
}
