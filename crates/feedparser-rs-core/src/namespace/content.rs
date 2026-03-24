/// Content Module for RSS 1.0
///
/// Namespace: <http://purl.org/rss/1.0/modules/content/>
/// Prefix: content
///
/// This module provides parsing support for the Content namespace,
/// commonly used in `WordPress` and other RSS feeds to provide full
/// HTML content separate from the summary.
///
/// Elements:
/// - `content:encoded` → adds to entry.content with type "text/html"
use crate::types::{Content, Entry};

/// Content namespace URI
pub const CONTENT_NAMESPACE: &str = "http://purl.org/rss/1.0/modules/content/";

/// Handle Content namespace element at entry level
///
/// # Arguments
///
/// * `element` - Local name of the element (without namespace prefix)
/// * `text` - Text content of the element
/// * `entry` - Entry to update
pub fn handle_entry_element(
    element: &str,
    text: &str,
    entry: &mut Entry,
    lang: Option<&str>,
    base: Option<&str>,
) {
    if element == "encoded" {
        // content:encoded → add to entry.content as HTML
        entry.content.push(Content {
            value: text.to_string(),
            content_type: Some("text/html".into()),
            language: lang.filter(|s| !s.is_empty()).map(Into::into),
            base: base.filter(|s| !s.is_empty()).map(ToString::to_string),
            src: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_encoded() {
        let mut entry = Entry::default();
        let html = r"<p>Full HTML content with <strong>formatting</strong>.</p>";

        handle_entry_element("encoded", html, &mut entry, None, None);

        assert_eq!(entry.content.len(), 1);
        assert_eq!(entry.content[0].value, html);
        assert_eq!(entry.content[0].content_type.as_deref(), Some("text/html"));
    }

    #[test]
    fn test_multiple_content_encoded() {
        let mut entry = Entry::default();

        handle_entry_element("encoded", "<p>First content</p>", &mut entry, None, None);
        handle_entry_element("encoded", "<p>Second content</p>", &mut entry, None, None);

        assert_eq!(entry.content.len(), 2);
    }

    #[test]
    fn test_content_with_cdata() {
        let mut entry = Entry::default();
        // CDATA markers are typically stripped by XML parser before we see it
        let html = r"<p>Content from <![CDATA[...]]></p>";

        handle_entry_element("encoded", html, &mut entry, None, None);

        assert!(!entry.content.is_empty());
    }

    #[test]
    fn test_ignore_unknown_elements() {
        let mut entry = Entry::default();

        handle_entry_element("unknown", "test", &mut entry, None, None);

        assert!(entry.content.is_empty());
    }

    #[test]
    fn test_content_encoded_with_lang_and_base() {
        let mut entry = Entry::default();
        let html = "<p>Content</p>";

        handle_entry_element(
            "encoded",
            html,
            &mut entry,
            Some("en"),
            Some("http://example.com/"),
        );

        assert_eq!(entry.content.len(), 1);
        assert_eq!(entry.content[0].language.as_deref(), Some("en"));
        assert_eq!(
            entry.content[0].base.as_deref(),
            Some("http://example.com/")
        );
    }

    #[test]
    fn test_content_encoded_empty_lang_treated_as_none() {
        let mut entry = Entry::default();

        handle_entry_element("encoded", "<p>Test</p>", &mut entry, Some(""), None);

        assert_eq!(entry.content.len(), 1);
        assert!(entry.content[0].language.is_none());
    }
}
