#![allow(
    missing_docs,
    clippy::if_then_some_else_none,
    clippy::single_match_else,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use feedparser_rs::{FeedVersion, detect_format, parse};

/// Helper function to load test fixtures
fn load_fixture(path: &str) -> Vec<u8> {
    // Fixtures are in the workspace root tests/fixtures/ directory
    let fixture_path = format!("../../tests/fixtures/{path}");
    std::fs::read(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to load fixture '{fixture_path}': {e}"))
}

/// Helper to assert basic feed validity
fn assert_feed_valid(result: &feedparser_rs::ParsedFeed) {
    assert!(result.version == FeedVersion::Unknown || !result.bozo);
}

#[test]
fn test_parse_rss_basic_fixture() {
    let xml = load_fixture("rss/basic.xml");
    let result = parse(&xml);

    assert!(result.is_ok(), "Failed to parse RSS fixture");
    let feed = result.unwrap();

    assert_feed_valid(&feed);
}

#[test]
fn test_parse_atom_basic_fixture() {
    let xml = load_fixture("atom/basic.xml");
    let result = parse(&xml);

    assert!(result.is_ok(), "Failed to parse Atom fixture");
    let feed = result.unwrap();

    assert_feed_valid(&feed);
}

#[test]
fn test_detect_format_rss() {
    let xml = load_fixture("rss/basic.xml");
    let version = detect_format(&xml);
    let _ = version;
}

#[test]
fn test_detect_format_atom() {
    let xml = load_fixture("atom/basic.xml");
    let version = detect_format(&xml);
    let _ = version;
}

#[test]
fn test_parse_empty_input() {
    let feed = parse(b"").unwrap();
    assert!(feed.bozo, "empty input must set bozo");
    assert_eq!(feed.version, FeedVersion::Unknown);
    assert!(feed.entries.is_empty());
}

#[test]
fn test_parse_whitespace_only() {
    let feed = parse(b"   \n  ").unwrap();
    assert!(feed.bozo, "whitespace-only input must set bozo");
    assert_eq!(feed.version, FeedVersion::Unknown);
    assert!(feed.entries.is_empty());
}

#[test]
fn test_parse_invalid_xml() {
    let feed = parse(b"<invalid><xml>").unwrap();
    assert!(feed.bozo, "invalid XML must set bozo");
    assert_eq!(feed.version, FeedVersion::Unknown);
}

#[test]
fn test_parse_doctype_only() {
    let feed = parse(b"<!DOCTYPE foo>").unwrap();
    assert!(feed.bozo, "DOCTYPE-only input must set bozo");
    assert_eq!(feed.version, FeedVersion::Unknown);
}

#[test]
fn test_parse_invalid_utf8_bytes() {
    let feed = parse(b"\xFF\xFE").unwrap();
    assert!(feed.bozo, "invalid UTF-8 bytes must set bozo");
    assert_eq!(feed.version, FeedVersion::Unknown);
}

#[test]
fn test_capacity_constructors() {
    use feedparser_rs::{Entry, FeedMeta, ParsedFeed};

    // Test ParsedFeed::with_capacity
    let feed = ParsedFeed::with_capacity(100);
    assert_eq!(feed.encoding, "utf-8");
    assert_eq!(feed.entries.capacity(), 100);
    assert!(feed.namespaces.capacity() >= 8);

    // Test FeedMeta::with_rss_capacity
    let rss_meta = FeedMeta::with_rss_capacity();
    assert!(rss_meta.links.capacity() >= 2);
    assert!(rss_meta.authors.capacity() >= 1);
    assert!(rss_meta.tags.capacity() >= 3);

    // Test FeedMeta::with_atom_capacity
    let atom_meta = FeedMeta::with_atom_capacity();
    assert!(atom_meta.links.capacity() >= 4);
    assert!(atom_meta.authors.capacity() >= 2);
    assert!(atom_meta.tags.capacity() >= 5);

    // Test Entry::with_capacity
    let entry = Entry::with_capacity();
    assert!(entry.links.capacity() >= 2);
    assert!(entry.content.capacity() >= 1);
    assert!(entry.authors.capacity() >= 1);
    assert!(entry.tags.capacity() >= 3);
}

#[test]
fn test_parse_json_feed_basic() {
    let json = load_fixture("json/basic-1.1.json");
    let result = parse(&json);

    assert!(result.is_ok(), "Failed to parse JSON Feed fixture");
    let feed = result.unwrap();

    assert_eq!(feed.version, FeedVersion::JsonFeed11);
    assert!(!feed.bozo);
    assert_eq!(feed.feed.title.as_deref(), Some("Example JSON Feed"));
    assert_eq!(feed.entries.len(), 1);
    assert_eq!(feed.entries[0].id.as_deref(), Some("1"));
    assert_eq!(feed.entries[0].title.as_deref(), Some("First Post"));
}

#[test]
fn test_parse_json_feed_10() {
    let json = load_fixture("json/basic-1.0.json");
    let result = parse(&json);

    assert!(result.is_ok());
    let feed = result.unwrap();

    assert_eq!(feed.version, FeedVersion::JsonFeed10);
    assert!(!feed.bozo);
}

#[test]
fn test_parse_json_feed_minimal() {
    let json = load_fixture("json/minimal.json");
    let result = parse(&json);

    assert!(result.is_ok());
    let feed = result.unwrap();

    assert_eq!(feed.version, FeedVersion::JsonFeed11);
    assert!(!feed.bozo);
    assert_eq!(feed.feed.title.as_deref(), Some("Minimal Feed"));
    assert_eq!(feed.entries.len(), 0);
}

#[test]
fn test_parse_itunes_podcast_feed() {
    let xml = load_fixture("podcast/itunes-basic.xml");
    let result = parse(&xml);

    assert!(result.is_ok(), "Failed to parse iTunes podcast fixture");
    let feed = result.unwrap();

    // Verify basic feed properties
    assert_eq!(feed.version, FeedVersion::Rss20);
    assert!(!feed.bozo, "Feed should not have bozo flag set");
    assert_eq!(feed.feed.title.as_deref(), Some("Example Podcast"));

    // Verify iTunes feed-level metadata
    assert!(
        feed.feed.itunes.is_some(),
        "Feed should have iTunes metadata"
    );
    let itunes = feed.feed.itunes.as_ref().unwrap();

    assert_eq!(itunes.author.as_deref(), Some("John Doe"));
    assert_eq!(itunes.explicit, Some(false));
    assert_eq!(
        itunes.image.as_deref(),
        Some("https://example.com/podcast-cover.jpg")
    );
    assert!(!itunes.categories.is_empty());
    assert_eq!(itunes.categories[0].text, "Technology");

    // Verify owner information
    assert!(itunes.owner.is_some());
    let owner = itunes.owner.as_ref().unwrap();
    assert_eq!(owner.name.as_deref(), Some("Jane Smith"));
    assert_eq!(owner.email.as_deref(), Some("contact@example.com"));

    // Verify keywords
    assert_eq!(itunes.keywords, vec!["rust", "programming", "tech"]);

    // Verify podcast type
    assert_eq!(itunes.podcast_type.as_deref(), Some("episodic"));

    assert!(!feed.entries.is_empty(), "Feed should have episodes");
}

#[test]
fn test_rss_with_unknown_entity_sets_bozo() {
    let xml = br#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>Test &unknown; Feed</title>
    <link>https://example.com</link>
    <description>desc</description>
  </channel>
</rss>"#;
    let result = parse(xml).unwrap();
    assert!(result.bozo, "Feed with unknown entity should set bozo=true");
    assert!(
        result
            .bozo_exception
            .as_deref()
            .unwrap_or("")
            .contains("Unresolvable entity"),
        "bozo_exception should mention unresolvable entity"
    );
}

#[test]
fn test_atom_with_unknown_entity_sets_bozo() {
    let xml = br#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <id>https://example.com/&unknown;feed</id>
  <title>Test Feed</title>
  <updated>2024-01-01T00:00:00Z</updated>
</feed>"#;
    let result = parse(xml).unwrap();
    assert!(
        result.bozo,
        "Atom feed with unknown entity should set bozo=true"
    );
}

#[test]
fn test_rss20_entry_entity_bozo() {
    let xml = load_fixture("malformed/entry-entity-rss20.xml");
    let feed = parse(&xml).unwrap();
    assert!(feed.bozo);
    assert_eq!(
        feed.bozo_exception.as_deref(),
        Some("Unresolvable entity in entry field")
    );
    assert_eq!(feed.entries.len(), 1);
}

#[test]
fn test_atom10_entry_entity_bozo() {
    let xml = load_fixture("malformed/entry-entity-atom10.xml");
    let feed = parse(&xml).unwrap();
    assert!(feed.bozo);
    assert_eq!(
        feed.bozo_exception.as_deref(),
        Some("Unresolvable entity in entry field")
    );
    assert_eq!(feed.entries.len(), 1);
}

#[test]
fn test_rss10_entry_entity_bozo() {
    let xml = load_fixture("malformed/entry-entity-rss10.xml");
    let feed = parse(&xml).unwrap();
    assert!(feed.bozo);
    assert_eq!(
        feed.bozo_exception.as_deref(),
        Some("Unresolvable entity in entry field")
    );
    assert_eq!(feed.entries.len(), 1);
}

#[test]
fn test_parse_atom03_fixture() {
    let xml = load_fixture("atom/atom03.xml");
    let result = parse(&xml);

    assert!(result.is_ok(), "Failed to parse Atom 0.3 fixture");
    let feed = result.unwrap();

    assert!(!feed.bozo, "Well-formed Atom 0.3 feed must not set bozo");
    assert_eq!(feed.version, FeedVersion::Atom03, "Version must be atom03");
    assert_eq!(feed.feed.title.as_deref(), Some("Example Atom 0.3 Feed"));
    assert!(feed.feed.link.is_some(), "Feed link must be populated");
    assert_eq!(feed.entries.len(), 2, "Feed must have two entries");

    let entry = &feed.entries[0];
    assert_eq!(entry.title.as_deref(), Some("First Entry"));
    assert!(entry.id.is_some(), "Entry id must be populated");
    assert!(entry.updated.is_some(), "Entry updated must be parsed");
}

#[test]
fn test_detect_format_atom03_fixture() {
    let xml = load_fixture("atom/atom03.xml");
    let version = detect_format(&xml);
    assert_eq!(version, FeedVersion::Atom03);
}

#[test]
fn test_feed_with_standard_entities_no_bozo() {
    let xml = br#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>AT&amp;T Feed</title>
    <link>https://example.com</link>
    <description>Less &lt;than&gt; and more</description>
  </channel>
</rss>"#;
    let result = parse(xml).unwrap();
    assert!(
        !result.bozo,
        "Feed with only standard entities should not set bozo"
    );
}
