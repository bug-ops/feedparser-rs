//! Tests for GitHub issue #45: RSS 2.0 feeds with atom namespace don't parse items
//!
//! This module tests handling of self-closing (empty) XML elements in RSS feeds.
//! The root cause was that empty elements like `<atom:link ... />` were treated
//! identically to `<atom:link>...</atom:link>`, causing `skip_element()` to consume
//! subsequent events looking for a closing tag that doesn't exist.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

// =============================================================================
// Basic regression test for issue #45
// =============================================================================

#[test]
fn test_rss20_with_atom_namespace() {
    let xml = r#"<?xml version="1.0"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
    <channel>
        <title>Example Feed</title>
        <atom:link href="https://example.com/feed" rel="self" type="application/rss+xml" />
        <link>https://example.com</link>
        <item>
            <title>First Post</title>
            <link>https://example.com/post/1</link>
        </item>
    </channel>
</rss>"#;
    let feed = feedparser_rs::parse(xml.as_bytes()).unwrap();
    assert_eq!(
        feed.entries.len(),
        1,
        "Should parse the item after atom:link"
    );
    assert_eq!(feed.feed.title.as_deref(), Some("Example Feed"));
    assert_eq!(feed.entries[0].title.as_deref(), Some("First Post"));
}

// =============================================================================
// Multiple empty elements at channel level
// =============================================================================

#[test]
fn test_multiple_empty_atom_links_in_channel() {
    let xml = r#"<?xml version="1.0"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
    <channel>
        <title>Multiple Empty Elements Feed</title>
        <atom:link href="https://example.com/feed" rel="self" type="application/rss+xml" />
        <atom:link href="https://hub.example.com/" rel="hub" />
        <atom:link href="https://example.com/webmention" rel="webmention" />
        <link>https://example.com</link>
        <item>
            <title>First Post</title>
            <link>https://example.com/post/1</link>
        </item>
        <item>
            <title>Second Post</title>
            <link>https://example.com/post/2</link>
        </item>
    </channel>
</rss>"#;
    let feed = feedparser_rs::parse(xml.as_bytes()).unwrap();
    assert_eq!(
        feed.entries.len(),
        2,
        "Should parse all items after multiple atom:link elements"
    );
    assert_eq!(feed.entries[0].title.as_deref(), Some("First Post"));
    assert_eq!(feed.entries[1].title.as_deref(), Some("Second Post"));
}

#[test]
fn test_empty_elements_interleaved_with_items() {
    let xml = r#"<?xml version="1.0"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
    <channel>
        <title>Interleaved Feed</title>
        <atom:link href="https://example.com/feed" rel="self" />
        <item>
            <title>First Post</title>
        </item>
        <atom:link href="https://hub.example.com/" rel="hub" />
        <item>
            <title>Second Post</title>
        </item>
    </channel>
</rss>"#;
    let feed = feedparser_rs::parse(xml.as_bytes()).unwrap();
    assert_eq!(
        feed.entries.len(),
        2,
        "Should parse items even when interleaved with empty elements"
    );
}

// =============================================================================
// Empty elements at item level
// =============================================================================

// Fixed: Empty atom:link inside <item> elements now works correctly.
// The is_empty check is now applied at item level in parse_item() (line 771).
#[test]
fn test_empty_atom_link_in_item() {
    let xml = r#"<?xml version="1.0"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
    <channel>
        <title>Item Level Empty Elements</title>
        <item>
            <title>Post with atom:link</title>
            <atom:link href="https://example.com/comments" rel="replies" type="application/atom+xml" />
            <description>Item description after empty atom:link</description>
        </item>
        <item>
            <title>Second Post</title>
        </item>
    </channel>
</rss>"#;
    let feed = feedparser_rs::parse(xml.as_bytes()).unwrap();
    assert_eq!(feed.entries.len(), 2);
    assert!(
        feed.entries[0].summary.is_some(),
        "Should parse description after empty atom:link in item"
    );
}

