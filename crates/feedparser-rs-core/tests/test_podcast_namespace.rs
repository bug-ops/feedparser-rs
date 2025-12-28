#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use feedparser_rs::{PodcastChapters, PodcastEntryMeta, PodcastSoundbite, parse};
use std::fmt::Write;

/// Helper for comparing f64 values in tests
fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < f64::EPSILON || (a == b)
}

#[test]
fn test_podcast_chapters_parsing() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
    <channel>
        <title>Test Podcast</title>
        <item>
            <title>Episode with Chapters</title>
            <podcast:chapters url="https://example.com/chapters.json" type="application/json+chapters" />
        </item>
    </channel>
</rss>"#;

    let feed = parse(xml.as_bytes()).expect("Failed to parse feed");
    assert!(!feed.bozo, "Feed should parse without errors");
    assert_eq!(feed.entries.len(), 1, "Should have one entry");

    let entry = &feed.entries[0];
    assert!(
        entry.podcast.is_some(),
        "Entry should have podcast metadata"
    );

    let podcast = entry.podcast.as_ref().unwrap();
    assert!(podcast.chapters.is_some(), "Podcast should have chapters");

    let chapters = podcast.chapters.as_ref().unwrap();
    assert_eq!(chapters.url, "https://example.com/chapters.json");
    assert_eq!(chapters.type_, "application/json+chapters");
}

#[test]
fn test_podcast_soundbite_parsing() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
    <channel>
        <title>Test Podcast</title>
        <item>
            <title>Episode with Soundbite</title>
            <podcast:soundbite startTime="120.5" duration="30.0">Great quote from the show</podcast:soundbite>
        </item>
    </channel>
</rss>"#;

    let feed = parse(xml.as_bytes()).expect("Failed to parse feed");
    assert!(!feed.bozo, "Feed should parse without errors");
    assert_eq!(feed.entries.len(), 1, "Should have one entry");

    let entry = &feed.entries[0];
    assert!(
        entry.podcast.is_some(),
        "Entry should have podcast metadata"
    );

    let podcast = entry.podcast.as_ref().unwrap();
    assert_eq!(podcast.soundbite.len(), 1, "Should have one soundbite");

    let soundbite = &podcast.soundbite[0];
    assert!(approx_eq(soundbite.start_time, 120.5));
    assert!(approx_eq(soundbite.duration, 30.0));
    assert_eq!(
        soundbite.title.as_deref(),
        Some("Great quote from the show")
    );
}

#[test]
fn test_podcast_soundbite_without_title() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
    <channel>
        <title>Test Podcast</title>
        <item>
            <title>Episode</title>
            <podcast:soundbite startTime="60" duration="15" />
        </item>
    </channel>
</rss>"#;

    let feed = parse(xml.as_bytes()).expect("Failed to parse feed");
    let entry = &feed.entries[0];
    let podcast = entry.podcast.as_ref().unwrap();
    let soundbite = &podcast.soundbite[0];

    assert!(approx_eq(soundbite.start_time, 60.0));
    assert!(approx_eq(soundbite.duration, 15.0));
    assert!(soundbite.title.is_none(), "Soundbite should have no title");
}

#[test]
fn test_multiple_soundbites() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
    <channel>
        <title>Test Podcast</title>
        <item>
            <title>Episode</title>
            <podcast:soundbite startTime="10" duration="5">First clip</podcast:soundbite>
            <podcast:soundbite startTime="100" duration="10">Second clip</podcast:soundbite>
            <podcast:soundbite startTime="200" duration="8">Third clip</podcast:soundbite>
        </item>
    </channel>
</rss>"#;

    let feed = parse(xml.as_bytes()).expect("Failed to parse feed");
    let entry = &feed.entries[0];
    let podcast = entry.podcast.as_ref().unwrap();

    assert_eq!(podcast.soundbite.len(), 3, "Should have three soundbites");
    assert_eq!(podcast.soundbite[0].title.as_deref(), Some("First clip"));
    assert_eq!(podcast.soundbite[1].title.as_deref(), Some("Second clip"));
    assert_eq!(podcast.soundbite[2].title.as_deref(), Some("Third clip"));
}

#[test]
fn test_itunes_complete_yes() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
    <channel>
        <title>Completed Podcast</title>
        <itunes:complete>Yes</itunes:complete>
    </channel>
</rss>"#;

    let feed = parse(xml.as_bytes()).expect("Failed to parse feed");
    assert!(
        feed.feed.itunes.is_some(),
        "Feed should have iTunes metadata"
    );

    let itunes = feed.feed.itunes.as_ref().unwrap();
    assert_eq!(itunes.complete, Some(true), "Podcast should be complete");
}

#[test]
fn test_itunes_complete_no() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
    <channel>
        <title>Active Podcast</title>
        <itunes:complete>No</itunes:complete>
    </channel>
