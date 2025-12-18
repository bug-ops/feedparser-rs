//! RSS 2.0 parser implementation

use crate::{
    ParserLimits,
    error::{FeedError, Result},
    namespace::{content, dublin_core, media_rss},
    types::{
        Enclosure, Entry, FeedVersion, Image, ItunesCategory, ItunesEntryMeta, ItunesFeedMeta,
        ItunesOwner, Link, MediaContent, MediaThumbnail, ParsedFeed, PodcastFunding, PodcastMeta,
        PodcastPerson, PodcastTranscript, Source, Tag, TextConstruct, TextType, parse_duration,
        parse_explicit,
    },
    util::{base_url::BaseUrlContext, parse_date, text::truncate_to_length},
};
use quick_xml::{Reader, events::Event};

use super::common::{
    EVENT_BUFFER_CAPACITY, LimitedCollectionExt, check_depth, init_feed, is_content_tag, is_dc_tag,
    is_itunes_tag, is_media_tag, read_text, skip_element,
};

/// Error message for malformed XML attributes (shared constant)
const MALFORMED_ATTRIBUTES_ERROR: &str = "Malformed XML attributes";

/// Extract attributes as owned key-value pairs
/// Returns (attributes, `has_errors`) tuple where `has_errors` indicates
/// if any attribute parsing errors occurred (for bozo flag)
///
/// Note: Keys are cloned to `Vec<u8>` because `quick_xml::Attribute` owns the key
/// data only for the lifetime of the event, but we need to store attributes across
/// multiple parsing calls in `parse_enclosure` and other functions.
#[inline]
fn collect_attributes(e: &quick_xml::events::BytesStart) -> (Vec<(Vec<u8>, String)>, bool) {
    let mut has_errors = false;
    let attrs = e
        .attributes()
        .filter_map(|result| {
            if let Ok(attr) = result {
                if let Ok(v) = attr.unescape_value() {
                    Some((attr.key.as_ref().to_vec(), v.to_string()))
                } else {
                    has_errors = true;
                    None
                }
            } else {
                has_errors = true;
                None
            }
        })
        .collect();
    (attrs, has_errors)
}

/// Find attribute value by key
#[inline]
fn find_attribute<'a>(attrs: &'a [(Vec<u8>, String)], key: &[u8]) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k.as_slice() == key)
        .map(|(_, v)| v.as_str())
}

/// Parse RSS 2.0 feed from raw bytes
///
/// Parses an RSS 2.0 feed in tolerant mode, setting the bozo flag
/// on errors but continuing to extract as much data as possible.
///
/// # Arguments
///
/// * `data` - Raw RSS XML data
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
///     <rss version="2.0">
///         <channel>
///             <title>Example</title>
///         </channel>
///     </rss>
/// "#;
///
/// let feed = parse_rss20(xml).unwrap();
/// assert_eq!(feed.feed.title.as_deref(), Some("Example"));
/// ```
#[allow(dead_code)]
pub fn parse_rss20(data: &[u8]) -> Result<ParsedFeed> {
    parse_rss20_with_limits(data, ParserLimits::default())
}