// Fixed: Multiple empty namespace elements inside <item> now work correctly.
// Same fix as test_empty_atom_link_in_item.
#[test]
fn test_multiple_empty_elements_in_item() {
    let xml = r#"<?xml version="1.0"?>
<rss version="2.0"
     xmlns:atom="http://www.w3.org/2005/Atom"
     xmlns:media="http://search.yahoo.com/mrss/">
    <channel>
        <title>Multiple Empty Elements in Item</title>
        <item>
            <title>Media Post</title>
            <atom:link href="https://example.com/self" rel="self" />
            <media:thumbnail url="https://example.com/thumb.jpg" width="100" height="100" />
            <media:content url="https://example.com/video.mp4" type="video/mp4" />
            <description>Description should be parsed</description>
        </item>
    </channel>
</rss>"#;
    let feed = feedparser_rs::parse(xml.as_bytes()).unwrap();
    assert_eq!(feed.entries.len(), 1);
    assert!(feed.entries[0].summary.is_some());
    assert_eq!(feed.entries[0].media_thumbnails.len(), 1);
    assert_eq!(feed.entries[0].media_content.len(), 1);
}

// =============================================================================
// Mixed empty and non-empty namespace elements
// =============================================================================

#[test]
fn test_mixed_empty_and_nonempty_namespace_tags() {
    let xml = r#"<?xml version="1.0"?>
<rss version="2.0"
     xmlns:atom="http://www.w3.org/2005/Atom"
     xmlns:dc="http://purl.org/dc/elements/1.1/">
    <channel>
        <title>Mixed Elements Feed</title>
        <atom:link href="https://example.com/feed" rel="self" />
        <dc:creator>John Doe</dc:creator>
        <atom:link href="https://hub.example.com/" rel="hub" />
        <dc:rights>Copyright 2024</dc:rights>
        <item>
            <title>Test Post</title>
            <dc:creator>Jane Doe</dc:creator>
            <atom:link href="https://example.com/replies" rel="replies" />
        </item>
    </channel>
</rss>"#;
    let feed = feedparser_rs::parse(xml.as_bytes()).unwrap();
    assert_eq!(feed.entries.len(), 1);
    assert_eq!(feed.feed.dc_creator.as_deref(), Some("John Doe"));
    assert_eq!(feed.feed.dc_rights.as_deref(), Some("Copyright 2024"));
    assert_eq!(feed.entries[0].dc_creator.as_deref(), Some("Jane Doe"));
}

#[test]
fn test_atom_link_before_and_after_content() {
    let xml = r#"<?xml version="1.0"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
    <channel>
        <atom:link href="https://example.com/feed" rel="self" />
        <title>Title After Empty Element</title>
        <link>https://example.com</link>
        <atom:link href="https://hub.example.com/" rel="hub" />
        <description>Description after second empty element</description>
        <item>
            <title>Post</title>
        </item>
    </channel>
</rss>"#;
    let feed = feedparser_rs::parse(xml.as_bytes()).unwrap();
    assert_eq!(
        feed.feed.title.as_deref(),
        Some("Title After Empty Element")
    );
    assert_eq!(
        feed.feed.subtitle.as_deref(),
        Some("Description after second empty element")
    );
    assert_eq!(feed.entries.len(), 1);
}

// =============================================================================
// Empty iTunes/Podcast namespace elements
// =============================================================================

// Fixed: Empty itunes:image at channel level now extracts href attribute correctly.
// The itunes:image handler also sets feed.feed.image for Python feedparser compatibility.
#[test]
fn test_empty_itunes_image_in_channel() {
    let xml = r#"<?xml version="1.0"?>
<rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
    <channel>
        <title>Podcast Feed</title>
        <itunes:image href="https://example.com/artwork.jpg" />
        <item>
            <title>Episode 1</title>
            <itunes:image href="https://example.com/ep1.jpg" />
            <description>Episode description</description>
        </item>
    </channel>
</rss>"#;
    let feed = feedparser_rs::parse(xml.as_bytes()).unwrap();
    assert_eq!(feed.entries.len(), 1);
    assert_eq!(
        feed.feed.image.as_ref().map(|i| &*i.url),
        Some("https://example.com/artwork.jpg")
    );
    assert!(feed.entries[0].summary.is_some());
}

