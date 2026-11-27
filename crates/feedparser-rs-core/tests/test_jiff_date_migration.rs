//! Acceptance tests for the chrono -> jiff 1.0.3 date migration (#532).
//!
//! Verifies the `parse_date` contract, `FeedMeta`/`Entry`/`Link`/`Source` `jiff::Timestamp`
//! fields normalized to UTC, preserved raw date strings, and parser edge-case regressions.

#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use feedparser_rs::{parse, util::date::parse_date};
use jiff::{Timestamp, tz::Offset};

#[derive(Clone, Copy)]
struct UtcExpect {
    year: i16,
    month: i8,
    day: i8,
    hour: i8,
    minute: i8,
    second: i8,
    nanos: i32,
}

impl UtcExpect {
    const fn ymd(year: i16, month: i8, day: i8) -> Self {
        Self {
            year,
            month,
            day,
            hour: 0,
            minute: 0,
            second: 0,
            nanos: 0,
        }
    }

    const fn ymd_hms(year: i16, month: i8, day: i8, hour: i8, minute: i8, second: i8) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            nanos: 0,
        }
    }

    const fn with_nanos(mut self, nanos: i32) -> Self {
        self.nanos = nanos;
        self
    }
}

fn assert_utc(ts: Timestamp, exp: UtcExpect) {
    let dt = Offset::UTC.to_datetime(ts);
    assert_eq!(dt.year(), exp.year, "year mismatch for {ts}");
    assert_eq!(dt.month(), exp.month, "month mismatch for {ts}");
    assert_eq!(dt.day(), exp.day, "day mismatch for {ts}");
    assert_eq!(dt.hour(), exp.hour, "hour mismatch for {ts}");
    assert_eq!(dt.minute(), exp.minute, "minute mismatch for {ts}");
    assert_eq!(dt.second(), exp.second, "second mismatch for {ts}");
    assert_eq!(dt.subsec_nanosecond(), exp.nanos, "nanos mismatch for {ts}");
}

#[test]
fn test_parse_date_rfc3339_fractional_digits_and_separators() {
    let cases = [
        (
            "2024-12-14T10:30:00Z",
            UtcExpect::ymd_hms(2024, 12, 14, 10, 30, 0),
        ),
        (
            "2024-12-14t10:30:00.5z",
            UtcExpect::ymd_hms(2024, 12, 14, 10, 30, 0).with_nanos(500_000_000),
        ),
        (
            "2024-12-14 10:30:00.123+02:00",
            UtcExpect::ymd_hms(2024, 12, 14, 8, 30, 0).with_nanos(123_000_000),
        ),
        (
            "2024-12-14T23:30:00.123456-05:00",
            UtcExpect::ymd_hms(2024, 12, 15, 4, 30, 0).with_nanos(123_456_000),
        ),
        (
            "2024-12-14T10:30:00.123456789Z",
            UtcExpect::ymd_hms(2024, 12, 14, 10, 30, 0).with_nanos(123_456_789),
        ),
    ];

    for (input, exp) in cases {
        let ts: Timestamp =
            parse_date(input).unwrap_or_else(|| panic!("expected Some(Timestamp) for {input:?}"));
        assert_utc(ts, exp);
    }
}

#[test]
fn test_parse_date_rfc3339_over_9_fractional_digits_returns_none() {
    assert_eq!(parse_date("2024-12-14T10:30:00.1234567890Z"), None);
    assert_eq!(parse_date("2024-12-14T10:30:00.1234567890+00:00"), None);
}

#[test]
fn test_parse_date_rfc2822_relaxed_weekday_and_obsolete_offsets() {
    let cases = [
        // Wrong weekday (#143): 2026-01-15 is Thu, 2026-01-01 is Thu
        (
            "Mon, 15 Jan 2026 10:30:00 +0000",
            UtcExpect::ymd_hms(2026, 1, 15, 10, 30, 0),
        ),
        (
            "Wed, 01 Jan 2026 00:00:00 +0200",
            UtcExpect::ymd_hms(2025, 12, 31, 22, 0, 0),
        ),
        // Missing weekday
        (
            "15 Jan 2026 10:30:00 +0000",
            UtcExpect::ymd_hms(2026, 1, 15, 10, 30, 0),
        ),
        // Obsolete zone names
        (
            "Sat, 14 Dec 2024 10:30:00 GMT",
            UtcExpect::ymd_hms(2024, 12, 14, 10, 30, 0),
        ),
        (
            "Sat, 14 Dec 2024 10:30:00 UT",
            UtcExpect::ymd_hms(2024, 12, 14, 10, 30, 0),
        ),
        (
            "Sat, 14 Dec 2024 10:30:00 EST",
            UtcExpect::ymd_hms(2024, 12, 14, 15, 30, 0),
        ),
        (
            "Sat, 14 Dec 2024 10:30:00 PDT",
            UtcExpect::ymd_hms(2024, 12, 14, 17, 30, 0),
        ),
    ];

    for (input, exp) in cases {
        let ts =
            parse_date(input).unwrap_or_else(|| panic!("expected Some(Timestamp) for {input:?}"));
        assert_utc(ts, exp);
    }
}