/// Parse RSS 2.0 with custom parser limits
pub fn parse_rss20_with_limits(data: &[u8], limits: ParserLimits) -> Result<ParsedFeed> {
    limits
        .check_feed_size(data.len())
        .map_err(|e| FeedError::InvalidFormat(e.to_string()))?;

    let mut reader = Reader::from_reader(data);
    reader.config_mut().trim_text(true);

    let mut feed = init_feed(FeedVersion::Rss20, limits.max_entries);
    let mut buf = Vec::with_capacity(EVENT_BUFFER_CAPACITY);
    let mut depth: usize = 1;
    let mut base_ctx = BaseUrlContext::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.local_name().as_ref() == b"channel" => {
                depth += 1;
                if let Err(e) =
                    parse_channel(&mut reader, &mut feed, &limits, &mut depth, &mut base_ctx)
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

/// Parse <channel> element (feed metadata and items)
fn parse_channel(
    reader: &mut Reader<&[u8]>,
    feed: &mut ParsedFeed,
    limits: &ParserLimits,
    depth: &mut usize,
    base_ctx: &mut BaseUrlContext,
) -> Result<()> {
    let mut buf = Vec::with_capacity(EVENT_BUFFER_CAPACITY);

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                *depth += 1;
                check_depth(*depth, limits.max_nesting_depth)?;

                // NOTE: Allocation here is necessary due to borrow checker constraints.
                // We need owned tag data to pass &mut buf to helper functions simultaneously.
                // Potential future optimization: restructure helpers to avoid this allocation.
                let tag = e.name().as_ref().to_vec();
                let (attrs, has_attr_errors) = collect_attributes(&e);
                if has_attr_errors {
                    feed.bozo = true;
                    feed.bozo_exception = Some(MALFORMED_ATTRIBUTES_ERROR.to_string());
                }

                // Use full qualified name to distinguish standard RSS tags from namespaced tags
                match tag.as_slice() {
                    b"title" | b"link" | b"description" | b"language" | b"pubDate"
                    | b"managingEditor" | b"webMaster" | b"generator" | b"ttl" | b"category" => {
                        parse_channel_standard(reader, &mut buf, &tag, feed, limits, base_ctx)?;
                    }
                    b"image" => {
                        if let Ok(image) = parse_image(reader, &mut buf, limits, depth) {
                            feed.feed.image = Some(image);
                        }
                    }
                    b"item" => {
                        if !feed.check_entry_limit(reader, &mut buf, limits, depth)? {
                            continue;
                        }

                        match parse_item(reader, &mut buf, limits, depth, base_ctx) {
                            Ok((entry, has_attr_errors)) => {
                                if has_attr_errors {
                                    feed.bozo = true;
                                    feed.bozo_exception =
                                        Some(MALFORMED_ATTRIBUTES_ERROR.to_string());
                                }
                                feed.entries.push(entry);
                            }
                            Err(e) => {
                                feed.bozo = true;
                                feed.bozo_exception = Some(e.to_string());
                            }
                        }
                    }
                    _ => {
                        let mut handled = parse_channel_itunes(
                            reader, &mut buf, &tag, &attrs, feed, limits, depth,
                        )?;
                        if !handled {
                            handled = parse_channel_podcast(
                                reader, &mut buf, &tag, &attrs, feed, limits,
                            )?;
                        }
                        if !handled {
                            handled = parse_channel_namespace(
                                reader, &mut buf, &tag, feed, limits, *depth,
                            )?;
                        }

                        if !handled {
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

/// Parse enclosure element from attributes
#[inline]
fn parse_enclosure(attrs: &[(Vec<u8>, String)], limits: &ParserLimits) -> Option<Enclosure> {
    let mut url = String::new();
    let mut length = None;
    let mut enc_type = None;

    for (key, value) in attrs {
        match key.as_slice() {
            b"url" => url = truncate_to_length(value, limits.max_attribute_length),
            b"length" => length = value.parse().ok(),
            b"type" => enc_type = Some(truncate_to_length(value, limits.max_attribute_length)),
            _ => {}
        }
    }

    if url.is_empty() {
        None
    } else {
        Some(Enclosure {
            url,
            length,
            enclosure_type: enc_type,
        })
    }
}

/// Parse standard RSS 2.0 channel elements
#[inline]
fn parse_channel_standard(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    tag: &[u8],
    feed: &mut ParsedFeed,
    limits: &ParserLimits,
    base_ctx: &mut BaseUrlContext,
) -> Result<()> {
    match tag {
        b"title" => {
            feed.feed.title = Some(read_text(reader, buf, limits)?);
        }
        b"link" => {
            let link_text = read_text(reader, buf, limits)?;
            feed.feed
                .set_alternate_link(link_text.clone(), limits.max_links_per_feed);

            if base_ctx.base().is_none() {
                base_ctx.update_base(&link_text);
            }
        }
        b"description" => {
            feed.feed.subtitle = Some(read_text(reader, buf, limits)?);
        }
        b"language" => {
            feed.feed.language = Some(read_text(reader, buf, limits)?);
        }
        b"pubDate" => {
            let text = read_text(reader, buf, limits)?;
            match parse_date(&text) {
                Some(dt) => feed.feed.published = Some(dt),
                None if !text.is_empty() => {
                    feed.bozo = true;
                    feed.bozo_exception = Some("Invalid pubDate format".to_string());
                }
                None => {}
            }
        }
        b"managingEditor" => {
            feed.feed.author = Some(read_text(reader, buf, limits)?);
        }
        b"webMaster" => {
            feed.feed.publisher = Some(read_text(reader, buf, limits)?);
        }
        b"generator" => {
            feed.feed.generator = Some(read_text(reader, buf, limits)?);
        }
        b"ttl" => {
            let text = read_text(reader, buf, limits)?;
            feed.feed.ttl = text.parse().ok();
        }
        b"category" => {
            let term = read_text(reader, buf, limits)?;
            feed.feed.tags.try_push_limited(
                Tag {
                    term,
                    scheme: None,
                    label: None,
                },
                limits.max_tags,
            );
        }
        _ => {}
    }
    Ok(())
}

/// Parse iTunes namespace tags at channel level
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_channel_itunes(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    limits: &ParserLimits,
    depth: &mut usize,
) -> Result<bool> {
    if is_itunes_tag(tag, b"author") {
        let text = read_text(reader, buf, limits)?;
        let itunes = feed.feed.itunes.get_or_insert_with(ItunesFeedMeta::default);
        itunes.author = Some(text);
        Ok(true)
    } else if is_itunes_tag(tag, b"owner") {
        let itunes = feed.feed.itunes.get_or_insert_with(ItunesFeedMeta::default);
        if let Ok(owner) = parse_itunes_owner(reader, buf, limits, depth) {
            itunes.owner = Some(owner);
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"category") {
        parse_itunes_category(reader, buf, attrs, feed, limits);
        Ok(true)
    } else if is_itunes_tag(tag, b"explicit") {
        let text = read_text(reader, buf, limits)?;
        let itunes = feed.feed.itunes.get_or_insert_with(ItunesFeedMeta::default);
        itunes.explicit = parse_explicit(&text);
        Ok(true)
    } else if is_itunes_tag(tag, b"image") {
        if let Some(value) = find_attribute(attrs, b"href") {
            let itunes = feed.feed.itunes.get_or_insert_with(ItunesFeedMeta::default);
            itunes.image = Some(truncate_to_length(value, limits.max_attribute_length));
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"keywords") {
        let text = read_text(reader, buf, limits)?;
        let itunes = feed.feed.itunes.get_or_insert_with(ItunesFeedMeta::default);
        itunes.keywords = text
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Ok(true)
    } else if is_itunes_tag(tag, b"type") {
        let text = read_text(reader, buf, limits)?;
        let itunes = feed.feed.itunes.get_or_insert_with(ItunesFeedMeta::default);
        itunes.podcast_type = Some(text);
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse iTunes category with potential subcategory
fn parse_itunes_category(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    limits: &ParserLimits,
) {
    let category_text = find_attribute(attrs, b"text")
        .map(|v| truncate_to_length(v, limits.max_attribute_length))
        .unwrap_or_default();

    // Parse potential nested subcategory
    let mut subcategory_text = None;
    let mut nesting = 0;
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(sub_e)) => {
                if is_itunes_tag(sub_e.name().as_ref(), b"category") {
                    nesting += 1;
                    if nesting == 1 {
                        for attr in sub_e.attributes().flatten() {
                            if attr.key.as_ref() == b"text"
                                && let Ok(value) = attr.unescape_value()
                            {
                                subcategory_text =
                                    Some(value.chars().take(limits.max_attribute_length).collect());
                                break;
                            }
                        }
                    }
                }
            }
            Ok(Event::Empty(sub_e)) => {
                if is_itunes_tag(sub_e.name().as_ref(), b"category") && subcategory_text.is_none() {
                    for attr in sub_e.attributes().flatten() {
                        if attr.key.as_ref() == b"text"
                            && let Ok(value) = attr.unescape_value()
                        {
                            subcategory_text =
                                Some(value.chars().take(limits.max_attribute_length).collect());
                            break;
                        }
                    }
                }
            }
            Ok(Event::End(end_e)) => {
                if is_itunes_tag(end_e.name().as_ref(), b"category") {
                    if nesting == 0 {
                        break;
                    }
                    nesting -= 1;
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    let itunes = feed.feed.itunes.get_or_insert_with(ItunesFeedMeta::default);
    itunes.categories.push(ItunesCategory {
        text: category_text,
        subcategory: subcategory_text,
    });
}

/// Parse Podcast 2.0 namespace tags at channel level
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
#[inline]
fn parse_channel_podcast(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    limits: &ParserLimits,
) -> Result<bool> {
    if tag.starts_with(b"podcast:guid") {
        let text = read_text(reader, buf, limits)?;
        let podcast = feed.feed.podcast.get_or_insert_with(PodcastMeta::default);
        podcast.guid = Some(text);
        Ok(true)
    } else if tag.starts_with(b"podcast:funding") {
        let url = find_attribute(attrs, b"url")
            .map(|v| truncate_to_length(v, limits.max_attribute_length))
            .unwrap_or_default();
        let message_text = read_text(reader, buf, limits)?;
        let message = if message_text.is_empty() {
            None
        } else {
            Some(message_text)
        };
        let podcast = feed.feed.podcast.get_or_insert_with(PodcastMeta::default);
        podcast.funding.push(PodcastFunding { url, message });
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse Dublin Core, Content, and Media RSS namespace tags at channel level
#[inline]
fn parse_channel_namespace(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    tag: &[u8],
    feed: &mut ParsedFeed,
    limits: &ParserLimits,
    depth: usize,
) -> Result<bool> {
    if let Some(dc_element) = is_dc_tag(tag) {
        let dc_elem = dc_element.to_string();
        let text = read_text(reader, buf, limits)?;
        dublin_core::handle_feed_element(&dc_elem, &text, &mut feed.feed);
        Ok(true)
    } else if let Some(_content_element) = is_content_tag(tag) {
        skip_element(reader, buf, limits, depth)?;
        Ok(true)
    } else if let Some(_media_element) = is_media_tag(tag) {
        skip_element(reader, buf, limits, depth)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse <item> element (entry)
///
/// Returns a tuple where:
/// - First element: the parsed `Entry`
/// - Second element: `bool` indicating whether attribute parsing errors occurred (for bozo flag)
fn parse_item(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
    depth: &mut usize,
    base_ctx: &BaseUrlContext,
) -> Result<(Entry, bool)> {
    let mut entry = Entry::with_capacity();
    let mut has_attr_errors = false;

    loop {
        match reader.read_event_into(buf) {
            Ok(event @ (Event::Start(_) | Event::Empty(_))) => {
                let is_empty = matches!(event, Event::Empty(_));
                let (Event::Start(e) | Event::Empty(e)) = &event else {
                    unreachable!()
                };

                *depth += 1;
                check_depth(*depth, limits.max_nesting_depth)?;

                // NOTE: Allocation here is necessary due to borrow checker constraints.
                // We need owned tag data to pass &mut buf to helper functions simultaneously.
                // Potential future optimization: restructure helpers to avoid this allocation.
                let tag = e.name().as_ref().to_vec();
                let (attrs, attr_error) = collect_attributes(e);
                if attr_error {
                    has_attr_errors = true;
                }

                // Use full qualified name to distinguish standard RSS tags from namespaced tags
                match tag.as_slice() {
                    b"title" | b"link" | b"description" | b"guid" | b"pubDate" | b"author"
                    | b"category" | b"comments" => {
                        parse_item_standard(reader, buf, &tag, &mut entry, limits, base_ctx)?;
                    }
                    b"enclosure" => {
                        if let Some(mut enclosure) = parse_enclosure(&attrs, limits) {
                            enclosure.url = base_ctx.resolve_safe(&enclosure.url);
                            entry
                                .enclosures
                                .try_push_limited(enclosure, limits.max_enclosures);
                        }
                        skip_element(reader, buf, limits, *depth)?;
                    }
                    b"source" => {
                        if let Ok(source) = parse_source(reader, buf, limits, depth) {
                            entry.source = Some(source);
                        }
                    }
                    _ => {
                        let mut handled =
                            parse_item_itunes(reader, buf, &tag, &attrs, &mut entry, limits)?;
                        if !handled {
                            handled = parse_item_podcast(
                                reader, buf, &tag, &attrs, &mut entry, limits, is_empty, *depth,
                            )?;
                        }
                        if !handled {
                            handled = parse_item_namespace(
                                reader, buf, &tag, &attrs, &mut entry, limits, is_empty, *depth,
                            )?;
                        }

                        if !handled {
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

    Ok((entry, has_attr_errors))
}

/// Parse standard RSS 2.0 item elements
#[inline]
fn parse_item_standard(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    tag: &[u8],
    entry: &mut Entry,
    limits: &ParserLimits,
    base_ctx: &BaseUrlContext,
) -> Result<()> {
    match tag {
        b"title" => {
            entry.title = Some(read_text(reader, buf, limits)?);
        }
        b"link" => {
            let link_text = read_text(reader, buf, limits)?;
            let resolved_link = base_ctx.resolve_safe(&link_text);
            entry.link = Some(resolved_link.clone());
            entry.links.try_push_limited(
                Link {
                    href: resolved_link,
                    rel: Some("alternate".to_string()),
                    ..Default::default()
                },
                limits.max_links_per_entry,
            );
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
        b"guid" => {
            entry.id = Some(read_text(reader, buf, limits)?);
        }
        b"pubDate" => {
            let text = read_text(reader, buf, limits)?;
            entry.published = parse_date(&text);
        }
        b"author" => {
            entry.author = Some(read_text(reader, buf, limits)?);
        }
        b"category" => {
            let term = read_text(reader, buf, limits)?;
            entry.tags.try_push_limited(
                Tag {
                    term,
                    scheme: None,
                    label: None,
                },
                limits.max_tags,
            );
        }
        b"comments" => {
            entry.comments = Some(read_text(reader, buf, limits)?);
        }
        _ => {}
    }
    Ok(())
}

/// Parse iTunes namespace tags at item level
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
#[inline]
fn parse_item_itunes(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    limits: &ParserLimits,
) -> Result<bool> {
    if is_itunes_tag(tag, b"title") {
        let text = read_text(reader, buf, limits)?;
        let itunes = entry.itunes.get_or_insert_with(ItunesEntryMeta::default);
        itunes.title = Some(text);
        Ok(true)
    } else if is_itunes_tag(tag, b"author") {
        let text = read_text(reader, buf, limits)?;
        let itunes = entry.itunes.get_or_insert_with(ItunesEntryMeta::default);
        itunes.author = Some(text);
        Ok(true)
    } else if is_itunes_tag(tag, b"duration") {
        let text = read_text(reader, buf, limits)?;
        let itunes = entry.itunes.get_or_insert_with(ItunesEntryMeta::default);
        itunes.duration = parse_duration(&text);
        Ok(true)
    } else if is_itunes_tag(tag, b"explicit") {
        let text = read_text(reader, buf, limits)?;
        let itunes = entry.itunes.get_or_insert_with(ItunesEntryMeta::default);
        itunes.explicit = parse_explicit(&text);
        Ok(true)
    } else if is_itunes_tag(tag, b"image") {
        if let Some(value) = find_attribute(attrs, b"href") {
            let itunes = entry.itunes.get_or_insert_with(ItunesEntryMeta::default);
            itunes.image = Some(truncate_to_length(value, limits.max_attribute_length));
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"episode") {
        let text = read_text(reader, buf, limits)?;
        let itunes = entry.itunes.get_or_insert_with(ItunesEntryMeta::default);
        itunes.episode = text.parse().ok();
        Ok(true)
    } else if is_itunes_tag(tag, b"season") {
        let text = read_text(reader, buf, limits)?;
        let itunes = entry.itunes.get_or_insert_with(ItunesEntryMeta::default);
        itunes.season = text.parse().ok();
        Ok(true)
    } else if is_itunes_tag(tag, b"episodeType") {
        let text = read_text(reader, buf, limits)?;
        let itunes = entry.itunes.get_or_insert_with(ItunesEntryMeta::default);
        itunes.episode_type = Some(text);
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse Podcast 2.0 namespace tags at item level
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
///
/// Note: Uses 8 parameters instead of a context struct due to borrow checker constraints
/// with multiple simultaneous `&mut` references during parsing.
#[inline]
#[allow(clippy::too_many_arguments)]
fn parse_item_podcast(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    limits: &ParserLimits,
    is_empty: bool,
    depth: usize,
) -> Result<bool> {
    if tag.starts_with(b"podcast:transcript") {
        parse_podcast_transcript(reader, buf, attrs, entry, limits, is_empty, depth)?;
        Ok(true)
    } else if tag.starts_with(b"podcast:person") {
        parse_podcast_person(reader, buf, attrs, entry, limits)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse Podcast 2.0 transcript element
///
/// Note: Currently always returns `Ok(())` but uses `Result` return type
/// for consistency with other parsers and potential future error handling.
fn parse_podcast_transcript(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    limits: &ParserLimits,
    is_empty: bool,
    depth: usize,
) -> Result<()> {
    let url = find_attribute(attrs, b"url")
        .map(|v| truncate_to_length(v, limits.max_attribute_length))
        .unwrap_or_default();
    let transcript_type =
        find_attribute(attrs, b"type").map(|v| truncate_to_length(v, limits.max_attribute_length));
    let language = find_attribute(attrs, b"language")
        .map(|v| truncate_to_length(v, limits.max_attribute_length));
    let rel =
        find_attribute(attrs, b"rel").map(|v| truncate_to_length(v, limits.max_attribute_length));

    if !url.is_empty() {
        entry.podcast_transcripts.push(PodcastTranscript {
            url,
            transcript_type,
            language,
            rel,
        });
    }

    if !is_empty {
        skip_element(reader, buf, limits, depth)?;
    }

    Ok(())
}

/// Parse Podcast 2.0 person element
fn parse_podcast_person(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    limits: &ParserLimits,
) -> Result<()> {
    let role =
        find_attribute(attrs, b"role").map(|v| truncate_to_length(v, limits.max_attribute_length));
    let group =
        find_attribute(attrs, b"group").map(|v| truncate_to_length(v, limits.max_attribute_length));
    let img =
        find_attribute(attrs, b"img").map(|v| truncate_to_length(v, limits.max_attribute_length));
    let href =
        find_attribute(attrs, b"href").map(|v| truncate_to_length(v, limits.max_attribute_length));

    let name = read_text(reader, buf, limits)?;
    if !name.is_empty() {
        entry.podcast_persons.push(PodcastPerson {
            name,
            role,
            group,
            img,
            href,
        });
    }

    Ok(())
}

/// Parse Dublin Core, Content, and Media RSS namespace tags at item level
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
///
/// Note: Uses 8 parameters instead of a context struct due to borrow checker constraints
/// with multiple simultaneous `&mut` references during parsing.
#[inline]
#[allow(clippy::too_many_arguments)]
fn parse_item_namespace(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    limits: &ParserLimits,
    is_empty: bool,
    depth: usize,
) -> Result<bool> {
    if let Some(dc_element) = is_dc_tag(tag) {
        let dc_elem = dc_element.to_string();
        let text = read_text(reader, buf, limits)?;
        dublin_core::handle_entry_element(&dc_elem, &text, entry);
        Ok(true)
    } else if let Some(content_element) = is_content_tag(tag) {
        let content_elem = content_element.to_string();
        let text = read_text(reader, buf, limits)?;
        content::handle_entry_element(&content_elem, &text, entry);
        Ok(true)
    } else if let Some(media_element) = is_media_tag(tag) {
        parse_item_media(
            reader,
            buf,
            media_element,
            attrs,
            entry,
            limits,
            is_empty,
            depth,
        )?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse Media RSS namespace elements
#[allow(clippy::too_many_arguments)]
fn parse_item_media(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    media_element: &str,
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    limits: &ParserLimits,
    is_empty: bool,
    depth: usize,
) -> Result<()> {
    match media_element {
        "thumbnail" => {
            let url = find_attribute(attrs, b"url")
                .map(|v| truncate_to_length(v, limits.max_attribute_length))
                .unwrap_or_default();
            let width = find_attribute(attrs, b"width").and_then(|v| v.parse().ok());
            let height = find_attribute(attrs, b"height").and_then(|v| v.parse().ok());

            if !url.is_empty() {
                entry
                    .media_thumbnails
                    .try_push_limited(MediaThumbnail { url, width, height }, limits.max_enclosures);
            }
            if !is_empty {
                skip_element(reader, buf, limits, depth)?;
            }
        }
        "content" => {
            let url = find_attribute(attrs, b"url")
                .map(|v| truncate_to_length(v, limits.max_attribute_length))
                .unwrap_or_default();
            let content_type = find_attribute(attrs, b"type")
                .map(|v| truncate_to_length(v, limits.max_attribute_length));
            let filesize = find_attribute(attrs, b"fileSize").and_then(|v| v.parse().ok());
            let duration = find_attribute(attrs, b"duration").and_then(|v| v.parse().ok());
            let width = find_attribute(attrs, b"width").and_then(|v| v.parse().ok());
            let height = find_attribute(attrs, b"height").and_then(|v| v.parse().ok());

            if !url.is_empty() {
                entry.media_content.try_push_limited(
                    MediaContent {
                        url,
                        content_type,
                        filesize,
                        width,
                        height,
                        duration,
                    },
                    limits.max_enclosures,
                );
            }
            if !is_empty {
                skip_element(reader, buf, limits, depth)?;
            }
        }
        _ => {
            let media_elem = media_element.to_string();
            let text = read_text(reader, buf, limits)?;
            media_rss::handle_entry_element(&media_elem, &text, entry);
        }
    }
    Ok(())
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
    let mut width = None;
    let mut height = None;
    let mut description = None;

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) => {
                *depth += 1;
                check_depth(*depth, limits.max_nesting_depth)?;

                match e.local_name().as_ref() {
                    b"url" => url = read_text(reader, buf, limits)?,
                    b"title" => title = Some(read_text(reader, buf, limits)?),
                    b"link" => link = Some(read_text(reader, buf, limits)?),
                    b"width" => {
                        if let Ok(w) = read_text(reader, buf, limits)?.parse() {
                            width = Some(w);
                        }
                    }
                    b"height" => {
                        if let Ok(h) = read_text(reader, buf, limits)?.parse() {
                            height = Some(h);
                        }
                    }
                    b"description" => description = Some(read_text(reader, buf, limits)?),
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
        width,
        height,
        description,
    })
}

/// Parse <source> element
fn parse_source(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
    depth: &mut usize,
) -> Result<Source> {
    let mut title = None;
    let mut link = None;
    let id = None;

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) => {
                *depth += 1;
                check_depth(*depth, limits.max_nesting_depth)?;

                match e.local_name().as_ref() {
                    b"title" => title = Some(read_text(reader, buf, limits)?),
                    b"url" => link = Some(read_text(reader, buf, limits)?),
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

/// Parse iTunes owner from <itunes:owner> element
fn parse_itunes_owner(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
    depth: &mut usize,
) -> Result<ItunesOwner> {
    let mut owner = ItunesOwner::default();

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) => {
                *depth += 1;
                check_depth(*depth, limits.max_nesting_depth)?;

                let tag_name = e.local_name();
                if is_itunes_tag(tag_name.as_ref(), b"name") {
                    owner.name = Some(read_text(reader, buf, limits)?);
                } else if is_itunes_tag(tag_name.as_ref(), b"email") {
                    owner.email = Some(read_text(reader, buf, limits)?);
                } else {
                    skip_element(reader, buf, limits, *depth)?;
                }
                *depth = depth.saturating_sub(1);
            }
            Ok(Event::End(_) | Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(owner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_basic_rss() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title>Test Feed</title>
                <link>http://example.com</link>
                <description>Test description</description>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.version, FeedVersion::Rss20);
        assert!(!feed.bozo);
        assert_eq!(feed.feed.title.as_deref(), Some("Test Feed"));
        assert_eq!(feed.feed.link.as_deref(), Some("http://example.com"));
        assert_eq!(feed.feed.subtitle.as_deref(), Some("Test description"));
    }

    #[test]
    fn test_parse_rss_with_items() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title>Test</title>
                <item>
                    <title>Item 1</title>
                    <link>http://example.com/1</link>
                    <description>Description 1</description>
                    <guid>item-1</guid>
                </item>
                <item>
                    <title>Item 2</title>
                    <link>http://example.com/2</link>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.entries.len(), 2);
        assert_eq!(feed.entries[0].title.as_deref(), Some("Item 1"));
        assert_eq!(feed.entries[0].id.as_deref(), Some("item-1"));
        assert_eq!(feed.entries[1].title.as_deref(), Some("Item 2"));
    }

    #[test]
    fn test_parse_rss_with_dates() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <pubDate>Sat, 14 Dec 2024 10:30:00 +0000</pubDate>
                <item>
                    <pubDate>Fri, 13 Dec 2024 09:00:00 +0000</pubDate>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(feed.feed.published.is_some());
        assert!(feed.entries[0].published.is_some());

        let dt = feed.feed.published.unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 14);
    }

    #[test]
    fn test_parse_rss_with_invalid_date() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <pubDate>not a date</pubDate>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(feed.bozo);
        assert!(feed.bozo_exception.is_some());
        assert!(feed.bozo_exception.unwrap().contains("Invalid pubDate"));
    }

    #[test]
    fn test_parse_rss_with_categories() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item>
                    <category>Tech</category>
                    <category>News</category>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.entries[0].tags.len(), 2);
        assert_eq!(feed.entries[0].tags[0].term, "Tech");
        assert_eq!(feed.entries[0].tags[1].term, "News");
    }

    #[test]
    fn test_parse_rss_with_enclosure() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item>
                    <enclosure url="http://example.com/audio.mp3"
                               length="12345"
                               type="audio/mpeg"/>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.entries[0].enclosures.len(), 1);
        assert_eq!(
            feed.entries[0].enclosures[0].url,
            "http://example.com/audio.mp3"
        );
        assert_eq!(feed.entries[0].enclosures[0].length, Some(12345));
        assert_eq!(
            feed.entries[0].enclosures[0].enclosure_type.as_deref(),
            Some("audio/mpeg")
        );
    }

    #[test]
    fn test_parse_rss_malformed_continues() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title>Test</title>
                <item>
                    <title>Item 1</title>
                </item>
                <!-- Missing close tag but continues -->
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        // Should still extract some data
        assert_eq!(feed.feed.title.as_deref(), Some("Test"));
    }

    #[test]
    fn test_parse_rss_with_cdata() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item>
                    <description><![CDATA[HTML <b>content</b> here]]></description>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(
            feed.entries[0].summary.as_deref(),
            Some("HTML <b>content</b> here")
        );
    }

    #[test]
    fn test_parse_rss_with_image() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <image>
                    <url>http://example.com/logo.png</url>
                    <title>Example Logo</title>
                    <link>http://example.com</link>
                    <width>144</width>
                    <height>36</height>
                </image>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(feed.feed.image.is_some());
        let img = feed.feed.image.as_ref().unwrap();
        assert_eq!(img.url, "http://example.com/logo.png");
        assert_eq!(img.title.as_deref(), Some("Example Logo"));
        assert_eq!(img.width, Some(144));
        assert_eq!(img.height, Some(36));
    }

    #[test]
    fn test_parse_rss_with_author() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item>
                    <author>john@example.com (John Doe)</author>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(
            feed.entries[0].author.as_deref(),
            Some("john@example.com (John Doe)")
        );
    }

    #[test]
    fn test_parse_rss_with_comments() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item>
                    <comments>http://example.com/comments</comments>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(
            feed.entries[0].comments.as_deref(),
            Some("http://example.com/comments")
        );
    }

    #[test]
    fn test_parse_rss_with_guid_permalink() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item>
                    <guid isPermaLink="true">http://example.com/1</guid>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.entries[0].id.as_deref(), Some("http://example.com/1"));
    }

    #[test]
    fn test_parse_rss_with_ttl() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <ttl>60</ttl>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.feed.ttl, Some(60));
    }

    #[test]
    fn test_parse_rss_with_language() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <language>en-US</language>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.feed.language.as_deref(), Some("en-US"));
    }

    #[test]
    fn test_parse_rss_with_generator() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <generator>WordPress 6.0</generator>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.feed.generator.as_deref(), Some("WordPress 6.0"));
    }

    #[test]
    fn test_parse_rss_with_limits() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item><title>1</title></item>
                <item><title>2</title></item>
                <item><title>3</title></item>
                <item><title>4</title></item>
            </channel>
        </rss>"#;

        let limits = ParserLimits {
            max_entries: 2,
            ..Default::default()
        };
        let feed = parse_rss20_with_limits(xml, limits).unwrap();
        assert_eq!(feed.entries.len(), 2);
    }

    #[test]
    fn test_parse_rss_multiple_categories_feed_level() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <category>Technology</category>
                <category>News</category>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.feed.tags.len(), 2);
        assert_eq!(feed.feed.tags[0].term, "Technology");
        assert_eq!(feed.feed.tags[1].term, "News");
    }

    #[test]
    fn test_parse_rss_with_source() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item>
                    <source>
                        <title>Source Feed</title>
                        <url>http://source.example.com</url>
                    </source>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(feed.entries[0].source.is_some());
        let source = feed.entries[0].source.as_ref().unwrap();
        assert_eq!(source.title.as_deref(), Some("Source Feed"));
        assert_eq!(source.link.as_deref(), Some("http://source.example.com"));
    }

    #[test]
    fn test_parse_rss_empty_elements() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title></title>
                <description></description>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(feed.feed.title.is_none() || feed.feed.title.as_deref() == Some(""));
    }

    #[test]
    fn test_parse_rss_nesting_depth_limit() {
        let mut xml = String::from(r#"<?xml version="1.0"?><rss version="2.0"><channel>"#);
        for _ in 0..150 {
            xml.push_str("<nested>");
        }
        xml.push_str("</channel></rss>");

        let limits = ParserLimits {
            max_nesting_depth: 100,
            ..Default::default()
        };
        let result = parse_rss20_with_limits(xml.as_bytes(), limits);
        assert!(result.is_err() || result.unwrap().bozo);
    }

    #[test]
    fn test_parse_rss_skip_cloud_element() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title>Test</title>
                <cloud domain="rpc.example.com" port="80" path="/RPC2"/>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.feed.title.as_deref(), Some("Test"));
    }

    // PRIORITY 1: iTunes Item-Level Tests (CRITICAL)

    #[test]
    fn test_parse_rss_itunes_episode_metadata() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Test Podcast</title>
                <item>
                    <title>Standard Title</title>
                    <itunes:title>iTunes Override Title</itunes:title>
                    <itunes:duration>1:23:45</itunes:duration>
                    <itunes:image href="https://example.com/episode-cover.jpg"/>
                    <itunes:explicit>yes</itunes:explicit>
                    <itunes:episode>42</itunes:episode>
                    <itunes:season>3</itunes:season>
                    <itunes:episodeType>full</itunes:episodeType>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(
            !feed.bozo,
            "Should parse iTunes episode metadata without errors"
        );
        assert_eq!(feed.entries.len(), 1);

        let entry = &feed.entries[0];
        let itunes = entry.itunes.as_ref().unwrap();

        assert_eq!(itunes.title.as_deref(), Some("iTunes Override Title"));
        assert_eq!(itunes.duration, Some(5025)); // 1:23:45 in seconds
        assert_eq!(
            itunes.image.as_deref(),
            Some("https://example.com/episode-cover.jpg")
        );
        assert_eq!(itunes.explicit, Some(true));
        assert_eq!(itunes.episode, Some(42));
        assert_eq!(itunes.season, Some(3));
        assert_eq!(itunes.episode_type.as_deref(), Some("full"));
    }

    #[test]
    fn test_parse_rss_itunes_duration_formats() {
        // Test HH:MM:SS format
        let xml1 = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <item><itunes:duration>1:23:45</itunes:duration></item>
            </channel>
        </rss>"#;
        let feed1 = parse_rss20(xml1).unwrap();
        assert_eq!(
            feed1.entries[0].itunes.as_ref().unwrap().duration,
            Some(5025)
        );

        // Test MM:SS format
        let xml2 = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <item><itunes:duration>23:45</itunes:duration></item>
            </channel>
        </rss>"#;
        let feed2 = parse_rss20(xml2).unwrap();
        assert_eq!(
            feed2.entries[0].itunes.as_ref().unwrap().duration,
            Some(1425)
        );

        // Test seconds-only format
        let xml3 = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <item><itunes:duration>3661</itunes:duration></item>
            </channel>
        </rss>"#;
        let feed3 = parse_rss20(xml3).unwrap();
        assert_eq!(
            feed3.entries[0].itunes.as_ref().unwrap().duration,
            Some(3661)
        );

        // Test invalid format
        let xml4 = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <item><itunes:duration>invalid</itunes:duration></item>
            </channel>
        </rss>"#;
        let feed4 = parse_rss20(xml4).unwrap();
        assert!(
            feed4.entries[0].itunes.as_ref().unwrap().duration.is_none(),
            "Invalid duration should result in None"
        );
    }

    #[test]
    fn test_parse_rss_itunes_nested_categories() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Test Podcast</title>
                <itunes:category text="Arts">
                    <itunes:category text="Design"/>
                </itunes:category>
                <itunes:category text="Technology">
                    <itunes:category text="Programming"/>
                </itunes:category>
                <itunes:category text="News"/>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        let itunes = feed.feed.itunes.as_ref().unwrap();

        assert_eq!(itunes.categories.len(), 3);

        // First category with subcategory
        assert_eq!(itunes.categories[0].text, "Arts");
        assert_eq!(itunes.categories[0].subcategory.as_deref(), Some("Design"));

        // Second category with subcategory
        assert_eq!(itunes.categories[1].text, "Technology");
        assert_eq!(
            itunes.categories[1].subcategory.as_deref(),
            Some("Programming")
        );

        // Third category without subcategory
        assert_eq!(itunes.categories[2].text, "News");
        assert!(itunes.categories[2].subcategory.is_none());
    }

    #[test]
    fn test_parse_rss_itunes_owner_parsing() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Test Podcast</title>
                <itunes:owner>
                    <itunes:name>John Smith</itunes:name>
                    <itunes:email>john@example.com</itunes:email>
                </itunes:owner>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        let itunes = feed.feed.itunes.as_ref().unwrap();
        let owner = itunes.owner.as_ref().unwrap();

        assert_eq!(owner.name.as_deref(), Some("John Smith"));
        assert_eq!(owner.email.as_deref(), Some("john@example.com"));
    }

    // PRIORITY 2: Podcast 2.0 Tests

    #[test]
    fn test_parse_rss_podcast_locked_and_guid() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Test Podcast</title>
                <podcast:guid>917393e3-1c1e-5d48-8e7f-cc9c0d9f2e95</podcast:guid>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);

        let podcast = feed.feed.podcast.as_ref().unwrap();
        assert_eq!(
            podcast.guid.as_deref(),
            Some("917393e3-1c1e-5d48-8e7f-cc9c0d9f2e95")
        );
    }

    #[test]
    fn test_parse_rss_podcast_funding() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Test Podcast</title>
                <podcast:funding url="https://patreon.com/example">Support on Patreon</podcast:funding>
                <podcast:funding url="https://buymeacoffee.com/example">Buy Me a Coffee</podcast:funding>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        let podcast = feed.feed.podcast.as_ref().unwrap();

        assert_eq!(podcast.funding.len(), 2);
        assert_eq!(podcast.funding[0].url, "https://patreon.com/example");
        assert_eq!(
            podcast.funding[0].message.as_deref(),
            Some("Support on Patreon")
        );
        assert_eq!(podcast.funding[1].url, "https://buymeacoffee.com/example");
    }

    #[test]
    fn test_parse_rss_podcast_transcript() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:transcript
                        url="https://example.com/transcripts/ep1.srt"
                        type="application/srt"
                        language="en"
                        rel="captions"/>
                    <podcast:transcript
                        url="https://example.com/transcripts/ep1.vtt"
                        type="text/vtt"/>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.entries.len(), 1);

        let transcripts = &feed.entries[0].podcast_transcripts;
        assert_eq!(transcripts.len(), 2);

        assert_eq!(
            transcripts[0].url,
            "https://example.com/transcripts/ep1.srt"
        );
        assert_eq!(
            transcripts[0].transcript_type.as_deref(),
            Some("application/srt")
        );
        assert_eq!(transcripts[0].language.as_deref(), Some("en"));
        assert_eq!(transcripts[0].rel.as_deref(), Some("captions"));

        assert_eq!(
            transcripts[1].url,
            "https://example.com/transcripts/ep1.vtt"
        );
        assert_eq!(transcripts[1].transcript_type.as_deref(), Some("text/vtt"));
    }

    #[test]
    fn test_parse_rss_podcast_person() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:person
                        role="host"
                        href="https://example.com/host"
                        img="https://example.com/host.jpg">Jane Doe</podcast:person>
                    <podcast:person role="guest">John Smith</podcast:person>
                    <podcast:person role="editor" group="production">Bob Editor</podcast:person>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        let persons = &feed.entries[0].podcast_persons;

        assert_eq!(persons.len(), 3);

        assert_eq!(persons[0].name, "Jane Doe");
        assert_eq!(persons[0].role.as_deref(), Some("host"));
        assert_eq!(persons[0].href.as_deref(), Some("https://example.com/host"));
        assert_eq!(
            persons[0].img.as_deref(),
            Some("https://example.com/host.jpg")
        );

        assert_eq!(persons[1].name, "John Smith");
        assert_eq!(persons[1].role.as_deref(), Some("guest"));

        assert_eq!(persons[2].name, "Bob Editor");
        assert_eq!(persons[2].role.as_deref(), Some("editor"));
        assert_eq!(persons[2].group.as_deref(), Some("production"));
    }

    // PRIORITY 3: Namespace Tests

    #[test]
    fn test_parse_rss_dublin_core_channel() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:dc="http://purl.org/dc/elements/1.1/">
            <channel>
                <title>Test Feed</title>
                <dc:creator>Jane Doe</dc:creator>
                <dc:publisher>Example Publishing</dc:publisher>
                <dc:date>2024-12-16T10:00:00Z</dc:date>
                <dc:rights>CC BY 4.0</dc:rights>
                <dc:subject>Technology</dc:subject>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);

        // DC creator should populate author
        assert_eq!(feed.feed.author.as_deref(), Some("Jane Doe"));

        // DC publisher
        assert_eq!(feed.feed.publisher.as_deref(), Some("Example Publishing"));

        // DC date should populate updated
        assert!(feed.feed.updated.is_some());

        // DC rights
        assert_eq!(feed.feed.rights.as_deref(), Some("CC BY 4.0"));

        // DC subject should add tags
        assert!(feed.feed.tags.iter().any(|t| t.term == "Technology"));
    }

    #[test]
    fn test_parse_rss_content_encoded() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:content="http://purl.org/rss/1.0/modules/content/">
            <channel>
                <item>
                    <title>Test Item</title>
                    <description>Plain text summary</description>
                    <content:encoded><![CDATA[
                        <p>This is <strong>HTML content</strong> with <a href="https://example.com">links</a></p>
                        <ul>
                            <li>Item 1</li>
                            <li>Item 2</li>
                        </ul>
                    ]]></content:encoded>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        let entry = &feed.entries[0];

        // Summary should be plain description
        assert_eq!(entry.summary.as_deref(), Some("Plain text summary"));

        // Content should contain the HTML
        assert_eq!(entry.content.len(), 1);
        assert!(
            entry.content[0]
                .value
                .contains("<strong>HTML content</strong>")
        );
        assert!(entry.content[0].value.contains("<ul>"));
    }
}
