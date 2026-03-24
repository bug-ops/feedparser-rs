//! Tests for issue #172: entry.author precedence and raw string format.
//!
//! 1. dc:creator must override <author> in entry.author
//! 2. <author> raw string must be preserved in entry.author (not parsed name)

#![allow(clippy::unwrap_used, clippy::expect_used)]

use feedparser_rs::parse;

#[test]
fn test_dc_creator_overrides_author() {
    let xml = include_bytes!("../../../tests/fixtures/rss/entry_author_dc_creator.xml");
    let result = parse(xml).unwrap();
    let entry = &result.entries[0];

    // dc:creator wins over <author>
    assert_eq!(entry.author.as_deref(), Some("DC Creator"));
    // author_detail should be parsed from <author>
    let detail = entry
        .author_detail
        .as_ref()
        .expect("author_detail must be set");
    assert_eq!(detail.name.as_deref(), Some("Name"));
    assert_eq!(detail.email.as_deref(), Some("email@x.com"));
    // dc_creator field
    assert_eq!(entry.dc_creator.as_deref(), Some("DC Creator"));
    // feed must not have bozo
    assert!(!result.bozo);
}

#[test]
fn test_dc_creator_overrides_author_when_dc_first() {
    // Regression test for order-dependent bug: dc:creator appears BEFORE <author>
    let xml = include_bytes!("../../../tests/fixtures/rss/entry_author_dc_creator_first.xml");
    let result = parse(xml).unwrap();
    let entry = &result.entries[0];

    // dc:creator must win even when it appears before <author>
    assert_eq!(entry.author.as_deref(), Some("DC Creator"));
    let detail = entry
        .author_detail
        .as_ref()
        .expect("author_detail must be set");
    assert_eq!(detail.name.as_deref(), Some("Name"));
    assert_eq!(detail.email.as_deref(), Some("email@x.com"));
    assert_eq!(entry.dc_creator.as_deref(), Some("DC Creator"));
    assert!(!result.bozo);
}

#[test]
fn test_author_raw_string_preserved() {
    let xml = include_bytes!("../../../tests/fixtures/rss/entry_author_raw.xml");
    let result = parse(xml).unwrap();
    let entry = &result.entries[0];

    // author flat field must be the raw string, not parsed name
    assert_eq!(entry.author.as_deref(), Some("email@x.com (Author Name)"));
    // author_detail must have parsed fields
    let detail = entry
        .author_detail
        .as_ref()
        .expect("author_detail must be set");
    assert_eq!(detail.name.as_deref(), Some("Author Name"));
    assert!(!result.bozo);
}
