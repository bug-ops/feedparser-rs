//! Common parsing utilities shared between RSS and Atom parsers
//!
//! This module eliminates code duplication by providing shared functionality
//! for XML parsing operations used by both feed formats.

use crate::{
    ParserLimits,
    error::{FeedError, Result},
    namespace::{
        georss::{self, GeoLocation, GeoType},
        namespaces as ns_uris,
    },
    types::{
        Entry, FeedMeta, FeedVersion, ItunesEntryMeta, ItunesFeedMeta, MediaContent,
        MediaThumbnail, ParsedFeed, PodcastMeta,
    },
    util::text::truncate_to_length,
};
use quick_xml::{
    Reader,
    events::{BytesRef, BytesStart, Event},
};
use std::{collections::HashMap, io::Write as _};

pub use crate::types::{FromAttributes, LimitedCollectionExt};
pub use crate::util::text::bytes_to_string;

/// Initial capacity for XML event buffer (fits most elements)
pub const EVENT_BUFFER_CAPACITY: usize = 1024;

/// Initial capacity for text content (typical field size)
pub const TEXT_BUFFER_CAPACITY: usize = 256;

/// Creates a new event buffer with optimized capacity
///
/// This factory function provides a semantic way to create XML event buffers
/// with consistent capacity across all parsers. Using this instead of direct
/// `Vec::with_capacity()` calls makes it easier to tune buffer sizes in one place.
///
/// # Returns
///
/// A `Vec<u8>` pre-allocated with `EVENT_BUFFER_CAPACITY` (1024 bytes)
///
/// # Examples
///
/// ```ignore
/// use feedparser_rs::parser::common::new_event_buffer;
///
/// let mut buf = new_event_buffer();
/// assert!(buf.capacity() >= 1024);
/// ```
#[inline]
#[must_use]
#[allow(dead_code)] // Future use: Will be adopted when refactoring parsers
pub fn new_event_buffer() -> Vec<u8> {
    Vec::with_capacity(EVENT_BUFFER_CAPACITY)
}

/// Creates a new text buffer with optimized capacity
///
/// This factory function provides a semantic way to create text content buffers
/// with consistent capacity across all parsers. Useful for accumulating text
/// content from XML elements.
///
/// # Returns
///
/// A `String` pre-allocated with `TEXT_BUFFER_CAPACITY` (256 bytes)
///
/// # Examples
///
/// ```ignore
/// use feedparser_rs::parser::common::new_text_buffer;
///
/// let mut text = new_text_buffer();
/// assert!(text.capacity() >= 256);
/// ```
#[inline]
#[must_use]
#[allow(dead_code)] // Future use: Will be adopted when refactoring parsers
pub fn new_text_buffer() -> String {
    String::with_capacity(TEXT_BUFFER_CAPACITY)
}

/// Context for parsing operations
///
/// Bundles together common parsing state to reduce function parameter count.
/// Future use: Will be adopted when refactoring parsers to reduce parameter passing
#[allow(dead_code)]
pub struct ParseContext<'a> {
    /// XML reader
    pub reader: Reader<&'a [u8]>,
    /// Reusable buffer for XML events
    pub buf: Vec<u8>,
    /// Parser limits for validation
    pub limits: ParserLimits,
    /// Current nesting depth
    pub depth: usize,
}

impl<'a> ParseContext<'a> {
    /// Create a new parse context from raw data
    #[allow(dead_code)]
    pub fn new(data: &'a [u8], limits: ParserLimits) -> Result<Self> {
        limits
            .check_feed_size(data.len())
            .map_err(|e| FeedError::InvalidFormat(e.to_string()))?;

        let reader = Reader::from_reader(data);

        Ok(Self {
            reader,
            buf: Vec::with_capacity(EVENT_BUFFER_CAPACITY),
            limits,
            depth: 1, // Start at 1 for root element
        })
    }

    /// Check and increment depth, returning error if limit exceeded
    #[inline]
    #[allow(dead_code)]
    pub fn check_depth(&mut self) -> Result<()> {
        self.depth += 1;
        if self.depth > self.limits.max_nesting_depth {
            return Err(FeedError::InvalidFormat(format!(
                "XML nesting depth {} exceeds maximum {}",
                self.depth, self.limits.max_nesting_depth
            )));
        }
        Ok(())
    }

    /// Decrement depth safely
    #[inline]
    #[allow(dead_code)]
    pub const fn decrement_depth(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// Clear the buffer
    #[inline]
    #[allow(dead_code)]
    pub fn clear_buf(&mut self) {
        self.buf.clear();
    }
}

/// Initialize a `ParsedFeed` with common setup for any format
#[inline]
pub fn init_feed(version: FeedVersion, max_entries: usize) -> ParsedFeed {
    let mut feed = ParsedFeed::with_capacity(max_entries);
    feed.version = version;
    feed.encoding = String::from("utf-8");
    feed
}

/// Build a `summary_detail`-style `TextConstruct` from a content block.
///
/// Used when `entry.summary` is back-filled from `entry.content[0]` (RSS
/// `content:encoded`, Atom `<content>`) so the derived summary carries the
/// content block's declared type instead of losing it. Unlabeled or
/// unrecognized content types map to `Html` (fail-closed) via
/// `TextType::from_type_attr`.
#[inline]
pub fn text_construct_from_content(content: &crate::types::Content) -> crate::types::TextConstruct {
    use crate::types::TextType;

    crate::types::TextConstruct {
        value: content.value.clone(),
        content_type: content
            .content_type
            .as_deref()
            .map_or(TextType::Html, TextType::from_type_attr),
        language: content.language.clone(),
        base: content.base.clone(),
    }
}

/// Check nesting depth and return error if exceeded
///
/// This is a standalone helper for parsers that don't use `ParseContext`.
#[inline]
pub fn check_depth(depth: usize, max_depth: usize) -> Result<()> {
    if depth > max_depth {
        return Err(FeedError::InvalidFormat(format!(
            "XML nesting depth {depth} exceeds maximum {max_depth}"
        )));
    }
    Ok(())
}

/// Extract local name from namespaced element if prefix matches
///
/// Validates tag name contains only alphanumeric characters and hyphens
/// to prevent injection attacks.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(extract_ns_local_name(b"dc:creator", b"dc:"), Some("creator"));
/// assert_eq!(extract_ns_local_name(b"dc:creator", b"atom:"), None);
/// assert_eq!(extract_ns_local_name(b"dc:<script>", b"dc:"), None); // Invalid chars
/// ```
#[inline]
pub fn extract_ns_local_name<'a>(name: &'a [u8], prefix: &[u8]) -> Option<&'a str> {
    if name.starts_with(prefix) {
        let tag_name = std::str::from_utf8(&name[prefix.len()..]).ok()?;
        // Security: validate tag name (alphanumeric, hyphen, underscore only)
        if !tag_name.is_empty()
            && tag_name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            Some(tag_name)
        } else {
            None
        }
    } else {
        None
    }
}

/// Split a namespaced tag into `(prefix_bytes_with_colon, local_name_bytes)`.
///
/// Returns `None` if there is no colon separator or the local name is empty.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(split_ns_tag(b"dc:creator"), Some((b"dc:" as &[u8], b"creator" as &[u8])));
/// assert_eq!(split_ns_tag(b"title"), None); // no prefix
/// assert_eq!(split_ns_tag(b"dc:"), None);  // empty local name
/// ```
#[inline]
fn split_ns_tag(name: &[u8]) -> Option<(&[u8], &[u8])> {
    let colon = name.iter().position(|&b| b == b':')?;
    let local = &name[colon + 1..];
    if local.is_empty() {
        return None;
    }
    Some((&name[..=colon], local))
}

