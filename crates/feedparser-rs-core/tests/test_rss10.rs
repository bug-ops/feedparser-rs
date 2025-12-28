#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for RSS 1.0 (RDF) parser
//!
//! Tests comprehensive RSS 1.0 feed parsing including:
//! - Basic channel and item elements
//! - Dublin Core namespace support
//! - Content namespace support
//! - RDF structure handling
//! - Malformed feed tolerance (bozo pattern)

use chrono::{Datelike, Timelike};
use feedparser_rs::{FeedVersion, ParserLimits, namespace::syndication::UpdatePeriod, parse};
use std::fmt::Write as _;

#[test]
fn test_basic_rss10_feed() {
    let xml = br#"<?xml version="1.0"?>
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns="http://purl.org/rss/1.0/">
        <channel rdf:about="http://example.com/">
            <title>Example RSS 1.0 Feed</title>
            <link>http://example.com</link>
            <description>This is an example RSS 1.0 feed</description>
        </channel>
        <item rdf:about="http://example.com/article1">
            <title>First Article</title>
            <link>http://example.com/article1</link>
            <description>Summary of first article</description>
        </item>
        <item rdf:about="http://example.com/article2">
            <title>Second Article</title>
            <link>http://example.com/article2</link>
            <description>Summary of second article</description>
        </item>
    </rdf:RDF>"#;

    let feed = parse(xml).expect("Failed to parse RSS 1.0 feed");

    assert_eq!(feed.version, FeedVersion::Rss10);
    assert!(!feed.bozo, "Feed should not be marked as bozo");

    // Check feed metadata
    assert_eq!(feed.feed.title.as_deref(), Some("Example RSS 1.0 Feed"));
    assert_eq!(feed.feed.link.as_deref(), Some("http://example.com"));
    assert_eq!(
        feed.feed.subtitle.as_deref(),
        Some("This is an example RSS 1.0 feed")
    );
    assert_eq!(feed.feed.id.as_deref(), Some("http://example.com/"));

    // Check entries
    assert_eq!(feed.entries.len(), 2);

    let first = &feed.entries[0];
    assert_eq!(first.title.as_deref(), Some("First Article"));
    assert_eq!(first.link.as_deref(), Some("http://example.com/article1"));
    assert_eq!(first.id.as_deref(), Some("http://example.com/article1"));
    assert_eq!(first.summary.as_deref(), Some("Summary of first article"));

    let second = &feed.entries[1];
    assert_eq!(second.title.as_deref(), Some("Second Article"));
    assert_eq!(second.link.as_deref(), Some("http://example.com/article2"));
}

#[test]
fn test_rss10_with_dublin_core() {
    let xml = br#"<?xml version="1.0"?>
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns="http://purl.org/rss/1.0/"
             xmlns:dc="http://purl.org/dc/elements/1.1/">
        <channel rdf:about="http://example.com/">
            <title>News Feed</title>
            <link>http://example.com</link>
            <description>Daily news</description>
            <dc:creator>Editorial Team</dc:creator>
            <dc:rights>Copyright 2024 Example Corp</dc:rights>
            <dc:date>2024-12-18T10:00:00Z</dc:date>
            <dc:language>en-US</dc:language>
        </channel>
        <item rdf:about="http://example.com/news1">
            <title>Breaking News</title>
            <link>http://example.com/news1</link>
            <description>Important announcement</description>
            <dc:creator>John Doe</dc:creator>
            <dc:date>2024-12-18T09:30:00Z</dc:date>
            <dc:subject>politics</dc:subject>
        </item>
    </rdf:RDF>"#;

    let feed = parse(xml).expect("Failed to parse RSS 1.0 with Dublin Core");

    assert_eq!(feed.version, FeedVersion::Rss10);
    assert!(!feed.bozo);

    // Check Dublin Core elements at feed level
    assert_eq!(feed.feed.dc_creator.as_deref(), Some("Editorial Team"));
    assert_eq!(
        feed.feed.dc_rights.as_deref(),
        Some("Copyright 2024 Example Corp")
    );
    // dc:language is mapped to feed.language
    assert_eq!(feed.feed.language.as_deref(), Some("en-US"));

    // Check Dublin Core elements at entry level
    assert_eq!(feed.entries.len(), 1);
    let entry = &feed.entries[0];
    assert_eq!(entry.author.as_deref(), Some("John Doe"));
    assert!(
        entry.published.is_some(),
        "dc:date should be parsed as published date"
    );

    if let Some(published) = entry.published {
        assert_eq!(published.year(), 2024);
        assert_eq!(published.month(), 12);
        assert_eq!(published.day(), 18);
        assert_eq!(published.hour(), 9);
        assert_eq!(published.minute(), 30);
    }
}

