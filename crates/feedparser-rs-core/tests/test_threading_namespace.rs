#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use feedparser_rs::parse;

const ATOM_THREADING_XML: &[u8] =
    include_bytes!("../../../tests/fixtures/threading/atom_threading.xml");
const RSS_THREADING_XML: &[u8] =
    include_bytes!("../../../tests/fixtures/threading/rss_threading.xml");
const MALFORMED_THREADING_XML: &[u8] =
    include_bytes!("../../../tests/fixtures/threading/malformed_threading.xml");

// ============================================================
// Atom threading tests
// ============================================================

#[test]
fn test_atom_threading_in_reply_to_full() {
    let feed = parse(ATOM_THREADING_XML).expect("Failed to parse feed");
    assert!(!feed.bozo, "Feed should parse without bozo");
    assert_eq!(feed.entries.len(), 5);

    let entry = &feed.entries[0];
    assert_eq!(entry.in_reply_to.len(), 1);
    let irt = &entry.in_reply_to[0];
    assert_eq!(irt.ref_.as_deref(), Some("tag:example.com,2024:post/1"));
    assert_eq!(irt.href.as_deref(), Some("https://example.com/post/1"));
    assert_eq!(irt.type_.as_deref(), Some("text/html"));
    assert_eq!(irt.source.as_deref(), Some("https://example.com/feed.xml"));
    assert_eq!(entry.thr_total, Some(15));
}

#[test]
fn test_atom_threading_multiple_in_reply_to() {
    let feed = parse(ATOM_THREADING_XML).expect("Failed to parse feed");
    let entry = &feed.entries[1];
    assert_eq!(entry.in_reply_to.len(), 3);
    assert_eq!(
        entry.in_reply_to[0].ref_.as_deref(),
        Some("tag:example.com,2024:post/1")
    );
    assert_eq!(
        entry.in_reply_to[1].ref_.as_deref(),
        Some("tag:example.com,2024:post/2")
    );
    assert_eq!(
        entry.in_reply_to[2].ref_.as_deref(),
        Some("tag:example.com,2024:post/3")
    );
    assert_eq!(
        entry.in_reply_to[2].href.as_deref(),
        Some("https://example.com/post/3")
    );
    assert!(entry.thr_total.is_none());
}

#[test]
fn test_atom_threading_partial_attributes() {
    let feed = parse(ATOM_THREADING_XML).expect("Failed to parse feed");
    let entry = &feed.entries[2];
    assert_eq!(entry.in_reply_to.len(), 1);
    let irt = &entry.in_reply_to[0];
    assert_eq!(irt.ref_.as_deref(), Some("tag:example.com,2024:post/1"));
    assert!(irt.href.is_none());
    assert!(irt.type_.is_none());
    assert!(irt.source.is_none());
}

#[test]
fn test_atom_threading_missing_ref_tolerated() {
    let feed = parse(ATOM_THREADING_XML).expect("Failed to parse feed");
    let entry = &feed.entries[3];
    // Entry with href and type but no ref should still be parsed
    assert_eq!(entry.in_reply_to.len(), 1);
    let irt = &entry.in_reply_to[0];
    assert!(irt.ref_.is_none());
    assert_eq!(irt.href.as_deref(), Some("https://example.com/post/1"));
    assert_eq!(irt.type_.as_deref(), Some("text/html"));
}

#[test]
fn test_atom_threading_total_only() {
    let feed = parse(ATOM_THREADING_XML).expect("Failed to parse feed");
    let entry = &feed.entries[4];
    assert!(entry.in_reply_to.is_empty());
    assert_eq!(entry.thr_total, Some(42));
}

// ============================================================
// RSS 2.0 threading tests
// ============================================================

#[test]
fn test_rss_threading_in_reply_to_full() {
    let feed = parse(RSS_THREADING_XML).expect("Failed to parse RSS feed");
    assert!(!feed.bozo, "RSS feed should parse without bozo");
    assert_eq!(feed.entries.len(), 2);

    let entry = &feed.entries[0];
    assert_eq!(entry.in_reply_to.len(), 1);
    let irt = &entry.in_reply_to[0];
    assert_eq!(irt.ref_.as_deref(), Some("tag:example.com,2024:post/1"));
    assert_eq!(irt.href.as_deref(), Some("https://example.com/post/1"));
    assert_eq!(irt.type_.as_deref(), Some("text/html"));
    assert_eq!(irt.source.as_deref(), Some("https://example.com/feed.xml"));
    assert_eq!(entry.thr_total, Some(7));
}

#[test]
fn test_rss_threading_total_only() {
    let feed = parse(RSS_THREADING_XML).expect("Failed to parse RSS feed");
    let entry = &feed.entries[1];
    assert!(entry.in_reply_to.is_empty());
    assert_eq!(entry.thr_total, Some(100));
}

// ============================================================
// Malformed input tests (no panic, no bozo from thr:)
// ============================================================

#[test]
fn test_malformed_total_non_numeric() {
    let feed = parse(MALFORMED_THREADING_XML).expect("Feed must not panic");
    let entry = &feed.entries[0];
    assert!(
        entry.thr_total.is_none(),
        "Non-numeric total should be None"
    );
}

#[test]
fn test_malformed_total_negative() {
    let feed = parse(MALFORMED_THREADING_XML).expect("Feed must not panic");
    let entry = &feed.entries[1];
    assert!(entry.thr_total.is_none(), "Negative total should be None");
}

#[test]
fn test_malformed_total_overflow() {
    let feed = parse(MALFORMED_THREADING_XML).expect("Feed must not panic");
    let entry = &feed.entries[2];
    assert!(entry.thr_total.is_none(), "Overflow total should be None");
}

#[test]
fn test_malformed_total_empty() {
    let feed = parse(MALFORMED_THREADING_XML).expect("Feed must not panic");
    let entry = &feed.entries[3];
    assert!(entry.thr_total.is_none(), "Empty total should be None");
}

#[test]
fn test_malformed_total_whitespace_only() {
    let feed = parse(MALFORMED_THREADING_XML).expect("Feed must not panic");
    let entry = &feed.entries[4];
    assert!(
        entry.thr_total.is_none(),
        "Whitespace-only total should be None"
    );
}

#[test]
fn test_valid_total_with_surrounding_whitespace() {
    let feed = parse(MALFORMED_THREADING_XML).expect("Feed must not panic");
    let entry = &feed.entries[5];
    assert_eq!(
        entry.thr_total,
        Some(42),
        "Whitespace-surrounded valid total should parse"
    );
}

#[test]
fn test_threading_empty_ref_normalized_to_none() {
    let feed = parse(MALFORMED_THREADING_XML).expect("Feed must not panic");
    let entry = &feed.entries[6];
    assert_eq!(entry.in_reply_to.len(), 1);
    let irt = &entry.in_reply_to[0];
    assert!(irt.ref_.is_none(), "Empty ref should be normalized to None");
    assert_eq!(irt.href.as_deref(), Some("https://example.com/post/1"));
}

#[test]
fn test_threading_all_empty_attrs_produces_no_entry() {
    let feed = parse(MALFORMED_THREADING_XML).expect("Feed must not panic");
    let entry = &feed.entries[7];
    // All attributes are empty after trim, so in_reply_to should be empty
    assert!(
        entry.in_reply_to.is_empty(),
        "All-empty-attribute thr:in-reply-to should produce no InReplyTo"
    );
}
