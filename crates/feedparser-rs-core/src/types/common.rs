use super::generics::{FromAttributes, ParseFrom};
use crate::util::text::bytes_to_string;
use serde_json::Value;

/// Link in feed or entry
#[derive(Debug, Clone, Default)]
pub struct Link {
    /// Link URL
    pub href: String,
    /// Link relationship type (e.g., "alternate", "enclosure", "self")
    pub rel: Option<String>,
    /// MIME type of the linked resource
    pub link_type: Option<String>,
    /// Human-readable link title
    pub title: Option<String>,
    /// Length of the linked resource in bytes
    pub length: Option<u64>,
    /// Language of the linked resource
    pub hreflang: Option<String>,
}

impl Link {
    /// Create a new link with just URL and relation type
    #[inline]
    pub fn new(href: impl Into<String>, rel: impl Into<String>) -> Self {
        Self {
            href: href.into(),
            rel: Some(rel.into()),
            link_type: None,
            title: None,
            length: None,
            hreflang: None,
        }
    }

    /// Create an alternate link (common for entry URLs)
    #[inline]
    pub fn alternate(href: impl Into<String>) -> Self {
        Self::new(href, "alternate")
    }

    /// Create a self link (for feed URLs)
    #[inline]
    pub fn self_link(href: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            href: href.into(),
            rel: Some("self".to_string()),
            link_type: Some(mime_type.into()),
            title: None,
            length: None,
            hreflang: None,
        }
    }

    /// Create an enclosure link (for media)
    #[inline]
    pub fn enclosure(href: impl Into<String>, mime_type: Option<String>) -> Self {
        Self {
            href: href.into(),
            rel: Some("enclosure".to_string()),
            link_type: mime_type,
            title: None,
            length: None,
            hreflang: None,
        }
    }

    /// Create a related link
    #[inline]
    pub fn related(href: impl Into<String>) -> Self {
        Self::new(href, "related")
    }

    /// Set MIME type (builder pattern)
    #[inline]
    #[must_use]
    pub fn with_type(mut self, mime_type: impl Into<String>) -> Self {
        self.link_type = Some(mime_type.into());
        self
    }
}

/// Person (author, contributor, etc.)
#[derive(Debug, Clone, Default)]
pub struct Person {
    /// Person's name
    pub name: Option<String>,
    /// Person's email address
    pub email: Option<String>,
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
    pub fn from_name(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            email: None,
            uri: None,
        }
    }
}

/// Tag/category
#[derive(Debug, Clone)]
pub struct Tag {
    /// Tag term/label
    pub term: String,
    /// Tag scheme/domain
    pub scheme: Option<String>,
    /// Human-readable tag label
    pub label: Option<String>,
}

impl Tag {
    /// Create a simple tag with just term
    #[inline]
    pub fn new(term: impl Into<String>) -> Self {
        Self {
            term: term.into(),
            scheme: None,
            label: None,
        }
    }
}

/// Image metadata
#[derive(Debug, Clone)]
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

/// Enclosure (attached media file)
#[derive(Debug, Clone)]
pub struct Enclosure {
    /// Enclosure URL
    pub url: String,
    /// File size in bytes
    pub length: Option<u64>,
    /// MIME type
    pub enclosure_type: Option<String>,
}

/// Content block
#[derive(Debug, Clone)]
pub struct Content {
    /// Content body
    pub value: String,
    /// Content MIME type
    pub content_type: Option<String>,
    /// Content language
    pub language: Option<String>,
    /// Base URL for relative links
    pub base: Option<String>,
}

impl Content {
    /// Create HTML content
    #[inline]
    pub fn html(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            content_type: Some("text/html".to_string()),
            language: None,
            base: None,
        }
    }

    /// Create plain text content
    #[inline]
    pub fn plain(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            content_type: Some("text/plain".to_string()),
            language: None,
            base: None,
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
    /// Content language
    pub language: Option<String>,
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
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }
}

/// Generator metadata
#[derive(Debug, Clone)]
pub struct Generator {
    /// Generator name
    pub value: String,
    /// Generator URI
    pub uri: Option<String>,
    /// Generator version
    pub version: Option<String>,
}

/// Source reference (for entries)
#[derive(Debug, Clone)]
pub struct Source {
    /// Source title
    pub title: Option<String>,
    /// Source link
    pub link: Option<String>,
    /// Source ID
    pub id: Option<String>,
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
    pub url: String,
    /// Thumbnail width in pixels
    pub width: Option<u32>,
    /// Thumbnail height in pixels
    pub height: Option<u32>,
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
    pub url: String,
    /// MIME type
    pub content_type: Option<String>,
    /// File size in bytes
    pub filesize: Option<u64>,
    /// Media width in pixels
    pub width: Option<u32>,
    /// Media height in pixels
    pub height: Option<u32>,
    /// Duration in seconds (for audio/video)
    pub duration: Option<u64>,
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
                b"length" => length = bytes_to_string(&attr.value).parse().ok(),
                _ => {}
            }
        }

        href.map(|href| Self {
            href,
            rel: rel.or_else(|| Some("alternate".to_string())),
            link_type,
            title,
            length,
            hreflang,
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
            term,
            scheme,
            label,
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
                b"length" => length = bytes_to_string(&attr.value).parse().ok(),
                b"type" => enclosure_type = Some(bytes_to_string(&attr.value)),
                _ => {}
            }
        }

        url.map(|url| Self {
            url,
            length,
            enclosure_type,
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
                b"width" => width = bytes_to_string(&attr.value).parse().ok(),
                b"height" => height = bytes_to_string(&attr.value).parse().ok(),
                _ => {}
            }
        }

        url.map(|url| Self { url, width, height })
    }
}

impl FromAttributes for MediaContent {
    fn from_attributes<'a, I>(attrs: I, max_attr_length: usize) -> Option<Self>
    where
        I: Iterator<Item = quick_xml::events::attributes::Attribute<'a>>,
    {
        let mut url = None;
        let mut content_type = None;
        let mut filesize = None;
        let mut width = None;
        let mut height = None;
        let mut duration = None;

        for attr in attrs {
            if attr.value.len() > max_attr_length {
                continue;
            }

            match attr.key.as_ref() {
                b"url" => url = Some(bytes_to_string(&attr.value)),
                b"type" => content_type = Some(bytes_to_string(&attr.value)),
                b"fileSize" => filesize = bytes_to_string(&attr.value).parse().ok(),
                b"width" => width = bytes_to_string(&attr.value).parse().ok(),
                b"height" => height = bytes_to_string(&attr.value).parse().ok(),
                b"duration" => duration = bytes_to_string(&attr.value).parse().ok(),
                _ => {}
            }
        }

        url.map(|url| Self {
            url,
            content_type,
            filesize,
            width,
            height,
            duration,
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
            name: obj.get("name").and_then(Value::as_str).map(String::from),
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
            url: url.to_string(),
            length: obj.get("size_in_bytes").and_then(Value::as_u64),
            enclosure_type: obj
                .get("mime_type")
                .and_then(Value::as_str)
                .map(String::from),
        })
    }
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

        let link = Link::enclosure(
            "https://example.com/audio.mp3",
            Some("audio/mpeg".to_string()),
        );
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
        assert_eq!(enclosure.length, Some(12345));
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
}