#[test]
fn test_empty_itunes_category() {
    let xml = r#"<?xml version="1.0"?>
<rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
    <channel>
        <title>Podcast Feed</title>
        <itunes:category text="Technology" />
        <itunes:category text="News">
            <itunes:category text="Tech News" />
        </itunes:category>
        <item>
            <title>Episode 1</title>
        </item>
        <item>
            <title>Episode 2</title>
        </item>
    </channel>
</rss>"#;
    let feed = feedparser_rs::parse(xml.as_bytes()).unwrap();
    assert_eq!(
        feed.entries.len(),
        2,
        "Should parse items after itunes:category elements"
    );
}

#[test]
fn test_empty_podcast_namespace_elements() {
    let xml = r#"<?xml version="1.0"?>
<rss version="2.0"
     xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd"
     xmlns:podcast="https://podcastindex.org/namespace/1.0">
    <channel>
        <title>Podcast 2.0 Feed</title>
        <podcast:locked owner="owner@example.com">no</podcast:locked>
        <podcast:funding url="https://example.com/donate">Support the show</podcast:funding>
        <item>
            <title>Episode 1</title>
            <podcast:transcript url="https://example.com/transcript.txt" type="text/plain" />
            <podcast:chapters url="https://example.com/chapters.json" type="application/json+chapters" />
            <description>Episode with podcast 2.0 elements</description>
        </item>
    </channel>
</rss>"#;
    let feed = feedparser_rs::parse(xml.as_bytes()).unwrap();
    assert_eq!(feed.entries.len(), 1);
    assert!(feed.entries[0].summary.is_some());
}

// =============================================================================
// Real-world feed patterns
// =============================================================================

// Fixed: Real-world podcast feed parsing now works correctly.
// Both itunes:image and enclosure elements are handled properly as empty elements.
#[test]
fn test_realistic_podcast_feed_with_atom_self_link() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0"
     xmlns:atom="http://www.w3.org/2005/Atom"
     xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd"
     xmlns:content="http://purl.org/rss/1.0/modules/content/">
    <channel>
        <title>Tech Podcast</title>
        <link>https://techpodcast.example.com</link>
        <description>A weekly podcast about technology</description>
        <atom:link href="https://techpodcast.example.com/feed.xml" rel="self" type="application/rss+xml" />
        <atom:link href="https://pubsubhubbub.appspot.com/" rel="hub" />
        <itunes:image href="https://techpodcast.example.com/artwork.jpg" />
        <itunes:author>Tech Team</itunes:author>
        <itunes:category text="Technology" />

        <item>
            <title>Episode 100: Milestone Episode</title>
            <link>https://techpodcast.example.com/ep100</link>
            <description>Our 100th episode celebration</description>
            <enclosure url="https://cdn.example.com/ep100.mp3" type="audio/mpeg" length="50000000" />
            <pubDate>Mon, 01 Jan 2024 10:00:00 +0000</pubDate>
            <itunes:duration>01:23:45</itunes:duration>
            <itunes:image href="https://techpodcast.example.com/ep100.jpg" />
        </item>

        <item>
            <title>Episode 101: Future of AI</title>
            <link>https://techpodcast.example.com/ep101</link>
            <description>Discussion about artificial intelligence</description>
            <content:encoded><![CDATA[<p>Full show notes with <strong>HTML</strong></p>]]></content:encoded>
            <enclosure url="https://cdn.example.com/ep101.mp3" type="audio/mpeg" length="45000000" />
            <pubDate>Mon, 08 Jan 2024 10:00:00 +0000</pubDate>
        </item>

        <item>
            <title>Episode 102: Cloud Computing</title>
            <link>https://techpodcast.example.com/ep102</link>
            <enclosure url="https://cdn.example.com/ep102.mp3" type="audio/mpeg" length="48000000" />
        </item>
    </channel>
