/// Slash namespace — `http://purl.org/rss/1.0/modules/slash/`
/// WFW (Well-Formed Web) namespace — `http://wellformedweb.org/CommentAPI/`
///
/// These namespaces are commonly used together in `WordPress`, `Blogger`, and
/// `Movable Type` feeds to expose comment metadata.
///
/// - `slash:comments` — integer count of comments on an entry
/// - `wfw:commentRss` — URL of the entry's comment RSS feed
use crate::types::Entry;

/// Slash namespace URI
pub const SLASH_NAMESPACE: &str = "http://purl.org/rss/1.0/modules/slash/";

/// WFW namespace URI
pub const WFW_NAMESPACE: &str = "http://wellformedweb.org/CommentAPI/";

/// Handle a Slash namespace element at entry level
///
/// # Arguments
///
/// * `element` - Local name of the element (without namespace prefix)
/// * `text` - Text content of the element
/// * `entry` - Entry to update
pub fn handle_slash_entry_element(element: &str, text: &str, entry: &mut Entry) {
    if element == "comments" {
        // Ignore non-integer values silently (bozo pattern: keep parsing)
        entry.slash_comments = text.trim().parse::<u32>().ok();
    }
}

/// Handle a WFW namespace element at entry level
///
/// # Arguments
///
/// * `element` - Local name of the element (without namespace prefix)
/// * `text` - Text content of the element
/// * `entry` - Entry to update
pub fn handle_wfw_entry_element(element: &str, text: &str, entry: &mut Entry) {
    if element == "commentRss" || element == "commentrss" {
        entry.wfw_comment_rss = Some(text.trim().to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slash_comments_integer() {
        let mut entry = Entry::default();
        handle_slash_entry_element("comments", "42", &mut entry);
        assert_eq!(entry.slash_comments, Some(42));
    }

    #[test]
    fn test_slash_comments_with_whitespace() {
        let mut entry = Entry::default();
        handle_slash_entry_element("comments", "  7  ", &mut entry);
        assert_eq!(entry.slash_comments, Some(7));
    }

    #[test]
    fn test_slash_comments_zero() {
        let mut entry = Entry::default();
        handle_slash_entry_element("comments", "0", &mut entry);
        assert_eq!(entry.slash_comments, Some(0));
    }

    #[test]
    fn test_slash_comments_invalid_is_ignored() {
        let mut entry = Entry::default();
        handle_slash_entry_element("comments", "not-a-number", &mut entry);
        assert_eq!(entry.slash_comments, None);
    }

    #[test]
    fn test_slash_unknown_element_ignored() {
        let mut entry = Entry::default();
        handle_slash_entry_element("section", "tech", &mut entry);
        assert_eq!(entry.slash_comments, None);
    }

    #[test]
    fn test_wfw_comment_rss() {
        let mut entry = Entry::default();
        handle_wfw_entry_element(
            "commentRss",
            "https://example.com/post/comments/feed/",
            &mut entry,
        );
        assert_eq!(
            entry.wfw_comment_rss.as_deref(),
            Some("https://example.com/post/comments/feed/")
        );
    }

    #[test]
    fn test_wfw_comment_rss_lowercase_variant() {
        let mut entry = Entry::default();
        handle_wfw_entry_element(
            "commentrss",
            "https://example.com/post/comments/feed/",
            &mut entry,
        );
        assert_eq!(
            entry.wfw_comment_rss.as_deref(),
            Some("https://example.com/post/comments/feed/")
        );
    }

    #[test]
    fn test_wfw_unknown_element_ignored() {
        let mut entry = Entry::default();
        handle_wfw_entry_element("other", "value", &mut entry);
        assert_eq!(entry.wfw_comment_rss, None);
    }
}
