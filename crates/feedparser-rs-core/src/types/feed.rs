use super::{
    common::{Generator, Image, Link, Person, Tag, TextConstruct},
    entry::Entry,
    generics::LimitedCollectionExt,
    podcast::{ItunesFeedMeta, PodcastMeta},
    version::FeedVersion,
};
use crate::namespace::syndication::SyndicationMeta;
use crate::{ParserLimits, error::Result};
use chrono::{DateTime, Utc};
use quick_xml::Reader;
use std::collections::HashMap;

/// Feed metadata
#[derive(Debug, Clone, Default)]
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
    /// Last update date
    pub updated: Option<DateTime<Utc>>,
    /// Initial publication date (RSS pubDate, Atom published)
    pub published: Option<DateTime<Utc>>,
    /// Primary author name (stored inline for names ≤24 bytes)
    pub author: Option<super::common::SmallString>,
    /// Detailed author information
    pub author_detail: Option<Person>,
    /// All authors
    pub authors: Vec<Person>,
    /// Contributors
    pub contributors: Vec<Person>,
    /// Publisher name (stored inline for names ≤24 bytes)
    pub publisher: Option<super::common::SmallString>,
    /// Detailed publisher information
    pub publisher_detail: Option<Person>,
    /// Feed language (e.g., "en-us") - stored inline as lang codes are ≤24 bytes
    pub language: Option<super::common::SmallString>,
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
    /// iTunes podcast metadata (if present)
    pub itunes: Option<Box<ItunesFeedMeta>>,
    /// Podcast 2.0 namespace metadata (if present)
    pub podcast: Option<Box<PodcastMeta>>,
    /// Dublin Core creator (author fallback) - stored inline for names ≤24 bytes
    pub dc_creator: Option<super::common::SmallString>,
    /// Dublin Core publisher (stored inline for names ≤24 bytes)
    pub dc_publisher: Option<super::common::SmallString>,
    /// Dublin Core rights (copyright)
    pub dc_rights: Option<String>,
    /// License URL (Creative Commons, etc.)
    pub license: Option<String>,
    /// Syndication module metadata (RSS 1.0)
    pub syndication: Option<Box<SyndicationMeta>>,
    /// Geographic location from `GeoRSS` namespace (feed level)
    pub geo: Option<Box<crate::namespace::georss::GeoLocation>>,
}

/// Parsed feed result
///
/// This is the main result type returned by the parser, analogous to
/// Python feedparser's `FeedParserDict`.
#[derive(Debug, Clone, Default)]
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
    pub version: FeedVersion,
    /// XML namespaces (prefix -> URI)
    pub namespaces: HashMap<String, String>,
    /// HTTP status code (if fetched from URL)
    pub status: Option<u16>,
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

impl ParsedFeed {
    /// Creates a new `ParsedFeed` with default UTF-8 encoding
    #[must_use]
    pub fn new() -> Self {
        Self {
            encoding: String::from("utf-8"),
            ..Default::default()
        }
    }

    /// Creates a `ParsedFeed` with pre-allocated capacity for entries
    ///
    /// This method pre-allocates space for the expected number of entries,
    /// reducing memory allocations during parsing.
    ///
    /// # Arguments
    ///
    /// * `entry_count` - Expected number of entries in the feed
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::ParsedFeed;
    ///
    /// let feed = ParsedFeed::with_capacity(50);
    /// assert_eq!(feed.encoding, "utf-8");
    /// ```
    #[must_use]
    pub fn with_capacity(entry_count: usize) -> Self {
        Self {
            entries: Vec::with_capacity(entry_count),
            namespaces: HashMap::with_capacity(8), // Typical feeds have 3-8 namespaces
            encoding: String::from("utf-8"),
            ..Default::default()
        }
    }

