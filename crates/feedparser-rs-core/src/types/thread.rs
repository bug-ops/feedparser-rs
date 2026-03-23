use super::common::{MimeType, SmallString, Url};

/// Atom Threading Extensions (RFC 4685) in-reply-to element
///
/// Represents a reference to another entry that this entry is a reply to.
/// Namespace URI: `http://purl.org/syndication/thread/1.0`
///
/// # Examples
///
/// ```
/// use feedparser_rs::InReplyTo;
///
/// let reply = InReplyTo {
///     ref_: Some("tag:example.com,2024:post/1".into()),
///     href: Some("https://example.com/post/1".into()),
///     type_: Some("text/html".into()),
///     source: Some("https://example.com/feed.xml".into()),
/// };
///
/// assert_eq!(reply.ref_.as_deref(), Some("tag:example.com,2024:post/1"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InReplyTo {
    /// IRI of the entry being replied to (ref attribute)
    ///
    /// Required by RFC 4685, but we tolerate missing values (bozo pattern).
    /// Empty-string values from malformed feeds are normalized to None.
    /// Field named `ref_` to avoid Rust keyword clash.
    pub ref_: Option<SmallString>,

    /// URL where the referenced entry can be found (href attribute)
    ///
    /// # Security
    ///
    /// This URL comes from untrusted feed input and has NOT been validated.
    /// Applications MUST validate URLs before fetching to prevent SSRF.
    pub href: Option<Url>,

    /// MIME type of the linked resource (type attribute)
    ///
    /// Field named `type_` to avoid Rust keyword clash.
    pub type_: Option<MimeType>,

    /// IRI of the feed containing the referenced entry (source attribute)
    ///
    /// Not to be confused with [`Entry::source`](super::entry::Entry::source)
    /// which is the RSS/Atom source feed reference. This field is the
    /// RFC 4685 `source` attribute: an IRI identifying the feed that
    /// contains the entry being replied to.
    ///
    /// # Security
    ///
    /// This URL comes from untrusted feed input and has NOT been validated.
    /// Applications MUST validate URLs before fetching to prevent SSRF.
    pub source: Option<Url>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_reply_to_default() {
        let reply = InReplyTo::default();
        assert!(reply.ref_.is_none());
        assert!(reply.href.is_none());
        assert!(reply.type_.is_none());
        assert!(reply.source.is_none());
    }

    #[test]
    fn test_in_reply_to_partial_construction() {
        let reply = InReplyTo {
            ref_: Some("tag:example.com,2024:post/1".into()),
            ..Default::default()
        };
        assert_eq!(reply.ref_.as_deref(), Some("tag:example.com,2024:post/1"));
        assert!(reply.href.is_none());
        assert!(reply.type_.is_none());
        assert!(reply.source.is_none());
    }

    #[test]
    fn test_in_reply_to_equality() {
        let a = InReplyTo {
            ref_: Some("tag:example.com,2024:1".into()),
            href: Some("https://example.com/1".into()),
            type_: Some("text/html".into()),
            source: Some("https://example.com/feed.xml".into()),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    #[allow(clippy::redundant_clone)]
    fn test_in_reply_to_clone() {
        let original = InReplyTo {
            ref_: Some("tag:example.com,2024:post/1".into()),
            href: Some("https://example.com/post/1".into()),
            type_: Some("text/html".into()),
            source: Some("https://example.com/feed.xml".into()),
        };
        let cloned = original.clone();
        assert_eq!(cloned.ref_.as_deref(), Some("tag:example.com,2024:post/1"));
        assert_eq!(cloned.href.as_deref(), Some("https://example.com/post/1"));
        assert_eq!(cloned.type_.as_deref(), Some("text/html"));
        assert_eq!(
            cloned.source.as_deref(),
            Some("https://example.com/feed.xml")
        );
    }
}
