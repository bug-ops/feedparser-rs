//! Common parsing utilities shared between RSS and Atom parsers
//!
//! This module eliminates code duplication by providing shared functionality
//! for XML parsing operations used by both feed formats.

use crate::{
    ParserLimits,
    error::{FeedError, Result},
    types::{FeedVersion, ParsedFeed},
};
use quick_xml::{Reader, events::Event};

pub use crate::types::{FromAttributes, LimitedCollectionExt};
pub use crate::util::text::bytes_to_string;

/// Initial capacity for XML event buffer (fits most elements)
pub const EVENT_BUFFER_CAPACITY: usize = 1024;

/// Initial capacity for text content (typical field size)
pub const TEXT_BUFFER_CAPACITY: usize = 256;

/// Creates a new event buffer with optimized capacity
///
/// This factory function provides a semantic way to create XML event buffers
/// with consistent capacity across all parsers. Using this instead of direct
/// `Vec::with_capacity()` calls makes it easier to tune buffer sizes in one place.
///
/// # Returns
///
/// A `Vec<u8>` pre-allocated with `EVENT_BUFFER_CAPACITY` (1024 bytes)
///
/// # Examples
///
/// ```ignore
/// use feedparser_rs::parser::common::new_event_buffer;
///
/// let mut buf = new_event_buffer();
/// assert!(buf.capacity() >= 1024);
/// ```
#[inline]
#[must_use]
#[allow(dead_code)] // Future use: Will be adopted when refactoring parsers
pub fn new_event_buffer() -> Vec<u8> {
    Vec::with_capacity(EVENT_BUFFER_CAPACITY)
}

/// Creates a new text buffer with optimized capacity
///
/// This factory function provides a semantic way to create text content buffers
/// with consistent capacity across all parsers. Useful for accumulating text
/// content from XML elements.
///
/// # Returns
///
/// A `String` pre-allocated with `TEXT_BUFFER_CAPACITY` (256 bytes)
///
/// # Examples
///
/// ```ignore
/// use feedparser_rs::parser::common::new_text_buffer;
///
/// let mut text = new_text_buffer();
/// assert!(text.capacity() >= 256);
/// ```
#[inline]
#[must_use]
#[allow(dead_code)] // Future use: Will be adopted when refactoring parsers
pub fn new_text_buffer() -> String {
    String::with_capacity(TEXT_BUFFER_CAPACITY)
}

/// Context for parsing operations
///
/// Bundles together common parsing state to reduce function parameter count.
/// Future use: Will be adopted when refactoring parsers to reduce parameter passing
#[allow(dead_code)]
pub struct ParseContext<'a> {
    /// XML reader
    pub reader: Reader<&'a [u8]>,
    /// Reusable buffer for XML events
    pub buf: Vec<u8>,
    /// Parser limits for validation
    pub limits: ParserLimits,
    /// Current nesting depth
    pub depth: usize,
}

impl<'a> ParseContext<'a> {
    /// Create a new parse context from raw data
    #[allow(dead_code)]
    pub fn new(data: &'a [u8], limits: ParserLimits) -> Result<Self> {
        limits
            .check_feed_size(data.len())
            .map_err(|e| FeedError::InvalidFormat(e.to_string()))?;

        let mut reader = Reader::from_reader(data);
        reader.config_mut().trim_text(true);

        Ok(Self {
            reader,
            buf: Vec::with_capacity(EVENT_BUFFER_CAPACITY),
            limits,
            depth: 1, // Start at 1 for root element
        })
    }

    /// Check and increment depth, returning error if limit exceeded
    #[inline]
    #[allow(dead_code)]
    pub fn check_depth(&mut self) -> Result<()> {
        self.depth += 1;
        if self.depth > self.limits.max_nesting_depth {
            return Err(FeedError::InvalidFormat(format!(
                "XML nesting depth {} exceeds maximum {}",
                self.depth, self.limits.max_nesting_depth
            )));
        }
        Ok(())
    }

    /// Decrement depth safely
    #[inline]
    #[allow(dead_code)]
    pub const fn decrement_depth(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// Clear the buffer
    #[inline]
    #[allow(dead_code)]
    pub fn clear_buf(&mut self) {
        self.buf.clear();
    }
}

/// Initialize a `ParsedFeed` with common setup for any format
#[inline]
pub fn init_feed(version: FeedVersion, max_entries: usize) -> ParsedFeed {
    let mut feed = ParsedFeed::with_capacity(max_entries);
    feed.version = version;
    feed.encoding = String::from("utf-8");
    feed
}

/// Check nesting depth and return error if exceeded
///
/// This is a standalone helper for parsers that don't use `ParseContext`.
#[inline]
pub fn check_depth(depth: usize, max_depth: usize) -> Result<()> {
    if depth > max_depth {
        return Err(FeedError::InvalidFormat(format!(
            "XML nesting depth {depth} exceeds maximum {max_depth}"
        )));
    }
    Ok(())
}

/// Extract local name from namespaced element if prefix matches
///
/// Validates tag name contains only alphanumeric characters and hyphens
/// to prevent injection attacks.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(extract_ns_local_name(b"dc:creator", b"dc:"), Some("creator"));
/// assert_eq!(extract_ns_local_name(b"dc:creator", b"atom:"), None);
/// assert_eq!(extract_ns_local_name(b"dc:<script>", b"dc:"), None); // Invalid chars
/// ```
#[inline]
pub fn extract_ns_local_name<'a>(name: &'a [u8], prefix: &[u8]) -> Option<&'a str> {
    if name.starts_with(prefix) {
        let tag_name = std::str::from_utf8(&name[prefix.len()..]).ok()?;
        // Security: validate tag name (alphanumeric + hyphen only)
        if !tag_name.is_empty() && tag_name.chars().all(|c| c.is_alphanumeric() || c == '-') {
            Some(tag_name)
        } else {
            None
        }
    } else {
        None
    }
}