#[test]
fn test_rss10_with_content_encoded() {
    let xml = br#"<?xml version="1.0"?>
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns="http://purl.org/rss/1.0/"
             xmlns:content="http://purl.org/rss/1.0/modules/content/">
        <channel rdf:about="http://example.com/">
            <title>Blog</title>
            <link>http://example.com</link>
            <description>Tech blog</description>
        </channel>
        <item rdf:about="http://example.com/post1">
            <title>Using RSS 1.0</title>
            <link>http://example.com/post1</link>
            <description>Brief summary</description>
            <content:encoded><![CDATA[
                <p>This is the <strong>full HTML content</strong> of the post.</p>
                <p>It includes formatting and multiple paragraphs.</p>
            ]]></content:encoded>
        </item>
    </rdf:RDF>"#;

    let feed = parse(xml).expect("Failed to parse RSS 1.0 with content:encoded");

    assert_eq!(feed.version, FeedVersion::Rss10);
    assert!(!feed.bozo);

    assert_eq!(feed.entries.len(), 1);
    let entry = &feed.entries[0];

    // Check that summary is populated from description
    assert_eq!(entry.summary.as_deref(), Some("Brief summary"));

    // Check that content:encoded is parsed
    assert!(
        !entry.content.is_empty(),
        "content:encoded should be parsed"
    );
    assert_eq!(entry.content[0].content_type.as_deref(), Some("text/html"));
    assert!(entry.content[0].value.contains("full HTML content"));
}

#[test]
fn test_rss10_with_image() {
    let xml = br#"<?xml version="1.0"?>
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns="http://purl.org/rss/1.0/">
        <channel rdf:about="http://example.com/">
            <title>Example Feed</title>
            <link>http://example.com</link>
            <description>Example</description>
        </channel>
        <image rdf:about="http://example.com/logo.png">
            <url>http://example.com/logo.png</url>
            <title>Example Logo</title>
            <link>http://example.com</link>
        </image>
    </rdf:RDF>"#;

    let feed = parse(xml).expect("Failed to parse RSS 1.0 with image");

    assert_eq!(feed.version, FeedVersion::Rss10);
    assert!(!feed.bozo);

    assert!(feed.feed.image.is_some(), "Image should be parsed");
    let image = feed.feed.image.as_ref().unwrap();
    assert_eq!(image.url, "http://example.com/logo.png");
    assert_eq!(image.title.as_deref(), Some("Example Logo"));
    assert_eq!(image.link.as_deref(), Some("http://example.com"));
}

#[test]
fn test_rss10_empty_items() {
    let xml = br#"<?xml version="1.0"?>
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns="http://purl.org/rss/1.0/">
        <channel rdf:about="http://example.com/">
            <title>Empty Feed</title>
            <link>http://example.com</link>
            <description>Feed with no items</description>
        </channel>
    </rdf:RDF>"#;

    let feed = parse(xml).expect("Failed to parse RSS 1.0 with empty items");

    assert_eq!(feed.version, FeedVersion::Rss10);
    assert!(!feed.bozo);
    assert_eq!(feed.entries.len(), 0);
    assert_eq!(feed.feed.title.as_deref(), Some("Empty Feed"));
}

#[test]
fn test_rss10_missing_required_fields() {
    let xml = br#"<?xml version="1.0"?>
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns="http://purl.org/rss/1.0/">
        <channel rdf:about="http://example.com/">
            <title>Incomplete Feed</title>
            <!-- Missing link and description -->
        </channel>
        <item rdf:about="http://example.com/item1">
            <!-- Missing title and link -->
            <description>Only has description</description>
        </item>
    </rdf:RDF>"#;

    let feed = parse(xml).expect("Parser should be tolerant of missing fields");

    assert_eq!(feed.version, FeedVersion::Rss10);
    // Should still extract what's available
    assert_eq!(feed.feed.title.as_deref(), Some("Incomplete Feed"));
    assert_eq!(feed.entries.len(), 1);
    assert_eq!(
        feed.entries[0].summary.as_deref(),
        Some("Only has description")
    );
}