/// Match a tag against a namespace URI using the declared prefix mapping.
///
/// Resolves the tag prefix via `namespaces` (prefix → URI) and checks whether
/// the resolved URI equals `target_uri`. If it does, returns the local element
/// name (validated as alphanumeric/hyphen/underscore). Falls back to checking
/// the canonical hardcoded prefix when the resolved prefix is absent from the map.
#[inline]
fn match_ns_tag_by_uri<'a>(
    name: &'a [u8],
    canonical_prefix: &[u8],
    target_uri: &str,
    namespaces: &HashMap<String, String>,
) -> Option<&'a str> {
    // Fast path: canonical prefix (e.g. "dc:")
    if let Some(local) = extract_ns_local_name(name, canonical_prefix) {
        return Some(local);
    }
    // URI-based fallback: find any declared prefix that maps to target_uri
    let (prefix_with_colon, _) = split_ns_tag(name)?;
    let prefix_str = std::str::from_utf8(&prefix_with_colon[..prefix_with_colon.len() - 1]).ok()?;
    if namespaces.get(prefix_str).map(String::as_str) == Some(target_uri) {
        extract_ns_local_name(name, prefix_with_colon)
    } else {
        None
    }
}

/// Check if element is a Dublin Core namespaced tag.
///
/// Recognises the canonical `dc:` prefix and any custom prefix declared with
/// `xmlns:X="http://purl.org/dc/elements/1.1/"`.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(is_dc_tag(b"dc:creator", &namespaces), Some("creator"));
/// assert_eq!(is_dc_tag(b"dublin:creator", &namespaces_with_dublin_uri), Some("creator"));
/// assert_eq!(is_dc_tag(b"content:encoded", &namespaces), None);
/// ```
#[inline]
pub fn is_dc_tag<'a>(name: &'a [u8], namespaces: &HashMap<String, String>) -> Option<&'a str> {
    match_ns_tag_by_uri(name, b"dc:", ns_uris::DUBLIN_CORE, namespaces)
}

/// Check if element is a Content namespaced tag
///
/// # Examples
///
/// ```ignore
/// assert_eq!(is_content_tag(b"content:encoded"), Some("encoded"));
/// assert_eq!(is_content_tag(b"dc:creator"), None);
/// ```
#[inline]
pub fn is_content_tag(name: &[u8]) -> Option<&str> {
    extract_ns_local_name(name, b"content:")
}

/// Check if element is a Syndication namespaced tag
///
/// Recognizes both `sy:` (RSS 1.0 spec convention) and `syn:` prefixes,
/// as both map to `http://purl.org/rss/1.0/modules/syndication/`.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(is_syn_tag(b"syn:updatePeriod"), Some("updatePeriod"));
/// assert_eq!(is_syn_tag(b"sy:updatePeriod"), Some("updatePeriod"));
/// assert_eq!(is_syn_tag(b"syn:updateFrequency"), Some("updateFrequency"));
/// assert_eq!(is_syn_tag(b"dc:creator"), None);
/// ```
#[inline]
pub fn is_syn_tag(name: &[u8]) -> Option<&str> {
    extract_ns_local_name(name, b"sy:").or_else(|| extract_ns_local_name(name, b"syn:"))
}

/// Check if element is a Media RSS namespaced tag
///
/// # Examples
///
/// ```ignore
/// assert_eq!(is_media_tag(b"media:thumbnail", &namespaces), Some("thumbnail"));
/// assert_eq!(is_media_tag(b"mrss:content", &namespaces_with_mrss_uri), Some("content"));
/// assert_eq!(is_media_tag(b"dc:creator", &namespaces), None);
/// ```
#[inline]
pub fn is_media_tag<'a>(name: &'a [u8], namespaces: &HashMap<String, String>) -> Option<&'a str> {
    match_ns_tag_by_uri(name, b"media:", ns_uris::MEDIA, namespaces)
}

/// Check if element is a Slash namespaced tag
///
/// # Examples
///
/// ```ignore
/// assert_eq!(is_slash_tag(b"slash:comments"), Some("comments"));
/// assert_eq!(is_slash_tag(b"dc:creator"), None);
/// ```
#[inline]
pub fn is_slash_tag(name: &[u8]) -> Option<&str> {
    extract_ns_local_name(name, b"slash:")
}

/// Check if element is a WFW namespaced tag
///
/// # Examples
///
/// ```ignore
/// assert_eq!(is_wfw_tag(b"wfw:commentRss"), Some("commentRss"));
/// assert_eq!(is_wfw_tag(b"dc:creator"), None);
/// ```
#[inline]
pub fn is_wfw_tag(name: &[u8]) -> Option<&str> {
    extract_ns_local_name(name, b"wfw:")
}

/// Check if element is a `GeoRSS` namespaced tag
///
/// # Examples
///
/// ```ignore
/// assert_eq!(is_georss_tag(b"georss:point"), Some("point"));
/// assert_eq!(is_georss_tag(b"georss:line"), Some("line"));
/// assert_eq!(is_georss_tag(b"dc:creator"), None);
/// ```
#[inline]
pub fn is_georss_tag(name: &[u8]) -> Option<&str> {
    extract_ns_local_name(name, b"georss:")
}

/// Check if element is a GML (Geography Markup Language) namespaced tag
///
/// Used to recognize `gml:Point`, `gml:LineString`, `gml:pos`, etc. nested
/// inside `georss:where` for the `GeoRSS` GML profile.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(is_gml_tag(b"gml:Point"), Some("Point"));
/// assert_eq!(is_gml_tag(b"georss:point"), None);
/// ```
#[inline]
pub fn is_gml_tag(name: &[u8]) -> Option<&str> {
    extract_ns_local_name(name, b"gml:")
}

/// Check if element is a W3C Basic Geo namespaced tag
///
/// # Examples
///
/// ```ignore
/// assert_eq!(is_geo_tag(b"geo:lat"), Some("lat"));
/// assert_eq!(is_geo_tag(b"geo:long"), Some("long"));
/// assert_eq!(is_geo_tag(b"dc:creator"), None);
/// ```
#[inline]
pub fn is_geo_tag(name: &[u8]) -> Option<&str> {
    extract_ns_local_name(name, b"geo:")
}

/// Check if element is a Threading (thr:) namespaced tag
///
/// # Examples
///
/// ```ignore
/// assert_eq!(is_thr_tag(b"thr:in-reply-to"), Some("in-reply-to"));
/// assert_eq!(is_thr_tag(b"thr:total"), Some("total"));
/// assert_eq!(is_thr_tag(b"dc:creator"), None);
/// ```
#[inline]
pub fn is_thr_tag(name: &[u8]) -> Option<&str> {
    extract_ns_local_name(name, b"thr:")
}

/// Check if element matches an iTunes namespace tag.
///
/// Resolves the canonical `itunes:` prefix and any custom prefix declared with
/// `xmlns:X="http://www.itunes.com/dtds/podcast-1.0.dtd"`. Also supports
/// unprefixed element names for compatibility with non-compliant feeds.
/// Local name is validated (alphanumeric, hyphen, underscore only) via
/// `match_ns_tag_by_uri`.
///
/// # Examples
///
/// ```ignore
/// assert!(is_itunes_tag(b"itunes:author", b"author", &namespaces));
/// assert!(is_itunes_tag(b"author", b"author", &namespaces)); // Fallback for non-prefixed
/// assert!(!is_itunes_tag(b"itunes:title", b"author", &namespaces));
/// ```
#[inline]
pub fn is_itunes_tag(name: &[u8], tag: &[u8], namespaces: &HashMap<String, String>) -> bool {
    // Prefixed: resolve via URI and validate local name via match_ns_tag_by_uri
    if let Some(local) = match_ns_tag_by_uri(name, b"itunes:", ns_uris::ITUNES, namespaces) {
        return local.as_bytes() == tag;
    }
    // Fallback for feeds without prefix: compare directly (no colon means no namespace)
    if !name.contains(&b':') {
        return name == tag;
    }
    false
}

