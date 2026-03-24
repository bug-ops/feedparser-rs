//! RSS 2.0 parser implementation

use crate::{
    ParserLimits,
    error::{FeedError, Result},
    namespace::{content, dublin_core, georss, media_rss, slash, syndication, threading},
    types::{
        Enclosure, Entry, FeedVersion, Image, ItunesCategory, ItunesEntryMeta, ItunesFeedMeta,
        ItunesOwner, Link, MediaContent, MediaThumbnail, ParsedFeed, Person, PodcastChapters,
        PodcastEntryMeta, PodcastFunding, PodcastMeta, PodcastPerson, PodcastSoundbite,
        PodcastTranscript, Source, Tag, TextConstruct, TextType, parse_explicit,
    },
    util::{
        base_url::BaseUrlContext,
        parse_date,
        text::{parse_rss_person, truncate_to_length},
    },
};
use quick_xml::{Reader, events::Event};

use super::common::{
    EVENT_BUFFER_CAPACITY, LimitedCollectionExt, check_depth, extract_namespaces, extract_xml_lang,
    init_feed, is_content_tag, is_dc_tag, is_georss_tag, is_itunes_tag, is_media_tag, is_slash_tag,
    is_syn_tag, is_thr_tag, is_wfw_tag, read_text, read_text_str, skip_element,
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
///
/// Pre-allocates space for 4 attributes (typical for enclosures: url, type, length, maybe one more)
#[inline]
fn collect_attributes(e: &quick_xml::events::BytesStart) -> (Vec<(Vec<u8>, String)>, bool) {
    let mut has_errors = false;
    let mut attrs = Vec::with_capacity(4);

    for result in e.attributes() {
        match result {
            Ok(attr) => {
                if let Ok(v) = attr.unescape_value() {
                    attrs.push((attr.key.as_ref().to_vec(), v.to_string()));
                } else {
                    has_errors = true;
                }
            }
            Err(_) => {
                has_errors = true;
            }
        }
    }

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

    let mut feed = init_feed(FeedVersion::Rss20, limits.max_entries);
    let mut buf = Vec::with_capacity(EVENT_BUFFER_CAPACITY);
    let mut depth: usize = 1;
    let mut base_ctx = BaseUrlContext::new();
    let mut found_channel = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.local_name().as_ref() == b"rss" => {
                extract_namespaces(&e, &mut feed, &limits);
            }
            Ok(Event::Start(e)) if e.local_name().as_ref() == b"channel" => {
                // Some feeds declare xmlns on <channel> rather than <rss>
                extract_namespaces(&e, &mut feed, &limits);
                let channel_lang = extract_xml_lang(&e, limits.max_attribute_length);
                found_channel = true;
                depth += 1;
                if let Err(e) = parse_channel(
                    &mut reader,
                    &mut feed,
                    &limits,
                    &mut depth,
                    &mut base_ctx,
                    channel_lang.as_deref(),
                ) {
                    feed.bozo = true;
                    feed.bozo_exception = Some(e.to_string());
                }
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) => {
                if !found_channel {
                    feed.bozo = true;
                    feed.bozo_exception =
                        Some("Feed is truncated or has unclosed XML elements".to_string());
                }
                break;
            }
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
    channel_lang: Option<&str>,
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

                // NOTE: Allocation here is necessary due to borrow checker constraints.
                // We need owned tag data to pass &mut buf to helper functions simultaneously.
                // Potential future optimization: restructure helpers to avoid this allocation.
                let tag = e.name().as_ref().to_vec();
                let (attrs, has_attr_errors) = collect_attributes(e);
                if has_attr_errors {
                    feed.bozo = true;
                    feed.bozo_exception = Some(MALFORMED_ATTRIBUTES_ERROR.to_string());
                }

                // Extract xml:lang before matching to avoid borrow issues
                let item_lang = extract_xml_lang(e, limits.max_attribute_length);

                // Use full qualified name to distinguish standard RSS tags from namespaced tags
                match tag.as_slice() {
                    b"title" | b"link" | b"description" | b"language" | b"pubDate"
                    | b"lastBuildDate" | b"managingEditor" | b"webMaster" | b"generator"
                    | b"ttl" | b"docs" | b"copyright"
                        if !is_empty =>
                    {
                        parse_channel_standard(
                            reader,
                            &mut buf,
                            &tag,
                            &attrs,
                            feed,
                            limits,
                            base_ctx,
                            channel_lang,
                        )?;
                    }
                    b"category" => {
                        parse_channel_standard(
                            reader,
                            &mut buf,
                            &tag,
                            &attrs,
                            feed,
                            limits,
                            base_ctx,
                            channel_lang,
                        )?;
                    }
                    b"image" if !is_empty => {
                        if let Ok(image) = parse_image(reader, &mut buf, limits, depth) {
                            feed.feed.image = Some(image);
                        }
                    }
                    b"item" if !is_empty => {
                        parse_channel_item(
                            item_lang.as_deref(),
                            reader,
                            &mut buf,
                            feed,
                            limits,
                            depth,
                            base_ctx,
                            channel_lang,
                        )?;
                    }
                    _ => {
                        parse_channel_extension(
                            reader, &mut buf, &tag, &attrs, feed, limits, depth, is_empty,
                        )?;
                    }
                }
                *depth = depth.saturating_sub(1);
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"channel" => {
                break;
            }
            Ok(Event::Eof) => {
                feed.bozo = true;
                feed.bozo_exception =
                    Some("Feed is truncated or has unclosed XML elements".to_string());
                break;
            }
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(())
}

/// Parse <item> element within channel
///
/// Note: Uses 8 parameters instead of a context struct due to borrow checker constraints
/// with multiple simultaneous `&mut` references during parsing.
#[inline]
#[allow(clippy::too_many_arguments)]
fn parse_channel_item(
    item_lang: Option<&str>,
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    feed: &mut ParsedFeed,
    limits: &ParserLimits,
    depth: &mut usize,
    base_ctx: &BaseUrlContext,
    channel_lang: Option<&str>,
) -> Result<()> {
    if !feed.check_entry_limit(reader, buf, limits, depth)? {
        return Ok(());
    }

    let effective_lang = item_lang.or(channel_lang);

    match parse_item(reader, buf, limits, depth, base_ctx, effective_lang) {
        Ok((mut entry, has_attr_errors, has_entity_bozo)) => {
            if has_attr_errors {
                feed.bozo = true;
                feed.bozo_exception = Some(MALFORMED_ATTRIBUTES_ERROR.to_string());
            }
            if has_entity_bozo && !feed.bozo {
                feed.bozo = true;
                feed.bozo_exception = Some("Unresolvable entity in entry field".to_string());
            }
            if entry.summary.is_none() {
                entry.summary = entry.content.first().map(|c| c.value.clone());
            }
            feed.entries.push(entry);
        }
        Err(e) => {
            feed.bozo = true;
            feed.bozo_exception = Some(e.to_string());
        }
    }

    Ok(())
}

/// Parse channel extension elements (iTunes, Podcast, namespaces)
#[inline]
#[allow(clippy::too_many_arguments)]
fn parse_channel_extension(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    limits: &ParserLimits,
    depth: &mut usize,
    is_empty: bool,
) -> Result<()> {
    if tag == b"atom:link" || tag == b"atom10:link" {
        let mut href = String::new();
        let mut rel: Option<String> = None;
        let mut link_type: Option<String> = None;
        for (key, value) in attrs {
            match key.as_slice() {
                b"href" => href = truncate_to_length(value, limits.max_attribute_length),
                b"rel" => rel = Some(truncate_to_length(value, limits.max_attribute_length)),
                b"type" => link_type = Some(truncate_to_length(value, limits.max_attribute_length)),
                _ => {}
            }
        }
        if !href.is_empty() {
            let link = Link {
                href: href.clone().into(),
                rel: rel.as_deref().map(Into::into),
                link_type: link_type.map(Into::into),
                ..Default::default()
            };
            if feed.feed.next_url.is_none() && rel.as_deref() == Some("next") {
                feed.feed.next_url = Some(href);
            }
            feed.feed
                .links
                .try_push_limited(link, limits.max_links_per_feed);
        }
        return Ok(());
    }
    let mut handled = parse_channel_itunes(reader, buf, tag, attrs, feed, limits, depth, is_empty)?;
    if !handled {
        handled = parse_channel_podcast(reader, buf, tag, attrs, feed, limits, is_empty)?;
    }
    if !handled {
        handled = parse_channel_namespace(reader, buf, tag, feed, limits, *depth, is_empty)?;
    }

    // Only skip element content if this is NOT an empty element
    if !handled && !is_empty {
        skip_element(reader, buf, limits, *depth)?;
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
            url: url.into(),
            length,
            enclosure_type: enc_type.map(Into::into),
        })
    }
}