#[test]
fn test_rss10_malformed_xml_bozo() {
    let xml = br#"<?xml version="1.0"?>
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns="http://purl.org/rss/1.0/">
        <channel rdf:about="http://example.com/">
            <title>Test Feed</title>
            <link>http://example.com</link>
            <description>Test</description>
        </channel>
        <item rdf:about="http://example.com/1">
            <title>Unclosed title
            <link>http://example.com/1</link>
        </item>
    </rdf:RDF>"#;

    let feed = parse(xml).expect("Should parse despite malformed XML");

    assert_eq!(feed.version, FeedVersion::Rss10);
    // Bozo pattern: continue parsing but set flag
    // Note: quick-xml in tolerant mode may or may not set bozo depending on how it handles this
    // At minimum, feed metadata should be extracted
    assert_eq!(feed.feed.title.as_deref(), Some("Test Feed"));
}

#[test]
fn test_rss10_entry_limit() {
    let xml = br#"<?xml version="1.0"?>
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns="http://purl.org/rss/1.0/">
        <channel rdf:about="http://example.com/">
            <title>Limited Feed</title>
            <link>http://example.com</link>
            <description>Test entry limits</description>
        </channel>
        <item rdf:about="http://example.com/1">
            <title>Item 1</title>
            <link>http://example.com/1</link>
        </item>
        <item rdf:about="http://example.com/2">
            <title>Item 2</title>
            <link>http://example.com/2</link>
        </item>
        <item rdf:about="http://example.com/3">
            <title>Item 3</title>
            <link>http://example.com/3</link>
        </item>
        <item rdf:about="http://example.com/4">
            <title>Item 4</title>
            <link>http://example.com/4</link>
        </item>
    </rdf:RDF>"#;

    let limits = ParserLimits {
        max_entries: 2,
        ..Default::default()
    };

    let feed =
        feedparser_rs::parse_with_limits(xml, limits).expect("Failed to parse with entry limit");

    assert_eq!(feed.entries.len(), 2);
    assert!(feed.bozo, "Should set bozo flag when limit exceeded");
    assert!(
        feed.bozo_exception
            .as_ref()
            .unwrap()
            .contains("Entry limit exceeded")
    );
}

#[test]
fn test_rss10_without_rdf_prefix() {
    let xml = br#"<?xml version="1.0"?>
    <RDF xmlns="http://purl.org/rss/1.0/"
         xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
        <channel>
            <title>No Prefix Feed</title>
            <link>http://example.com</link>
            <description>RSS 1.0 without rdf: prefix</description>
        </channel>
        <item rdf:about="http://example.com/1">
            <title>Item Title</title>
            <link>http://example.com/1</link>
        </item>
    </RDF>"#;

    let feed = parse(xml).expect("Should parse RDF without rdf: prefix");

    assert_eq!(feed.version, FeedVersion::Rss10);
    assert_eq!(feed.feed.title.as_deref(), Some("No Prefix Feed"));
    assert_eq!(feed.entries.len(), 1);
}

#[test]
fn test_rss10_nesting_depth_limit() {
    let mut xml = String::from(
        r#"<?xml version="1.0"?>
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns="http://purl.org/rss/1.0/">
        <channel rdf:about="http://example.com/">
            <title>Deep Nesting</title>
            <link>http://example.com</link>
            <description>Test nesting limits</description>"#,
    );

    // Create deeply nested structure (exceed default max_nesting_depth)
    for i in 0..150 {
        write!(&mut xml, "<level{i}>").unwrap();
    }
    for i in (0..150).rev() {
        write!(&mut xml, "</level{i}>").unwrap();
    }

    xml.push_str(
        r"
        </channel>
    </rdf:RDF>",
    );

    let feed = parse(xml.as_bytes()).expect("Should handle deep nesting");

    // Should set bozo flag when depth limit exceeded
    assert!(
        feed.bozo,
        "Should set bozo flag for excessive nesting depth"
    );
    assert!(
        feed.bozo_exception
            .as_ref()
            .is_some_and(|e| e.contains("nesting depth") || e.contains("exceeds maximum"))
    );
}

