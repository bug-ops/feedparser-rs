#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for namespace parsing (Dublin Core, Content, Media RSS)

use feedparser_rs::parse;

#[test]
fn test_rss_with_dublin_core() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:dc="http://purl.org/dc/elements/1.1/">
        <channel>
            <title>Test Feed</title>
            <link>http://example.com</link>
            <dc:creator>John Doe</dc:creator>
            <dc:publisher>Example Publisher</dc:publisher>
            <dc:rights>Copyright 2024</dc:rights>
            <item>
                <title>Test Entry</title>
                <dc:creator>Jane Smith</dc:creator>
                <dc:date>2024-01-15T10:30:00Z</dc:date>
                <dc:subject>Technology</dc:subject>
                <dc:subject>Rust</dc:subject>
                <dc:rights>CC BY 4.0</dc:rights>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();

    // Feed-level Dublin Core
    assert_eq!(feed.feed.dc_creator.as_deref(), Some("John Doe"));
    assert_eq!(feed.feed.dc_publisher.as_deref(), Some("Example Publisher"));
    assert_eq!(feed.feed.dc_rights.as_deref(), Some("Copyright 2024"));

    // Entry-level Dublin Core
    assert_eq!(feed.entries.len(), 1);
    let entry = &feed.entries[0];
    assert_eq!(entry.dc_creator.as_deref(), Some("Jane Smith"));
    assert!(entry.dc_date.is_some());
    assert_eq!(entry.dc_subject.len(), 2);
    assert_eq!(entry.dc_subject[0], "Technology");
    assert_eq!(entry.dc_subject[1], "Rust");
    assert_eq!(entry.dc_rights.as_deref(), Some("CC BY 4.0"));
}

#[test]
fn test_rss_with_content_encoded() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:content="http://purl.org/rss/1.0/modules/content/">
        <channel>
            <title>Test Feed</title>
            <item>
                <title>Test Entry</title>
                <description>Summary text</description>
                <content:encoded><![CDATA[<p>Full HTML content with <strong>formatting</strong>.</p>]]></content:encoded>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();

    assert_eq!(feed.entries.len(), 1);
    let entry = &feed.entries[0];

    // Summary should be from description
    assert_eq!(entry.summary.as_deref(), Some("Summary text"));

    // Content should be from content:encoded
    assert_eq!(entry.content.len(), 1);
    assert!(entry.content[0].value.contains("Full HTML content"));
    assert_eq!(entry.content[0].content_type.as_deref(), Some("text/html"));
}

#[test]
fn test_rss_with_media_rss() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>Test Feed</title>
            <item>
                <title>Video Entry</title>
                <media:title>Alternative Video Title</media:title>
                <media:description>Video description</media:description>
                <media:keywords>tech, rust, programming</media:keywords>
                <media:category>Technology</media:category>
                <media:thumbnail url="http://example.com/thumb.jpg" width="120" height="90" />
                <media:content url="http://example.com/video.mp4" type="video/mp4" fileSize="1024000" width="1920" height="1080" duration="600" />
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();

    assert_eq!(feed.entries.len(), 1);
    let entry = &feed.entries[0];

    // Media RSS text elements
    assert_eq!(entry.tags.len(), 4); // 3 keywords + 1 category

    // Media thumbnails
    assert_eq!(entry.media_thumbnail.len(), 1);
    assert_eq!(entry.media_thumbnail[0].url, "http://example.com/thumb.jpg");
    assert_eq!(entry.media_thumbnail[0].width.as_deref(), Some("120"));
    assert_eq!(entry.media_thumbnail[0].height.as_deref(), Some("90"));

    // Media content
    assert_eq!(entry.media_content.len(), 1);
    assert_eq!(entry.media_content[0].url, "http://example.com/video.mp4");
    assert_eq!(
        entry.media_content[0].content_type.as_deref(),
        Some("video/mp4")
    );
    assert_eq!(entry.media_content[0].filesize, Some(1_024_000));
    assert_eq!(entry.media_content[0].width.as_deref(), Some("1920"));
    assert_eq!(entry.media_content[0].height.as_deref(), Some("1080"));
    assert_eq!(entry.media_content[0].duration.as_deref(), Some("600"));
}

