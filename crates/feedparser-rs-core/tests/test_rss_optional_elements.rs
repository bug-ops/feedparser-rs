#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for RSS optional channel/entry elements.
//!
//! Covers:
//! - #254: `feed.generator_detail` populated from `<generator>` in RSS feeds
//! - #244: `entry.slash_hit_parade` from `<slash:hit_parade>`
//! - #200: `<cloud>`, `<textInput>`, `<skipHours>`, `<skipDays>` channel elements

use feedparser_rs::parse;

// --- #254: generator_detail in RSS ---

#[test]
fn test_rss_generator_detail_name_set() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Test</title>
            <link>http://example.com</link>
            <generator>MyBlogEngine</generator>
        </channel>
    </rss>"#;
    let result = parse(xml).unwrap();
    assert!(!result.bozo, "valid feed must not set bozo");
    let generator = result
        .feed
        .generator_detail
        .as_ref()
        .expect("generator_detail must be Some");
    assert_eq!(generator.name, "MyBlogEngine");
    assert!(
        generator.href.is_none(),
        "href must be None for RSS generator"
    );
    assert!(
        generator.version.is_none(),
        "version must be None for RSS generator"
    );
}

#[test]
fn test_rss_generator_detail_absent_when_no_generator() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Test</title>
            <link>http://example.com</link>
        </channel>
    </rss>"#;
    let result = parse(xml).unwrap();
    assert!(result.feed.generator_detail.is_none());
}

// --- #244: slash:hit_parade ---

#[test]
fn test_slash_hit_parade_in_rss_entry() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:slash="http://purl.org/rss/1.0/modules/slash/">
        <channel>
            <title>Test</title>
            <link>http://example.com</link>
            <item>
                <title>Post</title>
                <slash:hit_parade>42,21,21,17,14,9,4</slash:hit_parade>
            </item>
        </channel>
    </rss>"#;
    let result = parse(xml).unwrap();
    assert!(!result.bozo, "valid feed must not set bozo");
    let entry = &result.entries[0];
    assert_eq!(
        entry.slash_hit_parade.as_deref(),
        Some("42,21,21,17,14,9,4")
    );
}

#[test]
fn test_slash_hit_parade_absent_when_not_present() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:slash="http://purl.org/rss/1.0/modules/slash/">
        <channel>
            <title>Test</title>
            <link>http://example.com</link>
            <item>
                <title>Post</title>
                <slash:comments>5</slash:comments>
            </item>
        </channel>
    </rss>"#;
    let result = parse(xml).unwrap();
    assert!(result.entries[0].slash_hit_parade.is_none());
}

// --- #200: cloud ---

#[test]
fn test_rss_cloud_all_attributes() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Test</title>
            <link>http://example.com</link>
            <cloud domain="rpc.example.com" port="80" path="/RPC2"
                   registerProcedure="pingMe" protocol="soap"/>
        </channel>
    </rss>"#;
    let result = parse(xml).unwrap();
    assert!(!result.bozo, "valid feed must not set bozo");
    let cloud = result.feed.cloud.as_ref().expect("cloud must be Some");
    assert_eq!(cloud.domain.as_deref(), Some("rpc.example.com"));
    assert_eq!(cloud.port.as_deref(), Some("80"));
    assert_eq!(cloud.path.as_deref(), Some("/RPC2"));
    assert_eq!(cloud.register_procedure.as_deref(), Some("pingMe"));
    assert_eq!(cloud.protocol.as_deref(), Some("soap"));
}

#[test]
fn test_rss_cloud_absent_when_not_present() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Test</title>
            <link>http://example.com</link>
        </channel>
    </rss>"#;
    let result = parse(xml).unwrap();
    assert!(result.feed.cloud.is_none());
}

// --- #200: textInput ---