/// Extract xml:base attribute from element
///
/// Returns the base URL string if xml:base attribute exists.
/// Respects `max_attribute_length` limit for `DoS` protection.
///
/// # Arguments
///
/// * `element` - The XML element to extract xml:base from
/// * `max_attr_length` - Maximum allowed attribute length (`DoS` protection)
///
/// # Returns
///
/// * `Some(String)` - The xml:base value if found and within length limit
/// * `None` - If attribute not found or exceeds length limit
///
/// # Examples
///
/// ```ignore
/// use feedparser_rs::parser::common::extract_xml_base;
///
/// let element = /* BytesStart from quick-xml */;
/// if let Some(base) = extract_xml_base(&element, 1024) {
///     println!("Base URL: {}", base);
/// }
/// ```
pub fn extract_xml_base(
    element: &quick_xml::events::BytesStart,
    max_attr_length: usize,
) -> Option<String> {
    element
        .attributes()
        .flatten()
        .find(|attr| {
            let key = attr.key.as_ref();
            key == b"xml:base" || key == b"base"
        })
        .filter(|attr| attr.value.len() <= max_attr_length)
        .and_then(|attr| {
            attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .ok()
        })
        .map(|s| s.to_string())
}

/// Extract xml:lang attribute from element
///
/// Returns the language code if xml:lang or lang attribute exists.
/// Respects `max_attribute_length` limit for `DoS` protection.
///
/// # Arguments
///
/// * `element` - The XML element to extract xml:lang from
/// * `max_attr_length` - Maximum allowed attribute length (`DoS` protection)
///
/// # Returns
///
/// * `Some(String)` - The xml:lang value if found and within length limit
/// * `None` - If attribute not found or exceeds length limit
///
/// # Examples
///
/// ```ignore
/// use feedparser_rs::parser::common::extract_xml_lang;
///
/// let element = /* BytesStart from quick-xml */;
/// if let Some(lang) = extract_xml_lang(&element, 1024) {
///     println!("Language: {}", lang);
/// }
/// ```
pub fn extract_xml_lang(
    element: &quick_xml::events::BytesStart,
    max_attr_length: usize,
) -> Option<String> {
    element
        .attributes()
        .flatten()
        .find(|attr| {
            let key = attr.key.as_ref();
            key == b"xml:lang" || key == b"lang"
        })
        .filter(|attr| attr.value.len() <= max_attr_length)
        .and_then(|attr| {
            attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .ok()
        })
        .map(|s| s.to_string())
}

/// Extract XML namespace declarations from element attributes.
///
/// Collects all `xmlns` and `xmlns:prefix` attributes from the element and inserts
/// them into `feed.namespaces`. Follows Python feedparser key format:
/// - `xmlns="URI"` → key `""` (empty string for default namespace)
/// - `xmlns:dc="URI"` → key `"dc"` (prefix without colon)
///
/// Sets `feed.bozo = true` when:
/// - The number of namespaces exceeds `limits.max_namespaces`
/// - A namespace URI exceeds `limits.max_attribute_length`
/// - A namespace attribute value cannot be unescaped or decoded
///
/// # Arguments
///
/// * `element` - The XML element containing xmlns attributes
/// * `feed` - The feed to populate namespaces into
/// * `limits` - Parser limits for `DoS` protection
pub fn extract_namespaces(
    element: &quick_xml::events::BytesStart,
    feed: &mut ParsedFeed,
    limits: &ParserLimits,
) {
    for result in element.attributes() {
        let Ok(attr) = result else { continue };

        let key = attr.key.as_ref();

        // Determine namespace prefix: "" for xmlns, "prefix" for xmlns:prefix
        let prefix = if key == b"xmlns" {
            String::new()
        } else if let Some(suffix) = key.strip_prefix(b"xmlns:") {
            if let Ok(s) = std::str::from_utf8(suffix) {
                s.to_string()
            } else {
                feed.bozo = true;
                feed.bozo_exception = Some("Namespace prefix contains invalid UTF-8".to_string());
                continue;
            }
        } else {
            continue;
        };

        if attr.value.len() > limits.max_attribute_length {
            feed.bozo = true;
            feed.bozo_exception = Some(format!(
                "Namespace URI exceeds maximum attribute length of {} bytes",
                limits.max_attribute_length
            ));
            continue;
        }

        let uri = if let Ok(v) = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0) {
            v.to_string()
        } else {
            feed.bozo = true;
            feed.bozo_exception = Some("Malformed namespace URI value".to_string());
            continue;
        };

        if feed.namespaces.len() >= limits.max_namespaces {
            feed.bozo = true;
            feed.bozo_exception = Some(format!(
                "Namespace limit exceeded: {}",
                limits.max_namespaces
            ));
            break;
        }

        feed.namespaces.insert(prefix, uri);
    }
}

/// Read text content from current XML element (handles text and CDATA).
///
/// Returns `(text, had_bozo)` where `had_bozo` is `true` if any unresolved
/// entity references were encountered. Unlike feedparser-py (which treats all
/// entities atomically and fails if any one fails), this implementation resolves
/// each entity independently and signals bozo per-entity. This is a known deviation.
pub fn read_text(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
) -> Result<(String, bool)> {
    let mut text = String::with_capacity(TEXT_BUFFER_CAPACITY);
    let mut had_bozo = false;

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Text(e)) => {
                append_bytes(&mut text, e.as_ref(), limits.max_text_length)?;
            }
            Ok(Event::CData(e)) => {
                append_bytes(&mut text, e.as_ref(), limits.max_text_length)?;
            }
            Ok(Event::GeneralRef(e)) => {
                let (resolved, is_bozo) = resolve_entity(&e);
                had_bozo |= is_bozo;
                append_bytes(&mut text, resolved.as_bytes(), limits.max_text_length)?;
            }
            Ok(Event::End(_) | Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    let trimmed = text.trim().replace('\0', "");
    Ok((trimmed, had_bozo))
}

/// Resolve a general entity reference (numeric or named) to `(string, is_bozo)`.
/// Returns `true` for `is_bozo` when the entity is unknown or invalid.
fn resolve_entity(e: &BytesRef<'_>) -> (String, bool) {
    // Try numeric character references first: &#038; &#x26; etc.
    match e.resolve_char_ref() {
        Ok(Some(ch)) => return (ch.to_string(), false),
        Ok(None) => {} // Not a numeric reference; fall through to named entities.
        Err(_) => {
            // Invalid character reference — preserve as-is (bozo condition).
            let name = String::from_utf8_lossy(e.as_ref());
            return (format!("&{name};"), true);
        }
    }
    // These are the only 5 allowed XML named entities
    match e.as_ref() {
        b"amp" => ("&".to_string(), false),
        b"lt" => ("<".to_string(), false),
        b"gt" => (">".to_string(), false),
        b"quot" => ("\"".to_string(), false),
        b"apos" => ("'".to_string(), false),
        other => {
            // Unknown entity — preserve as-is (bozo condition).
            let name = String::from_utf8_lossy(other).into_owned();
            (format!("&{name};"), true)
        }
    }
}

#[inline]
fn append_bytes(text: &mut String, bytes: &[u8], max_len: usize) -> Result<()> {
    if text.len() + bytes.len() > max_len {
        return Err(FeedError::InvalidFormat(format!(
            "Text field exceeds maximum length of {max_len} bytes"
        )));
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => text.push_str(s),
        Err(_) => text.push_str(&String::from_utf8_lossy(bytes)),
    }
    Ok(())
}

