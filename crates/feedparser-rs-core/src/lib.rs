//! feedparser-rs-core: High-performance RSS/Atom/JSON Feed parser
//!
//! This crate provides a pure Rust implementation of feed parsing with
//! compatibility for Python's feedparser library.
//!
//! # Examples
//!
//! ```
//! use feedparser_rs::parse;
//!
//! let xml = r#"
//!     <?xml version="1.0"?>
//!     <rss version="2.0">
//!         <channel>
//!             <title>Example Feed</title>
//!         </channel>
//!     </rss>
//! "#;
//!
//! // Parsing will be fully implemented in Phase 2
//! let feed = parse(xml.as_bytes()).unwrap();
//! assert!(feed.bozo == false);
//! ```
//!
//! # Features
//!
//! - Parse RSS 0.9x, 1.0, 2.0
//! - Parse Atom 0.3, 1.0
//! - Parse JSON Feed 1.0, 1.1
//! - Tolerant parsing with bozo flag
//! - Multi-format date parsing
//! - HTML sanitization
//! - Encoding detection
//!
//! # Architecture
//!
//! The library provides core data structures like [`ParsedFeed`], [`Entry`], and [`FeedMeta`]
//! for representing parsed feed data. The main entry point is the [`parse`] function which
//! automatically detects feed format and returns parsed results.

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
    Content, Enclosure, Entry, FeedMeta, FeedVersion, Generator, Image, ItunesCategory,
    ItunesEntryMeta, ItunesFeedMeta, ItunesOwner, LimitedCollectionExt, Link, ParsedFeed, Person,
    PodcastChapters, PodcastEntryMeta, PodcastFunding, PodcastMeta, PodcastPerson,
    PodcastSoundbite, PodcastTranscript, PodcastValue, PodcastValueRecipient, Source, Tag,
    TextConstruct, TextType, parse_duration, parse_explicit,
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
