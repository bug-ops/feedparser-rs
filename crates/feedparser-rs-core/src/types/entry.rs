use super::{
    common::{
        Content, Enclosure, Link, MediaContent, MediaThumbnail, Person, Source, Tag, TextConstruct,
    },
    generics::LimitedCollectionExt,
    podcast::{ItunesEntryMeta, PodcastEntryMeta, PodcastPerson, PodcastTranscript},
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
    /// Short description/summary
    pub summary: Option<String>,
    /// Detailed summary with metadata
    pub summary_detail: Option<TextConstruct>,
    /// Full content blocks
    pub content: Vec<Content>,
    /// Publication date
    pub published: Option<DateTime<Utc>>,
    /// Last update date
    pub updated: Option<DateTime<Utc>>,
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
    pub fn set_title(&mut self, mut text: TextConstruct) {
        self.title = Some(std::mem::take(&mut text.value));
        self.title_detail = Some(text);
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
    pub fn set_summary(&mut self, mut text: TextConstruct) {
        self.summary = Some(std::mem::take(&mut text.value));
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
    pub fn set_author(&mut self, mut person: Person) {
        self.author = person.name.take();
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
