//! Node.js bindings for `feedparser-rs`, exposing the RSS/Atom/JSON Feed parser to
//! JavaScript/TypeScript consumers via `napi-rs`.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod error;

use chrono::{DateTime, Utc};
use error::convert_feed_error;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;

use feedparser_rs::types::{
    PodcastAlternateEnclosure as CorePodcastAlternateEnclosure,
    PodcastAlternateEnclosureSource as CorePodcastAlternateEnclosureSource,
    PodcastFollow as CorePodcastFollow, PodcastIntegrity as CorePodcastIntegrity,
    PodcastLocation as CorePodcastLocation, PodcastRemoteItem as CorePodcastRemoteItem,
    PodcastSocialInteract as CorePodcastSocialInteract, PodcastTxt as CorePodcastTxt,
    PodcastUpdateFrequency as CorePodcastUpdateFrequency,
};
use feedparser_rs::{
    self as core, Cloud as CoreCloud, Content as CoreContent, Enclosure as CoreEnclosure,
    Entry as CoreEntry, FeedMeta as CoreFeedMeta, Generator as CoreGenerator, Image as CoreImage,
    InReplyTo as CoreInReplyTo, ItunesCategory as CoreItunesCategory,
    ItunesEntryMeta as CoreItunesEntryMeta, ItunesFeedMeta as CoreItunesFeedMeta,
    ItunesOwner as CoreItunesOwner, Link as CoreLink, MediaContent as CoreMediaContent,
    MediaCopyright as CoreMediaCopyright, MediaCredit as CoreMediaCredit,
    MediaRating as CoreMediaRating, MediaThumbnail as CoreMediaThumbnail,
    ParsedFeed as CoreParsedFeed, ParserLimits, Person as CorePerson,
    PodcastChapters as CorePodcastChapters, PodcastChat as CorePodcastChat,
    PodcastEntryMeta as CorePodcastEntryMeta, PodcastFunding as CorePodcastFunding,
    PodcastMeta as CorePodcastMeta, PodcastPerson as CorePodcastPerson,
    PodcastSoundbite as CorePodcastSoundbite, PodcastTranscript as CorePodcastTranscript,
    PodcastValue as CorePodcastValue, PodcastValueRecipient as CorePodcastValueRecipient,
    PodcastValueTimeSplit as CorePodcastValueTimeSplit, Source as CoreSource,
    SyndicationMeta as CoreSyndicationMeta, Tag as CoreTag, TextConstruct as CoreTextConstruct,
    TextInput as CoreTextInput, TextType,
};

/// Default maximum feed size (100 MB) - prevents `DoS` attacks
const DEFAULT_MAX_FEED_SIZE: usize = 100 * 1024 * 1024;

/// Converts a timestamp to milliseconds since the epoch as `f64`, the numeric type JS uses
/// for all `Date` values. Precision loss only occurs for dates far beyond any real feed
/// timestamp (`f64` exactly represents millisecond timestamps up to roughly year 287396).
#[allow(clippy::cast_precision_loss)]
const fn timestamp_millis_f64(dt: DateTime<Utc>) -> f64 {
    dt.timestamp_millis() as f64
}

/// Parsing options accepted by `parseWithOptions` and `parseUrlWithOptions`
///
/// All fields are optional; omitted fields use the same defaults as
/// [`core::ParseOptions::default`] (HTML sanitization and relative URI
/// resolution both enabled, 100MB max feed size).
#[napi(object)]
#[derive(Default)]
pub struct ParseOptions {
    /// Maximum feed size in bytes (default: 100MB)
    #[napi(js_name = "maxSize")]
    pub max_size: Option<u32>,
    /// Whether to sanitize HTML content in feed entries (default: true)
    ///
    /// Disabling this is **not recommended** unless the feed source is fully trusted.
    #[napi(js_name = "sanitizeHtml")]
    pub sanitize_html: Option<bool>,
    /// Whether to resolve relative URLs against the feed's base URL (default: true)
    #[napi(js_name = "resolveRelativeUris")]
    pub resolve_relative_uris: Option<bool>,
}

impl ParseOptions {
    /// Converts to core `ParserLimits` + `ParseOptions`, applying defaults for unset fields.
    fn into_core(self) -> core::ParseOptions {
        let max_feed_size = self.max_size.map_or(DEFAULT_MAX_FEED_SIZE, |s| s as usize);
        core::ParseOptions {
            resolve_relative_uris: self.resolve_relative_uris.unwrap_or(true),
            sanitize_html: self.sanitize_html.unwrap_or(true),
            limits: ParserLimits {
                max_feed_size_bytes: max_feed_size,
                ..ParserLimits::default()
            },
        }
    }
}

/// Parse an RSS/Atom/JSON Feed from bytes or string
///
/// # Arguments
///
/// * `source` - Feed content as Buffer, string, or `Uint8Array`
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

/// Parse an RSS/Atom/JSON Feed with custom options
///
/// # Arguments
///
/// * `source` - Feed content as Buffer, string, or `Uint8Array`
/// * `options` - Optional parsing options (max size, HTML sanitization, URI resolution)
///
/// # Returns
///
/// Parsed feed result with metadata and entries
///
/// # Errors
///
/// Returns error if input exceeds size limit or parsing fails catastrophically
// napi FFI boundary: exported functions must take owned values (napi-rs cannot bind
// `&Either<..>`/`&str` for these types), so the value isn't consumed on every path.
#[allow(clippy::needless_pass_by_value)]
#[napi]
pub fn parse_with_options(
    source: Either<Buffer, String>,
    options: Option<ParseOptions>,
) -> Result<ParsedFeed> {
    let core_options = options.unwrap_or_default().into_core();

    // Validate input size BEFORE copying to prevent DoS (CWE-770)
    let input_len = match &source {
        Either::A(buf) => buf.len(),
        Either::B(s) => s.len(),
    };

    if input_len > core_options.limits.max_feed_size_bytes {
        return Err(Error::new(
            Status::InvalidArg,
            format!(
                "Feed size ({} bytes) exceeds maximum allowed ({} bytes)",
                input_len, core_options.limits.max_feed_size_bytes
            ),
        ));
    }

    let bytes: &[u8] = match &source {
        Either::A(buf) => buf.as_ref(),
        Either::B(s) => s.as_bytes(),
    };

    let parsed = core::parse_with_options(bytes, &core_options).map_err(convert_feed_error)?;

    Ok(ParsedFeed::from(parsed))
}

/// Detect feed format without full parsing
///
/// # Arguments
///
/// * `source` - Feed content as Buffer, string, or `Uint8Array`
///
/// # Returns
///
/// Feed version string (e.g., "rss20", "atom10")
// napi FFI boundary: exported functions must take owned values.
#[allow(clippy::needless_pass_by_value)]
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
/// using `ETag` and Last-Modified headers for bandwidth-efficient caching.
///
/// # Arguments
///
/// * `url` - HTTP or HTTPS URL to fetch
/// * `etag` - Optional `ETag` from previous fetch for conditional GET
/// * `modified` - Optional Last-Modified timestamp from previous fetch
/// * `user_agent` - Optional custom User-Agent header
///
/// # Returns
///
/// Parsed feed result with HTTP metadata fields populated:
/// - `status`: HTTP status code (200, 304, etc.)
/// - `href`: Final URL after redirects
/// - `etag`: `ETag` header value (for next request)
/// - `modified`: Last-Modified header value (for next request)
/// - `headers`: Full HTTP response headers
///
/// On 304 Not Modified, returns a feed with empty entries but status=304.
///
/// # Errors
///
/// Returns error if the URL is invalid, the request fails, or parsing fails catastrophically
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
// napi FFI boundary: exported functions must take owned values (napi-rs cannot bind
// JS string arguments by reference).
#[allow(clippy::needless_pass_by_value)]
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
    .map_err(convert_feed_error)?;

    Ok(ParsedFeed::from(parsed))
}

