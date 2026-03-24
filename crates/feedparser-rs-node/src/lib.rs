#![deny(clippy::all)]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;

use feedparser_rs::{
    self as core, Content as CoreContent, Enclosure as CoreEnclosure, Entry as CoreEntry,
    FeedMeta as CoreFeedMeta, Generator as CoreGenerator, Image as CoreImage,
    InReplyTo as CoreInReplyTo, ItunesCategory as CoreItunesCategory,
    ItunesEntryMeta as CoreItunesEntryMeta, ItunesFeedMeta as CoreItunesFeedMeta,
    ItunesOwner as CoreItunesOwner, Link as CoreLink, MediaContent as CoreMediaContent,
    MediaCopyright as CoreMediaCopyright, MediaCredit as CoreMediaCredit,
    MediaRating as CoreMediaRating, MediaThumbnail as CoreMediaThumbnail,
    ParsedFeed as CoreParsedFeed, ParserLimits, Person as CorePerson,
    PodcastChapters as CorePodcastChapters, PodcastEntryMeta as CorePodcastEntryMeta,
    PodcastFunding as CorePodcastFunding, PodcastMeta as CorePodcastMeta,
    PodcastPerson as CorePodcastPerson, PodcastSoundbite as CorePodcastSoundbite,
    PodcastTranscript as CorePodcastTranscript, PodcastValue as CorePodcastValue,
    PodcastValueRecipient as CorePodcastValueRecipient, Source as CoreSource,
    SyndicationMeta as CoreSyndicationMeta, Tag as CoreTag, TextConstruct as CoreTextConstruct,
    TextType,
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
            status: core.status.map(|s| s as u32),
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
            update_frequency: core.update_frequency.clone(),
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
    /// Geographic location (GeoRSS), exposed as `where` per Python feedparser API
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
    /// JSON Feed next_url for pagination (JSON Feed 1.1)
    pub next_url: Option<String>,
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
            updated_parsed: core.updated.map(|dt| dt.timestamp_millis() as f64),
            published: core.published_str,
            published_parsed: core.published.map(|dt| dt.timestamp_millis() as f64),
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
            id: core.id.map(|s| s.to_string()),
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
    /// Geographic location (GeoRSS), exposed as `where` per Python feedparser API
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
    /// WFW namespace: comment RSS feed URL
    #[napi(js_name = "wfwCommentRss")]
    pub wfw_comment_rss: Option<String>,
    /// Whether the RSS `<guid>` is a permalink (`isPermaLink` attribute).
    ///
    /// `true` when `isPermaLink="true"` or attribute absent (RSS 2.0 default).
    /// `false` when `isPermaLink="false"`. `null` when no `<guid>` element present.
    #[napi(js_name = "guidislink")]
    pub guidislink: Option<bool>,
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
            published_parsed: core.published.map(|dt| dt.timestamp_millis() as f64),
            updated: core.updated_str,
            updated_parsed: core.updated.map(|dt| dt.timestamp_millis() as f64),
            created: core.created.as_ref().map(|dt| dt.to_rfc3339()),
            created_parsed: core.created.map(|dt| dt.timestamp_millis() as f64),
            expired: core.expired.as_ref().map(|dt| dt.to_rfc3339()),
            expired_parsed: core.expired.map(|dt| dt.timestamp_millis() as f64),
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
            dc_date_parsed: core.dc_date.map(|dt| dt.timestamp_millis() as f64),
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
            itunes: core.itunes.map(|b| ItunesEntryMeta::from(*b)),
            podcast: core.podcast.map(|b| PodcastEntryMeta::from(*b)),
            thr_in_reply_to: core.in_reply_to.into_iter().map(InReplyTo::from).collect(),
            thr_total: core.thr_total,
            slash_comments: core.slash_comments.map(|n| n.to_string()),
            wfw_comment_rss: core.wfw_comment_rss,
            guidislink: core.guidislink,
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
            thr_updated: core.thr_updated.as_ref().map(|dt| dt.to_rfc3339()),
            thr_updated_parsed: core.thr_updated.map(|dt| dt.timestamp_millis() as f64),
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
}

impl From<CorePerson> for Person {
    fn from(core: CorePerson) -> Self {
        Self {
            name: core.name.map(|s| s.to_string()),
            email: core.email.map(|e| e.into_inner()),
            href: core.uri,
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
}

impl From<CoreEnclosure> for Enclosure {
    fn from(core: CoreEnclosure) -> Self {
        Self {
            href: core.url.into_inner(),
            length: core.length,
            enclosure_type: core.enclosure_type.map(|t| t.to_string()),
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
            id: core.id.map(|s| s.to_string()),
            links: core.links.into_iter().map(Link::from).collect(),
            updated: core.updated_str,
            rights: core.rights,
            guidislink: core.guidislink,
        }
    }
}

/// Geographic location from GeoRSS namespace
#[napi(object)]
pub struct GeoLocation {
    /// Type of geographic shape ("point", "line", "polygon", "box")
    #[napi(js_name = "geoType")]
    pub geo_type: String,
    /// Coordinate pairs as nested array [[lat, lng], ...]
    ///
    /// Format depends on geo_type:
    /// - "point": Single pair [[lat, lng]]
    /// - "line": Two or more pairs [[lat1, lng1], [lat2, lng2], ...]
    /// - "box": Two pairs [[lower-left-lat, lower-left-lng], [upper-right-lat, upper-right-lng]]
    /// - "polygon": Three or more pairs forming a closed shape [[lat1, lng1], ..., [lat1, lng1]]
    pub coordinates: Vec<Vec<f64>>,
    /// Coordinate Reference System (e.g., "EPSG:4326" for WGS84 latitude/longitude)
    pub crs: Option<String>,
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
            crs: core.srs_name,
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
}

impl From<CoreMediaThumbnail> for MediaThumbnail {
    fn from(core: CoreMediaThumbnail) -> Self {
        Self {
            url: core.url.into_inner(),
            width: core.width,
            height: core.height,
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

/// Media RSS rating (media:rating element)
#[napi(object)]
pub struct MediaRating {
    /// Rating scheme (default: "urn:simple")
    pub scheme: Option<String>,
    /// Rating value (e.g., "nonadult", "adult")
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
            author: core.author.map(|s| s.to_string()),
            owner: core.owner.map(ItunesOwner::from),
            categories: core
                .categories
                .into_iter()
                .map(ItunesCategory::from)
                .collect(),
            explicit: core.explicit,
            image: core.image.map(|u| u.into_inner()),
            keywords: core.keywords,
            podcast_type: core.podcast_type,
            complete: core.complete,
            new_feed_url: core.new_feed_url.map(|u| u.into_inner()),
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
            name: core.name.map(|s| s.to_string()),
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
            author: core.author.map(|s| s.to_string()),
            duration: core.duration,
            explicit: core.explicit,
            image: core.image.map(|u| u.into_inner()),
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
            name: core.name.map(|s| s.to_string()),
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
    pub person: Vec<PodcastPerson>,
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
            person: core.person.into_iter().map(PodcastPerson::from).collect(),
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
            language: core.language.map(|s| s.to_string()),
            rel: core.rel.map(|s| s.to_string()),
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
            img: core.img.map(|u| u.into_inner()),
            href: core.href.map(|u| u.into_inner()),
        }
    }
}
