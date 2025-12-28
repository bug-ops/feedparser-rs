//! URL resolution integration tests for xml:base support.
#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::Datelike;
use feedparser_rs::parse;

#[test]
fn test_atom_feed_level_xml_base() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://example.org/blog/">
        <title>Test Feed</title>
        <link href="index.html" rel="alternate" />
        <icon>icon.png</icon>
        <logo>logo.png</logo>
    </feed>
    "#;

    let feed = parse(xml).unwrap();

    assert_eq!(
        feed.feed.links[0].href,
        "http://example.org/blog/index.html"
    );
    assert_eq!(
        feed.feed.icon.as_deref(),
        Some("http://example.org/blog/icon.png")
    );
    assert_eq!(
        feed.feed.logo.as_deref(),
        Some("http://example.org/blog/logo.png")
    );
}

#[test]
fn test_atom_entry_level_xml_base() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://example.org/">
        <title>Test</title>
        <entry xml:base="posts/">
            <title>Post 1</title>
            <link href="123.html" rel="alternate" />
        </entry>
    </feed>
    "#;

    let feed = parse(xml).unwrap();
    let entry = &feed.entries[0];

    assert_eq!(
        entry.link.as_deref(),
        Some("http://example.org/posts/123.html")
    );
    assert_eq!(entry.links[0].href, "http://example.org/posts/123.html");
}

#[test]
fn test_atom_nested_xml_base_override() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://example.org/feed/">
        <title>Test</title>
        <entry xml:base="http://other.org/">
            <title>Post 1</title>
            <link href="post/123" rel="alternate" />
        </entry>
    </feed>
    "#;

    let feed = parse(xml).unwrap();
    let entry = &feed.entries[0];

    // Entry's absolute xml:base overrides feed base
    assert_eq!(entry.link.as_deref(), Some("http://other.org/post/123"));
}

#[test]
fn test_rss_item_link_resolution() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Test Podcast</title>
            <link>http://example.org/feed/</link>
            <item>
                <title>Episode 1</title>
                <link>episodes/ep1.html</link>
            </item>
        </channel>
    </rss>
    "#;

    let feed = parse(xml).unwrap();
    let entry = &feed.entries[0];

    assert_eq!(
        entry.link.as_deref(),
        Some("http://example.org/feed/episodes/ep1.html")
    );
}

#[test]
fn test_rss_enclosure_url_resolution() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Podcast</title>
            <link>http://example.org/feed/</link>
            <item>
                <title>Episode 1</title>
                <enclosure url="media/ep1.mp3" type="audio/mpeg" length="12345" />
            </item>
        </channel>
    </rss>
    "#;

    let feed = parse(xml).unwrap();
    let enclosure = &feed.entries[0].enclosures[0];

    assert_eq!(enclosure.url, "http://example.org/feed/media/ep1.mp3");
}

#[test]
fn test_absolute_urls_unchanged() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://example.org/">
        <entry>
            <link href="http://absolute.com/page" rel="alternate" />
        </entry>
    </feed>
    "#;

    let feed = parse(xml).unwrap();

    // Absolute URLs should not be modified
    assert_eq!(
        feed.entries[0].link.as_deref(),
        Some("http://absolute.com/page")
    );
}

#[test]
fn test_mailto_urls_unchanged() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://example.org/">
        <entry>
            <link href="mailto:test@example.com" rel="alternate" />
        </entry>
    </feed>
    "#;

    let feed = parse(xml).unwrap();

    // Special schemes should be preserved
    assert_eq!(
        feed.entries[0].link.as_deref(),
        Some("mailto:test@example.com")
    );
}

#[test]
fn test_rss_no_base_url_leaves_relative() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Test</title>
            <item>
                <link>relative/link.html</link>
            </item>
        </channel>
    </rss>
    "#;

    let feed = parse(xml).unwrap();

    // Without a channel link, relative URLs stay relative
    assert_eq!(feed.entries[0].link.as_deref(), Some("relative/link.html"));
}

#[test]
fn test_atom_feed_published_field() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom">
        <title>Test Feed</title>
        <published>2025-01-01T00:00:00Z</published>
    </feed>
    "#;

    let feed = parse(xml).unwrap();

    assert!(feed.feed.published.is_some());
    let dt = feed.feed.published.unwrap();
    assert_eq!(dt.year(), 2025);
    assert_eq!(dt.month(), 1);
    assert_eq!(dt.day(), 1);
}

#[test]
fn test_rss_channel_pubdate_maps_to_published() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Test Podcast</title>
            <pubDate>Wed, 18 Dec 2024 10:00:00 +0000</pubDate>
        </channel>
    </rss>
    "#;

    let feed = parse(xml).unwrap();

    assert!(feed.feed.published.is_some());
    let dt = feed.feed.published.unwrap();
    assert_eq!(dt.year(), 2024);
    assert_eq!(dt.month(), 12);
    assert_eq!(dt.day(), 18);
}
