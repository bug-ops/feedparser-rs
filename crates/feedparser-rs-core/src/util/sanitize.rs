//! HTML sanitization utilities
//!
//! This module provides functions for sanitizing HTML content to prevent XSS attacks
//! while preserving safe formatting.

use crate::ParserLimits;
use crate::types::{Content, Entry, FeedMeta, MimeType, ParsedFeed, TextConstruct, TextType};
use ammonia::Builder;
use std::collections::HashSet;
use std::sync::LazyLock;

/// Tags `sanitize_html` keeps; every other tag is stripped (its children are
/// kept, unwrapped).
const SAFE_TAGS: &[&str] = &[
    // Text formatting
    "a",
    "abbr",
    "acronym",
    "b",
    "cite",
    "code",
    "em",
    "i",
    "kbd",
    "mark",
    "s",
    "samp",
    "small",
    "strike",
    "strong",
    "sub",
    "sup",
    "u",
    "var", // Structural
    "br",
    "div",
    "hr",
    "p",
    "span", // Headings
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6", // Lists
    "dd",
    "dl",
    "dt",
    "li",
    "ol",
    "ul", // Tables
    "caption",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "tr", // Quotes
    "blockquote",
    "q",   // Pre-formatted
    "pre", // Media
    "img",
];

/// Shared, built-once sanitizer configuration.
///
/// Built lazily on first use (ammonia uses this same `LazyLock<Builder<'static>>`
/// pattern internally for its own default cleaner). Rebuilding the tag/attribute
/// `HashSet`s and `Builder` on every call was measured to cost ~12x on a typical
/// feed once `sanitize_feed` started calling this function dozens of times per
/// entry instead of zero (#438) — building it once amortizes that cost to nothing.
static SAFE_HTML_BUILDER: LazyLock<Builder<'static>> = LazyLock::new(|| {
    let safe_tags: HashSet<&'static str> = SAFE_TAGS.iter().copied().collect();

    let safe_attrs: HashSet<&'static str> = ["alt", "cite", "class", "href", "id", "src", "title"]
        .into_iter()
        .collect();

    let safe_url_schemes: HashSet<&'static str> = ["http", "https", "mailto"].into_iter().collect();

    let mut builder = Builder::default();
    builder
        .tags(safe_tags)
        .generic_attributes(safe_attrs)
        .link_rel(Some("nofollow noopener noreferrer"))
        .url_schemes(safe_url_schemes);
    builder
});

/// Sanitize HTML content, removing dangerous tags and attributes
///
/// This function uses ammonia to clean HTML content, allowing only safe tags
/// and attributes. It's designed to match feedparser's sanitization behavior.
///
/// # Performance
///
/// This is a low-level primitive: it always runs the input through ammonia's
/// HTML5 tree builder, which exhibits quadratic-time behavior on pathologically
/// deep tag nesting. Prefer [`sanitize_feed`] for parsed feed content — it
/// applies a nesting-depth bound (`ParserLimits::max_html_nesting_depth`) before
/// calling this function, falling back to plain-text escaping for input that
/// exceeds it.
///
/// # Arguments
///
/// * `input` - HTML string to sanitize
///
/// # Returns
///
/// Sanitized HTML string with dangerous content removed
///
/// # Examples
///
/// ```
/// use feedparser_rs::util::sanitize::sanitize_html;
///
/// let unsafe_html = r#"<p>Hello</p><script>alert('XSS')</script>"#;
/// let safe_html = sanitize_html(unsafe_html);
/// assert_eq!(safe_html, "<p>Hello</p>");
/// ```
pub fn sanitize_html(input: &str) -> String {
    SAFE_HTML_BUILDER.clean(input).to_string()
}

/// Decode HTML entities to Unicode characters
///
/// # Examples
///
/// ```
/// use feedparser_rs::util::sanitize::decode_entities;
///
/// assert_eq!(decode_entities("&lt;p&gt;Hello&lt;/p&gt;"), "<p>Hello</p>");
/// assert_eq!(decode_entities("&amp;amp;"), "&amp;");
/// ```
pub fn decode_entities(input: &str) -> String {
    html_escape::decode_html_entities(input).to_string()
}