</rss>"#;
    let feed = feedparser_rs::parse(xml.as_bytes()).unwrap();

    assert_eq!(feed.feed.title.as_deref(), Some("Tech Podcast"));
    assert_eq!(feed.entries.len(), 3, "Should parse all 3 episodes");

    assert_eq!(
        feed.entries[0].title.as_deref(),
        Some("Episode 100: Milestone Episode")
    );
    assert_eq!(feed.entries[0].enclosures.len(), 1);

    assert_eq!(
        feed.entries[1].title.as_deref(),
        Some("Episode 101: Future of AI")
    );
    assert!(
        !feed.entries[1].content.is_empty(),
        "Should have content:encoded"
    );

    assert_eq!(
        feed.entries[2].title.as_deref(),
        Some("Episode 102: Cloud Computing")
    );
}

#[test]
fn test_wordpress_style_feed_with_atom_link() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0"
     xmlns:content="http://purl.org/rss/1.0/modules/content/"
     xmlns:wfw="http://wellformedweb.org/CommentAPI/"
     xmlns:dc="http://purl.org/dc/elements/1.1/"
     xmlns:atom="http://www.w3.org/2005/Atom"
     xmlns:sy="http://purl.org/rss/1.0/modules/syndication/"
     xmlns:slash="http://purl.org/rss/1.0/modules/slash/">
    <channel>
        <title>WordPress Blog</title>
        <atom:link href="https://blog.example.com/feed/" rel="self" type="application/rss+xml" />
        <link>https://blog.example.com</link>
        <description>A WordPress blog</description>
        <lastBuildDate>Mon, 15 Jan 2024 12:00:00 +0000</lastBuildDate>
        <language>en-US</language>
        <sy:updatePeriod>hourly</sy:updatePeriod>
        <sy:updateFrequency>1</sy:updateFrequency>

        <item>
            <title>First Blog Post</title>
            <link>https://blog.example.com/first-post/</link>
            <dc:creator><![CDATA[admin]]></dc:creator>
            <pubDate>Mon, 15 Jan 2024 10:00:00 +0000</pubDate>
            <category><![CDATA[Uncategorized]]></category>
            <guid isPermaLink="false">https://blog.example.com/?p=1</guid>
            <description><![CDATA[Post summary here]]></description>
            <content:encoded><![CDATA[<p>Full post content</p>]]></content:encoded>
            <wfw:commentRss>https://blog.example.com/first-post/feed/</wfw:commentRss>
            <slash:comments>5</slash:comments>
        </item>

        <item>
            <title>Second Blog Post</title>
            <link>https://blog.example.com/second-post/</link>
            <dc:creator><![CDATA[editor]]></dc:creator>
            <pubDate>Tue, 16 Jan 2024 10:00:00 +0000</pubDate>
        </item>
    </channel>
</rss>"#;
    let feed = feedparser_rs::parse(xml.as_bytes()).unwrap();

    assert_eq!(feed.feed.title.as_deref(), Some("WordPress Blog"));
    assert_eq!(feed.entries.len(), 2);
    assert_eq!(feed.entries[0].dc_creator.as_deref(), Some("admin"));
    assert_eq!(feed.entries[1].dc_creator.as_deref(), Some("editor"));
}

// =============================================================================
// Edge cases with empty standard RSS elements (defensive tests)
// =============================================================================

#[test]
fn test_empty_standard_elements_ignored() {
    let xml = r#"<?xml version="1.0"?>
<rss version="2.0">
    <channel>
        <title>Feed with empty elements</title>
        <description />
        <link />
        <item>
            <title>Item title</title>
            <description />
            <link />
        </item>
    </channel>
</rss>"#;
    let feed = feedparser_rs::parse(xml.as_bytes()).unwrap();
    assert_eq!(feed.entries.len(), 1);
    assert_eq!(feed.entries[0].title.as_deref(), Some("Item title"));
}

