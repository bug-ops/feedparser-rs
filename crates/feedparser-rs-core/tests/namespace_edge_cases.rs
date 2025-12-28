#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Edge case tests for namespace parsing
//!
//! This module tests edge cases and error conditions for Dublin Core,
//! Content, and Media RSS namespace parsing.

use feedparser_rs::parse;

/// Tests namespace URI handling
///
/// Note: Current implementation is lenient and matches by prefix only,
/// not by full namespace URI. This test documents the current behavior.
#[test]
fn test_namespace_uri_matching() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:dc="http://purl.org/dc/elements/1.0/">
        <channel>
            <title>Test</title>
            <dc:creator>Test Author</dc:creator>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();

    // Current implementation: matches by prefix, not by full URI
    // So this WILL be parsed even with wrong URI version
    assert_eq!(feed.feed.dc_creator.as_deref(), Some("Test Author"));
    assert!(!feed.bozo);
}

/// Tests that empty Dublin Core elements are handled gracefully
#[test]
fn test_empty_dc_elements() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:dc="http://purl.org/dc/elements/1.1/">
        <channel>
            <title>Test</title>
            <item>
                <title>Entry</title>
                <dc:creator></dc:creator>
                <dc:date></dc:date>
                <dc:subject></dc:subject>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    let entry = &feed.entries[0];

    // Empty dc:creator becomes empty string in dc_creator field
    assert!(entry.dc_creator.is_some());
    assert!(entry.dc_creator.as_ref().unwrap().is_empty());

    // Empty dc:date should not parse
    assert!(entry.dc_date.is_none());

    // Empty dc:subject should create empty string in vec
    assert_eq!(entry.dc_subject.len(), 1);
    assert!(entry.dc_subject[0].is_empty());
}

/// Tests that invalid numeric attributes in Media RSS are handled gracefully
#[test]
fn test_media_invalid_numeric_attributes() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>Test</title>
            <item>
                <title>Entry</title>
                <media:thumbnail url="http://example.com/img.jpg"
                                 width="invalid"
                                 height="-50" />
                <media:content url="http://example.com/video.mp4"
                               fileSize="not_a_number"
                               duration="-100" />
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    let entry = &feed.entries[0];

    // Should still parse URL, but invalid attributes should be None
    assert_eq!(entry.media_thumbnails.len(), 1);
    assert!(entry.media_thumbnails[0].width.is_none());
    assert!(entry.media_thumbnails[0].height.is_none());

    assert_eq!(entry.media_content.len(), 1);
    assert!(entry.media_content[0].filesize.is_none());
}

/// Tests that Media RSS elements without required URL attribute are ignored
#[test]
fn test_media_missing_url() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>Test</title>
            <item>
                <title>Entry</title>
                <media:thumbnail width="100" height="100" />
                <media:content type="video/mp4" duration="600" />
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    let entry = &feed.entries[0];

    // Without URL, should not create thumbnail/content
    assert!(entry.media_thumbnails.is_empty());
    assert!(entry.media_content.is_empty());
}

/// Tests fallback behavior when both Dublin Core and native elements are present
#[test]
fn test_dc_fallback_behavior() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:dc="http://purl.org/dc/elements/1.1/">
        <channel>
            <title>RSS Title</title>
            <dc:creator>DC Author</dc:creator>
            <item>
                <title>Entry Title</title>
                <author>rss@example.com (RSS Author)</author>
                <dc:creator>DC Entry Author</dc:creator>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();

    // Feed level: DC creator should set author (no RSS author exists at feed level)
    assert_eq!(feed.feed.author.as_deref(), Some("DC Author"));
    assert_eq!(feed.feed.dc_creator.as_deref(), Some("DC Author"));

    // Entry level: RSS author should take precedence, but dc_creator is also stored
    let entry = &feed.entries[0];
    assert_eq!(
        entry.author.as_deref(),
        Some("rss@example.com (RSS Author)")
    );
    assert_eq!(entry.dc_creator.as_deref(), Some("DC Entry Author"));
}

