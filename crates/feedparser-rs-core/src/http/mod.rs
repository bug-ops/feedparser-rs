/// HTTP client module for fetching feeds from URLs
///
/// This module provides HTTP fetching capabilities with support for:
/// - Conditional GET (`ETag` and `Last-Modified` headers)
/// - Automatic decompression (gzip, deflate, brotli)
/// - Redirect following
/// - Custom User-Agent and headers
///
/// # Examples
///
/// ```no_run
/// use feedparser_rs::http::FeedHttpClient;
///
/// let client = FeedHttpClient::new().unwrap();
/// let response = client.get(
///     "https://example.com/feed.xml",
///     None,  // no ETag
///     None,  // no Last-Modified
///     None,  // no extra headers
/// ).unwrap();
///
/// if response.status == 200 {
///     println!("Fetched {} bytes", response.body.len());
/// }
/// ```
mod client;
/// SSRF-safe DNS resolver, closing the DNS-rebinding gap for `FeedHttpClient`
mod resolver;
mod response;

/// URL validation module for SSRF protection
pub mod validation;

pub use client::FeedHttpClient;
pub use response::FeedHttpResponse;
pub use validation::validate_url;
