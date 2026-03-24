use super::{
    common::{
        Content, Enclosure, Link, MediaContent, MediaThumbnail, Person, Source, Tag, TextConstruct,
    },
    generics::LimitedCollectionExt,
    podcast::{ItunesEntryMeta, PodcastEntryMeta, PodcastPerson, PodcastTranscript},
    thread::InReplyTo,
};
use chrono::{DateTime, Utc};

/// Feed entry/item
#[derive(Debug, Clone, Default)]
pub struct Entry {
    /// Unique entry identifier (stored inline for IDs ≤24 bytes)
    pub id: Option<super::common::SmallString>,
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
    /// Publication date
    pub published: Option<DateTime<Utc>>,
    /// Original publication date string as found in the feed (timezone preserved)
    pub published_str: Option<String>,
    /// Last update date
    pub updated: Option<DateTime<Utc>>,
    /// Original update date string as found in the feed (timezone preserved)
    pub updated_str: Option<String>,
    /// Creation date
    pub created: Option<DateTime<Utc>>,
    /// Expiration date
    pub expired: Option<DateTime<Utc>>,
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
    /// Tags/categories
    pub tags: Vec<Tag>,
    /// Media enclosures (audio, video, etc.)
    pub enclosures: Vec<Enclosure>,
    /// Comments URL or text
    pub comments: Option<String>,
    /// Source feed reference
    pub source: Option<Source>,
    /// iTunes episode metadata (if present)
    pub itunes: Option<Box<ItunesEntryMeta>>,
    /// Dublin Core creator (author fallback) - stored inline for names ≤24 bytes
    pub dc_creator: Option<super::common::SmallString>,
    /// Dublin Core date (publication date fallback)
    pub dc_date: Option<DateTime<Utc>>,
    /// Dublin Core subjects (tags)
    pub dc_subject: Vec<String>,
    /// Dublin Core rights (copyright)
    pub dc_rights: Option<String>,
    /// Media RSS thumbnails
    pub media_thumbnails: Vec<MediaThumbnail>,
    /// Media RSS content items
    pub media_content: Vec<MediaContent>,
    /// Podcast 2.0 transcripts for this episode
    pub podcast_transcripts: Vec<PodcastTranscript>,
    /// Podcast 2.0 persons for this episode (hosts, guests, etc.)
    pub podcast_persons: Vec<PodcastPerson>,
    /// Podcast 2.0 episode metadata
    pub podcast: Option<Box<PodcastEntryMeta>>,
    /// `GeoRSS` location data
    pub geo: Option<Box<crate::namespace::georss::GeoLocation>>,
    /// License URL (Creative Commons, etc.)
    pub license: Option<String>,
    /// Atom Threading Extensions: entries this is a reply to (thr:in-reply-to)
    pub in_reply_to: Vec<InReplyTo>,
    /// Atom Threading Extensions: total response count (thr:total)
    ///
    /// Stored as u32 in Rust for type safety. Python binding converts
    /// to string to match Python feedparser's API.
    pub thr_total: Option<u32>,
    /// Slash namespace: comment count (`slash:comments`)
    pub slash_comments: Option<u32>,
    /// WFW namespace: comment RSS feed URL (`wfw:commentRss`)
    pub wfw_comment_rss: Option<String>,
}

impl Entry {
    /// Creates `Entry` with pre-allocated capacity for collections
    ///
    /// Pre-allocates space for typical entry fields:
    /// - 1-2 links (alternate, related)
    /// - 1 content block
    /// - 1 author
    /// - 2-3 tags
    /// - 0-1 enclosures
    /// - 2 podcast transcripts (typical for podcasts with multiple languages)
    /// - 4 podcast persons (host, co-hosts, guests)
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::Entry;
    ///
    /// let entry = Entry::with_capacity();
    /// ```
    #[must_use]
    pub fn with_capacity() -> Self {
        Self {
            links: Vec::with_capacity(2),
            content: Vec::with_capacity(1),
            authors: Vec::with_capacity(1),
            contributors: Vec::with_capacity(0),
            tags: Vec::with_capacity(3),
            enclosures: Vec::with_capacity(1),
            dc_subject: Vec::with_capacity(2),
            media_thumbnails: Vec::with_capacity(1),
            media_content: Vec::with_capacity(1),
            podcast_transcripts: Vec::with_capacity(2),
            podcast_persons: Vec::with_capacity(4),
            // Most entries reply to at most one parent
            in_reply_to: Vec::with_capacity(1),
            ..Default::default()
        }
    }

