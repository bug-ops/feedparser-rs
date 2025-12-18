#![deny(clippy::all)]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;

use feedparser_rs::{
    self as core, Content as CoreContent, Enclosure as CoreEnclosure, Entry as CoreEntry,
    FeedMeta as CoreFeedMeta, Generator as CoreGenerator, Image as CoreImage, Link as CoreLink,
    ParsedFeed as CoreParsedFeed, ParserLimits, Person as CorePerson,
    PodcastPerson as CorePodcastPerson, PodcastTranscript as CorePodcastTranscript,
    Source as CoreSource, Tag as CoreTag, TextConstruct as CoreTextConstruct, TextType,
};

/// Default maximum feed size (100 MB) - prevents DoS attacks
const DEFAULT_MAX_FEED_SIZE: usize = 100 * 1024 * 1024;

/// Parse an RSS/Atom/JSON Feed from bytes or string
///
/// # Arguments
///
/// * `source` - Feed content as Buffer, string, or Uint8Array
///
/// # Returns
///
/// Parsed feed result with metadata and entries
///
/// # Errors
///
/// Returns error if input exceeds size limit or parsing fails catastrophically
#[napi]
pub fn parse(source: Either<Buffer, String>) -> Result<ParsedFeed> {
    parse_with_options(source, None)
}

/// Parse an RSS/Atom/JSON Feed with custom size limit
///
/// # Arguments
///
/// * `source` - Feed content as Buffer, string, or Uint8Array
/// * `max_size` - Optional maximum feed size in bytes (default: 100MB)
///
/// # Returns
///
/// Parsed feed result with metadata and entries
///
/// # Errors
///
/// Returns error if input exceeds size limit or parsing fails catastrophically
#[napi]
pub fn parse_with_options(
    source: Either<Buffer, String>,
    max_size: Option<u32>,
) -> Result<ParsedFeed> {
    let max_feed_size = max_size.map_or(DEFAULT_MAX_FEED_SIZE, |s| s as usize);

    // Validate input size BEFORE copying to prevent DoS (CWE-770)
    let input_len = match &source {
        Either::A(buf) => buf.len(),
        Either::B(s) => s.len(),
    };

    if input_len > max_feed_size {
        return Err(Error::from_reason(format!(
            "Feed size ({} bytes) exceeds maximum allowed ({} bytes)",
            input_len, max_feed_size
        )));
    }

    let bytes: &[u8] = match &source {
        Either::A(buf) => buf.as_ref(),
        Either::B(s) => s.as_bytes(),
    };

    let limits = ParserLimits {
        max_feed_size_bytes: max_feed_size,
        ..ParserLimits::default()
    };

    let parsed = core::parse_with_limits(bytes, limits)
        .map_err(|e| Error::from_reason(format!("Parse error: {}", e)))?;

    Ok(ParsedFeed::from(parsed))
}

