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
pub fn handle_entry_element(element: &str, text: &str, entry: &mut Entry) {
    if element == "encoded" {
        // content:encoded → add to entry.content as HTML
        entry.content.push(Content {
            value: text.to_string(),
            content_type: Some("text/html".into()),
            language: None,
            base: None,
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

        handle_entry_element("encoded", html, &mut entry);

        assert_eq!(entry.content.len(), 1);
        assert_eq!(entry.content[0].value, html);
        assert_eq!(entry.content[0].content_type.as_deref(), Some("text/html"));
    }

    #[test]
    fn test_multiple_content_encoded() {
        let mut entry = Entry::default();

        handle_entry_element("encoded", "<p>First content</p>", &mut entry);
        handle_entry_element("encoded", "<p>Second content</p>", &mut entry);

        assert_eq!(entry.content.len(), 2);
    }

    #[test]
    fn test_content_with_cdata() {
        let mut entry = Entry::default();
        // CDATA markers are typically stripped by XML parser before we see it
        let html = r"<p>Content from <![CDATA[...]]></p>";

        handle_entry_element("encoded", html, &mut entry);

        assert!(!entry.content.is_empty());
    }

    #[test]
    fn test_ignore_unknown_elements() {
        let mut entry = Entry::default();

        handle_entry_element("unknown", "test", &mut entry);

        assert!(entry.content.is_empty());
    }
}