#[test]
fn test_media_rss_extended_attributes() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>Test</title>
            <link>http://example.com</link>
            <item>
                <title>Item</title>
                <media:content
                    url="http://example.com/audio.mp3"
                    type="audio/mpeg"
                    medium="audio"
                    bitrate="128"
                    channels="2"
                    samplingrate="44.1"
                    codec="mp3"
                    expression="full"
                    isDefault="true"
                    lang="en"
                />
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    let entry = &feed.entries[0];
    assert_eq!(entry.media_content.len(), 1);
    let mc = &entry.media_content[0];
    assert_eq!(mc.medium.as_deref(), Some("audio"));
    assert_eq!(mc.bitrate.as_deref(), Some("128"));
    assert_eq!(mc.channels.as_deref(), Some("2"));
    assert_eq!(mc.samplingrate.as_deref(), Some("44.1"));
    assert_eq!(mc.codec.as_deref(), Some("mp3"));
    assert_eq!(mc.expression.as_deref(), Some("full"));
    assert_eq!(mc.isdefault.as_deref(), Some("true"));
    assert_eq!(mc.lang.as_deref(), Some("en"));
    assert!(mc.duration.is_none());
    assert!(mc.width.is_none());
    assert!(mc.height.is_none());
}

#[test]
fn test_atom_with_dublin_core() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom"
          xmlns:dc="http://purl.org/dc/elements/1.1/">
        <title>Test Feed</title>
        <id>http://example.com/feed</id>
        <updated>2024-01-15T10:00:00Z</updated>
        <dc:creator>Feed Creator</dc:creator>
        <dc:rights>Feed Rights</dc:rights>
        <entry>
            <title>Test Entry</title>
            <id>http://example.com/entry1</id>
            <updated>2024-01-15T10:00:00Z</updated>
            <dc:creator>Entry Author</dc:creator>
            <dc:subject>Atom</dc:subject>
        </entry>
    </feed>"#;

    let feed = parse(xml).unwrap();

    // Feed-level DC
    assert_eq!(feed.feed.dc_creator.as_deref(), Some("Feed Creator"));
    assert_eq!(feed.feed.dc_rights.as_deref(), Some("Feed Rights"));

    // Entry-level DC
    assert_eq!(feed.entries.len(), 1);
    assert_eq!(feed.entries[0].dc_creator.as_deref(), Some("Entry Author"));
    assert_eq!(feed.entries[0].dc_subject.len(), 1);
    assert_eq!(feed.entries[0].dc_subject[0], "Atom");
}

#[test]
fn test_atom_with_content_encoded() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom"
          xmlns:content="http://purl.org/rss/1.0/modules/content/">
        <title>Test Feed</title>
        <id>http://example.com/feed</id>
        <updated>2024-01-15T10:00:00Z</updated>
        <entry>
            <title>Test Entry</title>
            <id>http://example.com/entry1</id>
            <updated>2024-01-15T10:00:00Z</updated>
            <summary>Entry summary</summary>
            <content:encoded><![CDATA[<div>Full content</div>]]></content:encoded>
        </entry>
    </feed>"#;

    let feed = parse(xml).unwrap();

    assert_eq!(feed.entries.len(), 1);
    let entry = &feed.entries[0];

    // Should have both summary and content:encoded
    assert_eq!(entry.summary.as_deref(), Some("Entry summary"));
    assert!(
        entry
            .content
            .iter()
            .any(|c| c.value.contains("Full content"))
    );
}

#[test]
fn test_atom_with_media_rss() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom"
          xmlns:media="http://search.yahoo.com/mrss/">
        <title>Test Feed</title>
        <id>http://example.com/feed</id>
        <updated>2024-01-15T10:00:00Z</updated>
        <entry>
            <title>Test Entry</title>
            <id>http://example.com/entry1</id>
            <updated>2024-01-15T10:00:00Z</updated>
            <media:thumbnail url="http://example.com/image.jpg" width="200" height="150" />
            <media:content url="http://example.com/audio.mp3" type="audio/mpeg" />
        </entry>
    </feed>"#;

    let feed = parse(xml).unwrap();

    assert_eq!(feed.entries.len(), 1);
    let entry = &feed.entries[0];

    // Media thumbnails
    assert_eq!(entry.media_thumbnail.len(), 1);
    assert_eq!(entry.media_thumbnail[0].url, "http://example.com/image.jpg");

    // Media content
    assert_eq!(entry.media_content.len(), 1);
    assert_eq!(entry.media_content[0].url, "http://example.com/audio.mp3");
    assert_eq!(
        entry.media_content[0].content_type.as_deref(),
        Some("audio/mpeg")
    );
}

