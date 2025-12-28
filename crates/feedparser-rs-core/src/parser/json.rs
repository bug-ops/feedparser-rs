//! JSON Feed parser (versions 1.0 and 1.1)
//!
//! Specification: <https://www.jsonfeed.org/version/1.1/>

use crate::{
    ParserLimits,
    error::{FeedError, Result},
    types::{
        Content, Enclosure, Entry, FeedMeta, FeedVersion, Image, LimitedCollectionExt, Link,
        ParseFrom, ParsedFeed, Person, Tag, TextConstruct,
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
            feed.entries.push(parse_item(item, &limits));
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
        feed.image = Some(Image {
            url: favicon.to_string().into(),
            title: None,
            link: None,
            width: None,
            height: None,
            description: None,
        });
    }

    parse_authors(
        json,
        &mut feed.author,
        &mut feed.author_detail,
        &mut feed.authors,
        limits,
    );

    if let Some(language) = json.get("language").and_then(|v| v.as_str())
        && language.len() <= limits.max_text_length
    {
        feed.language = Some(language.into());
    }

    if let Some(expired) = json.get("expired").and_then(Value::as_bool)
        && expired
    {
        feed.ttl = Some(0);
    }
}

fn parse_item(json: &Value, limits: &ParserLimits) -> Entry {
    let mut entry = Entry::default();

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
    }

    if let Some(title) = json.get("title").and_then(|v| v.as_str()) {
        let truncated = truncate_to_length(title, limits.max_text_length);
        entry.set_title(TextConstruct::text(&truncated));
    }

    if let Some(content_html) = json.get("content_html").and_then(|v| v.as_str()) {
        let text = truncate_to_length(content_html, limits.max_text_length);
        let _ = entry
            .content
            .try_push_limited(Content::html(text), limits.max_entries);
    }

    if let Some(content_text) = json.get("content_text").and_then(|v| v.as_str()) {
        let text = truncate_to_length(content_text, limits.max_text_length);
        let _ = entry
            .content
            .try_push_limited(Content::plain(text), limits.max_entries);
    }

    if let Some(summary) = json.get("summary").and_then(|v| v.as_str()) {
        let truncated = truncate_to_length(summary, limits.max_text_length);
        entry.set_summary(TextConstruct::text(&truncated));
    }

    if let Some(image) = json.get("image").and_then(|v| v.as_str()) {
        let _ = entry.links.try_push_limited(
            Link::enclosure(image, Some("image/*".into())),
            limits.max_entries,
        );
    }

    if let Some(date_str) = json.get("date_published").and_then(|v| v.as_str()) {
        entry.published = parse_date(date_str);
    }

    if let Some(date_str) = json.get("date_modified").and_then(|v| v.as_str()) {
        entry.updated = parse_date(date_str);
    }

    parse_authors(
        json,
        &mut entry.author,
        &mut entry.author_detail,
        &mut entry.authors,
        limits,
    );

    if let Some(tags) = json.get("tags").and_then(|v| v.as_array()) {
        for tag_val in tags {
            if let Some(tag_str) = tag_val.as_str() {
                let _ = entry
                    .tags
                    .try_push_limited(Tag::new(tag_str), limits.max_entries);
            }
        }
    }

    if let Some(language) = json.get("language").and_then(|v| v.as_str()) {
        if let Some(detail) = &mut entry.title_detail {
            detail.language = Some(language.into());
        }
        if let Some(detail) = &mut entry.summary_detail {
            detail.language = Some(language.into());
        }
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
            "language": "en-US",
            "items": []
        }"#;

        let feed = parse_json_feed(json).unwrap();
        assert_eq!(feed.feed.title.as_deref(), Some("Example Feed"));
        assert_eq!(feed.feed.link.as_deref(), Some("https://example.com"));
        assert_eq!(feed.feed.subtitle.as_deref(), Some("Feed description"));
        assert_eq!(
            feed.feed.icon.as_deref(),
            Some("https://example.com/icon.png")
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
        assert_eq!(feed.entries[0].enclosures[0].length, Some(12345));
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
}