#[test]
fn test_rss_textinput_all_fields() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Test</title>
            <link>http://example.com</link>
            <textInput>
                <title>Search</title>
                <description>Search this site</description>
                <name>q</name>
                <link>http://example.com/search</link>
            </textInput>
        </channel>
    </rss>"#;
    let result = parse(xml).unwrap();
    assert!(!result.bozo, "valid feed must not set bozo");
    let ti = result
        .feed
        .textinput
        .as_ref()
        .expect("textinput must be Some");
    assert_eq!(ti.title.as_deref(), Some("Search"));
    assert_eq!(ti.description.as_deref(), Some("Search this site"));
    assert_eq!(ti.name.as_deref(), Some("q"));
    assert_eq!(ti.link.as_deref(), Some("http://example.com/search"));
}

#[test]
fn test_rss_textinput_absent_when_not_present() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Test</title>
            <link>http://example.com</link>
        </channel>
    </rss>"#;
    let result = parse(xml).unwrap();
    assert!(result.feed.textinput.is_none());
}

// --- #200: skipHours ---

#[test]
fn test_rss_skiphours_multiple_hours() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Test</title>
            <link>http://example.com</link>
            <skipHours>
                <hour>0</hour>
                <hour>1</hour>
                <hour>23</hour>
            </skipHours>
        </channel>
    </rss>"#;
    let result = parse(xml).unwrap();
    assert!(!result.bozo, "valid feed must not set bozo");
    assert_eq!(result.feed.skiphours, vec![0u32, 1, 23]);
}

#[test]
fn test_rss_skiphours_empty_when_not_present() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Test</title>
            <link>http://example.com</link>
        </channel>
    </rss>"#;
    let result = parse(xml).unwrap();
    assert!(result.feed.skiphours.is_empty());
}

// --- #200: skipDays ---

#[test]
fn test_rss_skipdays_multiple_days() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Test</title>
            <link>http://example.com</link>
            <skipDays>
                <day>Saturday</day>
                <day>Sunday</day>
            </skipDays>
        </channel>
    </rss>"#;
    let result = parse(xml).unwrap();
    assert!(!result.bozo, "valid feed must not set bozo");
    assert_eq!(result.feed.skipdays, vec!["Saturday", "Sunday"]);
}

#[test]
fn test_rss_skipdays_empty_when_not_present() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Test</title>
            <link>http://example.com</link>
        </channel>
    </rss>"#;
    let result = parse(xml).unwrap();
    assert!(result.feed.skipdays.is_empty());
}

// --- Combined: all optional elements together ---

#[test]
fn test_rss_all_optional_channel_elements_combined() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:slash="http://purl.org/rss/1.0/modules/slash/">
        <channel>
            <title>Full Feed</title>
            <link>http://example.com</link>
            <generator>MyEngine 2.0</generator>
            <cloud domain="rpc.example.com" port="80" path="/RPC2"
                   registerProcedure="ping" protocol="xml-rpc"/>
            <textInput>
                <title>Search</title>
                <description>Find articles</description>
                <name>query</name>
                <link>http://example.com/search</link>
            </textInput>
            <skipHours>
                <hour>6</hour>
                <hour>22</hour>
            </skipHours>
            <skipDays>
                <day>Monday</day>
            </skipDays>
            <item>
                <title>Article</title>
                <slash:hit_parade>10,5,3</slash:hit_parade>
            </item>
        </channel>
    </rss>"#;
    let result = parse(xml).unwrap();
    assert!(!result.bozo, "valid feed must not set bozo");

    let generator = result.feed.generator_detail.as_ref().unwrap();
    assert_eq!(generator.name, "MyEngine 2.0");

    let cloud = result.feed.cloud.as_ref().unwrap();
    assert_eq!(cloud.domain.as_deref(), Some("rpc.example.com"));
    assert_eq!(cloud.protocol.as_deref(), Some("xml-rpc"));

    let ti = result.feed.textinput.as_ref().unwrap();
    assert_eq!(ti.title.as_deref(), Some("Search"));

    assert_eq!(result.feed.skiphours, vec![6u32, 22]);
    assert_eq!(result.feed.skipdays, vec!["Monday"]);

    assert_eq!(
        result.entries[0].slash_hit_parade.as_deref(),
        Some("10,5,3")
    );
}