#[test]
fn test_parse_date_year_and_year_month_and_non_ascii_guard() {
    let year_only = parse_date("2024").expect("year-only should parse");
    assert_utc(year_only, UtcExpect::ymd(2024, 1, 1));

    let year_month = parse_date("2024-12").expect("year-month should parse");
    assert_utc(year_month, UtcExpect::ymd(2024, 12, 1));

    assert_eq!(parse_date("0999"), None);
    assert_eq!(parse_date("10000"), None);
    assert_eq!(parse_date("2024-13"), None);
    // 7 UTF-8 bytes where byte index 4 lies inside 'ä' (must not panic)
    assert_eq!(parse_date("abcä-1"), None);
}

#[test]
fn test_parse_date_asctime_valid_wrong_weekday_and_short_inputs() {
    // Bug #258: space-padded single-digit day
    let single_day = parse_date("Mon Jan  6 12:30:00 2025").expect("ASCTIME single-digit day");
    assert_utc(single_day, UtcExpect::ymd_hms(2025, 1, 6, 12, 30, 0));

    let double_day = parse_date("Mon Jan 16 12:30:00 2025").expect("ASCTIME double-digit day");
    assert_utc(double_day, UtcExpect::ymd_hms(2025, 1, 16, 12, 30, 0));

    // Wrong weekday in ASCTIME (2025-01-16 is Thu, not Tue) must still parse
    let wrong_weekday = parse_date("Tue Jan 16 12:30:00 2025").expect("ASCTIME wrong weekday");
    assert_utc(wrong_weekday, UtcExpect::ymd_hms(2025, 1, 16, 12, 30, 0));

    // Short / invalid inputs must return None without panicking
    assert_eq!(parse_date("abc"), None);
    assert_eq!(parse_date("Mon Jan 16 12:30:00 202"), None);
    assert_eq!(parse_date("äbc Jan 16 12:30:00 2025"), None);
}