</rss>"#;

    let feed = parse(xml.as_bytes()).expect("Failed to parse feed");
    let itunes = feed.feed.itunes.as_ref().unwrap();
    assert_eq!(
        itunes.complete,
        Some(false),
        "Podcast should not be complete"
    );
}

#[test]
fn test_itunes_new_feed_url() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
    <channel>
        <title>Migrated Podcast</title>
        <itunes:new-feed-url>https://example.com/new-podcast-feed.xml</itunes:new-feed-url>
    </channel>
</rss>"#;

    let feed = parse(xml.as_bytes()).expect("Failed to parse feed");
    assert!(
        feed.feed.itunes.is_some(),
        "Feed should have iTunes metadata"
    );

    let itunes = feed.feed.itunes.as_ref().unwrap();
    assert_eq!(
        itunes.new_feed_url.as_deref(),
        Some("https://example.com/new-podcast-feed.xml"),
        "Should have new feed URL"
    );
}

#[test]
fn test_malformed_soundbite_missing_duration() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
    <channel>
        <title>Test Podcast</title>
        <item>
            <title>Episode</title>
            <podcast:soundbite startTime="120">Missing duration</podcast:soundbite>
        </item>
    </channel>
</rss>"#;

    let feed = parse(xml.as_bytes()).expect("Failed to parse feed");
    let entry = &feed.entries[0];

    // Should gracefully handle missing duration without crashing
    if let Some(podcast) = &entry.podcast {
        assert_eq!(
            podcast.soundbite.len(),
            0,
            "Malformed soundbite should be skipped"
        );
    }
}

#[test]
fn test_malformed_soundbite_invalid_time() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
    <channel>
        <title>Test Podcast</title>
        <item>
            <title>Episode</title>
            <podcast:soundbite startTime="invalid" duration="30">Bad time</podcast:soundbite>
        </item>
    </channel>
</rss>"#;

    let feed = parse(xml.as_bytes()).expect("Failed to parse feed");
    // Feed should parse without error (bozo pattern)
    // Malformed soundbite should be skipped
    let entry = &feed.entries[0];
    if let Some(podcast) = &entry.podcast {
        assert_eq!(
            podcast.soundbite.len(),
            0,
            "Invalid soundbite should be skipped"
        );
    }
}

#[test]
fn test_podcast_entry_meta_default() {
    let meta = PodcastEntryMeta::default();
    assert!(meta.transcript.is_empty());
    assert!(meta.chapters.is_none());
    assert!(meta.soundbite.is_empty());
    assert!(meta.person.is_empty());
}

#[test]
fn test_podcast_chapters_default() {
    let chapters = PodcastChapters::default();
    assert!(chapters.url.is_empty());
    assert!(chapters.type_.is_empty());
}

#[test]
fn test_podcast_soundbite_default() {
    let soundbite = PodcastSoundbite::default();
    assert!(approx_eq(soundbite.start_time, 0.0));
    assert!(approx_eq(soundbite.duration, 0.0));
    assert!(soundbite.title.is_none());
}

#[test]
fn test_soundbite_limit_exceeded() {
    let mut xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
    <channel>
        <title>Test Podcast</title>
        <item>
            <title>Episode with Many Soundbites</title>"#
        .to_string();

    for i in 1..=15 {
        write!(
            xml,
            r#"
            <podcast:soundbite startTime="{}" duration="5">Clip {i}</podcast:soundbite>"#,
            i * 10
        )
        .unwrap();
    }

    xml.push_str(
        r"
        </item>
    </channel>
</rss>",
    );

    let feed = parse(xml.as_bytes()).expect("Failed to parse feed");
    assert!(!feed.bozo, "Feed should parse without errors");

    let entry = &feed.entries[0];
    let podcast = entry.podcast.as_ref().unwrap();

    assert_eq!(
        podcast.soundbite.len(),
        10,
        "Should only keep first 10 soundbites (default limit)"
    );
    assert_eq!(
        podcast.soundbite[0].title.as_deref(),
        Some("Clip 1"),
        "First soundbite should be preserved"
    );
    assert_eq!(
        podcast.soundbite[9].title.as_deref(),
        Some("Clip 10"),
        "10th soundbite should be preserved"
    );
}

#[test]
fn test_podcast_chapters_missing_url() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
    <channel>
        <title>Test Podcast</title>
        <item>
            <title>Episode</title>
            <podcast:chapters type="application/json+chapters" />
        </item>
    </channel>
</rss>"#;

    let feed = parse(xml.as_bytes()).expect("Failed to parse feed");
    assert!(!feed.bozo, "Feed should parse without errors");

    let entry = &feed.entries[0];

    if let Some(podcast) = &entry.podcast {
        assert!(
            podcast.chapters.is_none(),
            "Chapters should not be created without URL"
        );
    }
    // No podcast metadata is also acceptable - no assertion needed
}

