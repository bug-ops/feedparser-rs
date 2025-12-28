use pyo3::prelude::*;
use pyo3::types::PyModule;

use feedparser_rs as core;

mod error;
mod limits;
mod types;

use error::convert_feed_error;
use limits::PyParserLimits;
use types::PyParsedFeed;

#[pymodule]
fn _feedparser_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(parse_with_limits, m)?)?;
    #[cfg(feature = "http")]
    m.add_function(wrap_pyfunction!(parse_url, m)?)?;
    #[cfg(feature = "http")]
    m.add_function(wrap_pyfunction!(parse_url_with_limits, m)?)?;
    m.add_function(wrap_pyfunction!(detect_format, m)?)?;
    m.add_class::<PyParsedFeed>()?;
    m.add_class::<PyParserLimits>()?;
    m.add_class::<types::geo::PyGeoLocation>()?;
    m.add_class::<types::media::PyMediaThumbnail>()?;
    m.add_class::<types::media::PyMediaContent>()?;
    m.add_class::<types::podcast::PyItunesFeedMeta>()?;
    m.add_class::<types::podcast::PyItunesEntryMeta>()?;
    m.add_class::<types::podcast::PyItunesOwner>()?;
    m.add_class::<types::podcast::PyItunesCategory>()?;
    m.add_class::<types::podcast::PyPodcastMeta>()?;
    m.add_class::<types::podcast::PyPodcastTranscript>()?;
    m.add_class::<types::podcast::PyPodcastFunding>()?;
    m.add_class::<types::podcast::PyPodcastPerson>()?;
    m.add_class::<types::podcast::PyPodcastChapters>()?;
    m.add_class::<types::podcast::PyPodcastSoundbite>()?;
    m.add_class::<types::podcast::PyPodcastEntryMeta>()?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}

/// Parse an RSS/Atom/JSON Feed from bytes, string, or URL
///
/// Automatically detects whether `source` is a URL (http://, https://) or content.
/// For URLs, fetches and parses the feed. For content, parses directly.
///
/// # Arguments
///
/// * `source` - URL string, feed content string, or bytes
/// * `etag` - Optional ETag from previous fetch (for URLs with conditional GET)
/// * `modified` - Optional Last-Modified timestamp (for URLs with conditional GET)
/// * `user_agent` - Optional custom User-Agent header (for URLs)
///
/// # Examples
///
/// ```python
/// import feedparser_rs
///
/// # Parse from URL (auto-detected)
/// feed = feedparser_rs.parse("https://example.com/feed.xml")
///
/// # Parse from content
/// feed = feedparser_rs.parse("<rss>...</rss>")
///
/// # Parse from URL with caching
/// feed = feedparser_rs.parse(
///     "https://example.com/feed.xml",
///     etag=cached_etag,
///     modified=cached_modified
/// )
/// ```
#[pyfunction]
#[pyo3(signature = (source, /, etag=None, modified=None, user_agent=None))]
fn parse(
    py: Python<'_>,
    source: &Bound<'_, PyAny>,
    etag: Option<&str>,
    modified: Option<&str>,
    user_agent: Option<&str>,
) -> PyResult<PyParsedFeed> {
    parse_internal(py, source, etag, modified, user_agent, None)
}

/// Parse with custom resource limits for DoS protection
///
/// Like `parse()` but allows specifying custom limits for untrusted feeds.
///
/// # Arguments
///
/// * `source` - URL string, feed content string, or bytes
/// * `etag` - Optional ETag from previous fetch (for URLs)
/// * `modified` - Optional Last-Modified timestamp (for URLs)
/// * `user_agent` - Optional custom User-Agent header (for URLs)
/// * `limits` - Optional parser limits for DoS protection
///
/// # Examples
///
/// ```python
/// import feedparser_rs
///
/// limits = feedparser_rs.ParserLimits.strict()
///
/// # Parse from URL with limits
/// feed = feedparser_rs.parse_with_limits(
///     "https://example.com/feed.xml",
///     limits=limits
/// )
///
/// # Parse from content with limits
/// feed = feedparser_rs.parse_with_limits("<rss>...</rss>", limits=limits)
/// ```
#[pyfunction]
#[pyo3(signature = (source, /, etag=None, modified=None, user_agent=None, limits=None))]
fn parse_with_limits(
    py: Python<'_>,
    source: &Bound<'_, PyAny>,
    etag: Option<&str>,
    modified: Option<&str>,
    user_agent: Option<&str>,
    limits: Option<&PyParserLimits>,
) -> PyResult<PyParsedFeed> {
    parse_internal(py, source, etag, modified, user_agent, limits)
}