/// Parse standard RSS 2.0 channel elements
#[inline]
#[allow(clippy::too_many_arguments)]
fn parse_channel_standard(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    limits: &ParserLimits,
    base_ctx: &mut BaseUrlContext,
    channel_lang: Option<&str>,
) -> Result<()> {
    match tag {
        b"title" => {
            let (text, bozo) = read_text(reader, buf, limits)?;
            if bozo {
                feed.bozo = true;
                feed.bozo_exception = Some("Unresolvable entity in channel title".into());
            }
            feed.feed.set_title(TextConstruct {
                value: text,
                content_type: TextType::Text,
                language: channel_lang.map(std::convert::Into::into),
                base: base_ctx.base().map(String::from),
            });
        }
        b"link" => {
            let link_text = read_text_str(reader, buf, limits)?;
            feed.feed
                .set_alternate_link(link_text.clone(), limits.max_links_per_feed);

            if base_ctx.base().is_none() {
                base_ctx.update_base(&link_text);
            }
        }
        b"description" => {
            let (text, bozo) = read_text(reader, buf, limits)?;
            if bozo {
                feed.bozo = true;
                feed.bozo_exception =
                    Some("Unresolvable entity in channel description".to_string());
            }
            feed.feed.set_subtitle(TextConstruct {
                value: text,
                content_type: TextType::Html,
                language: channel_lang.map(std::convert::Into::into),
                base: base_ctx.base().map(String::from),
            });
        }
        b"language" => {
            feed.feed.language = Some(read_text_str(reader, buf, limits)?.into());
        }
        b"pubDate" => {
            let text = read_text_str(reader, buf, limits)?;
            match parse_date(&text) {
                Some(dt) => {
                    feed.feed.published = Some(dt);
                    feed.feed.published_str = Some(text.clone());
                    if feed.feed.updated.is_none() {
                        feed.feed.updated = Some(dt);
                        feed.feed.updated_str = Some(text);
                    }
                }
                None if !text.is_empty() => {
                    feed.bozo = true;
                    feed.bozo_exception = Some("Invalid pubDate format".to_string());
                }
                None => {}
            }
        }
        b"lastBuildDate" => {
            let text = read_text_str(reader, buf, limits)?;
            match parse_date(&text) {
                Some(dt) => {
                    feed.feed.updated = Some(dt);
                    feed.feed.updated_str = Some(text);
                }
                None if !text.is_empty() => {
                    feed.bozo = true;
                    feed.bozo_exception = Some("Invalid lastBuildDate format".to_string());
                }
                None => {}
            }
        }
        b"managingEditor" => {
            let text = read_text_str(reader, buf, limits)?;
            feed.feed.set_author(parse_rss_person(&text));
            // Preserve raw string in author flat field; author_detail has parsed fields
            feed.feed.author = Some(text.into());
        }
        b"webMaster" => {
            let text = read_text_str(reader, buf, limits)?;
            feed.feed.set_publisher(parse_rss_person(&text));
        }
        b"copyright" => {
            let text = read_text_str(reader, buf, limits)?;
            if !text.is_empty() {
                feed.feed.rights = Some(text);
            }
        }
        b"generator" => {
            feed.feed.generator = Some(read_text_str(reader, buf, limits)?);
        }
        b"ttl" => {
            feed.feed.ttl = Some(read_text_str(reader, buf, limits)?);
        }
        b"docs" => {
            feed.feed.docs = Some(read_text_str(reader, buf, limits)?);
        }
        b"category" => {
            let scheme = find_attribute(attrs, b"domain").map(|s| s.to_owned().into());
            let term = read_text_str(reader, buf, limits)?;
            if !term.is_empty() || scheme.is_some() {
                feed.feed.tags.try_push_limited(
                    Tag {
                        term: term.into(),
                        scheme,
                        label: None,
                    },
                    limits.max_tags,
                );
            }
        }
        _ => {}
    }
    Ok(())
}