/// Detect feed format without full parsing
///
/// # Arguments
///
/// * `source` - Feed content as Buffer, string, or Uint8Array
///
/// # Returns
///
/// Feed version string (e.g., "rss20", "atom10")
#[napi]
pub fn detect_format(source: Either<Buffer, String>) -> String {
    let bytes: &[u8] = match &source {
        Either::A(buf) => buf.as_ref(),
        Either::B(s) => s.as_bytes(),
    };

    let version = core::detect_format(bytes);

    version.to_string()
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
/// Parsed feed result with HTTP metadata fields populated:
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
/// ```javascript
/// const feedparser = require('feedparser-rs');
///
/// // First fetch
/// const feed = await feedparser.parseUrl("https://example.com/feed.xml");
/// console.log(feed.feed.title);
/// console.log(`ETag: ${feed.etag}`);
///
/// // Subsequent fetch with caching
/// const feed2 = await feedparser.parseUrl(
///   "https://example.com/feed.xml",
///   feed.etag,
///   feed.modified
/// );
///
/// if (feed2.status === 304) {
///   console.log("Feed not modified, use cached version");
/// }
/// ```
#[cfg(feature = "http")]
#[napi]
pub fn parse_url(
    url: String,
    etag: Option<String>,
    modified: Option<String>,
    user_agent: Option<String>,
) -> Result<ParsedFeed> {
    let parsed = core::parse_url(
        &url,
        etag.as_deref(),
        modified.as_deref(),
        user_agent.as_deref(),
    )
    .map_err(|e| Error::from_reason(format!("HTTP error: {}", e)))?;

    Ok(ParsedFeed::from(parsed))
}

/// Parse feed from URL with custom resource limits
///
/// Like `parseUrl` but allows specifying custom limits for DoS protection.
///
/// # Examples
///
/// ```javascript
/// const feedparser = require('feedparser-rs');
///
/// const feed = await feedparser.parseUrlWithOptions(
///   "https://example.com/feed.xml",
///   null, // etag
///   null, // modified
///   null, // user_agent
///   10485760 // max_size: 10MB
/// );
/// ```
#[cfg(feature = "http")]
#[napi]
pub fn parse_url_with_options(
    url: String,
    etag: Option<String>,
    modified: Option<String>,
    user_agent: Option<String>,
    max_size: Option<u32>,
) -> Result<ParsedFeed> {
    let max_feed_size = max_size.map_or(DEFAULT_MAX_FEED_SIZE, |s| s as usize);

    let limits = ParserLimits {
        max_feed_size_bytes: max_feed_size,
        ..ParserLimits::default()
    };

    let parsed = core::parse_url_with_limits(
        &url,
        etag.as_deref(),
        modified.as_deref(),
        user_agent.as_deref(),
        limits,
    )
    .map_err(|e| Error::from_reason(format!("HTTP error: {}", e)))?;

    Ok(ParsedFeed::from(parsed))
}

/// Parsed feed result
///
/// This is analogous to Python feedparser's `FeedParserDict`.
#[napi(object)]
pub struct ParsedFeed {
    /// Feed metadata
    pub feed: FeedMeta,
    /// Feed entries/items
    pub entries: Vec<Entry>,
    /// True if parsing encountered errors
    pub bozo: bool,
    /// Description of parsing error (if bozo is true)
    pub bozo_exception: Option<String>,
    /// Detected or declared encoding
    pub encoding: String,
    /// Detected feed format version
    pub version: String,
    /// XML namespaces (prefix -> URI)
    pub namespaces: HashMap<String, String>,
    /// HTTP status code (if fetched from URL)
    pub status: Option<u32>,
    /// Final URL after redirects (if fetched from URL)
    pub href: Option<String>,
    /// ETag header from HTTP response
    pub etag: Option<String>,
    /// Last-Modified header from HTTP response
    pub modified: Option<String>,
    /// HTTP response headers (if fetched from URL)
    #[cfg(feature = "http")]
    pub headers: Option<HashMap<String, String>>,
}

impl From<CoreParsedFeed> for ParsedFeed {
    fn from(core: CoreParsedFeed) -> Self {
        Self {
            feed: FeedMeta::from(core.feed),
            entries: core.entries.into_iter().map(Entry::from).collect(),
            bozo: core.bozo,
            bozo_exception: core.bozo_exception,
            encoding: core.encoding,
            version: core.version.to_string(),
            namespaces: core.namespaces,
            status: core.status.map(|s| s as u32),
            href: core.href,
            etag: core.etag,
            modified: core.modified,
            #[cfg(feature = "http")]
            headers: core.headers,
        }
    }
}

/// Feed metadata
#[napi(object)]
pub struct FeedMeta {
    /// Feed title
    pub title: Option<String>,
    /// Detailed title with metadata
    pub title_detail: Option<TextConstruct>,
    /// Primary feed link
    pub link: Option<String>,
    /// All links associated with this feed
    pub links: Vec<Link>,
    /// Feed subtitle/description
    pub subtitle: Option<String>,
    /// Detailed subtitle with metadata
    pub subtitle_detail: Option<TextConstruct>,
    /// Last update date (milliseconds since epoch)
    pub updated: Option<i64>,
    /// Initial publication date (milliseconds since epoch)
    pub published: Option<i64>,
    /// Primary author name
    pub author: Option<String>,
    /// Detailed author information
    pub author_detail: Option<Person>,
    /// All authors
    pub authors: Vec<Person>,
    /// Contributors
    pub contributors: Vec<Person>,
    /// Publisher name
    pub publisher: Option<String>,
    /// Detailed publisher information
    pub publisher_detail: Option<Person>,
    /// Feed language (e.g., "en-us")
    pub language: Option<String>,
    /// Copyright/rights statement
    pub rights: Option<String>,
    /// Detailed rights with metadata
    pub rights_detail: Option<TextConstruct>,
    /// Generator name
    pub generator: Option<String>,
    /// Detailed generator information
    pub generator_detail: Option<Generator>,
    /// Feed image
    pub image: Option<Image>,
    /// Icon URL (small image)
    pub icon: Option<String>,
    /// Logo URL (larger image)
    pub logo: Option<String>,
    /// Feed-level tags/categories
    pub tags: Vec<Tag>,
    /// Unique feed identifier
    pub id: Option<String>,
    /// Time-to-live (update frequency hint) in minutes
    pub ttl: Option<u32>,
    /// License URL (Creative Commons, etc.)
    pub license: Option<String>,
}

impl From<CoreFeedMeta> for FeedMeta {
    fn from(core: CoreFeedMeta) -> Self {
        Self {
            title: core.title,
            title_detail: core.title_detail.map(TextConstruct::from),
            link: core.link,
            links: core.links.into_iter().map(Link::from).collect(),
            subtitle: core.subtitle,
            subtitle_detail: core.subtitle_detail.map(TextConstruct::from),
            updated: core.updated.map(|dt| dt.timestamp_millis()),
            published: core.published.map(|dt| dt.timestamp_millis()),
            author: core.author,
            author_detail: core.author_detail.map(Person::from),
            authors: core.authors.into_iter().map(Person::from).collect(),
            contributors: core.contributors.into_iter().map(Person::from).collect(),
            publisher: core.publisher,
            publisher_detail: core.publisher_detail.map(Person::from),
            language: core.language,
            rights: core.rights,
            rights_detail: core.rights_detail.map(TextConstruct::from),
            generator: core.generator,
            generator_detail: core.generator_detail.map(Generator::from),
            image: core.image.map(Image::from),
            icon: core.icon,
            logo: core.logo,
            tags: core.tags.into_iter().map(Tag::from).collect(),
            id: core.id,
            ttl: core.ttl,
            license: core.license,
        }
    }
}

/// Feed entry/item
#[napi(object)]
pub struct Entry {
    /// Unique entry identifier
    pub id: Option<String>,
    /// Entry title
    pub title: Option<String>,
    /// Detailed title with metadata
    pub title_detail: Option<TextConstruct>,
    /// Primary link
    pub link: Option<String>,
    /// All links associated with this entry
    pub links: Vec<Link>,
    /// Short description/summary
    pub summary: Option<String>,
    /// Detailed summary with metadata
    pub summary_detail: Option<TextConstruct>,
    /// Full content blocks
    pub content: Vec<Content>,
    /// Publication date (milliseconds since epoch)
    pub published: Option<i64>,
    /// Last update date (milliseconds since epoch)
    pub updated: Option<i64>,
    /// Creation date (milliseconds since epoch)
    pub created: Option<i64>,
    /// Expiration date (milliseconds since epoch)
    pub expired: Option<i64>,
    /// Primary author name
    pub author: Option<String>,
    /// Detailed author information
    pub author_detail: Option<Person>,
    /// All authors
    pub authors: Vec<Person>,
    /// Contributors
    pub contributors: Vec<Person>,
    /// Publisher name
    pub publisher: Option<String>,
    /// Detailed publisher information
    pub publisher_detail: Option<Person>,
    /// Tags/categories
    pub tags: Vec<Tag>,
    /// Media enclosures (audio, video, etc.)
    pub enclosures: Vec<Enclosure>,
    /// Comments URL or text
    pub comments: Option<String>,
    /// Source feed reference
    pub source: Option<Source>,
    /// Podcast transcripts
    pub podcast_transcripts: Vec<PodcastTranscript>,
    /// Podcast persons
    pub podcast_persons: Vec<PodcastPerson>,
    /// License URL (Creative Commons, etc.)
    pub license: Option<String>,
}

impl From<CoreEntry> for Entry {
    fn from(core: CoreEntry) -> Self {
        // Pre-allocate Vec capacity to avoid reallocations
        let links_cap = core.links.len();
        let content_cap = core.content.len();
        let authors_cap = core.authors.len();
        let contributors_cap = core.contributors.len();
        let tags_cap = core.tags.len();
        let enclosures_cap = core.enclosures.len();
        let transcripts_cap = core.podcast_transcripts.len();
        let persons_cap = core.podcast_persons.len();

        Self {
            id: core.id,
            title: core.title,
            title_detail: core.title_detail.map(TextConstruct::from),
            link: core.link,
            links: {
                let mut v = Vec::with_capacity(links_cap);
                v.extend(core.links.into_iter().map(Link::from));
                v
            },
            summary: core.summary,
            summary_detail: core.summary_detail.map(TextConstruct::from),
            content: {
                let mut v = Vec::with_capacity(content_cap);
                v.extend(core.content.into_iter().map(Content::from));
                v
            },
            published: core.published.map(|dt| dt.timestamp_millis()),
            updated: core.updated.map(|dt| dt.timestamp_millis()),
            created: core.created.map(|dt| dt.timestamp_millis()),
            expired: core.expired.map(|dt| dt.timestamp_millis()),
            author: core.author,
            author_detail: core.author_detail.map(Person::from),
            authors: {
                let mut v = Vec::with_capacity(authors_cap);
                v.extend(core.authors.into_iter().map(Person::from));
                v
            },
            contributors: {
                let mut v = Vec::with_capacity(contributors_cap);
                v.extend(core.contributors.into_iter().map(Person::from));
                v
            },
            publisher: core.publisher,
            publisher_detail: core.publisher_detail.map(Person::from),
            tags: {
                let mut v = Vec::with_capacity(tags_cap);
                v.extend(core.tags.into_iter().map(Tag::from));
                v
            },
            enclosures: {
                let mut v = Vec::with_capacity(enclosures_cap);
                v.extend(core.enclosures.into_iter().map(Enclosure::from));
                v
            },
            comments: core.comments,
            source: core.source.map(Source::from),
            podcast_transcripts: {
                let mut v = Vec::with_capacity(transcripts_cap);
                v.extend(
                    core.podcast_transcripts
                        .into_iter()
                        .map(PodcastTranscript::from),
                );
                v
            },
            podcast_persons: {
                let mut v = Vec::with_capacity(persons_cap);
                v.extend(core.podcast_persons.into_iter().map(PodcastPerson::from));
                v
            },
            license: core.license,
        }
    }
}

/// Text construct with metadata
#[napi(object)]
pub struct TextConstruct {
    /// Text content
    pub value: String,
    /// Content type ("text", "html", "xhtml")
    #[napi(js_name = "type")]
    pub content_type: String,
    /// Content language
    pub language: Option<String>,
    /// Base URL for relative links
    pub base: Option<String>,
}

impl From<CoreTextConstruct> for TextConstruct {
    fn from(core: CoreTextConstruct) -> Self {
        Self {
            value: core.value,
            content_type: match core.content_type {
                TextType::Text => "text".to_string(),
                TextType::Html => "html".to_string(),
                TextType::Xhtml => "xhtml".to_string(),
            },
            language: core.language,
            base: core.base,
        }
    }
}

/// Link in feed or entry
#[napi(object)]
pub struct Link {
    /// Link URL
    pub href: String,
    /// Link relationship type (e.g., "alternate", "enclosure", "self")
    pub rel: Option<String>,
    /// MIME type of the linked resource
    #[napi(js_name = "type")]
    pub link_type: Option<String>,
    /// Human-readable link title
    pub title: Option<String>,
    /// Length of the linked resource in bytes
    pub length: Option<i64>,
    /// Language of the linked resource
    pub hreflang: Option<String>,
}

impl From<CoreLink> for Link {
    fn from(core: CoreLink) -> Self {
        Self {
            href: core.href,
            rel: core.rel,
            link_type: core.link_type,
            title: core.title,
            length: core.length.map(|l| i64::try_from(l).unwrap_or(i64::MAX)),
            hreflang: core.hreflang,
        }
    }
}

/// Person (author, contributor, etc.)
#[napi(object)]
pub struct Person {
    /// Person's name
    pub name: Option<String>,
    /// Person's email address
    pub email: Option<String>,
    /// Person's URI/website
    pub uri: Option<String>,
}

impl From<CorePerson> for Person {
    fn from(core: CorePerson) -> Self {
        Self {
            name: core.name,
            email: core.email,
            uri: core.uri,
        }
    }
}

/// Tag/category
#[napi(object)]
pub struct Tag {
    /// Tag term/label
    pub term: String,
    /// Tag scheme/domain
    pub scheme: Option<String>,
    /// Human-readable tag label
    pub label: Option<String>,
}

impl From<CoreTag> for Tag {
    fn from(core: CoreTag) -> Self {
        Self {
            term: core.term,
            scheme: core.scheme,
            label: core.label,
        }
    }
}

/// Image metadata
#[napi(object)]
pub struct Image {
    /// Image URL
    pub url: String,
    /// Image title
    pub title: Option<String>,
    /// Link associated with the image
    pub link: Option<String>,
    /// Image width in pixels
    pub width: Option<u32>,
    /// Image height in pixels
    pub height: Option<u32>,
    /// Image description
    pub description: Option<String>,
}

impl From<CoreImage> for Image {
    fn from(core: CoreImage) -> Self {
        Self {
            url: core.url,
            title: core.title,
            link: core.link,
            width: core.width,
            height: core.height,
            description: core.description,
        }
    }
}

/// Enclosure (attached media file)
#[napi(object)]
pub struct Enclosure {
    /// Enclosure URL
    pub url: String,
    /// File size in bytes
    pub length: Option<i64>,
    /// MIME type
    #[napi(js_name = "type")]
    pub enclosure_type: Option<String>,
}

impl From<CoreEnclosure> for Enclosure {
    fn from(core: CoreEnclosure) -> Self {
        Self {
            url: core.url,
            length: core.length.map(|l| i64::try_from(l).unwrap_or(i64::MAX)),
            enclosure_type: core.enclosure_type,
        }
    }
}

/// Content block
#[napi(object)]
pub struct Content {
    /// Content body
    pub value: String,
    /// Content MIME type
    #[napi(js_name = "type")]
    pub content_type: Option<String>,
    /// Content language
    pub language: Option<String>,
    /// Base URL for relative links
    pub base: Option<String>,
}

impl From<CoreContent> for Content {
    fn from(core: CoreContent) -> Self {
        Self {
            value: core.value,
            content_type: core.content_type,
            language: core.language,
            base: core.base,
        }
    }
}

/// Generator metadata
#[napi(object)]
pub struct Generator {
    /// Generator name
    pub value: String,
    /// Generator URI
    pub uri: Option<String>,
    /// Generator version
    pub version: Option<String>,
}

impl From<CoreGenerator> for Generator {
    fn from(core: CoreGenerator) -> Self {
        Self {
            value: core.value,
            uri: core.uri,
            version: core.version,
        }
    }
}

/// Source reference (for entries)
#[napi(object)]
pub struct Source {
    /// Source title
    pub title: Option<String>,
    /// Source link
    pub link: Option<String>,
    /// Source ID
    pub id: Option<String>,
}

impl From<CoreSource> for Source {
    fn from(core: CoreSource) -> Self {
        Self {
            title: core.title,
            link: core.link,
            id: core.id,
        }
    }
}

/// Podcast transcript metadata
#[napi(object)]
pub struct PodcastTranscript {
    /// Transcript URL
    pub url: String,
    /// Transcript type (e.g., "text/plain", "application/srt")
    #[napi(js_name = "type")]
    pub transcript_type: Option<String>,
    /// Transcript language
    pub language: Option<String>,
    /// Relationship type (e.g., "captions", "chapters")
    pub rel: Option<String>,
}

impl From<CorePodcastTranscript> for PodcastTranscript {
    fn from(core: CorePodcastTranscript) -> Self {
        Self {
            url: core.url,
            transcript_type: core.transcript_type,
            language: core.language,
            rel: core.rel,
        }
    }
}

/// Podcast person metadata
#[napi(object)]
pub struct PodcastPerson {
    /// Person's name
    pub name: String,
    /// Person's role (e.g., "host", "guest")
    pub role: Option<String>,
    /// Person's group (e.g., "cast", "crew")
    pub group: Option<String>,
    /// Person's image URL
    pub img: Option<String>,
    /// Person's URL/website
    pub href: Option<String>,
}

impl From<CorePodcastPerson> for PodcastPerson {
    fn from(core: CorePodcastPerson) -> Self {
        Self {
            name: core.name,
            role: core.role,
            group: core.group,
            img: core.img,
            href: core.href,
        }
    }
}