/// Strip all HTML tags, leaving only text content
///
/// # Examples
///
/// ```
/// use feedparser_rs::util::sanitize::strip_tags;
///
/// assert_eq!(strip_tags("<p>Hello <b>world</b></p>"), "Hello world");
/// ```
pub fn strip_tags(input: &str) -> String {
    Builder::default()
        .tags(HashSet::new())
        .clean(input)
        .to_string()
}

/// Sanitize every HTML-bearing field of a parsed feed, in place.
///
/// This is the single enforcement point for `ParseOptions::sanitize_html`. Format
/// parsers populate ~85 fields across `FeedMeta` and `Entry` that can carry markup;
/// sanitizing only at the handful of `set_*` convenience helpers would miss most of
/// them (all of RSS 1.0, every `entry.content` push, `dc:*`/`media:*` fields). Walking
/// the fully parsed structure once, after all format-specific parsing has finished,
/// is the only way to cover every call site without duplicating sanitization logic
/// into each parser.
///
/// Fields are matched against Python feedparser's `can_contain_dangerous_markup` set:
/// `Tag.term`/`label`, `Person.name`, `Enclosure.title`, `comments`,
/// `slash_hit_parade`, `Generator.name`, and podcast free-text fields are
/// deliberately excluded, since they are not rendered as markup by consumers.
///
/// # Examples
///
/// ```
/// use feedparser_rs::{ParserLimits, parse, util::sanitize::sanitize_feed};
///
/// let xml = br#"<rss version="2.0"><channel><title>Feed</title>
///     <item><title>Post</title>
///     <description>&lt;script&gt;alert(1)&lt;/script&gt;Hi</description></item>
/// </channel></rss>"#;
/// let mut feed = parse(xml).unwrap();
/// sanitize_feed(&mut feed, &ParserLimits::default());
/// assert!(!feed.entries[0].summary.as_deref().unwrap_or("").contains("<script>"));
/// ```
pub fn sanitize_feed(feed: &mut ParsedFeed, limits: &ParserLimits) {
    sanitize_feed_meta(&mut feed.feed, limits);
    for entry in &mut feed.entries {
        sanitize_entry(entry, limits);
    }
}

/// Sanitize the HTML-bearing fields of `FeedMeta`.
fn sanitize_feed_meta(meta: &mut FeedMeta, limits: &ParserLimits) {
    sanitize_pair(&mut meta.title, &mut meta.title_detail, limits);
    sanitize_pair(&mut meta.subtitle, &mut meta.subtitle_detail, limits);
    sanitize_pair(&mut meta.summary, &mut meta.summary_detail, limits);
    sanitize_pair(&mut meta.rights, &mut meta.rights_detail, limits);
    sanitize_opt(&mut meta.dc_rights, limits);

    if let Some(image) = &mut meta.image {
        sanitize_opt(&mut image.title, limits);
        sanitize_opt(&mut image.description, limits);
    }
    if let Some(textinput) = &mut meta.textinput {
        sanitize_opt(&mut textinput.title, limits);
        sanitize_opt(&mut textinput.description, limits);
    }
    if let Some(itunes) = &mut meta.itunes {
        sanitize_opt(&mut itunes.subtitle, limits);
        sanitize_opt(&mut itunes.summary, limits);
    }
}

/// Sanitize the HTML-bearing fields of `Entry`.
fn sanitize_entry(entry: &mut Entry, limits: &ParserLimits) {
    sanitize_pair(&mut entry.title, &mut entry.title_detail, limits);
    sanitize_pair(&mut entry.subtitle, &mut entry.subtitle_detail, limits);
    sanitize_pair(&mut entry.summary, &mut entry.summary_detail, limits);
    sanitize_pair(&mut entry.rights, &mut entry.rights_detail, limits);
    sanitize_opt(&mut entry.dc_rights, limits);
    sanitize_opt(&mut entry.media_title, limits);
    sanitize_opt(&mut entry.media_description, limits);

    for content in &mut entry.content {
        sanitize_content(content, limits);
    }

    if let Some(source) = &mut entry.source {
        sanitize_opt(&mut source.title, limits);
        sanitize_opt(&mut source.rights, limits);
    }
    if let Some(itunes) = &mut entry.itunes {
        sanitize_opt(&mut itunes.title, limits);
        sanitize_opt(&mut itunes.subtitle, limits);
        sanitize_opt(&mut itunes.summary, limits);
    }
}

