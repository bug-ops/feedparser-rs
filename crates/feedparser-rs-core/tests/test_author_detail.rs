//! Regression tests for issue #127: `author_detail.name` must not be None
//! when the feed provides an author name.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use feedparser_rs::parse;

const ATOM_FEED_LEVEL_AUTHOR: &[u8] = br#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test</title>
  <id>urn:test</id>
  <updated>2026-01-01T00:00:00Z</updated>
  <author><name>Jane Doe</name><email>jane@example.com</email></author>
  <entry>
    <title>Entry</title>
    <id>urn:entry1</id>
    <updated>2026-01-01T00:00:00Z</updated>
  </entry>
</feed>"#;

const ATOM_ENTRY_LEVEL_AUTHOR: &[u8] = br#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test</title>
  <id>urn:test</id>
  <updated>2026-01-01T00:00:00Z</updated>
  <entry>
    <title>Entry</title>
    <id>urn:entry1</id>
    <updated>2026-01-01T00:00:00Z</updated>
    <author><name>Jane Doe</name><email>jane@example.com</email></author>
  </entry>
</feed>"#;

const RSS2_AUTHOR: &[u8] = br#"<?xml version="1.0"?>
<rss version="2.0" xmlns:dc="http://purl.org/dc/elements/1.1/">
  <channel>
    <title>Test</title>
    <link>http://example.com</link>
    <description>Test feed</description>
    <dc:creator>Jane Doe</dc:creator>
    <item>
      <title>Item</title>
      <link>http://example.com/item1</link>
      <dc:creator>Jane Doe</dc:creator>
    </item>
  </channel>
</rss>"#;

#[test]
fn test_atom_feed_author_detail_name_not_none() {
    let feed = parse(ATOM_FEED_LEVEL_AUTHOR).expect("parse failed");
    assert!(!feed.bozo, "valid feed must not set bozo");
    let meta = &feed.feed;
    assert_eq!(meta.author.as_deref(), Some("Jane Doe"));
    let detail = meta
        .author_detail
        .as_ref()
        .expect("author_detail must be Some");
    assert_eq!(
        detail.name.as_deref(),
        Some("Jane Doe"),
        "author_detail.name must not be None"
    );
    assert_eq!(detail.email.as_deref(), Some("jane@example.com"));
}

#[test]
fn test_atom_entry_author_detail_name_not_none() {
    let feed = parse(ATOM_ENTRY_LEVEL_AUTHOR).expect("parse failed");
    assert!(!feed.bozo, "valid feed must not set bozo");
    let entry = &feed.entries[0];
    assert_eq!(entry.author.as_deref(), Some("Jane Doe"));
    let detail = entry
        .author_detail
        .as_ref()
        .expect("author_detail must be Some");
    assert_eq!(
        detail.name.as_deref(),
        Some("Jane Doe"),
        "author_detail.name must not be None"
    );
    assert_eq!(detail.email.as_deref(), Some("jane@example.com"));
}

#[test]
fn test_set_author_preserves_name_in_detail() {
    use feedparser_rs::{Entry, Person};
    let mut entry = Entry::default();
    entry.set_author(Person::from_name("Jane Doe"));
    assert_eq!(entry.author.as_deref(), Some("Jane Doe"));
    let detail = entry
        .author_detail
        .as_ref()
        .expect("author_detail must be Some");
    assert_eq!(detail.name.as_deref(), Some("Jane Doe"));
}

#[test]
fn test_set_publisher_preserves_name_in_detail() {
    use feedparser_rs::{Entry, Person};
    let mut entry = Entry::default();
    entry.set_publisher(Person::from_name("ACME Corp"));
    assert_eq!(entry.publisher.as_deref(), Some("ACME Corp"));
    let detail = entry
        .publisher_detail
        .as_ref()
        .expect("publisher_detail must be Some");
    assert_eq!(detail.name.as_deref(), Some("ACME Corp"));
}

#[test]
fn test_rss2_dc_creator_author_detail_name() {
    let feed = parse(RSS2_AUTHOR).expect("parse failed");
    let entry = &feed.entries[0];
    if let Some(detail) = &entry.author_detail {
        assert_eq!(
            detail.name.as_deref(),
            entry.author.as_deref(),
            "author_detail.name must match author shorthand"
        );
    }
}
