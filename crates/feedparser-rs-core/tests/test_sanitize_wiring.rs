//! Regression tests for issue #438: `ParseOptions.sanitize_html` was documented but
//! never consumed by any parse entry point, leaving stored XSS payloads intact in
//! `ParsedFeed` output across every supported format.
#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use feedparser_rs::{ParseOptions, parse, parse_with_options};

const DANGEROUS_MARKERS: &[&str] = &["<script", "onerror=", "onbegin="];

fn assert_no_dangerous_markup(haystack: &str) {
    for marker in DANGEROUS_MARKERS {
        assert!(
            !haystack.contains(marker),
            "expected sanitized output to not contain {marker:?}, got: {haystack}"
        );
    }
}

// ---------------------------------------------------------------------------
// 4-format XSS corpus (issue #438 reproduction)
// ---------------------------------------------------------------------------

const TITLE_PAYLOAD: &str = "&lt;script&gt;alert(1)&lt;/script&gt;Post";

#[test]
fn test_rss20_xss_payload_sanitized_by_default() {
    let xml = format!(
        r#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>{TITLE_PAYLOAD}</title>
    <item>
      <title>{TITLE_PAYLOAD}</title>
      <description>&lt;script&gt;alert(1)&lt;/script&gt;&lt;img src=x onerror=alert(1)&gt;</description>
    </item>
  </channel>
</rss>"#
    );

    let feed = parse(xml.as_bytes()).unwrap();
    assert!(!feed.bozo);
    assert_no_dangerous_markup(feed.feed.title.as_deref().unwrap_or(""));
    assert_no_dangerous_markup(feed.entries[0].title.as_deref().unwrap_or(""));
    assert_no_dangerous_markup(feed.entries[0].summary.as_deref().unwrap_or(""));
}

#[test]
fn test_atom_xss_payload_sanitized_by_default() {
    let xml = format!(
        r#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>{TITLE_PAYLOAD}</title>
  <entry>
    <title>{TITLE_PAYLOAD}</title>
    <summary type="html">&lt;script&gt;alert(1)&lt;/script&gt;&lt;svg&gt;&lt;animate onbegin="alert(1)" attributeName="x"&gt;&lt;/svg&gt;</summary>
  </entry>
</feed>"#
    );

    let feed = parse(xml.as_bytes()).unwrap();
    assert!(!feed.bozo);
    assert_no_dangerous_markup(feed.feed.title.as_deref().unwrap_or(""));
    assert_no_dangerous_markup(feed.entries[0].title.as_deref().unwrap_or(""));
    assert_no_dangerous_markup(feed.entries[0].summary.as_deref().unwrap_or(""));
}

#[test]
fn test_rss10_xss_payload_sanitized_by_default() {
    let xml = format!(
        r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns="http://purl.org/rss/1.0/">
  <channel rdf:about="http://example.com/">
    <title>{TITLE_PAYLOAD}</title>
    <link>http://example.com/</link>
    <description>Feed description</description>
  </channel>
  <item rdf:about="http://example.com/1">
    <title>{TITLE_PAYLOAD}</title>
    <description>&lt;script&gt;alert(1)&lt;/script&gt;&lt;img src=x onerror=alert(1)&gt;</description>
  </item>
</rdf:RDF>"#
    );

    let feed = parse(xml.as_bytes()).unwrap();
    assert!(!feed.bozo);
    assert_no_dangerous_markup(feed.feed.title.as_deref().unwrap_or(""));
    assert_no_dangerous_markup(feed.entries[0].title.as_deref().unwrap_or(""));
    assert_no_dangerous_markup(feed.entries[0].summary.as_deref().unwrap_or(""));
}

#[test]
fn test_json_feed_xss_payload_sanitized_by_default() {
    let json = br#"{
        "version": "https://jsonfeed.org/version/1.1",
        "title": "<script>alert(1)</script>Feed",
        "description": "<script>alert(1)</script>Description",
        "items": [
            {
                "id": "1",
                "title": "<script>alert(1)</script>Post",
                "summary": "<script>alert(1)</script>Summary",
                "content_html": "<script>alert(1)</script><img src=x onerror=alert(1)>"
            }
        ]
    }"#;

    let feed = parse(json).unwrap();
    assert!(!feed.bozo);
    assert_no_dangerous_markup(feed.feed.title.as_deref().unwrap_or(""));
    assert_no_dangerous_markup(feed.feed.subtitle.as_deref().unwrap_or(""));
    assert_no_dangerous_markup(feed.entries[0].title.as_deref().unwrap_or(""));
    assert_no_dangerous_markup(feed.entries[0].summary.as_deref().unwrap_or(""));
    assert_no_dangerous_markup(feed.entries[0].content[0].value.as_str());
}

