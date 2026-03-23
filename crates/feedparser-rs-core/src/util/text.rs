//! Text processing utilities
//!
//! This module provides functions for text manipulation,
//! such as trimming, normalizing whitespace, and encoding conversion.

use crate::types::{Email, Person};

/// Efficient bytes to string conversion - zero-copy for valid UTF-8
///
/// Uses `std::str::from_utf8()` for zero-copy conversion when the input
/// is valid UTF-8, falling back to lossy conversion otherwise.
///
/// # Examples
///
/// ```
/// use feedparser_rs::util::text::bytes_to_string;
///
/// let valid_utf8 = b"Hello, world!";
/// assert_eq!(bytes_to_string(valid_utf8), "Hello, world!");
///
/// let invalid_utf8 = &[0xFF, 0xFE, 0xFD];
/// let result = bytes_to_string(invalid_utf8);
/// assert!(!result.is_empty()); // Lossy conversion succeeded
/// ```
#[inline]
pub fn bytes_to_string(value: &[u8]) -> String {
    std::str::from_utf8(value).map_or_else(
        |_| String::from_utf8_lossy(value).into_owned(),
        std::string::ToString::to_string,
    )
}

/// Truncates string to maximum length by character count
///
/// Uses efficient byte-length check before expensive char iteration.
/// Prevents oversized attribute/text values that could cause memory issues.
///
/// # Examples
///
/// ```
/// use feedparser_rs::util::text::truncate_to_length;
///
/// assert_eq!(truncate_to_length("hello world", 5), "hello");
/// assert_eq!(truncate_to_length("hi", 100), "hi");
/// assert_eq!(truncate_to_length("", 10), "");
/// ```
#[inline]
#[must_use]
pub fn truncate_to_length(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        s.chars().take(max_len).collect()
    }
}

/// Parses an RSS person string into a structured `Person`.
///
/// Recognizes common RSS author formats:
/// - `email (Name)` → `Person { name: Some("Name"), email: Some("email") }`
/// - `Name <email>` → `Person { name: Some("Name"), email: Some("email") }`
/// - bare email (contains `@`) → `Person { email: Some(text) }`
/// - plain string → `Person { name: Some(text) }`
///
/// # Examples
///
/// ```
/// use feedparser_rs::util::text::parse_rss_person;
///
/// let p = parse_rss_person("editor@example.com (John Editor)");
/// assert_eq!(p.name.as_deref(), Some("John Editor"));
/// assert_eq!(p.email.as_deref(), Some("editor@example.com"));
///
/// let p = parse_rss_person("John Editor <editor@example.com>");
/// assert_eq!(p.name.as_deref(), Some("John Editor"));
/// assert_eq!(p.email.as_deref(), Some("editor@example.com"));
///
/// let p = parse_rss_person("just-a-name");
/// assert_eq!(p.name.as_deref(), Some("just-a-name"));
/// assert!(p.email.is_none());
/// ```
#[must_use]
pub fn parse_rss_person(text: &str) -> Person {
    let text = text.trim();

    // Pattern: "email (Name)"
    if let Some(paren_start) = text.find('(')
        && text.ends_with(')')
    {
        let email_part = text[..paren_start].trim();
        let name_part = text[paren_start + 1..text.len() - 1].trim();
        if !email_part.is_empty() {
            return Person {
                name: if name_part.is_empty() {
                    None
                } else {
                    Some(name_part.into())
                },
                email: Some(Email::new(email_part.to_string())),
                uri: None,
            };
        }
    }

    // Pattern: "Name <email>"
    if let Some(angle_start) = text.rfind('<')
        && text.ends_with('>')
    {
        let name_part = text[..angle_start].trim();
        let email_part = text[angle_start + 1..text.len() - 1].trim();
        if !email_part.is_empty() {
            return Person {
                name: if name_part.is_empty() {
                    None
                } else {
                    Some(name_part.into())
                },
                email: Some(Email::new(email_part.to_string())),
                uri: None,
            };
        }
    }

    // Bare email or plain name
    if text.contains('@') {
        Person {
            name: None,
            email: Some(Email::new(text.to_string())),
            uri: None,
        }
    } else {
        Person::from_name(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rss_person_email_paren_name() {
        let p = parse_rss_person("editor@example.com (John Editor)");
        assert_eq!(p.name.as_deref(), Some("John Editor"));
        assert_eq!(p.email.as_deref(), Some("editor@example.com"));
    }

    #[test]
    fn test_parse_rss_person_name_angle_email() {
        let p = parse_rss_person("John Editor <editor@example.com>");
        assert_eq!(p.name.as_deref(), Some("John Editor"));
        assert_eq!(p.email.as_deref(), Some("editor@example.com"));
    }

    #[test]
    fn test_parse_rss_person_bare_email() {
        let p = parse_rss_person("editor@example.com");
        assert!(p.name.is_none());
        assert_eq!(p.email.as_deref(), Some("editor@example.com"));
    }

    #[test]
    fn test_parse_rss_person_plain_name() {
        let p = parse_rss_person("John Editor");
        assert_eq!(p.name.as_deref(), Some("John Editor"));
        assert!(p.email.is_none());
    }

    #[test]
    fn test_parse_rss_person_empty_name_in_parens() {
        let p = parse_rss_person("editor@example.com ()");
        assert!(p.name.is_none());
        assert_eq!(p.email.as_deref(), Some("editor@example.com"));
    }
}