    /// Check if entry limit is reached, set bozo flag and skip element if so
    ///
    /// This helper consolidates the duplicate entry limit checking logic used in
    /// RSS and Atom parsers. If the entry limit is reached, it:
    /// - Sets `bozo` flag to true
    /// - Sets `bozo_exception` with descriptive error message
    /// - Skips the entry element
    /// - Returns `Ok(false)` to signal that the entry should not be processed
    ///
    /// # Arguments
    ///
    /// * `reader` - XML reader positioned at the entry element
    /// * `buf` - Buffer for XML event reading
    /// * `limits` - Parser limits including `max_entries`
    /// * `depth` - Current nesting depth (will be decremented)
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Entry can be processed (limit not reached)
    /// * `Ok(false)` - Entry limit reached, element was skipped
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Skipping the entry element fails (e.g., malformed XML)
    /// - Nesting depth exceeds limits while skipping
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // In parser:
    /// if !feed.check_entry_limit(reader, &mut buf, limits, depth)? {
    ///     continue;
    /// }
    /// // Process entry...
    /// ```
    #[inline]
    pub fn check_entry_limit(
        &mut self,
        reader: &mut Reader<&[u8]>,
        buf: &mut Vec<u8>,
        limits: &ParserLimits,
        depth: &mut usize,
    ) -> Result<bool> {
        use crate::parser::skip_element;

        if self.entries.is_at_limit(limits.max_entries) {
            self.bozo = true;
            self.bozo_exception = Some(format!("Entry limit exceeded: {}", limits.max_entries));
            skip_element(reader, buf, limits, *depth)?;
            *depth = depth.saturating_sub(1);
            Ok(false)
        } else {
            Ok(true)
        }
    }
}

impl FeedMeta {
    /// Creates `FeedMeta` with capacity hints for typical RSS 2.0 feeds
    ///
    /// Pre-allocates collections based on common RSS 2.0 field usage:
    /// - 1-2 links (channel link, self link)
    /// - 1 author (managingEditor)
    /// - 0-3 tags (categories)
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::FeedMeta;
    ///
    /// let meta = FeedMeta::with_rss_capacity();
    /// ```
    #[must_use]
    pub fn with_rss_capacity() -> Self {
        Self {
            links: Vec::with_capacity(2),
            authors: Vec::with_capacity(1),
            contributors: Vec::with_capacity(0),
            tags: Vec::with_capacity(3),
            ..Default::default()
        }
    }

    /// Creates `FeedMeta` with capacity hints for typical Atom 1.0 feeds
    ///
    /// Pre-allocates collections based on common Atom 1.0 field usage:
    /// - 3-5 links (alternate, self, related, etc.)
    /// - 1-2 authors
    /// - 1 contributor
    /// - 3-5 tags (categories)
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::FeedMeta;
    ///
    /// let meta = FeedMeta::with_atom_capacity();
    /// ```
    #[must_use]
    pub fn with_atom_capacity() -> Self {
        Self {
            links: Vec::with_capacity(4),
            authors: Vec::with_capacity(2),
            contributors: Vec::with_capacity(1),
            tags: Vec::with_capacity(5),
            ..Default::default()
        }
    }

    /// Sets title field with `TextConstruct`, storing both simple and detailed versions
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::{FeedMeta, TextConstruct};
    ///
    /// let mut meta = FeedMeta::default();
    /// meta.set_title(TextConstruct::text("Example Feed"));
    /// assert_eq!(meta.title.as_deref(), Some("Example Feed"));
    /// ```
    #[inline]
    pub fn set_title(&mut self, mut text: TextConstruct) {
        self.title = Some(std::mem::take(&mut text.value));
        self.title_detail = Some(text);
    }

    /// Sets subtitle field with `TextConstruct`, storing both simple and detailed versions
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::{FeedMeta, TextConstruct};
    ///
    /// let mut meta = FeedMeta::default();
    /// meta.set_subtitle(TextConstruct::text("A great feed"));
    /// assert_eq!(meta.subtitle.as_deref(), Some("A great feed"));
    /// ```
    #[inline]
    pub fn set_subtitle(&mut self, mut text: TextConstruct) {
        self.subtitle = Some(std::mem::take(&mut text.value));
        self.subtitle_detail = Some(text);
    }

