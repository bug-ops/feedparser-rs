//! RSS 1.0 (RDF) parser implementation
//!
//! RSS 1.0 differs significantly from RSS 2.0:
//! - Uses RDF (Resource Description Framework) as the container
//! - Root element is `<rdf:RDF>` instead of `<rss>`
//! - Items are siblings of channel, not children
//! - Items have `rdf:about` attributes for identification
//! - Supports Dublin Core and other RDF vocabularies

use crate::{
    ParserLimits,
    error::{FeedError, Result},
    namespace::{content, dublin_core, syndication},
    types::{Entry, FeedVersion, Image, ParsedFeed, TextConstruct, TextType},
};
use quick_xml::{Reader, events::Event};

use super::common::{
    EVENT_BUFFER_CAPACITY, LimitedCollectionExt, check_depth, init_feed, is_content_tag, is_dc_tag,
    is_syn_tag, read_text, skip_element,
};

/// Parse RSS 1.0 (RDF) feed from raw bytes
///
/// Parses an RSS 1.0 feed in tolerant mode, setting the bozo flag
/// on errors but continuing to extract as much data as possible.
///
/// # Arguments
///
/// * `data` - Raw RSS 1.0 XML data
///
/// # Returns
///
/// * `Ok(ParsedFeed)` - Successfully parsed feed (may have bozo flag set)
/// * `Err(FeedError)` - Fatal error that prevented any parsing
///
/// # Examples
///
/// ```ignore
/// let xml = br#"
///     <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
///              xmlns="http://purl.org/rss/1.0/">
///         <channel rdf:about="http://example.com/">
///             <title>Example</title>
///             <link>http://example.com/</link>
///             <description>Example RSS 1.0 feed</description>
///         </channel>
///         <item rdf:about="http://example.com/item1">
///             <title>First Item</title>
///             <link>http://example.com/item1</link>
///         </item>
///     </rdf:RDF>
/// "#;
///
/// let feed = parse_rss10(xml).unwrap();
/// assert_eq!(feed.feed.title.as_deref(), Some("Example"));
/// ```
#[allow(dead_code)]
pub fn parse_rss10(data: &[u8]) -> Result<ParsedFeed> {
    parse_rss10_with_limits(data, ParserLimits::default())
}

/// Parse RSS 1.0 with custom parser limits
pub fn parse_rss10_with_limits(data: &[u8], limits: ParserLimits) -> Result<ParsedFeed> {
    limits
        .check_feed_size(data.len())
        .map_err(|e| FeedError::InvalidFormat(e.to_string()))?;

    let mut reader = Reader::from_reader(data);
    reader.config_mut().trim_text(true);

    let mut feed = init_feed(FeedVersion::Rss10, limits.max_entries);
    let mut buf = Vec::with_capacity(EVENT_BUFFER_CAPACITY);
    let mut depth: usize = 1;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                let name = e.local_name();
                let full_name = e.name();

                depth += 1;

                // Handle RDF root element - continue to parse children
                if name.as_ref() == b"RDF" || full_name.as_ref() == b"rdf:RDF" {
                    // RDF root - nothing more to do, continue parsing children
                } else if name.as_ref() == b"channel" {
                    // Extract rdf:about as feed ID
                    for attr in e.attributes().flatten() {
                        if (attr.key.as_ref() == b"rdf:about"
                            || attr.key.local_name().as_ref() == b"about")
                            && let Ok(value) = attr.unescape_value()
                        {
                            feed.feed.id = Some(value.to_string());
                        }
                    }
                    if let Err(e) = parse_channel(&mut reader, &mut feed, &limits, &mut depth) {
                        feed.bozo = true;
                        feed.bozo_exception = Some(e.to_string());
                    }
                    depth = depth.saturating_sub(1);
                } else if name.as_ref() == b"item" {
                    if depth > limits.max_nesting_depth {
                        feed.bozo = true;
                        feed.bozo_exception = Some(format!(
                            "XML nesting depth {} exceeds maximum {}",
                            depth, limits.max_nesting_depth
                        ));
                        skip_element(&mut reader, &mut buf, &limits, depth)?;
                        depth = depth.saturating_sub(1);
                        buf.clear();
                        continue;
                    }

                    // Extract rdf:about as item ID first (before releasing borrow on buf)
                    let item_id = e.attributes().flatten().find_map(|attr| {
                        if attr.key.as_ref() == b"rdf:about"
                            || attr.key.local_name().as_ref() == b"about"
                        {
                            attr.unescape_value().ok().map(|v| v.to_string())
                        } else {
                            None
                        }
                    });

                    // Check entry limit (inline to avoid borrow issues)
                    if feed.entries.is_at_limit(limits.max_entries) {
                        feed.bozo = true;
                        feed.bozo_exception =
                            Some(format!("Entry limit exceeded: {}", limits.max_entries));
                        skip_element(&mut reader, &mut buf, &limits, depth)?;
                        depth = depth.saturating_sub(1);
                        buf.clear();
                        continue;
                    }

                    match parse_item(&mut reader, &mut buf, &limits, &mut depth, item_id) {
                        Ok(entry) => feed.entries.push(entry),
                        Err(err) => {
                            feed.bozo = true;
                            feed.bozo_exception = Some(err.to_string());
                        }
                    }
                    depth = depth.saturating_sub(1);
                } else if name.as_ref() == b"image" {
                    if let Ok(image) = parse_image(&mut reader, &mut buf, &limits, &mut depth) {
                        feed.feed.image = Some(image);
                    }
                    depth = depth.saturating_sub(1);
                } else if name.as_ref() == b"textinput" || name.as_ref() == b"textInput" {
                    // Skip textinput element (rarely used)
                    skip_element(&mut reader, &mut buf, &limits, depth)?;
                    depth = depth.saturating_sub(1);
                } else {
                    // Skip unknown elements at RDF level
                    skip_element(&mut reader, &mut buf, &limits, depth)?;
                    depth = depth.saturating_sub(1);
                }
            }
            Ok(Event::End(_)) => {
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                feed.bozo = true;
                feed.bozo_exception = Some(format!("XML parsing error: {e}"));
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(feed)
}