// ---------------------------------------------------------------------------
// Atom `type` attribute bypass matrix (audit P1 finding)
// ---------------------------------------------------------------------------

fn atom_summary_feed(type_attr_and_body: &str) -> Vec<u8> {
    format!(
        r#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Feed</title>
  <entry>
    <title>Post</title>
    {type_attr_and_body}
  </entry>
</feed>"#
    )
    .into_bytes()
}

#[test]
fn test_atom_type_html_keyword_is_sanitized() {
    let xml = atom_summary_feed(
        r#"<summary type="html">&lt;script&gt;alert(1)&lt;/script&gt;Hi</summary>"#,
    );
    let feed = parse(&xml).unwrap();
    assert_no_dangerous_markup(feed.entries[0].summary.as_deref().unwrap_or(""));
}

#[test]
fn test_atom_type_text_html_mime_spelling_is_sanitized() {
    // #438 audit P1: type="text/html" previously fell through to the default
    // TextType::Text branch and bypassed sanitization entirely.
    let xml = atom_summary_feed(
        r#"<summary type="text/html">&lt;script&gt;alert(1)&lt;/script&gt;Hi</summary>"#,
    );
    let feed = parse(&xml).unwrap();
    assert_no_dangerous_markup(feed.entries[0].summary.as_deref().unwrap_or(""));
}

#[test]
fn test_atom_type_application_xhtml_xml_mime_spelling_is_sanitized() {
    let xml = atom_summary_feed(
        r#"<summary type="application/xhtml+xml"><div xmlns="http://www.w3.org/1999/xhtml"><script>alert(1)</script>Hi</div></summary>"#,
    );
    let feed = parse(&xml).unwrap();
    assert_no_dangerous_markup(feed.entries[0].summary.as_deref().unwrap_or(""));
}

#[test]
fn test_atom_type_uppercase_html_is_sanitized() {
    // Case-insensitive: type="HTML" must map the same as type="html".
    let xml = atom_summary_feed(
        r#"<summary type="HTML">&lt;script&gt;alert(1)&lt;/script&gt;Hi</summary>"#,
    );
    let feed = parse(&xml).unwrap();
    assert_no_dangerous_markup(feed.entries[0].summary.as_deref().unwrap_or(""));
}

#[test]
fn test_atom_type_absent_is_sanitized() {
    // RFC 4287 §3.1.1 defaults an absent `type` to "text" for *display* purposes,
    // but an absent attribute makes no assertion at all — unlike explicit
    // type="text" below, it is not trusted and is sanitized like html/xhtml
    // (fail-closed, #438).
    let xml = atom_summary_feed(r"<summary>&lt;script&gt;alert(1)&lt;/script&gt;Hi</summary>");
    let feed = parse(&xml).unwrap();
    assert_no_dangerous_markup(feed.entries[0].summary.as_deref().unwrap_or(""));
}

#[test]
fn test_atom_type_absent_preserves_harmless_plain_text() {
    // Sanitizing text with no dangerous markup is a safe no-op: legitimate
    // unlabeled Atom summaries are not visibly mangled.
    let xml = atom_summary_feed(r"<summary>Plain text, not markup</summary>");
    let feed = parse(&xml).unwrap();
    assert_eq!(
        feed.entries[0].summary.as_deref(),
        Some("Plain text, not markup")
    );
}

#[test]
fn test_atom_type_literal_text_is_not_sanitized() {
    let xml = atom_summary_feed(r#"<summary type="text">Plain &amp; simple</summary>"#);
    let feed = parse(&xml).unwrap();
    assert_eq!(feed.entries[0].summary.as_deref(), Some("Plain & simple"));
}

// ---------------------------------------------------------------------------
// sanitize_html opt-out and idempotency
// ---------------------------------------------------------------------------

const fn xss_rss_feed() -> &'static [u8] {
    br#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>&lt;script&gt;alert(1)&lt;/script&gt;Feed</title>
    <item>
      <title>&lt;script&gt;alert(1)&lt;/script&gt;Post</title>
      <description>&lt;script&gt;alert(1)&lt;/script&gt;Hi</description>
    </item>
  </channel>
</rss>"#
}

#[test]
fn test_sanitize_html_false_preserves_raw_html() {
    let options = ParseOptions {
        sanitize_html: false,
        ..ParseOptions::default()
    };
    let feed = parse_with_options(xss_rss_feed(), &options).unwrap();
    assert_eq!(
        feed.entries[0].summary.as_deref(),
        Some("<script>alert(1)</script>Hi")
    );
}