#[test]
fn test_itunes_complete_case_insensitive() {
    let test_cases = vec![
        ("YES", Some(true)),
        ("yes", Some(true)),
        ("Yes", Some(true)),
        ("yEs", Some(true)),
        ("NO", Some(false)),
        ("no", Some(false)),
        ("No", Some(false)),
        ("nO", Some(false)),
    ];

    for (value, expected) in test_cases {
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
    <channel>
        <title>Test Podcast</title>
        <itunes:complete>{value}</itunes:complete>
    </channel>
</rss>"#
        );

        let feed = parse(xml.as_bytes()).expect("Failed to parse feed");
        let itunes = feed.feed.itunes.as_ref().unwrap();
        assert_eq!(
            itunes.complete, expected,
            "complete='{value}' should parse as {expected:?}"
        );
    }
}

#[test]
fn test_itunes_complete_whitespace() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
    <channel>
        <title>Test Podcast</title>
        <itunes:complete>  Yes  </itunes:complete>
    </channel>
</rss>"#;

    let feed = parse(xml.as_bytes()).expect("Failed to parse feed");
    let itunes = feed.feed.itunes.as_ref().unwrap();
    assert_eq!(
        itunes.complete,
        Some(true),
        "Whitespace around 'Yes' should be trimmed"
    );
}

#[test]
fn test_itunes_complete_invalid_value() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
    <channel>
        <title>Test Podcast</title>
        <itunes:complete>maybe</itunes:complete>
    </channel>
</rss>"#;

    let feed = parse(xml.as_bytes()).expect("Failed to parse feed");
    let itunes = feed.feed.itunes.as_ref().unwrap();
    assert_eq!(
        itunes.complete,
        Some(false),
        "Invalid value should be treated as false"
    );
}

#[test]
fn test_soundbite_negative_values() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
    <channel>
        <title>Test Podcast</title>
        <item>
            <title>Episode</title>
            <podcast:soundbite startTime="-10.5" duration="30">Negative start</podcast:soundbite>
            <podcast:soundbite startTime="120" duration="-5">Negative duration</podcast:soundbite>
        </item>
    </channel>
</rss>"#;

    let feed = parse(xml.as_bytes()).expect("Failed to parse feed");
    assert!(!feed.bozo, "Feed should parse without errors");

    let entry = &feed.entries[0];

    if let Some(podcast) = &entry.podcast {
        assert_eq!(
            podcast.soundbite.len(),
            2,
            "Negative values should still parse (f64 supports negatives)"
        );
        assert!(approx_eq(podcast.soundbite[0].start_time, -10.5));
        assert!(approx_eq(podcast.soundbite[1].duration, -5.0));
    }
}

#[test]
fn test_soundbite_zero_duration() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
    <channel>
        <title>Test Podcast</title>
        <item>
            <title>Episode</title>
            <podcast:soundbite startTime="0" duration="0">Zero duration</podcast:soundbite>
            <podcast:soundbite startTime="100.0" duration="0.0">Zero float</podcast:soundbite>
        </item>
    </channel>
</rss>"#;

    let feed = parse(xml.as_bytes()).expect("Failed to parse feed");
    assert!(!feed.bozo, "Feed should parse without errors");

    let entry = &feed.entries[0];
    let podcast = entry.podcast.as_ref().unwrap();

    assert_eq!(
        podcast.soundbite.len(),
        2,
        "Zero duration soundbites should be accepted"
    );
    assert!(approx_eq(podcast.soundbite[0].start_time, 0.0));
    assert!(approx_eq(podcast.soundbite[0].duration, 0.0));
    assert!(approx_eq(podcast.soundbite[1].start_time, 100.0));
    assert!(approx_eq(podcast.soundbite[1].duration, 0.0));
}

#[test]
fn test_itunes_new_feed_url_whitespace() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
    <channel>
        <title>Test Podcast</title>
        <itunes:new-feed-url>  https://example.com/new-feed.xml  </itunes:new-feed-url>
    </channel>
</rss>"#;

    let feed = parse(xml.as_bytes()).expect("Failed to parse feed");
    let itunes = feed.feed.itunes.as_ref().unwrap();
    assert_eq!(
        itunes.new_feed_url.as_deref(),
        Some("https://example.com/new-feed.xml"),
        "Whitespace should be trimmed from new-feed-url"
    );
}

#[test]
fn test_itunes_new_feed_url_empty() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
    <channel>
        <title>Test Podcast</title>
        <itunes:new-feed-url></itunes:new-feed-url>
    </channel>
</rss>"#;

    let feed = parse(xml.as_bytes()).expect("Failed to parse feed");

    if let Some(itunes) = &feed.feed.itunes {
        assert!(
            itunes.new_feed_url.is_none(),
            "Empty new-feed-url should result in None"
        );
    }
}