/// Skip unknown element and all its children (enforces nesting depth limits)
pub fn skip_element(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
    current_depth: usize,
) -> Result<()> {
    let mut local_depth: usize = 1;

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(_)) => {
                local_depth += 1;
                if current_depth + local_depth > limits.max_nesting_depth {
                    return Err(FeedError::InvalidFormat(format!(
                        "XML nesting depth exceeds maximum of {}",
                        limits.max_nesting_depth
                    )));
                }
            }
            Ok(Event::End(_)) => {
                local_depth = local_depth.saturating_sub(1);
                if local_depth == 0 {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(())
}

/// Parse a `<georss:where>` element (`GeoRSS` GML profile) and return the
/// geometry it contains (or `None` if no recognized geometry was found)
/// together with whether an entity-resolution error occurred in the
/// coordinate text (the "bozo" pattern; the caller should OR this into its
/// own bozo tracking, matching how `GeoRSS` Simple elements are handled).
///
/// Recognizes `gml:Point`/`gml:pos`, `gml:LineString`/`gml:posList`,
/// `gml:Polygon` wrapping `gml:exterior`/`gml:LinearRing`/`gml:posList`,
/// `gml:MultiSurface` wrapping `gml:surfaceMember`/`gml:Polygon`, and
/// `gml:Envelope`/`gml:lowerCorner`+`gml:upperCorner`. The `srsName` and
/// `srsDimension` attributes on the geometry element drive axis-order
/// normalization and 3D coordinate handling (see
/// [`georss::build_gml_geometry`] and [`georss::build_gml_envelope`]). For
/// `gml:Envelope`, `srsDimension` on `gml:lowerCorner`/`gml:upperCorner`
/// takes precedence over the root element's value.
/// Unrecognized nested elements are skipped tolerantly; malformed
/// coordinate text yields `None` without aborting the parse.
///
/// `depth` must already account for the `<georss:where>` start tag itself,
/// per the depth-accounting convention used throughout this module (the
/// caller increments depth before dispatching).
pub fn parse_georss_where(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
    depth: usize,
) -> Result<(Option<GeoLocation>, bool)> {
    let mut geometry = None;
    let mut had_bozo = false;

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) => {
                let next_depth = depth + 1;
                check_depth(next_depth, limits.max_nesting_depth)?;

                if geometry.is_some() {
                    skip_element(reader, buf, limits, next_depth)?;
                } else if let Some((geo_type, end_tag, srs_name, dims)) =
                    gml_geometry_start(&e, limits)
                {
                    if geo_type == GeoType::Box {
                        let (corners, bozo, dims) = find_gml_envelope_corners(
                            reader, buf, limits, next_depth, &end_tag, dims,
                        )?;
                        had_bozo |= bozo;
                        geometry = corners.and_then(|(lower, upper)| {
                            georss::build_gml_envelope(srs_name, &lower, &upper, dims)
                        });
                    } else {
                        let (coord_text, bozo) =
                            find_gml_coord_text(reader, buf, limits, next_depth, &end_tag)?;
                        had_bozo |= bozo;
                        geometry = coord_text.and_then(|text| {
                            georss::build_gml_geometry(geo_type, srs_name, &text, dims)
                        });
                    }
                } else {
                    skip_element(reader, buf, limits, next_depth)?;
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"where" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok((geometry, had_bozo))
}

/// Extract the geometry type, matching end-tag local name, `srsName`, and
/// `srsDimension` (default `2`, non-numeric falls back to `2`) from a
/// `<gml:Point>`/`<gml:LineString>`/`<gml:Polygon>` start tag. Attribute
/// names are matched case-insensitively (real-world feeds vary), and
/// `srsName` is trimmed (XML attribute-value normalization of a
/// line-wrapped attribute does not strip leading/trailing whitespace).
/// Returns `None` for any other element (the caller should skip it).
fn gml_geometry_start(
    start: &BytesStart<'_>,
    limits: &ParserLimits,
) -> Option<(GeoType, Vec<u8>, Option<String>, usize)> {
    let name = start.name();
    let local = is_gml_tag(name.as_ref())?;
    let geo_type = match local {
        "Point" => GeoType::Point,
        "LineString" => GeoType::Line,
        "Polygon" | "MultiSurface" => GeoType::Polygon,
        "Envelope" => GeoType::Box,
        _ => return None,
    };

    let srs_name = gml_attr(start, b"srsName", limits.max_attribute_length)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let dims = gml_attr(start, b"srsDimension", limits.max_attribute_length)
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(2);

    Some((geo_type, local.as_bytes().to_vec(), srs_name, dims))
}

/// Read an attribute's normalized value by case-insensitive key match.
fn gml_attr(start: &BytesStart<'_>, key: &[u8], max_len: usize) -> Option<String> {
    start
        .attributes()
        .flatten()
        .find(|a| a.key.as_ref().eq_ignore_ascii_case(key))
        .filter(|a| a.value.len() <= max_len)
        .and_then(|a| a.normalized_value(quick_xml::XmlVersion::Implicit1_0).ok())
        .map(|v| v.to_string())
}

/// Recursively search a GML geometry subtree for the first `gml:pos` or
/// `gml:posList` element, transparently descending through wrapper
/// elements (e.g. `gml:exterior`/`gml:LinearRing`) or skipping unrecognized
/// nested content. Consumes events up through the matching `end_tag` close
/// tag, so the caller's depth accounting stays balanced. Returns the
/// coordinate text (if found) together with whether an entity-resolution
/// error occurred while reading it.
///
/// For `gml:MultiSurface`, this only returns the first `gml:surfaceMember`
/// with parseable coordinates — `GeoLocation` has no multi-geometry slot, so
/// subsequent members are parsed (to keep depth accounting balanced) but
/// their coordinates are discarded.
fn find_gml_coord_text(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
    depth: usize,
    end_tag: &[u8],
) -> Result<(Option<String>, bool)> {
    let mut found = None;
    let mut had_bozo = false;

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) => {
                let next_depth = depth + 1;
                check_depth(next_depth, limits.max_nesting_depth)?;
                let name = e.name();
                let local = is_gml_tag(name.as_ref()).map(str::to_owned);

                if found.is_none() && matches!(local.as_deref(), Some("pos" | "posList")) {
                    let (text, bozo) = read_text(reader, buf, limits)?;
                    had_bozo |= bozo;
                    found = Some(text);
                } else if let Some(local) = local {
                    let (inner, bozo) =
                        find_gml_coord_text(reader, buf, limits, next_depth, local.as_bytes())?;
                    had_bozo |= bozo;
                    found = found.or(inner);
                } else {
                    skip_element(reader, buf, limits, next_depth)?;
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == end_tag => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok((found, had_bozo))
}

/// `(lower_corner, upper_corner)` raw coordinate text, whether an
/// entity-resolution error occurred, and the resolved `srsDimension`.
type EnvelopeCorners = (Option<(String, String)>, bool, usize);

/// Search a `gml:Envelope` subtree for its `gml:lowerCorner` and
/// `gml:upperCorner` children, returning their raw coordinate text together
/// with the resolved `srsDimension`. Unrecognized nested content is skipped
/// tolerantly. Consumes events up through the matching `end_tag` close tag,
/// so the caller's depth accounting stays balanced. Returns `(None, _, _)`
/// if either corner is missing; malformed corner text is validated later by
/// [`georss::build_gml_envelope`].
///
/// Per the GML spec, `srsDimension` may be specified on `gml:lowerCorner`/
/// `gml:upperCorner` themselves rather than only on the `gml:Envelope` root
/// — a common real-world placement. A dimension found on either corner
/// element takes precedence over `root_dims` (the value read from the
/// `gml:Envelope` start tag).
fn find_gml_envelope_corners(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
    depth: usize,
    end_tag: &[u8],
    root_dims: usize,
) -> Result<EnvelopeCorners> {
    let mut lower = None;
    let mut upper = None;
    let mut dims = root_dims;
    let mut had_bozo = false;

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) => {
                let next_depth = depth + 1;
                check_depth(next_depth, limits.max_nesting_depth)?;
                let name = e.name();
                let local = is_gml_tag(name.as_ref());
                let corner_dims = gml_attr(&e, b"srsDimension", limits.max_attribute_length)
                    .and_then(|v| v.trim().parse::<usize>().ok());

                match local {
                    Some("lowerCorner") if lower.is_none() => {
                        dims = corner_dims.unwrap_or(dims);
                        let (text, bozo) = read_text(reader, buf, limits)?;
                        had_bozo |= bozo;
                        lower = Some(text);
                    }
                    Some("upperCorner") if upper.is_none() => {
                        dims = corner_dims.unwrap_or(dims);
                        let (text, bozo) = read_text(reader, buf, limits)?;
                        had_bozo |= bozo;
                        upper = Some(text);
                    }
                    _ => skip_element(reader, buf, limits, next_depth)?,
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == end_tag => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok((lower.zip(upper), had_bozo, dims))
}

/// Read xhtml content from the current element, per RFC 4287 §3.1.1.3.
///
/// The outer `<div xmlns="http://www.w3.org/1999/xhtml">` wrapper is stripped;
/// its inner XML content is serialized back to a string preserving all markup.
/// On any parse error, returns whatever content was collected so far (bozo pattern).
///
/// Returns `(content, had_bozo)` where `had_bozo` is `true` if the expected `<div>`
/// wrapper was missing (malformed xhtml). Bozo propagation to the feed level is not
/// yet implemented at entry-field level; see issue #70.
///
/// # Errors
///
/// Returns `Err` only if the collected content exceeds `limits.max_text_length`.
pub fn read_xhtml_content(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
) -> Result<(String, bool)> {
    // Skip the outer <div> wrapper (RFC 4287 §3.1.1.3 requires it to be removed).
    // We must consume the first Start event (the div) before serializing children.
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(_)) => break, // found the div wrapper; start collecting its children
            Ok(Event::End(_) | Event::Eof) | Err(_) => return Ok((String::new(), true)),
            _ => {}
        }
        buf.clear();
    }
    buf.clear();

    serialize_inner_xml(reader, buf, limits).map(|s| (s, false))
}

/// Read xhtml content, discarding the bozo signal.
///
/// Use this at call sites where `ParsedFeed` is not in scope and bozo
/// propagation to the feed level is not yet implemented, such as entry-level
/// fields. See issue #70.
#[inline]
pub fn read_xhtml_content_str(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
) -> Result<String> {
    read_xhtml_content(reader, buf, limits).map(|(s, _)| s)
}

/// Serialize inner XML content of the current element to a string.
///
/// Reads events until the matching closing tag (depth 0) and writes each event
/// as raw XML. On error, returns whatever was collected so far.
fn serialize_inner_xml(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
) -> Result<String> {
    let mut output: Vec<u8> = Vec::with_capacity(TEXT_BUFFER_CAPACITY);
    let max_len = limits.max_text_length;
    {
        let mut writer = quick_xml::Writer::new(&mut output);
        let mut depth: usize = 0;

        loop {
            match reader.read_event_into(buf) {
                Ok(Event::Start(e)) => {
                    depth += 1;
                    let _ = writer.write_event(Event::Start(e));
                }
                Ok(Event::End(e)) => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                    let _ = writer.write_event(Event::End(e));
                }
                Ok(Event::Text(e)) => {
                    let _ = writer.write_event(Event::Text(e));
                }
                Ok(Event::GeneralRef(e)) => {
                    // #316: &apos; and &quot; must decode to literal characters.
                    // &amp;, &lt;, &gt; must remain escaped in HTML output.
                    // All other entity refs are re-emitted verbatim.
                    let inner = writer.get_mut();
                    match e.as_ref() {
                        b"apos" => {
                            let _ = inner.write_all(b"'");
                        }
                        b"quot" => {
                            let _ = inner.write_all(b"\"");
                        }
                        _ => {
                            let _ = inner.write_all(b"&");
                            let _ = inner.write_all(e.as_ref());
                            let _ = inner.write_all(b";");
                        }
                    }
                }
                Ok(Event::CData(e)) => {
                    let _ = writer.write_event(Event::CData(e));
                }
                Ok(Event::Empty(e)) => {
                    let _ = writer.write_event(Event::Empty(e));
                }
                Ok(Event::Comment(e)) => {
                    let _ = writer.write_event(Event::Comment(e));
                }
                Ok(Event::Eof) | Err(_) => break,
                _ => {}
            }
            buf.clear();
        }
    } // writer dropped here, releasing borrow on output

    if output.len() > max_len {
        return Err(FeedError::InvalidFormat(format!(
            "XHTML content exceeds maximum length of {max_len} bytes"
        )));
    }

    let result = String::from_utf8_lossy(&output).trim().to_string();
    Ok(result)
}

