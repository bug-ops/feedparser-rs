#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

//! # feedparser-rs: High-performance RSS/Atom/JSON Feed parser
//!
//! A pure Rust implementation of feed parsing with API compatibility for Python's
//! [feedparser](https://github.com/kurtmckee/feedparser) library. Designed for
//! 10-100x faster feed parsing with identical behavior.
//!
//! ## Quick Start
//!
//! ```
//! use feedparser_rs::parse;
//!
//! let xml = r#"
//!     <?xml version="1.0"?>
//!     <rss version="2.0">
//!         <channel>
//!             <title>Example Feed</title>
//!             <link>https://example.com</link>
//!             <item>
//!                 <title>First Post</title>
//!                 <link>https://example.com/post/1</link>
//!             </item>
//!         </channel>
//!     </rss>
//! "#;
//!
//! let feed = parse(xml.as_bytes()).unwrap();
//! assert!(!feed.bozo);
//! assert_eq!(feed.feed.title.as_deref(), Some("Example Feed"));
//! assert_eq!(feed.entries.len(), 1);
//! ```
//!
//! ## Supported Formats
//!
//! | Format | Versions | Detection |
//! |--------|----------|-----------|
//! | RSS | 0.90, 0.91, 0.92, 2.0 | `<rss>` element |
//! | RSS 1.0 | RDF-based | `<rdf:RDF>` with RSS namespace |
//! | Atom | 0.3, 1.0 | `<feed>` with Atom namespace |
//! | JSON Feed | 1.0, 1.1 | `version` field starting with `https://jsonfeed.org` |
//!
//! ## Namespace Extensions
//!
//! The parser supports common feed extensions:
//!
//! - **iTunes/Podcast** (`itunes:`) - Podcast metadata, categories, explicit flags
//! - **Podcast 2.0** (`podcast:`) - Transcripts, chapters, funding, persons
//! - **Dublin Core** (`dc:`) - Creator, date, rights, subject
//! - **Media RSS** (`media:`) - Thumbnails, content, descriptions
//! - **Content** (`content:encoded`) - Full HTML content
//! - **Syndication** (`sy:`) - Update frequency hints
//! - **`GeoRSS`** (`georss:`) - Geographic coordinates
//! - **Creative Commons** (`cc:`, `creativeCommons:`) - License information
//!
//! ## Type-Safe URL and MIME Handling
//!
//! The library uses semantic newtypes for improved type safety:
//!
//! ```
//! use feedparser_rs::{Url, MimeType, Email};
//!
//! // Url - wraps URL strings without validation (bozo-compatible)
//! let url = Url::new("https://example.com/feed.xml");
//! assert_eq!(url.as_str(), "https://example.com/feed.xml");
//! assert!(url.starts_with("https://")); // Deref to str
//!
//! // MimeType - uses Arc<str> for efficient cloning
//! let mime = MimeType::new("application/rss+xml");
//! let clone = mime.clone(); // Cheap: just increments refcount
//!
//! // Email - wraps email addresses
//! let email = Email::new("author@example.com");
//! ```
//!
//! These types implement <code>[`Deref`](std::ops::Deref)&lt;Target=str&gt;</code>, so string methods work directly:
//!
//! ```
//! use feedparser_rs::Url;
//!
//! let url = Url::new("https://example.com/path?query=1");
//! assert!(url.contains("example.com"));
//! assert_eq!(url.len(), 32);
//! ```
//!
//! ## The Bozo Pattern
//!
//! Following Python feedparser's philosophy, this library **never panics** on
//! malformed input. Instead, it sets the `bozo` flag and continues parsing:
//!
//! ```
//! use feedparser_rs::parse;
//!
//! // XML with undefined entity - triggers bozo
//! let xml_with_entity = b"<rss version='2.0'><channel><title>Test &#xFFFF;</title></channel></rss>";
//!
//! let feed = parse(xml_with_entity).unwrap();
//! // Parser handles invalid characters gracefully
//! assert!(feed.feed.title.is_some());
//! ```
//!
//! The bozo flag indicates the feed had issues but was still parseable.
//!
//! ## Resource Limits
//!
//! Protect against malicious feeds with [`ParserLimits`]:
//!
//! ```
//! use feedparser_rs::{parse_with_limits, ParserLimits};
//!
//! // Customize limits for untrusted input
//! let limits = ParserLimits {
//!     max_entries: 100,
//!     max_text_length: 50_000,
//!     ..Default::default()
//! };
//!
//! let xml = b"<rss version='2.0'><channel><title>Safe</title></channel></rss>";
//! let feed = parse_with_limits(xml, limits).unwrap();
//! ```
//!
//! ## HTTP Fetching
//!
//! With the `http` feature (enabled by default), fetch feeds from URLs:
//!
//! ```no_run
//! use feedparser_rs::parse_url;
//!
//! // Simple fetch
//! let feed = parse_url("https://example.com/feed.xml", None, None, None)?;
//!
//! // With conditional GET for caching
//! let feed2 = parse_url(
//!     "https://example.com/feed.xml",
//!     feed.etag.as_deref(),      // ETag from previous fetch
//!     feed.modified.as_deref(),  // Last-Modified from previous fetch
//!     Some("MyApp/1.0"),         // Custom User-Agent
//! )?;
//!
//! if feed2.status == Some(304) {
//!     println!("Feed not modified since last fetch");
//! }
//! # Ok::<(), feedparser_rs::FeedError>(())
//! ```
//!
//! ## Core Types
//!
//! - [`ParsedFeed`] - Complete parsed feed with metadata and entries
//! - [`FeedMeta`] - Feed-level metadata (title, link, author, etc.)
//! - [`Entry`] - Individual feed entry/item
//! - [`Link`], [`Person`], [`Tag`] - Common feed elements
//! - [`Url`], [`MimeType`], [`Email`] - Type-safe string wrappers
//!
//! ## Module Structure
//!
//! - [`types`] - All data structures for parsed feeds
//! - [`namespace`] - Handlers for namespace extensions (iTunes, Podcast 2.0, etc.)
//! - [`util`] - Helper functions for dates, HTML sanitization, encoding
//! - [`compat`] - Python feedparser API compatibility layer
//! - [`http`] - HTTP client for fetching feeds (requires `http` feature)

