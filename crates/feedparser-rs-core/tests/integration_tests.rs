#![allow(
    missing_docs,
    clippy::if_then_some_else_none,
    clippy::single_match_else,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use feedparser_rs::{FeedVersion, TextType, detect_format, parse};

/// Helper function to load test fixtures
fn load_fixture(path: &str) -> Vec<u8> {
    // Fixtures are in the workspace root tests/fixtures/ directory
    let fixture_path = format!("../../tests/fixtures/{path}");
    std::fs::read(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to load fixture '{fixture_path}': {e}"))
}

/// Helper to assert basic feed validity
fn assert_feed_valid(result: &feedparser_rs::ParsedFeed) {
    assert!(result.version == FeedVersion::Unknown || !result.bozo);
}

#[test]
fn test_parse_rss_basic_fixture() {
    let xml = load_fixture("rss/basic.xml");
    let result = parse(&xml);

    assert!(result.is_ok(), "Failed to parse RSS fixture");
    let feed = result.unwrap();

    assert_feed_valid(&feed);
}

#[test]
fn test_parse_atom_basic_fixture() {
    let xml = load_fixture("atom/basic.xml");
    let result = parse(&xml);

    assert!(result.is_ok(), "Failed to parse Atom fixture");
    let feed = result.unwrap();

    assert_feed_valid(&feed);
}

#[test]
fn test_detect_format_rss() {
    let xml = load_fixture("rss/basic.xml");
    let version = detect_format(&xml);
    let _ = version;
}

#[test]
fn test_detect_format_atom() {
    let xml = load_fixture("atom/basic.xml");
    let version = detect_format(&xml);
    let _ = version;
}

#[test]
fn test_parse_empty_input() {
    let feed = parse(b"").unwrap();
    assert!(feed.bozo, "empty input must set bozo");
    assert_eq!(feed.version, FeedVersion::Unknown);
    assert!(feed.entries.is_empty());
}

#[test]
fn test_parse_whitespace_only() {
    let feed = parse(b"   \n  ").unwrap();
    assert!(feed.bozo, "whitespace-only input must set bozo");
    assert_eq!(feed.version, FeedVersion::Unknown);
    assert!(feed.entries.is_empty());
}

#[test]
fn test_parse_invalid_xml() {
    let feed = parse(b"<invalid><xml>").unwrap();
    assert!(feed.bozo, "invalid XML must set bozo");
    assert_eq!(feed.version, FeedVersion::Unknown);
}

#[test]
fn test_parse_doctype_only() {
    let feed = parse(b"<!DOCTYPE foo>").unwrap();
    assert!(feed.bozo, "DOCTYPE-only input must set bozo");
    assert_eq!(feed.version, FeedVersion::Unknown);
}

#[test]
fn test_parse_invalid_utf8_bytes() {
    let feed = parse(b"\xFF\xFE").unwrap();
    assert!(feed.bozo, "invalid UTF-8 bytes must set bozo");
    assert_eq!(feed.version, FeedVersion::Unknown);
}

#[test]
fn test_capacity_constructors() {
    use feedparser_rs::{Entry, FeedMeta, ParsedFeed};

    // Test ParsedFeed::with_capacity
    let feed = ParsedFeed::with_capacity(100);
    assert_eq!(feed.encoding, "utf-8");
    assert_eq!(feed.entries.capacity(), 100);
    assert!(feed.namespaces.capacity() >= 8);

    // Test FeedMeta::with_rss_capacity
    let rss_meta = FeedMeta::with_rss_capacity();
    assert!(rss_meta.links.capacity() >= 2);
    assert!(rss_meta.authors.capacity() >= 1);
    assert!(rss_meta.tags.capacity() >= 3);

    // Test FeedMeta::with_atom_capacity
    let atom_meta = FeedMeta::with_atom_capacity();
    assert!(atom_meta.links.capacity() >= 4);
    assert!(atom_meta.authors.capacity() >= 2);
    assert!(atom_meta.tags.capacity() >= 5);

    // Test Entry::with_capacity
    let entry = Entry::with_capacity();
    assert!(entry.links.capacity() >= 2);
    assert!(entry.content.capacity() >= 1);
    assert!(entry.authors.capacity() >= 1);
    assert!(entry.tags.capacity() >= 3);
}

#[test]
fn test_parse_json_feed_basic() {
    let json = load_fixture("json/basic-1.1.json");
    let result = parse(&json);

    assert!(result.is_ok(), "Failed to parse JSON Feed fixture");
    let feed = result.unwrap();

    assert_eq!(feed.version, FeedVersion::JsonFeed11);
    assert!(!feed.bozo);
    assert_eq!(feed.feed.title.as_deref(), Some("Example JSON Feed"));
    assert_eq!(feed.entries.len(), 1);
    assert_eq!(feed.entries[0].id.as_deref(), Some("1"));
    assert_eq!(feed.entries[0].title.as_deref(), Some("First Post"));
}

#[test]
fn test_parse_json_feed_10() {
    let json = load_fixture("json/basic-1.0.json");
    let result = parse(&json);

    assert!(result.is_ok());
    let feed = result.unwrap();

    assert_eq!(feed.version, FeedVersion::JsonFeed10);
    assert!(!feed.bozo);
}

#[test]
fn test_parse_json_feed_minimal() {
    let json = load_fixture("json/minimal.json");
    let result = parse(&json);

    assert!(result.is_ok());
    let feed = result.unwrap();

    assert_eq!(feed.version, FeedVersion::JsonFeed11);
    assert!(!feed.bozo);
    assert_eq!(feed.feed.title.as_deref(), Some("Minimal Feed"));
    assert_eq!(feed.entries.len(), 0);
}

#[test]
fn test_parse_itunes_podcast_feed() {
    let xml = load_fixture("podcast/itunes-basic.xml");
    let result = parse(&xml);

    assert!(result.is_ok(), "Failed to parse iTunes podcast fixture");
    let feed = result.unwrap();

    // Verify basic feed properties
    assert_eq!(feed.version, FeedVersion::Rss20);
    assert!(!feed.bozo, "Feed should not have bozo flag set");
    assert_eq!(feed.feed.title.as_deref(), Some("Example Podcast"));

    // Verify iTunes feed-level metadata
    assert!(
        feed.feed.itunes.is_some(),
        "Feed should have iTunes metadata"
    );
    let itunes = feed.feed.itunes.as_ref().unwrap();

    assert_eq!(itunes.author.as_deref(), Some("John Doe"));
    assert_eq!(itunes.explicit, None); // "no" maps to None per Python feedparser compat
    assert_eq!(
        itunes.image.as_deref(),
        Some("https://example.com/podcast-cover.jpg")
    );
    assert!(!itunes.categories.is_empty());
    assert_eq!(itunes.categories[0].text, "Technology");

    // Verify owner information
    assert!(itunes.owner.is_some());
    let owner = itunes.owner.as_ref().unwrap();
    assert_eq!(owner.name.as_deref(), Some("Jane Smith"));
    assert_eq!(owner.email.as_deref(), Some("contact@example.com"));

    // Verify keywords
    assert_eq!(itunes.keywords, vec!["rust", "programming", "tech"]);

    // Verify podcast type
    assert_eq!(itunes.podcast_type.as_deref(), Some("episodic"));

    assert!(!feed.entries.is_empty(), "Feed should have episodes");
}

#[test]
fn test_rss_with_unknown_entity_sets_bozo() {
    let xml = br#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>Test &unknown; Feed</title>
    <link>https://example.com</link>
    <description>desc</description>
  </channel>
</rss>"#;
    let result = parse(xml).unwrap();
    assert!(result.bozo, "Feed with unknown entity should set bozo=true");
    assert!(
        result
            .bozo_exception
            .as_deref()
            .unwrap_or("")
            .contains("Unresolvable entity"),
        "bozo_exception should mention unresolvable entity"
    );
}

#[test]
fn test_atom_with_unknown_entity_sets_bozo() {
    let xml = br#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <id>https://example.com/&unknown;feed</id>
  <title>Test Feed</title>
  <updated>2024-01-01T00:00:00Z</updated>
</feed>"#;
    let result = parse(xml).unwrap();
    assert!(
        result.bozo,
        "Atom feed with unknown entity should set bozo=true"
    );
}

#[test]
fn test_rss20_entry_entity_bozo() {
    let xml = load_fixture("malformed/entry-entity-rss20.xml");
    let feed = parse(&xml).unwrap();
    assert!(feed.bozo);
    assert_eq!(
        feed.bozo_exception.as_deref(),
        Some("Unresolvable entity in entry field")
    );
    assert_eq!(feed.entries.len(), 1);
}

#[test]
fn test_atom10_entry_entity_bozo() {
    let xml = load_fixture("malformed/entry-entity-atom10.xml");
    let feed = parse(&xml).unwrap();
    assert!(feed.bozo);
    assert_eq!(
        feed.bozo_exception.as_deref(),
        Some("Unresolvable entity in entry field")
    );
    assert_eq!(feed.entries.len(), 1);
}

#[test]
fn test_rss10_entry_entity_bozo() {
    let xml = load_fixture("malformed/entry-entity-rss10.xml");
    let feed = parse(&xml).unwrap();
    assert!(feed.bozo);
    assert_eq!(
        feed.bozo_exception.as_deref(),
        Some("Unresolvable entity in entry field")
    );
    assert_eq!(feed.entries.len(), 1);
}

#[test]
fn test_parse_atom03_fixture() {
    let xml = load_fixture("atom/atom03.xml");
    let result = parse(&xml);

    assert!(result.is_ok(), "Failed to parse Atom 0.3 fixture");
    let feed = result.unwrap();

    assert!(!feed.bozo, "Well-formed Atom 0.3 feed must not set bozo");
    assert_eq!(feed.version, FeedVersion::Atom03, "Version must be atom03");
    assert_eq!(feed.feed.title.as_deref(), Some("Example Atom 0.3 Feed"));
    assert!(feed.feed.link.is_some(), "Feed link must be populated");
    assert_eq!(feed.entries.len(), 2, "Feed must have two entries");

    let entry = &feed.entries[0];
    assert_eq!(entry.title.as_deref(), Some("First Entry"));
    assert!(entry.id.is_some(), "Entry id must be populated");
    assert!(entry.updated.is_some(), "Entry updated must be parsed");
}

#[test]
fn test_detect_format_atom03_fixture() {
    let xml = load_fixture("atom/atom03.xml");
    let version = detect_format(&xml);
    assert_eq!(version, FeedVersion::Atom03);
}

#[test]
fn test_feed_with_standard_entities_no_bozo() {
    let xml = br#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>AT&amp;T Feed</title>
    <link>https://example.com</link>
    <description>Less &lt;than&gt; and more</description>
  </channel>
</rss>"#;
    let result = parse(xml).unwrap();
    assert!(
        !result.bozo,
        "Feed with only standard entities should not set bozo"
    );
}

#[test]
fn test_parse_json_feed_extensions_fixture() {
    let json = load_fixture("json/extensions.json");
    let result = parse(&json);

    assert!(result.is_ok(), "Failed to parse extensions fixture");
    let feed = result.unwrap();

    assert_eq!(feed.version, FeedVersion::JsonFeed11);
    assert!(!feed.bozo);
    assert_eq!(
        feed.feed.json_extensions["_cast"]["subcategory"],
        "Tech News"
    );
    assert_eq!(feed.entries[0].json_extensions["_explicit"], true);
}

#[test]
fn test_parse_json_feed_next_url_banner_fixture() {
    let json = load_fixture("json/next-url-banner.json");
    let result = parse(&json);

    assert!(result.is_ok(), "Failed to parse next-url-banner fixture");
    let feed = result.unwrap();

    assert_eq!(feed.version, FeedVersion::JsonFeed11);
    assert!(!feed.bozo);

    // Verify next_url is parsed
    assert_eq!(
        feed.feed.next_url.as_deref(),
        Some("https://example.com/feed.json?page=2")
    );

    // Entry 0: has banner_image
    let banner = feed.entries[0]
        .links
        .iter()
        .find(|l| l.rel.as_deref() == Some("banner"));
    assert!(banner.is_some(), "Entry 0 must have a banner link");
    assert_eq!(
        banner.unwrap().href.as_str(),
        "https://example.com/banner.jpg"
    );

    // Entry 1: has both image and banner_image
    let enclosure = feed.entries[1]
        .links
        .iter()
        .find(|l| l.rel.as_deref() == Some("enclosure"));
    let banner2 = feed.entries[1]
        .links
        .iter()
        .find(|l| l.rel.as_deref() == Some("banner"));
    assert!(enclosure.is_some(), "Entry 1 must have an enclosure link");
    assert!(banner2.is_some(), "Entry 1 must have a banner link");

    // Entry 2: no banner_image
    let no_banner = feed.entries[2]
        .links
        .iter()
        .find(|l| l.rel.as_deref() == Some("banner"));
    assert!(no_banner.is_none(), "Entry 2 must not have a banner link");
}

#[test]
fn test_atom_entry_subtitle_html() {
    let xml = load_fixture("atom/entry-subtitle.xml");
    let feed = parse(&xml).unwrap();

    assert!(!feed.bozo, "Valid feed must not set bozo");
    assert_eq!(feed.entries.len(), 3);

    // First entry: html subtitle with entities
    let entry = &feed.entries[0];
    assert_eq!(
        entry.subtitle.as_deref(),
        Some("A longer <em>teaser</em> description")
    );
    let detail = entry.subtitle_detail.as_ref().unwrap();
    assert_eq!(detail.content_type, TextType::Html);
    // summary is independent from subtitle
    assert_eq!(entry.summary.as_deref(), Some("Plain text summary"));

    // Second entry: subtitle without a type attr displays as RFC 4287 "text",
    // but carries no explicit safety assertion, so it is sanitize-typed as Html
    // (fail-closed, #438); harmless plain text is unaffected by sanitization.
    let entry2 = &feed.entries[1];
    assert_eq!(
        entry2.subtitle.as_deref(),
        Some("Plain text subtitle without type attribute")
    );
    let detail2 = entry2.subtitle_detail.as_ref().unwrap();
    assert_eq!(detail2.content_type, TextType::Html);

    // Third entry: no subtitle
    let entry3 = &feed.entries[2];
    assert!(entry3.subtitle.is_none());
    assert!(entry3.subtitle_detail.is_none());
}

#[test]
fn test_atom_entry_subtitle_does_not_affect_feed_subtitle() {
    let xml = load_fixture("atom/entry-subtitle.xml");
    let feed = parse(&xml).unwrap();

    assert_eq!(feed.feed.subtitle.as_deref(), Some("Feed-level subtitle"));
}

#[test]
fn test_rss_category_domain_attribute() {
    let xml = load_fixture("rss/rss20-category-domain.xml");
    let feed = parse(&xml).unwrap();

    assert!(!feed.bozo, "Valid feed must not be bozo");

    // Feed-level categories
    let feed_tags = &feed.feed.tags;
    assert_eq!(feed_tags.len(), 2);

    let tag_with_domain = &feed_tags[0];
    assert_eq!(tag_with_domain.term.as_str(), "music");
    assert_eq!(
        tag_with_domain.scheme.as_deref(),
        Some("http://www.sixapart.com/ns/types#")
    );

    let tag_plain = &feed_tags[1];
    assert_eq!(tag_plain.term.as_str(), "plain-term");
    assert!(tag_plain.scheme.is_none());

    // Entry-level categories
    let entry = &feed.entries[0];
    let entry_tags = &entry.tags;
    assert_eq!(entry_tags.len(), 3);

    assert_eq!(entry_tags[0].term.as_str(), "Technology");
    assert_eq!(
        entry_tags[0].scheme.as_deref(),
        Some("http://example.com/topics")
    );

    assert_eq!(entry_tags[1].term.as_str(), "Rust");
    assert_eq!(
        entry_tags[1].scheme.as_deref(),
        Some("http://example.com/topics")
    );

    assert_eq!(entry_tags[2].term.as_str(), "no-domain");
    assert!(entry_tags[2].scheme.is_none());
}

#[test]
fn test_atom_enclosure_full_attributes() {
    let xml = load_fixture("atom/with-enclosures.xml");
    let feed = parse(&xml).unwrap();

    assert!(
        !feed.bozo,
        "feed must not be bozo for valid Atom with enclosures"
    );

    // Entry 0: full enclosure attrs
    let entry = &feed.entries[0];
    assert_eq!(entry.enclosures.len(), 1);
    assert_eq!(
        entry.enclosures[0].url.as_str(),
        "http://example.com/ep1.mp3"
    );
    assert_eq!(
        entry.enclosures[0].enclosure_type.as_deref(),
        Some("audio/mpeg")
    );
    assert_eq!(entry.enclosures[0].length.as_deref(), Some("12345678"));

    // Enclosure link also appears in entry.links (dual population)
    assert!(
        entry
            .links
            .iter()
            .any(|l| l.href.as_str() == "http://example.com/ep1.mp3"),
        "enclosure link must also be in entry.links"
    );

    // Primary link is the alternate, not the enclosure
    assert_eq!(entry.link.as_deref(), Some("http://example.com/ep1"));
}

#[test]
fn test_atom_enclosure_missing_type() {
    let xml = load_fixture("atom/with-enclosures.xml");
    let feed = parse(&xml).unwrap();

    assert!(!feed.bozo);

    // Entry 1: enclosure with no type attr
    let entry = &feed.entries[1];
    assert_eq!(entry.enclosures.len(), 1);
    assert_eq!(
        entry.enclosures[0].url.as_str(),
        "http://example.com/ep2.mp3"
    );
    assert_eq!(
        entry.enclosures[0].enclosure_type.as_deref(),
        Some("text/html")
    );
    assert_eq!(entry.enclosures[0].length.as_deref(), Some("9876543"));
}

#[test]
fn test_atom_enclosure_missing_length() {
    let xml = load_fixture("atom/with-enclosures.xml");
    let feed = parse(&xml).unwrap();

    assert!(!feed.bozo);

    // Entry 2: enclosure with no length attr
    let entry = &feed.entries[2];
    assert_eq!(entry.enclosures.len(), 1);
    assert_eq!(
        entry.enclosures[0].url.as_str(),
        "http://example.com/ep3.mp3"
    );
    assert_eq!(
        entry.enclosures[0].enclosure_type.as_deref(),
        Some("audio/mpeg")
    );
    assert!(entry.enclosures[0].length.is_none());
}

#[test]
fn test_atom_enclosure_invalid_length() {
    let xml = load_fixture("atom/with-enclosures.xml");
    let feed = parse(&xml).unwrap();

    assert!(!feed.bozo, "invalid length must not set bozo");

    // Entry 3: enclosure with non-numeric length — raw string is preserved
    let entry = &feed.entries[3];
    assert_eq!(entry.enclosures.len(), 1);
    assert_eq!(
        entry.enclosures[0].url.as_str(),
        "http://example.com/ep4.mp3"
    );
    assert_eq!(entry.enclosures[0].length.as_deref(), Some("not-a-number"));
}

#[test]
fn test_atom_enclosure_multiple() {
    let xml = load_fixture("atom/with-enclosures.xml");
    let feed = parse(&xml).unwrap();

    assert!(!feed.bozo);

    // Entry 4: two enclosure links
    let entry = &feed.entries[4];
    assert_eq!(entry.enclosures.len(), 2);

    let mp3 = entry
        .enclosures
        .iter()
        .find(|e| e.url.as_str() == "http://example.com/ep5.mp3")
        .expect("mp3 enclosure");
    assert_eq!(mp3.enclosure_type.as_deref(), Some("audio/mpeg"));
    assert_eq!(mp3.length.as_deref(), Some("5000000"));

    let pdf = entry
        .enclosures
        .iter()
        .find(|e| e.url.as_str() == "http://example.com/ep5.pdf")
        .expect("pdf enclosure");
    assert_eq!(pdf.enclosure_type.as_deref(), Some("application/pdf"));
    assert_eq!(pdf.length.as_deref(), Some("1000000"));

    // Both enclosure links appear in entry.links as well
    assert_eq!(
        entry
            .links
            .iter()
            .filter(|l| l.rel.as_deref() == Some("enclosure"))
            .count(),
        2
    );
}

#[test]
fn test_text_construct_value_populated() {
    let xml = load_fixture("atom/entry-subtitle.xml");
    let feed = parse(&xml).unwrap();

    // subtitle_detail.value must equal subtitle string for html type
    let entry = &feed.entries[0];
    let detail = entry.subtitle_detail.as_ref().unwrap();
    assert_eq!(detail.content_type, TextType::Html);
    assert_eq!(
        detail.value,
        entry.subtitle.as_deref().unwrap_or(""),
        "subtitle_detail.value must equal subtitle"
    );

    // subtitle_detail.value must equal subtitle string for an untyped (fail-closed
    // Html, #438) subtitle too
    let entry2 = &feed.entries[1];
    let detail2 = entry2.subtitle_detail.as_ref().unwrap();
    assert_eq!(detail2.content_type, TextType::Html);
    assert_eq!(
        detail2.value,
        entry2.subtitle.as_deref().unwrap_or(""),
        "subtitle_detail.value must equal subtitle for untyped subtitle"
    );
}

#[test]
fn test_atom_title_and_summary_detail_value() {
    let xml = load_fixture("atom/basic.xml");
    let feed = parse(&xml).unwrap();

    let entry = &feed.entries[0];

    // title_detail.value must equal title
    let title_detail = entry.title_detail.as_ref().unwrap();
    assert_eq!(
        title_detail.value,
        entry.title.as_deref().unwrap_or(""),
        "title_detail.value must equal title"
    );

    // summary_detail.value must equal summary
    let summary_detail = entry.summary_detail.as_ref().unwrap();
    assert_eq!(
        summary_detail.value,
        entry.summary.as_deref().unwrap_or(""),
        "summary_detail.value must equal summary"
    );
}

#[test]
fn test_rss_source_element_title_and_link() {
    let xml = load_fixture("rss/with-source.xml");
    let feed = parse(&xml).unwrap();

    assert!(!feed.bozo, "valid feed must not set bozo");
    let entry = &feed.entries[0];
    let source = entry.source.as_ref().expect("entry must have source");
    assert_eq!(source.title.as_deref(), Some("Other Feed Name"));
    assert_eq!(source.href.as_deref(), Some("https://otherfeed.com/rss"));
}

// Regression tests for issue #152: whitespace around XML entities must be preserved.
#[test]
fn test_rss20_entity_whitespace_preserved() {
    let xml = load_fixture("rss/rss20-entity-whitespace.xml");
    let feed = parse(&xml).unwrap();

    assert!(!feed.bozo, "valid feed must not set bozo");
    assert_eq!(feed.entries.len(), 5);

    // space before entity sequence
    assert_eq!(feed.entries[0].summary.as_deref(), Some("word <b>bold</b>"));

    // space after entity sequence
    assert_eq!(
        feed.entries[1].summary.as_deref(),
        Some("<b>bold</b> after")
    );

    // spaces on both sides
    assert_eq!(
        feed.entries[2].summary.as_deref(),
        Some("word <b>bold</b> after")
    );

    // multiple entity sequences with spaces between
    assert_eq!(
        feed.entries[3].summary.as_deref(),
        Some("a <em>b</em> c <strong>d</strong> e")
    );

    // &amp; entity with adjacent spaces
    //
    // The parser resolves the &amp; entity to a literal "&" (common.rs::resolve_entity);
    // HTML sanitization then re-escapes it since entry.summary carries TextType::Html
    // (matches Python feedparser's sanitizer behavior for bare ampersands, see #438).
    assert_eq!(feed.entries[4].summary.as_deref(), Some("foo &amp; bar"));
}

#[test]
fn test_atom_xhtml_content_preserves_markup() {
    let xml = load_fixture("atom/xhtml-content.xml");
    let feed = parse(&xml).unwrap();

    // Entry 0: content type="xhtml" with <p>Hello <b>world</b></p>
    let entry = &feed.entries[0];
    assert!(!feed.bozo, "valid xhtml feed must not set bozo");
    assert!(!entry.content.is_empty(), "xhtml content must be populated");
    let value = &entry.content[0].value;
    assert!(
        value.contains("<b>world</b>"),
        "markup must be preserved: {value}"
    );
    assert!(
        value.contains("<p>"),
        "paragraph tag must be preserved: {value}"
    );
    assert!(
        !value.contains("<div"),
        "outer div must be stripped: {value}"
    );
}

#[test]
fn test_atom_xhtml_summary_preserves_markup() {
    let xml = load_fixture("atom/xhtml-content.xml");
    let feed = parse(&xml).unwrap();

    // Entry 1: summary type="xhtml" with <p>Summary with <em>markup</em></p>
    let entry = &feed.entries[1];
    let summary = entry.summary.as_deref().unwrap_or("");
    assert!(
        summary.contains("<em>markup</em>"),
        "xhtml summary markup must be preserved: {summary}"
    );
    assert!(
        !summary.contains("<div"),
        "outer div must be stripped from summary: {summary}"
    );
}

#[test]
fn test_atom_xhtml_empty_content_no_panic() {
    let xml = load_fixture("atom/xhtml-content.xml");
    let feed = parse(&xml).unwrap();

    // Entry 2: content type="xhtml" with empty <div>
    let entry = &feed.entries[2];
    if !entry.content.is_empty() {
        assert_eq!(
            entry.content[0].value, "",
            "empty xhtml content must yield empty string"
        );
    }
}

#[test]
fn test_atom_xhtml_malformed_no_div_no_panic() {
    let xml = load_fixture("atom/xhtml-content.xml");
    // Parsing must not panic on malformed xhtml (no div wrapper)
    let result = parse(&xml);
    assert!(
        result.is_ok(),
        "parsing malformed xhtml feed must not return error"
    );
}

#[test]
fn test_atom_xhtml_content_existing_fixture() {
    // The with-content.xml fixture has a second entry with type="xhtml"
    let xml = load_fixture("atom/with-content.xml");
    let feed = parse(&xml).unwrap();

    let xhtml_entry = feed
        .entries
        .iter()
        .find(|e| !e.content.is_empty() && e.content[0].value.contains("<h2>"))
        .or_else(|| {
            feed.entries
                .iter()
                .find(|e| e.title.as_deref() == Some("Second Post with XHTML Content"))
        });

    if let Some(entry) = xhtml_entry {
        let value = &entry.content[0].value;
        assert!(
            value.contains("<h2>"),
            "h2 markup must be preserved: {value}"
        );
        assert!(
            value.contains("<em>XHTML</em>"),
            "em markup must be preserved: {value}"
        );
        assert!(
            !value.contains("<div"),
            "outer div must be stripped: {value}"
        );
    }
}

#[test]
fn test_atom_content_xhtml_type_normalization() {
    let xml = br#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test Feed</title>
  <id>urn:test:xhtml-normalization</id>
  <updated>2024-01-01T00:00:00Z</updated>
  <entry>
    <title>Entry</title>
    <id>urn:test:entry1</id>
    <updated>2024-01-01T00:00:00Z</updated>
    <content type="xhtml"><div xmlns="http://www.w3.org/1999/xhtml"><p>Hello</p></div></content>
  </entry>
  <entry>
    <title>Entry 2</title>
    <id>urn:test:entry2</id>
    <updated>2024-01-01T00:00:00Z</updated>
    <content type="html">&lt;p&gt;Hello&lt;/p&gt;</content>
  </entry>
  <entry>
    <title>Entry 3</title>
    <id>urn:test:entry3</id>
    <updated>2024-01-01T00:00:00Z</updated>
    <content type="text">plain text</content>
  </entry>
</feed>"#;

    let feed = parse(xml).unwrap();
    assert_eq!(feed.entries.len(), 3);

    let xhtml_type = feed.entries[0].content[0].content_type.as_deref();
    assert_eq!(
        xhtml_type,
        Some("application/xhtml+xml"),
        "xhtml should normalize to application/xhtml+xml"
    );

    let html_type = feed.entries[1].content[0].content_type.as_deref();
    assert_eq!(
        html_type,
        Some("text/html"),
        "html should normalize to text/html"
    );

    let text_type = feed.entries[2].content[0].content_type.as_deref();
    assert_eq!(
        text_type,
        Some("text/plain"),
        "text should normalize to text/plain"
    );
}

// --- Issue #273/#274/#275 regression tests ---

/// #273: entry.id promoted to entry.link when no explicit <link> present
#[test]
fn test_atom_entry_id_promoted_to_link_when_no_explicit_link() {
    let xml = load_fixture("atom/id-only-no-link.xml");
    let result = parse(&xml).unwrap();

    let entry0 = &result.entries[0];
    assert_eq!(
        entry0.link.as_deref(),
        Some("http://example.com/entry1"),
        "entry.link should be promoted from entry.id when no explicit link"
    );
    assert_eq!(
        entry0.guidislink,
        Some(true),
        "guidislink is true when entry.link is promoted from entry.id (#285)"
    );
}

/// #273: guidislink=false when explicit <link> present
#[test]
fn test_atom_entry_guidislink_false_when_explicit_link() {
    let xml = load_fixture("atom/id-only-no-link.xml");
    let result = parse(&xml).unwrap();

    let entry1 = &result.entries[1];
    assert_eq!(
        entry1.link.as_deref(),
        Some("http://example.com/entry2-link"),
        "entry.link should be the explicit link, not the id"
    );
    assert_eq!(
        entry1.guidislink,
        Some(false),
        "guidislink should be false when explicit link is present"
    );
}

/// #274: feed.id promoted to feed.link when no explicit feed <link> present
#[test]
fn test_atom_feed_id_promoted_to_feed_link_when_no_explicit_link() {
    let xml = load_fixture("atom/id-only-no-link.xml");
    let result = parse(&xml).unwrap();

    assert_eq!(
        result.feed.link.as_deref(),
        Some("http://example.com/feed"),
        "feed.link should be promoted from feed.id when no explicit link"
    );
}

/// #274: feed.link NOT overwritten when explicit feed <link> is present
#[test]
fn test_atom_feed_explicit_link_not_overwritten_by_id() {
    let xml = load_fixture("atom/basic.xml");
    let result = parse(&xml).unwrap();

    assert_eq!(
        result.feed.link.as_deref(),
        Some("http://example.com"),
        "feed.link should remain the explicit link"
    );
}

/// #275: entry.updated falls back to entry.published when <updated> absent
#[test]
fn test_atom_entry_updated_fallback_from_published() {
    let xml = load_fixture("atom/id-only-no-link.xml");
    let result = parse(&xml).unwrap();

    let entry0 = &result.entries[0];
    assert!(entry0.published.is_some(), "entry.published should be set");
    assert_eq!(
        entry0.updated, entry0.published,
        "entry.updated should equal entry.published when <updated> is absent"
    );
}

/// #275: entry.updated NOT overwritten when <updated> is explicitly set
#[test]
fn test_atom_xhtml_content_html_entities_preserved() {
    // Regression test for issue #215: &amp;, &lt;, &gt; in xhtml text nodes must be re-escaped
    let xml = br#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>T</title><link href="http://x.com"/><updated>2024-01-01T00:00:00Z</updated><id>urn:t</id>
  <entry>
    <id>urn:e:1</id><updated>2024-01-01T00:00:00Z</updated><title>E</title>
    <content type="xhtml">
      <div xmlns="http://www.w3.org/1999/xhtml"><p>Tom &amp; Jerry</p><p>x &lt; y</p></div>
    </content>
  </entry>
</feed>"#;
    let feed = parse(xml).unwrap();
    assert!(!feed.bozo, "valid feed must not be bozo");
    let value = &feed.entries[0].content[0].value;
    assert!(
        value.contains("&amp;"),
        "expected &amp; preserved, got: {value:?}"
    );
    assert!(
        value.contains("&lt;"),
        "expected &lt; preserved, got: {value:?}"
    );
    assert!(
        !value.contains("Tom  Jerry"),
        "entity must not be silently dropped"
    );
}

#[test]
fn test_atom_entry_updated_not_overwritten_when_set() {
    let xml = load_fixture("atom/basic.xml");
    let result = parse(&xml).unwrap();

    let entry0 = &result.entries[0];
    assert!(entry0.updated.is_some(), "entry.updated should be set");
    // In basic.xml, updated=2024-12-14T09:00:00Z, published is absent
    // published should be None, updated should be the explicit value
    assert_eq!(entry0.updated_str.as_deref(), Some("2024-12-14T09:00:00Z"));
}