/// Sanitize a flat/detail field pair, fail-closed on the detail's declared type.
///
/// `TextType::Text` is the only type that skips sanitization. A missing detail
/// (`None`) is treated the same as `Html`/`Xhtml`: the parser could not establish
/// that the value is safe plain text, so it must not be trusted by default.
fn sanitize_pair(
    value: &mut Option<String>,
    detail: &mut Option<TextConstruct>,
    limits: &ParserLimits,
) {
    if matches!(
        detail.as_ref().map(|d| d.content_type),
        Some(TextType::Text)
    ) {
        return;
    }
    sanitize_opt(value, limits);
    if let Some(detail) = detail {
        detail.value = sanitize_html_bounded(&detail.value, limits.max_html_nesting_depth);
    }
}

/// Sanitize a `Content` block, fail-closed on its declared MIME type.
///
/// Only an explicit `text/plain` type skips sanitization; a missing or
/// unrecognized type (including `text/html`, `application/xhtml+xml`, and
/// anything else) is sanitized.
fn sanitize_content(content: &mut Content, limits: &ParserLimits) {
    if content
        .content_type
        .as_deref()
        .is_some_and(|t| t.eq_ignore_ascii_case(MimeType::TEXT_PLAIN))
    {
        return;
    }
    content.value = sanitize_html_bounded(&content.value, limits.max_html_nesting_depth);
}

/// Sanitize a plain string field that carries no type metadata.
///
/// These fields (`dc_rights`, `image.title`, iTunes free text, etc.) have no
/// signal to indicate they are markup-free, so they are always sanitized.
fn sanitize_opt(value: &mut Option<String>, limits: &ParserLimits) {
    if let Some(v) = value {
        *v = sanitize_html_bounded(v, limits.max_html_nesting_depth);
    }
}

/// Sanitize HTML, falling back to plain-text escaping when the input is nested
/// deeper than `max_depth`.
///
/// Ammonia's HTML5 tree builder exhibits quadratic-time behavior on
/// pathologically deep tag nesting within a single text field (verified:
/// hundreds-of-times slowdown on a single deeply nested `<div>` chain). Rather
/// than feed such input to ammonia unbounded, it is instead escaped as plain
/// text via `escape_html_plain`, which is O(n) regardless of nesting shape and
/// still guarantees no markup survives (#438).
fn sanitize_html_bounded(input: &str, max_depth: usize) -> String {
    if html_nesting_exceeds(input, max_depth) {
        escape_html_plain(input)
    } else {
        sanitize_html(input)
    }
}

/// HTML void elements — self-closing by the HTML5 spec, never nest. A gallery
/// of hundreds of `<img>`/`<br>` tags is not "deeply nested" and must not
/// trip the depth bound.
const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Elements HTML5 implicitly closes when another instance of the same tag
/// opens without an explicit end tag (e.g. `<li>a<li>b` auto-closes the first
/// `<li>`). A long run of unclosed instances — common in real feed content
/// (list items, table rows/cells) — must not accumulate depth.
const AUTO_CLOSE_ELEMENTS: &[&str] = &["li", "option", "p", "td", "th", "tr"];

/// Elements that act as a scope barrier per HTML5's "has an element in
/// scope" algorithm: a closing tag cannot pop through one of these to reach
/// a same-named ancestor further down the stack. `</div>` while a `<table>`
/// is still open inside it does not close the div — the table blocks the
/// search, exactly as html5ever's tree builder behaves, so both elements
/// stay genuinely open (and genuinely nested) until the table itself closes.
const SCOPE_BARRIERS: &[&str] = &[
    "table", "td", "th", "caption", "object", "marquee", "applet",
];

/// HTML5's "formatting elements" (the list used by the tree builder's
/// adoption-agency/reconstruction algorithm). Repeating one of these unclosed
/// does not create the pathologically deep, expensive-to-sanitize DOM that
/// repeating a structural element (`<div>`, `<table>`, `<section>`, ...)
/// does — verified empirically during review: `<b>`/`<font>` repeated 40,000
/// times unclosed sanitize in ~200ms (linear), while `<div>`/`<section>` at
/// the same scale take 30+ seconds (quadratic). Excluded from the depth
/// count for the same reason void elements are: they are not the hazard this
/// guard exists to catch (#438).
const FORMATTING_ELEMENTS: &[&str] = &[
    "a", "b", "big", "code", "em", "font", "i", "nobr", "s", "small", "strike", "strong", "tt", "u",
];

