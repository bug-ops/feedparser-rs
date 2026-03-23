/// Atom Threading Extensions (RFC 4685) namespace handler
///
/// Namespace URI: `http://purl.org/syndication/thread/1.0`
/// Prefix: `thr`
///
/// Handles two elements:
/// - `thr:in-reply-to` — attribute-only element referencing the parent entry
/// - `thr:total` — text element with total response count
use crate::types::{Entry, InReplyTo, MimeType, SmallString, Url};

/// Atom Threading Extensions namespace URI
pub const THREADING_NAMESPACE: &str = "http://purl.org/syndication/thread/1.0";

/// Normalize attribute value: empty string after trim becomes None
#[inline]
fn non_empty(s: &str) -> Option<&str> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Build an `InReplyTo` from its four optional fields.
///
/// Returns `None` only if ALL fields are `None` (fully empty element).
#[inline]
fn build_in_reply_to(
    ref_: Option<SmallString>,
    href: Option<Url>,
    type_: Option<MimeType>,
    source: Option<Url>,
) -> Option<InReplyTo> {
    if ref_.is_none() && href.is_none() && type_.is_none() && source.is_none() {
        return None;
    }
    Some(InReplyTo {
        ref_,
        href,
        type_,
        source,
    })
}

/// Parse `thr:in-reply-to` from a quick-xml attribute iterator (Atom and RSS 1.0 parser paths)
///
/// Returns `None` only if ALL fields are `None` after normalization (fully empty element).
/// Returns `Some(InReplyTo)` even if `ref` is missing, to tolerate malformed feeds.
///
/// # Arguments
///
/// * `attrs` - Iterator over quick-xml attributes (from `element.attributes().flatten()`)
/// * `max_attr_len` - Maximum attribute value length for `DoS` protection
pub fn parse_in_reply_to_from_attrs<'a>(
    attrs: impl Iterator<Item = quick_xml::events::attributes::Attribute<'a>>,
    max_attr_len: usize,
) -> Option<InReplyTo> {
    let mut ref_ = None;
    let mut href = None;
    let mut type_ = None;
    let mut source = None;

    for attr in attrs {
        if attr.value.len() > max_attr_len {
            continue;
        }
        let Ok(value) = attr.unescape_value() else {
            continue;
        };
        match attr.key.as_ref() {
            b"ref" => ref_ = non_empty(&value).map(|s| s.to_string().into()),
            b"href" => href = non_empty(&value).map(|s| s.to_string().into()),
            b"type" => type_ = non_empty(&value).map(|s| s.to_string().into()),
            b"source" => source = non_empty(&value).map(|s| s.to_string().into()),
            _ => {}
        }
    }

    build_in_reply_to(ref_, href, type_, source)
}

/// Parse `thr:in-reply-to` from collected attributes (RSS 2.0 parser path)
///
/// Returns `None` only if ALL fields are `None` after normalization.
///
/// # Arguments
///
/// * `attrs` - Slice of collected `(key_bytes, value_string)` attribute pairs
/// * `max_attr_len` - Maximum attribute value length for `DoS` protection
pub fn parse_in_reply_to_from_collected(
    attrs: &[(Vec<u8>, String)],
    max_attr_len: usize,
) -> Option<InReplyTo> {
    let mut ref_ = None;
    let mut href = None;
    let mut type_ = None;
    let mut source = None;

    for (key, value) in attrs {
        if value.len() > max_attr_len {
            continue;
        }
        match key.as_slice() {
            b"ref" => ref_ = non_empty(value).map(|s| s.to_string().into()),
            b"href" => href = non_empty(value).map(|s| s.to_string().into()),
            b"type" => type_ = non_empty(value).map(|s| s.to_string().into()),
            b"source" => source = non_empty(value).map(|s| s.to_string().into()),
            _ => {}
        }
    }

    build_in_reply_to(ref_, href, type_, source)
}