#[test]
fn test_parse_date_all_date_formats_and_offset_preservation() {
    let cases: &[(&str, UtcExpect)] = &[
        // ISO 8601 / RFC 3339 variants
        (
            "2024-12-14T10:30:45.123+02:00",
            UtcExpect::ymd_hms(2024, 12, 14, 8, 30, 45).with_nanos(123_000_000),
        ),
        (
            "2024-12-14T10:30:45+02:00",
            UtcExpect::ymd_hms(2024, 12, 14, 8, 30, 45),
        ),
        (
            "2024-12-14T10:30:45.123Z",
            UtcExpect::ymd_hms(2024, 12, 14, 10, 30, 45).with_nanos(123_000_000),
        ),
        (
            "2024-12-14T10:30:45Z",
            UtcExpect::ymd_hms(2024, 12, 14, 10, 30, 45),
        ),
        (
            "2024-12-14T10:30:45",
            UtcExpect::ymd_hms(2024, 12, 14, 10, 30, 45),
        ),
        (
            "2024-12-14 10:30:45",
            UtcExpect::ymd_hms(2024, 12, 14, 10, 30, 45),
        ),
        ("2024-12-14", UtcExpect::ymd(2024, 12, 14)),
        // W3C Date-Time variants (including offset preservation via DATE_FORMATS)
        (
            "2024-12-14 10:30:45+02:00",
            UtcExpect::ymd_hms(2024, 12, 14, 8, 30, 45),
        ),
        (
            "2024-12-14 10:30:45+02:00:15",
            UtcExpect::ymd_hms(2024, 12, 14, 8, 30, 30),
        ),
        (
            "2024/12/14 10:30:45",
            UtcExpect::ymd_hms(2024, 12, 14, 10, 30, 45),
        ),
        ("2024/12/14", UtcExpect::ymd(2024, 12, 14)),
        // RFC 822 variants
        (
            "14 Dec 2024 10:30:45",
            UtcExpect::ymd_hms(2024, 12, 14, 10, 30, 45),
        ),
        ("14 Dec 2024", UtcExpect::ymd(2024, 12, 14)),
        (
            "14 December 2024 10:30:45",
            UtcExpect::ymd_hms(2024, 12, 14, 10, 30, 45),
        ),
        ("14 December 2024", UtcExpect::ymd(2024, 12, 14)),
        // US date formats
        (
            "December 14, 2024 10:30:45",
            UtcExpect::ymd_hms(2024, 12, 14, 10, 30, 45),
        ),
        ("December 14, 2024", UtcExpect::ymd(2024, 12, 14)),
        (
            "Dec 14, 2024 10:30:45",
            UtcExpect::ymd_hms(2024, 12, 14, 10, 30, 45),
        ),
        ("Dec 14, 2024", UtcExpect::ymd(2024, 12, 14)),
        (
            "12/14/2024 10:30:45",
            UtcExpect::ymd_hms(2024, 12, 14, 10, 30, 45),
        ),
        ("12/14/2024", UtcExpect::ymd(2024, 12, 14)),
        ("12-14-2024", UtcExpect::ymd(2024, 12, 14)),
        // EU date formats
        (
            "14.12.2024 10:30:45",
            UtcExpect::ymd_hms(2024, 12, 14, 10, 30, 45),
        ),
        ("14.12.2024", UtcExpect::ymd(2024, 12, 14)),
        (
            "14/12/2024 10:30:45",
            UtcExpect::ymd_hms(2024, 12, 14, 10, 30, 45),
        ),
        ("14/12/2024", UtcExpect::ymd(2024, 12, 14)),
        ("14-Dec-2024", UtcExpect::ymd(2024, 12, 14)),
        ("14-December-2024", UtcExpect::ymd(2024, 12, 14)),
    ];

    for &(input, exp) in cases {
        let ts =
            parse_date(input).unwrap_or_else(|| panic!("expected Some(Timestamp) for {input:?}"));
        assert_utc(ts, exp);
    }
}

#[test]
fn test_parse_date_date_formats_offset_overflow_returns_none() {
    // 9999-12-30 20:00:00 is valid in UTC, so a naive fallback to Offset::UTC would return Some.
    let utc_valid = parse_date("9999-12-30 20:00:00").expect("9999-12-30 20:00:00Z is valid");
    assert_utc(utc_valid, UtcExpect::ymd_hms(9999, 12, 30, 20, 0, 0));

    // With -05:00 offset, UTC instant is 9999-12-31 01:00:00Z > Timestamp::MAX; must return None.
    assert_eq!(parse_date("9999-12-30 20:00:00-05:00"), None);
}

