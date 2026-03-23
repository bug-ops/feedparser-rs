//! Regression tests for GUID XML entity decoding (REG-002).
//!
//! Covers entity references in `<guid>` elements: standard named entities,
//! numeric character references (decimal and hex), unknown/malformed entities.
//! Mirrors the Python-level test suite in `feedparser-rs-py/tests/test_guid_entities.py`.

#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use feedparser_rs::parse;

fn rss_with_guid(guid: &str) -> Vec<u8> {
    format!(
        r#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>T</title>
    <item>
      <title>Post</title>
      <guid>{guid}</guid>
    </item>
  </channel>
</rss>"#
    )
    .into_bytes()
}

#[test]
fn test_guid_amp_entity_decoded() {
    let xml = rss_with_guid("https://example.com/?a=1&amp;b=2");
    let feed = parse(&xml).unwrap();
    assert_eq!(
        feed.entries[0].id.as_deref(),
        Some("https://example.com/?a=1&b=2")
    );
    assert!(!feed.bozo, "standard &amp; entity must not set bozo");
}

#[test]
fn test_guid_numeric_decimal_char_ref_decoded() {
    // &#038; is the decimal character reference for '&'
    let xml = rss_with_guid("https://sidequested.com/?post_type=webcomic1&#038;p=3172");
    let feed = parse(&xml).unwrap();
    assert_eq!(
        feed.entries[0].id.as_deref(),
        Some("https://sidequested.com/?post_type=webcomic1&p=3172")
    );
    assert!(!feed.bozo, "&#038; entity must not set bozo");
}

#[test]
fn test_guid_hex_char_ref_decoded() {
    // &#x26; is the hex character reference for '&'
    let xml = rss_with_guid("https://example.com/?a=1&#x26;b=2");
    let feed = parse(&xml).unwrap();
    assert_eq!(
        feed.entries[0].id.as_deref(),
        Some("https://example.com/?a=1&b=2")
    );
    assert!(!feed.bozo, "&#x26; entity must not set bozo");
}

#[test]
fn test_guid_multiple_amp_entities_decoded() {
    let xml = rss_with_guid("https://example.com/?a=1&amp;b=2&amp;c=3");
    let feed = parse(&xml).unwrap();
    assert_eq!(
        feed.entries[0].id.as_deref(),
        Some("https://example.com/?a=1&b=2&c=3")
    );
    assert!(!feed.bozo);
}

#[test]
fn test_guid_unknown_entity_preserved() {
    // Unknown entities are preserved verbatim (bozo pattern — no panic)
    let xml = rss_with_guid("https://example.com/?a=1&customEntity;b=2");
    let feed = parse(&xml).unwrap();
    assert_eq!(
        feed.entries[0].id.as_deref(),
        Some("https://example.com/?a=1&customEntity;b=2")
    );
}

#[test]
fn test_guid_mixed_valid_and_unknown_entities() {
    let xml = rss_with_guid("AT&amp;T&unknown;");
    let feed = parse(&xml).unwrap();
    assert_eq!(feed.entries[0].id.as_deref(), Some("AT&T&unknown;"));
}

#[test]
fn test_guid_malformed_hex_char_ref_preserved() {
    // &#x; has no hex digits — preserved verbatim
    let xml = rss_with_guid("pre&#x;suf");
    let feed = parse(&xml).unwrap();
    assert_eq!(feed.entries[0].id.as_deref(), Some("pre&#x;suf"));
}

#[test]
fn test_guid_malformed_decimal_char_ref_preserved() {
    // &#; has no digits — preserved verbatim
    let xml = rss_with_guid("pre&#;suf");
    let feed = parse(&xml).unwrap();
    assert_eq!(feed.entries[0].id.as_deref(), Some("pre&#;suf"));
}

#[test]
fn test_guid_empty_entity_name_preserved() {
    // &; has an empty name — preserved verbatim
    let xml = rss_with_guid("pre&;suf");
    let feed = parse(&xml).unwrap();
    assert_eq!(feed.entries[0].id.as_deref(), Some("pre&;suf"));
}