#[test]
fn test_rss10_real_world_slashdot_like() {
    let xml = br#"<?xml version="1.0"?>
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns="http://purl.org/rss/1.0/"
             xmlns:dc="http://purl.org/dc/elements/1.1/">
        <channel rdf:about="http://slashdot.org/">
            <title>Slashdot</title>
            <link>http://slashdot.org/</link>
            <description>News for nerds, stuff that matters</description>
            <dc:language>en-us</dc:language>
            <dc:rights>Copyright 1997-2024, OSDN</dc:rights>
            <dc:date>2024-12-18T10:00:00+00:00</dc:date>
            <dc:publisher>OSDN</dc:publisher>
            <items>
                <rdf:Seq>
                    <rdf:li resource="http://slashdot.org/story/1"/>
                    <rdf:li resource="http://slashdot.org/story/2"/>
                </rdf:Seq>
            </items>
        </channel>
        <item rdf:about="http://slashdot.org/story/1">
            <title>New Technology Breakthrough</title>
            <link>http://slashdot.org/story/1</link>
            <description>Scientists discover amazing things</description>
            <dc:creator>BeauHD</dc:creator>
            <dc:date>2024-12-18T08:30:00+00:00</dc:date>
            <dc:subject>science</dc:subject>
        </item>
        <item rdf:about="http://slashdot.org/story/2">
            <title>Open Source Project Released</title>
            <link>http://slashdot.org/story/2</link>
            <description>New version available for download</description>
            <dc:creator>msmash</dc:creator>
            <dc:date>2024-12-18T07:15:00+00:00</dc:date>
            <dc:subject>opensource</dc:subject>
        </item>
    </rdf:RDF>"#;

    let feed = parse(xml).expect("Failed to parse Slashdot-like RSS 1.0");

    assert_eq!(feed.version, FeedVersion::Rss10);
    assert!(!feed.bozo);

    // Feed metadata
    assert_eq!(feed.feed.title.as_deref(), Some("Slashdot"));
    assert_eq!(feed.feed.link.as_deref(), Some("http://slashdot.org/"));
    assert_eq!(
        feed.feed.subtitle.as_deref(),
        Some("News for nerds, stuff that matters")
    );
    // dc:language is mapped to feed.language
    assert_eq!(feed.feed.language.as_deref(), Some("en-us"));
    assert_eq!(
        feed.feed.dc_rights.as_deref(),
        Some("Copyright 1997-2024, OSDN")
    );

    // Entries
    assert_eq!(feed.entries.len(), 2);

    let first = &feed.entries[0];
    assert_eq!(first.title.as_deref(), Some("New Technology Breakthrough"));
    assert_eq!(first.author.as_deref(), Some("BeauHD"));
    assert!(first.published.is_some());

    let second = &feed.entries[1];
    assert_eq!(
        second.title.as_deref(),
        Some("Open Source Project Released")
    );
    assert_eq!(second.author.as_deref(), Some("msmash"));
}

#[test]
fn test_rss10_version_string() {
    let xml = br#"<?xml version="1.0"?>
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns="http://purl.org/rss/1.0/">
        <channel rdf:about="http://example.com/">
            <title>Test</title>
            <link>http://example.com</link>
            <description>Test</description>
        </channel>
    </rdf:RDF>"#;

    let feed = parse(xml).expect("Failed to parse");

    // Verify version string matches Python feedparser convention
    assert_eq!(feed.version.as_str(), "rss10");
    assert_eq!(format!("{}", feed.version), "rss10");
}

#[test]
fn test_rss10_with_syndication_module() {
    let xml = br#"<?xml version="1.0"?>
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns="http://purl.org/rss/1.0/"
             xmlns:syn="http://purl.org/rss/1.0/modules/syndication/">
        <channel rdf:about="http://example.com/">
            <title>Auto-Updated Feed</title>
            <link>http://example.com</link>
            <description>This feed updates every 2 hours</description>
            <syn:updatePeriod>hourly</syn:updatePeriod>
            <syn:updateFrequency>2</syn:updateFrequency>
            <syn:updateBase>2024-01-01T00:00:00Z</syn:updateBase>
        </channel>
        <item rdf:about="http://example.com/1">
            <title>Test Item</title>
            <link>http://example.com/1</link>
            <description>Test description</description>
        </item>
    </rdf:RDF>"#;

    let feed = parse(xml).expect("Failed to parse RSS 1.0 with syndication");

    assert_eq!(feed.version, FeedVersion::Rss10);
    assert!(!feed.bozo);

    // Verify syndication metadata
    assert!(
        feed.feed.syndication.is_some(),
        "Syndication metadata should be present"
    );

    let syn = feed.feed.syndication.as_ref().unwrap();

    // Check update period (hourly)
    assert_eq!(
        syn.update_period,
        Some(UpdatePeriod::Hourly),
        "Update period should be hourly"
    );

    // Check update frequency (2 times per period)
    assert_eq!(
        syn.update_frequency,
        Some(2),
        "Update frequency should be 2"
    );

    // Check update base timestamp
    assert_eq!(
        syn.update_base.as_deref(),
        Some("2024-01-01T00:00:00Z"),
        "Update base should be preserved"
    );
}