/// Tests parsing multiple Media RSS thumbnails in a single entry
#[test]
fn test_multiple_media_thumbnails() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>Test</title>
            <item>
                <title>Entry</title>
                <media:thumbnail url="http://example.com/thumb1.jpg" width="100" height="100" />
                <media:thumbnail url="http://example.com/thumb2.jpg" width="200" height="200" />
                <media:thumbnail url="http://example.com/thumb3.jpg" width="300" height="300" />
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    let entry = &feed.entries[0];

    assert_eq!(entry.media_thumbnails.len(), 3);
    assert_eq!(entry.media_thumbnails[0].width, Some(100));
    assert_eq!(entry.media_thumbnails[1].width, Some(200));
    assert_eq!(entry.media_thumbnails[2].width, Some(300));
}

/// Tests parsing of Unicode/non-ASCII characters in Dublin Core elements
#[test]
fn test_dc_unicode_content() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <rss version="2.0" xmlns:dc="http://purl.org/dc/elements/1.1/">
        <channel>
            <title>Test</title>
            <item>
                <title>Entry</title>
                <dc:creator>José García 日本語 Русский</dc:creator>
                <dc:subject>Тест</dc:subject>
                <dc:subject>テスト</dc:subject>
                <dc:rights>© 2024 版权所有</dc:rights>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml.as_bytes()).unwrap();
    let entry = &feed.entries[0];

    assert_eq!(
        entry.dc_creator.as_deref(),
        Some("José García 日本語 Русский")
    );
    assert_eq!(entry.dc_subject.len(), 2);
    assert_eq!(entry.dc_rights.as_deref(), Some("© 2024 版权所有"));
}

/// Tests that both self-closing and normal closing Media RSS elements work
#[test]
fn test_self_closing_media_elements() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>Test</title>
            <item>
                <title>Entry</title>
                <media:thumbnail url="http://example.com/thumb1.jpg" width="100" />
                <media:thumbnail url="http://example.com/thumb2.jpg" width="200"></media:thumbnail>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    let entry = &feed.entries[0];

    // Both self-closing and normal closing should parse correctly
    assert_eq!(entry.media_thumbnails.len(), 2);
}

/// Tests that RSS feeds without namespace declarations still parse correctly
#[test]
fn test_rss_without_namespaces() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Basic Feed</title>
            <item>
                <title>Basic Entry</title>
                <description>Content</description>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();

    assert!(!feed.bozo);
    assert_eq!(feed.feed.title.as_deref(), Some("Basic Feed"));
    assert_eq!(feed.entries[0].title.as_deref(), Some("Basic Entry"));
}

/// Tests handling of whitespace in Dublin Core elements
#[test]
fn test_dc_elements_with_whitespace() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:dc="http://purl.org/dc/elements/1.1/">
        <channel>
            <title>Test</title>
            <item>
                <title>Entry</title>
                <dc:creator>
                    John Doe
                </dc:creator>
                <dc:subject>  Technology  </dc:subject>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    let entry = &feed.entries[0];

    // Whitespace should be trimmed by XML parser
    assert!(entry.dc_creator.is_some());
    let creator = entry.dc_creator.as_ref().unwrap();
    assert!(creator.contains("John Doe"));

    // Subject should also be trimmed
    assert_eq!(entry.dc_subject.len(), 1);
    assert_eq!(entry.dc_subject[0].trim(), "Technology");
}

/// Tests that empty content:encoded elements are handled gracefully
#[test]
fn test_empty_content_encoded() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:content="http://purl.org/rss/1.0/modules/content/">
        <channel>
            <title>Test</title>
            <item>
                <title>Entry</title>
                <content:encoded></content:encoded>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    let entry = &feed.entries[0];

    // Empty content:encoded should create empty content
    assert_eq!(entry.content.len(), 1);
    assert!(entry.content[0].value.is_empty());
}

/// Tests that invalid date formats in dc:date are handled gracefully
#[test]
fn test_invalid_dc_date() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:dc="http://purl.org/dc/elements/1.1/">
        <channel>
            <title>Test</title>
            <item>
                <title>Entry</title>
                <dc:date>not-a-valid-date</dc:date>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    let entry = &feed.entries[0];

    // Invalid date should not set dc_date or published
    assert!(entry.dc_date.is_none());
    assert!(entry.published.is_none());
}