/// Read text content, discarding the bozo signal.
///
/// Use this at call sites where `ParsedFeed` is not in scope and bozo
/// propagation to the feed level is not yet implemented, such as entry-level
/// fields (title, summary, content, author, etc.). See #70.
#[inline]
pub fn read_text_str(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
) -> Result<String> {
    read_text(reader, buf, limits).map(|(t, _)| t)
}

/// Get or create the feed-level iTunes metadata block.
#[inline]
pub fn itunes_feed_meta(feed: &mut FeedMeta) -> &mut ItunesFeedMeta {
    feed.itunes
        .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()))
}

/// Get or create the entry-level iTunes metadata block.
#[inline]
pub fn itunes_entry_meta(entry: &mut Entry) -> &mut ItunesEntryMeta {
    entry
        .itunes
        .get_or_insert_with(|| Box::new(ItunesEntryMeta::default()))
}

/// Get or create the feed-level Podcast 2.0 metadata block.
#[inline]
pub fn podcast_feed_meta(feed: &mut FeedMeta) -> &mut PodcastMeta {
    feed.podcast
        .get_or_insert_with(|| Box::new(PodcastMeta::default()))
}

/// Find an attribute value by key in a collected `(key, value)` attribute slice.
#[inline]
pub fn find_attribute<'a>(attrs: &'a [(Vec<u8>, String)], key: &[u8]) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k.as_slice() == key)
        .map(|(_, v)| v.as_str())
}

/// Build a `media:thumbnail` value from its collected attributes.
///
/// Returns `None` when the `url` attribute is missing or empty, matching the
/// `if !url.is_empty()` guard used at every call site.
pub fn build_media_thumbnail(
    attrs: &[(Vec<u8>, String)],
    limits: &ParserLimits,
) -> Option<MediaThumbnail> {
    let url = find_attribute(attrs, b"url")
        .map(|v| truncate_to_length(v, limits.max_attribute_length))
        .unwrap_or_default();
    if url.is_empty() {
        return None;
    }
    Some(MediaThumbnail {
        url: url.into(),
        width: find_attribute(attrs, b"width").map(str::to_owned),
        height: find_attribute(attrs, b"height").map(str::to_owned),
        time: find_attribute(attrs, b"time").map(str::to_owned),
    })
}

/// Build a `media:content` value from its collected attributes.
///
/// Returns `None` when the `url` attribute is missing or empty, matching the
/// `if !url.is_empty()` guard used at every call site.
pub fn build_media_content(
    attrs: &[(Vec<u8>, String)],
    limits: &ParserLimits,
) -> Option<MediaContent> {
    let url = find_attribute(attrs, b"url")
        .map(|v| truncate_to_length(v, limits.max_attribute_length))
        .unwrap_or_default();
    if url.is_empty() {
        return None;
    }
    Some(MediaContent {
        url: url.into(),
        content_type: find_attribute(attrs, b"type")
            .map(|v| truncate_to_length(v, limits.max_attribute_length))
            .map(Into::into),
        medium: find_attribute(attrs, b"medium")
            .map(|v| truncate_to_length(v, limits.max_attribute_length)),
        filesize: find_attribute(attrs, b"fileSize").map(str::to_owned),
        duration: find_attribute(attrs, b"duration").map(str::to_owned),
        width: find_attribute(attrs, b"width").map(str::to_owned),
        height: find_attribute(attrs, b"height").map(str::to_owned),
        bitrate: find_attribute(attrs, b"bitrate").map(str::to_owned),
        lang: find_attribute(attrs, b"lang").map(str::to_owned),
        channels: find_attribute(attrs, b"channels").map(str::to_owned),
        codec: find_attribute(attrs, b"codec").map(str::to_owned),
        expression: find_attribute(attrs, b"expression").map(str::to_owned),
        isdefault: find_attribute(attrs, b"isDefault").map(str::to_owned),
        samplingrate: find_attribute(attrs, b"samplingrate").map(str::to_owned),
        framerate: find_attribute(attrs, b"framerate").map(str::to_owned),
    })
}

