//! Integration tests for non-UTF-8 feed parsing.
//!
//! Verifies that feeds encoded in ISO-8859-1, Windows-1252, UTF-8 with BOM,
//! and UTF-16 LE are correctly decoded and their fields contain valid Unicode.
#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use feedparser_rs::parse;

fn load_fixture(path: &str) -> Vec<u8> {
    let fixture_path = format!("../../tests/fixtures/{path}");
    std::fs::read(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to load fixture '{fixture_path}': {e}"))
}

#[test]
fn test_parse_iso8859_1_feed() {
    let data = load_fixture("encoding/iso8859-1.xml");
    let feed = parse(&data).expect("ISO-8859-1 feed must parse without error");

    // Valid feed must not set bozo flag
    assert!(
        !feed.bozo,
        "bozo should not be set for valid ISO-8859-1 feed"
    );

    // encoding_rs normalises ISO-8859-1 to windows-1252 (they are aliases)
    assert_eq!(feed.encoding, "windows-1252");

    // Title must contain correctly decoded Unicode characters
    let title = feed.feed.title.as_deref().expect("title must be present");
    assert!(
        title.contains('é') && title.contains('ü') && title.contains('ñ'),
        "title '{title}' must contain Latin-1 characters decoded to Unicode"
    );
}

#[test]
fn test_parse_windows1252_feed() {
    let data = load_fixture("encoding/windows1252.xml");
    let feed = parse(&data).expect("Windows-1252 feed must parse without error");

    assert!(
        !feed.bozo,
        "bozo should not be set for valid Windows-1252 feed"
    );
    assert_eq!(feed.encoding, "windows-1252");

    // Euro sign (0x80 in Windows-1252) must decode to U+20AC
    let title = feed.feed.title.as_deref().expect("title must be present");
    assert!(
        title.contains('€'),
        "title '{title}' must contain Euro sign decoded from Windows-1252 0x80"
    );
}

#[test]
fn test_parse_utf8_bom_feed() {
    let data = load_fixture("encoding/utf8-bom.xml");

    // Confirm fixture actually starts with UTF-8 BOM
    assert_eq!(
        &data[..3],
        &[0xEF, 0xBB, 0xBF],
        "fixture must start with UTF-8 BOM"
    );

    let feed = parse(&data).expect("UTF-8 BOM feed must parse without error");

    assert!(!feed.bozo, "bozo should not be set for UTF-8 BOM feed");
    assert_eq!(feed.encoding, "utf-8");

    let title = feed.feed.title.as_deref().expect("title must be present");
    assert_eq!(title, "BOM Feed");
}

#[test]
fn test_parse_utf16le_bom_feed() {
    let data = load_fixture("encoding/utf16le-bom.xml");

    // Confirm fixture starts with UTF-16 LE BOM
    assert_eq!(
        &data[..2],
        &[0xFF, 0xFE],
        "fixture must start with UTF-16 LE BOM"
    );

    let feed = parse(&data).expect("UTF-16 LE feed must parse without error");

    assert!(
        !feed.bozo,
        "bozo should not be set for valid UTF-16 LE feed"
    );
    assert_eq!(feed.encoding, "utf-16le");

    let title = feed.feed.title.as_deref().expect("title must be present");
    assert_eq!(title, "UTF-16 Feed");
}