/// A field containing more than this many tags is treated as pathologically
/// nested regardless of what the depth stack reports. This is a cheap,
/// purely additive O(1) backstop, independent of how closely the tag-name
/// model above tracks html5ever's real semantics: legitimate feed content
/// never needs anywhere close to 10,000 tags in a single field, so bounding
/// total tag count here closes the whole class of "the depth heuristic
/// diverges from html5ever in some as-yet-undiscovered way" bugs, not just
/// the specific ones found so far (#438).
const MAX_TAGS_PER_FIELD: usize = 10_000;

/// Extract a tag's element name: the leading run of bytes up to the first
/// ASCII whitespace or `/` (attributes, or the trailing `/` of `<br/>`).
fn tag_name(inner: &[u8]) -> &[u8] {
    let end = inner
        .iter()
        .position(|b| b.is_ascii_whitespace() || *b == b'/')
        .unwrap_or(inner.len());
    &inner[..end]
}

fn contains_name_ci(names: &[&str], name: &[u8]) -> bool {
    names
        .iter()
        .any(|n| name.eq_ignore_ascii_case(n.as_bytes()))
}

/// Search the stack from the top for an element named `name`, stopping (and
/// reporting no match) if a [`SCOPE_BARRIERS`] element is encountered first
/// — mirrors html5ever's "has an element in scope" check.
fn find_in_scope(stack: &[&[u8]], name: &[u8]) -> Option<usize> {
    for (idx, tag) in stack.iter().enumerate().rev() {
        if tag.eq_ignore_ascii_case(name) {
            return Some(idx);
        }
        if contains_name_ci(SCOPE_BARRIERS, tag) {
            return None;
        }
    }
    None
}

/// Cheap, single-pass estimate of HTML open-tag nesting depth using a
/// name-matched tag stack.
///
/// Intentionally approximate — it does not build a full DOM or validate the
/// document, so it is not a substitute for ammonia's tree builder — but
/// unlike a naive open/close counter, it tracks tag *names*, so it cannot be
/// fooled by a run of mismatched closing tags (e.g. `("<div></x>")*n`, which
/// never actually closes any `<div>`, real HTML5 parsers included: an end
/// tag with no matching open element on the stack is simply ignored). It
/// also recognizes HTML void elements (`<br>`, `<img>`, ...), auto-closing
/// elements (`<li>`, `<p>`, ...), formatting elements (`<b>`, `<font>`, ...),
/// and scope-barrier elements (`<table>`, `<td>`, ...) so ordinary valid
/// HTML — image galleries, `<br>`-separated text, unclosed `<li>`/`<p>`
/// runs, tables, runs of unclosed inline formatting — is never misjudged as
/// pathologically nested, while genuinely deep nesting (including nesting
/// hidden behind a scope barrier) is never missed. An O(1) total-tag-count
/// backstop ([`MAX_TAGS_PER_FIELD`]) additionally bounds worst-case behavior
/// independent of how well the rest of this model matches html5ever (#438).
fn html_nesting_exceeds(html: &str, max_depth: usize) -> bool {
    let bytes = html.as_bytes();
    let mut stack: Vec<&[u8]> = Vec::new();
    let mut tag_count: usize = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let Some(rel_end) = bytes[i..].iter().position(|&b| b == b'>') else {
            break; // no closing '>' in the remainder: no more complete tags to scan
        };
        let inner = &bytes[i + 1..i + rel_end];
        i += rel_end + 1;

        if inner.first() == Some(&b'!') || inner.first() == Some(&b'?') {
            continue; // comment, doctype, or processing instruction: doesn't nest
        }

        tag_count += 1;
        if tag_count > MAX_TAGS_PER_FIELD {
            return true;
        }

        if inner.first() == Some(&b'/') {
            // Closing tag: only pops a *matching* open element (and anything
            // opened after it) that is actually in scope. An end tag with no
            // match in scope is ignored, exactly like html5ever's tree
            // builder — this is what keeps a run of bogus closing tags, or
            // one blocked by a scope barrier, from masking real nesting.
            let name = tag_name(&inner[1..]);
            if let Some(pos) = find_in_scope(&stack, name) {
                stack.truncate(pos);
            }
            continue;
        }

        let self_closing = inner.last() == Some(&b'/');
        let name = tag_name(if self_closing {
            &inner[..inner.len() - 1]
        } else {
            inner
        });

        if self_closing
            || contains_name_ci(VOID_ELEMENTS, name)
            || contains_name_ci(FORMATTING_ELEMENTS, name)
        {
            continue; // doesn't nest (or, for formatting elements, doesn't nest expensively)
        }

        if contains_name_ci(AUTO_CLOSE_ELEMENTS, name)
            && let Some(pos) = find_in_scope(&stack, name)
        {
            stack.truncate(pos);
        }

        stack.push(name);
        if stack.len() > max_depth {
            return true;
        }
    }
    false
}