/// Skip to end of specified element (for attribute-only elements like `<link>`)
pub fn skip_to_end(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>, tag: &[u8]) -> Result<()> {
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::End(e)) if e.local_name().as_ref() == tag => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

/// Defensive cap on the number of events [`skip_to_end_qualified`] will read while
/// draining. A malformed document could in principle make `read_event_into` return
/// `Err` repeatedly without ever advancing to a matching `End` or `Eof`; since that
/// function must never propagate an error (it is only ever called from an
/// error-recovery path that already gave up on strict parsing), an unbounded loop
/// would turn a would-be abort into a hang, which is a strictly worse failure mode.
/// This is generous enough to never trigger on any legitimate document — feeds are
/// already bounded by `ParserLimits::max_feed_size_bytes` well before this many
/// events could occur — so it only protects against the pathological case.
const MAX_DRAIN_EVENTS: u32 = 1_000_000;

/// Drain the reader to the closing tag matching `tag` (fully-qualified name,
/// including any namespace prefix), or `Eof`.
///
/// Used to resynchronize the reader after a field-level parse error is caught
/// and swallowed as bozo instead of propagated: without draining, any
/// leftover children of the failing element (e.g. `<textInput>`'s own
/// `<title>`) would be read by the *parent* loop next and could be
/// misdispatched as if they were siblings of the failing element, silently
/// corrupting unrelated fields. Tolerates further malformed XML while
/// draining — never propagates an error — so it is always safe to call from
/// an error-recovery path without risking another unbounded abort.
///
/// Tracks nesting via `Start`/`End` events that match `tag` by name, so a
/// same-named descendant (e.g. `<textInput>` containing another `<textInput>`)
/// does not stop the drain at its inner closing tag. This is a strict
/// improvement over a naive first-match search, not a total fix: if the
/// original error occurred *inside* an already-open same-named descendant
/// whose own `Start` this function never observed (it only starts counting
/// from its own entry point), the nesting count can still be off by one. That
/// residual case is narrow enough (an error nested inside a self-nesting
/// container, which is itself rare in real-world feeds) to accept rather than
/// chase further.
pub fn skip_to_end_qualified(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>, tag: &[u8]) {
    let mut nested: u32 = 0;
    for _ in 0..MAX_DRAIN_EVENTS {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == tag => nested += 1,
            Ok(Event::End(e)) if e.name().as_ref() == tag => {
                if nested == 0 {
                    break;
                }
                nested -= 1;
            }
            Ok(Event::Eof) => break,
            Err(_) | Ok(_) => {}
        }
        buf.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_to_string_valid_utf8() {
        let bytes = b"Hello, World!";
        assert_eq!(bytes_to_string(bytes), "Hello, World!");
    }

    #[test]
    fn test_bytes_to_string_invalid_utf8() {
        let bytes = &[0xff, 0xfe, 0x48, 0x65, 0x6c, 0x6c, 0x6f];
        let result = bytes_to_string(bytes);
        assert!(result.contains("Hello"));
    }

    #[test]
    fn test_read_text_basic() {
        let xml = b"<title>Test Title</title>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        let limits = ParserLimits::default();

        // Skip to after the start tag
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(_)) => break,
                Ok(Event::Eof) => panic!("Unexpected EOF"),
                _ => {}
            }
            buf.clear();
        }
        buf.clear();

        let (text, had_bozo) = read_text(&mut reader, &mut buf, &limits).unwrap();
        assert_eq!(text, "Test Title");
        assert!(!had_bozo);
    }

    #[test]
    fn test_read_text_exceeds_limit() {
        let xml = b"<title>This is a very long title</title>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        let limits = ParserLimits {
            max_text_length: 10,
            ..ParserLimits::default()
        };

        // Skip to after the start tag
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(_)) => break,
                Ok(Event::Eof) => panic!("Unexpected EOF"),
                _ => {}
            }
            buf.clear();
        }
        buf.clear();

        let result = read_text(&mut reader, &mut buf, &limits);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_text_numeric_char_ref() {
        let xml = b"<guid>https://example.com/?post_type=webcomic1&#038;p=3172</guid>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        let limits = ParserLimits::default();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(_)) => break,
                Ok(Event::Eof) => panic!("Unexpected EOF"),
                _ => {}
            }
            buf.clear();
        }
        buf.clear();

        let (text, had_bozo) = read_text(&mut reader, &mut buf, &limits).unwrap();
        assert_eq!(text, "https://example.com/?post_type=webcomic1&p=3172");
        assert!(!had_bozo);
    }

    #[test]
    fn test_read_text_amp_entity() {
        let xml = b"<guid>https://example.com/?a=1&amp;b=2</guid>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        let limits = ParserLimits::default();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(_)) => break,
                Ok(Event::Eof) => panic!("Unexpected EOF"),
                _ => {}
            }
            buf.clear();
        }
        buf.clear();

        let (text, had_bozo) = read_text(&mut reader, &mut buf, &limits).unwrap();
        assert_eq!(text, "https://example.com/?a=1&b=2");
        assert!(!had_bozo);
    }

    #[test]
    fn test_read_text_hex_char_ref() {
        let xml = b"<guid>https://example.com/?a=1&#x26;b=2</guid>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        let limits = ParserLimits::default();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(_)) => break,
                Ok(Event::Eof) => panic!("Unexpected EOF"),
                _ => {}
            }
            buf.clear();
        }
        buf.clear();

        let (text, had_bozo) = read_text(&mut reader, &mut buf, &limits).unwrap();
        assert_eq!(text, "https://example.com/?a=1&b=2");
        assert!(!had_bozo);
    }

    #[test]
    fn test_read_text_multiple_entities() {
        let xml = b"<guid>https://example.com/?a=1&amp;b=2&amp;c=3</guid>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        let limits = ParserLimits::default();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(_)) => break,
                Ok(Event::Eof) => panic!("Unexpected EOF"),
                _ => {}
            }
            buf.clear();
        }
        buf.clear();

        let (text, had_bozo) = read_text(&mut reader, &mut buf, &limits).unwrap();
        assert_eq!(text, "https://example.com/?a=1&b=2&c=3");
        assert!(!had_bozo);
    }

    #[test]
    fn test_read_text_unknown_entity_preserved() {
        // Unknown entities should be kept verbatim, not cause errors (bozo pattern).
        let xml = b"<guid>https://example.com/?a=1&customEntity;b=2</guid>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        let limits = ParserLimits::default();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(_)) => break,
                Ok(Event::Eof) => panic!("Unexpected EOF"),
                _ => {}
            }
            buf.clear();
        }
        buf.clear();

        let (text, had_bozo) = read_text(&mut reader, &mut buf, &limits).unwrap();
        assert_eq!(text, "https://example.com/?a=1&customEntity;b=2");
        assert!(had_bozo);
    }

    #[test]
    fn test_read_text_mixed_valid_and_unknown_entities() {
        // Mix of standard and unknown entities — all should resolve without error.
        let xml = b"<title>AT&amp;T&unknown;rocks</title>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        let limits = ParserLimits::default();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(_)) => break,
                Ok(Event::Eof) => panic!("Unexpected EOF"),
                _ => {}
            }
            buf.clear();
        }
        buf.clear();

        let (text, had_bozo) = read_text(&mut reader, &mut buf, &limits).unwrap();
        assert_eq!(text, "AT&T&unknown;rocks");
        assert!(had_bozo);
    }

    /// Advance `reader` past the first Start event and return a fresh reader ready for `read_text`.
    fn advance_past_start(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) {
        loop {
            match reader.read_event_into(buf) {
                Ok(Event::Start(_)) => break,
                Ok(Event::Eof) => panic!("Unexpected EOF"),
                _ => {}
            }
            buf.clear();
        }
        buf.clear();
    }

    #[test]
    fn test_read_text_malformed_hex_char_ref() {
        // &#x; (no hex digits after x) must be preserved verbatim, not cause an error.
        let xml = b"<guid>pre&#x;suf</guid>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        advance_past_start(&mut reader, &mut buf);
        let (text, had_bozo) = read_text(&mut reader, &mut buf, &ParserLimits::default()).unwrap();
        assert_eq!(text, "pre&#x;suf");
        assert!(had_bozo);
    }

    #[test]
    fn test_read_text_malformed_decimal_char_ref() {
        // &#; (no digits at all) must be preserved verbatim, not cause an error.
        let xml = b"<guid>pre&#;suf</guid>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        advance_past_start(&mut reader, &mut buf);
        let (text, had_bozo) = read_text(&mut reader, &mut buf, &ParserLimits::default()).unwrap();
        assert_eq!(text, "pre&#;suf");
        assert!(had_bozo);
    }

    #[test]
    fn test_read_text_empty_entity_name() {
        // &; (empty entity name) must be preserved verbatim, not cause an error.
        let xml = b"<guid>pre&;suf</guid>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        advance_past_start(&mut reader, &mut buf);
        let (text, had_bozo) = read_text(&mut reader, &mut buf, &ParserLimits::default()).unwrap();
        assert_eq!(text, "pre&;suf");
        assert!(had_bozo);
    }

    fn advance_past_start_xhtml(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) {
        loop {
            match reader.read_event_into(buf) {
                Ok(Event::Start(_)) => break,
                Ok(Event::Eof) => panic!("Unexpected EOF"),
                _ => {}
            }
            buf.clear();
        }
        buf.clear();
    }

    #[test]
    fn test_read_xhtml_content_preserves_markup() {
        let xml = b"<content type=\"xhtml\"><div xmlns=\"http://www.w3.org/1999/xhtml\"><p>Hello <b>world</b></p></div></content>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        advance_past_start_xhtml(&mut reader, &mut buf);
        let (result, had_bozo) =
            read_xhtml_content(&mut reader, &mut buf, &ParserLimits::default()).unwrap();
        assert_eq!(result, "<p>Hello <b>world</b></p>");
        assert!(!had_bozo);
    }

    #[test]
    fn test_read_xhtml_content_no_outer_div() {
        let xml = b"<content type=\"xhtml\"><div xmlns=\"http://www.w3.org/1999/xhtml\"><p>Hello <b>world</b></p></div></content>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        advance_past_start_xhtml(&mut reader, &mut buf);
        let (result, had_bozo) =
            read_xhtml_content(&mut reader, &mut buf, &ParserLimits::default()).unwrap();
        assert!(!result.contains("<div"), "outer <div> must be stripped");
        assert!(!had_bozo);
    }

    #[test]
    fn test_read_xhtml_content_empty() {
        let xml =
            b"<content type=\"xhtml\"><div xmlns=\"http://www.w3.org/1999/xhtml\"></div></content>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        advance_past_start_xhtml(&mut reader, &mut buf);
        let (result, had_bozo) =
            read_xhtml_content(&mut reader, &mut buf, &ParserLimits::default()).unwrap();
        assert_eq!(result, "");
        assert!(!had_bozo);
    }

    #[test]
    fn test_read_xhtml_content_no_div_wrapper_no_panic() {
        // Malformed: no <div> wrapper at all — must return empty and signal bozo
        let xml = b"<content type=\"xhtml\"></content>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        advance_past_start_xhtml(&mut reader, &mut buf);
        let (result, had_bozo) =
            read_xhtml_content(&mut reader, &mut buf, &ParserLimits::default()).unwrap();
        assert_eq!(result, "");
        assert!(had_bozo);
    }

    #[test]
    fn test_read_xhtml_content_nested_elements() {
        let xml = b"<content type=\"xhtml\"><div xmlns=\"http://www.w3.org/1999/xhtml\"><ul><li>A</li><li>B</li></ul></div></content>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        advance_past_start_xhtml(&mut reader, &mut buf);
        let (result, had_bozo) =
            read_xhtml_content(&mut reader, &mut buf, &ParserLimits::default()).unwrap();
        assert!(result.contains("<ul>"));
        assert!(result.contains("<li>A</li>"));
        assert!(result.contains("<li>B</li>"));
        assert!(!result.contains("<div"));
        assert!(!had_bozo);
    }

    #[test]
    fn test_read_xhtml_content_preserves_entities() {
        // Bug #215: &amp; and &lt; must survive the round-trip through serialize_inner_xml
        let xml = b"<content type=\"xhtml\"><div xmlns=\"http://www.w3.org/1999/xhtml\"><p>Tom &amp; Jerry &lt;rocks&gt;</p></div></content>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        advance_past_start_xhtml(&mut reader, &mut buf);
        let (result, had_bozo) =
            read_xhtml_content(&mut reader, &mut buf, &ParserLimits::default()).unwrap();
        assert!(!had_bozo);
        assert!(
            result.contains("&amp;"),
            "& must be escaped as &amp; in output, got: {result}"
        );
        assert!(
            result.contains("&lt;"),
            "< must be escaped as &lt; in output, got: {result}"
        );
        assert!(
            !result.contains("Tom & Jerry"),
            "bare & must not appear in output, got: {result}"
        );
    }

    #[test]
    fn test_read_xhtml_content_apos_and_quot_decoded() {
        // #316: &apos; must become ' and &quot; must become " in xhtml output
        let xml = b"<content type=\"xhtml\"><div xmlns=\"http://www.w3.org/1999/xhtml\"><p>it&apos;s a &quot;test&quot;</p></div></content>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        advance_past_start_xhtml(&mut reader, &mut buf);
        let (result, had_bozo) =
            read_xhtml_content(&mut reader, &mut buf, &ParserLimits::default()).unwrap();
        assert!(!had_bozo);
        assert!(
            result.contains("it's a"),
            "&apos; must decode to literal apostrophe, got: {result}"
        );
        assert!(
            result.contains("\"test\""),
            "&quot; must decode to literal quote, got: {result}"
        );
        assert!(
            !result.contains("&apos;"),
            "&apos; must not remain escaped in output, got: {result}"
        );
        assert!(
            !result.contains("&quot;"),
            "&quot; must not remain escaped in output, got: {result}"
        );
    }

    #[test]
    fn test_skip_element_basic() {
        let xml = b"<parent><child>content</child></parent>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        let limits = ParserLimits::default();
        let depth = 1;

        // Skip to after the start tag
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(_)) => break,
                Ok(Event::Eof) => panic!("Unexpected EOF"),
                _ => {}
            }
            buf.clear();
        }
        buf.clear();

        let result = skip_element(&mut reader, &mut buf, &limits, depth);
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolve_entity_valid_named() {
        let xml = b"<t>&amp;</t>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        advance_past_start(&mut reader, &mut buf);
        let (text, had_bozo) = read_text(&mut reader, &mut buf, &ParserLimits::default()).unwrap();
        assert_eq!(text, "&");
        assert!(!had_bozo);
    }

    #[test]
    fn test_resolve_entity_valid_numeric() {
        let xml = b"<t>&#38;</t>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        advance_past_start(&mut reader, &mut buf);
        let (text, had_bozo) = read_text(&mut reader, &mut buf, &ParserLimits::default()).unwrap();
        assert_eq!(text, "&");
        assert!(!had_bozo);
    }

    #[test]
    fn test_resolve_entity_unknown_named() {
        let xml = b"<t>&foo;</t>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        advance_past_start(&mut reader, &mut buf);
        let (text, had_bozo) = read_text(&mut reader, &mut buf, &ParserLimits::default()).unwrap();
        assert_eq!(text, "&foo;");
        assert!(had_bozo);
    }

    #[test]
    fn test_read_text_returns_bozo_on_unknown_entity() {
        let xml = b"<t>hello &custom; world</t>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        advance_past_start(&mut reader, &mut buf);
        let (text, had_bozo) = read_text(&mut reader, &mut buf, &ParserLimits::default()).unwrap();
        assert_eq!(text, "hello &custom; world");
        assert!(had_bozo);
    }

    #[test]
    fn test_read_text_no_bozo_on_standard_entities() {
        let xml = b"<t>a&amp;b&lt;c&gt;</t>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        advance_past_start(&mut reader, &mut buf);
        let (text, had_bozo) = read_text(&mut reader, &mut buf, &ParserLimits::default()).unwrap();
        assert_eq!(text, "a&b<c>");
        assert!(!had_bozo);
    }

    #[test]
    fn test_read_text_mixed_entities_bozo() {
        let xml = b"<t>&amp;&unknown;</t>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        advance_past_start(&mut reader, &mut buf);
        let (text, had_bozo) = read_text(&mut reader, &mut buf, &ParserLimits::default()).unwrap();
        assert_eq!(text, "&&unknown;");
        assert!(had_bozo);
    }

    fn parse_namespaces_from_element(xml: &[u8], limits: &ParserLimits) -> ParsedFeed {
        let mut reader = Reader::from_reader(xml);
        let mut buf = Vec::new();
        let mut feed = init_feed(crate::types::FeedVersion::Rss20, limits.max_entries);
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e) | Event::Empty(e)) => {
                    extract_namespaces(&e, &mut feed, limits);
                    break;
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
            buf.clear();
        }
        feed
    }

    #[test]
    fn test_extract_namespaces_default_and_prefixed() {
        let xml = b"<rss xmlns=\"http://default.example.com/\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\"/>";
        let limits = ParserLimits::default();
        let feed = parse_namespaces_from_element(xml, &limits);
        assert!(!feed.bozo);
        assert_eq!(
            feed.namespaces.get("").map(String::as_str),
            Some("http://default.example.com/")
        );
        assert_eq!(
            feed.namespaces.get("dc").map(String::as_str),
            Some("http://purl.org/dc/elements/1.1/")
        );
    }

    #[test]
    fn test_extract_namespaces_empty_default() {
        let xml = b"<rss xmlns=\"\"/>";
        let limits = ParserLimits::default();
        let feed = parse_namespaces_from_element(xml, &limits);
        assert!(!feed.bozo);
        assert_eq!(feed.namespaces.get("").map(String::as_str), Some(""));
    }

    #[test]
    fn test_extract_namespaces_no_xmlns() {
        let xml = b"<rss version=\"2.0\"/>";
        let limits = ParserLimits::default();
        let feed = parse_namespaces_from_element(xml, &limits);
        assert!(!feed.bozo);
        assert!(feed.namespaces.is_empty());
    }

    #[test]
    fn test_extract_namespaces_limit_exceeded_sets_bozo() {
        let xml =
            b"<rss xmlns:a=\"http://a.com/\" xmlns:b=\"http://b.com/\" xmlns:c=\"http://c.com/\"/>";
        let limits = ParserLimits {
            max_namespaces: 2,
            ..ParserLimits::default()
        };
        let feed = parse_namespaces_from_element(xml, &limits);
        assert!(feed.bozo);
        assert_eq!(feed.namespaces.len(), 2);
    }

    #[test]
    fn test_extract_namespaces_uri_too_long_sets_bozo() {
        let long_uri = "http://".to_string() + &"a".repeat(200);
        let xml = format!("<rss xmlns:dc=\"{long_uri}\"/>");
        let limits = ParserLimits {
            max_attribute_length: 100,
            ..ParserLimits::default()
        };
        let feed = parse_namespaces_from_element(xml.as_bytes(), &limits);
        assert!(feed.bozo);
        assert!(feed.namespaces.is_empty());
    }

    #[test]
    fn test_is_dc_tag_custom_prefix() {
        let mut ns = HashMap::new();
        ns.insert(
            "dublin".to_string(),
            "http://purl.org/dc/elements/1.1/".to_string(),
        );
        assert_eq!(is_dc_tag(b"dublin:creator", &ns), Some("creator"));
        assert_eq!(is_dc_tag(b"dublin:date", &ns), Some("date"));
        // canonical prefix still works
        let empty = HashMap::new();
        assert_eq!(is_dc_tag(b"dc:creator", &empty), Some("creator"));
        // unrelated prefix does not match
        assert!(is_dc_tag(b"foo:creator", &ns).is_none());
    }

    #[test]
    fn test_is_media_tag_custom_prefix() {
        let mut ns = HashMap::new();
        ns.insert(
            "mrss".to_string(),
            "http://search.yahoo.com/mrss/".to_string(),
        );
        assert_eq!(is_media_tag(b"mrss:thumbnail", &ns), Some("thumbnail"));
        assert_eq!(is_media_tag(b"mrss:content", &ns), Some("content"));
        // canonical prefix still works
        let empty = HashMap::new();
        assert_eq!(is_media_tag(b"media:thumbnail", &empty), Some("thumbnail"));
        // unrelated prefix does not match
        assert!(is_media_tag(b"foo:thumbnail", &ns).is_none());
    }

    #[test]
    fn test_is_itunes_tag_custom_prefix() {
        let mut ns = HashMap::new();
        ns.insert(
            "podcast".to_string(),
            "http://www.itunes.com/dtds/podcast-1.0.dtd".to_string(),
        );
        assert!(is_itunes_tag(b"podcast:author", b"author", &ns));
        assert!(is_itunes_tag(b"podcast:explicit", b"explicit", &ns));
        // canonical prefix still works
        let empty = HashMap::new();
        assert!(is_itunes_tag(b"itunes:author", b"author", &empty));
        // unrelated prefix does not match
        assert!(!is_itunes_tag(b"foo:author", b"author", &ns));
    }

    #[test]
    fn test_ns_tag_with_invalid_security_chars() {
        let mut ns = HashMap::new();
        ns.insert(
            "dublin".to_string(),
            "http://purl.org/dc/elements/1.1/".to_string(),
        );
        // Security: invalid chars in local name must be rejected
        assert!(is_dc_tag(b"dublin:../../etc/passwd", &ns).is_none());
        assert!(is_dc_tag(b"dublin:tag<script>", &ns).is_none());
    }

    #[test]
    fn test_is_itunes_tag_security() {
        let empty = HashMap::new();
        // Path traversal in local name must not match
        assert!(!is_itunes_tag(
            b"itunes:../../etc/passwd",
            b"../../etc/passwd",
            &empty
        ));
        // Same with custom prefix
        let mut ns = HashMap::new();
        ns.insert(
            "it".to_string(),
            "http://www.itunes.com/dtds/podcast-1.0.dtd".to_string(),
        );
        assert!(!is_itunes_tag(
            b"it:../../etc/passwd",
            b"../../etc/passwd",
            &ns
        ));
        // Unprefixed path traversal also rejected (no colon → direct match, but value differs)
        assert!(!is_itunes_tag(b"../../etc/passwd", b"author", &empty));
    }

    #[test]
    fn test_read_text_strips_null_bytes() {
        let xml = b"<title>Hello\x00World</title>";
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        let limits = ParserLimits::default();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(_)) => break,
                Ok(Event::Eof) => panic!("Unexpected EOF"),
                _ => {}
            }
            buf.clear();
        }

        let (text, _bozo) = read_text(&mut reader, &mut buf, &limits).unwrap();
        assert_eq!(text, "HelloWorld");
    }
}