/// Check if element is a Dublin Core namespaced tag
///
/// # Examples
///
/// ```ignore
/// assert_eq!(is_dc_tag(b"dc:creator"), Some("creator"));
/// assert_eq!(is_dc_tag(b"dc:subject"), Some("subject"));
/// assert_eq!(is_dc_tag(b"content:encoded"), None);
/// ```
#[inline]
pub fn is_dc_tag(name: &[u8]) -> Option<&str> {
    extract_ns_local_name(name, b"dc:")
}

/// Check if element is a Content namespaced tag
///
/// # Examples
///
/// ```ignore
/// assert_eq!(is_content_tag(b"content:encoded"), Some("encoded"));
/// assert_eq!(is_content_tag(b"dc:creator"), None);
/// ```
#[inline]
pub fn is_content_tag(name: &[u8]) -> Option<&str> {
    extract_ns_local_name(name, b"content:")
}

/// Check if element is a Syndication namespaced tag
///
/// # Examples
///
/// ```ignore
/// assert_eq!(is_syn_tag(b"syn:updatePeriod"), Some("updatePeriod"));
/// assert_eq!(is_syn_tag(b"syn:updateFrequency"), Some("updateFrequency"));
/// assert_eq!(is_syn_tag(b"dc:creator"), None);
/// ```
#[inline]
pub fn is_syn_tag(name: &[u8]) -> Option<&str> {
    extract_ns_local_name(name, b"syn:")
}

/// Check if element is a Media RSS namespaced tag
///
/// # Examples
///
/// ```ignore
/// assert_eq!(is_media_tag(b"media:thumbnail"), Some("thumbnail"));
/// assert_eq!(is_media_tag(b"media:content"), Some("content"));
/// assert_eq!(is_media_tag(b"dc:creator"), None);
/// ```
#[inline]
pub fn is_media_tag(name: &[u8]) -> Option<&str> {
    extract_ns_local_name(name, b"media:")
}

/// Check if element is a `GeoRSS` namespaced tag
///
/// # Examples
///
/// ```ignore
/// assert_eq!(is_georss_tag(b"georss:point"), Some("point"));
/// assert_eq!(is_georss_tag(b"georss:line"), Some("line"));
/// assert_eq!(is_georss_tag(b"dc:creator"), None);
/// ```
#[inline]
pub fn is_georss_tag(name: &[u8]) -> Option<&str> {
    extract_ns_local_name(name, b"georss:")
}

/// Check if element matches an iTunes namespace tag
///
/// Supports both prefixed (itunes:author) and unprefixed (author) forms
/// for compatibility with non-compliant feeds.
///
/// # Examples
///
/// ```ignore
/// assert!(is_itunes_tag(b"itunes:author", b"author"));
/// assert!(is_itunes_tag(b"author", b"author")); // Fallback for non-prefixed
/// assert!(!is_itunes_tag(b"itunes:title", b"author"));
/// ```
#[inline]
pub fn is_itunes_tag(name: &[u8], tag: &[u8]) -> bool {
    if name.starts_with(b"itunes:") && &name[7..] == tag {
        return true;
    }
    // Fallback for feeds without prefix
    name == tag
}

/// Extract xml:base attribute from element
///
/// Returns the base URL string if xml:base attribute exists.
/// Respects `max_attribute_length` limit for `DoS` protection.
///
/// # Arguments
///
/// * `element` - The XML element to extract xml:base from
/// * `max_attr_length` - Maximum allowed attribute length (`DoS` protection)
///
/// # Returns
///
/// * `Some(String)` - The xml:base value if found and within length limit
/// * `None` - If attribute not found or exceeds length limit
///
/// # Examples
///
/// ```ignore
/// use feedparser_rs::parser::common::extract_xml_base;
///
/// let element = /* BytesStart from quick-xml */;
/// if let Some(base) = extract_xml_base(&element, 1024) {
///     println!("Base URL: {}", base);
/// }
/// ```
pub fn extract_xml_base(
    element: &quick_xml::events::BytesStart,
    max_attr_length: usize,
) -> Option<String> {
    element
        .attributes()
        .flatten()
        .find(|attr| {
            let key = attr.key.as_ref();
            key == b"xml:base" || key == b"base"
        })
        .filter(|attr| attr.value.len() <= max_attr_length)
        .and_then(|attr| attr.unescape_value().ok())
        .map(|s| s.to_string())
}

