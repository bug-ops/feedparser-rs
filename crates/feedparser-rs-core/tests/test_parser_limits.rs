//! Integration tests for [`ParserLimits`] enforcement
//!
//! Each test verifies that a specific limit is actually enforced when parsing
//! via `parse_with_limits()`. Tests set a low limit, provide input that exceeds
//! it, and assert the output is truncated or an error is returned.

#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use feedparser_rs::{ParserLimits, parse_with_limits};

// ── Feed-size ────────────────────────────────────────────────────────────────

#[test]
fn test_max_feed_size_bytes_returns_err() {
    let xml = br#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>Test</title>
    <link>http://example.com</link>
  </channel>
</rss>"#;

    let limits = ParserLimits {
        max_feed_size_bytes: 10, // much smaller than the feed
        ..Default::default()
    };
    let result = parse_with_limits(xml, limits);
    assert!(
        result.is_err(),
        "parse_with_limits must return Err when feed exceeds max_feed_size_bytes"
    );
}

// ── Links ────────────────────────────────────────────────────────────────────

#[test]
fn test_max_links_per_feed() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test</title>
  <link href="http://example.com/1" rel="alternate"/>
  <link href="http://example.com/2" rel="alternate"/>
  <link href="http://example.com/3" rel="alternate"/>
  <link href="http://example.com/4" rel="alternate"/>
  <link href="http://example.com/5" rel="alternate"/>
</feed>"#;

    let limits = ParserLimits {
        max_links_per_feed: 2,
        ..Default::default()
    };
    let feed = parse_with_limits(xml, limits).expect("parse failed");
    assert!(
        feed.feed.links.len() <= 2,
        "feed links must be truncated to max_links_per_feed (got {})",
        feed.feed.links.len()
    );
}

#[test]
fn test_max_links_per_entry() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test</title>
  <entry>
    <title>Entry</title>
    <link href="http://example.com/1" rel="alternate"/>
    <link href="http://example.com/2" rel="alternate"/>
    <link href="http://example.com/3" rel="alternate"/>
    <link href="http://example.com/4" rel="alternate"/>
    <id>urn:uuid:1</id>
  </entry>
</feed>"#;

    let limits = ParserLimits {
        max_links_per_entry: 1,
        ..Default::default()
    };
    let feed = parse_with_limits(xml, limits).expect("parse failed");
    assert!(!feed.entries.is_empty(), "should have one entry");
    let entry = &feed.entries[0];
    assert!(
        entry.links.len() <= 1,
        "entry links must be truncated to max_links_per_entry (got {})",
        entry.links.len()
    );
}

// ── Authors / Contributors ───────────────────────────────────────────────────

#[test]
fn test_max_authors() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test</title>
  <author><name>Alice</name></author>
  <author><name>Bob</name></author>
  <author><name>Carol</name></author>
  <author><name>Dave</name></author>
  <entry>
    <title>Entry</title>
    <id>urn:uuid:1</id>
    <author><name>Eve</name></author>
    <author><name>Frank</name></author>
    <author><name>Grace</name></author>
  </entry>
</feed>"#;

    let limits = ParserLimits {
        max_authors: 2,
        ..Default::default()
    };
    let feed = parse_with_limits(xml, limits).expect("parse failed");
    assert!(
        feed.feed.authors.len() <= 2,
        "feed authors must be truncated to max_authors (got {})",
        feed.feed.authors.len()
    );
    if !feed.entries.is_empty() {
        assert!(
            feed.entries[0].authors.len() <= 2,
            "entry authors must be truncated to max_authors (got {})",
            feed.entries[0].authors.len()
        );
    }
}

#[test]
fn test_max_contributors() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test</title>
  <contributor><name>Alice</name></contributor>
  <contributor><name>Bob</name></contributor>
  <contributor><name>Carol</name></contributor>
  <entry>
    <title>Entry</title>
    <id>urn:uuid:1</id>
    <contributor><name>Dave</name></contributor>
    <contributor><name>Eve</name></contributor>
    <contributor><name>Frank</name></contributor>
  </entry>
</feed>"#;

    let limits = ParserLimits {
        max_contributors: 1,
        ..Default::default()
    };
    let feed = parse_with_limits(xml, limits).expect("parse failed");
    assert!(
        feed.feed.contributors.len() <= 1,
        "feed contributors must be truncated to max_contributors (got {})",
        feed.feed.contributors.len()
    );
    if !feed.entries.is_empty() {
        assert!(
            feed.entries[0].contributors.len() <= 1,
            "entry contributors must be truncated to max_contributors (got {})",
            feed.entries[0].contributors.len()
        );
    }
}

// ── Tags ─────────────────────────────────────────────────────────────────────