#[test]
fn test_sanitize_html_default_true_strips_script() {
    let feed = parse_with_options(xss_rss_feed(), &ParseOptions::default()).unwrap();
    assert_no_dangerous_markup(feed.feed.title.as_deref().unwrap_or(""));
    assert_no_dangerous_markup(feed.entries[0].title.as_deref().unwrap_or(""));
    assert_no_dangerous_markup(feed.entries[0].summary.as_deref().unwrap_or(""));
}

#[test]
fn test_sanitize_html_is_idempotent() {
    let first = parse(xss_rss_feed()).unwrap();
    let second = parse(xss_rss_feed()).unwrap();
    assert_eq!(first.feed.title, second.feed.title);
    assert_eq!(first.entries[0].title, second.entries[0].title);
    assert_eq!(first.entries[0].summary, second.entries[0].summary);

    // Re-running the sanitizer directly on already-sanitized output must not change it.
    let mut third = first.clone();
    feedparser_rs::util::sanitize::sanitize_feed(
        &mut third,
        &feedparser_rs::ParserLimits::default(),
    );
    assert_eq!(first.feed.title, third.feed.title);
    assert_eq!(first.entries[0].title, third.entries[0].title);
    assert_eq!(first.entries[0].summary, third.entries[0].summary);
    assert_eq!(
        first.entries[0].title_detail.as_ref().map(|d| &d.value),
        third.entries[0].title_detail.as_ref().map(|d| &d.value)
    );
}

// ---------------------------------------------------------------------------
// Pathological HTML nesting (audit C4: algorithmic-complexity DoS)
// ---------------------------------------------------------------------------

#[test]
fn test_pathologically_nested_html_is_bounded_not_sanitized_unbounded() {
    // Ammonia's HTML5 tree builder is quadratic-time on deeply nested tags within
    // a single text field. A description this deep must not be handed to it
    // unbounded — `sanitize_feed` falls back to O(n) plain-text escaping instead,
    // which still guarantees no tag survives.
    let nesting_depth = 20_000;
    let mut description = String::with_capacity(nesting_depth * 11);
    for _ in 0..nesting_depth {
        description.push_str("<div>");
    }
    description.push_str("<script>alert(1)</script>");
    for _ in 0..nesting_depth {
        description.push_str("</div>");
    }

    let xml = format!(
        r#"<?xml version="1.0"?>
<rss version="2.0"><channel><title>F</title>
<item><title>T</title><description><![CDATA[{description}]]></description></item>
</channel></rss>"#
    );

    let start = std::time::Instant::now();
    let feed = parse(xml.as_bytes()).unwrap();
    let elapsed = start.elapsed();

    let summary = feed.entries[0].summary.as_deref().unwrap_or("");
    assert_no_dangerous_markup(summary);
    // The bounded fallback escapes every '<', so no real tag (opening or
    // self-closing) can survive — this is what proves the fallback engaged
    // rather than ammonia successfully sanitizing 20,000 levels of nesting.
    assert!(
        !summary.contains('<'),
        "expected all tags escaped by the nesting-depth fallback, got a tag in: {summary}"
    );
    // Soft timing guard: this is O(n) once bounded, so it should complete in low
    // milliseconds; a generous bound avoids CI flakiness while still catching a
    // regression back to ammonia's unbounded O(n^2) behavior (which took ~9s in
    // the security review's PoC on comparable input).
    assert!(
        elapsed.as_secs() < 5,
        "parsing pathologically nested HTML took too long: {elapsed:?}"
    );
}

#[test]
fn test_moderately_nested_html_is_still_sanitized_normally() {
    // Nesting below the bound must still go through ammonia normally, preserving
    // safe formatting rather than falling back to plain-text escaping.
    let xml = br#"<?xml version="1.0"?>
<rss version="2.0"><channel><title>F</title>
<item><title>T</title><description>&lt;div&gt;&lt;p&gt;&lt;b&gt;bold&lt;/b&gt;&lt;/p&gt;&lt;/div&gt;</description></item>
</channel></rss>"#;

    let feed = parse(xml).unwrap();
    let summary = feed.entries[0].summary.as_deref().unwrap_or("");
    assert_eq!(summary, "<div><p><b>bold</b></p></div>");
}