#[test]
fn test_mixed_namespaces() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:dc="http://purl.org/dc/elements/1.1/"
         xmlns:content="http://purl.org/rss/1.0/modules/content/"
         xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>Test Feed</title>
            <dc:creator>Feed Author</dc:creator>
            <item>
                <title>Test Entry</title>
                <dc:creator>Entry Author</dc:creator>
                <dc:subject>Mixed</dc:subject>
                <content:encoded><![CDATA[<p>Content</p>]]></content:encoded>
                <media:thumbnail url="http://example.com/thumb.png" />
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();

    assert_eq!(feed.feed.dc_creator.as_deref(), Some("Feed Author"));

    let entry = &feed.entries[0];
    assert_eq!(entry.dc_creator.as_deref(), Some("Entry Author"));
    assert_eq!(entry.dc_subject.len(), 1);
    assert!(entry.content.iter().any(|c| c.value.contains("Content")));
    assert_eq!(entry.media_thumbnail.len(), 1);
}

#[test]
fn test_media_content_medium_attribute() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>Test Feed</title>
            <item>
                <title>Video Entry</title>
                <media:content url="http://example.com/video.mp4" type="video/mp4" medium="video" />
            </item>
            <item>
                <title>Audio Entry</title>
                <media:content url="http://example.com/audio.mp3" type="audio/mpeg" medium="audio" />
            </item>
            <item>
                <title>No Medium Entry</title>
                <media:content url="http://example.com/file.bin" type="application/octet-stream" />
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();

    assert_eq!(feed.entries.len(), 3);

    let video = &feed.entries[0];
    assert_eq!(video.media_content.len(), 1);
    assert_eq!(video.media_content[0].medium.as_deref(), Some("video"));

    let audio = &feed.entries[1];
    assert_eq!(audio.media_content.len(), 1);
    assert_eq!(audio.media_content[0].medium.as_deref(), Some("audio"));

    let no_medium = &feed.entries[2];
    assert_eq!(no_medium.media_content.len(), 1);
    assert!(no_medium.media_content[0].medium.is_none());
}

#[test]
fn test_media_group_rss() {
    // media:content inside media:group must be promoted to entry.media_content
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>T</title><link>http://example.com/</link>
            <item>
                <title>I</title><link>http://example.com/1</link>
                <media:group>
                    <media:content url="http://example.com/video-hd.mp4" medium="video" type="video/mp4"/>
                    <media:content url="http://example.com/video-sd.mp4" medium="video" type="video/mp4"/>
                    <media:thumbnail url="http://example.com/thumb.jpg" width="320" height="180"/>
                </media:group>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo, "valid feed must not set bozo");
    let entry = &feed.entries[0];
    assert_eq!(
        entry.media_content.len(),
        2,
        "both media:content inside group must be parsed"
    );
    assert_eq!(
        entry.media_content[0].url,
        "http://example.com/video-hd.mp4"
    );
    assert_eq!(
        entry.media_content[1].url,
        "http://example.com/video-sd.mp4"
    );
    assert_eq!(entry.media_thumbnail.len(), 1);
    assert_eq!(entry.media_thumbnail[0].url, "http://example.com/thumb.jpg");
}

#[test]
fn test_media_group_mixed_with_direct_content() {
    // media:content appearing both directly and inside media:group must all be collected
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>T</title><link>http://example.com/</link>
            <item>
                <title>I</title><link>http://example.com/1</link>
                <media:content url="http://example.com/direct.mp4" medium="video" type="video/mp4"/>
                <media:group>
                    <media:content url="http://example.com/grouped.mp4" medium="video" type="video/mp4"/>
                </media:group>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    assert_eq!(feed.entries[0].media_content.len(), 2);
}

