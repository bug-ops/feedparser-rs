//! JSON Feed parser (versions 1.0 and 1.1)
//!
//! Specification: <https://www.jsonfeed.org/version/1.1/>

use crate::{
    ParserLimits,
    error::{FeedError, Result},
    types::{
        Content, Enclosure, Entry, FeedMeta, FeedVersion, LimitedCollectionExt, Link, ParseFrom,
        ParsedFeed, Person, Tag, TextConstruct,
    },
    util::{date::parse_date, text::truncate_to_length},
};
use serde_json::Value;

/// Parse JSON Feed with default limits
#[allow(dead_code)]
pub fn parse_json_feed(data: &[u8]) -> Result<ParsedFeed> {
    parse_json_feed_with_limits(data, ParserLimits::default())
}

/// Parse JSON Feed with custom limits
pub fn parse_json_feed_with_limits(data: &[u8], limits: ParserLimits) -> Result<ParsedFeed> {
    if data.len() > limits.max_feed_size_bytes {
        return Err(FeedError::InvalidFormat(format!(
            "Feed size {} exceeds limit {}",
            data.len(),
            limits.max_feed_size_bytes
        )));
    }

    let mut feed = ParsedFeed::with_capacity(limits.max_entries);

    let json: Value = match serde_json::from_slice(data) {
        Ok(v) => v,
        Err(e) => {
            feed.bozo = true;
            feed.bozo_exception = Some(format!("JSON parse error: {e}"));
            return Ok(feed);
        }
    };

    let version = json
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| FeedError::InvalidFormat("Missing version field".to_string()))?;

    feed.version = match version {
        "https://jsonfeed.org/version/1" => FeedVersion::JsonFeed10,
        "https://jsonfeed.org/version/1.1" => FeedVersion::JsonFeed11,
        _ => {
            feed.bozo = true;
            feed.bozo_exception = Some(format!("Unknown JSON Feed version: {version}"));
            FeedVersion::Unknown
        }
    };

    parse_feed_metadata(&json, &mut feed.feed, &limits);

    // Feed-level language for fallback in items
    let feed_lang = json
        .get("language")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty() && s.len() <= limits.max_text_length);

    if let Some(items) = json.get("items").and_then(|v| v.as_array()) {
        for (idx, item) in items.iter().enumerate() {
            if idx >= limits.max_entries {
                feed.bozo = true;
                feed.bozo_exception = Some(format!(
                    "Entry count exceeds limit of {}",
                    limits.max_entries
                ));
                break;
            }
            feed.entries
                .push(parse_item(item, &limits, feed_lang, &feed.feed.authors));
        }
    }

    Ok(feed)
}

fn parse_feed_metadata(json: &Value, feed: &mut FeedMeta, limits: &ParserLimits) {
    if let Some(title) = json.get("title").and_then(|v| v.as_str()) {
        let truncated = truncate_to_length(title, limits.max_text_length);
        feed.set_title(TextConstruct::text(&truncated));
    }

    if let Some(url) = json.get("home_page_url").and_then(|v| v.as_str())
        && url.len() <= limits.max_text_length
    {
        feed.link = Some(url.to_string());
    }

    if let Some(feed_url) = json.get("feed_url").and_then(|v| v.as_str()) {
        let _ = feed.links.try_push_limited(
            Link::self_link(feed_url, "application/feed+json"),
            limits.max_entries,
        );
    }

    if let Some(description) = json.get("description").and_then(|v| v.as_str()) {
        let truncated = truncate_to_length(description, limits.max_text_length);
        feed.subtitle_detail = Some(TextConstruct::text(&truncated));
        feed.subtitle = Some(truncated);
    }

    if let Some(icon) = json.get("icon").and_then(|v| v.as_str())
        && icon.len() <= limits.max_text_length
    {
        feed.icon = Some(icon.to_string());
    }

    if let Some(favicon) = json.get("favicon").and_then(|v| v.as_str())
        && favicon.len() <= limits.max_text_length
    {
        feed.logo = Some(favicon.to_string());
    }

    parse_authors(
        json,
        &mut feed.author,
        &mut feed.author_detail,
        &mut feed.authors,
        limits,
    );

    if let Some(language) = json.get("language").and_then(|v| v.as_str())
        && !language.is_empty()
        && language.len() <= limits.max_text_length
    {
        feed.language = Some(language.into());
        // Propagate feed-level language to title and subtitle TextConstructs
        if let Some(detail) = &mut feed.title_detail {
            detail.language = Some(language.into());
        }
        if let Some(detail) = &mut feed.subtitle_detail {
            detail.language = Some(language.into());
        }
    }

    if let Some(expired) = json.get("expired").and_then(Value::as_bool)
        && expired
    {
        feed.ttl = Some("0".to_string());
    }

    if let Some(next_url) = json.get("next_url").and_then(|v| v.as_str())
        && !next_url.is_empty()
        && next_url.len() <= limits.max_text_length
    {
        feed.next_url = Some(next_url.to_string());
    }
}