/// Parse <channel> element (feed metadata)
fn parse_channel(
    reader: &mut Reader<&[u8]>,
    feed: &mut ParsedFeed,
    limits: &ParserLimits,
    depth: &mut usize,
) -> Result<()> {
    let mut buf = Vec::with_capacity(EVENT_BUFFER_CAPACITY);

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                *depth += 1;
                check_depth(*depth, limits.max_nesting_depth)?;

                let name = e.local_name();
                let full_name = e.name();

                match name.as_ref() {
                    b"title" => {
                        feed.feed.title = Some(read_text(reader, &mut buf, limits)?);
                    }
                    b"link" => {
                        let link_text = read_text(reader, &mut buf, limits)?;
                        feed.feed
                            .set_alternate_link(link_text, limits.max_links_per_feed);
                    }
                    b"description" => {
                        feed.feed.subtitle = Some(read_text(reader, &mut buf, limits)?);
                    }
                    b"items" => {
                        // RSS 1.0 has an <items> element containing rdf:Seq with rdf:li references
                        // We skip this as items are parsed at the RDF root level
                        skip_element(reader, &mut buf, limits, *depth)?;
                    }
                    b"image" => {
                        // This is a reference, not the actual image - skip it
                        skip_element(reader, &mut buf, limits, *depth)?;
                    }
                    b"textinput" | b"textInput" => {
                        // This is a reference, not the actual textinput - skip it
                        skip_element(reader, &mut buf, limits, *depth)?;
                    }
                    _ => {
                        // Check for Dublin Core namespace tags
                        if let Some(dc_element) = is_dc_tag(full_name.as_ref()) {
                            let dc_elem = dc_element.to_string();
                            let text = read_text(reader, &mut buf, limits)?;
                            dublin_core::handle_feed_element(&dc_elem, &text, &mut feed.feed);
                        } else if let Some(syn_element) = is_syn_tag(full_name.as_ref()) {
                            let syn_elem = syn_element.to_string();
                            let text = read_text(reader, &mut buf, limits)?;
                            syndication::handle_feed_element(&syn_elem, &text, &mut feed.feed);
                        } else {
                            skip_element(reader, &mut buf, limits, *depth)?;
                        }
                    }
                }
                *depth = depth.saturating_sub(1);
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"channel" => {
                break;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(())
}

/// Parse <item> element (entry)
fn parse_item(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
    depth: &mut usize,
    item_id: Option<String>,
) -> Result<Entry> {
    let mut entry = Entry::with_capacity();
    entry.id = item_id;

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                *depth += 1;
                check_depth(*depth, limits.max_nesting_depth)?;

                let name = e.local_name();
                let full_name = e.name();

                match name.as_ref() {
                    b"title" => {
                        entry.title = Some(read_text(reader, buf, limits)?);
                    }
                    b"link" => {
                        let link_text = read_text(reader, buf, limits)?;
                        entry.set_alternate_link(link_text, limits.max_links_per_entry);
                    }
                    b"description" => {
                        let desc = read_text(reader, buf, limits)?;
                        entry.summary = Some(desc.clone());
                        entry.summary_detail = Some(TextConstruct {
                            value: desc,
                            content_type: TextType::Html,
                            language: None,
                            base: None,
                        });
                    }
                    _ => {
                        // Check for Dublin Core namespace tags
                        if let Some(dc_element) = is_dc_tag(full_name.as_ref()) {
                            let dc_elem = dc_element.to_string();
                            let text = read_text(reader, buf, limits)?;
                            // dublin_core::handle_entry_element already handles dc:date -> published
                            dublin_core::handle_entry_element(&dc_elem, &text, &mut entry);
                        } else if let Some(content_element) = is_content_tag(full_name.as_ref()) {
                            let content_elem = content_element.to_string();
                            let text = read_text(reader, buf, limits)?;
                            content::handle_entry_element(&content_elem, &text, &mut entry);
                        } else {
                            skip_element(reader, buf, limits, *depth)?;
                        }
                    }
                }
                *depth = depth.saturating_sub(1);
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"item" => {
                break;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(entry)
}

/// Parse <image> element
fn parse_image(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
    depth: &mut usize,
) -> Result<Image> {
    let mut url = String::new();
    let mut title = None;
    let mut link = None;

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                *depth += 1;
                check_depth(*depth, limits.max_nesting_depth)?;

                match e.local_name().as_ref() {
                    b"url" => url = read_text(reader, buf, limits)?,
                    b"title" => title = Some(read_text(reader, buf, limits)?),
                    b"link" => link = Some(read_text(reader, buf, limits)?),
                    _ => skip_element(reader, buf, limits, *depth)?,
                }
                *depth = depth.saturating_sub(1);
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"image" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    if url.is_empty() {
        return Err(FeedError::InvalidFormat("Image missing url".to_string()));
    }

    Ok(Image {
        url,
        title,
        link,
        width: None,
        height: None,
        description: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_basic_rss10() {
        let xml = br#"<?xml version="1.0"?>
        <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                 xmlns="http://purl.org/rss/1.0/">
            <channel rdf:about="http://example.com/">
                <title>Test Feed</title>
                <link>http://example.com</link>
                <description>Test description</description>
            </channel>
        </rdf:RDF>"#;

        let feed = parse_rss10(xml).unwrap();
        assert_eq!(feed.version, FeedVersion::Rss10);
        assert!(!feed.bozo);
        assert_eq!(feed.feed.title.as_deref(), Some("Test Feed"));
        assert_eq!(feed.feed.link.as_deref(), Some("http://example.com"));
        assert_eq!(feed.feed.subtitle.as_deref(), Some("Test description"));
        assert_eq!(feed.feed.id.as_deref(), Some("http://example.com/"));
    }

    #[test]
    fn test_parse_rss10_with_items() {
        let xml = br#"<?xml version="1.0"?>
        <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                 xmlns="http://purl.org/rss/1.0/">
            <channel rdf:about="http://example.com/">
                <title>Test</title>
                <link>http://example.com</link>
                <description>Test</description>
                <items>
                    <rdf:Seq>
                        <rdf:li resource="http://example.com/1"/>
                        <rdf:li resource="http://example.com/2"/>
                    </rdf:Seq>
                </items>
            </channel>
            <item rdf:about="http://example.com/1">
                <title>Item 1</title>
                <link>http://example.com/1</link>
                <description>Description 1</description>
            </item>
            <item rdf:about="http://example.com/2">
                <title>Item 2</title>
                <link>http://example.com/2</link>
            </item>
        </rdf:RDF>"#;

        let feed = parse_rss10(xml).unwrap();
        assert_eq!(feed.entries.len(), 2);
        assert_eq!(feed.entries[0].title.as_deref(), Some("Item 1"));
        assert_eq!(feed.entries[0].id.as_deref(), Some("http://example.com/1"));
        assert_eq!(feed.entries[1].title.as_deref(), Some("Item 2"));
    }

    #[test]
    fn test_parse_rss10_with_dublin_core() {
        let xml = br#"<?xml version="1.0"?>
        <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                 xmlns="http://purl.org/rss/1.0/"
                 xmlns:dc="http://purl.org/dc/elements/1.1/">
            <channel rdf:about="http://example.com/">
                <title>Test</title>
                <link>http://example.com</link>
                <description>Test</description>
                <dc:creator>John Doe</dc:creator>
                <dc:rights>Copyright 2024</dc:rights>
            </channel>
            <item rdf:about="http://example.com/1">
                <title>Item 1</title>
                <link>http://example.com/1</link>
                <dc:date>2024-12-15T10:00:00Z</dc:date>
                <dc:creator>Jane Smith</dc:creator>
            </item>
        </rdf:RDF>"#;

        let feed = parse_rss10(xml).unwrap();
        assert_eq!(feed.feed.dc_creator.as_deref(), Some("John Doe"));
        assert_eq!(feed.feed.dc_rights.as_deref(), Some("Copyright 2024"));

        assert_eq!(feed.entries.len(), 1);
        let entry = &feed.entries[0];
        assert!(entry.published.is_some());
        let dt = entry.published.unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_parse_rss10_with_image() {
        let xml = br#"<?xml version="1.0"?>
        <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                 xmlns="http://purl.org/rss/1.0/">
            <channel rdf:about="http://example.com/">
                <title>Test</title>
                <link>http://example.com</link>
                <description>Test</description>
            </channel>
            <image rdf:about="http://example.com/logo.png">
                <url>http://example.com/logo.png</url>
                <title>Logo</title>
                <link>http://example.com</link>
            </image>
        </rdf:RDF>"#;

        let feed = parse_rss10(xml).unwrap();
        assert!(feed.feed.image.is_some());
        let img = feed.feed.image.as_ref().unwrap();
        assert_eq!(img.url, "http://example.com/logo.png");
        assert_eq!(img.title.as_deref(), Some("Logo"));
    }

    #[test]
    fn test_parse_rss10_without_rdf_prefix() {
        // Some RSS 1.0 feeds don't use the rdf: prefix
        let xml = br#"<?xml version="1.0"?>
        <RDF xmlns="http://purl.org/rss/1.0/"
             xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <channel>
                <title>Test</title>
                <link>http://example.com</link>
                <description>Test</description>
            </channel>
        </RDF>"#;

        let feed = parse_rss10(xml).unwrap();
        assert_eq!(feed.version, FeedVersion::Rss10);
        assert_eq!(feed.feed.title.as_deref(), Some("Test"));
    }

    #[test]
    fn test_parse_rss10_entry_limit() {
        let xml = br#"<?xml version="1.0"?>
        <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                 xmlns="http://purl.org/rss/1.0/">
            <channel rdf:about="http://example.com/">
                <title>Test</title>
                <link>http://example.com</link>
                <description>Test</description>
            </channel>
            <item rdf:about="http://example.com/1"><title>1</title><link>http://example.com/1</link></item>
            <item rdf:about="http://example.com/2"><title>2</title><link>http://example.com/2</link></item>
            <item rdf:about="http://example.com/3"><title>3</title><link>http://example.com/3</link></item>
            <item rdf:about="http://example.com/4"><title>4</title><link>http://example.com/4</link></item>
        </rdf:RDF>"#;

        let limits = ParserLimits {
            max_entries: 2,
            ..Default::default()
        };
        let feed = parse_rss10_with_limits(xml, limits).unwrap();
        assert_eq!(feed.entries.len(), 2);
        assert!(feed.bozo);
    }

    #[test]
    fn test_parse_rss10_malformed_continues() {
        let xml = br#"<?xml version="1.0"?>
        <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                 xmlns="http://purl.org/rss/1.0/">
            <channel rdf:about="http://example.com/">
                <title>Test</title>
                <link>http://example.com</link>
                <description>Test</description>
            </channel>
            <item rdf:about="http://example.com/1">
                <title>Item 1</title>
                <!-- Missing close tag but continues -->
        </rdf:RDF>"#;

        let feed = parse_rss10(xml).unwrap();
        // Should still extract some data
        assert_eq!(feed.feed.title.as_deref(), Some("Test"));
    }

    #[test]
    fn test_is_dc_tag_valid() {
        assert_eq!(is_dc_tag(b"dc:creator"), Some("creator"));
        assert_eq!(is_dc_tag(b"dc:date"), Some("date"));
        assert_eq!(is_dc_tag(b"dc:description"), Some("description"));
        assert_eq!(is_dc_tag(b"dc:subject"), Some("subject"));
        assert_eq!(is_dc_tag(b"dc:content-type"), Some("content-type"));
    }

    #[test]
    fn test_is_dc_tag_rejects_malicious() {
        // Path traversal attempts
        assert!(is_dc_tag(b"dc:../../etc/passwd").is_none());
        assert!(is_dc_tag(b"dc:../../../root").is_none());

        // Special characters
        assert!(is_dc_tag(b"dc:invalid<tag>").is_none());
        assert!(is_dc_tag(b"dc:tag&name").is_none());
        assert!(is_dc_tag(b"dc:tag;name").is_none());
        assert!(is_dc_tag(b"dc:tag/name").is_none());
        assert!(is_dc_tag(b"dc:tag\\name").is_none());

        // Empty tag name
        assert!(is_dc_tag(b"dc:").is_none());
    }

    #[test]
    fn test_is_dc_tag_non_dc() {
        assert!(is_dc_tag(b"title").is_none());
        assert!(is_dc_tag(b"link").is_none());
        assert!(is_dc_tag(b"atom:title").is_none());
    }

    #[test]
    fn test_parse_rss10_with_content_encoded() {
        let xml = br#"<?xml version="1.0"?>
        <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                 xmlns="http://purl.org/rss/1.0/"
                 xmlns:content="http://purl.org/rss/1.0/modules/content/">
            <channel rdf:about="http://example.com/">
                <title>Test</title>
                <link>http://example.com</link>
                <description>Test</description>
            </channel>
            <item rdf:about="http://example.com/1">
                <title>Item 1</title>
                <link>http://example.com/1</link>
                <description>Brief summary</description>
                <content:encoded><![CDATA[<p>Full <strong>HTML</strong> content</p>]]></content:encoded>
            </item>
        </rdf:RDF>"#;

        let feed = parse_rss10(xml).unwrap();
        assert_eq!(feed.entries.len(), 1);

        let entry = &feed.entries[0];
        assert_eq!(entry.summary.as_deref(), Some("Brief summary"));

        // Verify content:encoded is parsed
        assert!(!entry.content.is_empty());
        assert_eq!(entry.content[0].content_type.as_deref(), Some("text/html"));
        assert!(entry.content[0].value.contains("Full"));
        assert!(entry.content[0].value.contains("HTML"));
    }

    #[test]
    fn test_parse_rss10_with_syndication() {
        let xml = br#"<?xml version="1.0"?>
        <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                 xmlns="http://purl.org/rss/1.0/"
                 xmlns:syn="http://purl.org/rss/1.0/modules/syndication/">
            <channel rdf:about="http://example.com/">
                <title>Test</title>
                <link>http://example.com</link>
                <description>Test</description>
                <syn:updatePeriod>hourly</syn:updatePeriod>
                <syn:updateFrequency>2</syn:updateFrequency>
                <syn:updateBase>2024-01-01T00:00:00Z</syn:updateBase>
            </channel>
        </rdf:RDF>"#;

        let feed = parse_rss10(xml).unwrap();
        assert!(feed.feed.syndication.is_some());

        let syn = feed.feed.syndication.as_ref().unwrap();
        assert_eq!(
            syn.update_period,
            Some(crate::namespace::syndication::UpdatePeriod::Hourly)
        );
        assert_eq!(syn.update_frequency, Some(2));
        assert_eq!(syn.update_base.as_deref(), Some("2024-01-01T00:00:00Z"));
    }
}