/// Compatibility utilities for Python feedparser API
pub mod compat;
mod error;
#[cfg(feature = "http")]
/// HTTP client module for fetching feeds from URLs
pub mod http;
mod limits;
/// Namespace handlers for extended feed formats
pub mod namespace;
mod options;
mod parser;

/// Type definitions for feed data structures
///
/// This module contains all the data types used to represent parsed feeds,
/// including the main `ParsedFeed` struct and related types.
pub mod types;

/// Utility functions for feed parsing
///
/// This module provides helper functions for date parsing, HTML sanitization,
/// and encoding detection that are useful for feed processing.
pub mod util;

pub use error::{FeedError, Result};
pub use limits::{LimitError, ParserLimits};
pub use options::ParseOptions;
pub use parser::{detect_format, parse, parse_with_limits};
pub use types::{
    Content, Email, Enclosure, Entry, FeedMeta, FeedVersion, Generator, Image, ItunesCategory,
    ItunesEntryMeta, ItunesFeedMeta, ItunesOwner, LimitedCollectionExt, Link, MediaContent,
    MediaThumbnail, MimeType, ParsedFeed, Person, PodcastChapters, PodcastEntryMeta,
    PodcastFunding, PodcastMeta, PodcastPerson, PodcastSoundbite, PodcastTranscript, PodcastValue,
    PodcastValueRecipient, Source, Tag, TextConstruct, TextType, Url, parse_duration,
    parse_explicit,
};

pub use namespace::syndication::{SyndicationMeta, UpdatePeriod};

#[cfg(feature = "http")]
pub use http::{FeedHttpClient, FeedHttpResponse};

