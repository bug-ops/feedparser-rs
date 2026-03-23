pub mod atom;
mod common;
mod detect;
pub mod json;
pub mod namespace_detection;
pub mod rss;
pub mod rss10;

use crate::{error::Result, types::ParsedFeed};

pub use common::skip_element;
pub use detect::detect_format;

/// Parse feed from raw bytes
///
/// This is the main entry point for parsing feeds. It automatically detects
/// the feed format (RSS, Atom, JSON) and parses accordingly.
///
/// # Errors
///
/// Returns a `FeedError` if the feed cannot be parsed. However, in most cases,
/// the parser will set the `bozo` flag and return partial results rather than
/// returning an error.
///
/// # Examples
///
/// ```
/// use feedparser_rs::parse;
///
/// let xml = r#"
///     <?xml version="1.0"?>
///     <rss version="2.0">
///         <channel>
///             <title>Example Feed</title>
///         </channel>
///     </rss>
/// "#;
///
/// let feed = parse(xml.as_bytes()).unwrap();
/// assert_eq!(feed.feed.title.as_deref(), Some("Example Feed"));
/// ```
pub fn parse(data: &[u8]) -> Result<ParsedFeed> {
    parse_with_limits(data, crate::ParserLimits::default())
}

/// Parse feed with custom parser limits
///
/// This allows controlling resource usage when parsing untrusted feeds.
///
/// # Examples
///
/// ```
/// use feedparser_rs::{parse_with_limits, ParserLimits};
///
/// let xml = b"<rss version=\"2.0\"><channel><title>Test</title></channel></rss>";
/// let limits = ParserLimits::strict();
/// let feed = parse_with_limits(xml, limits).unwrap();
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - Feed size exceeds limits
/// - Format is unknown or unsupported
/// - Fatal parsing error occurs
pub fn parse_with_limits(data: &[u8], limits: crate::ParserLimits) -> Result<ParsedFeed> {
    use crate::types::FeedVersion;
    use crate::util::encoding::detect_and_convert;

    // Detect encoding and convert to UTF-8 before parsing.
    // This handles ISO-8859-1, Windows-1252, UTF-16, and BOM-prefixed feeds.
    let (utf8_string, detected_encoding) = detect_and_convert(data)
        .unwrap_or_else(|_| (String::from_utf8_lossy(data).into_owned(), "UTF-8"));

    let utf8_bytes = utf8_string.as_bytes();
    let encoding_label = detected_encoding.to_lowercase();

    // Detect format on UTF-8 data (required for correct UTF-16 detection)
    let version = detect_format(utf8_bytes);

    // Parse based on detected format, then update the encoding field
    let mut feed = match version {
        // RSS variants (all use RSS 2.0 parser for now)
        FeedVersion::Rss20 | FeedVersion::Rss092 | FeedVersion::Rss091 | FeedVersion::Rss090 => {
            rss::parse_rss20_with_limits(utf8_bytes, limits)
        }

        // Atom variants
        FeedVersion::Atom10 | FeedVersion::Atom03 => {
            atom::parse_atom10_with_limits(utf8_bytes, limits)
        }

        // RSS 1.0 (RDF)
        FeedVersion::Rss10 => rss10::parse_rss10_with_limits(utf8_bytes, limits),

        // JSON Feed
        FeedVersion::JsonFeed10 | FeedVersion::JsonFeed11 => {
            json::parse_json_feed_with_limits(utf8_bytes, limits)
        }

        // Unknown format - try RSS first (most common)
        FeedVersion::Unknown => {
            // Try RSS first
            if let Ok(feed) = rss::parse_rss20_with_limits(utf8_bytes, limits) {
                return Ok(with_encoding(feed, encoding_label));
            }

            // Try Atom
            atom::parse_atom10_with_limits(utf8_bytes, limits)
        }
    }?;

    feed.encoding = encoding_label;
    Ok(feed)
}

#[inline]
fn with_encoding(mut feed: ParsedFeed, encoding: String) -> ParsedFeed {
    feed.encoding = encoding;
    feed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_returns_ok() {
        let result = parse(b"test");
        assert!(result.is_ok());
    }
}