#[test]
fn test_max_tags() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test</title>
  <entry>
    <title>Entry</title>
    <id>urn:uuid:1</id>
    <category term="tag1"/>
    <category term="tag2"/>
    <category term="tag3"/>
    <category term="tag4"/>
    <category term="tag5"/>
  </entry>
</feed>"#;

    let limits = ParserLimits {
        max_tags: 2,
        ..Default::default()
    };
    let feed = parse_with_limits(xml, limits).expect("parse failed");
    assert!(!feed.entries.is_empty(), "should have one entry");
    assert!(
        feed.entries[0].tags.len() <= 2,
        "entry tags must be truncated to max_tags (got {})",
        feed.entries[0].tags.len()
    );
}

// ── Content blocks ───────────────────────────────────────────────────────────

#[test]
fn test_max_content_blocks() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test</title>
  <entry>
    <title>Entry</title>
    <id>urn:uuid:1</id>
    <content type="text">Block one</content>
    <content type="html">&lt;p&gt;Block two&lt;/p&gt;</content>
    <content type="text">Block three</content>
  </entry>
</feed>"#;

    let limits = ParserLimits {
        max_content_blocks: 1,
        ..Default::default()
    };
    let feed = parse_with_limits(xml, limits).expect("parse failed");
    assert!(!feed.entries.is_empty(), "should have one entry");
    assert!(
        feed.entries[0].content.len() <= 1,
        "entry content blocks must be truncated to max_content_blocks (got {})",
        feed.entries[0].content.len()
    );
}

// ── Enclosures ───────────────────────────────────────────────────────────────

#[test]
fn test_max_enclosures() {
    let xml = br#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>Podcast</title>
    <item>
      <title>Episode 1</title>
      <enclosure url="http://example.com/ep1.mp3" type="audio/mpeg" length="1000"/>
      <enclosure url="http://example.com/ep2.mp3" type="audio/mpeg" length="2000"/>
      <enclosure url="http://example.com/ep3.mp3" type="audio/mpeg" length="3000"/>
    </item>
  </channel>
</rss>"#;

    let limits = ParserLimits {
        max_enclosures: 1,
        ..Default::default()
    };
    let feed = parse_with_limits(xml, limits).expect("parse failed");
    assert!(!feed.entries.is_empty(), "should have one entry");
    assert!(
        feed.entries[0].enclosures.len() <= 1,
        "entry enclosures must be truncated to max_enclosures (got {})",
        feed.entries[0].enclosures.len()
    );
}

// ── Namespaces ───────────────────────────────────────────────────────────────

#[test]
fn test_max_namespaces_is_configurable() {
    // max_namespaces is a declared limit field. Verify it can be set and the
    // feed still parses successfully (enforcement is future work).
    let xml = br#"<?xml version="1.0"?>
<rss version="2.0"
     xmlns:dc="http://purl.org/dc/elements/1.1/"
     xmlns:content="http://purl.org/rss/1.0/modules/content/">
  <channel>
    <title>Test</title>
  </channel>
</rss>"#;

    let limits = ParserLimits {
        max_namespaces: 1,
        ..Default::default()
    };
    let result = parse_with_limits(xml, limits);
    assert!(
        result.is_ok(),
        "feed with custom max_namespaces must parse without error"
    );
}

// ── Text length ───────────────────────────────────────────────────────────────

#[test]
fn test_max_text_length() {
    let long_title = "A".repeat(200);
    let xml = format!(
        r#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>{long_title}</title>
    <link>http://example.com</link>
  </channel>
</rss>"#
    );

    let limits = ParserLimits {
        max_text_length: 50,
        ..Default::default()
    };
    let result = parse_with_limits(xml.as_bytes(), limits);
    // Exceeding max_text_length sets bozo=true (via error handler) or returns Err.
    if let Ok(feed) = result {
        assert!(
            feed.bozo,
            "feed must set bozo=true when text exceeds max_text_length"
        );
    }
}

// ── Attribute length ──────────────────────────────────────────────────────────

#[test]
fn test_max_attribute_length() {
    // Use an Atom feed with a long link href attribute.
    let long_href = format!("http://example.com/{}", "x".repeat(500));
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test</title>
  <link href="{long_href}" rel="alternate"/>
</feed>"#
    );

    let limits = ParserLimits {
        max_attribute_length: 30,
        ..Default::default()
    };
    let feed = parse_with_limits(xml.as_bytes(), limits).expect("parse failed");
    // The link href should be truncated to at most max_attribute_length bytes.
    for link in &feed.feed.links {
        assert!(
            link.href.len() <= 30,
            "link href must be truncated to max_attribute_length (got {})",
            link.href.len()
        );
    }
}

// ── Podcast: soundbites ───────────────────────────────────────────────────────