/// Parse iTunes namespace tags at channel level
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
#[allow(clippy::too_many_arguments)]
fn parse_channel_itunes(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    limits: &ParserLimits,
    depth: &mut usize,
    is_empty: bool,
) -> Result<bool> {
    if is_itunes_tag(tag, b"author") {
        if !is_empty {
            let text = read_text_str(reader, buf, limits)?;
            // Promote to feed.author if not already set
            if feed.feed.author.is_none() {
                feed.feed.set_author(Person::from_name(&text));
                feed.feed
                    .authors
                    .try_push_limited(Person::from_name(&text), limits.max_authors);
            }
            let itunes = feed
                .feed
                .itunes
                .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
            itunes.author = Some(text);
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"subtitle") {
        if !is_empty {
            let text = read_text_str(reader, buf, limits)?;
            // Promote to feed.subtitle if not already set
            if feed.feed.subtitle.is_none() {
                feed.feed.set_subtitle(TextConstruct::text(&text));
            }
            let itunes = feed
                .feed
                .itunes
                .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
            itunes.subtitle = Some(text);
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"summary") {
        if !is_empty {
            let text = read_text_str(reader, buf, limits)?;
            // feed.subtitle doubles as summary in RSS (no separate summary field on FeedMeta)
            // promote to subtitle only when subtitle is absent
            if feed.feed.subtitle.is_none() {
                feed.feed.set_subtitle(TextConstruct::text(&text));
            }
            let itunes = feed
                .feed
                .itunes
                .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
            itunes.summary = Some(text);
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"owner") {
        if !is_empty {
            let itunes = feed
                .feed
                .itunes
                .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
            if let Ok(owner) = parse_itunes_owner(reader, buf, limits, depth) {
                itunes.owner = Some(owner);
            }
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"category") {
        parse_itunes_category(reader, buf, attrs, feed, limits, is_empty);
        Ok(true)
    } else if is_itunes_tag(tag, b"explicit") {
        if !is_empty {
            let text = read_text_str(reader, buf, limits)?;
            let itunes = feed
                .feed
                .itunes
                .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
            itunes.explicit = parse_explicit(&text);
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"image") {
        if let Some(value) = find_attribute(attrs, b"href") {
            let url = truncate_to_length(value, limits.max_attribute_length);
            let itunes = feed
                .feed
                .itunes
                .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
            itunes.image = Some(url.clone().into());
            // Also set feed.image if not already set (for Python feedparser compatibility)
            if feed.feed.image.is_none() {
                feed.feed.image = Some(Image {
                    url: url.into(),
                    title: None,
                    link: None,
                    width: None,
                    height: None,
                    description: None,
                });
            }
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"keywords") {
        if !is_empty {
            let text = read_text_str(reader, buf, limits)?;
            let itunes = feed
                .feed
                .itunes
                .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
            itunes.keywords = text
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"type") {
        if !is_empty {
            let text = read_text_str(reader, buf, limits)?;
            let itunes = feed
                .feed
                .itunes
                .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
            itunes.podcast_type = Some(text);
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"complete") {
        if !is_empty {
            let text = read_text_str(reader, buf, limits)?;
            let itunes = feed
                .feed
                .itunes
                .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
            itunes.complete = Some(text.trim().eq_ignore_ascii_case("Yes"));
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"new-feed-url") {
        if !is_empty {
            let text = read_text_str(reader, buf, limits)?;
            if !text.is_empty() {
                let itunes = feed
                    .feed
                    .itunes
                    .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
                itunes.new_feed_url = Some(text.trim().to_string().into());
            }
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"block") {
        if !is_empty {
            let text = read_text_str(reader, buf, limits)?;
            let itunes = feed
                .feed
                .itunes
                .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
            itunes.block = Some(u8::from(text.trim().eq_ignore_ascii_case("yes")));
        }
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
    is_empty: bool,
) {
    let category_text = find_attribute(attrs, b"text")
        .map(|v| truncate_to_length(v, limits.max_attribute_length))
        .unwrap_or_default();

    // Parse potential nested subcategory (only if not an empty element)
    let mut subcategory_text = None;
    if !is_empty {
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
                                    subcategory_text = Some(
                                        value.chars().take(limits.max_attribute_length).collect(),
                                    );
                                    break;
                                }
                            }
                        }
                    }
                }
                Ok(Event::Empty(sub_e)) => {
                    if is_itunes_tag(sub_e.name().as_ref(), b"category")
                        && subcategory_text.is_none()
                    {
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
    }

    let itunes = feed
        .feed
        .itunes
        .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
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
    is_empty: bool,
) -> Result<bool> {
    if tag.starts_with(b"podcast:guid") {
        if !is_empty {
            let text = read_text_str(reader, buf, limits)?;
            let podcast = feed
                .feed
                .podcast
                .get_or_insert_with(|| Box::new(PodcastMeta::default()));
            podcast.guid = Some(text);
        }
        Ok(true)
    } else if tag.starts_with(b"podcast:funding") {
        let url = find_attribute(attrs, b"url")
            .map(|v| truncate_to_length(v, limits.max_attribute_length))
            .unwrap_or_default();
        let message = if is_empty {
            None
        } else {
            let message_text = read_text_str(reader, buf, limits)?;
            if message_text.is_empty() {
                None
            } else {
                Some(message_text)
            }
        };
        let podcast = feed
            .feed
            .podcast
            .get_or_insert_with(|| Box::new(PodcastMeta::default()));
        podcast.funding.try_push_limited(
            PodcastFunding {
                url: url.into(),
                message,
            },
            limits.max_podcast_funding,
        );
        Ok(true)
    } else if tag.starts_with(b"podcast:value") {
        if !is_empty {
            parse_podcast_value(reader, buf, attrs, feed, limits)?;
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse Dublin Core, Content, `GeoRSS`, and Media RSS namespace tags at channel level
#[inline]
fn parse_channel_namespace(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    tag: &[u8],
    feed: &mut ParsedFeed,
    limits: &ParserLimits,
    depth: usize,
    is_empty: bool,
) -> Result<bool> {
    if let Some(dc_element) = is_dc_tag(tag) {
        if !is_empty {
            let dc_elem = dc_element.to_string();
            let text = read_text_str(reader, buf, limits)?;
            dublin_core::handle_feed_element(&dc_elem, &text, &mut feed.feed);
        }
        Ok(true)
    } else if let Some(_content_element) = is_content_tag(tag) {
        if !is_empty {
            skip_element(reader, buf, limits, depth)?;
        }
        Ok(true)
    } else if let Some(_media_element) = is_media_tag(tag) {
        if !is_empty {
            skip_element(reader, buf, limits, depth)?;
        }
        Ok(true)
    } else if let Some(georss_element) = is_georss_tag(tag) {
        if !is_empty {
            let text = read_text_str(reader, buf, limits)?;
            georss::handle_feed_element(georss_element.as_bytes(), &text, &mut feed.feed, limits);
        }
        Ok(true)
    } else if tag.starts_with(b"creativeCommons:license") || tag == b"license" {
        if !is_empty {
            feed.feed.license = Some(read_text_str(reader, buf, limits)?);
        }
        Ok(true)
    } else if is_thr_tag(tag).is_some() {
        // Atom Threading Extensions at channel level — recognize and skip
        if !is_empty {
            skip_element(reader, buf, limits, depth)?;
        }
        Ok(true)
    } else if let Some(syn_element) = is_syn_tag(tag) {
        if !is_empty {
            let syn_elem = syn_element.to_string();
            let text = read_text_str(reader, buf, limits)?;
            syndication::handle_feed_element(&syn_elem, &text, &mut feed.feed);
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse <item> element (entry)
///
/// Returns a tuple where:
/// - First element: the parsed `Entry`
/// - Second element: `bool` indicating whether attribute parsing errors occurred
/// - Third element: `bool` indicating whether unresolvable entities were encountered
fn parse_item(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
    depth: &mut usize,
    base_ctx: &BaseUrlContext,
    item_lang: Option<&str>,
) -> Result<(Entry, bool, bool)> {
    let mut entry = Entry::with_capacity();
    let mut has_attr_errors = false;
    let mut has_entity_bozo = false;
    let mut has_explicit_link = false;
    let mut guid_is_permalink: Option<bool> = None;

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
                        parse_item_standard(
                            reader,
                            buf,
                            &tag,
                            &attrs,
                            &mut entry,
                            limits,
                            base_ctx,
                            item_lang,
                            &mut has_entity_bozo,
                            &mut has_explicit_link,
                            &mut guid_is_permalink,
                        )?;
                    }
                    b"enclosure" => {
                        if let Some(mut enclosure) = parse_enclosure(&attrs, limits) {
                            enclosure.url = base_ctx.resolve_safe(&enclosure.url).into();
                            let link = Link {
                                href: enclosure.url.clone(),
                                rel: Some("enclosure".into()),
                                link_type: enclosure.enclosure_type.clone(),
                                length: enclosure.length.clone(),
                                ..Link::default()
                            };
                            entry
                                .links
                                .try_push_limited(link, limits.max_links_per_entry);
                            entry
                                .enclosures
                                .try_push_limited(enclosure, limits.max_enclosures);
                        }
                        if !is_empty {
                            skip_element(reader, buf, limits, *depth)?;
                        }
                    }
                    b"source" => {
                        if let Ok(source) = parse_source(reader, buf, &attrs, limits) {
                            entry.source = Some(source);
                        }
                    }
                    _ => {
                        let mut handled = parse_item_itunes(
                            reader,
                            buf,
                            &tag,
                            &attrs,
                            &mut entry,
                            limits,
                            is_empty,
                            *depth,
                            &mut has_entity_bozo,
                        )?;
                        if !handled {
                            handled = parse_item_podcast(
                                reader, buf, &tag, &attrs, &mut entry, limits, is_empty, *depth,
                            )?;
                        }
                        if !handled {
                            handled = parse_item_namespace(
                                reader,
                                buf,
                                &tag,
                                &attrs,
                                &mut entry,
                                limits,
                                is_empty,
                                *depth,
                                &mut has_entity_bozo,
                            )?;
                        }

                        if !handled && !is_empty {
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

    // guidislink = isPermaLink AND no explicit <link> element was present in the item.
    // Per Python feedparser semantics: when <link> exists, guid is just an identifier.
    if let Some(is_permalink) = guid_is_permalink {
        entry.guidislink = Some(is_permalink && !has_explicit_link);
        // If an explicit <link> was present, remove the guid-as-link fallback from entry.link
        // only if it was set as fallback (i.e., entry.link equals entry.id). We detect this
        // by checking: if has_explicit_link, entry.link was already overwritten by <link> arm.
        // No additional cleanup needed — <link> arm overwrites entry.link directly.
    }

    // dc:creator takes precedence over <author>
    if let Some(dc) = &entry.dc_creator {
        entry.author = Some(dc.clone());
    }

    Ok((entry, has_attr_errors, has_entity_bozo))
}

/// Parse standard RSS 2.0 item elements
#[inline]
#[allow(clippy::too_many_arguments)]
fn parse_item_standard(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    limits: &ParserLimits,
    base_ctx: &BaseUrlContext,
    item_lang: Option<&str>,
    bozo: &mut bool,
    has_explicit_link: &mut bool,
    guid_is_permalink: &mut Option<bool>,
) -> Result<()> {
    match tag {
        b"title" => {
            let (text, had_bozo) = read_text(reader, buf, limits)?;
            *bozo |= had_bozo;
            entry.set_title(TextConstruct {
                value: text,
                content_type: TextType::Text,
                language: item_lang.map(std::convert::Into::into),
                base: base_ctx.base().map(String::from),
            });
        }
        b"link" => {
            let link_text = read_text_str(reader, buf, limits)?;
            let resolved_link = base_ctx.resolve_safe(&link_text);
            entry.link = Some(resolved_link.clone());
            entry.links.try_push_limited(
                Link {
                    href: resolved_link.into(),
                    rel: Some("alternate".into()),
                    ..Default::default()
                },
                limits.max_links_per_entry,
            );
            *has_explicit_link = true;
        }
        b"description" => {
            let (text, had_bozo) = read_text(reader, buf, limits)?;
            *bozo |= had_bozo;
            entry.set_summary(TextConstruct {
                value: text,
                content_type: TextType::Html,
                language: item_lang.map(std::convert::Into::into),
                base: base_ctx.base().map(String::from),
            });
        }
        b"guid" => {
            // isPermaLink defaults to true per RSS 2.0 spec when attribute is absent
            let is_permalink = find_attribute(attrs, b"isPermaLink")
                .is_none_or(|v| v.eq_ignore_ascii_case("true"));
            let (text, had_bozo) = read_text(reader, buf, limits)?;
            *bozo |= had_bozo;
            entry.id = Some(text.clone().into());
            // Defer guidislink computation: need to know if explicit <link> was present.
            // guidislink = isPermaLink AND no explicit <link> element in the item.
            // Order is not guaranteed, so we track in guid_is_permalink and compute after loop.
            *guid_is_permalink = Some(is_permalink);
            // Use guid as entry.link fallback when it is a permalink and no <link> present yet.
            // If <link> appears after <guid>, the fallback will be overwritten — that is correct.
            if is_permalink && entry.link.is_none() {
                let resolved = base_ctx.resolve_safe(&text);
                entry.link = Some(resolved.clone());
                entry.links.try_push_limited(
                    Link {
                        href: resolved.into(),
                        rel: Some("alternate".into()),
                        ..Default::default()
                    },
                    limits.max_links_per_entry,
                );
            }
        }
        b"pubDate" => {
            let text = read_text_str(reader, buf, limits)?;
            let dt = parse_date(&text);
            entry.published = dt;
            entry.published_str = Some(text.clone());
            if entry.updated.is_none() {
                entry.updated = dt;
                entry.updated_str = Some(text);
            }
        }
        b"author" => {
            let (text, had_bozo) = read_text(reader, buf, limits)?;
            *bozo |= had_bozo;
            entry.set_author(parse_rss_person(&text));
            // Preserve raw string in author flat field; author_detail has parsed fields
            entry.author = Some(text.into());
        }
        b"category" => {
            let scheme = find_attribute(attrs, b"domain").map(|s| s.to_owned().into());
            let (term, had_bozo) = read_text(reader, buf, limits)?;
            *bozo |= had_bozo;
            entry.tags.try_push_limited(
                Tag {
                    term: term.into(),
                    scheme,
                    label: None,
                },
                limits.max_tags,
            );
        }
        b"comments" => {
            entry.comments = Some(read_text_str(reader, buf, limits)?);
        }
        _ => {}
    }
    Ok(())
}

/// Parse iTunes namespace tags at item level
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
///
/// Note: Uses 9 parameters instead of a context struct due to borrow checker constraints
/// with multiple simultaneous `&mut` references during parsing.
#[inline]
#[allow(clippy::too_many_arguments)]
fn parse_item_itunes(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    limits: &ParserLimits,
    is_empty: bool,
    depth: usize,
    bozo: &mut bool,
) -> Result<bool> {
    if is_itunes_tag(tag, b"title") {
        let (text, had_bozo) = read_text(reader, buf, limits)?;
        *bozo |= had_bozo;
        let itunes = entry
            .itunes
            .get_or_insert_with(|| Box::new(ItunesEntryMeta::default()));
        itunes.title = Some(text);
        Ok(true)
    } else if is_itunes_tag(tag, b"author") {
        let (text, had_bozo) = read_text(reader, buf, limits)?;
        *bozo |= had_bozo;
        // Promote to entry.author if not already set
        if entry.author.is_none() {
            entry.set_author(Person::from_name(&text));
            entry
                .authors
                .try_push_limited(Person::from_name(&text), limits.max_authors);
        }
        let itunes = entry
            .itunes
            .get_or_insert_with(|| Box::new(ItunesEntryMeta::default()));
        itunes.author = Some(text);
        Ok(true)
    } else if is_itunes_tag(tag, b"subtitle") {
        let (text, had_bozo) = read_text(reader, buf, limits)?;
        *bozo |= had_bozo;
        // Promote to entry.subtitle if not already set
        if entry.subtitle.is_none() {
            entry.set_subtitle(TextConstruct::text(&text));
        }
        let itunes = entry
            .itunes
            .get_or_insert_with(|| Box::new(ItunesEntryMeta::default()));
        itunes.subtitle = Some(text);
        Ok(true)
    } else if is_itunes_tag(tag, b"summary") {
        let (text, had_bozo) = read_text(reader, buf, limits)?;
        *bozo |= had_bozo;
        // Promote to entry.summary if not already set
        if entry.summary.is_none() {
            entry.set_summary(TextConstruct::text(&text));
        }
        let itunes = entry
            .itunes
            .get_or_insert_with(|| Box::new(ItunesEntryMeta::default()));
        itunes.summary = Some(text);
        Ok(true)
    } else if is_itunes_tag(tag, b"duration") {
        let text = read_text_str(reader, buf, limits)?;
        let itunes = entry
            .itunes
            .get_or_insert_with(|| Box::new(ItunesEntryMeta::default()));
        if !text.is_empty() {
            itunes.duration = Some(text);
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"explicit") {
        let text = read_text_str(reader, buf, limits)?;
        let itunes = entry
            .itunes
            .get_or_insert_with(|| Box::new(ItunesEntryMeta::default()));
        itunes.explicit = parse_explicit(&text);
        Ok(true)
    } else if is_itunes_tag(tag, b"image") {
        if let Some(value) = find_attribute(attrs, b"href") {
            let itunes = entry
                .itunes
                .get_or_insert_with(|| Box::new(ItunesEntryMeta::default()));
            itunes.image = Some(truncate_to_length(value, limits.max_attribute_length).into());
        }
        if !is_empty {
            skip_element(reader, buf, limits, depth)?;
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"episode") {
        let text = read_text_str(reader, buf, limits)?;
        let itunes = entry
            .itunes
            .get_or_insert_with(|| Box::new(ItunesEntryMeta::default()));
        if !text.is_empty() {
            itunes.episode = Some(text);
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"season") {
        let text = read_text_str(reader, buf, limits)?;
        let itunes = entry
            .itunes
            .get_or_insert_with(|| Box::new(ItunesEntryMeta::default()));
        if !text.is_empty() {
            itunes.season = Some(text);
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"episodeType") {
        let text = read_text_str(reader, buf, limits)?;
        let itunes = entry
            .itunes
            .get_or_insert_with(|| Box::new(ItunesEntryMeta::default()));
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
    } else if tag.starts_with(b"podcast:chapters") {
        parse_podcast_chapters(reader, buf, attrs, entry, limits, is_empty, depth)?;
        Ok(true)
    } else if tag.starts_with(b"podcast:soundbite") {
        parse_podcast_soundbite(reader, buf, attrs, entry, limits, is_empty, depth)?;
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
        let transcript = PodcastTranscript {
            url: url.into(),
            transcript_type: transcript_type.map(Into::into),
            language,
            rel,
        };
        entry
            .podcast_transcripts
            .try_push_limited(transcript.clone(), limits.max_podcast_transcripts);
        let podcast = entry
            .podcast
            .get_or_insert_with(|| Box::new(PodcastEntryMeta::default()));
        podcast
            .transcript
            .try_push_limited(transcript, limits.max_podcast_transcripts);
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

    let name = read_text_str(reader, buf, limits)?;
    if !name.is_empty() {
        let person = PodcastPerson {
            name,
            role,
            group,
            img: img.map(Into::into),
            href: href.map(Into::into),
        };
        entry
            .podcast_persons
            .try_push_limited(person.clone(), limits.max_podcast_persons);
        let podcast = entry
            .podcast
            .get_or_insert_with(|| Box::new(PodcastEntryMeta::default()));
        podcast
            .person
            .try_push_limited(person, limits.max_podcast_persons);
    }

    Ok(())
}

/// Parse Podcast 2.0 chapters element
fn parse_podcast_chapters(
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
    let type_ = find_attribute(attrs, b"type")
        .map(|v| truncate_to_length(v, limits.max_attribute_length))
        .unwrap_or_default();

    if !url.is_empty() {
        let podcast = entry
            .podcast
            .get_or_insert_with(|| Box::new(PodcastEntryMeta::default()));
        podcast.chapters = Some(PodcastChapters {
            url: url.into(),
            type_: type_.into(),
        });
    }

    if !is_empty {
        skip_element(reader, buf, limits, depth)?;
    }

    Ok(())
}

/// Parse Podcast 2.0 soundbite element
fn parse_podcast_soundbite(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    limits: &ParserLimits,
    is_empty: bool,
    depth: usize,
) -> Result<()> {
    let start_time = find_attribute(attrs, b"startTime").and_then(|v| v.parse::<f64>().ok());
    let duration = find_attribute(attrs, b"duration").and_then(|v| v.parse::<f64>().ok());

    if let (Some(start_time), Some(duration)) = (start_time, duration) {
        let title = if is_empty {
            None
        } else {
            let text = read_text_str(reader, buf, limits)?;
            if text.is_empty() { None } else { Some(text) }
        };

        let podcast = entry
            .podcast
            .get_or_insert_with(|| Box::new(PodcastEntryMeta::default()));
        podcast.soundbite.try_push_limited(
            PodcastSoundbite {
                start_time,
                duration,
                title,
            },
            limits.max_podcast_soundbites,
        );
    } else if !is_empty {
        skip_element(reader, buf, limits, depth)?;
    }

    Ok(())
}

/// Parse Dublin Core, Content, and Media RSS namespace tags at item level
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
///
/// Note: Uses 9 parameters instead of a context struct due to borrow checker constraints
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
    bozo: &mut bool,
) -> Result<bool> {
    if let Some(dc_element) = is_dc_tag(tag) {
        let dc_elem = dc_element.to_string();
        let (text, had_bozo) = read_text(reader, buf, limits)?;
        *bozo |= had_bozo;
        dublin_core::handle_entry_element(&dc_elem, &text, entry);
        Ok(true)
    } else if let Some(content_element) = is_content_tag(tag) {
        let content_elem = content_element.to_string();
        let (text, had_bozo) = read_text(reader, buf, limits)?;
        *bozo |= had_bozo;
        content::handle_entry_element(&content_elem, &text, entry);
        Ok(true)
    } else if let Some(georss_element) = is_georss_tag(tag) {
        let (text, had_bozo) = read_text(reader, buf, limits)?;
        *bozo |= had_bozo;
        georss::handle_entry_element(georss_element.as_bytes(), &text, entry, limits);
        Ok(true)
    } else if let Some(slash_element) = is_slash_tag(tag) {
        let slash_elem = slash_element.to_string();
        let (text, had_bozo) = read_text(reader, buf, limits)?;
        *bozo |= had_bozo;
        slash::handle_slash_entry_element(&slash_elem, &text, entry);
        Ok(true)
    } else if let Some(wfw_element) = is_wfw_tag(tag) {
        let wfw_elem = wfw_element.to_string();
        let (text, had_bozo) = read_text(reader, buf, limits)?;
        *bozo |= had_bozo;
        slash::handle_wfw_entry_element(&wfw_elem, &text, entry);
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
    } else if tag.starts_with(b"creativeCommons:license") || tag == b"license" {
        entry.license = Some(read_text_str(reader, buf, limits)?);
        Ok(true)
    } else if let Some(thr_element) = is_thr_tag(tag) {
        // Atom Threading Extensions (RFC 4685)
        match thr_element {
            "in-reply-to" => {
                if let Some(reply) =
                    threading::parse_in_reply_to_from_collected(attrs, limits.max_attribute_length)
                {
                    // Shares max_links_per_entry limit; split if needed later
                    entry
                        .in_reply_to
                        .try_push_limited(reply, limits.max_links_per_entry);
                }
                if !is_empty {
                    skip_element(reader, buf, limits, depth)?;
                }
            }
            "total" if !is_empty => {
                let (text, had_bozo) = read_text(reader, buf, limits)?;
                *bozo |= had_bozo;
                threading::handle_total(&text, entry);
            }
            _ => {
                if !is_empty {
                    skip_element(reader, buf, limits, depth)?;
                }
            }
        }
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
            let width = find_attribute(attrs, b"width").map(str::to_owned);
            let height = find_attribute(attrs, b"height").map(str::to_owned);

            if !url.is_empty() {
                entry.media_thumbnail.try_push_limited(
                    MediaThumbnail {
                        url: url.into(),
                        width,
                        height,
                    },
                    limits.max_enclosures,
                );
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
            let medium = find_attribute(attrs, b"medium")
                .map(|v| truncate_to_length(v, limits.max_attribute_length));
            let filesize = find_attribute(attrs, b"fileSize").and_then(|v| v.parse().ok());
            let duration = find_attribute(attrs, b"duration").map(str::to_owned);
            let width = find_attribute(attrs, b"width").map(str::to_owned);
            let height = find_attribute(attrs, b"height").map(str::to_owned);
            let bitrate = find_attribute(attrs, b"bitrate").map(str::to_owned);
            let lang = find_attribute(attrs, b"lang").map(str::to_owned);
            let channels = find_attribute(attrs, b"channels").map(str::to_owned);
            let codec = find_attribute(attrs, b"codec").map(str::to_owned);
            let expression = find_attribute(attrs, b"expression").map(str::to_owned);
            let isdefault = find_attribute(attrs, b"isDefault").map(str::to_owned);
            let samplingrate = find_attribute(attrs, b"samplingrate").map(str::to_owned);

            if !url.is_empty() {
                entry.media_content.try_push_limited(
                    MediaContent {
                        url: url.into(),
                        content_type: content_type.map(Into::into),
                        medium,
                        filesize,
                        width,
                        height,
                        duration,
                        bitrate,
                        lang,
                        channels,
                        codec,
                        expression,
                        isdefault,
                        samplingrate,
                    },
                    limits.max_enclosures,
                );
            }
            if !is_empty {
                skip_element(reader, buf, limits, depth)?;
            }
        }
        "group" => {
            // media:group is a transparent container; parse its children as if they
            // were direct siblings of the enclosing item.
            if !is_empty {
                parse_media_group(reader, buf, entry, limits, depth)?;
            }
        }
        _ => {
            let media_elem = media_element.to_string();
            let text = read_text_str(reader, buf, limits)?;
            media_rss::handle_entry_element(&media_elem, &text, entry);
        }
    }
    Ok(())
}

/// Parse children of `<media:group>`, treating the group as a transparent container.
///
/// Per the Media RSS spec, `<media:group>` groups related media objects but has no
/// semantic meaning of its own; its children are promoted to the parent item level.
fn parse_media_group(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    entry: &mut Entry,
    limits: &ParserLimits,
    depth: usize,
) -> Result<()> {
    let mut inner_depth = depth;
    loop {
        buf.clear();
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) => {
                inner_depth += 1;
                check_depth(inner_depth, limits.max_nesting_depth)?;
                let tag = e.name().as_ref().to_vec();
                let (attrs, _) = collect_attributes(&e);
                if let Some(media_element) = is_media_tag(&tag) {
                    parse_item_media(
                        reader,
                        buf,
                        media_element,
                        &attrs,
                        entry,
                        limits,
                        false,
                        inner_depth,
                    )?;
                } else {
                    skip_element(reader, buf, limits, inner_depth)?;
                }
                inner_depth = inner_depth.saturating_sub(1);
            }
            Ok(Event::Empty(e)) => {
                let tag = e.name().as_ref().to_vec();
                let (attrs, _) = collect_attributes(&e);
                if let Some(media_element) = is_media_tag(&tag) {
                    parse_item_media(
                        reader,
                        buf,
                        media_element,
                        &attrs,
                        entry,
                        limits,
                        true,
                        inner_depth,
                    )?;
                }
            }
            Ok(Event::End(_) | Event::Eof) | Err(_) => break,
            _ => {}
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
                    b"url" => url = read_text_str(reader, buf, limits)?,
                    b"title" => title = Some(read_text_str(reader, buf, limits)?),
                    b"link" => link = Some(read_text_str(reader, buf, limits)?),
                    b"width" => {
                        if let Ok(w) = read_text_str(reader, buf, limits)?.parse() {
                            width = Some(w);
                        }
                    }
                    b"height" => {
                        if let Ok(h) = read_text_str(reader, buf, limits)?.parse() {
                            height = Some(h);
                        }
                    }
                    b"description" => description = Some(read_text_str(reader, buf, limits)?),
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
        url: url.into(),
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
    attrs: &[(Vec<u8>, String)],
    limits: &ParserLimits,
) -> Result<Source> {
    // RSS 2.0: <source url="...">Title text</source>
    // The url attribute becomes link; the text content becomes title.
    let link = find_attribute(attrs, b"url").map(ToOwned::to_owned);
    let id = None;

    let raw_title = read_text_str(reader, buf, limits)?;
    let title_str = raw_title.trim();
    let title = if title_str.is_empty() {
        None
    } else {
        Some(title_str.to_owned())
    };

    Ok(Source {
        title,
        href: link,
        link: None,
        author: None,
        id,
        links: Vec::new(),
        updated: None,
        updated_str: None,
        rights: None,
        guidislink: None,
    })
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
                    owner.name = Some(read_text_str(reader, buf, limits)?);
                } else if is_itunes_tag(tag_name.as_ref(), b"email") {
                    owner.email = Some(read_text_str(reader, buf, limits)?);
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

/// Parse Podcast 2.0 value element from <podcast:value> element
///
/// Parses value-for-value payment information including payment type, method,
/// suggested amount, and nested valueRecipient elements.
fn parse_podcast_value(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    limits: &ParserLimits,
) -> Result<()> {
    use crate::types::{PodcastValue, PodcastValueRecipient};

    let type_ = find_attribute(attrs, b"type")
        .map(|v| truncate_to_length(v, limits.max_attribute_length))
        .unwrap_or_default();
    let method = find_attribute(attrs, b"method")
        .map(|v| truncate_to_length(v, limits.max_attribute_length))
        .unwrap_or_default();
    let suggested = find_attribute(attrs, b"suggested")
        .map(|v| truncate_to_length(v, limits.max_attribute_length));

    let mut recipients = Vec::with_capacity(2);

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                let tag_name = e.name();
                if tag_name.as_ref().starts_with(b"podcast:valueRecipient") {
                    let (recipient_attrs, _) = collect_attributes(&e);

                    let name = find_attribute(&recipient_attrs, b"name")
                        .map(|v| truncate_to_length(v, limits.max_attribute_length));
                    let recipient_type = find_attribute(&recipient_attrs, b"type")
                        .map(|v| truncate_to_length(v, limits.max_attribute_length))
                        .unwrap_or_default();
                    let address = find_attribute(&recipient_attrs, b"address")
                        .map(|v| truncate_to_length(v, limits.max_attribute_length))
                        .unwrap_or_default();
                    let split = find_attribute(&recipient_attrs, b"split")
                        .and_then(|v| v.parse::<u32>().ok())
                        .unwrap_or(0);
                    let fee = find_attribute(&recipient_attrs, b"fee").and_then(|v| {
                        if v.eq_ignore_ascii_case("true") {
                            Some(true)
                        } else if v.eq_ignore_ascii_case("false") {
                            Some(false)
                        } else {
                            None
                        }
                    });

                    recipients.try_push_limited(
                        PodcastValueRecipient {
                            name,
                            type_: recipient_type,
                            address,
                            split,
                            fee,
                        },
                        limits.max_value_recipients,
                    );
                }
            }
            Ok(Event::End(e)) if e.name().as_ref().starts_with(b"podcast:value") => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    let podcast = feed
        .feed
        .podcast
        .get_or_insert_with(|| Box::new(PodcastMeta::default()));
    podcast.value = Some(PodcastValue {
        type_,
        method,
        suggested,
        recipients,
    });

    Ok(())
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
    fn test_parse_rss_last_build_date() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title>Test</title>
                <lastBuildDate>Sat, 14 Dec 2024 10:30:00 +0000</lastBuildDate>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo, "valid feed must not be bozo");
        assert!(
            feed.feed.updated.is_some(),
            "feed.updated must be populated from lastBuildDate"
        );
        assert_eq!(
            feed.feed.updated_str.as_deref(),
            Some("Sat, 14 Dec 2024 10:30:00 +0000")
        );
        let dt = feed.feed.updated.unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 14);
    }

    #[test]
    fn test_parse_rss_invalid_last_build_date() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <lastBuildDate>not a date</lastBuildDate>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(feed.bozo);
        assert!(
            feed.bozo_exception
                .as_deref()
                .unwrap_or("")
                .contains("Invalid lastBuildDate")
        );
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
        assert_eq!(
            feed.entries[0].enclosures[0].length.as_deref(),
            Some("12345")
        );
        assert_eq!(
            feed.entries[0].enclosures[0].enclosure_type.as_deref(),
            Some("audio/mpeg")
        );
    }

    #[test]
    fn test_enclosure_also_in_links() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item>
                    <link>http://example.com/ep1</link>
                    <enclosure url="http://example.com/audio.mp3"
                               length="12345678"
                               type="audio/mpeg"/>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        let entry = &feed.entries[0];

        assert_eq!(entry.enclosures.len(), 1);

        let enc_link = entry
            .links
            .iter()
            .find(|l| l.rel.as_deref() == Some("enclosure"));
        assert!(enc_link.is_some(), "enclosure must appear in entry.links");
        let enc_link = enc_link.unwrap();
        assert_eq!(enc_link.href.as_str(), "http://example.com/audio.mp3");
        assert_eq!(enc_link.link_type.as_deref(), Some("audio/mpeg"));
        assert_eq!(enc_link.length.as_deref(), Some("12345678"));
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
        assert!(img.description.is_none());
    }

    #[test]
    fn test_parse_rss_image_description() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <image>
                    <url>http://example.com/logo.png</url>
                    <title>Example Logo</title>
                    <link>http://example.com</link>
                    <description>Feed logo description</description>
                </image>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        let img = feed.feed.image.as_ref().unwrap();
        assert_eq!(img.url, "http://example.com/logo.png");
        assert_eq!(img.description.as_deref(), Some("Feed logo description"));
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
        // author flat field preserves raw string
        assert_eq!(
            feed.entries[0].author.as_deref(),
            Some("john@example.com (John Doe)")
        );
        let detail = feed.entries[0].author_detail.as_ref().unwrap();
        assert_eq!(detail.name.as_deref(), Some("John Doe"));
        assert_eq!(detail.email.as_deref(), Some("john@example.com"));
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
        assert_eq!(feed.entries[0].guidislink, Some(true));
        assert_eq!(
            feed.entries[0].link.as_deref(),
            Some("http://example.com/1")
        );
    }

    #[test]
    fn test_parse_rss_guid_not_permalink() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item>
                    <guid isPermaLink="false">tag:example.com,2024:1</guid>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(
            feed.entries[0].id.as_deref(),
            Some("tag:example.com,2024:1")
        );
        assert_eq!(feed.entries[0].guidislink, Some(false));
        assert!(
            feed.entries[0].link.is_none(),
            "non-permalink guid must not set entry.link"
        );
    }

    #[test]
    fn test_parse_rss_guid_default_is_permalink() {
        // Per RSS 2.0 spec, isPermaLink defaults to true when attribute is absent
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item>
                    <guid>http://example.com/default</guid>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.entries[0].guidislink, Some(true));
        assert_eq!(
            feed.entries[0].link.as_deref(),
            Some("http://example.com/default")
        );
    }

    #[test]
    fn test_parse_rss_guid_permalink_does_not_override_explicit_link() {
        // When both <link> and <guid isPermaLink="true"> are present,
        // guidislink must be false — guid is just an identifier when <link> exists.
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item>
                    <link>http://example.com/explicit-link</link>
                    <guid isPermaLink="true">http://example.com/guid</guid>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(
            feed.entries[0].link.as_deref(),
            Some("http://example.com/explicit-link")
        );
        assert_eq!(feed.entries[0].guidislink, Some(false));
    }

    #[test]
    fn test_parse_rss_guidislink_guid_before_link() {
        // <guid> appears before <link> — order must not affect guidislink result
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item>
                    <guid isPermaLink="true">http://example.com/guid</guid>
                    <link>http://example.com/explicit-link</link>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(
            feed.entries[0].link.as_deref(),
            Some("http://example.com/explicit-link")
        );
        assert_eq!(feed.entries[0].guidislink, Some(false));
    }

    #[test]
    fn test_parse_rss_guidislink_no_link_permalink_true() {
        // No <link>, isPermaLink=true — guidislink must be true
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item>
                    <guid isPermaLink="true">http://example.com/guid</guid>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.entries[0].guidislink, Some(true));
    }

    #[test]
    fn test_parse_rss_guidislink_no_link_permalink_false() {
        // No <link>, isPermaLink=false — guidislink must be false
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item>
                    <guid isPermaLink="false">tag:example.com,2024:item1</guid>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.entries[0].guidislink, Some(false));
    }

    #[test]
    fn test_parse_rss_guidislink_explicit_link_permalink_false() {
        // Has <link>, isPermaLink=false — guidislink must be false
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item>
                    <link>http://example.com/link</link>
                    <guid isPermaLink="false">tag:example.com,2024:item2</guid>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.entries[0].guidislink, Some(false));
    }

    #[test]
    fn test_parse_rss_guidislink_default_guid_with_explicit_link() {
        // Has <link> + default guid (isPermaLink defaults to true) — guidislink must be false
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item>
                    <link>http://example.com/link</link>
                    <guid>http://example.com/guid</guid>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.entries[0].guidislink, Some(false));
        assert_eq!(
            feed.entries[0].link.as_deref(),
            Some("http://example.com/link")
        );
    }

    #[test]
    fn test_parse_rss_no_guid_guidislink_is_none() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item>
                    <title>No guid here</title>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(feed.entries[0].guidislink.is_none());
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
        assert_eq!(feed.feed.ttl, Some("60".to_string()));
    }

    #[test]
    fn test_parse_rss_with_docs() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <docs>https://www.rssboard.org/rss-specification</docs>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(
            feed.feed.docs.as_deref(),
            Some("https://www.rssboard.org/rss-specification")
        );
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
    fn test_parse_rss_copyright_maps_to_rights() {
        let xml = br#"<?xml version="1.0"?><rss version="2.0"><channel>
            <title>T</title><link>http://x.com</link>
            <copyright>Copyright 2026 ACME Corp</copyright>
            <item><title>I</title></item>
        </channel></rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(
            feed.feed.rights.as_deref(),
            Some("Copyright 2026 ACME Corp")
        );
        assert!(!feed.bozo);
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
        // RSS 2.0 spec: <source url="...">Title text</source>
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item>
                    <source url="http://source.example.com">Source Feed</source>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(feed.entries[0].source.is_some());
        let source = feed.entries[0].source.as_ref().unwrap();
        assert_eq!(source.title.as_deref(), Some("Source Feed"));
        assert_eq!(source.href.as_deref(), Some("http://source.example.com"));
    }

    #[test]
    fn test_parse_rss_source_no_guidislink() {
        // RSS <source> has no <id> child, so guidislink and links are always empty/None
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item>
                    <source url="http://source.example.com">Source Feed</source>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        let source = feed.entries[0].source.as_ref().unwrap();
        assert!(source.guidislink.is_none());
        assert!(source.links.is_empty());
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
        assert_eq!(itunes.duration.as_deref(), Some("1:23:45"));
        assert_eq!(
            itunes.image.as_deref(),
            Some("https://example.com/episode-cover.jpg")
        );
        assert_eq!(itunes.explicit, Some(true));
        assert_eq!(itunes.episode.as_deref(), Some("42"));
        assert_eq!(itunes.season.as_deref(), Some("3"));
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
            feed1.entries[0]
                .itunes
                .as_ref()
                .unwrap()
                .duration
                .as_deref(),
            Some("1:23:45")
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
            feed2.entries[0]
                .itunes
                .as_ref()
                .unwrap()
                .duration
                .as_deref(),
            Some("23:45")
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
            feed3.entries[0]
                .itunes
                .as_ref()
                .unwrap()
                .duration
                .as_deref(),
            Some("3661")
        );

        // Test empty duration produces None
        let xml4 = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <item><itunes:duration></itunes:duration></item>
            </channel>
        </rss>"#;
        let feed4 = parse_rss20(xml4).unwrap();
        assert!(
            feed4.entries[0].itunes.as_ref().unwrap().duration.is_none(),
            "Empty duration should result in None"
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

    #[test]
    fn test_podcast_entry_meta_transcript_populated() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:transcript
                        url="https://example.com/ep1.srt"
                        type="application/srt"
                        language="en"
                        rel="captions"/>
                    <podcast:transcript
                        url="https://example.com/ep1.vtt"
                        type="text/vtt"/>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        let entry = &feed.entries[0];

        let podcast = entry
            .podcast
            .as_deref()
            .expect("entry.podcast should be Some");
        assert_eq!(podcast.transcript.len(), 2);
        assert_eq!(podcast.transcript[0].url, "https://example.com/ep1.srt");
        assert_eq!(
            podcast.transcript[0].transcript_type.as_deref(),
            Some("application/srt")
        );
        assert_eq!(podcast.transcript[0].language.as_deref(), Some("en"));
        assert_eq!(podcast.transcript[0].rel.as_deref(), Some("captions"));
        assert_eq!(podcast.transcript[1].url, "https://example.com/ep1.vtt");
        assert_eq!(
            podcast.transcript[1].transcript_type.as_deref(),
            Some("text/vtt")
        );
    }

    #[test]
    fn test_podcast_entry_meta_person_populated() {
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
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        let entry = &feed.entries[0];

        let podcast = entry
            .podcast
            .as_deref()
            .expect("entry.podcast should be Some");
        assert_eq!(podcast.person.len(), 2);
        assert_eq!(podcast.person[0].name, "Jane Doe");
        assert_eq!(podcast.person[0].role.as_deref(), Some("host"));
        assert_eq!(
            podcast.person[0].href.as_deref(),
            Some("https://example.com/host")
        );
        assert_eq!(
            podcast.person[0].img.as_deref(),
            Some("https://example.com/host.jpg")
        );
        assert_eq!(podcast.person[1].name, "John Smith");
        assert_eq!(podcast.person[1].role.as_deref(), Some("guest"));
    }

    #[test]
    fn test_podcast_entry_meta_non_none_without_soundbites_chapters() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:transcript url="https://example.com/ep1.txt" type="text/plain"/>
                    <podcast:person role="host">Jane Doe</podcast:person>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        let entry = &feed.entries[0];

        let podcast = entry
            .podcast
            .as_deref()
            .expect("entry.podcast must be Some when only transcript/person present");
        assert_eq!(podcast.transcript.len(), 1);
        assert_eq!(podcast.person.len(), 1);
        assert!(podcast.chapters.is_none());
        assert!(podcast.soundbite.is_empty());
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

    #[test]
    fn test_parse_rss_xml_lang_channel() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel xml:lang="en-US">
                <title>English Channel</title>
                <description>Test description</description>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.feed.title.as_deref(), Some("English Channel"));

        assert!(feed.feed.title_detail.is_some());
        let title_detail = feed.feed.title_detail.as_ref().unwrap();
        assert_eq!(title_detail.language.as_deref(), Some("en-US"));

        assert!(feed.feed.subtitle_detail.is_some());
        let subtitle_detail = feed.feed.subtitle_detail.as_ref().unwrap();
        assert_eq!(subtitle_detail.language.as_deref(), Some("en-US"));
    }

    #[test]
    fn test_parse_rss_xml_lang_item() {
        let xml = b"<?xml version=\"1.0\"?>
        <rss version=\"2.0\">
            <channel xml:lang=\"en\">
                <item xml:lang=\"fr-FR\">
                    <title>Article en fran\xc3\xa7ais</title>
                    <description>Description en fran\xc3\xa7ais</description>
                </item>
                <item>
                    <title>English Article</title>
                    <description>English description</description>
                </item>
            </channel>
        </rss>";

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.entries.len(), 2);

        let french_entry = &feed.entries[0];
        assert!(french_entry.title_detail.is_some());
        assert_eq!(
            french_entry
                .title_detail
                .as_ref()
                .unwrap()
                .language
                .as_deref(),
            Some("fr-FR")
        );
        assert_eq!(
            french_entry
                .summary_detail
                .as_ref()
                .unwrap()
                .language
                .as_deref(),
            Some("fr-FR")
        );

        let english_entry = &feed.entries[1];
        assert!(english_entry.title_detail.is_some());
        assert_eq!(
            english_entry
                .title_detail
                .as_ref()
                .unwrap()
                .language
                .as_deref(),
            Some("en")
        );
    }

    #[test]
    fn test_parse_rss_xml_lang_empty() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel xml:lang="">
                <title>Empty Lang Channel</title>
                <description>Test with empty xml:lang</description>
                <item xml:lang="">
                    <title>Empty Lang Item</title>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();

        // Empty xml:lang should be treated as empty string (converted to None or empty)
        if let Some(ref title_detail) = feed.feed.title_detail {
            assert_eq!(title_detail.language.as_deref(), Some(""));
        }

        assert_eq!(feed.entries.len(), 1);
        if let Some(ref title_detail) = feed.entries[0].title_detail {
            assert_eq!(title_detail.language.as_deref(), Some(""));
        }
    }

    #[test]
    fn test_parse_rss_license_channel() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:creativeCommons="http://backend.userland.com/creativeCommonsRssModule">
            <channel>
                <title>Test Feed</title>
                <creativeCommons:license>https://creativecommons.org/licenses/by/4.0/</creativeCommons:license>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(
            feed.feed.license.as_deref(),
            Some("https://creativecommons.org/licenses/by/4.0/")
        );
    }

    #[test]
    fn test_parse_rss_license_item() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item>
                    <title>Licensed Item</title>
                    <license>https://creativecommons.org/licenses/by-sa/3.0/</license>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.entries.len(), 1);
        assert_eq!(
            feed.entries[0].license.as_deref(),
            Some("https://creativecommons.org/licenses/by-sa/3.0/")
        );
    }

    #[test]
    fn test_parse_rss_podcast_value_lightning() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Test Podcast</title>
                <podcast:value type="lightning" method="keysend" suggested="0.00000005000">
                    <podcast:valueRecipient
                        name="Host"
                        type="node"
                        address="03ae9f91a0cb8ff43840e3c322c4c61f019d8c1c3cea15a25cfc425ac605e61a4a"
                        split="90"
                        fee="false"/>
                    <podcast:valueRecipient
                        name="Producer"
                        type="node"
                        address="02d5c1bf8b940dc9cadca86d1b0a3c37fbe39cee4c7e839e33bef9174531d27f52"
                        split="10"
                        fee="false"/>
                </podcast:value>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo, "Feed should parse without errors");

        let podcast = feed.feed.podcast.as_ref().unwrap();
        let value = podcast.value.as_ref().unwrap();

        assert_eq!(value.type_, "lightning");
        assert_eq!(value.method, "keysend");
        assert_eq!(value.suggested.as_deref(), Some("0.00000005000"));
        assert_eq!(value.recipients.len(), 2);

        assert_eq!(value.recipients[0].name.as_deref(), Some("Host"));
        assert_eq!(value.recipients[0].type_, "node");
        assert_eq!(
            value.recipients[0].address,
            "03ae9f91a0cb8ff43840e3c322c4c61f019d8c1c3cea15a25cfc425ac605e61a4a"
        );
        assert_eq!(value.recipients[0].split, 90);
        assert_eq!(value.recipients[0].fee, Some(false));

        assert_eq!(value.recipients[1].name.as_deref(), Some("Producer"));
        assert_eq!(value.recipients[1].type_, "node");
        assert_eq!(
            value.recipients[1].address,
            "02d5c1bf8b940dc9cadca86d1b0a3c37fbe39cee4c7e839e33bef9174531d27f52"
        );
        assert_eq!(value.recipients[1].split, 10);
        assert_eq!(value.recipients[1].fee, Some(false));
    }

    #[test]
    fn test_parse_rss_podcast_value_without_suggested() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Test Podcast</title>
                <podcast:value type="lightning" method="keysend">
                    <podcast:valueRecipient
                        name="Host"
                        type="node"
                        address="abc123"
                        split="100"/>
                </podcast:value>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        let value = feed.feed.podcast.as_ref().unwrap().value.as_ref().unwrap();

        assert_eq!(value.type_, "lightning");
        assert_eq!(value.method, "keysend");
        assert!(value.suggested.is_none());
        assert_eq!(value.recipients.len(), 1);
        assert_eq!(value.recipients[0].split, 100);
    }

    #[test]
    fn test_parse_rss_podcast_value_with_fee_recipient() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Test Podcast</title>
                <podcast:value type="lightning" method="keysend">
                    <podcast:valueRecipient
                        type="node"
                        address="fee_address"
                        split="5"
                        fee="true"/>
                    <podcast:valueRecipient
                        name="Host"
                        type="node"
                        address="host_address"
                        split="95"
                        fee="false"/>
                </podcast:value>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        let value = feed.feed.podcast.as_ref().unwrap().value.as_ref().unwrap();

        assert_eq!(value.recipients.len(), 2);
        assert!(value.recipients[0].name.is_none());
        assert_eq!(value.recipients[0].fee, Some(true));
        assert_eq!(value.recipients[1].fee, Some(false));
    }

    #[test]
    fn test_parse_rss_podcast_value_respects_limits() {
        let mut xml = String::from(
            r#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Test Podcast</title>
                <podcast:value type="lightning" method="keysend">"#,
        );

        for i in 0..25 {
            use std::fmt::Write;
            let _ = write!(
                xml,
                r#"<podcast:valueRecipient type="node" address="addr_{i}" split="4"/>"#
            );
        }

        xml.push_str(
            r"</podcast:value>
            </channel>
        </rss>",
        );

        let limits = ParserLimits {
            max_value_recipients: 5,
            ..Default::default()
        };
        let feed = parse_rss20_with_limits(xml.as_bytes(), limits).unwrap();
        let value = feed.feed.podcast.as_ref().unwrap().value.as_ref().unwrap();

        assert_eq!(
            value.recipients.len(),
            5,
            "Should respect max_value_recipients limit"
        );
    }

    #[test]
    fn test_parse_rss_podcast_value_empty_recipients() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Test Podcast</title>
                <podcast:value type="lightning" method="keysend" suggested="0.00000005000">
                </podcast:value>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        let value = feed.feed.podcast.as_ref().unwrap().value.as_ref().unwrap();

        assert_eq!(value.type_, "lightning");
        assert_eq!(value.method, "keysend");
        assert_eq!(value.suggested.as_deref(), Some("0.00000005000"));
        assert_eq!(value.recipients.len(), 0);
    }

    #[test]
    fn test_parse_rss_feed_next_url_atom_link() {
        let xml = include_bytes!("../../../../tests/fixtures/rss-pagination.xml");
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        assert_eq!(
            feed.feed.next_url.as_deref(),
            Some("http://example.com/feed?page=2")
        );
    }

    #[test]
    fn test_parse_rss_feed_next_url_absent() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title>No Pagination</title>
                <link>http://example.com/</link>
                <description>Feed without pagination</description>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        assert!(feed.feed.next_url.is_none());
    }

    #[test]
    fn test_rss_author_email_paren_name() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title>T</title>
                <link>http://x.com</link>
                <managingEditor>editor@example.com (John Editor)</managingEditor>
                <item>
                    <title>Item</title>
                    <author>author@example.com (Jane Author)</author>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);

        let author_detail = feed.feed.author_detail.as_ref().unwrap();
        assert_eq!(author_detail.name.as_deref(), Some("John Editor"));
        assert_eq!(author_detail.email.as_deref(), Some("editor@example.com"));
        // author flat field preserves raw string
        assert_eq!(
            feed.feed.author.as_deref(),
            Some("editor@example.com (John Editor)")
        );

        let entry_detail = feed.entries[0].author_detail.as_ref().unwrap();
        assert_eq!(entry_detail.name.as_deref(), Some("Jane Author"));
        assert_eq!(entry_detail.email.as_deref(), Some("author@example.com"));
        // author flat field preserves raw string
        assert_eq!(
            feed.entries[0].author.as_deref(),
            Some("author@example.com (Jane Author)")
        );
    }

    #[test]
    fn test_rss_author_name_angle_email() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title>T</title>
                <link>http://x.com</link>
                <managingEditor>John Editor &lt;editor@example.com&gt;</managingEditor>
                <item>
                    <title>Item</title>
                    <author>Jane Author &lt;author@example.com&gt;</author>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);

        let author_detail = feed.feed.author_detail.as_ref().unwrap();
        assert_eq!(author_detail.name.as_deref(), Some("John Editor"));
        assert_eq!(author_detail.email.as_deref(), Some("editor@example.com"));

        let entry_detail = feed.entries[0].author_detail.as_ref().unwrap();
        assert_eq!(entry_detail.name.as_deref(), Some("Jane Author"));
        assert_eq!(entry_detail.email.as_deref(), Some("author@example.com"));
    }

    #[test]
    fn test_rss_webmaster_publisher_detail() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title>T</title>
                <link>http://x.com</link>
                <webMaster>webmaster@example.com (Web Master)</webMaster>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);

        let pub_detail = feed.feed.publisher_detail.as_ref().unwrap();
        assert_eq!(pub_detail.name.as_deref(), Some("Web Master"));
        assert_eq!(pub_detail.email.as_deref(), Some("webmaster@example.com"));
        assert_eq!(feed.feed.publisher.as_deref(), Some("Web Master"));
    }

    #[test]
    fn test_rss_content_encoded_fallback_to_summary() {
        let xml = br#"<?xml version="1.0"?>
<rss version="2.0" xmlns:content="http://purl.org/rss/content/">
<channel><title>T</title><link>http://x.com</link><description>D</description>
  <item><title>I</title><content:encoded>&lt;p&gt;Full HTML&lt;/p&gt;</content:encoded></item>
</channel></rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.entries[0].summary.as_deref(), Some("<p>Full HTML</p>"));
    }

    #[test]
    fn test_rss_namespaces_on_rss_element() {
        let xml = br#"<?xml version="1.0"?>
<rss version="2.0"
     xmlns:dc="http://purl.org/dc/elements/1.1/"
     xmlns:content="http://purl.org/rss/1.0/modules/content/"
     xmlns:media="http://search.yahoo.com/mrss/">
<channel><title>T</title><link>http://x.com</link></channel>
</rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        assert_eq!(
            feed.namespaces.get("dc").map(String::as_str),
            Some("http://purl.org/dc/elements/1.1/")
        );
        assert_eq!(
            feed.namespaces.get("content").map(String::as_str),
            Some("http://purl.org/rss/1.0/modules/content/")
        );
        assert_eq!(
            feed.namespaces.get("media").map(String::as_str),
            Some("http://search.yahoo.com/mrss/")
        );
    }

    #[test]
    fn test_rss_namespaces_on_channel_element() {
        // Some feeds (e.g., WordPress.com) declare xmlns on <channel> instead of <rss>
        let xml = br#"<?xml version="1.0"?>
<rss version="2.0">
<channel xmlns:dc="http://purl.org/dc/elements/1.1/"
         xmlns:content="http://purl.org/rss/1.0/modules/content/">
<title>T</title><link>http://x.com</link>
</channel>
</rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        assert_eq!(
            feed.namespaces.get("dc").map(String::as_str),
            Some("http://purl.org/dc/elements/1.1/")
        );
        assert_eq!(
            feed.namespaces.get("content").map(String::as_str),
            Some("http://purl.org/rss/1.0/modules/content/")
        );
    }

    #[test]
    fn test_rss_no_namespaces() {
        let xml = br#"<?xml version="1.0"?>
<rss version="2.0"><channel><title>T</title></channel></rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        assert!(feed.namespaces.is_empty());
    }

    #[test]
    fn test_rss_namespace_limit_exceeded() {
        let xml = br#"<?xml version="1.0"?>
<rss version="2.0"
     xmlns:a="http://example.com/a"
     xmlns:b="http://example.com/b"
     xmlns:c="http://example.com/c"
     xmlns:d="http://example.com/d">
<channel><title>T</title></channel>
</rss>"#;
        let limits = crate::ParserLimits {
            max_namespaces: 2,
            ..crate::ParserLimits::default()
        };
        let feed = parse_rss20_with_limits(xml, limits).unwrap();
        assert!(feed.bozo);
        assert_eq!(feed.namespaces.len(), 2);
    }

    #[test]
    fn test_itunes_author_promotes_to_feed_author_when_absent() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Podcast</title>
                <itunes:author>Podcast Author</itunes:author>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.feed.author.as_deref(), Some("Podcast Author"));
        assert_eq!(
            feed.feed.author_detail.as_ref().unwrap().name.as_deref(),
            Some("Podcast Author")
        );
        assert_eq!(feed.feed.authors.len(), 1);
        assert_eq!(
            feed.feed.itunes.as_ref().unwrap().author.as_deref(),
            Some("Podcast Author")
        );
    }

    #[test]
    fn test_itunes_author_does_not_overwrite_existing_feed_author() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Podcast</title>
                <managingEditor>editor@example.com (Existing Author)</managingEditor>
                <itunes:author>iTunes Author</itunes:author>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        // managingEditor sets author; itunes:author must not overwrite it
        assert_ne!(feed.feed.author.as_deref(), Some("iTunes Author"));
        assert_eq!(
            feed.feed.itunes.as_ref().unwrap().author.as_deref(),
            Some("iTunes Author")
        );
    }

    #[test]
    fn test_itunes_subtitle_promotes_to_feed_subtitle_when_absent() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Podcast</title>
                <itunes:subtitle>iTunes Subtitle</itunes:subtitle>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.feed.subtitle.as_deref(), Some("iTunes Subtitle"));
        assert_eq!(
            feed.feed.itunes.as_ref().unwrap().subtitle.as_deref(),
            Some("iTunes Subtitle")
        );
    }

    #[test]
    fn test_itunes_subtitle_does_not_overwrite_existing_feed_subtitle() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Podcast</title>
                <description>Existing Subtitle</description>
                <itunes:subtitle>iTunes Subtitle</itunes:subtitle>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.feed.subtitle.as_deref(), Some("Existing Subtitle"));
        assert_eq!(
            feed.feed.itunes.as_ref().unwrap().subtitle.as_deref(),
            Some("iTunes Subtitle")
        );
    }

    #[test]
    fn test_itunes_summary_promotes_to_feed_subtitle_when_absent() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Podcast</title>
                <itunes:summary>iTunes Summary</itunes:summary>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.feed.subtitle.as_deref(), Some("iTunes Summary"));
        assert_eq!(
            feed.feed.itunes.as_ref().unwrap().summary.as_deref(),
            Some("iTunes Summary")
        );
    }

    #[test]
    fn test_entry_itunes_author_promotes_when_absent() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Podcast</title>
                <item>
                    <title>Episode 1</title>
                    <itunes:author>Episode Author</itunes:author>
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let entry = &feed.entries[0];
        assert_eq!(entry.author.as_deref(), Some("Episode Author"));
        assert_eq!(
            entry.author_detail.as_ref().unwrap().name.as_deref(),
            Some("Episode Author")
        );
        assert_eq!(
            entry.itunes.as_ref().unwrap().author.as_deref(),
            Some("Episode Author")
        );
    }

    #[test]
    fn test_entry_itunes_author_does_not_overwrite_existing() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Podcast</title>
                <item>
                    <title>Episode 1</title>
                    <author>existing@example.com</author>
                    <itunes:author>iTunes Author</itunes:author>
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let entry = &feed.entries[0];
        assert_ne!(entry.author.as_deref(), Some("iTunes Author"));
        assert_eq!(
            entry.itunes.as_ref().unwrap().author.as_deref(),
            Some("iTunes Author")
        );
    }

    #[test]
    fn test_entry_itunes_subtitle_promotes_when_absent() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Podcast</title>
                <item>
                    <title>Episode 1</title>
                    <itunes:subtitle>Episode Subtitle</itunes:subtitle>
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let entry = &feed.entries[0];
        assert_eq!(entry.subtitle.as_deref(), Some("Episode Subtitle"));
        assert_eq!(
            entry.itunes.as_ref().unwrap().subtitle.as_deref(),
            Some("Episode Subtitle")
        );
    }

    #[test]
    fn test_entry_itunes_summary_promotes_when_absent() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Podcast</title>
                <item>
                    <title>Episode 1</title>
                    <itunes:summary>Episode Summary</itunes:summary>
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let entry = &feed.entries[0];
        assert_eq!(entry.summary.as_deref(), Some("Episode Summary"));
        assert_eq!(
            entry.itunes.as_ref().unwrap().summary.as_deref(),
            Some("Episode Summary")
        );
    }

    #[test]
    fn test_entry_itunes_summary_does_not_overwrite_existing() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Podcast</title>
                <item>
                    <title>Episode 1</title>
                    <description>Existing Summary</description>
                    <itunes:summary>iTunes Summary</itunes:summary>
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let entry = &feed.entries[0];
        assert_eq!(entry.summary.as_deref(), Some("Existing Summary"));
        assert_eq!(
            entry.itunes.as_ref().unwrap().summary.as_deref(),
            Some("iTunes Summary")
        );
    }

    // Regression tests for issue #201: pubDate must populate entry.updated when
    // no other updated source (dc:date, atom:updated) is present.

    #[test]
    fn test_entry_pubdate_populates_updated_when_no_dc_date() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <item>
                    <title>Only pubDate</title>
                    <pubDate>Mon, 10 Mar 2025 08:00:00 +0000</pubDate>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        let entry = &feed.entries[0];

        assert!(
            entry.published.is_some(),
            "entry.published must be set from pubDate"
        );
        assert!(
            entry.updated.is_some(),
            "entry.updated must be populated from pubDate when no dc:date present"
        );
        assert!(
            entry.updated_str.is_some(),
            "entry.updated_str must be non-None"
        );
        assert_eq!(
            entry.updated, entry.published,
            "entry.updated must equal entry.published when promoted from pubDate"
        );
    }

    #[test]
    fn test_feed_pubdate_populates_updated_when_no_last_build_date() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title>Feed with only pubDate</title>
                <pubDate>Tue, 11 Mar 2025 12:00:00 +0000</pubDate>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);

        assert!(
            feed.feed.published.is_some(),
            "feed.published must be set from channel pubDate"
        );
        assert!(
            feed.feed.updated.is_some(),
            "feed.updated must be populated from pubDate when no lastBuildDate present"
        );
        assert_eq!(
            feed.feed.updated, feed.feed.published,
            "feed.updated must equal feed.published when promoted from pubDate"
        );
    }

    #[test]
    fn test_entry_dc_date_takes_precedence_over_pubdate_for_updated() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:dc="http://purl.org/dc/elements/1.1/">
            <channel>
                <item>
                    <title>Both dates</title>
                    <pubDate>Mon, 10 Mar 2025 08:00:00 +0000</pubDate>
                    <dc:date>2025-03-15T20:00:00Z</dc:date>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        let entry = &feed.entries[0];

        assert!(entry.updated.is_some());
        // dc:date is 2025-03-15, pubDate is 2025-03-10 — updated must use dc:date
        let updated = entry.updated.unwrap();
        assert_eq!(updated.year(), 2025);
        assert_eq!(updated.month(), 3);
        assert_eq!(
            updated.day(),
            15,
            "entry.updated must use dc:date, not pubDate"
        );
    }

    #[test]
    fn test_parse_rss2_with_syndication_syn_prefix() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:syn="http://purl.org/rss/1.0/modules/syndication/">
          <channel>
            <title>Test</title>
            <link>http://example.com</link>
            <syn:updatePeriod>hourly</syn:updatePeriod>
            <syn:updateFrequency>2</syn:updateFrequency>
            <syn:updateBase>2024-01-01T00:00:00Z</syn:updateBase>
          </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(feed.feed.syndication.is_some());

        let syn = feed.feed.syndication.as_ref().unwrap();
        assert_eq!(
            syn.update_period,
            Some(crate::namespace::syndication::UpdatePeriod::Hourly)
        );
        assert_eq!(syn.update_frequency, Some("2".to_string()));
        assert_eq!(syn.update_base.as_deref(), Some("2024-01-01T00:00:00Z"));
    }

    #[test]
    fn test_parse_rss2_with_syndication_sy_prefix() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:sy="http://purl.org/rss/1.0/modules/syndication/">
          <channel>
            <title>Test</title>
            <link>http://example.com</link>
            <sy:updatePeriod>daily</sy:updatePeriod>
            <sy:updateFrequency>1</sy:updateFrequency>
            <sy:updateBase>2024-06-01T00:00:00Z</sy:updateBase>
          </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(feed.feed.syndication.is_some());

        let syn = feed.feed.syndication.as_ref().unwrap();
        assert_eq!(
            syn.update_period,
            Some(crate::namespace::syndication::UpdatePeriod::Daily)
        );
        assert_eq!(syn.update_frequency, Some("1".to_string()));
        assert_eq!(syn.update_base.as_deref(), Some("2024-06-01T00:00:00Z"));
    }
}