/// Parse feed from URL with custom options
///
/// Like `parseUrl` but allows specifying custom options for `DoS` protection,
/// HTML sanitization, and relative URI resolution.
///
/// # Errors
///
/// Returns error if the URL is invalid, the request fails, or parsing fails catastrophically
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
///   { maxSize: 10485760 } // 10MB
/// );
/// ```
// napi FFI boundary: exported functions must take owned values (napi-rs cannot bind
// JS string arguments by reference).
#[allow(clippy::needless_pass_by_value)]
#[cfg(feature = "http")]
#[napi]
pub fn parse_url_with_options(
    url: String,
    etag: Option<String>,
    modified: Option<String>,
    user_agent: Option<String>,
    options: Option<ParseOptions>,
) -> Result<ParsedFeed> {
    let core_options = options.unwrap_or_default().into_core();

    let parsed = core::parse_url_with_options(
        &url,
        etag.as_deref(),
        modified.as_deref(),
        user_agent.as_deref(),
        &core_options,
    )
    .map_err(convert_feed_error)?;

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
    /// `ETag` header from HTTP response
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
            entries: {
                let mut v = Vec::with_capacity(core.entries.len());
                v.extend(core.entries.into_iter().map(Entry::from));
                v
            },
            bozo: core.bozo,
            bozo_exception: core.bozo_exception,
            encoding: core.encoding,
            version: core.version.to_string(),
            namespaces: core.namespaces,
            status: core.status.map(u32::from),
            href: core.href,
            etag: core.etag,
            modified: core.modified,
            #[cfg(feature = "http")]
            headers: core.headers,
        }
    }
}

/// Syndication module metadata (RSS 1.0)
#[napi(object)]
pub struct SyndicationMeta {
    /// Update period (hourly, daily, weekly, monthly, yearly)
    ///
    /// # Example
    ///
    /// "daily" with updateFrequency: 2 means the feed updates twice per day
    #[napi(js_name = "updatePeriod")]
    pub update_period: Option<String>,
    /// Number of times updated per period
    #[napi(js_name = "updateFrequency")]
    pub update_frequency: Option<String>,
    /// Base date for update schedule (ISO 8601)
    #[napi(js_name = "updateBase")]
    pub update_base: Option<String>,
}

impl From<CoreSyndicationMeta> for SyndicationMeta {
    fn from(core: CoreSyndicationMeta) -> Self {
        Self {
            update_period: core.update_period.map(|p| p.as_str().to_string()),
            update_frequency: core.update_frequency,
            update_base: core.update_base,
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
    /// Feed summary (populated from itunes:summary when present)
    pub summary: Option<String>,
    /// Detailed summary with metadata
    pub summary_detail: Option<TextConstruct>,
    /// Last update date (original string from feed, timezone preserved)
    pub updated: Option<String>,
    /// Parsed last update date as milliseconds since epoch
    #[napi(js_name = "updatedParsed")]
    pub updated_parsed: Option<f64>,
    /// Initial publication date (original string from feed, timezone preserved)
    pub published: Option<String>,
    /// Parsed publication date as milliseconds since epoch
    #[napi(js_name = "publishedParsed")]
    pub published_parsed: Option<f64>,
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
    /// Time-to-live (update frequency hint) in minutes (kept as string for API compatibility)
    pub ttl: Option<String>,
    /// URL of documentation for the RSS format used
    pub docs: Option<String>,
    /// License URL (Creative Commons, etc.)
    pub license: Option<String>,
    /// Syndication module metadata (RSS 1.0)
    pub syndication: Option<SyndicationMeta>,
    /// Dublin Core creator (author fallback)
    #[napi(js_name = "dcCreator")]
    pub dc_creator: Option<String>,
    /// Dublin Core publisher
    #[napi(js_name = "dcPublisher")]
    pub dc_publisher: Option<String>,
    /// Dublin Core rights (copyright)
    #[napi(js_name = "dcRights")]
    pub dc_rights: Option<String>,
    /// Geographic location (`GeoRSS`), exposed as `where` per Python feedparser API
    #[napi(js_name = "where")]
    pub r#where: Option<GeoLocation>,
    /// W3C Basic Geo latitude (`geo:lat`)
    #[napi(js_name = "geoLat")]
    pub geo_lat: Option<String>,
    /// W3C Basic Geo longitude (`geo:long`)
    #[napi(js_name = "geoLong")]
    pub geo_long: Option<String>,
    /// iTunes podcast metadata
    pub itunes: Option<ItunesFeedMeta>,
    /// Podcast 2.0 metadata
    pub podcast: Option<PodcastMeta>,
    /// JSON Feed `next_url` for pagination (JSON Feed 1.1)
    pub next_url: Option<String>,
    /// Media RSS thumbnails at feed/channel level
    #[napi(js_name = "mediaThumbnails")]
    pub media_thumbnail: Vec<MediaThumbnail>,
    /// Media RSS content items at feed/channel level
    #[napi(js_name = "mediaContent")]
    pub media_content: Vec<MediaContent>,
    /// Media RSS rating (`media:rating`) at feed level
    #[napi(js_name = "mediaRating")]
    pub media_rating: Option<MediaRating>,
    /// Media RSS keywords (`media:keywords`) at feed level
    #[napi(js_name = "mediaKeywords")]
    pub media_keywords: Option<String>,
    /// RSS 2.0 cloud subscription endpoint
    pub cloud: Option<Cloud>,
    /// RSS 2.0 text input form
    pub textinput: Option<TextInput>,
    /// RSS 2.0 skip hours (0-23)
    pub skiphours: Vec<u32>,
    /// RSS 2.0 skip days
    pub skipdays: Vec<String>,
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
            summary: core.summary,
            summary_detail: core.summary_detail.map(TextConstruct::from),
            updated: core.updated_str,
            updated_parsed: core.updated.map(timestamp_millis_f64),
            published: core.published_str,
            published_parsed: core.published.map(timestamp_millis_f64),
            author: core.author.map(|s| s.to_string()),
            author_detail: core.author_detail.map(Person::from),
            authors: core.authors.into_iter().map(Person::from).collect(),
            contributors: core.contributors.into_iter().map(Person::from).collect(),
            publisher: core.publisher.map(|s| s.to_string()),
            publisher_detail: core.publisher_detail.map(Person::from),
            language: core.language.map(|s| s.to_string()),
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
            docs: core.docs,
            license: core.license,
            syndication: core.syndication.map(|b| SyndicationMeta::from(*b)),
            dc_creator: core.dc_creator.map(|s| s.to_string()),
            dc_publisher: core.dc_publisher.map(|s| s.to_string()),
            dc_rights: core.dc_rights,
            r#where: core.r#where.map(|b| GeoLocation::from(*b)),
            geo_lat: core.geo_lat,
            geo_long: core.geo_long,
            itunes: core.itunes.map(|b| ItunesFeedMeta::from(*b)),
            podcast: core.podcast.map(|b| PodcastMeta::from(*b)),
            next_url: core.next_url,
            media_thumbnail: core
                .media_thumbnail
                .into_iter()
                .map(MediaThumbnail::from)
                .collect(),
            media_content: core
                .media_content
                .into_iter()
                .map(MediaContent::from)
                .collect(),
            media_rating: core.media_rating.map(MediaRating::from),
            media_keywords: core.media_keywords,
            cloud: core.cloud.map(Cloud::from),
            textinput: core.textinput.map(TextInput::from),
            skiphours: core.skiphours,
            skipdays: core.skipdays,
        }
    }
}