#[test]
fn test_mismatched_closing_tags_cannot_bypass_the_nesting_guard() {
    // Adversarial bypass of a naive open/close *counter*: a run of `<div>`
    // paired with a closing tag that never matches (`</x>`) holds a simple
    // counter at a shallow depth forever while building a real n-deep DOM,
    // so ammonia would still run unbounded on it. The name-matched tag stack
    // is not fooled by this: `</x>` matches nothing, so it pops nothing, and
    // the `<div>`s are correctly counted as staying open.
    // 80,000 repeats of this exact pattern was independently verified during
    // review to take ~15.6s against the naive open/close counter this test
    // guards against (the counter never advances past depth 1, so ammonia runs
    // unbounded on the resulting 80,000-deep DOM); the name-matched tag stack
    // closes that gap in single-digit milliseconds. 2,000 is used here to keep
    // the suite fast while still being 20x the default max_html_nesting_depth.
    let repeats = 2_000;
    let mut description = String::with_capacity(repeats * 12);
    for _ in 0..repeats {
        description.push_str("<div></x>");
    }

    let xml = format!(
        r#"<?xml version="1.0"?>
<rss version="2.0"><channel><title>F</title>
<item><title>T</title><description><![CDATA[{description}]]></description></item>
</channel></rss>"#
    );

    let start = std::time::Instant::now();
    let feed = parse(xml.as_bytes()).unwrap();
    let elapsed = start.elapsed();

    let summary = feed.entries[0].summary.as_deref().unwrap_or("");
    // Proves the guard actually engaged (rather than ammonia happening to be
    // fast on this input): the fallback escapes every '<', so no real `<div`
    // can survive once triggered.
    assert!(
        !summary.contains("<div"),
        "expected the nesting-depth fallback to engage on a mismatched-closing-tag \
         bypass attempt, got: {summary}"
    );
    assert!(
        elapsed.as_millis() < 500,
        "mismatched-closing-tag input must not reach ammonia's unbounded tree \
         builder; took {elapsed:?}"
    );
}

#[test]
fn test_many_void_elements_are_not_treated_as_deeply_nested() {
    // Void elements (br, img, ...) never nest and must not accumulate depth,
    // even well past the default max_html_nesting_depth of 100.
    let description: String = "<img src=\"x.png\">".repeat(150);
    let xml = format!(
        r#"<?xml version="1.0"?>
<rss version="2.0"><channel><title>F</title>
<item><title>T</title><description><![CDATA[{description}]]></description></item>
</channel></rss>"#
    );

    let feed = parse(xml.as_bytes()).unwrap();
    let summary = feed.entries[0].summary.as_deref().unwrap_or("");
    // Sanitized normally (img is an allowed tag), not escaped to tag soup.
    assert!(
        summary.contains("<img"),
        "150 <img> tags must not trip the nesting-depth fallback, got: {summary}"
    );
}

#[test]
fn test_many_unclosed_auto_closing_elements_are_not_deeply_nested() {
    // <li> auto-closes the previous <li>; a long run of unclosed <li>s inside
    // a <ul> must not accumulate depth past the bound.
    let items: String = "<li>item".repeat(150);
    let description = format!("<ul>{items}</ul>");
    let xml = format!(
        r#"<?xml version="1.0"?>
<rss version="2.0"><channel><title>F</title>
<item><title>T</title><description><![CDATA[{description}]]></description></item>
</channel></rss>"#
    );

    let feed = parse(xml.as_bytes()).unwrap();
    let summary = feed.entries[0].summary.as_deref().unwrap_or("");
    assert!(
        summary.contains("<li"),
        "150 unclosed <li> elements must not trip the nesting-depth fallback, got: {summary}"
    );
}

#[test]
fn test_scope_barrier_cannot_bypass_the_nesting_guard() {
    // html5ever's "has an element in scope" algorithm treats <table> (among
    // others) as a scope barrier: a </div> closing tag cannot pop through an
    // open <table> to reach an outer <div>. A search that ignores scope
    // barriers gets this wrong — it pops straight through the table,
    // believing the div closed cleanly every iteration, and never notices
    // that both elements are actually staying open and genuinely nesting
    // (independently verified: this exact pattern at 80,000 repeats stayed
    // quadratic — ~3.7s — under a barrier-unaware search; the scope-aware
    // search closes it).
    let repeats = 2_000;
    let mut description = String::with_capacity(repeats * 19);
    for _ in 0..repeats {
        description.push_str("<div><table></div>");
    }

    let xml = format!(
        r#"<?xml version="1.0"?>
<rss version="2.0"><channel><title>F</title>
<item><title>T</title><description><![CDATA[{description}]]></description></item>
</channel></rss>"#
    );

    let start = std::time::Instant::now();
    let feed = parse(xml.as_bytes()).unwrap();
    let elapsed = start.elapsed();

    let summary = feed.entries[0].summary.as_deref().unwrap_or("");
    assert!(
        !summary.contains("<div") && !summary.contains("<table"),
        "expected the nesting-depth fallback to engage on the table scope-barrier \
         bypass attempt, got: {summary}"
    );
    assert!(
        elapsed.as_millis() < 500,
        "table scope-barrier bypass input must not reach ammonia's unbounded tree \
         builder; took {elapsed:?}"
    );
}

