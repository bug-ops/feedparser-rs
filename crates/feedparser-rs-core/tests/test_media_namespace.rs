#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for media namespace fixes (#208, #229, #302)

use feedparser_rs::parse;

// --- Issue #229: media:thumbnail time attribute ---

#[test]
fn test_media_thumbnail_time_attribute_rss_entry() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>Test Feed</title>
            <link>http://example.com</link>
            <item>
                <title>Entry with thumbnail time</title>
                <media:thumbnail url="http://example.com/thumb.jpg" width="320" height="180" time="12:05:01.123"/>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert_eq!(feed.entries.len(), 1);
    let entry = &feed.entries[0];
    assert_eq!(entry.media_thumbnail.len(), 1);
    let thumb = &entry.media_thumbnail[0];
    assert_eq!(&*thumb.url, "http://example.com/thumb.jpg");
    assert_eq!(thumb.width.as_deref(), Some("320"));
    assert_eq!(thumb.height.as_deref(), Some("180"));
    assert_eq!(thumb.time.as_deref(), Some("12:05:01.123"));
}

#[test]
fn test_media_thumbnail_time_attribute_atom_entry() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xmlns:media="http://search.yahoo.com/mrss/">
        <title>Test Feed</title>
        <link href="http://example.com/"/>
        <id>http://example.com/</id>
        <entry>
            <title>Entry with thumbnail time</title>
            <link href="http://example.com/1"/>
            <id>http://example.com/1</id>
            <media:thumbnail url="http://example.com/thumb.jpg" width="640" height="480" time="00:01:30.000"/>
        </entry>
    </feed>"#;

    let feed = parse(xml).unwrap();
    assert_eq!(feed.entries.len(), 1);
    let entry = &feed.entries[0];
    assert_eq!(entry.media_thumbnail.len(), 1);
    let thumb = &entry.media_thumbnail[0];
    assert_eq!(thumb.time.as_deref(), Some("00:01:30.000"));
}

#[test]
fn test_media_thumbnail_without_time() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>Test Feed</title>
            <link>http://example.com</link>
            <item>
                <title>Entry without thumbnail time</title>
                <media:thumbnail url="http://example.com/thumb.jpg" width="320" height="180"/>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    let entry = &feed.entries[0];
    assert_eq!(entry.media_thumbnail.len(), 1);
    let thumb = &entry.media_thumbnail[0];
    assert!(thumb.time.is_none());
}

// --- Issue #302: Feed/channel-level media:rating and media:keywords ---

#[test]
fn test_rss_channel_media_keywords() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>Test Feed</title>
            <link>http://example.com</link>
            <media:keywords>news, technology, rust</media:keywords>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert_eq!(
        feed.feed.media_keywords.as_deref(),
        Some("news, technology, rust")
    );
}

#[test]
fn test_rss_channel_media_rating() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>Test Feed</title>
            <link>http://example.com</link>
            <media:rating>nonadult</media:rating>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert_eq!(
        feed.feed.media_rating.as_ref().map(|r| r.content.as_str()),
        Some("nonadult")
    );
}

#[test]
fn test_rss_channel_media_thumbnail() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>Test Feed</title>
            <link>http://example.com</link>
            <media:thumbnail url="http://example.com/channel-thumb.jpg" width="640" height="480" time="00:01:00.000"/>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert_eq!(feed.feed.media_thumbnail.len(), 1);
    let thumb = &feed.feed.media_thumbnail[0];
    assert_eq!(&*thumb.url, "http://example.com/channel-thumb.jpg");
    assert_eq!(thumb.width.as_deref(), Some("640"));
    assert_eq!(thumb.height.as_deref(), Some("480"));
    assert_eq!(thumb.time.as_deref(), Some("00:01:00.000"));
}

#[test]
fn test_rss_channel_media_content() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>Test Feed</title>
            <link>http://example.com</link>
            <media:content url="http://example.com/channel-video.mp4" type="video/mp4" medium="video"/>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert_eq!(feed.feed.media_content.len(), 1);
    let content = &feed.feed.media_content[0];
    assert_eq!(&*content.url, "http://example.com/channel-video.mp4");
    assert_eq!(content.medium.as_deref(), Some("video"));
}

#[test]
fn test_atom_feed_media_keywords() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xmlns:media="http://search.yahoo.com/mrss/">
        <title>Test Feed</title>
        <link href="http://example.com/"/>
        <id>http://example.com/</id>
        <media:keywords>tech, programming</media:keywords>
    </feed>"#;

    let feed = parse(xml).unwrap();
    assert_eq!(
        feed.feed.media_keywords.as_deref(),
        Some("tech, programming")
    );
}

#[test]
fn test_atom_feed_media_rating() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xmlns:media="http://search.yahoo.com/mrss/">
        <title>Test Feed</title>
        <link href="http://example.com/"/>
        <id>http://example.com/</id>
        <media:rating>nonadult</media:rating>
    </feed>"#;

    let feed = parse(xml).unwrap();
    assert_eq!(
        feed.feed.media_rating.as_ref().map(|r| r.content.as_str()),
        Some("nonadult")
    );
}

#[test]
fn test_atom_feed_media_thumbnail() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xmlns:media="http://search.yahoo.com/mrss/">
        <title>Test Feed</title>
        <link href="http://example.com/"/>
        <id>http://example.com/</id>
        <media:thumbnail url="http://example.com/feed-thumb.jpg" width="800" height="600" time="00:00:30.000"/>
    </feed>"#;

    let feed = parse(xml).unwrap();
    assert_eq!(feed.feed.media_thumbnail.len(), 1);
    let thumb = &feed.feed.media_thumbnail[0];
    assert_eq!(&*thumb.url, "http://example.com/feed-thumb.jpg");
    assert_eq!(thumb.time.as_deref(), Some("00:00:30.000"));
}

// --- Issue #208: media:rating in entry ---

#[test]
fn test_rss_entry_media_rating() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>Test Feed</title>
            <link>http://example.com</link>
            <item>
                <title>Rated Entry</title>
                <media:rating>adult</media:rating>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert_eq!(feed.entries.len(), 1);
    assert_eq!(
        feed.entries[0]
            .media_rating
            .as_ref()
            .map(|r| r.content.as_str()),
        Some("adult")
    );
}

#[test]
fn test_atom_entry_media_rating() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xmlns:media="http://search.yahoo.com/mrss/">
        <title>Test Feed</title>
        <link href="http://example.com/"/>
        <id>http://example.com/</id>
        <entry>
            <title>Rated Entry</title>
            <link href="http://example.com/1"/>
            <id>http://example.com/1</id>
            <media:rating>adult</media:rating>
        </entry>
    </feed>"#;

    let feed = parse(xml).unwrap();
    assert_eq!(feed.entries.len(), 1);
    assert_eq!(
        feed.entries[0]
            .media_rating
            .as_ref()
            .map(|r| r.content.as_str()),
        Some("adult")
    );
}

#[test]
fn test_valid_feed_no_bozo() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>Test Feed</title>
            <link>http://example.com</link>
            <media:keywords>news, tech</media:keywords>
            <media:rating>nonadult</media:rating>
            <item>
                <title>Entry</title>
                <media:rating>nonadult</media:rating>
                <media:thumbnail url="http://example.com/thumb.jpg" time="00:01:00.000"/>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo, "Valid feed should not set bozo=true");
}