fn parse_item(
    json: &Value,
    limits: &ParserLimits,
    feed_lang: Option<&str>,
    feed_authors: &[Person],
) -> Entry {
    let mut entry = Entry::default();

    // Determine effective language: item-level overrides feed-level; empty clears it
    let item_lang = json
        .get("language")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty() && s.len() <= limits.max_text_length);
    let effective_lang = item_lang.or(feed_lang);

    if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
        entry.id = Some(id.into());
    }

    if let Some(url) = json.get("url").and_then(|v| v.as_str()) {
        entry.link = Some(url.to_string());
        let _ = entry
            .links
            .try_push_limited(Link::alternate(url), limits.max_entries);
    }

    if let Some(external_url) = json.get("external_url").and_then(|v| v.as_str()) {
        let _ = entry
            .links
            .try_push_limited(Link::related(external_url), limits.max_entries);
        entry.external_url = Some(external_url.to_string());
    }

    if let Some(title) = json.get("title").and_then(|v| v.as_str()) {
        let truncated = truncate_to_length(title, limits.max_text_length);
        entry.set_title(TextConstruct {
            value: truncated,
            content_type: crate::types::TextType::Text,
            language: effective_lang.map(Into::into),
            base: None,
        });
    }

    if let Some(content_html) = json.get("content_html").and_then(|v| v.as_str()) {
        let text = truncate_to_length(content_html, limits.max_text_length);
        let _ = entry.content.try_push_limited(
            Content {
                value: text,
                content_type: Some("text/html".into()),
                language: effective_lang.map(Into::into),
                base: None,
                src: None,
            },
            limits.max_entries,
        );
    }

    if let Some(content_text) = json.get("content_text").and_then(|v| v.as_str()) {
        let text = truncate_to_length(content_text, limits.max_text_length);
        let _ = entry.content.try_push_limited(
            Content {
                value: text,
                content_type: Some("text/plain".into()),
                language: effective_lang.map(Into::into),
                base: None,
                src: None,
            },
            limits.max_entries,
        );
    }

    if let Some(summary) = json.get("summary").and_then(|v| v.as_str()) {
        let truncated = truncate_to_length(summary, limits.max_text_length);
        entry.set_summary(TextConstruct {
            value: truncated,
            content_type: crate::types::TextType::Text,
            language: effective_lang.map(Into::into),
            base: None,
        });
    }

    if let Some(image) = json.get("image").and_then(|v| v.as_str()) {
        let _ = entry.links.try_push_limited(
            Link::enclosure(image, Some("image/*".into())),
            limits.max_entries,
        );
    }

    if let Some(banner) = json.get("banner_image").and_then(|v| v.as_str())
        && !banner.is_empty()
    {
        let _ = entry
            .links
            .try_push_limited(Link::banner(banner), limits.max_entries);
    }

    if let Some(date_str) = json.get("date_published").and_then(|v| v.as_str()) {
        entry.published = parse_date(date_str);
        entry.published_str = Some(date_str.to_string());
    }

    if let Some(date_str) = json.get("date_modified").and_then(|v| v.as_str()) {
        entry.updated = parse_date(date_str);
        entry.updated_str = Some(date_str.to_string());
    }

    parse_authors(
        json,
        &mut entry.author,
        &mut entry.author_detail,
        &mut entry.authors,
        limits,
    );

    // JSON Feed spec: inherit feed-level authors when entry has none
    if entry.authors.is_empty() && !feed_authors.is_empty() {
        entry.authors = feed_authors.to_vec();
        if entry.author.is_none()
            && let Some(first) = feed_authors.first()
        {
            entry.author.clone_from(&first.name);
            entry.author_detail = Some(first.clone());
        }
    }

    if let Some(tags) = json.get("tags").and_then(|v| v.as_array()) {
        for tag_val in tags {
            if let Some(tag_str) = tag_val.as_str() {
                let _ = entry
                    .tags
                    .try_push_limited(Tag::new(tag_str), limits.max_entries);
            }
        }
    }

    // Set entry.language from item-level field (effective_lang already set on constructs above)
    if let Some(lang) = effective_lang {
        entry.language = Some(lang.into());
    }

    if let Some(attachments) = json.get("attachments").and_then(|v| v.as_array()) {
        for attachment in attachments {
            if let Some(enclosure) = Enclosure::parse_from(attachment) {
                let _ = entry
                    .enclosures
                    .try_push_limited(enclosure, limits.max_entries);
            }
        }
    }

    entry
}

