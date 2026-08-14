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
/// the feed format (RSS, Atom, JSON) and parses accordingly. Uses
/// [`crate::ParseOptions::default`], which sanitizes HTML content and resolves
/// relative URIs.
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
    parse_with_options(data, &crate::ParseOptions::default())
}

/// Parse feed with custom parser limits
///
/// This allows controlling resource usage when parsing untrusted feeds. HTML
/// sanitization and relative URI resolution use their [`crate::ParseOptions::default`]
/// settings (both enabled); use [`parse_with_options`] to control them directly.
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
    parse_with_options(
        data,
        &crate::ParseOptions {
            limits,
            ..crate::ParseOptions::default()
        },
    )
}

/// Parse feed from raw bytes with full control over parser behavior
///
/// This is the most flexible entry point: [`parse`] and [`parse_with_limits`] are
/// thin wrappers around it. Controls HTML sanitization, relative URI resolution,
/// and resource limits via [`crate::ParseOptions`].
///
/// # Examples
///
/// ```
/// use feedparser_rs::{parse_with_options, ParseOptions};
///
/// // Trust the feed source and skip sanitization
/// let options = ParseOptions {
///     sanitize_html: false,
///     ..ParseOptions::default()
/// };
/// let xml = b"<rss version=\"2.0\"><channel><title>Test</title></channel></rss>";
/// let feed = parse_with_options(xml, &options).unwrap();
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - Feed size exceeds limits
/// - Format is unknown or unsupported
/// - Fatal parsing error occurs
pub fn parse_with_options(data: &[u8], options: &crate::ParseOptions) -> Result<ParsedFeed> {
    use crate::types::FeedVersion;
    use crate::util::encoding::detect_and_convert;

    let limits = options.limits;
    let resolve_relative_uris = options.resolve_relative_uris;

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
        // RSS variants (all use RSS 2.0 parser; overwrite version after parsing)
        FeedVersion::Rss20
        | FeedVersion::Rss092
        | FeedVersion::Rss091Netscape
        | FeedVersion::Rss091Userland
        | FeedVersion::Rss090 => {
            let mut parsed =
                rss::parse_rss20_with_options(utf8_bytes, limits, resolve_relative_uris)?;
            parsed.version = version;
            Ok(parsed)
        }

        // Atom variants
        FeedVersion::Atom10 | FeedVersion::Atom03 => {
            atom::parse_atom10_with_options(utf8_bytes, limits, resolve_relative_uris)
        }

        // RSS 1.0 (RDF)
        FeedVersion::Rss10 => {
            rss10::parse_rss10_with_options(utf8_bytes, limits, resolve_relative_uris)
        }

        // JSON Feed: `resolve_relative_uris` is intentionally not threaded through
        // here — the JSON Feed parser does not resolve relative URIs against a
        // base URL at all (JSON Feed has no `xml:base`-equivalent concept), so
        // there is nothing for the option to gate.
        FeedVersion::JsonFeed10 | FeedVersion::JsonFeed11 => {
            json::parse_json_feed_with_limits(utf8_bytes, limits)
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
    }?;

    feed.encoding = encoding_label;

    if options.sanitize_html {
        crate::util::sanitize::sanitize_feed(&mut feed, &limits);
    }

    Ok(feed)
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

    #[test]
    fn test_rss091n_version_string() {
        // #283: RSS 0.91 with Netscape DOCTYPE must report "rss091n"
        let xml = br#"<?xml version="1.0"?>
<!DOCTYPE rss PUBLIC "-//Netscape Communications//DTD RSS 0.91//EN"
    "http://my.netscape.com/publish/formats/rss-0.91.dtd">
<rss version="0.91">
<channel><title>T</title><link>http://example.com</link><description>D</description>
<language>en</language></channel></rss>"#;
        let feed = parse(xml).unwrap();
        assert_eq!(feed.version.as_str(), "rss091n");
    }

    #[test]
    fn test_rss091u_version_string() {
        // #283: RSS 0.91 without Netscape DOCTYPE must report "rss091u"
        let xml = br#"<?xml version="1.0"?>
<rss version="0.91">
<channel><title>T</title><link>http://example.com</link><description>D</description>
<language>en</language></channel></rss>"#;
        let feed = parse(xml).unwrap();
        assert_eq!(feed.version.as_str(), "rss091u");
    }

    #[test]
    fn test_rss092_version_string() {
        // #283: RSS 0.92 feeds must report version "rss092", not "rss20"
        let xml = br#"<?xml version="1.0"?>
<rss version="0.92">
<channel><title>T</title><link>http://example.com</link><description>D</description>
</channel></rss>"#;
        let feed = parse(xml).unwrap();
        assert_eq!(feed.version.as_str(), "rss092");
    }

    #[test]
    fn test_rss20_version_string_unchanged() {
        // #283: RSS 2.0 feeds must still report version "rss20"
        let xml = br#"<?xml version="1.0"?>
<rss version="2.0">
<channel><title>T</title><link>http://example.com</link><description>D</description>
</channel></rss>"#;
        let feed = parse(xml).unwrap();
        assert_eq!(feed.version.as_str(), "rss20");
    }
}