/// Internal parse function that handles both URL and content sources
fn parse_internal(
    py: Python<'_>,
    source: &Bound<'_, PyAny>,
    etag: Option<&str>,
    modified: Option<&str>,
    user_agent: Option<&str>,
    limits: Option<&PyParserLimits>,
) -> PyResult<PyParsedFeed> {
    // Try to extract as string first
    if let Ok(s) = source.extract::<String>() {
        // Check if it's a URL
        if s.starts_with("http://") || s.starts_with("https://") {
            // Handle URL - requires http feature
            #[cfg(feature = "http")]
            {
                let parser_limits = limits.map(|l| l.to_core_limits()).unwrap_or_default();
                let parsed =
                    core::parse_url_with_limits(&s, etag, modified, user_agent, parser_limits)
                        .map_err(convert_feed_error)?;
                return PyParsedFeed::from_core(py, parsed);
            }
            #[cfg(not(feature = "http"))]
            {
                return Err(pyo3::exceptions::PyNotImplementedError::new_err(
                    "URL fetching requires the 'http' feature. Build with: maturin develop --features http",
                ));
            }
        }

        // Parse as content
        let parser_limits = limits.map(|l| l.to_core_limits()).unwrap_or_default();
        let parsed =
            core::parse_with_limits(s.as_bytes(), parser_limits).map_err(convert_feed_error)?;
        return PyParsedFeed::from_core(py, parsed);
    }

    // Try to extract as bytes
    if let Ok(b) = source.extract::<Vec<u8>>() {
        let parser_limits = limits.map(|l| l.to_core_limits()).unwrap_or_default();
        let parsed = core::parse_with_limits(&b, parser_limits).map_err(convert_feed_error)?;
        return PyParsedFeed::from_core(py, parsed);
    }

    Err(pyo3::exceptions::PyTypeError::new_err(
        "source must be str, bytes, or URL",
    ))
}

/// Detect feed format without full parsing
#[pyfunction]
#[pyo3(signature = (source, /))]
fn detect_format(source: &Bound<'_, PyAny>) -> PyResult<String> {
    let bytes: Vec<u8> = if let Ok(s) = source.extract::<String>() {
        s.into_bytes()
    } else if let Ok(b) = source.extract::<Vec<u8>>() {
        b
    } else {
        return Err(pyo3::exceptions::PyTypeError::new_err(
            "source must be str or bytes",
        ));
    };
    Ok(core::detect_format(&bytes).to_string())
}

/// Parse feed from HTTP/HTTPS URL with conditional GET support
///
/// Fetches the feed from the given URL and parses it. Supports conditional GET
/// using ETag and Last-Modified headers for bandwidth-efficient caching.
///
/// # Arguments
///
/// * `url` - HTTP or HTTPS URL to fetch
/// * `etag` - Optional ETag from previous fetch for conditional GET
/// * `modified` - Optional Last-Modified timestamp from previous fetch
/// * `user_agent` - Optional custom User-Agent header
///
/// # Returns
///
/// Returns a `FeedParserDict` with HTTP metadata fields populated:
/// - `status`: HTTP status code (200, 304, etc.)
/// - `href`: Final URL after redirects
/// - `etag`: ETag header value (for next request)
/// - `modified`: Last-Modified header value (for next request)
/// - `headers`: Full HTTP response headers
///
/// On 304 Not Modified, returns a feed with empty entries but status=304.
///
/// # Examples
///
/// ```python
/// import feedparser_rs
///
/// # First fetch
/// feed = feedparser_rs.parse_url("https://example.com/feed.xml")
/// print(feed.feed.title)
/// print(f"ETag: {feed.etag}")
///
/// # Subsequent fetch with caching
/// feed2 = feedparser_rs.parse_url(
///     "https://example.com/feed.xml",
///     etag=feed.etag,
///     modified=feed.modified
/// )
///
/// if feed2.status == 304:
///     print("Feed not modified, use cached version")
/// ```
#[cfg(feature = "http")]
#[pyfunction]
#[pyo3(signature = (url, etag=None, modified=None, user_agent=None))]
fn parse_url(
    py: Python<'_>,
    url: &str,
    etag: Option<&str>,
    modified: Option<&str>,
    user_agent: Option<&str>,
) -> PyResult<PyParsedFeed> {
    let parsed = core::parse_url(url, etag, modified, user_agent).map_err(convert_feed_error)?;
    PyParsedFeed::from_core(py, parsed)
}

/// Parse feed from URL with custom resource limits
///
/// Like `parse_url` but allows specifying custom limits for DoS protection.
///
/// # Examples
///
/// ```python
/// import feedparser_rs
///
/// limits = feedparser_rs.ParserLimits.strict()
/// feed = feedparser_rs.parse_url_with_limits(
///     "https://example.com/feed.xml",
///     limits=limits
/// )
/// ```
#[cfg(feature = "http")]
#[pyfunction]
#[pyo3(signature = (url, etag=None, modified=None, user_agent=None, limits=None))]
fn parse_url_with_limits(
    py: Python<'_>,
    url: &str,
    etag: Option<&str>,
    modified: Option<&str>,
    user_agent: Option<&str>,
    limits: Option<&PyParserLimits>,
) -> PyResult<PyParsedFeed> {
    let parser_limits = limits.map(|l| l.to_core_limits()).unwrap_or_default();
    let parsed = core::parse_url_with_limits(url, etag, modified, user_agent, parser_limits)
        .map_err(convert_feed_error)?;
    PyParsedFeed::from_core(py, parsed)
}