/// Handle `thr:total` text content
///
/// Parses non-negative integer from text. Silently ignores non-numeric, negative,
/// overflow, empty, and whitespace-only values (consistent with how `parse_duration`
/// handles invalid input throughout the codebase).
///
/// # Arguments
///
/// * `text` - Text content of the `thr:total` element
/// * `entry` - Entry to update with the parsed count
pub fn handle_total(text: &str, entry: &mut Entry) {
    entry.thr_total = text.trim().parse::<u32>().ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_total_valid() {
        let mut entry = Entry::default();
        handle_total("15", &mut entry);
        assert_eq!(entry.thr_total, Some(15));
    }

    #[test]
    fn test_handle_total_with_whitespace() {
        let mut entry = Entry::default();
        handle_total("  42  ", &mut entry);
        assert_eq!(entry.thr_total, Some(42));
    }

    #[test]
    fn test_handle_total_zero() {
        let mut entry = Entry::default();
        handle_total("0", &mut entry);
        assert_eq!(entry.thr_total, Some(0));
    }

    #[test]
    fn test_handle_total_non_numeric() {
        let mut entry = Entry::default();
        handle_total("abc", &mut entry);
        assert_eq!(entry.thr_total, None);
    }

    #[test]
    fn test_handle_total_negative() {
        let mut entry = Entry::default();
        handle_total("-5", &mut entry);
        assert_eq!(entry.thr_total, None);
    }

    #[test]
    fn test_handle_total_overflow() {
        let mut entry = Entry::default();
        handle_total("99999999999999", &mut entry);
        assert_eq!(entry.thr_total, None);
    }

    #[test]
    fn test_handle_total_empty() {
        let mut entry = Entry::default();
        handle_total("", &mut entry);
        assert_eq!(entry.thr_total, None);
    }

    #[test]
    fn test_handle_total_whitespace_only() {
        let mut entry = Entry::default();
        handle_total("   ", &mut entry);
        assert_eq!(entry.thr_total, None);
    }

    #[test]
    fn test_parse_in_reply_to_from_collected_full() {
        let attrs = vec![
            (b"ref".to_vec(), "tag:example.com,2024:post/1".to_string()),
            (b"href".to_vec(), "https://example.com/post/1".to_string()),
            (b"type".to_vec(), "text/html".to_string()),
            (
                b"source".to_vec(),
                "https://example.com/feed.xml".to_string(),
            ),
        ];
        let result = parse_in_reply_to_from_collected(&attrs, 1024);
        assert!(result.is_some());
        let irt = result.unwrap();
        assert_eq!(irt.ref_.as_deref(), Some("tag:example.com,2024:post/1"));
        assert_eq!(irt.href.as_deref(), Some("https://example.com/post/1"));
        assert_eq!(irt.type_.as_deref(), Some("text/html"));
        assert_eq!(irt.source.as_deref(), Some("https://example.com/feed.xml"));
    }

    #[test]
    fn test_parse_in_reply_to_from_collected_empty_ref() {
        let attrs = vec![
            (b"ref".to_vec(), String::new()),
            (b"href".to_vec(), "https://example.com/post/1".to_string()),
        ];
        let result = parse_in_reply_to_from_collected(&attrs, 1024);
        assert!(result.is_some());
        let irt = result.unwrap();
        // Empty ref should be normalized to None
        assert!(irt.ref_.is_none());
        assert_eq!(irt.href.as_deref(), Some("https://example.com/post/1"));
    }

    #[test]
    fn test_parse_in_reply_to_from_collected_all_empty() {
        let attrs = vec![
            (b"ref".to_vec(), String::new()),
            (b"href".to_vec(), String::new()),
            (b"type".to_vec(), "  ".to_string()),
            (b"source".to_vec(), String::new()),
        ];
        let result = parse_in_reply_to_from_collected(&attrs, 1024);
        // All fields are None -> return None
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_in_reply_to_from_collected_truncated_by_limit() {
        let attrs = vec![(b"ref".to_vec(), "tag:example.com,2024:post/1".to_string())];
        let result = parse_in_reply_to_from_collected(&attrs, 5);
        // Value exceeds max_attr_len, should be skipped -> all None -> None
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_in_reply_to_from_collected_only_ref() {
        let attrs = vec![(b"ref".to_vec(), "tag:example.com,2024:post/1".to_string())];
        let result = parse_in_reply_to_from_collected(&attrs, 1024);
        assert!(result.is_some());
        let irt = result.unwrap();
        assert_eq!(irt.ref_.as_deref(), Some("tag:example.com,2024:post/1"));
        assert!(irt.href.is_none());
        assert!(irt.type_.is_none());
        assert!(irt.source.is_none());
    }

    #[test]
    fn test_non_empty_normalization() {
        assert_eq!(non_empty(""), None);
        assert_eq!(non_empty("  "), None);
        assert_eq!(non_empty("hello"), Some("hello"));
        assert_eq!(non_empty("  hello  "), Some("hello"));
    }
}