#[test]
fn test_max_podcast_soundbites_configurable() {
    use std::fmt::Write as _;
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
  <channel>
    <title>Test Podcast</title>
    <item>
      <title>Episode</title>"#,
    );
    for i in 1..=8 {
        write!(
            xml,
            r#"<podcast:soundbite startTime="{}" duration="5">Clip {i}</podcast:soundbite>"#,
            i * 10
        )
        .unwrap();
    }
    xml.push_str("</item></channel></rss>");

    let limits = ParserLimits {
        max_podcast_soundbites: 3,
        ..Default::default()
    };
    let feed = parse_with_limits(xml.as_bytes(), limits).expect("parse failed");
    assert!(!feed.entries.is_empty());
    let podcast = feed.entries[0]
        .podcast
        .as_ref()
        .expect("podcast should be present");
    assert!(
        podcast.soundbite.len() <= 3,
        "soundbites must be truncated to max_podcast_soundbites (got {})",
        podcast.soundbite.len()
    );
}

// ── Podcast: transcripts ──────────────────────────────────────────────────────

#[test]
fn test_max_podcast_transcripts() {
    use std::fmt::Write as _;
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
  <channel>
    <title>Test Podcast</title>
    <item>
      <title>Episode</title>"#,
    );
    for i in 1..=6 {
        write!(
            xml,
            r#"<podcast:transcript url="http://example.com/t{i}.srt" type="application/x-subrip"/>"#
        )
        .unwrap();
    }
    xml.push_str("</item></channel></rss>");

    let limits = ParserLimits {
        max_podcast_transcripts: 2,
        ..Default::default()
    };
    let feed = parse_with_limits(xml.as_bytes(), limits).expect("parse failed");
    assert!(!feed.entries.is_empty());
    let transcripts = &feed.entries[0].podcast_transcripts;
    assert!(
        transcripts.len() <= 2,
        "transcripts must be truncated to max_podcast_transcripts (got {})",
        transcripts.len()
    );
}

// ── Podcast: funding ─────────────────────────────────────────────────────────

#[test]
fn test_max_podcast_funding() {
    use std::fmt::Write as _;
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
  <channel>
    <title>Test Podcast</title>"#,
    );
    for i in 1..=5 {
        write!(
            xml,
            r#"<podcast:funding url="http://example.com/fund{i}">Support {i}</podcast:funding>"#
        )
        .unwrap();
    }
    xml.push_str("</channel></rss>");

    let limits = ParserLimits {
        max_podcast_funding: 2,
        ..Default::default()
    };
    let feed = parse_with_limits(xml.as_bytes(), limits).expect("parse failed");
    let podcast = feed
        .feed
        .podcast
        .as_ref()
        .expect("podcast should be present");
    assert!(
        podcast.funding.len() <= 2,
        "funding elements must be truncated to max_podcast_funding (got {})",
        podcast.funding.len()
    );
}

// ── Podcast: persons ─────────────────────────────────────────────────────────

#[test]
fn test_max_podcast_persons() {
    use std::fmt::Write as _;
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
  <channel>
    <title>Test Podcast</title>
    <item>
      <title>Episode</title>"#,
    );
    for i in 1..=6 {
        write!(
            xml,
            r#"<podcast:person role="host">Person {i}</podcast:person>"#
        )
        .unwrap();
    }
    xml.push_str("</item></channel></rss>");

    let limits = ParserLimits {
        max_podcast_persons: 2,
        ..Default::default()
    };
    let feed = parse_with_limits(xml.as_bytes(), limits).expect("parse failed");
    assert!(!feed.entries.is_empty());
    let persons = &feed.entries[0].podcast_persons;
    assert!(
        persons.len() <= 2,
        "persons must be truncated to max_podcast_persons (got {})",
        persons.len()
    );
}

// ── Podcast: value recipients ─────────────────────────────────────────────────

#[test]
fn test_max_value_recipients() {
    use std::fmt::Write as _;
    let mut xml = String::from(
        r#"<?xml version="1.0"?>
<rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
  <channel>
    <title>Test Podcast</title>
    <podcast:value type="lightning" method="keysend">"#,
    );
    for i in 0..8 {
        write!(
            xml,
            r#"<podcast:valueRecipient type="node" address="addr_{i}" split="10"/>"#
        )
        .unwrap();
    }
    xml.push_str("</podcast:value></channel></rss>");

    let limits = ParserLimits {
        max_value_recipients: 3,
        ..Default::default()
    };
    let feed = parse_with_limits(xml.as_bytes(), limits).expect("parse failed");
    let podcast = feed
        .feed
        .podcast
        .as_ref()
        .expect("podcast should be present");
    let value = podcast
        .value
        .as_ref()
        .expect("value block should be present");
    assert!(
        value.recipients.len() <= 3,
        "value recipients must be truncated to max_value_recipients (got {})",
        value.recipients.len()
    );
}