impl From<CoreCloud> for Cloud {
    fn from(core: CoreCloud) -> Self {
        Self {
            domain: core.domain,
            port: core.port,
            path: core.path,
            register_procedure: core.register_procedure,
            protocol: core.protocol,
        }
    }
}

impl From<CoreTextInput> for TextInput {
    fn from(core: CoreTextInput) -> Self {
        Self {
            title: core.title,
            description: core.description,
            name: core.name,
            link: core.link,
        }
    }
}

/// Atom Threading Extensions in-reply-to reference (RFC 4685)
#[napi(object)]
pub struct InReplyTo {
    /// IRI of the entry being replied to (ref attribute)
    #[napi(js_name = "ref")]
    pub ref_field: Option<String>,
    /// URL where the referenced entry can be found
    pub href: Option<String>,
    /// MIME type of the linked resource
    #[napi(js_name = "type")]
    pub type_field: Option<String>,
    /// IRI of the feed containing the referenced entry
    pub source: Option<String>,
}

impl From<CoreInReplyTo> for InReplyTo {
    fn from(core: CoreInReplyTo) -> Self {
        Self {
            ref_field: core.ref_.map(|s| s.to_string()),
            href: core.href.map(|s| s.to_string()),
            type_field: core.type_.map(|s| s.to_string()),
            source: core.source.map(|s| s.to_string()),
        }
    }
}

/// Media RSS rating (`media:rating`)
#[napi(object)]
pub struct MediaRating {
    /// Rating scheme URI (e.g. "urn:simple", "urn:mpaa")
    pub scheme: Option<String>,
    /// Rating value (e.g. "adult", "nonadult", "pg-13")
    pub content: String,
}

