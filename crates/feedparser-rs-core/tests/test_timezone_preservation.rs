//! Regression tests for issue #140: timezone string preservation in date fields.
//!
//! Verifies that `published_str` / `updated_str` on `Entry` and `FeedMeta` retain the
//! original timezone string from the feed, while `published` / `updated` (`DateTime<Utc>`)
//! remain correctly normalized to UTC.
#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::Timelike as _;
use feedparser_rs::parse;

fn load_fixture(path: &str) -> Vec<u8> {
    let fixture_path = format!("../../tests/fixtures/{path}");
    std::fs::read(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to load fixture '{fixture_path}': {e}"))
}

// ---------------------------------------------------------------------------
// RSS feeds
// ---------------------------------------------------------------------------

#[test]
fn test_rss_entry_published_str_preserves_plus0200() {
    let xml = load_fixture("rss/timezone-plus0200.xml");
    let feed = parse(&xml).expect("parse failed");

    let entry = &feed.entries[0];
    let published_str = entry
        .published_str
        .as_deref()
        .expect("published_str missing");
    assert!(
        published_str.contains("+0200"),
        "expected '+0200' in published_str, got: {published_str}"
    );
}

#[test]
fn test_rss_entry_published_utc_normalized_plus0200() {
    let xml = load_fixture("rss/timezone-plus0200.xml");
    let feed = parse(&xml).expect("parse failed");

    let entry = &feed.entries[0];
    let published = entry.published.expect("published DateTime missing");

    // Feed date is "Sat, 14 Dec 2024 11:00:00 +0200" → UTC is 09:00:00
    assert_eq!(published.naive_utc().hour(), 9);
    assert_eq!(published.naive_utc().minute(), 0);
}

#[test]
fn test_rss_feed_published_str_preserves_plus0200() {
    let xml = load_fixture("rss/timezone-plus0200.xml");
    let feed = parse(&xml).expect("parse failed");

    let published_str = feed
        .feed
        .published_str
        .as_deref()
        .expect("feed published_str missing");
    assert!(
        published_str.contains("+0200"),
        "expected '+0200' in feed published_str, got: {published_str}"
    );
}

#[test]
fn test_rss_entry_published_str_preserves_minus0500() {
    let xml = load_fixture("rss/timezone-minus0500.xml");
    let feed = parse(&xml).expect("parse failed");

    let entry = &feed.entries[0];
    let published_str = entry
        .published_str
        .as_deref()
        .expect("published_str missing");
    assert!(
        published_str.contains("-0500"),
        "expected '-0500' in published_str, got: {published_str}"
    );
}

#[test]
fn test_rss_entry_published_utc_normalized_minus0500() {
    let xml = load_fixture("rss/timezone-minus0500.xml");
    let feed = parse(&xml).expect("parse failed");

    let entry = &feed.entries[0];
    let published = entry.published.expect("published DateTime missing");

    // Feed date is "Sat, 14 Dec 2024 04:00:00 -0500" → UTC is 09:00:00
    assert_eq!(published.naive_utc().hour(), 9);
    assert_eq!(published.naive_utc().minute(), 0);
}

// ---------------------------------------------------------------------------
// Atom feeds
// ---------------------------------------------------------------------------

#[test]
fn test_atom_entry_updated_str_preserves_minus0800() {
    let xml = load_fixture("atom/timezone-minus0800.xml");
    let feed = parse(&xml).expect("parse failed");

    let entry = &feed.entries[0];
    let updated_str = entry.updated_str.as_deref().expect("updated_str missing");
    assert!(
        updated_str.contains("-08:00"),
        "expected '-08:00' in updated_str, got: {updated_str}"
    );
}

#[test]
fn test_atom_entry_published_str_preserves_minus0800() {
    let xml = load_fixture("atom/timezone-minus0800.xml");
    let feed = parse(&xml).expect("parse failed");

    let entry = &feed.entries[0];
    let published_str = entry
        .published_str
        .as_deref()
        .expect("published_str missing");
    assert!(
        published_str.contains("-08:00"),
        "expected '-08:00' in published_str, got: {published_str}"
    );
}

#[test]
fn test_atom_entry_updated_utc_normalized_minus0800() {
    let xml = load_fixture("atom/timezone-minus0800.xml");
    let feed = parse(&xml).expect("parse failed");

    let entry = &feed.entries[0];
    let updated = entry.updated.expect("updated DateTime missing");

    // Feed date is "2024-12-14T09:00:00-08:00" → UTC is 17:00:00
    assert_eq!(updated.naive_utc().hour(), 17);
    assert_eq!(updated.naive_utc().minute(), 0);
}

#[test]
fn test_atom_feed_updated_str_preserves_minus0800() {
    let xml = load_fixture("atom/timezone-minus0800.xml");
    let feed = parse(&xml).expect("parse failed");

    let updated_str = feed
        .feed
        .updated_str
        .as_deref()
        .expect("feed updated_str missing");
    assert!(
        updated_str.contains("-08:00"),
        "expected '-08:00' in feed updated_str, got: {updated_str}"
    );
}

#[test]
fn test_atom_entry_updated_str_preserves_plus0530() {
    let xml = load_fixture("atom/timezone-plus0530.xml");
    let feed = parse(&xml).expect("parse failed");

    let entry = &feed.entries[0];
    let updated_str = entry.updated_str.as_deref().expect("updated_str missing");
    assert!(
        updated_str.contains("+05:30"),
        "expected '+05:30' in updated_str, got: {updated_str}"
    );
}

#[test]
fn test_atom_entry_published_str_preserves_plus0530() {
    let xml = load_fixture("atom/timezone-plus0530.xml");
    let feed = parse(&xml).expect("parse failed");

    let entry = &feed.entries[0];
    let published_str = entry
        .published_str
        .as_deref()
        .expect("published_str missing");
    assert!(
        published_str.contains("+05:30"),
        "expected '+05:30' in published_str, got: {published_str}"
    );
}

// ---------------------------------------------------------------------------
// UTC feeds — must still work and have correct *_str
// ---------------------------------------------------------------------------

#[test]
fn test_rss_utc_plus0000_preserved() {
    let xml = load_fixture("rss/basic.xml");
    let feed = parse(&xml).expect("parse failed");

    let entry = &feed.entries[0];
    let published_str = entry
        .published_str
        .as_deref()
        .expect("published_str missing");
    assert!(
        published_str.contains("+0000"),
        "expected '+0000' in published_str, got: {published_str}"
    );
}

#[test]
fn test_atom_utc_z_preserved() {
    let xml = load_fixture("atom/basic.xml");
    let feed = parse(&xml).expect("parse failed");

    let entry = &feed.entries[0];
    let updated_str = entry.updated_str.as_deref().expect("updated_str missing");
    // RFC3339 UTC is represented as "Z"
    assert!(
        updated_str.ends_with('Z'),
        "expected 'Z' suffix in updated_str, got: {updated_str}"
    );
}

// ---------------------------------------------------------------------------
// Absent date fields → *_str must be None
// ---------------------------------------------------------------------------

#[test]
fn test_entry_without_date_has_none_published_str() {
    // The atom/entry-subtitle.xml has no published field on entries
    let xml = load_fixture("atom/entry-subtitle.xml");
    let feed = parse(&xml).expect("parse failed");

    if let Some(entry) = feed.entries.first() {
        // If entry has no published date, published_str must also be None
        if entry.published.is_none() {
            assert!(
                entry.published_str.is_none(),
                "published_str should be None when published is None"
            );
        }
    }
}
