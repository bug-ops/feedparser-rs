//! Atom 1.0 parser implementation

use crate::{
    ParserLimits,
    error::{FeedError, Result},
    namespace::{content, dublin_core, media_rss},
    types::{
        Content, Entry, FeedVersion, Generator, Link, MediaContent, MediaThumbnail, ParsedFeed,
        Person, Source, Tag, TextConstruct, TextType,
    },
    util::{base_url::BaseUrlContext, parse_date},
};
use quick_xml::{Reader, events::Event};

use super::common::{
    EVENT_BUFFER_CAPACITY, FromAttributes, LimitedCollectionExt, bytes_to_string, check_depth,
    extract_xml_base, init_feed, is_content_tag, is_dc_tag, is_media_tag, read_text, skip_element,
    skip_to_end,
};

/// Parse Atom 1.0 feed from raw bytes
///
/// Parses an Atom 1.0 feed in tolerant mode, setting the bozo flag
/// on errors but continuing to extract as much data as possible.
///
/// # Arguments
///
/// * `data` - Raw Atom XML data
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
///     <feed xmlns="http://www.w3.org/2005/Atom">
///         <title>Example Feed</title>
///         <link href="http://example.org/"/>
///         <updated>2024-12-14T10:00:00Z</updated>
///         <id>urn:uuid:60a76c80-d399-11d9-b93C-0003939e0af6</id>
///     </feed>
/// "#;
///
/// let feed = parse_atom10(xml).unwrap();
/// assert_eq!(feed.feed.title.as_deref(), Some("Example Feed"));
/// ```
#[allow(dead_code)]
pub fn parse_atom10(data: &[u8]) -> Result<ParsedFeed> {
    parse_atom10_with_limits(data, ParserLimits::default())
}

