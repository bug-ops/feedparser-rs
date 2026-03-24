use super::generics::{FromAttributes, ParseFrom};
use crate::util::{date::parse_date, text::bytes_to_string};
use chrono::{DateTime, Utc};
use compact_str::CompactString;
use serde_json::Value;
use std::ops::Deref;
use std::sync::Arc;

/// Optimized string type for small strings (≤24 bytes stored inline)
///
/// Uses `CompactString` which stores strings up to 24 bytes inline without heap allocation.
/// This significantly reduces allocations for common short strings like language codes,
/// author names, category terms, and other metadata fields.
///
/// `CompactString` implements `Deref<Target=str>`, so it can be used transparently as a string.
///
/// # Examples
///
/// ```
/// use feedparser_rs::types::SmallString;
///
/// let s: SmallString = "en-US".into();
/// assert_eq!(s.as_str(), "en-US");
/// assert_eq!(s.len(), 5); // Stored inline, no heap allocation
/// ```
pub type SmallString = CompactString;

/// URL newtype for type-safe URL handling
///
/// Provides a semantic wrapper around string URLs without validation.
/// Following the bozo pattern, URLs are not validated during parsing.
///
/// # Examples
///
/// ```
/// use feedparser_rs::Url;
///
/// let url = Url::new("https://example.com");
/// assert_eq!(url.as_str(), "https://example.com");
///
/// // Deref coercion allows transparent string access
/// let len: usize = url.len();
/// assert_eq!(len, 19);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Url(String);