#[test]
fn test_media_content_framerate_attribute() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>T</title><link>http://example.com/</link>
            <item>
                <title>I</title>
                <media:content url="http://example.com/video.mp4" type="video/mp4"
                    bitrate="1500" channels="2" samplingrate="44.1" framerate="29.97"/>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let mc = &feed.entries[0].media_content[0];
    assert_eq!(mc.bitrate.as_deref(), Some("1500"));
    assert_eq!(mc.channels.as_deref(), Some("2"));
    assert_eq!(mc.samplingrate.as_deref(), Some("44.1"));
    assert_eq!(mc.framerate.as_deref(), Some("29.97"));
}

#[test]
fn test_media_thumbnail_nested_in_media_content_rss() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>T</title><link>http://example.com/</link>
            <item>
                <title>I</title>
                <media:content url="http://example.com/video.mp4" type="video/mp4">
                    <media:thumbnail url="http://example.com/nested-thumb.jpg" width="320" height="240"/>
                </media:content>
                <media:thumbnail url="http://example.com/top-thumb.jpg" width="640" height="480"/>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let entry = &feed.entries[0];
    assert_eq!(entry.media_thumbnail.len(), 2);
    let urls: Vec<&str> = entry
        .media_thumbnail
        .iter()
        .map(|t| t.url.as_ref())
        .collect();
    assert!(urls.contains(&"http://example.com/nested-thumb.jpg"));
    assert!(urls.contains(&"http://example.com/top-thumb.jpg"));
}

#[test]
fn test_media_thumbnail_nested_in_media_content_atom() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xmlns:media="http://search.yahoo.com/mrss/">
        <title>T</title>
        <id>http://example.com/</id>
        <entry>
            <title>I</title>
            <id>http://example.com/1</id>
            <media:content url="http://example.com/video.mp4" type="video/mp4">
                <media:thumbnail url="http://example.com/nested-thumb.jpg" width="320" height="240"/>
            </media:content>
            <media:thumbnail url="http://example.com/top-thumb.jpg"/>
        </entry>
    </feed>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let entry = &feed.entries[0];
    assert_eq!(entry.media_thumbnail.len(), 2);
    let urls: Vec<&str> = entry
        .media_thumbnail
        .iter()
        .map(|t| t.url.as_ref())
        .collect();
    assert!(urls.contains(&"http://example.com/nested-thumb.jpg"));
    assert!(urls.contains(&"http://example.com/top-thumb.jpg"));
}

#[test]
fn test_rss_media_rating_feed_level() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>Rating Feed</title>
            <link>http://example.com</link>
            <media:rating scheme="urn:simple">nonadult</media:rating>
            <media:keywords>tech, programming, rust</media:keywords>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    let rating = feed.feed.media_rating.as_ref().unwrap();
    assert_eq!(rating.scheme.as_deref(), Some("urn:simple"));
    assert_eq!(rating.content, "nonadult");
    assert_eq!(
        feed.feed.media_keywords.as_deref(),
        Some("tech, programming, rust")
    );
}

#[test]
fn test_rss_media_rating_entry_level() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>Rating Feed</title>
            <link>http://example.com</link>
            <item>
                <title>Rated Entry</title>
                <media:rating scheme="urn:mpaa">pg-13</media:rating>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert_eq!(feed.entries.len(), 1);
    let rating = feed.entries[0].media_rating.as_ref().unwrap();
    assert_eq!(rating.scheme.as_deref(), Some("urn:mpaa"));
    assert_eq!(rating.content, "pg-13");
}

#[test]
fn test_atom_media_rating_feed_level() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xmlns:media="http://search.yahoo.com/mrss/">
        <title>Rating Atom Feed</title>
        <id>http://example.com/rating-feed</id>
        <updated>2024-01-01T00:00:00Z</updated>
        <media:rating scheme="urn:simple">adult</media:rating>
        <media:keywords>video, streaming</media:keywords>
    </feed>"#;

    let feed = parse(xml).unwrap();
    let rating = feed.feed.media_rating.as_ref().unwrap();
    assert_eq!(rating.scheme.as_deref(), Some("urn:simple"));
    assert_eq!(rating.content, "adult");
    assert_eq!(
        feed.feed.media_keywords.as_deref(),
        Some("video, streaming")
    );
}

#[test]
fn test_rss_media_rating_no_scheme() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
        <channel>
            <title>Feed</title>
            <link>http://example.com</link>
            <media:rating>adult</media:rating>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    let rating = feed.feed.media_rating.as_ref().unwrap();
    assert!(rating.scheme.is_none());
    assert_eq!(rating.content, "adult");
}