#[test]
fn test_atom10_feed_entry_link_source_timestamps_and_raw_strings() {
    let xml = br#"<?xml version="1.0" encoding="utf-8"?>
    <feed xmlns="http://www.w3.org/2005/Atom"
          xmlns:dc="http://purl.org/dc/elements/1.1/"
          xmlns:thr="http://purl.org/syndication/thread/1.0">
      <title>Atom Feed</title>
      <id>urn:uuid:feed-1</id>
      <updated>2024-12-14T11:00:00+02:00</updated>
      <entry>
        <title>Entry 1</title>
        <id>urn:uuid:entry-1</id>
        <published>2024-12-14T04:00:00-05:00</published>
        <updated>2024-12-14T12:30:00.123+02:00</updated>
        <link rel="replies" href="https://example.com/replies" thr:updated="2024-12-14T15:00:00+03:00"/>
        <source>
          <title>Source Feed</title>
          <id>urn:uuid:source-1</id>
          <updated>2024-12-14T06:00:00-04:00</updated>
        </source>
      </entry>
      <entry>
        <title>Entry 2 (dc:date override)</title>
        <id>urn:uuid:entry-2</id>
        <updated>2024-12-14T12:30:00Z</updated>
        <dc:date>2024-12-14T08:00:00-03:00</dc:date>
      </entry>
    </feed>"#;

    let feed = parse(xml).expect("Atom 1.0 feed should parse");
    assert!(!feed.bozo);

    // FeedMeta.updated + updated_str
    let feed_updated: Timestamp = feed.feed.updated.expect("feed.updated missing");
    assert_utc(feed_updated, UtcExpect::ymd_hms(2024, 12, 14, 9, 0, 0));
    assert_eq!(
        feed.feed.updated_str.as_deref(),
        Some("2024-12-14T11:00:00+02:00")
    );

    // Entry 1: published, updated, expired + raw strings
    let entry1 = &feed.entries[0];
    let entry_published: Timestamp = entry1.published.expect("entry.published missing");
    assert_utc(entry_published, UtcExpect::ymd_hms(2024, 12, 14, 9, 0, 0));
    assert_eq!(
        entry1.published_str.as_deref(),
        Some("2024-12-14T04:00:00-05:00")
    );

    let entry_updated: Timestamp = entry1.updated.expect("entry.updated missing");
    assert_utc(
        entry_updated,
        UtcExpect::ymd_hms(2024, 12, 14, 10, 30, 0).with_nanos(123_000_000),
    );
    assert_eq!(
        entry1.updated_str.as_deref(),
        Some("2024-12-14T12:30:00.123+02:00")
    );

    let entry_expired: Option<Timestamp> = entry1.expired;
    assert_eq!(entry_expired, None);

    // Link.thr_updated
    let replies_link = entry1
        .links
        .iter()
        .find(|l| l.rel.as_deref() == Some("replies"))
        .expect("replies link missing");
    let thr_updated: Timestamp = replies_link.thr_updated.expect("link.thr_updated missing");
    assert_utc(thr_updated, UtcExpect::ymd_hms(2024, 12, 14, 12, 0, 0));

    // Source / SourceFields updated + updated_str
    let source = entry1.source.as_ref().expect("entry.source missing");
    let source_updated: Timestamp = source.updated.expect("source.updated missing");
    assert_utc(source_updated, UtcExpect::ymd_hms(2024, 12, 14, 10, 0, 0));
    assert_eq!(
        source.updated_str.as_deref(),
        Some("2024-12-14T06:00:00-04:00")
    );

    // Entry 2: dc:date populates dc_date and intentionally overrides updated/updated_str
    let entry2 = &feed.entries[1];
    let entry_dc_date: Timestamp = entry2.dc_date.expect("entry.dc_date missing");
    assert_utc(entry_dc_date, UtcExpect::ymd_hms(2024, 12, 14, 11, 0, 0));
    assert_eq!(entry2.updated, Some(entry_dc_date));
    assert_eq!(
        entry2.updated_str.as_deref(),
        Some("2024-12-14T08:00:00-03:00")
    );
}

#[test]
fn test_atom03_created_and_rss20_published_timestamps() {
    let atom03 = br#"<?xml version="1.0" encoding="utf-8"?>
    <feed version="0.3" xmlns="http://purl.org/atom/ns#">
      <title>Atom 0.3 Feed</title>
      <entry>
        <title>Entry 1</title>
        <id>tag:example.com,2024:1</id>
        <created>2024-11-30T02:00:00+02:00</created>
      </entry>
    </feed>"#;

    let atom_feed = parse(atom03).expect("Atom 0.3 feed should parse");
    let entry = &atom_feed.entries[0];
    let created: Timestamp = entry.created.expect("entry.created missing");
    assert_utc(created, UtcExpect::ymd(2024, 11, 30));
    assert_eq!(
        entry.created_str.as_deref(),
        Some("2024-11-30T02:00:00+02:00")
    );

    let rss20 = br#"<?xml version="1.0" encoding="utf-8"?>
    <rss version="2.0">
      <channel>
        <title>RSS Feed</title>
        <link>https://example.com</link>
        <description>Desc</description>
        <pubDate>Sat, 14 Dec 2024 11:00:00 +0200</pubDate>
        <item>
          <title>Item 1</title>
          <pubDate>Sat, 14 Dec 2024 04:00:00 -0500</pubDate>
        </item>
      </channel>
    </rss>"#;

    let rss_feed = parse(rss20).expect("RSS 2.0 feed should parse");
    let feed_pub: Timestamp = rss_feed.feed.published.expect("rss feed.published missing");
    assert_utc(feed_pub, UtcExpect::ymd_hms(2024, 12, 14, 9, 0, 0));
    assert_eq!(
        rss_feed.feed.published_str.as_deref(),
        Some("Sat, 14 Dec 2024 11:00:00 +0200")
    );

    let item_pub: Timestamp = rss_feed.entries[0]
        .published
        .expect("rss entry.published missing");
    assert_utc(item_pub, UtcExpect::ymd_hms(2024, 12, 14, 9, 0, 0));
    assert_eq!(
        rss_feed.entries[0].published_str.as_deref(),
        Some("Sat, 14 Dec 2024 04:00:00 -0500")
    );
}