/// Parse feed from HTTP/HTTPS URL
///
/// Fetches the feed from the given URL and parses it. Supports conditional GET
/// using `ETag` and `Last-Modified` headers for bandwidth-efficient caching.
///
/// # Arguments
///
/// * `url` - HTTP or HTTPS URL to fetch
/// * `etag` - Optional `ETag` from previous fetch for conditional GET
/// * `modified` - Optional `Last-Modified` timestamp from previous fetch
/// * `user_agent` - Optional custom User-Agent header
///
/// # Returns
///
/// Returns a `ParsedFeed` with HTTP metadata fields populated:
/// - `status`: HTTP status code (200, 304, etc.)
/// - `href`: Final URL after redirects
/// - `etag`: `ETag` header value (for next request)
/// - `modified`: `Last-Modified` header value (for next request)
/// - `headers`: Full HTTP response headers
///
/// On 304 Not Modified, returns a feed with empty entries but status=304.
///
/// # Errors
///
/// Returns `FeedError::Http` if:
/// - Network error occurs
/// - URL is invalid
/// - HTTP status is 4xx or 5xx (except 304)
///
/// # Examples
///
/// ```no_run
/// use feedparser_rs::parse_url;
///
/// // First fetch
/// let feed = parse_url("https://example.com/feed.xml", None, None, None).unwrap();
/// println!("Title: {:?}", feed.feed.title);
/// println!("ETag: {:?}", feed.etag);
///
/// // Subsequent fetch with caching
/// let feed2 = parse_url(
///     "https://example.com/feed.xml",
///     feed.etag.as_deref(),
///     feed.modified.as_deref(),
///     None
/// ).unwrap();
///
/// if feed2.status == Some(304) {
///     println!("Feed not modified, use cached version");
/// }
/// ```
#[cfg(feature = "http")]
pub fn parse_url(
    url: &str,
    etag: Option<&str>,
    modified: Option<&str>,
    user_agent: Option<&str>,
) -> Result<ParsedFeed> {
    use http::FeedHttpClient;

    // Create HTTP client
    let mut client = FeedHttpClient::new()?;
    if let Some(agent) = user_agent {
        client = client.with_user_agent(agent.to_string());
    }

    // Fetch feed
    let response = client.get(url, etag, modified, None)?;

    // Handle 304 Not Modified
    if response.status == 304 {
        return Ok(ParsedFeed {
            status: Some(304),
            href: Some(response.url),
            etag: etag.map(String::from),
            modified: modified.map(String::from),
            #[cfg(feature = "http")]
            headers: Some(response.headers),
            encoding: String::from("utf-8"),
            ..Default::default()
        });
    }

    // Handle error status codes
    if response.status >= 400 {
        return Err(FeedError::Http {
            message: format!("HTTP {} for URL: {}", response.status, response.url),
        });
    }

    // Parse feed from response body
    let mut feed = parse(&response.body)?;

    // Add HTTP metadata
    feed.status = Some(response.status);
    feed.href = Some(response.url);
    feed.etag = response.etag;
    feed.modified = response.last_modified;
    #[cfg(feature = "http")]
    {
        feed.headers = Some(response.headers);
    }

    // Override encoding if HTTP header specifies
    if let Some(http_encoding) = response.encoding {
        feed.encoding = http_encoding;
    }

    Ok(feed)
}

/// Parse feed from URL with custom parser limits
///
/// Like `parse_url` but allows specifying custom limits for resource control.
///
/// # Errors
///
/// Returns `FeedError::Http` if the request fails or `FeedError::Parse` if parsing fails.
///
/// # Examples
///
/// ```no_run
/// use feedparser_rs::{parse_url_with_limits, ParserLimits};
///
/// let limits = ParserLimits::strict();
/// let feed = parse_url_with_limits(
///     "https://example.com/feed.xml",
///     None,
///     None,
///     None,
///     limits
/// ).unwrap();
/// ```
#[cfg(feature = "http")]
pub fn parse_url_with_limits(
    url: &str,
    etag: Option<&str>,
    modified: Option<&str>,
    user_agent: Option<&str>,
    limits: ParserLimits,
) -> Result<ParsedFeed> {
    use http::FeedHttpClient;

    let mut client = FeedHttpClient::new()?;
    if let Some(agent) = user_agent {
        client = client.with_user_agent(agent.to_string());
    }

    let response = client.get(url, etag, modified, None)?;

    if response.status == 304 {
        return Ok(ParsedFeed {
            status: Some(304),
            href: Some(response.url),
            etag: etag.map(String::from),
            modified: modified.map(String::from),
            #[cfg(feature = "http")]
            headers: Some(response.headers),
            encoding: String::from("utf-8"),
            ..Default::default()
        });
    }

    if response.status >= 400 {
        return Err(FeedError::Http {
            message: format!("HTTP {} for URL: {}", response.status, response.url),
        });
    }

    let mut feed = parse_with_limits(&response.body, limits)?;

    feed.status = Some(response.status);
    feed.href = Some(response.url);
    feed.etag = response.etag;
    feed.modified = response.last_modified;
    #[cfg(feature = "http")]
    {
        feed.headers = Some(response.headers);
    }

    if let Some(http_encoding) = response.encoding {
        feed.encoding = http_encoding;
    }

    Ok(feed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let xml = r#"
            <?xml version="1.0"?>
            <rss version="2.0">
                <channel>
                    <title>Test</title>
                </channel>
            </rss>
        "#;

        let result = parse(xml.as_bytes());
        assert!(result.is_ok());
    }

    #[test]
    fn test_parsed_feed_new() {
        let feed = ParsedFeed::new();
        assert_eq!(feed.encoding, "utf-8");
        assert!(!feed.bozo);
        assert_eq!(feed.version, FeedVersion::Unknown);
    }

    #[test]
    fn test_feed_version_display() {
        assert_eq!(FeedVersion::Rss20.to_string(), "rss20");
        assert_eq!(FeedVersion::Atom10.to_string(), "atom10");
    }
}