impl From<CoreMediaRating> for MediaRating {
    fn from(core: CoreMediaRating) -> Self {
        Self {
            scheme: core.scheme,
            content: core.content,
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
    /// Entry subtitle (Atom §4.2.12 at entry level)
    pub subtitle: Option<String>,
    /// Detailed subtitle with metadata
    pub subtitle_detail: Option<TextConstruct>,
    /// Rights/copyright statement
    pub rights: Option<String>,
    /// Detailed rights with metadata
    pub rights_detail: Option<TextConstruct>,
    /// Short description/summary
    pub summary: Option<String>,
    /// Detailed summary with metadata
    pub summary_detail: Option<TextConstruct>,
    /// Full content blocks
    pub content: Vec<Content>,
    /// Publication date (original string from feed, timezone preserved)
    pub published: Option<String>,
    /// Parsed publication date as milliseconds since epoch
    #[napi(js_name = "publishedParsed")]
    pub published_parsed: Option<f64>,
    /// Last update date (original string from feed, timezone preserved)
    pub updated: Option<String>,
    /// Parsed last update date as milliseconds since epoch
    #[napi(js_name = "updatedParsed")]
    pub updated_parsed: Option<f64>,
    /// Creation date (RFC 3339 string)
    pub created: Option<String>,
    /// Parsed creation date as milliseconds since epoch
    #[napi(js_name = "createdParsed")]
    pub created_parsed: Option<f64>,
    /// Expiration date (RFC 3339 string)
    pub expired: Option<String>,
    /// Parsed expiration date as milliseconds since epoch
    #[napi(js_name = "expiredParsed")]
    pub expired_parsed: Option<f64>,
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
    /// Geographic location (`GeoRSS`), exposed as `where` per Python feedparser API
    #[napi(js_name = "where")]
    pub r#where: Option<GeoLocation>,
    /// W3C Basic Geo latitude (`geo:lat`)
    #[napi(js_name = "geoLat")]
    pub geo_lat: Option<String>,
    /// W3C Basic Geo longitude (`geo:long`)
    #[napi(js_name = "geoLong")]
    pub geo_long: Option<String>,
    /// Dublin Core creator (author)
    #[napi(js_name = "dcCreator")]
    pub dc_creator: Option<String>,
    /// Dublin Core date (RFC 3339 string)
    #[napi(js_name = "dcDate")]
    pub dc_date: Option<String>,
    /// Parsed Dublin Core date as JS Date object
    #[napi(js_name = "dcDateParsed")]
    pub dc_date_parsed: Option<f64>,
    /// Dublin Core subject tags
    #[napi(js_name = "dcSubject")]
    pub dc_subject: Vec<String>,
    /// Dublin Core rights (copyright)
    #[napi(js_name = "dcRights")]
    pub dc_rights: Option<String>,
    /// Media RSS thumbnails
    #[napi(js_name = "mediaThumbnails")]
    pub media_thumbnail: Vec<MediaThumbnail>,
    /// Media RSS content
    #[napi(js_name = "mediaContent")]
    pub media_content: Vec<MediaContent>,
    /// Media RSS credits (media:credit elements)
    #[napi(js_name = "mediaCredit")]
    pub media_credit: Vec<MediaCredit>,
    /// Media RSS copyright (media:copyright element)
    #[napi(js_name = "mediaCopyright")]
    pub media_copyright: Option<MediaCopyright>,
    /// Media RSS rating (media:rating element)
    #[napi(js_name = "mediaRating")]
    pub media_rating: Option<MediaRating>,
    /// Media RSS keywords (raw comma-separated string)
    #[napi(js_name = "mediaKeywords")]
    pub media_keywords: Option<String>,
    /// Media RSS description (plain text only)
    #[napi(js_name = "mediaDescription")]
    pub media_description: Option<String>,
    /// Media RSS title (plain text only)
    #[napi(js_name = "mediaTitle")]
    pub media_title: Option<String>,
    /// iTunes episode metadata
    pub itunes: Option<ItunesEntryMeta>,
    /// Podcast 2.0 episode metadata
    pub podcast: Option<PodcastEntryMeta>,
    /// Atom Threading Extensions: in-reply-to references (RFC 4685)
    #[napi(js_name = "thrInReplyTo")]
    pub thr_in_reply_to: Vec<InReplyTo>,
    /// Atom Threading Extensions: total reply count (RFC 4685)
    #[napi(js_name = "thrTotal")]
    pub thr_total: Option<u32>,
    /// Slash namespace: comment count
    #[napi(js_name = "slashComments")]
    pub slash_comments: Option<String>,
    /// Slash namespace: hit parade
    #[napi(js_name = "slashHitParade")]
    pub slash_hit_parade: Option<String>,
    /// WFW namespace: comment RSS feed URL
    #[napi(js_name = "wfwCommentRss")]
    pub wfw_comment_rss: Option<String>,
    /// Whether the RSS `<guid>` is a permalink (`isPermaLink` attribute).
    ///
    /// `true` when `isPermaLink="true"` or attribute absent (RSS 2.0 default).
    /// `false` when `isPermaLink="false"`. `null` when no `<guid>` element present.
    #[napi(js_name = "guidislink")]
    pub guidislink: Option<bool>,
    /// Entry language (JSON Feed `language` field)
    pub language: Option<String>,
    /// External URL where the full content lives (JSON Feed `external_url`)
    #[napi(js_name = "externalUrl")]
    pub external_url: Option<String>,
}

impl From<CoreEntry> for Entry {
    fn from(core: CoreEntry) -> Self {
        Self {
            id: core.id.map(|s| s.to_string()),
            title: core.title,
            title_detail: core.title_detail.map(TextConstruct::from),
            link: core.link,
            links: core.links.into_iter().map(Link::from).collect(),
            subtitle: core.subtitle,
            subtitle_detail: core.subtitle_detail.map(TextConstruct::from),
            rights: core.rights,
            rights_detail: core.rights_detail.map(TextConstruct::from),
            summary: core.summary,
            summary_detail: core.summary_detail.map(TextConstruct::from),
            content: core.content.into_iter().map(Content::from).collect(),
            published: core.published_str,
            published_parsed: core.published.map(timestamp_millis_f64),
            updated: core.updated_str,
            updated_parsed: core.updated.map(timestamp_millis_f64),
            created: core.created_str,
            created_parsed: core.created.map(timestamp_millis_f64),
            expired: core.expired.as_ref().map(DateTime::to_rfc3339),
            expired_parsed: core.expired.map(timestamp_millis_f64),
            author: core.author.map(|s| s.to_string()),
            author_detail: core.author_detail.map(Person::from),
            authors: core.authors.into_iter().map(Person::from).collect(),
            contributors: core.contributors.into_iter().map(Person::from).collect(),
            publisher: core.publisher.map(|s| s.to_string()),
            publisher_detail: core.publisher_detail.map(Person::from),
            tags: core.tags.into_iter().map(Tag::from).collect(),
            enclosures: core.enclosures.into_iter().map(Enclosure::from).collect(),
            comments: core.comments,
            source: core.source.map(Source::from),
            podcast_transcripts: core
                .podcast_transcripts
                .into_iter()
                .map(PodcastTranscript::from)
                .collect(),
            podcast_persons: core
                .podcast_persons
                .into_iter()
                .map(PodcastPerson::from)
                .collect(),
            license: core.license,
            r#where: core.r#where.map(|b| GeoLocation::from(*b)),
            geo_lat: core.geo_lat,
            geo_long: core.geo_long,
            dc_creator: core.dc_creator.map(|s| s.to_string()),
            dc_date: core.dc_date.map(|dt| dt.to_rfc3339()),
            dc_date_parsed: core.dc_date.map(timestamp_millis_f64),
            dc_subject: core.dc_subject,
            dc_rights: core.dc_rights,
            media_thumbnail: core
                .media_thumbnail
                .into_iter()
                .map(MediaThumbnail::from)
                .collect(),
            media_content: core
                .media_content
                .into_iter()
                .map(MediaContent::from)
                .collect(),
            media_credit: core
                .media_credit
                .into_iter()
                .map(MediaCredit::from)
                .collect(),
            media_copyright: core.media_copyright.map(MediaCopyright::from),
            media_rating: core.media_rating.map(MediaRating::from),
            media_keywords: core.media_keywords,
            media_description: core.media_description,
            media_title: core.media_title,
            itunes: core.itunes.map(|b| ItunesEntryMeta::from(*b)),
            podcast: core.podcast.map(|b| PodcastEntryMeta::from(*b)),
            thr_in_reply_to: core.in_reply_to.into_iter().map(InReplyTo::from).collect(),
            thr_total: core.thr_total,
            slash_comments: core.slash_comments.map(|n| n.to_string()),
            slash_hit_parade: core.slash_hit_parade,
            wfw_comment_rss: core.wfw_comment_rss,
            guidislink: core.guidislink,
            language: core.language.map(|s| s.to_string()),
            external_url: core.external_url,
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
                TextType::Text => "text/plain".to_string(),
                TextType::Html => "text/html".to_string(),
                TextType::Xhtml => "application/xhtml+xml".to_string(),
            },
            language: core.language.map(|s| s.to_string()),
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
    /// Length of the linked resource in bytes (raw string value, as in Python feedparser)
    pub length: Option<String>,
    /// Language of the linked resource
    pub hreflang: Option<String>,
    /// RFC 4685 §4: number of replies at the IRI
    pub thr_count: Option<u32>,
    /// RFC 4685 §4: when the reply resource was last modified (RFC 3339)
    pub thr_updated: Option<String>,
    /// Parsed thr:updated as milliseconds since epoch
    #[napi(js_name = "thrUpdatedParsed")]
    pub thr_updated_parsed: Option<f64>,
}

impl From<CoreLink> for Link {
    fn from(core: CoreLink) -> Self {
        Self {
            href: core.href.into_inner(),
            rel: core.rel.map(|s| s.to_string()),
            link_type: core.link_type.map(|t| t.to_string()),
            title: core.title,
            length: core.length,
            hreflang: core.hreflang.map(|s| s.to_string()),
            thr_count: core.thr_count,
            thr_updated: core.thr_updated.as_ref().map(DateTime::to_rfc3339),
            thr_updated_parsed: core.thr_updated.map(timestamp_millis_f64),
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
    pub href: Option<String>,
    /// Person's avatar image URL (JSON Feed only)
    pub avatar: Option<String>,
}

impl From<CorePerson> for Person {
    fn from(core: CorePerson) -> Self {
        Self {
            name: core.name.map(|s| s.to_string()),
            email: core.email.map(core::Email::into_inner),
            href: core.uri,
            avatar: core.avatar,
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
            term: core.term.to_string(),
            scheme: core.scheme.map(|s| s.to_string()),
            label: core.label.map(|s| s.to_string()),
        }
    }
}

/// RSS 2.0 cloud subscription endpoint
#[napi(object)]
pub struct Cloud {
    /// Cloud server domain
    pub domain: Option<String>,
    /// Cloud server port
    pub port: Option<String>,
    /// Cloud server path
    pub path: Option<String>,
    /// Remote procedure to call for registration
    #[napi(js_name = "registerprocedure")]
    pub register_procedure: Option<String>,
    /// Protocol used for notifications (e.g., "xml-rpc", "soap", "http-post")
    pub protocol: Option<String>,
}

/// RSS 2.0 text input form
#[napi(object)]
pub struct TextInput {
    /// Text input field label
    pub title: Option<String>,
    /// Text input field description
    pub description: Option<String>,
    /// Text input field name (for form submission)
    pub name: Option<String>,
    /// URL to submit the text input to
    pub link: Option<String>,
}

/// Image metadata
#[napi(object)]
pub struct Image {
    /// Image URL (primary field)
    pub href: String,
    /// Image URL alias (same as href, Python feedparser compatibility)
    pub url: String,
    /// Image title
    pub title: Option<String>,
    /// Detailed title with type metadata
    pub title_detail: Option<TextConstruct>,
    /// Link associated with the image
    pub link: Option<String>,
    /// Image width in pixels
    pub width: Option<u32>,
    /// Image height in pixels
    pub height: Option<u32>,
    /// Image description (alias for subtitle)
    pub description: Option<String>,
    /// Image subtitle (alias for description)
    pub subtitle: Option<String>,
    /// Detailed subtitle/description with type metadata
    pub subtitle_detail: Option<TextConstruct>,
    /// Links synthesized from href
    pub links: Vec<Link>,
}

impl From<CoreImage> for Image {
    fn from(core: CoreImage) -> Self {
        let href = core.url.into_inner();
        let link = CoreLink::alternate(href.clone());
        let links = vec![Link::from(link)];
        let title_detail = core
            .title
            .as_deref()
            .map(|t| TextConstruct::from(CoreTextConstruct::text(t)));
        let subtitle_detail = core
            .description
            .as_deref()
            .map(|d| TextConstruct::from(CoreTextConstruct::text(d)));
        Self {
            url: href.clone(),
            href,
            title: core.title,
            title_detail,
            link: core.link,
            width: core.width,
            height: core.height,
            description: core.description.clone(),
            subtitle: core.description,
            subtitle_detail,
            links,
        }
    }
}

/// Enclosure (attached media file)
#[napi(object)]
pub struct Enclosure {
    /// Enclosure URL
    pub href: String,
    /// File size in bytes (raw string value, as in Python feedparser)
    pub length: Option<String>,
    /// MIME type
    #[napi(js_name = "type")]
    pub enclosure_type: Option<String>,
    /// Attachment title (JSON Feed only)
    pub title: Option<String>,
    /// Duration in seconds as raw string (JSON Feed `duration_in_seconds`)
    pub duration: Option<String>,
}

impl From<CoreEnclosure> for Enclosure {
    fn from(core: CoreEnclosure) -> Self {
        Self {
            href: core.url.into_inner(),
            length: core.length,
            enclosure_type: core.enclosure_type.map(|t| t.to_string()),
            title: core.title,
            duration: core.duration,
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
    /// Out-of-line content URL (Atom `<content src="...">`)
    pub src: Option<String>,
}

impl From<CoreContent> for Content {
    fn from(core: CoreContent) -> Self {
        Self {
            value: core.value,
            content_type: core.content_type.map(|t| t.to_string()),
            language: core.language.map(|s| s.to_string()),
            base: core.base,
            src: core.src,
        }
    }
}

/// Generator metadata
#[napi(object)]
pub struct Generator {
    /// Generator name (text content of the `<generator>` element)
    pub name: String,
    /// Generator URI (`href` attribute, matching Python feedparser API)
    pub href: Option<String>,
    /// Generator version
    pub version: Option<String>,
}

impl From<CoreGenerator> for Generator {
    fn from(core: CoreGenerator) -> Self {
        Self {
            name: core.name,
            href: core.href,
            version: core.version.map(|s| s.to_string()),
        }
    }
}

/// Source reference (for entries)
#[napi(object)]
pub struct Source {
    /// Source title
    pub title: Option<String>,
    /// Primary source URL for RSS `<source url="...">` (RSS-only)
    pub href: Option<String>,
    /// Primary source URL for Atom `<source><link href="..."/>` (Atom-only)
    pub link: Option<String>,
    /// Source author flat string (Atom `<source><author>`)
    pub author: Option<String>,
    /// Source ID
    pub id: Option<String>,
    /// All links from the source element
    pub links: Vec<Link>,
    /// Last update date string (Atom `<updated>`, RFC 3339)
    pub updated: Option<String>,
    /// Rights/copyright statement (Atom `<rights>`)
    pub rights: Option<String>,
    /// Whether `<id>` was used as the link
    pub guidislink: Option<bool>,
}

impl From<CoreSource> for Source {
    fn from(core: CoreSource) -> Self {
        Self {
            title: core.title,
            href: core.href,
            link: core.link,
            author: core.author,
            id: core.id,
            links: core.links.into_iter().map(Link::from).collect(),
            updated: core.updated_str,
            rights: core.rights,
            guidislink: core.guidislink,
        }
    }
}

/// Geographic location from `GeoRSS` namespace
#[napi(object)]
pub struct GeoLocation {
    /// Type of geographic shape ("point", "line", "polygon", "box")
    #[napi(js_name = "geoType")]
    pub geo_type: String,
    /// Coordinate pairs as nested array [[lat, lng], ...]
    ///
    /// Format depends on `geo_type`:
    /// - "point": Single pair [[lat, lng]]
    /// - "line": Two or more pairs [[lat1, lng1], [lat2, lng2], ...]
    /// - "box": Two pairs [[lower-left-lat, lower-left-lng], [upper-right-lat, upper-right-lng]]
    /// - "polygon": Three or more pairs forming a closed shape [[lat1, lng1], ..., [lat1, lng1]]
    pub coordinates: Vec<Vec<f64>>,
    /// Coordinate Reference System (from GeoRSS/GML `srsName`, e.g. "EPSG:4326" for WGS84 latitude/longitude)
    #[napi(js_name = "srsName")]
    pub srs_name: Option<String>,
    /// Elevation in meters (from `georss:elev`)
    pub elev: Option<f64>,
    /// Feature type classification (from `georss:featuretypetag`)
    #[napi(js_name = "featureTypeTag")]
    pub feature_type_tag: Option<String>,
    /// Human-readable place name (from `georss:featurename`)
    #[napi(js_name = "featureName")]
    pub feature_name: Option<String>,
    /// Relationship type (from `georss:relationshiptag`)
    #[napi(js_name = "relationshipTag")]
    pub relationship_tag: Option<String>,
}

impl From<feedparser_rs::namespace::georss::GeoLocation> for GeoLocation {
    fn from(core: feedparser_rs::namespace::georss::GeoLocation) -> Self {
        use feedparser_rs::namespace::georss::GeoType;

        Self {
            geo_type: match core.geo_type {
                GeoType::Point => "point".to_string(),
                GeoType::Line => "line".to_string(),
                GeoType::Polygon => "polygon".to_string(),
                GeoType::Box => "box".to_string(),
            },
            coordinates: core
                .coordinates
                .into_iter()
                .map(|(lat, lng)| vec![lat, lng])
                .collect(),
            srs_name: core.srs_name,
            elev: core.elev,
            feature_type_tag: core.feature_type_tag,
            feature_name: core.feature_name,
            relationship_tag: core.relationship_tag,
        }
    }
}

/// Media RSS thumbnail
#[napi(object)]
pub struct MediaThumbnail {
    /// Thumbnail URL
    ///
    /// Note: URL from untrusted feed input. Validate before fetching.
    pub url: String,
    /// Width in pixels (raw string value, as in Python feedparser)
    pub width: Option<String>,
    /// Height in pixels (raw string value, as in Python feedparser)
    pub height: Option<String>,
    /// Time offset in NTP format (time attribute)
    ///
    /// Indicates which frame of the media this thumbnail represents.
    pub time: Option<String>,
}

impl From<CoreMediaThumbnail> for MediaThumbnail {
    fn from(core: CoreMediaThumbnail) -> Self {
        Self {
            url: core.url.into_inner(),
            width: core.width,
            height: core.height,
            time: core.time,
        }
    }
}

/// Media RSS content
#[napi(object)]
pub struct MediaContent {
    /// Media URL
    ///
    /// Note: URL from untrusted feed input. Validate before fetching.
    pub url: String,
    /// MIME type
    #[napi(js_name = "type")]
    pub content_type: Option<String>,
    /// Medium type: "image", "video", "audio", "document", "executable"
    pub medium: Option<String>,
    /// File size in bytes (raw string value, as in Python feedparser)
    pub filesize: Option<String>,
    /// Width in pixels (raw string value, as in Python feedparser)
    pub width: Option<String>,
    /// Height in pixels (raw string value, as in Python feedparser)
    pub height: Option<String>,
    /// Duration (raw string value, as in Python feedparser)
    pub duration: Option<String>,
    /// Bitrate in kilobits per second (raw string value)
    pub bitrate: Option<String>,
    /// Language of the media
    pub lang: Option<String>,
    /// Number of audio channels (raw string value)
    pub channels: Option<String>,
    /// Codec used to produce the media
    pub codec: Option<String>,
    /// Expression type: "full", "sample", "nonstop"
    pub expression: Option<String>,
    /// Whether this is the default media object (raw string value)
    pub isdefault: Option<String>,
    /// Sampling rate in kHz (raw string value)
    pub samplingrate: Option<String>,
    /// Frame rate in frames per second (raw string value)
    pub framerate: Option<String>,
}

impl From<CoreMediaContent> for MediaContent {
    fn from(core: CoreMediaContent) -> Self {
        Self {
            url: core.url.into_inner(),
            content_type: core.content_type.map(|t| t.to_string()),
            medium: core.medium,
            filesize: core.filesize,
            width: core.width,
            height: core.height,
            duration: core.duration,
            bitrate: core.bitrate,
            lang: core.lang,
            channels: core.channels,
            codec: core.codec,
            expression: core.expression,
            isdefault: core.isdefault,
            samplingrate: core.samplingrate,
            framerate: core.framerate,
        }
    }
}

/// Media RSS credit (media:credit element)
#[napi(object)]
pub struct MediaCredit {
    /// Credit role (e.g., "author", "producer")
    pub role: Option<String>,
    /// Credit scheme URI (default: "urn:ebu")
    pub scheme: Option<String>,
    /// Credit text content (person/entity name)
    pub content: String,
}

impl From<CoreMediaCredit> for MediaCredit {
    fn from(core: CoreMediaCredit) -> Self {
        Self {
            role: core.role,
            scheme: core.scheme,
            content: core.content,
        }
    }
}

/// Media RSS copyright (media:copyright element)
#[napi(object)]
pub struct MediaCopyright {
    /// Copyright URL
    pub url: Option<String>,
}

impl From<CoreMediaCopyright> for MediaCopyright {
    fn from(core: CoreMediaCopyright) -> Self {
        Self { url: core.url }
    }
}

/// iTunes podcast feed metadata
#[napi(object)]
pub struct ItunesFeedMeta {
    /// Podcast author
    pub author: Option<String>,
    /// Podcast owner information
    pub owner: Option<ItunesOwner>,
    /// Podcast categories
    pub categories: Vec<ItunesCategory>,
    /// Explicit content flag
    pub explicit: Option<bool>,
    /// Podcast artwork URL
    ///
    /// Note: URL from untrusted feed input. Validate before fetching.
    pub image: Option<String>,
    /// Podcast keywords
    pub keywords: Vec<String>,
    /// Podcast type (episodic/serial)
    #[napi(js_name = "podcastType")]
    pub podcast_type: Option<String>,
    /// Podcast completion status (raw XML text value, e.g., "Yes", "No")
    pub complete: Option<String>,
    /// New feed URL for migrated podcasts
    ///
    /// Note: URL from untrusted feed input. Validate before fetching.
    #[napi(js_name = "newFeedUrl")]
    pub new_feed_url: Option<String>,
    /// Block flag: 1 = blocked ("yes"), 0 = not blocked
    pub block: Option<u8>,
}

impl From<CoreItunesFeedMeta> for ItunesFeedMeta {
    fn from(core: CoreItunesFeedMeta) -> Self {
        Self {
            author: core.author,
            owner: core.owner.map(ItunesOwner::from),
            categories: core
                .categories
                .into_iter()
                .map(ItunesCategory::from)
                .collect(),
            explicit: core.explicit,
            image: core.image.map(core::Url::into_inner),
            keywords: core.keywords,
            podcast_type: core.podcast_type,
            complete: core.complete,
            new_feed_url: core.new_feed_url.map(core::Url::into_inner),
            block: core.block,
        }
    }
}

/// iTunes owner information
#[napi(object)]
pub struct ItunesOwner {
    /// Owner name
    pub name: Option<String>,
    /// Owner email
    pub email: Option<String>,
}

impl From<CoreItunesOwner> for ItunesOwner {
    fn from(core: CoreItunesOwner) -> Self {
        Self {
            name: core.name,
            email: core.email,
        }
    }
}

/// iTunes category
#[napi(object)]
pub struct ItunesCategory {
    /// Category text
    pub text: String,
    /// Subcategory
    pub subcategory: Option<String>,
}

impl From<CoreItunesCategory> for ItunesCategory {
    fn from(core: CoreItunesCategory) -> Self {
        Self {
            text: core.text,
            subcategory: core.subcategory,
        }
    }
}

/// iTunes episode metadata
#[napi(object)]
pub struct ItunesEntryMeta {
    /// Episode title override
    pub title: Option<String>,
    /// Episode author
    pub author: Option<String>,
    /// Episode duration as raw string (itunes:duration)
    ///
    /// Preserved verbatim from the feed: "3600", "60:00", "1:00:00", "1:23:45", etc.
    pub duration: Option<String>,
    /// Explicit content flag for this episode
    pub explicit: Option<bool>,
    /// Episode-specific artwork URL
    ///
    /// Note: URL from untrusted feed input. Validate before fetching.
    pub image: Option<String>,
    /// Episode number as raw string (itunes:episode)
    pub episode: Option<String>,
    /// Season number as raw string (itunes:season)
    pub season: Option<String>,
    /// Episode type: "full", "trailer", or "bonus"
    #[napi(js_name = "episodeType")]
    pub episode_type: Option<String>,
}

impl From<CoreItunesEntryMeta> for ItunesEntryMeta {
    fn from(core: CoreItunesEntryMeta) -> Self {
        Self {
            title: core.title,
            author: core.author,
            duration: core.duration,
            explicit: core.explicit,
            image: core.image.map(core::Url::into_inner),
            episode: core.episode,
            season: core.season,
            episode_type: core.episode_type,
        }
    }
}

/// Podcast 2.0 namespace metadata (feed level)
#[napi(object)]
pub struct PodcastMeta {
    /// Podcast transcripts
    pub transcripts: Vec<PodcastTranscript>,
    /// Podcast funding links
    pub funding: Vec<PodcastFunding>,
    /// Podcast persons (hosts, etc.)
    pub persons: Vec<PodcastPerson>,
    /// Podcast GUID
    pub guid: Option<String>,
    /// Value-for-value payment information
    pub value: Option<PodcastValue>,
    /// Content medium type (podcast:medium)
    pub medium: Option<String>,
    /// Ownership transfer lock value: "yes" or "no" (podcast:locked)
    pub locked: Option<String>,
    /// Email of the lock owner (podcast:locked owner attribute)
    pub locked_owner: Option<String>,
    /// Chat room references (podcast:chat)
    pub chat: Vec<PodcastChat>,
    /// Whether the podcast uses Podping for update notifications
    /// (podcast:podping `usesPodping` attribute)
    #[napi(js_name = "podpingUsesPodping")]
    pub podping_uses_podping: Option<bool>,
    /// Related feed references (podcast:podroll)
    pub podroll: Vec<PodcastRemoteItem>,
    /// Geographic location (podcast:location)
    pub location: Option<PodcastLocation>,
    /// Text records (podcast:txt)
    pub txt: Vec<PodcastTxt>,
    /// Update frequency schedule (podcast:updateFrequency)
    #[napi(js_name = "updateFrequency")]
    pub update_frequency: Option<PodcastUpdateFrequency>,
    /// Follow links (podcast:follow)
    pub follow: Vec<PodcastFollow>,
}

impl From<CorePodcastMeta> for PodcastMeta {
    fn from(core: CorePodcastMeta) -> Self {
        Self {
            transcripts: core
                .transcripts
                .into_iter()
                .map(PodcastTranscript::from)
                .collect(),
            funding: core.funding.into_iter().map(PodcastFunding::from).collect(),
            persons: core.persons.into_iter().map(PodcastPerson::from).collect(),
            guid: core.guid,
            value: core.value.map(PodcastValue::from),
            medium: core.medium,
            locked: core.locked,
            locked_owner: core.locked_owner,
            chat: core.chat.into_iter().map(PodcastChat::from).collect(),
            podping_uses_podping: core.podping_uses_podping,
            podroll: core
                .podroll
                .into_iter()
                .map(PodcastRemoteItem::from)
                .collect(),
            location: core.location.map(PodcastLocation::from),
            txt: core.txt.into_iter().map(PodcastTxt::from).collect(),
            update_frequency: core.update_frequency.map(PodcastUpdateFrequency::from),
            follow: core.follow.into_iter().map(PodcastFollow::from).collect(),
        }
    }
}

/// Podcast 2.0 value element for monetization
#[napi(object)]
pub struct PodcastValue {
    /// Payment type: "lightning", "hive", etc.
    #[napi(js_name = "type")]
    pub value_type: String,
    /// Payment method: "keysend" for Lightning Network
    pub method: String,
    /// Suggested payment amount
    pub suggested: Option<String>,
    /// List of payment recipients with split percentages
    pub recipients: Vec<PodcastValueRecipient>,
    /// Time-bounded payment splits for pre-recorded remote content
    #[napi(js_name = "timeSplits")]
    pub time_splits: Vec<PodcastValueTimeSplit>,
}

impl From<CorePodcastValue> for PodcastValue {
    fn from(core: CorePodcastValue) -> Self {
        Self {
            value_type: core.type_,
            method: core.method,
            suggested: core.suggested,
            recipients: core
                .recipients
                .into_iter()
                .map(PodcastValueRecipient::from)
                .collect(),
            time_splits: core
                .time_splits
                .into_iter()
                .map(PodcastValueTimeSplit::from)
                .collect(),
        }
    }
}

/// Podcast 2.0 value time split for pre-recorded remote content
#[napi(object)]
pub struct PodcastValueTimeSplit {
    /// Start time in seconds within the episode
    #[napi(js_name = "startTime")]
    pub start_time: f64,
    /// Duration in seconds of this split
    pub duration: f64,
    /// Start time within the remote item, in seconds
    #[napi(js_name = "remoteStartTime")]
    pub remote_start_time: f64,
    /// Percentage of the payment routed to this split
    #[napi(js_name = "remotePercentage")]
    pub remote_percentage: f64,
    /// Payment recipients for this split
    pub recipients: Vec<PodcastValueRecipient>,
    /// Remote item this split routes payment to, if any
    #[napi(js_name = "remoteItem")]
    pub remote_item: Option<PodcastRemoteItem>,
}

impl From<CorePodcastValueTimeSplit> for PodcastValueTimeSplit {
    fn from(core: CorePodcastValueTimeSplit) -> Self {
        Self {
            start_time: core.start_time,
            duration: core.duration,
            remote_start_time: core.remote_start_time,
            remote_percentage: core.remote_percentage,
            recipients: core
                .recipients
                .into_iter()
                .map(PodcastValueRecipient::from)
                .collect(),
            remote_item: core.remote_item.map(PodcastRemoteItem::from),
        }
    }
}

/// Podcast 2.0 remote item reference
#[napi(object)]
pub struct PodcastRemoteItem {
    /// Feed GUID
    #[napi(js_name = "feedGuid")]
    pub feed_guid: Option<String>,
    /// Feed URL
    ///
    /// Note: URL from untrusted feed input. Validate before fetching.
    #[napi(js_name = "feedUrl")]
    pub feed_url: Option<String>,
    /// Item GUID
    #[napi(js_name = "itemGuid")]
    pub item_guid: Option<String>,
    /// Content medium type
    pub medium: Option<String>,
    /// Display title
    pub title: Option<String>,
}

impl From<CorePodcastRemoteItem> for PodcastRemoteItem {
    fn from(core: CorePodcastRemoteItem) -> Self {
        Self {
            feed_guid: core.feed_guid,
            feed_url: core.feed_url.map(core::Url::into_inner),
            item_guid: core.item_guid,
            medium: core.medium,
            title: core.title,
        }
    }
}

/// Podcast 2.0 chat room reference (podcast:chat)
#[napi(object)]
pub struct PodcastChat {
    /// Chat server address
    pub server: String,
    /// Chat protocol: "matrix", "xmpp", etc.
    pub protocol: String,
    /// Account identifier on the chat server
    #[napi(js_name = "accountId")]
    pub account_id: Option<String>,
    /// Space identifier, for protocols that group rooms
    pub space: Option<String>,
}

impl From<CorePodcastChat> for PodcastChat {
    fn from(core: CorePodcastChat) -> Self {
        Self {
            server: core.server,
            protocol: core.protocol,
            account_id: core.account_id,
            space: core.space,
        }
    }
}

/// Value recipient for payment splitting
#[napi(object)]
pub struct PodcastValueRecipient {
    /// Recipient's name
    pub name: Option<String>,
    /// Recipient type: "node" for Lightning Network nodes
    #[napi(js_name = "type")]
    pub recipient_type: String,
    /// Payment address (e.g., Lightning node public key)
    pub address: String,
    /// Payment split percentage
    pub split: u32,
    /// Whether this is a fee recipient
    pub fee: Option<bool>,
}

impl From<CorePodcastValueRecipient> for PodcastValueRecipient {
    fn from(core: CorePodcastValueRecipient) -> Self {
        Self {
            name: core.name,
            recipient_type: core.type_,
            address: core.address,
            split: core.split,
            fee: core.fee,
        }
    }
}

/// Podcast funding link
#[napi(object)]
pub struct PodcastFunding {
    /// Funding URL
    ///
    /// Note: URL from untrusted feed input. Validate before fetching.
    pub url: String,
    /// Funding message
    pub message: Option<String>,
}

impl From<CorePodcastFunding> for PodcastFunding {
    fn from(core: CorePodcastFunding) -> Self {
        Self {
            url: core.url.into_inner(),
            message: core.message,
        }
    }
}

/// Podcast 2.0 episode metadata
#[napi(object)]
pub struct PodcastEntryMeta {
    /// Episode transcripts
    pub transcript: Vec<PodcastTranscript>,
    /// Episode chapters
    pub chapters: Option<PodcastChapters>,
    /// Episode soundbites
    pub soundbite: Vec<PodcastSoundbite>,
    /// Episode persons
    pub persons: Vec<PodcastPerson>,
    /// Content medium type (podcast:medium)
    pub medium: Option<String>,
    /// Season number (podcast:season number attribute)
    pub season: Option<String>,
    /// Episode number (podcast:episode number attribute)
    pub episode: Option<String>,
    /// Chat room references (podcast:chat)
    pub chat: Vec<PodcastChat>,
    /// Alternate enclosures (podcast:alternateEnclosure)
    #[napi(js_name = "alternateEnclosures")]
    pub alternate_enclosures: Vec<PodcastAlternateEnclosure>,
    /// Geographic location (podcast:location)
    pub location: Option<PodcastLocation>,
    /// Social interaction threads (podcast:socialInteract)
    #[napi(js_name = "socialInteract")]
    pub social_interact: Vec<PodcastSocialInteract>,
    /// Text records (podcast:txt)
    pub txt: Vec<PodcastTxt>,
    /// Follow links (podcast:follow)
    pub follow: Vec<PodcastFollow>,
    /// Value-for-value payment information (podcast:value)
    pub value: Option<PodcastValue>,
}

impl From<CorePodcastEntryMeta> for PodcastEntryMeta {
    fn from(core: CorePodcastEntryMeta) -> Self {
        Self {
            transcript: core
                .transcript
                .into_iter()
                .map(PodcastTranscript::from)
                .collect(),
            chapters: core.chapters.map(PodcastChapters::from),
            soundbite: core
                .soundbite
                .into_iter()
                .map(PodcastSoundbite::from)
                .collect(),
            persons: core.persons.into_iter().map(PodcastPerson::from).collect(),
            medium: core.medium,
            season: core.season,
            episode: core.episode,
            chat: core.chat.into_iter().map(PodcastChat::from).collect(),
            alternate_enclosures: core
                .alternate_enclosures
                .into_iter()
                .map(PodcastAlternateEnclosure::from)
                .collect(),
            location: core.location.map(PodcastLocation::from),
            social_interact: core
                .social_interact
                .into_iter()
                .map(PodcastSocialInteract::from)
                .collect(),
            txt: core.txt.into_iter().map(PodcastTxt::from).collect(),
            follow: core.follow.into_iter().map(PodcastFollow::from).collect(),
            value: core.value.map(PodcastValue::from),
        }
    }
}

/// Podcast chapters
#[napi(object)]
pub struct PodcastChapters {
    /// Chapters URL
    ///
    /// Note: URL from untrusted feed input. Validate before fetching.
    pub url: String,
    /// Chapters MIME type (e.g., "application/json+chapters", "application/xml+chapters")
    #[napi(js_name = "type")]
    pub chapters_type: String,
}

impl From<CorePodcastChapters> for PodcastChapters {
    fn from(core: CorePodcastChapters) -> Self {
        Self {
            url: core.url.into_inner(),
            chapters_type: core.type_.to_string(),
        }
    }
}

/// Podcast soundbite
#[napi(object)]
pub struct PodcastSoundbite {
    /// Start time in seconds
    #[napi(js_name = "startTime")]
    pub start_time: f64,
    /// Duration in seconds
    pub duration: f64,
    /// Title
    pub title: Option<String>,
}

impl From<CorePodcastSoundbite> for PodcastSoundbite {
    fn from(core: CorePodcastSoundbite) -> Self {
        Self {
            start_time: core.start_time,
            duration: core.duration,
            title: core.title,
        }
    }
}

/// Podcast transcript metadata
#[napi(object)]
pub struct PodcastTranscript {
    /// Transcript URL
    ///
    /// Note: URL from untrusted feed input. Validate before fetching.
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
            url: core.url.into_inner(),
            transcript_type: core.transcript_type.map(|t| t.to_string()),
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
    ///
    /// Note: URL from untrusted feed input. Validate before fetching.
    pub img: Option<String>,
    /// Person's URL/website
    ///
    /// Note: URL from untrusted feed input. Validate before fetching.
    pub href: Option<String>,
}

impl From<CorePodcastPerson> for PodcastPerson {
    fn from(core: CorePodcastPerson) -> Self {
        Self {
            name: core.name,
            role: core.role,
            group: core.group,
            img: core.img.map(core::Url::into_inner),
            href: core.href.map(core::Url::into_inner),
        }
    }
}

/// Podcast 2.0 geographic location (podcast:location)
#[napi(object)]
pub struct PodcastLocation {
    /// Human-readable location name
    pub name: String,
    /// Geographic coordinates (e.g., "geo:37.786971,-122.399677")
    pub geo: Option<String>,
    /// OpenStreetMap reference (e.g., "R113314")
    pub osm: Option<String>,
}

impl From<CorePodcastLocation> for PodcastLocation {
    fn from(core: CorePodcastLocation) -> Self {
        Self {
            name: core.name,
            geo: core.geo,
            osm: core.osm,
        }
    }
}

/// Podcast 2.0 text record (podcast:txt)
#[napi(object)]
pub struct PodcastTxt {
    /// Purpose of the text
    pub purpose: Option<String>,
    /// Text content
    pub value: String,
}

impl From<CorePodcastTxt> for PodcastTxt {
    fn from(core: CorePodcastTxt) -> Self {
        Self {
            purpose: core.purpose,
            value: core.value,
        }
    }
}

/// Podcast 2.0 update frequency (podcast:updateFrequency)
#[napi(object)]
pub struct PodcastUpdateFrequency {
    /// iCalendar RRULE string
    pub rrule: Option<String>,
    /// Whether the podcast is complete
    pub complete: Option<bool>,
    /// Start date in ISO 8601
    pub dtstart: Option<String>,
    /// Human-readable label
    pub label: Option<String>,
}

impl From<CorePodcastUpdateFrequency> for PodcastUpdateFrequency {
    fn from(core: CorePodcastUpdateFrequency) -> Self {
        Self {
            rrule: core.rrule,
            complete: core.complete,
            dtstart: core.dtstart,
            label: core.label,
        }
    }
}

/// Podcast 2.0 follow link (podcast:follow)
#[napi(object)]
pub struct PodcastFollow {
    /// Follow URL
    ///
    /// Note: URL from untrusted feed input. Validate before fetching.
    pub url: String,
    /// Platform name
    pub platform: Option<String>,
}

impl From<CorePodcastFollow> for PodcastFollow {
    fn from(core: CorePodcastFollow) -> Self {
        Self {
            url: core.url.into_inner(),
            platform: core.platform,
        }
    }
}

/// Podcast 2.0 social interaction thread (podcast:socialInteract)
#[napi(object)]
pub struct PodcastSocialInteract {
    /// Social thread URI
    ///
    /// Note: URL from untrusted feed input. Validate before fetching.
    pub uri: String,
    /// Social protocol: "activitypub", "twitter", etc.
    pub protocol: Option<String>,
    /// Account identifier
    #[napi(js_name = "accountId")]
    pub account_id: Option<String>,
    /// Account URL
    ///
    /// Note: URL from untrusted feed input. Validate before fetching.
    #[napi(js_name = "accountUrl")]
    pub account_url: Option<String>,
    /// Priority (lower = higher priority)
    pub priority: Option<u32>,
}

impl From<CorePodcastSocialInteract> for PodcastSocialInteract {
    fn from(core: CorePodcastSocialInteract) -> Self {
        Self {
            uri: core.uri.into_inner(),
            protocol: core.protocol,
            account_id: core.account_id,
            account_url: core.account_url.map(core::Url::into_inner),
            priority: core.priority,
        }
    }
}

/// Podcast 2.0 alternate enclosure (podcast:alternateEnclosure)
#[napi(object)]
pub struct PodcastAlternateEnclosure {
    /// MIME type
    #[napi(js_name = "type")]
    pub enclosure_type: String,
    /// File size in bytes
    ///
    /// Note: represented as `f64` (napi has `ToNapiValue` but no `FromNapiValue` for `u64`,
    /// which `#[napi(object)]` requires since object structs are bidirectional); exact up to
    /// 2^53 bytes.
    pub length: Option<f64>,
    /// Bitrate in kbps
    pub bitrate: Option<f64>,
    /// Video height in pixels
    pub height: Option<u32>,
    /// Language code
    pub lang: Option<String>,
    /// Title
    pub title: Option<String>,
    /// Relationship: "default", "alternate", etc.
    pub rel: Option<String>,
    /// Codecs string
    pub codecs: Option<String>,
    /// Whether this is the default enclosure
    pub default: Option<bool>,
    /// Source URIs for this enclosure
    pub sources: Vec<PodcastAlternateEnclosureSource>,
    /// Integrity verification
    pub integrity: Option<PodcastIntegrity>,
}

#[allow(clippy::cast_precision_loss)]
impl From<CorePodcastAlternateEnclosure> for PodcastAlternateEnclosure {
    fn from(core: CorePodcastAlternateEnclosure) -> Self {
        Self {
            enclosure_type: core.type_.to_string(),
            length: core.length.map(|l| l as f64),
            bitrate: core.bitrate,
            height: core.height,
            lang: core.lang,
            title: core.title,
            rel: core.rel,
            codecs: core.codecs,
            default: core.default,
            sources: core
                .sources
                .into_iter()
                .map(PodcastAlternateEnclosureSource::from)
                .collect(),
            integrity: core.integrity.map(PodcastIntegrity::from),
        }
    }
}

/// Podcast 2.0 alternate enclosure source
#[napi(object)]
pub struct PodcastAlternateEnclosureSource {
    /// Source URI
    ///
    /// Note: URL from untrusted feed input. Validate before fetching.
    pub uri: String,
    /// Optional MIME type override
    #[napi(js_name = "contentType")]
    pub content_type: Option<String>,
}

impl From<CorePodcastAlternateEnclosureSource> for PodcastAlternateEnclosureSource {
    fn from(core: CorePodcastAlternateEnclosureSource) -> Self {
        Self {
            uri: core.uri.into_inner(),
            content_type: core.content_type.map(|t| t.to_string()),
        }
    }
}

/// Podcast 2.0 integrity verification for alternate enclosures
#[napi(object)]
pub struct PodcastIntegrity {
    /// Integrity type: "sri" or "pgp-signature"
    #[napi(js_name = "type")]
    pub integrity_type: String,
    /// Integrity value
    pub value: String,
}

impl From<CorePodcastIntegrity> for PodcastIntegrity {
    fn from(core: CorePodcastIntegrity) -> Self {
        Self {
            integrity_type: core.type_,
            value: core.value,
        }
    }
}