/// Parse Atom with custom limits
pub fn parse_atom10_with_limits(data: &[u8], limits: ParserLimits) -> Result<ParsedFeed> {
    limits
        .check_feed_size(data.len())
        .map_err(|e| FeedError::InvalidFormat(e.to_string()))?;

    let mut reader = Reader::from_reader(data);
    reader.config_mut().trim_text(true);

    let mut feed = init_feed(FeedVersion::Atom10, limits.max_entries);
    let mut buf = Vec::with_capacity(EVENT_BUFFER_CAPACITY);
    let mut depth: usize = 1;
    let mut base_ctx = BaseUrlContext::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.local_name().as_ref() == b"feed" => {
                if let Some(xml_base) = extract_xml_base(&e, limits.max_attribute_length) {
                    base_ctx.update_base(&xml_base);
                }

                depth += 1;
                if let Err(e) =
                    parse_feed_element(&mut reader, &mut feed, &limits, &mut depth, &base_ctx)
                {
                    feed.bozo = true;
                    feed.bozo_exception = Some(e.to_string());
                }
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

/// Parse <feed> element
#[allow(clippy::too_many_lines)]
fn parse_feed_element(
    reader: &mut Reader<&[u8]>,
    feed: &mut ParsedFeed,
    limits: &ParserLimits,
    depth: &mut usize,
    base_ctx: &BaseUrlContext,
) -> Result<()> {
    let mut buf = Vec::with_capacity(EVENT_BUFFER_CAPACITY);

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(event @ (Event::Start(_) | Event::Empty(_))) => {
                let is_empty = matches!(event, Event::Empty(_));
                let (Event::Start(e) | Event::Empty(e)) = &event else {
                    unreachable!()
                };

                *depth += 1;
                check_depth(*depth, limits.max_nesting_depth)?;

                let element = e.to_owned();
                // Use name() instead of local_name() to preserve namespace prefixes
                match element.name().as_ref() {
                    b"title" if !is_empty => {
                        let text = parse_text_construct(reader, &mut buf, &element, limits)?;
                        feed.feed.set_title(text);
                    }
                    b"link" => {
                        if let Some(mut link) = Link::from_attributes(
                            element.attributes().flatten(),
                            limits.max_attribute_length,
                        ) {
                            link.href = base_ctx.resolve_safe(&link.href);

                            if feed.feed.link.is_none() && link.rel.as_deref() == Some("alternate")
                            {
                                feed.feed.link = Some(link.href.clone());
                            }
                            feed.feed
                                .links
                                .try_push_limited(link, limits.max_links_per_feed);
                        }
                        if !is_empty {
                            skip_to_end(reader, &mut buf, b"link")?;
                        }
                    }
                    b"subtitle" if !is_empty => {
                        let text = parse_text_construct(reader, &mut buf, &element, limits)?;
                        feed.feed.set_subtitle(text);
                    }
                    b"id" if !is_empty => {
                        feed.feed.id = Some(read_text(reader, &mut buf, limits)?);
                    }
                    b"updated" if !is_empty => {
                        let text = read_text(reader, &mut buf, limits)?;
                        feed.feed.updated = parse_date(&text);
                    }
                    b"published" if !is_empty => {
                        let text = read_text(reader, &mut buf, limits)?;
                        feed.feed.published = parse_date(&text);
                    }
                    b"author" if !is_empty => {
                        if let Ok(person) = parse_person(reader, &mut buf, limits, depth) {
                            if feed.feed.author.is_none() {
                                feed.feed.set_author(person.clone());
                            }
                            feed.feed
                                .authors
                                .try_push_limited(person, limits.max_authors);
                        }
                    }
                    b"contributor" if !is_empty => {
                        if let Ok(person) = parse_person(reader, &mut buf, limits, depth) {
                            feed.feed
                                .contributors
                                .try_push_limited(person, limits.max_contributors);
                        }
                    }
                    b"category" => {
                        if let Some(tag) = Tag::from_attributes(
                            element.attributes().flatten(),
                            limits.max_attribute_length,
                        ) {
                            feed.feed.tags.try_push_limited(tag, limits.max_tags);
                        }
                        if !is_empty {
                            skip_to_end(reader, &mut buf, b"category")?;
                        }
                    }
                    b"generator" if !is_empty => {
                        let generator = parse_generator(reader, &mut buf, &element, limits)?;
                        feed.feed.set_generator(generator);
                    }
                    b"icon" if !is_empty => {
                        let url = read_text(reader, &mut buf, limits)?;
                        feed.feed.icon = Some(base_ctx.resolve_safe(&url));
                    }
                    b"logo" if !is_empty => {
                        let url = read_text(reader, &mut buf, limits)?;
                        feed.feed.logo = Some(base_ctx.resolve_safe(&url));
                    }
                    b"rights" if !is_empty => {
                        let text = parse_text_construct(reader, &mut buf, &element, limits)?;
                        feed.feed.set_rights(text);
                    }
                    b"entry" if !is_empty => {
                        if !feed.check_entry_limit(reader, &mut buf, limits, depth)? {
                            continue;
                        }

                        let mut entry_ctx = base_ctx.child();
                        if let Some(xml_base) =
                            extract_xml_base(&element, limits.max_attribute_length)
                        {
                            entry_ctx.update_base(&xml_base);
                        }

                        match parse_entry(reader, &mut buf, limits, depth, &entry_ctx) {
                            Ok(entry) => feed.entries.push(entry),
                            Err(e) => {
                                feed.bozo = true;
                                feed.bozo_exception = Some(e.to_string());
                            }
                        }
                    }
                    tag => {
                        // Check for namespace elements
                        let handled = if let Some(dc_element) = is_dc_tag(tag) {
                            let dc_elem = dc_element.to_string();
                            if !is_empty {
                                let text = read_text(reader, &mut buf, limits)?;
                                dublin_core::handle_feed_element(&dc_elem, &text, &mut feed.feed);
                            }
                            true
                        } else if let Some(_content_element) = is_content_tag(tag) {
                            // Content namespace - typically entry-level
                            if !is_empty {
                                skip_element(reader, &mut buf, limits, *depth)?;
                            }
                            true
                        } else if let Some(_media_element) = is_media_tag(tag) {
                            // Media RSS - typically entry-level
                            if !is_empty {
                                skip_element(reader, &mut buf, limits, *depth)?;
                            }
                            true
                        } else {
                            false
                        };

                        if !handled && !is_empty {
                            skip_element(reader, &mut buf, limits, *depth)?;
                        }
                    }
                }
                *depth = depth.saturating_sub(1);
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"feed" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(())
}

/// Parse <entry> element
#[allow(clippy::too_many_lines)]
fn parse_entry(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
    depth: &mut usize,
    base_ctx: &BaseUrlContext,
) -> Result<Entry> {
    let mut entry = Entry::with_capacity();

    loop {
        match reader.read_event_into(buf) {
            Ok(event @ (Event::Start(_) | Event::Empty(_))) => {
                let is_empty = matches!(event, Event::Empty(_));
                let (Event::Start(e) | Event::Empty(e)) = &event else {
                    unreachable!()
                };

                *depth += 1;
                check_depth(*depth, limits.max_nesting_depth)?;

                let element = e.to_owned();
                // Use name() instead of local_name() to preserve namespace prefixes
                match element.name().as_ref() {
                    b"title" if !is_empty => {
                        let text = parse_text_construct(reader, buf, &element, limits)?;
                        entry.set_title(text);
                    }
                    b"link" => {
                        if let Some(mut link) = Link::from_attributes(
                            element.attributes().flatten(),
                            limits.max_attribute_length,
                        ) {
                            link.href = base_ctx.resolve_safe(&link.href);

                            if entry.link.is_none() && link.rel.as_deref() == Some("alternate") {
                                entry.link = Some(link.href.clone());
                            }
                            entry
                                .links
                                .try_push_limited(link, limits.max_links_per_entry);
                        }
                        if !is_empty {
                            skip_to_end(reader, buf, b"link")?;
                        }
                    }
                    b"id" if !is_empty => {
                        entry.id = Some(read_text(reader, buf, limits)?);
                    }
                    b"updated" if !is_empty => {
                        let text = read_text(reader, buf, limits)?;
                        entry.updated = parse_date(&text);
                    }
                    b"published" if !is_empty => {
                        let text = read_text(reader, buf, limits)?;
                        entry.published = parse_date(&text);
                    }
                    b"summary" if !is_empty => {
                        let text = parse_text_construct(reader, buf, &element, limits)?;
                        entry.set_summary(text);
                    }
                    b"content" if !is_empty => {
                        let content = parse_content(reader, buf, &element, limits)?;
                        entry
                            .content
                            .try_push_limited(content, limits.max_content_blocks);
                    }
                    b"author" if !is_empty => {
                        if let Ok(person) = parse_person(reader, buf, limits, depth) {
                            if entry.author.is_none() {
                                entry.set_author(person.clone());
                            }
                            entry.authors.try_push_limited(person, limits.max_authors);
                        }
                    }
                    b"contributor" if !is_empty => {
                        if let Ok(person) = parse_person(reader, buf, limits, depth) {
                            entry
                                .contributors
                                .try_push_limited(person, limits.max_contributors);
                        }
                    }
                    b"category" => {
                        if let Some(tag) = Tag::from_attributes(
                            element.attributes().flatten(),
                            limits.max_attribute_length,
                        ) {
                            entry.tags.try_push_limited(tag, limits.max_tags);
                        }
                        if !is_empty {
                            skip_to_end(reader, buf, b"category")?;
                        }
                    }
                    b"source" if !is_empty => {
                        if let Ok(source) = parse_atom_source(reader, buf, limits, depth) {
                            entry.source = Some(source);
                        }
                    }
                    tag => {
                        // Check for namespace elements
                        let handled = if let Some(dc_element) = is_dc_tag(tag) {
                            let dc_elem = dc_element.to_string();
                            if !is_empty {
                                let text = read_text(reader, buf, limits)?;
                                dublin_core::handle_entry_element(&dc_elem, &text, &mut entry);
                            }
                            true
                        } else if let Some(content_element) = is_content_tag(tag) {
                            let content_elem = content_element.to_string();
                            if !is_empty {
                                let text = read_text(reader, buf, limits)?;
                                content::handle_entry_element(&content_elem, &text, &mut entry);
                            }
                            true
                        } else if let Some(media_element) = is_media_tag(tag) {
                            // Media RSS namespace
                            if media_element == "thumbnail" {
                                if let Some(thumbnail) = MediaThumbnail::from_attributes(
                                    element.attributes().flatten(),
                                    limits.max_attribute_length,
                                ) {
                                    entry
                                        .media_thumbnails
                                        .try_push_limited(thumbnail, limits.max_enclosures);
                                }
                                if !is_empty {
                                    skip_element(reader, buf, limits, *depth)?;
                                }
                            } else if media_element == "content" {
                                if let Some(media) = MediaContent::from_attributes(
                                    element.attributes().flatten(),
                                    limits.max_attribute_length,
                                ) {
                                    entry
                                        .media_content
                                        .try_push_limited(media, limits.max_enclosures);
                                }
                                if !is_empty {
                                    skip_element(reader, buf, limits, *depth)?;
                                }
                            } else {
                                let media_elem = media_element.to_string();
                                if !is_empty {
                                    let text = read_text(reader, buf, limits)?;
                                    media_rss::handle_entry_element(&media_elem, &text, &mut entry);
                                }
                            }
                            true
                        } else {
                            false
                        };

                        if !handled && !is_empty {
                            skip_element(reader, buf, limits, *depth)?;
                        }
                    }
                }
                *depth = depth.saturating_sub(1);
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"entry" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(entry)
}

/// Parse Atom text construct (title, summary, rights, etc.)
fn parse_text_construct(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    e: &quick_xml::events::BytesStart,
    limits: &ParserLimits,
) -> Result<TextConstruct> {
    let mut content_type = TextType::Text;

    for attr in e.attributes().flatten() {
        if attr.value.len() > limits.max_attribute_length {
            continue;
        }
        if attr.key.as_ref() == b"type" {
            match attr.value.as_ref() {
                b"text" => content_type = TextType::Text,
                b"html" => content_type = TextType::Html,
                b"xhtml" => content_type = TextType::Xhtml,
                _ => {}
            }
        }
    }

    let value = read_text(reader, buf, limits)?;

    Ok(TextConstruct {
        value,
        content_type,
        language: None,
        base: None,
    })
}

/// Parse <person> element (author, contributor)
fn parse_person(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
    depth: &mut usize,
) -> Result<Person> {
    let mut name = None;
    let mut email = None;
    let mut uri = None;

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) => {
                *depth += 1;
                check_depth(*depth, limits.max_nesting_depth)?;

                match e.local_name().as_ref() {
                    b"name" => name = Some(read_text(reader, buf, limits)?),
                    b"email" => email = Some(read_text(reader, buf, limits)?),
                    b"uri" => uri = Some(read_text(reader, buf, limits)?),
                    _ => skip_element(reader, buf, limits, *depth)?,
                }
                *depth = depth.saturating_sub(1);
            }
            Ok(Event::End(e))
                if e.local_name().as_ref() == b"author"
                    || e.local_name().as_ref() == b"contributor" =>
            {
                break;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(Person { name, email, uri })
}

/// Parse <generator> element
fn parse_generator(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    e: &quick_xml::events::BytesStart,
    limits: &ParserLimits,
) -> Result<Generator> {
    let mut uri = None;
    let mut version = None;

    for attr in e.attributes().flatten() {
        if attr.value.len() > limits.max_attribute_length {
            continue;
        }
        match attr.key.as_ref() {
            b"uri" => uri = Some(bytes_to_string(&attr.value)),
            b"version" => version = Some(bytes_to_string(&attr.value)),
            _ => {}
        }
    }

    Ok(Generator {
        value: read_text(reader, buf, limits)?,
        uri,
        version,
    })
}

/// Parse <content> element
fn parse_content(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    e: &quick_xml::events::BytesStart,
    limits: &ParserLimits,
) -> Result<Content> {
    let mut content_type = None;

    for attr in e.attributes().flatten() {
        if attr.value.len() > limits.max_attribute_length {
            continue;
        }
        if attr.key.as_ref() == b"type" {
            content_type = Some(bytes_to_string(&attr.value));
        }
    }

    Ok(Content {
        value: read_text(reader, buf, limits)?,
        content_type,
        language: None,
        base: None,
    })
}

/// Parse <source> element (renamed to avoid confusion with RSS source)
fn parse_atom_source(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
    depth: &mut usize,
) -> Result<Source> {
    let mut title = None;
    let mut link = None;
    let mut id = None;

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                *depth += 1;
                check_depth(*depth, limits.max_nesting_depth)?;

                let element = e.to_owned();
                // Use name() instead of local_name() to preserve namespace prefixes
                match element.name().as_ref() {
                    b"title" => title = Some(read_text(reader, buf, limits)?),
                    b"link" => {
                        if let Some(l) = Link::from_attributes(
                            element.attributes().flatten(),
                            limits.max_attribute_length,
                        ) && link.is_none()
                        {
                            link = Some(l.href);
                        }
                        skip_to_end(reader, buf, b"link")?;
                    }
                    b"id" => id = Some(read_text(reader, buf, limits)?),
                    _ => skip_element(reader, buf, limits, *depth)?,
                }
                *depth = depth.saturating_sub(1);
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"source" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(Source { title, link, id })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_atom() {
        let xml = br#"<?xml version="1.0" encoding="utf-8"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <title>Example Feed</title>
            <link href="http://example.org/"/>
            <updated>2024-12-14T10:00:00Z</updated>
            <id>urn:uuid:60a76c80-d399-11d9-b93C-0003939e0af6</id>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.version, FeedVersion::Atom10);
        assert!(!feed.bozo);
        assert_eq!(feed.feed.title.as_deref(), Some("Example Feed"));
        assert_eq!(feed.feed.link.as_deref(), Some("http://example.org/"));
        assert!(feed.feed.updated.is_some());
    }

    #[test]
    fn test_parse_atom_with_entries() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <title>Test</title>
            <entry>
                <title>Entry 1</title>
                <link href="http://example.org/1"/>
                <id>entry1</id>
                <updated>2024-12-14T09:00:00Z</updated>
            </entry>
            <entry>
                <title>Entry 2</title>
                <id>entry2</id>
                <updated>2024-12-13T09:00:00Z</updated>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.entries.len(), 2);
        assert_eq!(feed.entries[0].title.as_deref(), Some("Entry 1"));
        assert_eq!(feed.entries[0].id.as_deref(), Some("entry1"));
    }

    #[test]
    fn test_parse_atom_with_author() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <author>
                <name>John Doe</name>
                <email>john@example.com</email>
                <uri>http://example.com/~john</uri>
            </author>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.feed.author.as_deref(), Some("John Doe"));
        assert_eq!(feed.feed.authors.len(), 1);
        assert_eq!(
            feed.feed.authors[0].email.as_deref(),
            Some("john@example.com")
        );
    }

    #[test]
    fn test_parse_atom_text_types() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <title type="text">Plain text</title>
            <subtitle type="html">&lt;b&gt;HTML&lt;/b&gt; content</subtitle>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert_eq!(
            feed.feed.title_detail.as_ref().unwrap().content_type,
            TextType::Text
        );
        assert_eq!(
            feed.feed.subtitle_detail.as_ref().unwrap().content_type,
            TextType::Html
        );
    }

    #[test]
    fn test_parse_atom_with_content() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <entry>
                <title>Test Entry</title>
                <id>test</id>
                <updated>2024-12-14T09:00:00Z</updated>
                <content type="html">&lt;p&gt;Content&lt;/p&gt;</content>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.entries[0].content.len(), 1);
        assert!(feed.entries[0].content[0].value.contains("Content"));
    }

    #[test]
    fn test_parse_atom_with_categories() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <title>Test</title>
            <category term="technology" scheme="http://example.com/categories" label="Tech"/>
            <category term="news"/>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.feed.tags.len(), 2);
        assert_eq!(feed.feed.tags[0].term, "technology");
        assert_eq!(feed.feed.tags[0].label.as_deref(), Some("Tech"));
    }

    #[test]
    fn test_parse_atom_with_generator() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <title>Test</title>
            <generator uri="http://example.com/" version="1.0">Example CMS</generator>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(feed.feed.generator_detail.is_some());
        let generator_detail = feed.feed.generator_detail.as_ref().unwrap();
        assert_eq!(generator_detail.uri.as_deref(), Some("http://example.com/"));
        assert_eq!(generator_detail.version.as_deref(), Some("1.0"));
    }

    #[test]
    fn test_parse_atom_with_icon_and_logo() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <icon>http://example.com/icon.png</icon>
            <logo>http://example.com/logo.png</logo>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert_eq!(
            feed.feed.icon.as_deref(),
            Some("http://example.com/icon.png")
        );
        assert_eq!(
            feed.feed.logo.as_deref(),
            Some("http://example.com/logo.png")
        );
    }

    #[test]
    fn test_parse_atom_with_rights() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <rights type="html">&lt;p&gt;Copyright 2024&lt;/p&gt;</rights>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(feed.feed.rights.is_some());
        assert!(feed.feed.rights_detail.is_some());
    }

    #[test]
    fn test_parse_atom_with_contributors() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <contributor>
                <name>Jane Doe</name>
                <email>jane@example.com</email>
            </contributor>
            <contributor>
                <name>Bob Smith</name>
            </contributor>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.feed.contributors.len(), 2);
        assert_eq!(feed.feed.contributors[0].name.as_deref(), Some("Jane Doe"));
    }

    #[test]
    fn test_parse_atom_entry_with_summary() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <entry>
                <title>Entry</title>
                <id>test</id>
                <updated>2024-12-14T09:00:00Z</updated>
                <summary type="text">This is a summary</summary>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert_eq!(
            feed.entries[0].summary.as_deref(),
            Some("This is a summary")
        );
        assert!(feed.entries[0].summary_detail.is_some());
    }

    #[test]
    fn test_parse_atom_entry_with_published() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <entry>
                <title>Entry</title>
                <id>test</id>
                <updated>2024-12-14T09:00:00Z</updated>
                <published>2024-12-13T09:00:00Z</published>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(feed.entries[0].published.is_some());
        assert!(feed.entries[0].updated.is_some());
    }

    #[test]
    fn test_parse_atom_entry_with_source() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <entry>
                <title>Entry</title>
                <id>test</id>
                <updated>2024-12-14T09:00:00Z</updated>
                <source>
                    <title>Source Feed</title>
                    <id>source-id</id>
                    <link href="http://source.example.com"/>
                </source>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(feed.entries[0].source.is_some());
        let source = feed.entries[0].source.as_ref().unwrap();
        assert_eq!(source.title.as_deref(), Some("Source Feed"));
        assert_eq!(source.id.as_deref(), Some("source-id"));
    }

    #[test]
    fn test_parse_atom_multiple_links() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <link href="http://example.com/" rel="alternate"/>
            <link href="http://example.com/feed" rel="self"/>
            <link href="http://example.com/related" rel="related"/>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.feed.links.len(), 3);
        assert_eq!(feed.feed.link.as_deref(), Some("http://example.com/"));
    }

    #[test]
    fn test_parse_atom_xhtml_content() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <title type="xhtml">
                <div xmlns="http://www.w3.org/1999/xhtml">XHTML Title</div>
            </title>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert_eq!(
            feed.feed.title_detail.as_ref().unwrap().content_type,
            TextType::Xhtml
        );
    }

    #[test]
    fn test_parse_atom_with_limits_exceeded() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <entry><title>E1</title><id>1</id><updated>2024-01-01T00:00:00Z</updated></entry>
            <entry><title>E2</title><id>2</id><updated>2024-01-01T00:00:00Z</updated></entry>
            <entry><title>E3</title><id>3</id><updated>2024-01-01T00:00:00Z</updated></entry>
        </feed>"#;

        let limits = ParserLimits {
            max_entries: 2,
            ..Default::default()
        };
        let feed = parse_atom10_with_limits(xml, limits).unwrap();
        assert_eq!(feed.entries.len(), 2);
    }

    #[test]
    fn test_parse_atom_malformed_continues() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <title>Valid Title</title>
            <invalid_tag>
                <nested>broken
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(feed.bozo);
        assert!(feed.feed.title.is_some());
    }

    #[test]
    fn test_parse_atom_empty_elements() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <link href="http://example.com/"/>
            <category term="test"/>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.feed.links.len(), 1);
        assert_eq!(feed.feed.tags.len(), 1);
    }
}
