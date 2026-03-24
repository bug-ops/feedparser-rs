//! Atom 1.0 parser implementation

use crate::{
    ParserLimits,
    error::{FeedError, Result},
    namespace::{content, dublin_core, media_rss, slash, threading},
    types::{
        Content, Enclosure, Entry, FeedVersion, Generator, Image, ItunesCategory, ItunesEntryMeta,
        ItunesFeedMeta, ItunesOwner, Link, MediaContent, MediaThumbnail, ParsedFeed, Person,
        Source, Tag, TextConstruct, TextType, parse_explicit,
    },
    util::{base_url::BaseUrlContext, parse_date, text::truncate_to_length},
};
use quick_xml::{Reader, events::Event};

use super::common::{
    EVENT_BUFFER_CAPACITY, FromAttributes, LimitedCollectionExt, bytes_to_string, check_depth,
    extract_namespaces, extract_xml_base, extract_xml_lang, init_feed, is_content_tag, is_dc_tag,
    is_itunes_tag, is_media_tag, is_slash_tag, is_thr_tag, is_wfw_tag, read_text, read_text_str,
    read_xhtml_content_str, skip_element, skip_to_end,
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

    let mut feed = init_feed(FeedVersion::Atom10, limits.max_entries);
    let mut buf = Vec::with_capacity(EVENT_BUFFER_CAPACITY);
    let mut depth: usize = 1;
    let mut base_ctx = BaseUrlContext::new();
    let mut found_feed_element = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.local_name().as_ref() == b"feed" => {
                if let Some(xml_base) = extract_xml_base(&e, limits.max_attribute_length) {
                    base_ctx.update_base(&xml_base);
                }

                // Atom uses xml:lang exclusively for language (RFC 4287 has no <language> element).
                let feed_lang = extract_xml_lang(&e, limits.max_attribute_length);
                if let Some(ref lang) = feed_lang {
                    feed.feed.language = Some(lang.as_str().into());
                }

                extract_namespaces(&e, &mut feed, &limits);

                // Use populated namespaces map to detect Atom 0.3 (avoids re-iterating attributes)
                if feed
                    .namespaces
                    .get("")
                    .is_some_and(|uri| uri == "http://purl.org/atom/ns#")
                {
                    feed.version = FeedVersion::Atom03;
                }

                found_feed_element = true;
                depth += 1;
                if let Err(e) = parse_feed_element(
                    &mut reader,
                    &mut feed,
                    &limits,
                    &mut depth,
                    &base_ctx,
                    feed_lang.as_deref(),
                ) {
                    feed.bozo = true;
                    feed.bozo_exception = Some(e.to_string());
                }
                // #274: promote feed.id → feed.link when no explicit link was found
                if feed.feed.link.is_none()
                    && let Some(id) = feed.feed.id.as_deref()
                {
                    feed.feed.link = Some(id.to_string());
                }
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) => {
                if !found_feed_element {
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

/// Parse <feed> element
#[allow(clippy::too_many_lines)]
fn parse_feed_element(
    reader: &mut Reader<&[u8]>,
    feed: &mut ParsedFeed,
    limits: &ParserLimits,
    depth: &mut usize,
    base_ctx: &BaseUrlContext,
    feed_lang: Option<&str>,
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
                        let text =
                            parse_text_construct(reader, &mut buf, &element, limits, feed_lang)?;
                        feed.feed.set_title(text);
                    }
                    b"link" => {
                        if let Some(mut link) = Link::from_attributes(
                            element.attributes().flatten(),
                            limits.max_attribute_length,
                        ) {
                            link.href = base_ctx.resolve_safe(&link.href).into();

                            if feed.feed.link.is_none() && link.rel.as_deref() == Some("alternate")
                            {
                                feed.feed.link = Some(link.href.to_string());
                            }
                            if feed.feed.license.is_none() && link.rel.as_deref() == Some("license")
                            {
                                feed.feed.license = Some(link.href.to_string());
                            }
                            if feed.feed.next_url.is_none() && link.rel.as_deref() == Some("next") {
                                feed.feed.next_url = Some(link.href.to_string());
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
                        let text =
                            parse_text_construct(reader, &mut buf, &element, limits, feed_lang)?;
                        feed.feed.set_subtitle(text);
                    }
                    b"id" if !is_empty => {
                        let (text, bozo) = read_text(reader, &mut buf, limits)?;
                        if bozo {
                            feed.bozo = true;
                            feed.bozo_exception =
                                Some("Unresolvable entity in feed id".to_string());
                        }
                        feed.feed.id = Some(text);
                    }
                    b"updated" | b"modified" if !is_empty => {
                        let text = read_text_str(reader, &mut buf, limits)?;
                        feed.feed.updated = parse_date(&text);
                        feed.feed.updated_str = Some(text);
                    }
                    b"published" | b"issued" if !is_empty => {
                        let text = read_text_str(reader, &mut buf, limits)?;
                        feed.feed.published = parse_date(&text);
                        feed.feed.published_str = Some(text);
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
                        let url = read_text_str(reader, &mut buf, limits)?;
                        feed.feed.icon = Some(base_ctx.resolve_safe(&url));
                    }
                    b"logo" if !is_empty => {
                        let url = read_text_str(reader, &mut buf, limits)?;
                        feed.feed.logo = Some(base_ctx.resolve_safe(&url));
                    }
                    b"rights" if !is_empty => {
                        let text =
                            parse_text_construct(reader, &mut buf, &element, limits, feed_lang)?;
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

                        // Entry-level xml:lang overrides feed-level; fall back to feed_lang.
                        let entry_lang_owned =
                            extract_xml_lang(&element, limits.max_attribute_length);
                        let effective_lang = entry_lang_owned.as_deref().or(feed_lang);

                        let mut entry_bozo = false;
                        match parse_entry(
                            reader,
                            &mut buf,
                            limits,
                            depth,
                            &entry_ctx,
                            &mut entry_bozo,
                            effective_lang,
                        ) {
                            Ok(mut entry) => {
                                if entry_bozo && !feed.bozo {
                                    feed.bozo = true;
                                    feed.bozo_exception =
                                        Some("Unresolvable entity in entry field".to_string());
                                }
                                if entry.summary.is_none() {
                                    entry.summary = entry.content.first().map(|c| c.value.clone());
                                }
                                // #273: promote entry.id → entry.link when no explicit link
                                promote_entry_id_to_link(&mut entry);
                                // #275: fallback entry.updated from entry.published
                                promote_entry_published_to_updated(&mut entry);
                                feed.entries.push(entry);
                            }
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
                                let text = read_text_str(reader, &mut buf, limits)?;
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
                        } else if is_thr_tag(tag).is_some() {
                            // Atom Threading Extensions - feed-level thr: elements are unusual; skip
                            if !is_empty {
                                skip_element(reader, &mut buf, limits, *depth)?;
                            }
                            true
                        } else if is_itunes_tag(tag, b"image") {
                            if let Some(url) = extract_href_attr(&element, limits) {
                                let itunes = feed
                                    .feed
                                    .itunes
                                    .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
                                itunes.image = Some(url.clone().into());
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
                            if !is_empty {
                                skip_element(reader, &mut buf, limits, *depth)?;
                            }
                            true
                        } else if is_itunes_tag(tag, b"category") {
                            parse_atom_itunes_category(
                                reader, &mut buf, &element, feed, limits, is_empty,
                            )?;
                            true
                        } else if is_itunes_tag(tag, b"owner") && !is_empty {
                            if let Ok(owner) =
                                parse_atom_itunes_owner(reader, &mut buf, limits, depth)
                            {
                                let itunes = feed
                                    .feed
                                    .itunes
                                    .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
                                itunes.owner = Some(owner);
                            }
                            true
                        } else if is_itunes_tag(tag, b"author") && !is_empty {
                            let text = read_text_str(reader, &mut buf, limits)?;
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
                            true
                        } else if is_itunes_tag(tag, b"subtitle") && !is_empty {
                            let text = read_text_str(reader, &mut buf, limits)?;
                            if feed.feed.subtitle.is_none() {
                                feed.feed.set_subtitle(TextConstruct::text(&text));
                            }
                            let itunes = feed
                                .feed
                                .itunes
                                .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
                            itunes.subtitle = Some(text);
                            true
                        } else if is_itunes_tag(tag, b"summary") && !is_empty {
                            let text = read_text_str(reader, &mut buf, limits)?;
                            if feed.feed.subtitle.is_none() {
                                feed.feed.set_subtitle(TextConstruct::text(&text));
                            }
                            let itunes = feed
                                .feed
                                .itunes
                                .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
                            itunes.summary = Some(text);
                            true
                        } else if is_itunes_tag(tag, b"explicit") && !is_empty {
                            let text = read_text_str(reader, &mut buf, limits)?;
                            let itunes = feed
                                .feed
                                .itunes
                                .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
                            itunes.explicit = parse_explicit(&text);
                            true
                        } else if is_itunes_tag(tag, b"keywords") && !is_empty {
                            let text = read_text_str(reader, &mut buf, limits)?;
                            let itunes = feed
                                .feed
                                .itunes
                                .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
                            itunes.keywords = text
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                            true
                        } else if is_itunes_tag(tag, b"type") && !is_empty {
                            let text = read_text_str(reader, &mut buf, limits)?;
                            let itunes = feed
                                .feed
                                .itunes
                                .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
                            itunes.podcast_type = Some(text);
                            true
                        } else if is_itunes_tag(tag, b"complete") && !is_empty {
                            let text = read_text_str(reader, &mut buf, limits)?;
                            let itunes = feed
                                .feed
                                .itunes
                                .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
                            itunes.complete = Some(text.trim().eq_ignore_ascii_case("Yes"));
                            true
                        } else if is_itunes_tag(tag, b"new-feed-url") && !is_empty {
                            let text = read_text_str(reader, &mut buf, limits)?;
                            if !text.is_empty() {
                                let itunes = feed
                                    .feed
                                    .itunes
                                    .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
                                itunes.new_feed_url = Some(text.trim().to_string().into());
                            }
                            true
                        } else if is_itunes_tag(tag, b"block") && !is_empty {
                            let text = read_text_str(reader, &mut buf, limits)?;
                            let itunes = feed
                                .feed
                                .itunes
                                .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
                            itunes.block = Some(u8::from(text.trim().eq_ignore_ascii_case("yes")));
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

/// Parse <entry> element
#[allow(clippy::too_many_lines)]
fn parse_entry(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
    depth: &mut usize,
    base_ctx: &BaseUrlContext,
    bozo: &mut bool,
    entry_lang: Option<&str>,
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
                        let text = parse_text_construct(reader, buf, &element, limits, entry_lang)?;
                        entry.set_title(text);
                    }
                    b"link" => {
                        if let Some(mut link) = Link::from_attributes(
                            element.attributes().flatten(),
                            limits.max_attribute_length,
                        ) {
                            link.href = base_ctx.resolve_safe(&link.href).into();

                            if entry.link.is_none() && link.rel.as_deref() == Some("alternate") {
                                entry.link = Some(link.href.to_string());
                            }
                            if entry.license.is_none() && link.rel.as_deref() == Some("license") {
                                entry.license = Some(link.href.to_string());
                            }
                            if link.rel.as_deref() == Some("enclosure") {
                                entry.enclosures.try_push_limited(
                                    Enclosure {
                                        url: link.href.clone(),
                                        length: link.length.clone(),
                                        enclosure_type: link.link_type.clone(),
                                    },
                                    limits.max_enclosures,
                                );
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
                        let (text, had_bozo) = read_text(reader, buf, limits)?;
                        *bozo |= had_bozo;
                        entry.id = Some(text.into());
                    }
                    b"updated" | b"modified" if !is_empty => {
                        let text = read_text_str(reader, buf, limits)?;
                        entry.updated = parse_date(&text);
                        entry.updated_str = Some(text);
                    }
                    b"published" | b"issued" if !is_empty => {
                        let text = read_text_str(reader, buf, limits)?;
                        entry.published = parse_date(&text);
                        entry.published_str = Some(text);
                    }
                    b"subtitle" if !is_empty => {
                        let text = parse_text_construct(reader, buf, &element, limits, entry_lang)?;
                        entry.set_subtitle(text);
                    }
                    b"rights" if !is_empty => {
                        let text = parse_text_construct(reader, buf, &element, limits, entry_lang)?;
                        entry.set_rights(text);
                    }
                    b"summary" if !is_empty => {
                        let text = parse_text_construct(reader, buf, &element, limits, entry_lang)?;
                        entry.set_summary(text);
                    }
                    b"content" => {
                        // Handle both inline content (<content>...</content>) and
                        // out-of-line content (<content src="..." />, RFC 4287 §4.1.3.2).
                        if is_empty {
                            if let Some(content) = parse_content_empty(&element, limits, entry_lang)
                            {
                                entry
                                    .content
                                    .try_push_limited(content, limits.max_content_blocks);
                            }
                        } else {
                            let content = parse_content(reader, buf, &element, limits, entry_lang)?;
                            entry
                                .content
                                .try_push_limited(content, limits.max_content_blocks);
                        }
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
                                let (text, had_bozo) = read_text(reader, buf, limits)?;
                                *bozo |= had_bozo;
                                dublin_core::handle_entry_element(&dc_elem, &text, &mut entry);
                            }
                            true
                        } else if let Some(content_element) = is_content_tag(tag) {
                            let content_elem = content_element.to_string();
                            if !is_empty {
                                let (text, had_bozo) = read_text(reader, buf, limits)?;
                                *bozo |= had_bozo;
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
                                        .media_thumbnail
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
                            } else if media_element == "group" && !is_empty {
                                // media:group is a transparent container; promote children to entry
                                parse_atom_media_group(
                                    reader, buf, &mut entry, limits, depth, bozo,
                                )?;
                            } else {
                                let media_elem = media_element.to_string();
                                if !is_empty {
                                    let (text, had_bozo) = read_text(reader, buf, limits)?;
                                    *bozo |= had_bozo;
                                    media_rss::handle_entry_element(&media_elem, &text, &mut entry);
                                }
                            }
                            true
                        } else if let Some(thr_element) = is_thr_tag(tag) {
                            // Atom Threading Extensions (RFC 4685)
                            match thr_element {
                                "in-reply-to" => {
                                    if let Some(reply) = threading::parse_in_reply_to_from_attrs(
                                        element.attributes().flatten(),
                                        limits.max_attribute_length,
                                    ) {
                                        // Shares max_links_per_entry limit; split if needed later
                                        entry
                                            .in_reply_to
                                            .try_push_limited(reply, limits.max_links_per_entry);
                                    }
                                    if !is_empty {
                                        skip_element(reader, buf, limits, *depth)?;
                                    }
                                }
                                "total" if !is_empty => {
                                    let (text, had_bozo) = read_text(reader, buf, limits)?;
                                    *bozo |= had_bozo;
                                    threading::handle_total(&text, &mut entry);
                                }
                                _ => {
                                    if !is_empty {
                                        skip_element(reader, buf, limits, *depth)?;
                                    }
                                }
                            }
                            true
                        } else if let Some(slash_element) = is_slash_tag(tag) {
                            let slash_elem = slash_element.to_string();
                            if !is_empty {
                                let (text, had_bozo) = read_text(reader, buf, limits)?;
                                *bozo |= had_bozo;
                                slash::handle_slash_entry_element(&slash_elem, &text, &mut entry);
                            }
                            true
                        } else if let Some(wfw_element) = is_wfw_tag(tag) {
                            let wfw_elem = wfw_element.to_string();
                            if !is_empty {
                                let (text, had_bozo) = read_text(reader, buf, limits)?;
                                *bozo |= had_bozo;
                                slash::handle_wfw_entry_element(&wfw_elem, &text, &mut entry);
                            }
                            true
                        } else if is_itunes_tag(tag, b"image") {
                            if let Some(url) = extract_href_attr(&element, limits) {
                                let itunes = entry
                                    .itunes
                                    .get_or_insert_with(|| Box::new(ItunesEntryMeta::default()));
                                itunes.image = Some(
                                    truncate_to_length(&url, limits.max_attribute_length).into(),
                                );
                            }
                            if !is_empty {
                                skip_element(reader, buf, limits, *depth)?;
                            }
                            true
                        } else if is_itunes_tag(tag, b"title") && !is_empty {
                            let (text, had_bozo) = read_text(reader, buf, limits)?;
                            *bozo |= had_bozo;
                            let itunes = entry
                                .itunes
                                .get_or_insert_with(|| Box::new(ItunesEntryMeta::default()));
                            itunes.title = Some(text);
                            true
                        } else if is_itunes_tag(tag, b"author") && !is_empty {
                            let (text, had_bozo) = read_text(reader, buf, limits)?;
                            *bozo |= had_bozo;
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
                            true
                        } else if is_itunes_tag(tag, b"subtitle") && !is_empty {
                            let (text, had_bozo) = read_text(reader, buf, limits)?;
                            *bozo |= had_bozo;
                            let itunes = entry
                                .itunes
                                .get_or_insert_with(|| Box::new(ItunesEntryMeta::default()));
                            itunes.subtitle = Some(text);
                            true
                        } else if is_itunes_tag(tag, b"summary") && !is_empty {
                            let (text, had_bozo) = read_text(reader, buf, limits)?;
                            *bozo |= had_bozo;
                            if entry.summary.is_none() {
                                entry.set_summary(TextConstruct::text(&text));
                            }
                            let itunes = entry
                                .itunes
                                .get_or_insert_with(|| Box::new(ItunesEntryMeta::default()));
                            itunes.summary = Some(text);
                            true
                        } else if is_itunes_tag(tag, b"duration") && !is_empty {
                            let (text, had_bozo) = read_text(reader, buf, limits)?;
                            *bozo |= had_bozo;
                            let itunes = entry
                                .itunes
                                .get_or_insert_with(|| Box::new(ItunesEntryMeta::default()));
                            itunes.duration = if text.is_empty() { None } else { Some(text) };
                            true
                        } else if is_itunes_tag(tag, b"explicit") && !is_empty {
                            let (text, had_bozo) = read_text(reader, buf, limits)?;
                            *bozo |= had_bozo;
                            let itunes = entry
                                .itunes
                                .get_or_insert_with(|| Box::new(ItunesEntryMeta::default()));
                            itunes.explicit = parse_explicit(&text);
                            true
                        } else if is_itunes_tag(tag, b"episode") && !is_empty {
                            let (text, had_bozo) = read_text(reader, buf, limits)?;
                            *bozo |= had_bozo;
                            let itunes = entry
                                .itunes
                                .get_or_insert_with(|| Box::new(ItunesEntryMeta::default()));
                            itunes.episode = text.trim().parse().ok();
                            true
                        } else if is_itunes_tag(tag, b"season") && !is_empty {
                            let (text, had_bozo) = read_text(reader, buf, limits)?;
                            *bozo |= had_bozo;
                            let itunes = entry
                                .itunes
                                .get_or_insert_with(|| Box::new(ItunesEntryMeta::default()));
                            itunes.season = text.trim().parse().ok();
                            true
                        } else if is_itunes_tag(tag, b"episodeType") && !is_empty {
                            let (text, had_bozo) = read_text(reader, buf, limits)?;
                            *bozo |= had_bozo;
                            let itunes = entry
                                .itunes
                                .get_or_insert_with(|| Box::new(ItunesEntryMeta::default()));
                            itunes.episode_type = Some(text);
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

    // Atom uses <id> not <guid>; guidislink is always false when <id> is present.
    // Python feedparser never sets guidislink=true for Atom entries.
    entry.guidislink = entry.id.as_ref().map(|_| false);

    Ok(entry)
}

/// Promote entry.id → entry.link when no explicit `<link>` is present (#273).
///
/// `guidislink` is always `Some(false)` for Atom entries (set before this call).
fn promote_entry_id_to_link(entry: &mut Entry) {
    if entry.link.is_none() {
        if let Some(id) = entry.id.as_deref() {
            entry.link = Some(id.to_string());
        }
    }
}

/// Fallback entry.updated from entry.published when `<updated>` is absent (#275).
fn promote_entry_published_to_updated(entry: &mut Entry) {
    if entry.updated.is_none() {
        entry.updated = entry.published;
        if entry.updated_str.is_none() {
            entry.updated_str.clone_from(&entry.published_str);
        }
    }
}

/// Parse Atom text construct (title, summary, rights, etc.)
fn parse_text_construct(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    e: &quick_xml::events::BytesStart,
    limits: &ParserLimits,
    lang: Option<&str>,
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

    let value = match content_type {
        TextType::Xhtml => read_xhtml_content_str(reader, buf, limits)?,
        _ => read_text_str(reader, buf, limits)?,
    };

    Ok(TextConstruct {
        value,
        content_type,
        language: lang.map(Into::into),
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
                    b"name" => name = Some(read_text_str(reader, buf, limits)?.into()),
                    b"email" => email = Some(read_text_str(reader, buf, limits)?.into()),
                    b"uri" => uri = Some(read_text_str(reader, buf, limits)?),
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
            b"version" => version = Some(bytes_to_string(&attr.value).into()),
            _ => {}
        }
    }

    Ok(Generator {
        name: read_text_str(reader, buf, limits)?,
        href: uri,
        version,
    })
}

/// Parse <content> element
fn parse_content(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    e: &quick_xml::events::BytesStart,
    limits: &ParserLimits,
    lang: Option<&str>,
) -> Result<Content> {
    let mut content_type = None;
    let mut is_xhtml = false;
    let mut src = None;

    for attr in e.attributes().flatten() {
        if attr.value.len() > limits.max_attribute_length {
            continue;
        }
        match attr.key.as_ref() {
            b"type" => {
                if attr.value.as_ref() == b"xhtml" {
                    is_xhtml = true;
                }
                let normalized = match attr.value.as_ref() {
                    b"xhtml" => "application/xhtml+xml".to_string(),
                    b"html" => "text/html".to_string(),
                    b"text" => "text/plain".to_string(),
                    _ => bytes_to_string(&attr.value),
                };
                content_type = Some(normalized.into());
            }
            b"src" => src = Some(bytes_to_string(&attr.value)),
            _ => {}
        }
    }

    // RFC 4287 §4.1.3.2: when src is present, content is out-of-line; value is empty.
    if src.is_some() {
        skip_to_end(reader, buf, b"content")?;
        return Ok(Content {
            value: String::new(),
            content_type,
            language: lang.map(Into::into),
            base: None,
            src,
        });
    }

    let value = if is_xhtml {
        read_xhtml_content_str(reader, buf, limits)?
    } else {
        read_text_str(reader, buf, limits)?
    };

    Ok(Content {
        value,
        content_type,
        language: lang.map(Into::into),
        base: None,
        src: None,
    })
}

/// Parse children of `<media:group>` in an Atom entry.
///
/// `<media:group>` is a transparent container per the Media RSS spec; its children
/// are treated as if they appeared directly under the entry element.
fn parse_atom_media_group(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    entry: &mut Entry,
    limits: &ParserLimits,
    depth: &mut usize,
    bozo: &mut bool,
) -> Result<()> {
    loop {
        buf.clear();
        match reader.read_event_into(buf) {
            Ok(Event::Empty(e)) => {
                let tag = e.name().as_ref().to_vec();
                handle_atom_media_group_child(&tag, &e, entry, limits);
            }
            Ok(Event::Start(e)) => {
                let tag = e.name().as_ref().to_vec();
                handle_atom_media_group_child(&tag, &e, entry, limits);
                *depth += 1;
                skip_element(reader, buf, limits, *depth)?;
                *depth = depth.saturating_sub(1);
            }
            Ok(Event::End(_) | Event::Eof) => break,
            Err(_) => {
                *bozo = true;
                break;
            }
            _ => {}
        }
    }
    Ok(())
}

fn handle_atom_media_group_child(
    tag: &[u8],
    element: &quick_xml::events::BytesStart<'_>,
    entry: &mut Entry,
    limits: &ParserLimits,
) {
    let Some(child_elem) = is_media_tag(tag) else {
        return;
    };
    match child_elem {
        "content" => {
            if let Some(media) = MediaContent::from_attributes(
                element.attributes().flatten(),
                limits.max_attribute_length,
            ) {
                entry
                    .media_content
                    .try_push_limited(media, limits.max_enclosures);
            }
        }
        "thumbnail" => {
            if let Some(thumbnail) = MediaThumbnail::from_attributes(
                element.attributes().flatten(),
                limits.max_attribute_length,
            ) {
                entry
                    .media_thumbnail
                    .try_push_limited(thumbnail, limits.max_enclosures);
            }
        }
        _ => {}
    }
}

/// Parse self-closing `<content ... />` elements (out-of-line content with `src` attribute).
///
/// Returns `None` when `src` is absent (empty inline content with no body is not useful).
fn parse_content_empty(
    e: &quick_xml::events::BytesStart,
    limits: &ParserLimits,
    lang: Option<&str>,
) -> Option<Content> {
    let mut content_type = None;
    let mut src = None;

    for attr in e.attributes().flatten() {
        if attr.value.len() > limits.max_attribute_length {
            continue;
        }
        match attr.key.as_ref() {
            b"type" => {
                let normalized = match attr.value.as_ref() {
                    b"xhtml" => "application/xhtml+xml".to_string(),
                    b"html" => "text/html".to_string(),
                    b"text" => "text/plain".to_string(),
                    _ => bytes_to_string(&attr.value),
                };
                content_type = Some(normalized.into());
            }
            b"src" => src = Some(bytes_to_string(&attr.value)),
            _ => {}
        }
    }

    src.map(|src_val| Content {
        value: String::new(),
        content_type,
        language: lang.map(Into::into),
        base: None,
        src: Some(src_val),
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
    let mut first_link_href: Option<String> = None;
    let mut id = None;
    let mut links = Vec::new();
    let mut updated = None;
    let mut updated_str = None;
    let mut rights = None;
    let mut has_explicit_link = false;
    let mut author = None;

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
                    b"title" if !is_empty => title = Some(read_text_str(reader, buf, limits)?),
                    b"link" => {
                        if let Some(lnk) = Link::from_attributes(
                            element.attributes().flatten(),
                            limits.max_attribute_length,
                        ) {
                            // Track first alternate rel for link; fall back to first link seen.
                            if lnk.rel.as_deref() == Some("alternate") && link.is_none() {
                                link = Some(lnk.href.to_string());
                            }
                            if first_link_href.is_none() {
                                first_link_href = Some(lnk.href.to_string());
                            }
                            has_explicit_link = true;
                            links.push(lnk);
                        }
                        if !is_empty {
                            skip_to_end(reader, buf, b"link")?;
                        }
                    }
                    b"id" if !is_empty => id = Some(read_text_str(reader, buf, limits)?),
                    b"updated" | b"modified" if !is_empty => {
                        let text = read_text_str(reader, buf, limits)?;
                        updated = parse_date(&text);
                        updated_str = Some(text);
                    }
                    b"rights" if !is_empty => {
                        rights = Some(read_text_str(reader, buf, limits)?);
                    }
                    b"author" if !is_empty => {
                        if let Ok(person) = parse_person(reader, buf, limits, depth) {
                            author = person.flat_string().map(|s| s.to_string());
                        }
                    }
                    _ if !is_empty => skip_element(reader, buf, limits, *depth)?,
                    _ => {}
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

    // Fall back to first link of any rel if no alternate was found
    if link.is_none() {
        link = first_link_href;
    }

    // Compute guidislink per Python feedparser semantics:
    // - None when no <id> present
    // - Some(true) when <id> looks like a URL and no explicit <link> present
    // - Some(false) otherwise
    let guidislink = id.as_deref().map(|id_val| {
        let id_is_url = id_val.starts_with("http://")
            || id_val.starts_with("https://")
            || id_val.starts_with("ftp://");
        id_is_url && !has_explicit_link
    });

    // When guidislink is true, populate link from the id value (matching Python feedparser)
    if guidislink == Some(true) {
        link.clone_from(&id);
    }

    Ok(Source {
        title,
        href: None,
        link,
        author,
        id,
        links,
        updated,
        updated_str,
        rights,
        guidislink,
    })
}

/// Extract the `href` attribute value from an XML element, truncated to limit.
fn extract_href_attr(
    element: &quick_xml::events::BytesStart<'_>,
    limits: &ParserLimits,
) -> Option<String> {
    for attr in element.attributes().flatten() {
        if attr.key.as_ref() == b"href" && attr.value.len() <= limits.max_attribute_length {
            return String::from_utf8(attr.value.into_owned()).ok();
        }
    }
    None
}

/// Parse `<itunes:owner>` element (name and email children).
fn parse_atom_itunes_owner(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
    depth: &mut usize,
) -> Result<ItunesOwner> {
    let mut owner = ItunesOwner::default();
    loop {
        buf.clear();
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
    }
    Ok(owner)
}

/// Parse `<itunes:category>` with optional nested subcategory.
fn parse_atom_itunes_category(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    element: &quick_xml::events::BytesStart<'_>,
    feed: &mut ParsedFeed,
    limits: &ParserLimits,
    is_empty: bool,
) -> Result<()> {
    let text = element
        .attributes()
        .flatten()
        .find(|a| a.key.as_ref() == b"text")
        .and_then(|a| String::from_utf8(a.value.into_owned()).ok())
        .unwrap_or_default();

    if text.is_empty() {
        if !is_empty {
            skip_element(reader, buf, limits, 0)?;
        }
        return Ok(());
    }

    let mut subcategory: Option<String> = None;

    if !is_empty {
        loop {
            buf.clear();
            match reader.read_event_into(buf) {
                Ok(Event::Empty(e)) if is_itunes_tag(e.name().as_ref(), b"category") => {
                    subcategory = e
                        .attributes()
                        .flatten()
                        .find(|a| a.key.as_ref() == b"text")
                        .and_then(|a| String::from_utf8(a.value.into_owned()).ok());
                }
                Ok(Event::Start(e)) if is_itunes_tag(e.name().as_ref(), b"category") => {
                    subcategory = e
                        .attributes()
                        .flatten()
                        .find(|a| a.key.as_ref() == b"text")
                        .and_then(|a| String::from_utf8(a.value.into_owned()).ok());
                    skip_to_end(reader, buf, b"category")?;
                }
                Ok(Event::End(_) | Event::Eof) => break,
                Err(e) => return Err(e.into()),
                _ => {}
            }
        }
    }

    let itunes = feed
        .feed
        .itunes
        .get_or_insert_with(|| Box::new(ItunesFeedMeta::default()));
    itunes.categories.push(ItunesCategory { text, subcategory });
    Ok(())
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
        assert_eq!(
            feed.feed.author.as_deref(),
            Some("John Doe (john@example.com)")
        );
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
        assert_eq!(generator_detail.name, "Example CMS");
        assert_eq!(
            generator_detail.href.as_deref(),
            Some("http://example.com/")
        );
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
    fn test_parse_atom_source_link_before_id() {
        // Regression test for issue #174: skip_to_end on empty <link/> consumed
        // subsequent siblings including <id>, causing source.id to return None.
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <title>Test</title>
            <id>urn:test</id>
            <entry>
                <title>Entry</title>
                <id>urn:entry</id>
                <source>
                    <title>Source</title>
                    <link href="http://x.com/"/>
                    <id>source-id-here</id>
                </source>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        let source = feed.entries[0].source.as_ref().unwrap();
        assert_eq!(source.id.as_deref(), Some("source-id-here"));
        assert_eq!(source.link.as_deref(), Some("http://x.com/"));
    }

    #[test]
    fn test_parse_atom_source_updated() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <entry>
                <id>e1</id>
                <source>
                    <title>Source Feed</title>
                    <id>urn:source</id>
                    <updated>2025-01-12T00:00:00Z</updated>
                </source>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        let source = feed.entries[0].source.as_ref().unwrap();
        assert!(source.updated.is_some());
        assert_eq!(source.updated_str.as_deref(), Some("2025-01-12T00:00:00Z"));
    }

    #[test]
    fn test_parse_atom_source_rights() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <entry>
                <id>e1</id>
                <source>
                    <title>Source Feed</title>
                    <rights>Copyright 2025</rights>
                </source>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        let source = feed.entries[0].source.as_ref().unwrap();
        assert_eq!(source.rights.as_deref(), Some("Copyright 2025"));
    }

    #[test]
    fn test_parse_atom_source_links() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <entry>
                <id>e1</id>
                <source>
                    <title>Source</title>
                    <link href="http://a.com/" rel="alternate"/>
                    <link href="http://a.com/feed" rel="self"/>
                </source>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        let source = feed.entries[0].source.as_ref().unwrap();
        assert_eq!(source.links.len(), 2);
        assert_eq!(source.link.as_deref(), Some("http://a.com/"));
    }

    #[test]
    fn test_parse_atom_source_guidislink_true() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <entry>
                <id>e1</id>
                <source>
                    <title>Source</title>
                    <id>http://example.com/feed</id>
                </source>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        let source = feed.entries[0].source.as_ref().unwrap();
        assert_eq!(source.guidislink, Some(true));
        assert_eq!(source.link.as_deref(), Some("http://example.com/feed"));
    }

    #[test]
    fn test_parse_atom_source_guidislink_false_with_link() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <entry>
                <id>e1</id>
                <source>
                    <title>Source</title>
                    <id>http://example.com/feed</id>
                    <link href="http://other.com/"/>
                </source>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        let source = feed.entries[0].source.as_ref().unwrap();
        assert_eq!(source.guidislink, Some(false));
        assert_eq!(source.link.as_deref(), Some("http://other.com/"));
    }

    #[test]
    fn test_parse_atom_source_guidislink_urn_id() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <entry>
                <id>e1</id>
                <source>
                    <title>Source</title>
                    <id>urn:uuid:60a76c80-d399-11d9</id>
                </source>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        let source = feed.entries[0].source.as_ref().unwrap();
        assert_eq!(source.guidislink, Some(false));
        assert!(source.link.is_none());
    }

    #[test]
    fn test_parse_atom_source_no_id() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <entry>
                <id>e1</id>
                <source>
                    <title>Source</title>
                    <link href="http://a.com/"/>
                </source>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        let source = feed.entries[0].source.as_ref().unwrap();
        assert!(source.guidislink.is_none());
        assert_eq!(source.link.as_deref(), Some("http://a.com/"));
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
    fn test_parse_atom_link_type_defaults() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <link href="http://example.com/" rel="alternate"/>
            <link href="http://example.com/feed" rel="self"/>
            <link href="http://hub.example.com/" rel="hub"/>
            <link href="http://example.com/audio.mp3" rel="enclosure"/>
            <link href="http://example.com/explicit" rel="alternate" type="application/xhtml+xml"/>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        let links = &feed.feed.links;
        assert_eq!(links.len(), 5);

        let alternate = links
            .iter()
            .find(|l| l.rel.as_deref() == Some("alternate") && !l.href.contains("explicit"))
            .unwrap();
        assert_eq!(
            alternate.link_type.as_deref(),
            Some("text/html"),
            "alternate without type should default to text/html"
        );

        let self_link = links
            .iter()
            .find(|l| l.rel.as_deref() == Some("self"))
            .unwrap();
        assert_eq!(
            self_link.link_type.as_deref(),
            Some("application/atom+xml"),
            "self without type should default to application/atom+xml"
        );

        let hub = links
            .iter()
            .find(|l| l.rel.as_deref() == Some("hub"))
            .unwrap();
        assert_eq!(
            hub.link_type.as_deref(),
            Some("text/html"),
            "hub without type should default to text/html"
        );

        let enclosure = links
            .iter()
            .find(|l| l.rel.as_deref() == Some("enclosure"))
            .unwrap();
        assert_eq!(
            enclosure.link_type.as_deref(),
            Some("text/html"),
            "enclosure without type should default to text/html"
        );

        let explicit = links.iter().find(|l| l.href.contains("explicit")).unwrap();
        assert_eq!(
            explicit.link_type.as_deref(),
            Some("application/xhtml+xml"),
            "explicit type must be preserved"
        );
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
        let title_detail = feed.feed.title_detail.as_ref().unwrap();
        assert_eq!(title_detail.content_type, TextType::Xhtml);
        assert_eq!(title_detail.value, "XHTML Title");
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

    #[test]
    fn test_parse_atom_license_feed() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <title>Test Feed</title>
            <link rel="license" href="https://creativecommons.org/licenses/by/4.0/"/>
            <link rel="alternate" href="https://example.com/"/>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert_eq!(
            feed.feed.license.as_deref(),
            Some("https://creativecommons.org/licenses/by/4.0/")
        );
        assert_eq!(feed.feed.link.as_deref(), Some("https://example.com/"));
    }

    #[test]
    fn test_parse_atom_license_entry() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <entry>
                <title>Licensed Entry</title>
                <id>urn:uuid:1</id>
                <link rel="license" href="https://creativecommons.org/licenses/by-sa/3.0/"/>
                <link rel="alternate" href="https://example.com/entry/1"/>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.entries.len(), 1);
        assert_eq!(
            feed.entries[0].license.as_deref(),
            Some("https://creativecommons.org/licenses/by-sa/3.0/")
        );
        assert_eq!(
            feed.entries[0].link.as_deref(),
            Some("https://example.com/entry/1")
        );
    }

    #[test]
    fn test_parse_atom_feed_next_url() {
        let xml = include_bytes!("../../../../tests/fixtures/atom-pagination.xml");
        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        assert_eq!(
            feed.feed.next_url.as_deref(),
            Some("http://example.com/feed?page=2")
        );
    }

    #[test]
    fn test_parse_atom_feed_next_url_absent() {
        let xml = br#"<?xml version="1.0" encoding="utf-8"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <title>No Pagination</title>
            <link href="http://example.com/" rel="alternate"/>
            <id>urn:uuid:no-pagination</id>
            <updated>2024-01-01T00:00:00Z</updated>
        </feed>"#;
        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        assert!(feed.feed.next_url.is_none());
    }

    #[test]
    fn test_thr_count_and_updated_happy_path() {
        let xml = br#"<?xml version="1.0" encoding="utf-8"?>
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:thr="http://purl.org/syndication/thread/1.0">
          <title>Test</title>
          <id>urn:uuid:test</id>
          <updated>2024-01-15T12:00:00Z</updated>
          <entry>
            <title>Post</title>
            <id>urn:uuid:entry-1</id>
            <updated>2024-01-15T12:00:00Z</updated>
            <link rel="replies" href="http://example.com/replies"
                  thr:count="10" thr:updated="2024-01-15T12:00:00Z"/>
          </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        let replies_link = feed.entries[0]
            .links
            .iter()
            .find(|l| l.rel.as_deref() == Some("replies"))
            .expect("replies link");
        assert_eq!(replies_link.thr_count, Some(10));
        assert!(replies_link.thr_updated.is_some());
    }

    #[test]
    fn test_thr_count_zero() {
        let xml = br#"<?xml version="1.0" encoding="utf-8"?>
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:thr="http://purl.org/syndication/thread/1.0">
          <title>Test</title>
          <id>urn:uuid:test</id>
          <updated>2024-01-15T12:00:00Z</updated>
          <entry>
            <title>Post</title>
            <id>urn:uuid:entry-1</id>
            <updated>2024-01-15T12:00:00Z</updated>
            <link rel="replies" href="http://example.com/replies" thr:count="0"/>
          </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        let replies_link = feed.entries[0]
            .links
            .iter()
            .find(|l| l.rel.as_deref() == Some("replies"))
            .expect("replies link");
        assert_eq!(replies_link.thr_count, Some(0));
    }

    #[test]
    fn test_thr_count_whitespace_trimmed() {
        let xml = br#"<?xml version="1.0" encoding="utf-8"?>
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:thr="http://purl.org/syndication/thread/1.0">
          <title>Test</title>
          <id>urn:uuid:test</id>
          <updated>2024-01-15T12:00:00Z</updated>
          <entry>
            <title>Post</title>
            <id>urn:uuid:entry-1</id>
            <updated>2024-01-15T12:00:00Z</updated>
            <link rel="replies" href="http://example.com/replies"
                  thr:count=" 10 " thr:updated=" 2024-01-15T12:00:00Z "/>
          </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        let replies_link = feed.entries[0]
            .links
            .iter()
            .find(|l| l.rel.as_deref() == Some("replies"))
            .expect("replies link");
        assert_eq!(replies_link.thr_count, Some(10));
        assert!(replies_link.thr_updated.is_some());
    }

    #[test]
    fn test_thr_attrs_missing_yields_none() {
        let xml = br#"<?xml version="1.0" encoding="utf-8"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
          <title>Test</title>
          <id>urn:uuid:test</id>
          <updated>2024-01-15T12:00:00Z</updated>
          <entry>
            <title>Post</title>
            <id>urn:uuid:entry-1</id>
            <updated>2024-01-15T12:00:00Z</updated>
            <link rel="replies" href="http://example.com/replies"/>
          </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        let replies_link = feed.entries[0]
            .links
            .iter()
            .find(|l| l.rel.as_deref() == Some("replies"))
            .expect("replies link");
        assert_eq!(replies_link.thr_count, None);
        assert!(replies_link.thr_updated.is_none());
    }

    #[test]
    fn test_thr_count_malformed_no_bozo() {
        let xml = br#"<?xml version="1.0" encoding="utf-8"?>
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:thr="http://purl.org/syndication/thread/1.0">
          <title>Test</title>
          <id>urn:uuid:test</id>
          <updated>2024-01-15T12:00:00Z</updated>
          <entry>
            <title>Post</title>
            <id>urn:uuid:entry-1</id>
            <updated>2024-01-15T12:00:00Z</updated>
            <link rel="replies" href="http://example.com/replies"
                  thr:count="abc" thr:updated="not-a-date"/>
          </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo, "malformed thr: attrs must not set bozo");
        let replies_link = feed.entries[0]
            .links
            .iter()
            .find(|l| l.rel.as_deref() == Some("replies"))
            .expect("replies link");
        assert_eq!(replies_link.thr_count, None);
        assert!(replies_link.thr_updated.is_none());
    }

    #[test]
    fn test_thr_count_negative_no_bozo() {
        let xml = br#"<?xml version="1.0" encoding="utf-8"?>
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:thr="http://purl.org/syndication/thread/1.0">
          <title>Test</title>
          <id>urn:uuid:test</id>
          <updated>2024-01-15T12:00:00Z</updated>
          <entry>
            <title>Post</title>
            <id>urn:uuid:entry-1</id>
            <updated>2024-01-15T12:00:00Z</updated>
            <link rel="replies" href="http://example.com/replies" thr:count="-5"/>
          </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        let replies_link = feed.entries[0]
            .links
            .iter()
            .find(|l| l.rel.as_deref() == Some("replies"))
            .expect("replies link");
        assert_eq!(replies_link.thr_count, None);
    }

    #[test]
    fn test_thr_count_overflow_no_bozo() {
        let xml = br#"<?xml version="1.0" encoding="utf-8"?>
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:thr="http://purl.org/syndication/thread/1.0">
          <title>Test</title>
          <id>urn:uuid:test</id>
          <updated>2024-01-15T12:00:00Z</updated>
          <entry>
            <title>Post</title>
            <id>urn:uuid:entry-1</id>
            <updated>2024-01-15T12:00:00Z</updated>
            <link rel="replies" href="http://example.com/replies"
                  thr:count="99999999999"/>
          </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        let replies_link = feed.entries[0]
            .links
            .iter()
            .find(|l| l.rel.as_deref() == Some("replies"))
            .expect("replies link");
        assert_eq!(replies_link.thr_count, None);
    }

    #[test]
    fn test_thr_count_on_non_replies_link() {
        let xml = br#"<?xml version="1.0" encoding="utf-8"?>
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:thr="http://purl.org/syndication/thread/1.0">
          <title>Test</title>
          <id>urn:uuid:test</id>
          <updated>2024-01-15T12:00:00Z</updated>
          <entry>
            <title>Post</title>
            <id>urn:uuid:entry-1</id>
            <updated>2024-01-15T12:00:00Z</updated>
            <link rel="alternate" href="http://example.com/post" thr:count="5"/>
          </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        let alt_link = feed.entries[0]
            .links
            .iter()
            .find(|l| l.rel.as_deref() == Some("alternate"))
            .expect("alternate link");
        assert_eq!(alt_link.thr_count, Some(5));
    }

    #[test]
    fn test_parse_entry_rights() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <title>Test</title>
            <entry>
                <title>Entry with rights</title>
                <id>entry1</id>
                <updated>2024-01-01T00:00:00Z</updated>
                <rights>Copyright 2024 Example Corp</rights>
            </entry>
            <entry>
                <title>Entry without rights</title>
                <id>entry2</id>
                <updated>2024-01-01T00:00:00Z</updated>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        assert_eq!(
            feed.entries[0].rights.as_deref(),
            Some("Copyright 2024 Example Corp")
        );
        assert!(feed.entries[0].rights_detail.is_some());
        assert!(feed.entries[1].rights.is_none());
    }

    #[test]
    fn test_parse_atom_xml_lang_feed_language() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom" xml:lang="de">
            <title>German Example</title>
            <subtitle>Subtitle</subtitle>
            <rights>All rights reserved</rights>
            <id>urn:test:lang-feed</id>
            <updated>2024-01-01T00:00:00Z</updated>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        assert_eq!(feed.feed.language.as_deref(), Some("de"));
        assert_eq!(
            feed.feed.title_detail.as_ref().unwrap().language.as_deref(),
            Some("de")
        );
        assert_eq!(
            feed.feed
                .subtitle_detail
                .as_ref()
                .unwrap()
                .language
                .as_deref(),
            Some("de")
        );
        assert_eq!(
            feed.feed
                .rights_detail
                .as_ref()
                .unwrap()
                .language
                .as_deref(),
            Some("de")
        );
    }

    #[test]
    fn test_parse_atom_xml_lang_entry_inherits_feed() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom" xml:lang="de">
            <title>Feed</title>
            <id>urn:test</id>
            <updated>2024-01-01T00:00:00Z</updated>
            <entry>
                <title>Entry inheriting feed lang</title>
                <id>entry1</id>
                <updated>2024-01-01T00:00:00Z</updated>
                <summary>Summary</summary>
                <content>Body</content>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        assert_eq!(
            feed.entries[0]
                .title_detail
                .as_ref()
                .unwrap()
                .language
                .as_deref(),
            Some("de")
        );
        assert_eq!(
            feed.entries[0]
                .summary_detail
                .as_ref()
                .unwrap()
                .language
                .as_deref(),
            Some("de")
        );
        assert_eq!(feed.entries[0].content[0].language.as_deref(), Some("de"));
    }

    #[test]
    fn test_parse_atom_xml_lang_entry_overrides_feed() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom" xml:lang="de">
            <title>Feed</title>
            <id>urn:test</id>
            <updated>2024-01-01T00:00:00Z</updated>
            <entry xml:lang="fr">
                <title>Entry in French</title>
                <id>entry1</id>
                <updated>2024-01-01T00:00:00Z</updated>
                <summary>Summary in French</summary>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        assert_eq!(feed.feed.language.as_deref(), Some("de"));
        assert_eq!(
            feed.entries[0]
                .title_detail
                .as_ref()
                .unwrap()
                .language
                .as_deref(),
            Some("fr")
        );
        assert_eq!(
            feed.entries[0]
                .summary_detail
                .as_ref()
                .unwrap()
                .language
                .as_deref(),
            Some("fr")
        );
    }

    #[test]
    fn test_parse_atom_xml_lang_no_language() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <title>No Language</title>
            <id>urn:test</id>
            <updated>2024-01-01T00:00:00Z</updated>
            <entry>
                <title>Entry</title>
                <id>entry1</id>
                <updated>2024-01-01T00:00:00Z</updated>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        assert!(feed.feed.language.is_none());
        assert!(feed.feed.title_detail.as_ref().unwrap().language.is_none());
        assert!(
            feed.entries[0]
                .title_detail
                .as_ref()
                .unwrap()
                .language
                .is_none()
        );
    }

    #[test]
    fn test_parse_atom_xml_lang_invalid_tag_passthrough() {
        // Invalid BCP 47 values must be stored as-is (bozo pattern: no validation).
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom" xml:lang="not-a-real-lang">
            <title>Test</title>
            <id>urn:test</id>
            <updated>2024-01-01T00:00:00Z</updated>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        assert_eq!(feed.feed.language.as_deref(), Some("not-a-real-lang"));
        assert_eq!(
            feed.feed.title_detail.as_ref().unwrap().language.as_deref(),
            Some("not-a-real-lang")
        );
    }

    #[test]
    fn test_parse_atom03_xml_lang() {
        // Atom 0.3 uses the same code path — xml:lang should propagate identically.
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://purl.org/atom/ns#" xml:lang="ja">
            <title>Japanese Feed</title>
            <id>urn:test:atom03</id>
            <modified>2024-01-01T00:00:00Z</modified>
            <entry>
                <title>Article</title>
                <id>entry1</id>
                <modified>2024-01-01T00:00:00Z</modified>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.version, FeedVersion::Atom03);
        assert_eq!(feed.feed.language.as_deref(), Some("ja"));
        assert_eq!(
            feed.feed.title_detail.as_ref().unwrap().language.as_deref(),
            Some("ja")
        );
        assert_eq!(
            feed.entries[0]
                .title_detail
                .as_ref()
                .unwrap()
                .language
                .as_deref(),
            Some("ja")
        );
    }

    #[test]
    fn test_atom_content_fallback_to_summary() {
        let xml = br#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
<title>T</title><id>u</id><updated>2026-01-01T00:00:00Z</updated>
<entry><title>E</title><id>u2</id><updated>2026-01-01T00:00:00Z</updated>
  <content type="html">&lt;p&gt;Content only&lt;/p&gt;</content></entry>
</feed>"#;
        let feed = parse_atom10(xml).unwrap();
        assert_eq!(
            feed.entries[0].summary.as_deref(),
            Some("<p>Content only</p>")
        );
    }

    #[test]
    fn test_atom_namespaces_default_and_prefixed() {
        let xml = br#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom"
      xmlns:dc="http://purl.org/dc/elements/1.1/">
<title>T</title><id>u</id><updated>2026-01-01T00:00:00Z</updated>
</feed>"#;
        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        assert_eq!(
            feed.namespaces.get("").map(String::as_str),
            Some("http://www.w3.org/2005/Atom")
        );
        assert_eq!(
            feed.namespaces.get("dc").map(String::as_str),
            Some("http://purl.org/dc/elements/1.1/")
        );
    }

    #[test]
    fn test_atom_no_namespaces() {
        let xml = br#"<?xml version="1.0"?>
<feed><title>T</title><id>u</id><updated>2026-01-01T00:00:00Z</updated></feed>"#;
        let feed = parse_atom10(xml).unwrap();
        assert!(feed.namespaces.is_empty());
    }

    #[test]
    fn test_atom03_detected_via_namespaces() {
        let xml = br#"<?xml version="1.0"?>
<feed xmlns="http://purl.org/atom/ns#">
<title>T</title><id>u</id><modified>2004-01-01T00:00:00Z</modified>
</feed>"#;
        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.version, crate::types::FeedVersion::Atom03);
        assert_eq!(
            feed.namespaces.get("").map(String::as_str),
            Some("http://purl.org/atom/ns#")
        );
    }

    #[test]
    fn test_atom_itunes_feed_metadata() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <title>Podcast Feed</title>
            <itunes:author>Jane Doe</itunes:author>
            <itunes:subtitle>A great show</itunes:subtitle>
            <itunes:summary>Long description</itunes:summary>
            <itunes:explicit>yes</itunes:explicit>
            <itunes:image href="https://example.com/cover.jpg"/>
            <itunes:type>serial</itunes:type>
            <itunes:complete>Yes</itunes:complete>
            <itunes:new-feed-url>https://example.com/new.xml</itunes:new-feed-url>
            <itunes:block>yes</itunes:block>
            <itunes:keywords>tech, rust</itunes:keywords>
            <itunes:category text="Technology">
                <itunes:category text="Software"/>
            </itunes:category>
            <itunes:owner>
                <itunes:name>Owner Name</itunes:name>
                <itunes:email>owner@example.com</itunes:email>
            </itunes:owner>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        let itunes = feed.feed.itunes.as_ref().unwrap();
        assert_eq!(itunes.author.as_deref(), Some("Jane Doe"));
        assert_eq!(itunes.subtitle.as_deref(), Some("A great show"));
        assert_eq!(itunes.summary.as_deref(), Some("Long description"));
        assert_eq!(itunes.explicit, Some(true));
        assert_eq!(
            itunes.image.as_deref(),
            Some("https://example.com/cover.jpg")
        );
        assert_eq!(
            feed.feed.image.as_ref().map(|i| i.url.as_str()),
            Some("https://example.com/cover.jpg")
        );
        assert_eq!(itunes.podcast_type.as_deref(), Some("serial"));
        assert_eq!(itunes.complete, Some(true));
        assert_eq!(
            itunes.new_feed_url.as_deref(),
            Some("https://example.com/new.xml")
        );
        assert_eq!(itunes.block, Some(1));
        assert_eq!(itunes.keywords, vec!["tech", "rust"]);
        assert_eq!(itunes.categories.len(), 1);
        assert_eq!(itunes.categories[0].text, "Technology");
        assert_eq!(
            itunes.categories[0].subcategory.as_deref(),
            Some("Software")
        );
        let owner = itunes.owner.as_ref().unwrap();
        assert_eq!(owner.name.as_deref(), Some("Owner Name"));
        assert_eq!(owner.email.as_deref(), Some("owner@example.com"));
        // itunes:author promotes to feed.author
        assert_eq!(feed.feed.author.as_deref(), Some("Jane Doe"));
    }

    #[test]
    fn test_atom_itunes_entry_metadata() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <title>Podcast</title>
            <entry>
                <id>ep1</id>
                <title>Episode One</title>
                <itunes:title>iTunes Title</itunes:title>
                <itunes:author>Episode Author</itunes:author>
                <itunes:duration>1:23:45</itunes:duration>
                <itunes:explicit>yes</itunes:explicit>
                <itunes:image href="https://example.com/ep.jpg"/>
                <itunes:episode>5</itunes:episode>
                <itunes:season>2</itunes:season>
                <itunes:episodeType>full</itunes:episodeType>
                <itunes:subtitle>Ep subtitle</itunes:subtitle>
                <itunes:summary>Ep summary</itunes:summary>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        let itunes = feed.entries[0].itunes.as_ref().unwrap();
        assert_eq!(itunes.title.as_deref(), Some("iTunes Title"));
        assert_eq!(itunes.author.as_deref(), Some("Episode Author"));
        assert_eq!(itunes.duration.as_deref(), Some("1:23:45"));
        assert_eq!(itunes.explicit, Some(true));
        assert_eq!(itunes.image.as_deref(), Some("https://example.com/ep.jpg"));
        assert_eq!(itunes.episode.as_deref(), Some("5"));
        assert_eq!(itunes.season.as_deref(), Some("2"));
        assert_eq!(itunes.episode_type.as_deref(), Some("full"));
        assert_eq!(itunes.subtitle.as_deref(), Some("Ep subtitle"));
        assert_eq!(itunes.summary.as_deref(), Some("Ep summary"));
    }

    #[test]
    fn test_atom_itunes_explicit_no_returns_none() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <title>P</title>
            <itunes:explicit>no</itunes:explicit>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        let itunes = feed.feed.itunes.as_ref().unwrap();
        assert_eq!(itunes.explicit, None);
    }

    // Regression tests for fixes #262, #252, #251

    #[test]
    fn test_atom_source_link_field_populated() {
        // Fix #262: Atom <source><link href="..."/> should populate source.link, not source.href
        let xml = br#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
<title>T</title><id>u</id><updated>2026-01-01T00:00:00Z</updated>
<entry><title>E</title><id>e1</id><updated>2026-01-01T00:00:00Z</updated>
  <source>
    <title>Origin</title>
    <id>urn:source</id>
    <link href="http://origin.example.com/"/>
  </source>
</entry>
</feed>"#;
        let feed = parse_atom10(xml).unwrap();
        let source = feed.entries[0].source.as_ref().unwrap();
        assert_eq!(source.link.as_deref(), Some("http://origin.example.com/"));
        assert!(source.href.is_none(), "href must be None for Atom sources");
    }

    #[test]
    fn test_atom_source_author_field() {
        // Fix #262: Atom <source><author> should populate source.author
        let xml = br#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
<title>T</title><id>u</id><updated>2026-01-01T00:00:00Z</updated>
<entry><title>E</title><id>e1</id><updated>2026-01-01T00:00:00Z</updated>
  <source>
    <title>Origin</title>
    <author><name>Alice</name><email>alice@example.com</email></author>
  </source>
</entry>
</feed>"#;
        let feed = parse_atom10(xml).unwrap();
        let source = feed.entries[0].source.as_ref().unwrap();
        assert_eq!(source.author.as_deref(), Some("Alice (alice@example.com)"));
    }

    #[test]
    fn test_atom_content_src_attribute() {
        // Fix #252: <content src="..."> should parse out-of-line content
        let xml = br#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
<title>T</title><id>u</id><updated>2026-01-01T00:00:00Z</updated>
<entry><title>E</title><id>e1</id><updated>2026-01-01T00:00:00Z</updated>
  <content type="image/png" src="http://example.com/image.png"/>
</entry>
</feed>"#;
        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.entries[0].content.is_empty());
        let content = &feed.entries[0].content[0];
        assert_eq!(content.src.as_deref(), Some("http://example.com/image.png"));
        assert_eq!(content.value, "");
        assert_eq!(content.content_type.as_deref(), Some("image/png"));
    }

    #[test]
    fn test_atom_author_flat_string_with_email() {
        // Fix #251: flat author string should be "Name (email)" when email is present
        let xml = br#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
<title>T</title><id>u</id><updated>2026-01-01T00:00:00Z</updated>
<author><name>Bob</name><email>bob@example.com</email></author>
<entry><title>E</title><id>e1</id><updated>2026-01-01T00:00:00Z</updated>
  <author><name>Carol</name><email>carol@example.com</email></author>
</entry>
</feed>"#;
        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.feed.author.as_deref(), Some("Bob (bob@example.com)"));
        assert_eq!(
            feed.entries[0].author.as_deref(),
            Some("Carol (carol@example.com)")
        );
    }

    #[test]
    fn test_atom_author_flat_string_name_only() {
        // Fix #251: when no email, flat string is just the name (no regression)
        let xml = br#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
<title>T</title><id>u</id><updated>2026-01-01T00:00:00Z</updated>
<entry><title>E</title><id>e1</id><updated>2026-01-01T00:00:00Z</updated>
  <author><name>Dave</name></author>
</entry>
</feed>"#;
        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.entries[0].author.as_deref(), Some("Dave"));
    }

    #[test]
    fn test_atom_entry_guidislink_is_false_when_id_present() {
        // Atom entries with <id> must have guidislink=Some(false)
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <title>Test</title>
            <entry>
                <title>Entry</title>
                <id>urn:uuid:1234</id>
                <updated>2024-01-01T00:00:00Z</updated>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.entries[0].guidislink, Some(false));
    }

    #[test]
    fn test_atom_entry_guidislink_is_none_when_no_id() {
        // Atom entries without <id> must have guidislink=None
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <title>Test</title>
            <entry>
                <title>Entry without id</title>
                <updated>2024-01-01T00:00:00Z</updated>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.entries[0].guidislink, None);
    }
}