/// Unified author parsing for both feed and entry levels
///
/// Extracts authors from JSON Feed format (supports both "authors" array and legacy "author" object)
fn parse_authors(
    json: &Value,
    author: &mut Option<crate::types::SmallString>,
    author_detail: &mut Option<Person>,
    authors: &mut Vec<Person>,
    limits: &ParserLimits,
) {
    if let Some(authors_arr) = json.get("authors").and_then(Value::as_array) {
        for author_val in authors_arr {
            if let Some(parsed) = Person::parse_from(author_val) {
                if author.is_none() && parsed.name.is_some() {
                    author.clone_from(&parsed.name);
                    *author_detail = Some(parsed.clone());
                }
                let _ = authors.try_push_limited(parsed, limits.max_entries);
            }
        }
    } else if let Some(parsed) = json.get("author").and_then(Person::parse_from) {
        author.clone_from(&parsed.name);
        *author_detail = Some(parsed.clone());
        let _ = authors.try_push_limited(parsed, limits.max_entries);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_json_feed() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Test Feed",
            "items": []
        }"#;

        let feed = parse_json_feed(json).unwrap();
        assert_eq!(feed.version, FeedVersion::JsonFeed11);
        assert_eq!(feed.feed.title.as_deref(), Some("Test Feed"));
        assert!(!feed.bozo);
    }

    #[test]
    fn test_parse_json_feed_10() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1",
            "title": "Test Feed 1.0",
            "items": []
        }"#;

        let feed = parse_json_feed(json).unwrap();
        assert_eq!(feed.version, FeedVersion::JsonFeed10);
    }

    #[test]
    fn test_parse_json_feed_with_items() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Test Feed",
            "items": [
                {
                    "id": "1",
                    "title": "First Post",
                    "content_html": "<p>Hello world</p>",
                    "url": "https://example.com/1"
                }
            ]
        }"#;

        let feed = parse_json_feed(json).unwrap();
        assert_eq!(feed.entries.len(), 1);
        assert_eq!(feed.entries[0].id.as_deref(), Some("1"));
        assert_eq!(feed.entries[0].title.as_deref(), Some("First Post"));
        assert_eq!(
            feed.entries[0].link.as_deref(),
            Some("https://example.com/1")
        );
    }

    #[test]
    fn test_parse_json_feed_metadata() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Example Feed",
            "home_page_url": "https://example.com",
            "feed_url": "https://example.com/feed.json",
            "description": "Feed description",
            "icon": "https://example.com/icon.png",
            "favicon": "https://example.com/favicon.ico",
            "language": "en-US",
            "items": []
        }"#;

        let feed = parse_json_feed(json).unwrap();
        assert_eq!(feed.feed.title.as_deref(), Some("Example Feed"));
        assert_eq!(feed.feed.link.as_deref(), Some("https://example.com"));
        assert_eq!(feed.feed.subtitle.as_deref(), Some("Feed description"));
        // icon (512x512 app icon) → feed.icon
        assert_eq!(
            feed.feed.icon.as_deref(),
            Some("https://example.com/icon.png")
        );
        // favicon (small browser icon) → feed.logo
        assert_eq!(
            feed.feed.logo.as_deref(),
            Some("https://example.com/favicon.ico")
        );
        assert_eq!(feed.feed.language.as_deref(), Some("en-US"));
    }

    #[test]
    fn test_parse_json_feed_with_authors() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Test",
            "authors": [
                {"name": "John Doe", "url": "https://example.com/john"}
            ],
            "items": []
        }"#;

        let feed = parse_json_feed(json).unwrap();
        assert_eq!(feed.feed.author.as_deref(), Some("John Doe"));
        assert_eq!(feed.feed.authors.len(), 1);
        assert_eq!(
            feed.feed.authors[0].uri.as_deref(),
            Some("https://example.com/john")
        );
    }

    #[test]
    fn test_parse_item_with_dates() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Test",
            "items": [
                {
                    "id": "1",
                    "date_published": "2024-01-01T10:00:00Z",
                    "date_modified": "2024-01-02T12:00:00Z"
                }
            ]
        }"#;

        let feed = parse_json_feed(json).unwrap();
        assert!(feed.entries[0].published.is_some());
        assert!(feed.entries[0].updated.is_some());
    }

    #[test]
    fn test_parse_item_with_tags() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Test",
            "items": [
                {
                    "id": "1",
                    "tags": ["rust", "json", "feed"]
                }
            ]
        }"#;

        let feed = parse_json_feed(json).unwrap();
        assert_eq!(feed.entries[0].tags.len(), 3);
        assert_eq!(feed.entries[0].tags[0].term, "rust");
    }

    #[test]
    fn test_parse_item_with_attachments() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Test",
            "items": [
                {
                    "id": "1",
                    "attachments": [
                        {
                            "url": "https://example.com/file.mp3",
                            "mime_type": "audio/mpeg",
                            "size_in_bytes": 12345
                        }
                    ]
                }
            ]
        }"#;

        let feed = parse_json_feed(json).unwrap();
        assert_eq!(feed.entries[0].enclosures.len(), 1);
        assert_eq!(
            feed.entries[0].enclosures[0].url,
            "https://example.com/file.mp3"
        );
        assert_eq!(
            feed.entries[0].enclosures[0].enclosure_type.as_deref(),
            Some("audio/mpeg")
        );
        assert_eq!(
            feed.entries[0].enclosures[0].length.as_deref(),
            Some("12345")
        );
    }

    #[test]
    fn test_parse_invalid_json() {
        let json = b"not valid json";
        let feed = parse_json_feed(json).unwrap();
        assert!(feed.bozo);
        assert!(feed.bozo_exception.is_some());
    }

    #[test]
    fn test_parse_missing_version() {
        let json = br#"{"title": "Test"}"#;
        let result = parse_json_feed(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unknown_version() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/9.9",
            "title": "Test",
            "items": []
        }"#;

        let feed = parse_json_feed(json).unwrap();
        assert!(feed.bozo);
        assert!(feed.bozo_exception.is_some());
        assert_eq!(feed.version, FeedVersion::Unknown);
    }

    #[test]
    fn test_respects_max_entries_limit() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Test",
            "items": [
                {"id": "1"},
                {"id": "2"},
                {"id": "3"},
                {"id": "4"},
                {"id": "5"}
            ]
        }"#;

        let limits = ParserLimits {
            max_entries: 3,
            ..ParserLimits::default()
        };

        let feed = parse_json_feed_with_limits(json, limits).unwrap();
        assert_eq!(feed.entries.len(), 3);
        assert!(feed.bozo);
    }

    #[test]
    fn test_truncate_to_length() {
        assert_eq!(truncate_to_length("hello", 10), "hello");
        assert_eq!(truncate_to_length("hello world", 5), "hello");
        assert_eq!(truncate_to_length("", 10), "");
    }

    #[test]
    fn test_parse_json_feed_next_url() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Test",
            "next_url": "https://example.com/feed.json?page=2",
            "items": []
        }"#;

        let feed = parse_json_feed(json).unwrap();
        assert_eq!(
            feed.feed.next_url.as_deref(),
            Some("https://example.com/feed.json?page=2")
        );
        assert!(!feed.bozo);
    }

    #[test]
    fn test_parse_json_feed_next_url_absent() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Test",
            "items": []
        }"#;

        let feed = parse_json_feed(json).unwrap();
        assert!(feed.feed.next_url.is_none());
    }

    #[test]
    fn test_parse_json_feed_next_url_empty_string() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Test",
            "next_url": "",
            "items": []
        }"#;

        let feed = parse_json_feed(json).unwrap();
        assert!(feed.feed.next_url.is_none());
    }

    #[test]
    fn test_parse_json_feed_next_url_null() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Test",
            "next_url": null,
            "items": []
        }"#;

        let feed = parse_json_feed(json).unwrap();
        assert!(feed.feed.next_url.is_none());
    }

    #[test]
    fn test_parse_next_url_respects_text_limit() {
        let long_url = "https://example.com/".to_string() + &"a".repeat(10_000);
        let json = format!(
            r#"{{
                "version": "https://jsonfeed.org/version/1.1",
                "title": "Test",
                "next_url": "{long_url}",
                "items": []
            }}"#
        );

        let limits = ParserLimits {
            max_text_length: 100,
            ..ParserLimits::default()
        };
        let feed = parse_json_feed_with_limits(json.as_bytes(), limits).unwrap();
        assert!(feed.feed.next_url.is_none());
    }

    #[test]
    fn test_parse_item_banner_image() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Test",
            "items": [
                {
                    "id": "1",
                    "banner_image": "https://example.com/banner.jpg"
                }
            ]
        }"#;

        let feed = parse_json_feed(json).unwrap();
        let banner = feed.entries[0]
            .links
            .iter()
            .find(|l| l.rel.as_deref() == Some("banner"));
        assert!(banner.is_some());
        assert_eq!(
            banner.unwrap().href.as_str(),
            "https://example.com/banner.jpg"
        );
    }

    #[test]
    fn test_parse_item_banner_image_absent() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Test",
            "items": [{"id": "1"}]
        }"#;

        let feed = parse_json_feed(json).unwrap();
        let banner = feed.entries[0]
            .links
            .iter()
            .find(|l| l.rel.as_deref() == Some("banner"));
        assert!(banner.is_none());
    }

    #[test]
    fn test_parse_item_banner_image_empty_string() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Test",
            "items": [{"id": "1", "banner_image": ""}]
        }"#;

        let feed = parse_json_feed(json).unwrap();
        let banner = feed.entries[0]
            .links
            .iter()
            .find(|l| l.rel.as_deref() == Some("banner"));
        assert!(banner.is_none());
    }

    #[test]
    fn test_parse_item_image_and_banner_image() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Test",
            "items": [
                {
                    "id": "1",
                    "image": "https://example.com/image.jpg",
                    "banner_image": "https://example.com/banner.jpg"
                }
            ]
        }"#;

        let feed = parse_json_feed(json).unwrap();
        let enclosure = feed.entries[0]
            .links
            .iter()
            .find(|l| l.rel.as_deref() == Some("enclosure"));
        let banner = feed.entries[0]
            .links
            .iter()
            .find(|l| l.rel.as_deref() == Some("banner"));
        assert!(enclosure.is_some());
        assert!(banner.is_some());
        assert_eq!(
            enclosure.unwrap().href.as_str(),
            "https://example.com/image.jpg"
        );
        assert_eq!(
            banner.unwrap().href.as_str(),
            "https://example.com/banner.jpg"
        );
    }

    #[test]
    fn test_parse_item_external_url() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Test",
            "items": [
                {
                    "id": "1",
                    "external_url": "https://example.com/original"
                }
            ]
        }"#;

        let feed = parse_json_feed(json).unwrap();
        assert_eq!(
            feed.entries[0].external_url.as_deref(),
            Some("https://example.com/original")
        );
        // Also stored as a related link for backward compat
        let related = feed.entries[0]
            .links
            .iter()
            .find(|l| l.rel.as_deref() == Some("related"));
        assert!(related.is_some());
    }

    #[test]
    fn test_parse_item_language() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Test",
            "items": [
                {
                    "id": "1",
                    "title": "Hello",
                    "language": "de"
                }
            ]
        }"#;

        let feed = parse_json_feed(json).unwrap();
        assert_eq!(feed.entries[0].language.as_deref(), Some("de"));
    }

    #[test]
    fn test_parse_attachment_title_and_duration() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Test",
            "items": [
                {
                    "id": "1",
                    "attachments": [
                        {
                            "url": "https://example.com/audio.mp3",
                            "mime_type": "audio/mpeg",
                            "title": "Episode 1",
                            "duration_in_seconds": 7200
                        }
                    ]
                }
            ]
        }"#;

        let feed = parse_json_feed(json).unwrap();
        let enc = &feed.entries[0].enclosures[0];
        assert_eq!(enc.title.as_deref(), Some("Episode 1"));
        assert_eq!(enc.duration.as_deref(), Some("7200"));
    }

    #[test]
    fn test_parse_author_avatar() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Test",
            "authors": [
                {
                    "name": "Jane Doe",
                    "url": "https://example.com/jane",
                    "avatar": "https://example.com/jane/avatar.png"
                }
            ],
            "items": [
                {
                    "id": "1",
                    "authors": [
                        {
                            "name": "John Doe",
                            "avatar": "https://example.com/john/avatar.jpg"
                        }
                    ]
                }
            ]
        }"#;

        let feed = parse_json_feed(json).unwrap();
        assert_eq!(
            feed.feed.authors[0].avatar.as_deref(),
            Some("https://example.com/jane/avatar.png")
        );
        assert_eq!(
            feed.entries[0].authors[0].avatar.as_deref(),
            Some("https://example.com/john/avatar.jpg")
        );
        assert!(!feed.bozo);
    }

    #[test]
    fn test_json_feed_level_language_propagates_to_item_constructs() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Feed",
            "language": "de",
            "items": [
                {
                    "id": "1",
                    "title": "Artikel",
                    "content_html": "<p>Inhalt</p>",
                    "summary": "Kurz"
                }
            ]
        }"#;
        let feed = parse_json_feed(json).unwrap();
        assert!(!feed.bozo);
        let entry = &feed.entries[0];
        assert_eq!(
            entry.title_detail.as_ref().unwrap().language.as_deref(),
            Some("de")
        );
        assert_eq!(
            entry.summary_detail.as_ref().unwrap().language.as_deref(),
            Some("de")
        );
        assert_eq!(entry.content[0].language.as_deref(), Some("de"));
    }

    #[test]
    fn test_json_item_language_overrides_feed_language() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Feed",
            "language": "de",
            "items": [
                {
                    "id": "1",
                    "title": "Titre",
                    "language": "fr"
                }
            ]
        }"#;
        let feed = parse_json_feed(json).unwrap();
        assert!(!feed.bozo);
        let entry = &feed.entries[0];
        assert_eq!(
            entry.title_detail.as_ref().unwrap().language.as_deref(),
            Some("fr")
        );
        assert_eq!(entry.language.as_deref(), Some("fr"));
    }

    #[test]
    fn test_json_feed_language_on_feed_title_detail() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Feed Titel",
            "description": "Beschreibung",
            "language": "de",
            "items": []
        }"#;
        let feed = parse_json_feed(json).unwrap();
        assert!(!feed.bozo);
        assert_eq!(
            feed.feed.title_detail.as_ref().unwrap().language.as_deref(),
            Some("de")
        );
        assert_eq!(
            feed.feed
                .subtitle_detail
                .as_ref()
                .unwrap()
                .language
                .as_deref(),
            Some("de")
        );
    }

    #[test]
    fn test_entry_inherits_feed_authors_when_none() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Test",
            "authors": [{"name": "Feed Author"}],
            "items": [
                {"id": "1"},
                {"id": "2", "authors": [{"name": "Item Author"}]}
            ]
        }"#;

        let feed = parse_json_feed(json).unwrap();
        // Item without authors inherits from feed
        assert_eq!(
            feed.entries[0].author.as_deref(),
            Some("Feed Author"),
            "entry without authors should inherit feed-level author"
        );
        assert_eq!(feed.entries[0].authors.len(), 1);
        // Item with its own authors does NOT inherit
        assert_eq!(
            feed.entries[1].author.as_deref(),
            Some("Item Author"),
            "entry with own authors should not be overridden"
        );
        assert_eq!(feed.entries[1].authors.len(), 1);
        assert!(!feed.bozo);
    }

    #[test]
    fn test_json_no_language_yields_none() {
        let json = br#"{
            "version": "https://jsonfeed.org/version/1.1",
            "title": "Feed",
            "items": [{"id": "1", "title": "Entry"}]
        }"#;
        let feed = parse_json_feed(json).unwrap();
        assert!(!feed.bozo);
        assert!(feed.feed.title_detail.as_ref().unwrap().language.is_none());
        assert!(
            feed.entries[0]
                .title_detail
                .as_ref()
                .unwrap()
                .language
                .is_none()
        );
    }
}