/// Escape the characters that give HTML its structure, without parsing it.
///
/// O(n) regardless of input shape — the fallback used by `sanitize_html_bounded`
/// when `html_nesting_exceeds` rejects input as too deep to safely hand to
/// ammonia. No tag can survive this transform, so it is safe even though it
/// does not attempt to preserve any formatting.
fn escape_html_plain(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_removes_script() {
        let html = r"<p>Hello</p><script>alert('XSS')</script>";
        let clean = sanitize_html(html);
        assert!(!clean.contains("script"));
        assert!(clean.contains("Hello"));
    }

    #[test]
    fn test_sanitize_allows_safe_tags() {
        let html = r#"<p>Hello <b>world</b> <a href="http://example.com">link</a></p>"#;
        let clean = sanitize_html(html);
        assert!(clean.contains("<p>"));
        assert!(clean.contains("<b>"));
        assert!(clean.contains("<a"));
    }

    #[test]
    fn test_sanitize_removes_onclick() {
        let html = r#"<a href="/" onclick="alert('XSS')">Click</a>"#;
        let clean = sanitize_html(html);
        assert!(!clean.contains("onclick"));
        assert!(clean.contains("href"));
    }

    #[test]
    fn test_decode_entities() {
        assert_eq!(decode_entities("&lt;p&gt;"), "<p>");
        assert_eq!(decode_entities("&amp;"), "&");
        assert_eq!(decode_entities("&quot;"), "\"");
        assert_eq!(decode_entities("&#39;"), "'");
    }

    #[test]
    fn test_decode_numeric_entities() {
        assert_eq!(decode_entities("&#60;"), "<");
        assert_eq!(decode_entities("&#x3C;"), "<");
    }

    #[test]
    fn test_strip_tags() {
        let html = "<p>Hello <b>world</b></p>";
        assert_eq!(strip_tags(html), "Hello world");
    }

    #[test]
    fn test_xss_img_onerror() {
        let html = r#"<img src="x" onerror="alert('XSS')">"#;
        let clean = sanitize_html(html);
        assert!(!clean.contains("onerror"));
    }

    #[test]
    fn test_xss_javascript_url() {
        let html = r#"<a href="javascript:alert('XSS')">Click</a>"#;
        let clean = sanitize_html(html);
        assert!(!clean.contains("javascript:"));
    }

    #[test]
    fn test_xss_iframe() {
        let html = r#"<iframe src="http://evil.com"></iframe>"#;
        let clean = sanitize_html(html);
        assert!(!clean.contains("iframe"));
    }

    #[test]
    fn test_xss_data_url() {
        let html = r#"<a href="data:text/html,<script>alert('XSS')</script>">Click</a>"#;
        let clean = sanitize_html(html);
        assert!(!clean.contains("data:"));
    }

    #[test]
    fn test_sanitize_empty_string() {
        assert_eq!(sanitize_html(""), "");
    }

    #[test]
    fn test_sanitize_plain_text() {
        let text = "Plain text with no tags";
        assert_eq!(sanitize_html(text), text);
    }

    #[test]
    fn test_decode_entities_no_entities() {
        let text = "No entities here";
        assert_eq!(decode_entities(text), text);
    }

    #[test]
    fn test_strip_tags_nested() {
        let html = "<div><p>Hello <span><b>world</b></span></p></div>";
        assert_eq!(strip_tags(html), "Hello world");
    }

    #[test]
    fn test_sanitize_link_rel_attribute() {
        let html = r#"<a href="http://example.com">Link</a>"#;
        let clean = sanitize_html(html);
        assert!(clean.contains("nofollow"));
        assert!(clean.contains("noopener"));
        assert!(clean.contains("noreferrer"));
    }
}