impl Url {
    /// Creates a new URL from any type that can be converted to a String
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::Url;
    ///
    /// let url1 = Url::new("https://example.com");
    /// let url2 = Url::new(String::from("https://example.com"));
    /// assert_eq!(url1, url2);
    /// ```
    #[inline]
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the URL as a string slice
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::Url;
    ///
    /// let url = Url::new("https://example.com");
    /// assert_eq!(url.as_str(), "https://example.com");
    /// ```
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the `Url` and returns the inner `String`
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::Url;
    ///
    /// let url = Url::new("https://example.com");
    /// let inner: String = url.into_inner();
    /// assert_eq!(inner, "https://example.com");
    /// ```
    #[inline]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Deref for Url {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<String> for Url {
    #[inline]
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Url {
    #[inline]
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for Url {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Url {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq<str> for Url {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for Url {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<String> for Url {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

/// MIME type newtype with string interning
///
/// Uses `Arc<str>` for efficient cloning of common MIME types.
/// Multiple references to the same MIME type share the same allocation.
///
/// # Examples
///
/// ```
/// use feedparser_rs::MimeType;
///
/// let mime = MimeType::new("text/html");
/// assert_eq!(mime.as_str(), "text/html");
///
/// // Cloning is cheap (just increments reference count)
/// let clone = mime.clone();
/// assert_eq!(mime, clone);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MimeType(Arc<str>);

// Custom serde implementation for MimeType since Arc<str> doesn't implement Serialize/Deserialize
impl serde::Serialize for MimeType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> serde::Deserialize<'de> for MimeType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String as serde::Deserialize>::deserialize(deserializer)?;
        Ok(Self::new(s))
    }
}

impl MimeType {
    /// Creates a new MIME type from any string-like type
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::MimeType;
    ///
    /// let mime = MimeType::new("application/json");
    /// assert_eq!(mime.as_str(), "application/json");
    /// ```
    #[inline]
    pub fn new(s: impl AsRef<str>) -> Self {
        Self(Arc::from(s.as_ref()))
    }

    /// Returns the MIME type as a string slice
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::MimeType;
    ///
    /// let mime = MimeType::new("text/plain");
    /// assert_eq!(mime.as_str(), "text/plain");
    /// ```
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Common MIME type constants for convenience.
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::MimeType;
    ///
    /// let html = MimeType::new(MimeType::TEXT_HTML);
    /// assert_eq!(html.as_str(), "text/html");
    /// ```
    pub const TEXT_HTML: &'static str = "text/html";

    /// `text/plain` MIME type constant
    pub const TEXT_PLAIN: &'static str = "text/plain";

    /// `application/xml` MIME type constant
    pub const APPLICATION_XML: &'static str = "application/xml";

    /// `application/json` MIME type constant
    pub const APPLICATION_JSON: &'static str = "application/json";
}

impl Default for MimeType {
    #[inline]
    fn default() -> Self {
        Self(Arc::from(""))
    }
}

impl Deref for MimeType {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<String> for MimeType {
    #[inline]
    fn from(s: String) -> Self {
        Self(Arc::from(s.as_str()))
    }
}

impl From<&str> for MimeType {
    #[inline]
    fn from(s: &str) -> Self {
        Self(Arc::from(s))
    }
}

impl AsRef<str> for MimeType {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MimeType {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq<str> for MimeType {
    fn eq(&self, other: &str) -> bool {
        &*self.0 == other
    }
}

impl PartialEq<&str> for MimeType {
    fn eq(&self, other: &&str) -> bool {
        &*self.0 == *other
    }
}

impl PartialEq<String> for MimeType {
    fn eq(&self, other: &String) -> bool {
        &*self.0 == other
    }
}

/// Email newtype for type-safe email handling
///
/// Provides a semantic wrapper around email addresses without validation.
/// Following the bozo pattern, emails are not validated during parsing.
///
/// # Examples
///
/// ```
/// use feedparser_rs::Email;
///
/// let email = Email::new("user@example.com");
/// assert_eq!(email.as_str(), "user@example.com");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Email(String);

impl Email {
    /// Creates a new email from any type that can be converted to a String
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::Email;
    ///
    /// let email = Email::new("user@example.com");
    /// assert_eq!(email.as_str(), "user@example.com");
    /// ```
    #[inline]
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the email as a string slice
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::Email;
    ///
    /// let email = Email::new("user@example.com");
    /// assert_eq!(email.as_str(), "user@example.com");
    /// ```
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the `Email` and returns the inner `String`
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::Email;
    ///
    /// let email = Email::new("user@example.com");
    /// let inner: String = email.into_inner();
    /// assert_eq!(inner, "user@example.com");
    /// ```
    #[inline]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Deref for Email {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<String> for Email {
    #[inline]
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Email {
    #[inline]
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for Email {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Email {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq<str> for Email {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for Email {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<String> for Email {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

/// Link in feed or entry
#[derive(Debug, Clone, Default)]
pub struct Link {
    /// Link URL
    pub href: Url,
    /// Link relationship type (e.g., "alternate", "enclosure", "self")
    /// Stored inline as these are typically short (≤24 bytes)
    pub rel: Option<SmallString>,
    /// MIME type of the linked resource
    pub link_type: Option<MimeType>,
    /// Human-readable link title
    pub title: Option<String>,
    /// Length of the linked resource in bytes (raw string value, as in Python feedparser)
    pub length: Option<String>,
    /// Language of the linked resource (stored inline for lang codes ≤24 bytes)
    pub hreflang: Option<SmallString>,
    /// RFC 4685 §4: number of replies at the IRI
    pub thr_count: Option<u32>,
    /// RFC 4685 §4: when the reply resource was last modified
    pub thr_updated: Option<DateTime<Utc>>,
}

impl Link {
    /// Create a new link with just URL and relation type
    #[inline]
    pub fn new(href: impl Into<Url>, rel: impl AsRef<str>) -> Self {
        Self {
            href: href.into(),
            rel: Some(rel.as_ref().into()),
            link_type: None,
            title: None,
            length: None,
            hreflang: None,
            thr_count: None,
            thr_updated: None,
        }
    }

    /// Create an alternate link (common for entry URLs)
    #[inline]
    pub fn alternate(href: impl Into<Url>) -> Self {
        Self::new(href, "alternate")
    }

    /// Create a self link (for feed URLs)
    #[inline]
    pub fn self_link(href: impl Into<Url>, mime_type: impl Into<MimeType>) -> Self {
        Self {
            href: href.into(),
            rel: Some("self".into()),
            link_type: Some(mime_type.into()),
            title: None,
            length: None,
            hreflang: None,
            thr_count: None,
            thr_updated: None,
        }
    }

    /// Create an enclosure link (for media)
    #[inline]
    pub fn enclosure(href: impl Into<Url>, mime_type: Option<MimeType>) -> Self {
        Self {
            href: href.into(),
            rel: Some("enclosure".into()),
            link_type: mime_type,
            title: None,
            length: None,
            hreflang: None,
            thr_count: None,
            thr_updated: None,
        }
    }

    /// Create a related link
    #[inline]
    pub fn related(href: impl Into<Url>) -> Self {
        Self::new(href, "related")
    }

    /// Create a banner image link (JSON Feed `banner_image`, project-internal convention)
    ///
    /// Note: `rel="banner"` is not a standard link relation. It is used here as a project
    /// convention to store JSON Feed `banner_image` in the existing `Link` type. Consumers
    /// must search `entry.links` for `rel="banner"` to retrieve this field.
    #[inline]
    pub fn banner(href: impl Into<Url>) -> Self {
        Self::new(href, "banner")
    }

    /// Set MIME type (builder pattern)
    #[inline]
    #[must_use]
    pub fn with_type(mut self, mime_type: impl Into<MimeType>) -> Self {
        self.link_type = Some(mime_type.into());
        self
    }
}

/// Person (author, contributor, etc.)
#[derive(Debug, Clone, Default)]
pub struct Person {
    /// Person's name (stored inline for names ≤24 bytes)
    pub name: Option<SmallString>,
    /// Person's email address
    pub email: Option<Email>,
    /// Person's URI/website
    pub uri: Option<String>,
}

impl Person {
    /// Create person from just a name
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::types::Person;
    ///
    /// let person = Person::from_name("John Doe");
    /// assert_eq!(person.name.as_deref(), Some("John Doe"));
    /// assert!(person.email.is_none());
    /// assert!(person.uri.is_none());
    /// ```
    #[inline]
    pub fn from_name(name: impl AsRef<str>) -> Self {
        Self {
            name: Some(name.as_ref().into()),
            email: None,
            uri: None,
        }
    }

    /// Build flat author string in `"Name (email)"` format when email is present.
    ///
    /// Returns `None` if neither name nor email is set.
    #[must_use]
    pub fn flat_string(&self) -> Option<SmallString> {
        match (&self.name, &self.email) {
            (Some(name), Some(email)) => Some(format!("{name} ({email})").into()),
            (Some(name), None) => Some(name.clone()),
            (None, Some(email)) => Some(email.as_str().into()),
            (None, None) => None,
        }
    }
}

/// Tag/category
#[derive(Debug, Clone)]
pub struct Tag {
    /// Tag term/label (stored inline for terms ≤24 bytes)
    pub term: SmallString,
    /// Tag scheme/domain (stored inline for schemes ≤24 bytes)
    pub scheme: Option<SmallString>,
    /// Human-readable tag label (stored inline for labels ≤24 bytes)
    pub label: Option<SmallString>,
}

impl Tag {
    /// Create a simple tag with just term
    #[inline]
    pub fn new(term: impl AsRef<str>) -> Self {
        Self {
            term: term.as_ref().into(),
            scheme: None,
            label: None,
        }
    }
}

/// Image metadata
#[derive(Debug, Clone)]
pub struct Image {
    /// Image URL
    pub url: Url,
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

/// Enclosure (attached media file)
#[derive(Debug, Clone)]
pub struct Enclosure {
    /// Enclosure URL
    pub url: Url,
    /// File size in bytes (raw string value, as in Python feedparser)
    pub length: Option<String>,
    /// MIME type
    pub enclosure_type: Option<MimeType>,
}

/// Content block
#[derive(Debug, Clone)]
pub struct Content {
    /// Content body
    pub value: String,
    /// Content MIME type
    pub content_type: Option<MimeType>,
    /// Content language (stored inline for lang codes ≤24 bytes)
    pub language: Option<SmallString>,
    /// Base URL for relative links
    pub base: Option<String>,
    /// Out-of-line content URL (Atom `<content src="...">`, RFC 4287 §4.1.3.2)
    pub src: Option<String>,
}

impl Content {
    /// Create HTML content
    #[inline]
    pub fn html(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            content_type: Some(MimeType::new(MimeType::TEXT_HTML)),
            language: None,
            base: None,
            src: None,
        }
    }

    /// Create plain text content
    #[inline]
    pub fn plain(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            content_type: Some(MimeType::new(MimeType::TEXT_PLAIN)),
            language: None,
            base: None,
            src: None,
        }
    }
}

/// Text construct type (Atom-style)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextType {
    /// Plain text
    Text,
    /// HTML content
    Html,
    /// XHTML content
    Xhtml,
}

/// Text construct with metadata
#[derive(Debug, Clone)]
pub struct TextConstruct {
    /// Text content
    pub value: String,
    /// Content type
    pub content_type: TextType,
    /// Content language (stored inline for lang codes ≤24 bytes)
    pub language: Option<SmallString>,
    /// Base URL for relative links
    pub base: Option<String>,
}

impl TextConstruct {
    /// Create plain text construct
    #[inline]
    pub fn text(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            content_type: TextType::Text,
            language: None,
            base: None,
        }
    }

    /// Create HTML text construct
    #[inline]
    pub fn html(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            content_type: TextType::Html,
            language: None,
            base: None,
        }
    }

    /// Set language (builder pattern)
    #[inline]
    #[must_use]
    pub fn with_language(mut self, language: impl AsRef<str>) -> Self {
        self.language = Some(language.as_ref().into());
        self
    }
}

/// Generator metadata
#[derive(Debug, Clone)]
pub struct Generator {
    /// Generator name (text content of the `<generator>` element)
    pub name: String,
    /// Generator URI (href attribute)
    pub href: Option<String>,
    /// Generator version (stored inline for versions ≤24 bytes)
    pub version: Option<SmallString>,
}

/// Source reference (for entries)
#[derive(Debug, Clone)]
pub struct Source {
    /// Source title
    pub title: Option<String>,
    /// Primary source URL for RSS `<source url="...">` (RSS-only field)
    pub href: Option<String>,
    /// Primary source URL for Atom `<source><link href="..."/>` (Atom-only field)
    pub link: Option<String>,
    /// Source author (flat string, Atom `<source><author>`)
    pub author: Option<String>,
    /// Source unique identifier
    pub id: Option<String>,
    /// All links from the source element
    pub links: Vec<Link>,
    /// Last update date (Atom `<updated>`)
    pub updated: Option<DateTime<Utc>>,
    /// Original update date string (timezone preserved)
    pub updated_str: Option<String>,
    /// Rights/copyright statement (Atom `<rights>`)
    pub rights: Option<String>,
    /// Whether `<id>` was used as the link.
    ///
    /// `Some(true)` when `<id>` looks like a URL and no explicit `<link>` was present.
    /// `Some(false)` when `<id>` is present but a `<link>` was also present, or `<id>` is not a URL.
    /// `None` for RSS sources (RSS `<source>` has no `<id>`).
    pub guidislink: Option<bool>,
}

/// Media RSS thumbnail
#[derive(Debug, Clone)]
pub struct MediaThumbnail {
    /// Thumbnail URL
    ///
    /// # Security Warning
    ///
    /// This URL comes from untrusted feed input and has NOT been validated for SSRF.
    /// Applications MUST validate URLs before fetching to prevent SSRF attacks.
    pub url: Url,
    /// Thumbnail width in pixels (raw string value, as in Python feedparser)
    pub width: Option<String>,
    /// Thumbnail height in pixels (raw string value, as in Python feedparser)
    pub height: Option<String>,
}

/// Media RSS content
#[derive(Debug, Clone)]
pub struct MediaContent {
    /// Media URL
    ///
    /// # Security Warning
    ///
    /// This URL comes from untrusted feed input and has NOT been validated for SSRF.
    /// Applications MUST validate URLs before fetching to prevent SSRF attacks.
    pub url: Url,
    /// MIME type
    pub content_type: Option<MimeType>,
    /// Medium type: "image", "video", "audio", "document", "executable"
    pub medium: Option<String>,
    /// File size in bytes (raw string value, as in Python feedparser)
    pub filesize: Option<String>,
    /// Media width in pixels (raw string value, as in Python feedparser)
    pub width: Option<String>,
    /// Media height in pixels (raw string value, as in Python feedparser)
    pub height: Option<String>,
    /// Duration in seconds (raw string value, as in Python feedparser)
    pub duration: Option<String>,
    /// Bitrate in kilobits per second (raw string value)
    pub bitrate: Option<String>,
    /// Language of the media (lang attribute)
    pub lang: Option<String>,
    /// Number of audio channels (raw string value)
    pub channels: Option<String>,
    /// Codec used to produce the media (codec attribute)
    pub codec: Option<String>,
    /// Expression type: "full", "sample", "nonstop"
    pub expression: Option<String>,
    /// Whether this is the default media object (isDefault attribute, raw string)
    pub isdefault: Option<String>,
    /// Sampling rate in kHz (raw string value)
    pub samplingrate: Option<String>,
    /// Frame rate in frames per second (raw string value)
    pub framerate: Option<String>,
}

/// Media RSS rating element (`media:rating`)
///
/// Describes the permissible audience for the media content.
/// Commonly uses the MPAA or urn:simple rating schemes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MediaRating {
    /// Rating scheme URI (scheme attribute), e.g. "urn:simple", "urn:mpaa"
    pub scheme: Option<String>,
    /// Rating value, e.g. "adult", "nonadult", "pg-13"
    pub content: String,
}

impl FromAttributes for Link {
    fn from_attributes<'a, I>(attrs: I, max_attr_length: usize) -> Option<Self>
    where
        I: Iterator<Item = quick_xml::events::attributes::Attribute<'a>>,
    {
        let mut href = None;
        let mut rel = None;
        let mut link_type = None;
        let mut title = None;
        let mut hreflang = None;
        let mut length = None;
        let mut thr_count = None;
        let mut thr_updated = None;

        for attr in attrs {
            if attr.value.len() > max_attr_length {
                continue;
            }
            match attr.key.as_ref() {
                b"href" => href = Some(bytes_to_string(&attr.value)),
                b"rel" => rel = Some(bytes_to_string(&attr.value)),
                b"type" => link_type = Some(bytes_to_string(&attr.value)),
                b"title" => title = Some(bytes_to_string(&attr.value)),
                b"hreflang" => hreflang = Some(bytes_to_string(&attr.value)),
                b"length" => length = Some(bytes_to_string(&attr.value)),
                b"thr:count" => {
                    thr_count = bytes_to_string(&attr.value).trim().parse::<u32>().ok();
                }
                b"thr:updated" => {
                    thr_updated = parse_date(bytes_to_string(&attr.value).trim());
                }
                _ => {}
            }
        }

        let rel_str: Option<SmallString> = rel
            .map(std::convert::Into::into)
            .or_else(|| Some("alternate".into()));
        let resolved_type = link_type
            .map(MimeType::new)
            .or_else(|| match rel_str.as_deref() {
                Some("self") => Some(MimeType::new("application/atom+xml")),
                _ => Some(MimeType::new("text/html")),
            });

        href.map(|href| Self {
            href: Url::new(href),
            rel: rel_str,
            link_type: resolved_type,
            title,
            length,
            hreflang: hreflang.map(std::convert::Into::into),
            thr_count,
            thr_updated,
        })
    }
}

impl FromAttributes for Tag {
    fn from_attributes<'a, I>(attrs: I, max_attr_length: usize) -> Option<Self>
    where
        I: Iterator<Item = quick_xml::events::attributes::Attribute<'a>>,
    {
        let mut term = None;
        let mut scheme = None;
        let mut label = None;

        for attr in attrs {
            if attr.value.len() > max_attr_length {
                continue;
            }

            match attr.key.as_ref() {
                b"term" => term = Some(bytes_to_string(&attr.value)),
                b"scheme" | b"domain" => scheme = Some(bytes_to_string(&attr.value)),
                b"label" => label = Some(bytes_to_string(&attr.value)),
                _ => {}
            }
        }

        term.map(|term| Self {
            term: term.into(),
            scheme: scheme.map(std::convert::Into::into),
            label: label.map(std::convert::Into::into),
        })
    }
}

impl FromAttributes for Enclosure {
    fn from_attributes<'a, I>(attrs: I, max_attr_length: usize) -> Option<Self>
    where
        I: Iterator<Item = quick_xml::events::attributes::Attribute<'a>>,
    {
        let mut url = None;
        let mut length = None;
        let mut enclosure_type = None;

        for attr in attrs {
            if attr.value.len() > max_attr_length {
                continue;
            }

            match attr.key.as_ref() {
                b"url" => url = Some(bytes_to_string(&attr.value)),
                b"length" => length = Some(bytes_to_string(&attr.value)),
                b"type" => enclosure_type = Some(bytes_to_string(&attr.value)),
                _ => {}
            }
        }

        url.map(|url| Self {
            url: Url::new(url),
            length,
            enclosure_type: enclosure_type.map(MimeType::new),
        })
    }
}

impl FromAttributes for MediaThumbnail {
    fn from_attributes<'a, I>(attrs: I, max_attr_length: usize) -> Option<Self>
    where
        I: Iterator<Item = quick_xml::events::attributes::Attribute<'a>>,
    {
        let mut url = None;
        let mut width = None;
        let mut height = None;

        for attr in attrs {
            if attr.value.len() > max_attr_length {
                continue;
            }

            match attr.key.as_ref() {
                b"url" => url = Some(bytes_to_string(&attr.value)),
                b"width" => width = Some(bytes_to_string(&attr.value)),
                b"height" => height = Some(bytes_to_string(&attr.value)),
                _ => {}
            }
        }

        url.map(|url| Self {
            url: Url::new(url),
            width,
            height,
        })
    }
}

impl FromAttributes for MediaContent {
    fn from_attributes<'a, I>(attrs: I, max_attr_length: usize) -> Option<Self>
    where
        I: Iterator<Item = quick_xml::events::attributes::Attribute<'a>>,
    {
        let mut url = None;
        let mut content_type = None;
        let mut medium = None;
        let mut filesize = None;
        let mut width = None;
        let mut height = None;
        let mut duration = None;
        let mut bitrate = None;
        let mut lang = None;
        let mut channels = None;
        let mut codec = None;
        let mut expression = None;
        let mut isdefault = None;
        let mut samplingrate = None;
        let mut framerate = None;

        for attr in attrs {
            if attr.value.len() > max_attr_length {
                continue;
            }

            match attr.key.as_ref() {
                b"url" => url = Some(bytes_to_string(&attr.value)),
                b"type" => content_type = Some(bytes_to_string(&attr.value)),
                b"medium" => medium = Some(bytes_to_string(&attr.value)),
                b"fileSize" => filesize = Some(bytes_to_string(&attr.value)),
                b"width" => width = Some(bytes_to_string(&attr.value)),
                b"height" => height = Some(bytes_to_string(&attr.value)),
                b"duration" => duration = Some(bytes_to_string(&attr.value)),
                b"bitrate" => bitrate = Some(bytes_to_string(&attr.value)),
                b"lang" => lang = Some(bytes_to_string(&attr.value)),
                b"channels" => channels = Some(bytes_to_string(&attr.value)),
                b"codec" => codec = Some(bytes_to_string(&attr.value)),
                b"expression" => expression = Some(bytes_to_string(&attr.value)),
                b"isDefault" => isdefault = Some(bytes_to_string(&attr.value)),
                b"samplingrate" => samplingrate = Some(bytes_to_string(&attr.value)),
                b"framerate" => framerate = Some(bytes_to_string(&attr.value)),
                _ => {}
            }
        }

        url.map(|url| Self {
            url: Url::new(url),
            content_type: content_type.map(MimeType::new),
            medium,
            filesize,
            width,
            height,
            duration,
            bitrate,
            lang,
            channels,
            codec,
            expression,
            isdefault,
            samplingrate,
            framerate,
        })
    }
}

// ParseFrom implementations for JSON Feed parsing

impl ParseFrom<&Value> for Person {
    /// Parse Person from JSON Feed author object
    ///
    /// JSON Feed format: `{"name": "...", "url": "...", "avatar": "..."}`
    fn parse_from(json: &Value) -> Option<Self> {
        json.as_object().map(|obj| Self {
            name: obj
                .get("name")
                .and_then(Value::as_str)
                .map(std::convert::Into::into),
            email: None, // JSON Feed doesn't have email field
            uri: obj.get("url").and_then(Value::as_str).map(String::from),
        })
    }
}

impl ParseFrom<&Value> for Enclosure {
    /// Parse Enclosure from JSON Feed attachment object
    ///
    /// JSON Feed format: `{"url": "...", "mime_type": "...", "size_in_bytes": ...}`
    fn parse_from(json: &Value) -> Option<Self> {
        let obj = json.as_object()?;
        let url = obj.get("url").and_then(Value::as_str)?;
        Some(Self {
            url: Url::new(url),
            length: obj
                .get("size_in_bytes")
                .and_then(Value::as_u64)
                .map(|v| v.to_string()),
            enclosure_type: obj
                .get("mime_type")
                .and_then(Value::as_str)
                .map(MimeType::new),
        })
    }
}

/// Media RSS credit element (media:credit)
#[derive(Debug, Clone, Default)]
pub struct MediaCredit {
    /// Credit role (e.g., "author", "producer")
    pub role: Option<String>,
    /// Credit scheme URI (default: "urn:ebu")
    pub scheme: Option<String>,
    /// Credit text content (person/entity name)
    pub content: String,
}

/// Media RSS copyright element (media:copyright)
#[derive(Debug, Clone, Default)]
pub struct MediaCopyright {
    /// Copyright URL
    pub url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_link_default() {
        let link = Link::default();
        assert!(link.href.is_empty());
        assert!(link.rel.is_none());
    }

    #[test]
    fn test_link_builders() {
        let link = Link::alternate("https://example.com");
        assert_eq!(link.href, "https://example.com");
        assert_eq!(link.rel.as_deref(), Some("alternate"));

        let link = Link::self_link("https://example.com/feed", "application/feed+json");
        assert_eq!(link.rel.as_deref(), Some("self"));
        assert_eq!(link.link_type.as_deref(), Some("application/feed+json"));

        let link = Link::enclosure("https://example.com/audio.mp3", Some("audio/mpeg".into()));
        assert_eq!(link.rel.as_deref(), Some("enclosure"));
        assert_eq!(link.link_type.as_deref(), Some("audio/mpeg"));

        let link = Link::related("https://other.com");
        assert_eq!(link.rel.as_deref(), Some("related"));
    }

    #[test]
    fn test_tag_builder() {
        let tag = Tag::new("rust");
        assert_eq!(tag.term, "rust");
        assert!(tag.scheme.is_none());
    }

    #[test]
    fn test_text_construct_builders() {
        let text = TextConstruct::text("Hello");
        assert_eq!(text.value, "Hello");
        assert_eq!(text.content_type, TextType::Text);

        let html = TextConstruct::html("<p>Hello</p>");
        assert_eq!(html.content_type, TextType::Html);

        let with_lang = TextConstruct::text("Hello").with_language("en");
        assert_eq!(with_lang.language.as_deref(), Some("en"));
    }

    #[test]
    fn test_content_builders() {
        let html = Content::html("<p>Content</p>");
        assert_eq!(html.content_type.as_deref(), Some("text/html"));

        let plain = Content::plain("Content");
        assert_eq!(plain.content_type.as_deref(), Some("text/plain"));
    }

    #[test]
    fn test_person_default() {
        let person = Person::default();
        assert!(person.name.is_none());
        assert!(person.email.is_none());
        assert!(person.uri.is_none());
    }

    #[test]
    fn test_person_parse_from_json() {
        let json = json!({"name": "John Doe", "url": "https://example.com"});
        let person = Person::parse_from(&json).unwrap();
        assert_eq!(person.name.as_deref(), Some("John Doe"));
        assert_eq!(person.uri.as_deref(), Some("https://example.com"));
        assert!(person.email.is_none());
    }

    #[test]
    fn test_person_parse_from_empty_json() {
        let json = json!({});
        let person = Person::parse_from(&json).unwrap();
        assert!(person.name.is_none());
    }

    #[test]
    fn test_enclosure_parse_from_json() {
        let json = json!({
            "url": "https://example.com/file.mp3",
            "mime_type": "audio/mpeg",
            "size_in_bytes": 12345
        });
        let enclosure = Enclosure::parse_from(&json).unwrap();
        assert_eq!(enclosure.url, "https://example.com/file.mp3");
        assert_eq!(enclosure.enclosure_type.as_deref(), Some("audio/mpeg"));
        assert_eq!(enclosure.length.as_deref(), Some("12345"));
    }

    #[test]
    fn test_enclosure_parse_from_json_missing_url() {
        let json = json!({"mime_type": "audio/mpeg"});
        assert!(Enclosure::parse_from(&json).is_none());
    }

    #[test]
    fn test_text_type_equality() {
        assert_eq!(TextType::Text, TextType::Text);
        assert_ne!(TextType::Text, TextType::Html);
    }

    // Newtype tests

    #[test]
    fn test_url_new() {
        let url = Url::new("https://example.com");
        assert_eq!(url.as_str(), "https://example.com");
    }

    #[test]
    fn test_url_from_string() {
        let url: Url = String::from("https://example.com").into();
        assert_eq!(url.as_str(), "https://example.com");
    }

    #[test]
    fn test_url_from_str() {
        let url: Url = "https://example.com".into();
        assert_eq!(url.as_str(), "https://example.com");
    }

    #[test]
    fn test_url_deref() {
        let url = Url::new("https://example.com");
        // Deref allows calling str methods directly
        assert_eq!(url.len(), 19);
        assert!(url.starts_with("https://"));
    }

    #[test]
    fn test_url_into_inner() {
        let url = Url::new("https://example.com");
        let inner = url.into_inner();
        assert_eq!(inner, "https://example.com");
    }

    #[test]
    fn test_url_default() {
        let url = Url::default();
        assert_eq!(url.as_str(), "");
    }

    #[test]
    fn test_url_clone() {
        let url1 = Url::new("https://example.com");
        let url2 = url1.clone();
        assert_eq!(url1, url2);
    }

    #[test]
    fn test_mime_type_new() {
        let mime = MimeType::new("text/html");
        assert_eq!(mime.as_str(), "text/html");
    }

    #[test]
    fn test_mime_type_from_string() {
        let mime: MimeType = String::from("application/json").into();
        assert_eq!(mime.as_str(), "application/json");
    }

    #[test]
    fn test_mime_type_from_str() {
        let mime: MimeType = "text/plain".into();
        assert_eq!(mime.as_str(), "text/plain");
    }

    #[test]
    fn test_mime_type_deref() {
        let mime = MimeType::new("text/html");
        assert_eq!(mime.len(), 9);
        assert!(mime.starts_with("text/"));
    }

    #[test]
    fn test_mime_type_default() {
        let mime = MimeType::default();
        assert_eq!(mime.as_str(), "");
    }

    #[test]
    fn test_mime_type_clone() {
        let mime1 = MimeType::new("application/xml");
        let mime2 = mime1.clone();
        assert_eq!(mime1, mime2);
        // Arc cloning is cheap - just increments refcount
    }

    #[test]
    fn test_mime_type_constants() {
        assert_eq!(MimeType::TEXT_HTML, "text/html");
        assert_eq!(MimeType::TEXT_PLAIN, "text/plain");
        assert_eq!(MimeType::APPLICATION_XML, "application/xml");
        assert_eq!(MimeType::APPLICATION_JSON, "application/json");
    }

    #[test]
    fn test_email_new() {
        let email = Email::new("user@example.com");
        assert_eq!(email.as_str(), "user@example.com");
    }

    #[test]
    fn test_email_from_string() {
        let email: Email = String::from("user@example.com").into();
        assert_eq!(email.as_str(), "user@example.com");
    }

    #[test]
    fn test_email_from_str() {
        let email: Email = "user@example.com".into();
        assert_eq!(email.as_str(), "user@example.com");
    }

    #[test]
    fn test_email_deref() {
        let email = Email::new("user@example.com");
        assert_eq!(email.len(), 16);
        assert!(email.contains('@'));
    }

    #[test]
    fn test_email_into_inner() {
        let email = Email::new("user@example.com");
        let inner = email.into_inner();
        assert_eq!(inner, "user@example.com");
    }

    #[test]
    fn test_email_default() {
        let email = Email::default();
        assert_eq!(email.as_str(), "");
    }

    #[test]
    fn test_email_clone() {
        let email1 = Email::new("user@example.com");
        let email2 = email1.clone();
        assert_eq!(email1, email2);
    }
}
