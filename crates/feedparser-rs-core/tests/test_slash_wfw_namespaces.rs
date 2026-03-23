#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for Slash and WFW namespace parsing.
//!
//! Exercises the full `parse()` path end-to-end for:
//! - `slash:comments` — integer comment count
//! - `wfw:commentRss` — comment RSS feed URL
//! - Fixture-based parsing (`tests/fixtures/rss/slash_wfw.xml`)
//! - Bozo pattern: invalid values are silently ignored

use feedparser_rs::parse;

#[test]
fn test_slash_comments_in_rss_entry() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:slash="http://purl.org/rss/1.0/modules/slash/">
        <channel>
            <title>Test</title>
            <link>http://example.com</link>
            <item>
                <title>Post</title>
                <slash:comments>42</slash:comments>
            </item>
        </channel>
    </rss>"#;
    let result = parse(xml).unwrap();
    let entry = &result.entries[0];
    assert_eq!(entry.slash_comments, Some(42));
}

#[test]
fn test_wfw_comment_rss_in_rss_entry() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:wfw="http://wellformedweb.org/CommentAPI/">
        <channel>
            <title>Test</title>
            <link>http://example.com</link>
            <item>
                <title>Post</title>
                <wfw:commentRss>https://example.com/post/comments/feed/</wfw:commentRss>
            </item>
        </channel>
    </rss>"#;
    let result = parse(xml).unwrap();
    let entry = &result.entries[0];
    assert_eq!(
        entry.wfw_comment_rss.as_deref(),
        Some("https://example.com/post/comments/feed/")
    );
}

#[test]
fn test_slash_and_wfw_together() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:slash="http://purl.org/rss/1.0/modules/slash/"
         xmlns:wfw="http://wellformedweb.org/CommentAPI/">
        <channel>
            <title>Blog</title>
            <link>https://example.com</link>
            <item>
                <title>My Post</title>
                <link>https://example.com/my-post</link>
                <slash:comments>42</slash:comments>
                <wfw:commentRss>https://example.com/my-post/comments/feed/</wfw:commentRss>
            </item>
        </channel>
    </rss>"#;
    let result = parse(xml).unwrap();
    assert!(!result.bozo, "valid feed must not set bozo");
    let entry = &result.entries[0];
    assert_eq!(entry.slash_comments, Some(42));
    assert_eq!(
        entry.wfw_comment_rss.as_deref(),
        Some("https://example.com/my-post/comments/feed/")
    );
}

#[test]
fn test_slash_zero_comments() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:slash="http://purl.org/rss/1.0/modules/slash/">
        <channel>
            <title>Test</title>
            <link>http://example.com</link>
            <item>
                <title>Post</title>
                <slash:comments>0</slash:comments>
            </item>
        </channel>
    </rss>"#;
    let result = parse(xml).unwrap();
    assert_eq!(result.entries[0].slash_comments, Some(0));
}

#[test]
fn test_slash_invalid_comment_count_ignored() {
    // Bozo pattern: non-integer value must be silently ignored, bozo stays false
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:slash="http://purl.org/rss/1.0/modules/slash/">
        <channel>
            <title>Test</title>
            <link>http://example.com</link>
            <item>
                <title>Post</title>
                <slash:comments>not-a-number</slash:comments>
            </item>
        </channel>
    </rss>"#;
    let result = parse(xml).unwrap();
    assert_eq!(result.entries[0].slash_comments, None);
}

#[test]
fn test_entry_without_slash_wfw_has_none() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Test</title>
            <link>http://example.com</link>
            <item>
                <title>Post Without Comment Fields</title>
            </item>
        </channel>
    </rss>"#;
    let result = parse(xml).unwrap();
    let entry = &result.entries[0];
    assert_eq!(entry.slash_comments, None);
    assert_eq!(entry.wfw_comment_rss, None);
}

#[test]
fn test_slash_wfw_fixture() {
    let xml =
        std::fs::read("../../tests/fixtures/rss/slash_wfw.xml").expect("fixture file must exist");
    let result = parse(&xml).unwrap();
    assert!(!result.bozo, "valid feed must not set bozo");
    assert_eq!(result.entries.len(), 3);

    // Entry 0: 42 comments
    assert_eq!(result.entries[0].slash_comments, Some(42));
    assert_eq!(
        result.entries[0].wfw_comment_rss.as_deref(),
        Some("https://example.com/my-post/comments/feed/")
    );

    // Entry 1: 0 comments
    assert_eq!(result.entries[1].slash_comments, Some(0));

    // Entry 2: no comment fields
    assert_eq!(result.entries[2].slash_comments, None);
    assert_eq!(result.entries[2].wfw_comment_rss, None);
}