    /// Sets rights field with `TextConstruct`, storing both simple and detailed versions
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::{FeedMeta, TextConstruct};
    ///
    /// let mut meta = FeedMeta::default();
    /// meta.set_rights(TextConstruct::text("© 2025 Example"));
    /// assert_eq!(meta.rights.as_deref(), Some("© 2025 Example"));
    /// ```
    #[inline]
    pub fn set_rights(&mut self, mut text: TextConstruct) {
        self.rights = Some(std::mem::take(&mut text.value));
        self.rights_detail = Some(text);
    }

    /// Sets generator field with `Generator`, storing both simple and detailed versions
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::{FeedMeta, Generator};
    ///
    /// # fn main() {
    /// let mut meta = FeedMeta::default();
    /// let generator = Generator {
    ///     value: "Example Generator".to_string(),
    ///     uri: None,
    ///     version: None,
    /// };
    /// meta.set_generator(generator);
    /// assert_eq!(meta.generator.as_deref(), Some("Example Generator"));
    /// # }
    /// ```
    #[inline]
    pub fn set_generator(&mut self, mut generator: Generator) {
        self.generator = Some(std::mem::take(&mut generator.value));
        self.generator_detail = Some(generator);
    }

    /// Sets author field with `Person`, storing both simple and detailed versions
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::{FeedMeta, Person};
    ///
    /// let mut meta = FeedMeta::default();
    /// meta.set_author(Person::from_name("John Doe"));
    /// assert_eq!(meta.author.as_deref(), Some("John Doe"));
    /// ```
    #[inline]
    pub fn set_author(&mut self, mut person: Person) {
        self.author = person.name.take();
        self.author_detail = Some(person);
    }

    /// Sets publisher field with `Person`, storing both simple and detailed versions
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::{FeedMeta, Person};
    ///
    /// let mut meta = FeedMeta::default();
    /// meta.set_publisher(Person::from_name("ACME Corp"));
    /// assert_eq!(meta.publisher.as_deref(), Some("ACME Corp"));
    /// ```
    #[inline]
    pub fn set_publisher(&mut self, mut person: Person) {
        self.publisher = person.name.take();
        self.publisher_detail = Some(person);
    }

    /// Sets the primary link and adds it to the links collection
    ///
    /// This is a convenience method that:
    /// 1. Sets the `link` field (if not already set)
    /// 2. Adds an "alternate" link to the `links` collection
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::FeedMeta;
    ///
    /// let mut meta = FeedMeta::default();
    /// meta.set_alternate_link("https://example.com".to_string(), 10);
    /// assert_eq!(meta.link.as_deref(), Some("https://example.com"));
    /// assert_eq!(meta.links.len(), 1);
    /// assert_eq!(meta.links[0].rel.as_deref(), Some("alternate"));
    /// ```
    #[inline]
    pub fn set_alternate_link(&mut self, href: String, max_links: usize) {
        if self.link.is_none() {
            self.link = Some(href.clone());
        }
        self.links.try_push_limited(
            Link {
                href: href.into(),
                rel: Some("alternate".into()),
                ..Default::default()
            },
            max_links,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feed_meta_default() {
        let meta = FeedMeta::default();
        assert!(meta.title.is_none());
        assert!(meta.links.is_empty());
        assert!(meta.authors.is_empty());
    }

    #[test]
    fn test_parsed_feed_default() {
        let feed = ParsedFeed::default();
        assert!(!feed.bozo);
        assert!(feed.bozo_exception.is_none());
        assert_eq!(feed.version, FeedVersion::Unknown);
        assert!(feed.entries.is_empty());
    }

    #[test]
    fn test_parsed_feed_new() {
        let feed = ParsedFeed::new();
        assert_eq!(feed.encoding, "utf-8");
        assert!(!feed.bozo);
    }

    #[test]
    fn test_parsed_feed_clone() {
        let feed = ParsedFeed {
            version: FeedVersion::Rss20,
            bozo: true,
            ..ParsedFeed::new()
        };

        assert_eq!(feed.version, FeedVersion::Rss20);
        assert!(feed.bozo);
    }
}