/// Extract xml:lang attribute from element
///
/// Returns the language code if xml:lang or lang attribute exists.
/// Respects `max_attribute_length` limit for `DoS` protection.
///
/// # Arguments
///
/// * `element` - The XML element to extract xml:lang from
/// * `max_attr_length` - Maximum allowed attribute length (`DoS` protection)
///
/// # Returns
///
/// * `Some(String)` - The xml:lang value if found and within length limit
/// * `None` - If attribute not found or exceeds length limit
///
/// # Examples
///
/// ```ignore
/// use feedparser_rs::parser::common::extract_xml_lang;
///
/// let element = /* BytesStart from quick-xml */;
/// if let Some(lang) = extract_xml_lang(&element, 1024) {
///     println!("Language: {}", lang);
/// }
/// ```
pub fn extract_xml_lang(
    element: &quick_xml::events::BytesStart,
    max_attr_length: usize,
) -> Option<String> {
    element
        .attributes()
        .flatten()
        .find(|attr| {
            let key = attr.key.as_ref();
            key == b"xml:lang" || key == b"lang"
        })
        .filter(|attr| attr.value.len() <= max_attr_length)
        .and_then(|attr| attr.unescape_value().ok())
        .map(|s| s.to_string())
}

/// Read text content from current XML element (handles text and CDATA)
pub fn read_text(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
) -> Result<String> {
    let mut text = String::with_capacity(TEXT_BUFFER_CAPACITY);

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Text(e)) => {
                append_bytes(&mut text, e.as_ref(), limits.max_text_length)?;
            }
            Ok(Event::CData(e)) => {
                append_bytes(&mut text, e.as_ref(), limits.max_text_length)?;
            }
            Ok(Event::End(_) | Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(text)
}

#[inline]
fn append_bytes(text: &mut String, bytes: &[u8], max_len: usize) -> Result<()> {
    if text.len() + bytes.len() > max_len {
        return Err(FeedError::InvalidFormat(format!(
            "Text field exceeds maximum length of {max_len} bytes"
        )));
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => text.push_str(s),
        Err(_) => text.push_str(&String::from_utf8_lossy(bytes)),
    }
    Ok(())
}

/// Skip unknown element and all its children (enforces nesting depth limits)
pub fn skip_element(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
    current_depth: usize,
) -> Result<()> {
    let mut local_depth: usize = 1;

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(_)) => {
                local_depth += 1;
                if current_depth + local_depth > limits.max_nesting_depth {
                    return Err(FeedError::InvalidFormat(format!(
                        "XML nesting depth exceeds maximum of {}",
                        limits.max_nesting_depth
                    )));
                }
            }
            Ok(Event::End(_)) => {
                local_depth = local_depth.saturating_sub(1);
                if local_depth == 0 {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(())
}

/// Skip to end of specified element (for attribute-only elements like `<link>`)
pub fn skip_to_end(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>, tag: &[u8]) -> Result<()> {
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::End(e)) if e.local_name().as_ref() == tag => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_to_string_valid_utf8() {
        let bytes = b"Hello, World!";
        assert_eq!(bytes_to_string(bytes), "Hello, World!");
    }

    #[test]
    fn test_bytes_to_string_invalid_utf8() {
        let bytes = &[0xff, 0xfe, 0x48, 0x65, 0x6c, 0x6c, 0x6f];
        let result = bytes_to_string(bytes);
        assert!(result.contains("Hello"));
    }

    #[test]
    fn test_read_text_basic() {
        let xml = b"<title>Test Title</title>";
        let mut reader = Reader::from_reader(&xml[..]);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        let limits = ParserLimits::default();

        // Skip to after the start tag
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(_)) => break,
                Ok(Event::Eof) => panic!("Unexpected EOF"),
                _ => {}
            }
            buf.clear();
        }
        buf.clear();

        let text = read_text(&mut reader, &mut buf, &limits).unwrap();
        assert_eq!(text, "Test Title");
    }

    #[test]
    fn test_read_text_exceeds_limit() {
        let xml = b"<title>This is a very long title</title>";
        let mut reader = Reader::from_reader(&xml[..]);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        let limits = ParserLimits {
            max_text_length: 10,
            ..ParserLimits::default()
        };

        // Skip to after the start tag
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(_)) => break,
                Ok(Event::Eof) => panic!("Unexpected EOF"),
                _ => {}
            }
            buf.clear();
        }
        buf.clear();

        let result = read_text(&mut reader, &mut buf, &limits);
        assert!(result.is_err());
    }

    #[test]
    fn test_skip_element_basic() {
        let xml = b"<parent><child>content</child></parent>";
        let mut reader = Reader::from_reader(&xml[..]);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        let limits = ParserLimits::default();
        let depth = 1;

        // Skip to after the start tag
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(_)) => break,
                Ok(Event::Eof) => panic!("Unexpected EOF"),
                _ => {}
            }
            buf.clear();
        }
        buf.clear();

        let result = skip_element(&mut reader, &mut buf, &limits, depth);
        assert!(result.is_ok());
    }
}