// Fixed: Self-closing enclosure elements now work correctly.
// The is_empty check is now applied before calling skip_element for enclosure elements.
#[test]
fn test_self_closing_enclosure_followed_by_content() {
    let xml = r#"<?xml version="1.0"?>
<rss version="2.0">
    <channel>
        <title>Enclosure Test</title>
        <item>
            <title>Episode with enclosure</title>
            <enclosure url="https://example.com/audio.mp3" type="audio/mpeg" length="12345" />
            <description>Description after enclosure</description>
            <pubDate>Mon, 01 Jan 2024 10:00:00 +0000</pubDate>
        </item>
        <item>
            <title>Second Episode</title>
            <enclosure url="https://example.com/audio2.mp3" type="audio/mpeg" length="67890" />
        </item>
    </channel>
</rss>"#;
    let feed = feedparser_rs::parse(xml.as_bytes()).unwrap();
    assert_eq!(feed.entries.len(), 2);
    assert_eq!(feed.entries[0].enclosures.len(), 1);
    assert!(
        feed.entries[0].summary.is_some(),
        "Description after empty enclosure should be parsed"
    );
    assert!(
        feed.entries[0].published.is_some(),
        "pubDate after empty enclosure should be parsed"
    );
    assert_eq!(feed.entries[1].enclosures.len(), 1);
}

// =============================================================================
// Combination stress tests
// =============================================================================

#[test]
fn test_many_empty_elements_followed_by_many_items() {
    let xml = r#"<?xml version="1.0"?>
<rss version="2.0"
     xmlns:atom="http://www.w3.org/2005/Atom"
     xmlns:media="http://search.yahoo.com/mrss/">
    <channel>
        <title>Stress Test Feed</title>
        <atom:link href="https://example.com/feed" rel="self" />
        <atom:link href="https://hub.example.com/" rel="hub" />
        <atom:link href="https://example.com/websub" rel="websub" />
        <atom:link href="https://example.com/webmention" rel="webmention" />
        <atom:link href="https://example.com/micropub" rel="micropub" />
        <item><title>Item 1</title></item>
        <item><title>Item 2</title></item>
        <item><title>Item 3</title></item>
        <item><title>Item 4</title></item>
        <item><title>Item 5</title></item>
        <item><title>Item 6</title></item>
        <item><title>Item 7</title></item>
        <item><title>Item 8</title></item>
        <item><title>Item 9</title></item>
        <item><title>Item 10</title></item>
    </channel>
</rss>"#;
    let feed = feedparser_rs::parse(xml.as_bytes()).unwrap();
    assert_eq!(
        feed.entries.len(),
        10,
        "All 10 items should be parsed after multiple empty atom:link elements"
    );
    for (i, entry) in feed.entries.iter().enumerate() {
        assert_eq!(
            entry.title.as_deref(),
            Some(format!("Item {}", i + 1).as_str()),
            "Item {} should have correct title",
            i + 1
        );
    }
}

#[test]
fn test_alternating_empty_elements_and_items() {
    let xml = r#"<?xml version="1.0"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
    <channel>
        <title>Alternating Test</title>
        <atom:link href="https://example.com/1" rel="related" />
        <item><title>Item 1</title></item>
        <atom:link href="https://example.com/2" rel="related" />
        <item><title>Item 2</title></item>
        <atom:link href="https://example.com/3" rel="related" />
        <item><title>Item 3</title></item>
        <atom:link href="https://example.com/4" rel="related" />
        <item><title>Item 4</title></item>
        <atom:link href="https://example.com/5" rel="related" />
        <item><title>Item 5</title></item>
    </channel>
</rss>"#;
    let feed = feedparser_rs::parse(xml.as_bytes()).unwrap();
    assert_eq!(
        feed.entries.len(),
        5,
        "All items should be parsed when alternating with empty elements"
    );
}