#[test]
fn test_mixed_tag_auto_close_run_is_not_deeply_nested() {
    // Opening a new <p> while a <p> (with intervening non-auto-close content
    // like <span>) is still open implicitly closes the old <p> and
    // everything inside it — not just when <p> is directly on top of the
    // stack. A shallow top-of-stack-only check misses this and treats each
    // iteration as accumulating depth, silently escaping an ordinary
    // (if slightly malformed) field to visible tag soup.
    let paragraphs: String = "<p><span>Text ".repeat(60);
    let xml = format!(
        r#"<?xml version="1.0"?>
<rss version="2.0"><channel><title>F</title>
<item><title>T</title><description><![CDATA[{paragraphs}]]></description></item>
</channel></rss>"#
    );

    let feed = parse(xml.as_bytes()).unwrap();
    let summary = feed.entries[0].summary.as_deref().unwrap_or("");
    assert!(
        summary.contains("<p") && summary.contains("<span"),
        "a run of <p><span> pairs auto-closing the previous <p> must not trip \
         the nesting-depth fallback, got: {summary}"
    );
}

#[test]
fn test_many_unclosed_formatting_elements_are_not_deeply_nested() {
    // HTML5 "formatting elements" (b, font, ...) do not create the
    // pathologically deep, expensive-to-sanitize DOM that repeating a
    // structural element does when left unclosed — independently verified:
    // <b>/<font> repeated 40,000 times unclosed sanitizes in ~200ms
    // (linear), while <div>/<section> at the same scale take 30+ seconds
    // (quadratic). A run of unclosed <b> must not trip the fallback.
    let description: String = "<b>bold ".repeat(150);
    let xml = format!(
        r#"<?xml version="1.0"?>
<rss version="2.0"><channel><title>F</title>
<item><title>T</title><description><![CDATA[{description}]]></description></item>
</channel></rss>"#
    );

    let feed = parse(xml.as_bytes()).unwrap();
    let summary = feed.entries[0].summary.as_deref().unwrap_or("");
    assert!(
        summary.contains("<b>"),
        "150 unclosed <b> elements must not trip the nesting-depth fallback, got: {summary}"
    );
}

#[test]
fn test_excessive_flat_tag_count_trips_the_max_tags_backstop() {
    // O(1) backstop, independent of the tag-name model above: a field with
    // more tags than any legitimate feed content would ever contain is
    // treated as pathological regardless of what the depth stack reports.
    // Uses void elements (which never contribute to stack depth) so this
    // is exercising the tag-count backstop specifically, not the depth
    // guard.
    let description: String = "<br>".repeat(10_001);
    let xml = format!(
        r#"<?xml version="1.0"?>
<rss version="2.0"><channel><title>F</title>
<item><title>T</title><description><![CDATA[{description}]]></description></item>
</channel></rss>"#
    );

    let feed = parse(xml.as_bytes()).unwrap();
    let summary = feed.entries[0].summary.as_deref().unwrap_or("");
    assert!(
        !summary.contains("<br"),
        "expected the max-tags backstop to engage past 10,000 tags in one field, \
         got a <br> tag in: {summary}"
    );
}

#[test]
fn test_unclosed_th_header_row_is_not_deeply_nested() {
    // HTML5 makes </th> optional exactly like </td>, but AUTO_CLOSE_ELEMENTS
    // initially only listed "td", not "th" (review round-4 nitpick N1): a
    // table row consisting entirely of unclosed <th> header cells could
    // accumulate ~3 levels per row and trip the fallback around 34 rows.
    let headers: String = "<th>Header ".repeat(60);
    let description = format!("<table><tr>{headers}</tr></table>");
    let xml = format!(
        r#"<?xml version="1.0"?>
<rss version="2.0"><channel><title>F</title>
<item><title>T</title><description><![CDATA[{description}]]></description></item>
</channel></rss>"#
    );

    let feed = parse(xml.as_bytes()).unwrap();
    let summary = feed.entries[0].summary.as_deref().unwrap_or("");
    assert!(
        summary.contains("<th"),
        "60 unclosed <th> cells in one row must not trip the nesting-depth fallback, \
         got: {summary}"
    );
}