/// Tests parsing multiple media:content elements
#[test]
fn test_multiple_media_content() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>Test</title>
            <item>
                <title>Entry</title>
                <media:content url="http://example.com/video-low.mp4" type="video/mp4" width="640" height="480" />
                <media:content url="http://example.com/video-high.mp4" type="video/mp4" width="1920" height="1080" />
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    let entry = &feed.entries[0];

    assert_eq!(entry.media_content.len(), 2);
    assert_eq!(entry.media_content[0].width, Some(640));
    assert_eq!(entry.media_content[1].width, Some(1920));
}

/// Tests that dc:contributor elements are handled separately from dc:creator
#[test]
fn test_dc_contributor_vs_creator() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:dc="http://purl.org/dc/elements/1.1/">
        <channel>
            <title>Test</title>
            <item>
                <title>Entry</title>
                <dc:creator>Primary Author</dc:creator>
                <dc:contributor>Contributor 1</dc:contributor>
                <dc:contributor>Contributor 2</dc:contributor>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    let entry = &feed.entries[0];

    // Creator should be in author
    assert_eq!(entry.author.as_deref(), Some("Primary Author"));

    // Contributors should be in contributors list
    assert_eq!(entry.contributors.len(), 2);
    assert_eq!(entry.contributors[0].name.as_deref(), Some("Contributor 1"));
    assert_eq!(entry.contributors[1].name.as_deref(), Some("Contributor 2"));
}

/// Tests large `content:encoded` to ensure no buffer issues
#[test]
fn test_large_content_encoded() {
    let large_html = "x".repeat(100_000);
    let xml = format!(
        r#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:content="http://purl.org/rss/1.0/modules/content/">
            <channel>
                <title>Test</title>
                <item>
                    <title>Entry</title>
                    <content:encoded><![CDATA[<p>{large_html}</p>]]></content:encoded>
                </item>
            </channel>
        </rss>"#
    );

    let feed = parse(xml.as_bytes()).unwrap();
    let entry = &feed.entries[0];

    assert_eq!(entry.content.len(), 1);
    assert!(entry.content[0].value.len() > 100_000);
}

/// Tests that `dc:publisher` is stored in the `dc_publisher` field
#[test]
fn test_dc_publisher_field() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:dc="http://purl.org/dc/elements/1.1/">
        <channel>
            <title>Test</title>
            <dc:publisher>Example Publisher</dc:publisher>
            <item>
                <title>Entry</title>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();

    assert_eq!(feed.feed.dc_publisher.as_deref(), Some("Example Publisher"));
    assert_eq!(feed.feed.publisher.as_deref(), Some("Example Publisher"));
}

/// Tests that media:keywords with only commas doesn't create tags
#[test]
fn test_media_keywords_only_commas() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>Test</title>
            <item>
                <title>Entry</title>
                <media:keywords>, , ,</media:keywords>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    let entry = &feed.entries[0];

    // Only commas should not create any tags
    assert!(entry.tags.is_empty());
}

/// Tests content:encoded in Atom feeds
#[test]
fn test_content_encoded_in_atom() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom"
          xmlns:content="http://purl.org/rss/1.0/modules/content/">
        <title>Test Feed</title>
        <id>http://example.com/feed</id>
        <updated>2024-01-15T10:00:00Z</updated>
        <entry>
            <title>Test Entry</title>
            <id>http://example.com/entry1</id>
            <updated>2024-01-15T10:00:00Z</updated>
            <summary>Summary text</summary>
            <content type="html">&lt;p&gt;Atom content&lt;/p&gt;</content>
            <content:encoded><![CDATA[<p>Content module content</p>]]></content:encoded>
        </entry>
    </feed>"#;

    let feed = parse(xml).unwrap();
    let entry = &feed.entries[0];

    // Should have both Atom content and content:encoded
    assert!(entry.content.len() >= 2);

    // Verify content:encoded is captured
    assert!(
        entry
            .content
            .iter()
            .any(|c| c.value.contains("Content module content"))
    );
}
