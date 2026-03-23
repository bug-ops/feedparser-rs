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

    // Detect format
    let version = detect_format(data);

    // Parse based on detected format
    match version {
        // RSS variants (all use RSS 2.0 parser for now)
        FeedVersion::Rss20 | FeedVersion::Rss092 | FeedVersion::Rss091 | FeedVersion::Rss090 => {
            rss::parse_rss20_with_limits(data, limits)
        }

        // Atom variants
        FeedVersion::Atom10 | FeedVersion::Atom03 => atom::parse_atom10_with_limits(data, limits),

        // RSS 1.0 (RDF)
        FeedVersion::Rss10 => rss10::parse_rss10_with_limits(data, limits),

        // JSON Feed
        FeedVersion::JsonFeed10 | FeedVersion::JsonFeed11 => {
            json::parse_json_feed_with_limits(data, limits)
        }

        // Unknown format - return a bozo feed since the format is unrecognized.
        // The bozo pattern requires we never panic and always return partial data,
        // but unrecognizable input must signal the caller via bozo=true.
        FeedVersion::Unknown => {
            let mut feed = crate::types::ParsedFeed::new();
            feed.version = FeedVersion::Unknown;
            feed.bozo = true;
            feed.bozo_exception = Some("Feed format not recognized".to_string());
            Ok(feed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_returns_ok_bozo_for_garbage() {
        let feed = parse(b"test").unwrap();
        assert!(feed.bozo, "unrecognized input must set bozo");
        assert_eq!(feed.version, crate::types::FeedVersion::Unknown);
        assert!(feed.entries.is_empty());
    }
}