    /// Sets title field with `TextConstruct`, storing both simple and detailed versions
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::{Entry, TextConstruct};
    ///
    /// let mut entry = Entry::default();
    /// entry.set_title(TextConstruct::text("Great Article"));
    /// assert_eq!(entry.title.as_deref(), Some("Great Article"));
    /// ```
    #[inline]
    pub fn set_title(&mut self, text: TextConstruct) {
        self.title = Some(text.value.clone());
        self.title_detail = Some(text);
    }

    /// Sets subtitle field with `TextConstruct`, storing both simple and detailed versions
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::{Entry, TextConstruct};
    ///
    /// let mut entry = Entry::default();
    /// entry.set_subtitle(TextConstruct::text("A teaser"));
    /// assert_eq!(entry.subtitle.as_deref(), Some("A teaser"));
    /// ```
    #[inline]
    pub fn set_subtitle(&mut self, text: TextConstruct) {
        self.subtitle = Some(text.value.clone());
        self.subtitle_detail = Some(text);
    }

    /// Sets rights field with `TextConstruct`, storing both simple and detailed versions
    #[inline]
    pub fn set_rights(&mut self, text: TextConstruct) {
        self.rights = Some(text.value.clone());
        self.rights_detail = Some(text);
    }

    /// Sets summary field with `TextConstruct`, storing both simple and detailed versions
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::{Entry, TextConstruct};
    ///
    /// let mut entry = Entry::default();
    /// entry.set_summary(TextConstruct::text("A summary"));
    /// assert_eq!(entry.summary.as_deref(), Some("A summary"));
    /// ```
    #[inline]
    pub fn set_summary(&mut self, text: TextConstruct) {
        self.summary = Some(text.value.clone());
        self.summary_detail = Some(text);
    }

    /// Sets author field with `Person`, storing both simple and detailed versions
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::{Entry, Person};
    ///
    /// let mut entry = Entry::default();
    /// entry.set_author(Person::from_name("Jane Doe"));
    /// assert_eq!(entry.author.as_deref(), Some("Jane Doe"));
    /// ```
    #[inline]
    pub fn set_author(&mut self, person: Person) {
        self.author.clone_from(&person.name);
        self.author_detail = Some(person);
    }

    /// Sets publisher field with `Person`, storing both simple and detailed versions
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::{Entry, Person};
    ///
    /// let mut entry = Entry::default();
    /// entry.set_publisher(Person::from_name("ACME Corp"));
    /// assert_eq!(entry.publisher.as_deref(), Some("ACME Corp"));
    /// ```
    #[inline]
    pub fn set_publisher(&mut self, person: Person) {
        self.publisher.clone_from(&person.name);
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
    /// use feedparser_rs::Entry;
    ///
    /// let mut entry = Entry::default();
    /// entry.set_alternate_link("https://example.com/post/1".to_string(), 10);
    /// assert_eq!(entry.link.as_deref(), Some("https://example.com/post/1"));
    /// assert_eq!(entry.links.len(), 1);
    /// assert_eq!(entry.links[0].rel.as_deref(), Some("alternate"));
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
    fn test_entry_default() {
        let entry = Entry::default();
        assert!(entry.id.is_none());
        assert!(entry.title.is_none());
        assert!(entry.links.is_empty());
        assert!(entry.content.is_empty());
        assert!(entry.authors.is_empty());
    }

    #[test]
    #[allow(clippy::redundant_clone)]
    fn test_entry_clone() {
        fn create_entry() -> Entry {
            Entry {
                title: Some("Test".to_string()),
                links: vec![Link::default()],
                ..Default::default()
            }
        }
        let entry = create_entry();
        let cloned = entry.clone();
        assert_eq!(cloned.title.as_deref(), Some("Test"));
        assert_eq!(cloned.links.len(), 1);
    }
}
