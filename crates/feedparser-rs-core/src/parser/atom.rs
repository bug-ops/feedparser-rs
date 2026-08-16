//! Atom 1.0 parser implementation

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::{
    ParserLimits,
    error::{FeedError, Result},
    namespace::{content, dublin_core, georss, media_rss, slash, threading},
    types::{
        Content, Enclosure, Entry, FeedMeta, FeedVersion, Generator, Image, ItunesCategory,
        ItunesFeedMeta, ItunesOwner, Link, MediaContent, MediaCopyright, MediaCredit,
        MediaThumbnail, MimeType, ParsedFeed, Person, Source, Tag, TextConstruct, TextType,
        parse_explicit,
    },
    util::{base_url::BaseUrlContext, parse_date, text::truncate_to_length},
};
use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};

use super::common::{
    EVENT_BUFFER_CAPACITY, FromAttributes, LimitedCollectionExt, bytes_to_string, check_depth,
    extract_namespaces, extract_xml_base, extract_xml_lang, init_feed, is_content_tag, is_dc_tag,
    is_geo_tag, is_georss_tag, is_itunes_tag, is_media_tag, is_slash_tag, is_thr_tag, is_wfw_tag,
    itunes_entry_meta, itunes_feed_meta, parse_georss_where, read_text, read_text_str,
    read_xhtml_content_str, skip_element, skip_to_end, skip_to_end_qualified,
    text_construct_from_content,
};
use super::context::{EntryCtx, XmlCtx};

/// Feed-tier parse context: XML plumbing plus the read-only xml:base/xml:lang
/// inherited from the `<feed>` root element.
///
/// Never mutates `base` — unlike `EntryCtx`, the feed tier only ever derives
/// child contexts via `base_ctx.child()` for entries, so `base` is a shared
/// reference rather than owned.
struct FeedCtx<'r, 'd, 'p> {
    /// XML event-loop plumbing (reader, buffer, limits).
    xml: XmlCtx<'r, 'd>,
    /// xml:base resolution context inherited from the `<feed>` root element.
    base: &'p BaseUrlContext,
    /// xml:lang inherited by elements that don't declare their own.
    lang: Option<&'p str>,
}

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
///
/// Relative URI resolution is always enabled; use [`parse_atom10_with_options`] to
/// control it.
pub fn parse_atom10_with_limits(data: &[u8], limits: ParserLimits) -> Result<ParsedFeed> {
    parse_atom10_with_options(data, limits, true)
}

/// Parse Atom with custom limits and relative URI resolution control
pub fn parse_atom10_with_options(
    data: &[u8],
    limits: ParserLimits,
    resolve_relative_uris: bool,
) -> Result<ParsedFeed> {
    limits
        .check_feed_size(data.len())
        .map_err(|e| FeedError::InvalidFormat(e.to_string()))?;

    let mut reader = Reader::from_reader(data);

    let mut feed = init_feed(FeedVersion::Atom10, limits.max_entries);
    let mut buf = Vec::with_capacity(EVENT_BUFFER_CAPACITY);
    let mut depth: usize = 1;
    let mut base_ctx = BaseUrlContext::new().with_resolve(resolve_relative_uris);
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
                // Post-process: iTunes subtitle/summary always win (order-independent)
                apply_itunes_feed_promotions(&mut feed.feed);
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

    // RFC 4287 §4.1.2: entries with no authors inherit feed-level authors.
    if !feed.feed.authors.is_empty() {
        let feed_authors = feed.feed.authors.clone();
        for entry in &mut feed.entries {
            if entry.authors.is_empty() {
                entry.authors.clone_from(&feed_authors);
                if entry.author.is_none() {
                    entry.author.clone_from(&feed.feed.author);
                }
            }
        }
    }

    Ok(feed)
}

/// Apply iTunes subtitle/summary promotions to feed-level fields.
///
/// Called after all XML elements have been parsed. Atom uses a single-loop model so
/// itunes: and standard elements appear in document order — post-processing ensures
/// promotion is order-independent. Empty/whitespace-only values do not override valid data.
fn apply_itunes_feed_promotions(feed: &mut FeedMeta) {
    // Clone to avoid simultaneous immutable + mutable borrow on `feed`.
    let subtitle = feed.itunes.as_ref().and_then(|it| it.subtitle.clone());
    let summary = feed.itunes.as_ref().and_then(|it| it.summary.clone());

    if let Some(ref s) = subtitle
        && !s.trim().is_empty()
    {
        feed.set_subtitle(TextConstruct::html(s));
    }
    if let Some(ref s) = summary
        && !s.trim().is_empty()
    {
        feed.set_summary(TextConstruct::html(s));
        if feed.subtitle.is_none() {
            feed.set_subtitle(TextConstruct::html(s));
        }
    }
}

/// Apply iTunes subtitle/summary promotions to entry-level fields.
///
/// Called after all XML elements in an entry have been parsed.
fn apply_itunes_entry_promotions(entry: &mut Entry) {
    // Clone to avoid simultaneous immutable + mutable borrow on `entry`.
    let subtitle = entry.itunes.as_ref().and_then(|it| it.subtitle.clone());
    let summary = entry.itunes.as_ref().and_then(|it| it.summary.clone());

    if let Some(ref s) = subtitle
        && !s.trim().is_empty()
    {
        entry.set_subtitle(TextConstruct::html(s));
    }
    if let Some(ref s) = summary
        && !s.trim().is_empty()
    {
        entry.set_summary(TextConstruct::html(s));
    }
}

/// Record a feed-level field parse failure as bozo instead of propagating it.
///
/// Invariant: feed-level field parse failures must not abort sibling `<entry>`
/// recovery — mirrors the catch-and-continue already applied to `<entry>` parsing
/// (see `parse_feed_entry`).
///
/// First error wins: `bozo_exception` is only set the first time, matching the
/// convention used by `parse_feed_entry`, so a later, less informative failure
/// does not clobber the original diagnostic.
fn record_feed_bozo(feed: &mut ParsedFeed, err: &FeedError) {
    if !feed.bozo {
        feed.bozo_exception = Some(err.to_string());
    }
    feed.bozo = true;
}

/// Recover from a feed-level field parse error caught by `parse_feed_element`.
///
/// Records the error as bozo (see [`record_feed_bozo`]), then drains the reader to
/// `tag`'s own closing tag and restores `depth` to its pre-dispatch value. Without
/// draining, a partially-consumed container's leftover children would be read next
/// by the enclosing loop and misdispatched as if they were real feed-level siblings,
/// silently overwriting unrelated feed metadata (#463).
fn recover_feed_field_error(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    tag: &[u8],
    depth: &mut usize,
    depth_before: usize,
    feed: &mut ParsedFeed,
    err: &FeedError,
) {
    record_feed_bozo(feed, err);
    skip_to_end_qualified(reader, buf, tag);
    *depth = depth_before;
}

/// `FeedCtx`-taking wrapper around [`recover_feed_field_error`], so
/// `dispatch_feed_tag`'s call sites don't each need to spell out
/// `ctx.xml.reader, ctx.xml.buf` inline.
fn recover_feed_error(
    ctx: &mut FeedCtx,
    tag: &[u8],
    depth: &mut usize,
    depth_before: usize,
    feed: &mut ParsedFeed,
    err: &FeedError,
) {
    recover_feed_field_error(
        ctx.xml.reader,
        ctx.xml.buf,
        tag,
        depth,
        depth_before,
        feed,
        err,
    );
}

/// Parse <feed> element
fn parse_feed_element(
    reader: &mut Reader<&[u8]>,
    feed: &mut ParsedFeed,
    limits: &ParserLimits,
    depth: &mut usize,
    base_ctx: &BaseUrlContext,
    feed_lang: Option<&str>,
) -> Result<()> {
    let mut buf = Vec::with_capacity(EVENT_BUFFER_CAPACITY);
    let mut ctx = FeedCtx {
        xml: XmlCtx {
            reader,
            buf: &mut buf,
            limits,
        },
        base: base_ctx,
        lang: feed_lang,
    };

    loop {
        match ctx.xml.reader.read_event_into(ctx.xml.buf) {
            Ok(event @ (Event::Start(_) | Event::Empty(_))) => {
                let is_empty = matches!(event, Event::Empty(_));
                let (Event::Start(e) | Event::Empty(e)) = &event else {
                    unreachable!()
                };

                *depth += 1;
                check_depth(*depth, ctx.xml.limits.max_nesting_depth)?;

                let element = e.to_owned();
                if !dispatch_feed_tag(&mut ctx, &element, feed, depth, is_empty)? {
                    continue;
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
        ctx.xml.buf.clear();
    }

    Ok(())
}

/// Dispatch a single `<feed>` child element to its handler.
///
/// Returns `Ok(true)` for normal flow (the caller decrements `depth` and later
/// clears its event buffer as usual). Returns `Ok(false)` only for the `entry`
/// branch's entry-limit-skip path, which has already adjusted `depth` itself —
/// the caller must `continue` its loop without touching `depth`/`buf` again
/// (mirrors `parse_feed_entry`'s own return-value convention).
fn dispatch_feed_tag(
    ctx: &mut FeedCtx,
    element: &BytesStart<'_>,
    feed: &mut ParsedFeed,
    depth: &mut usize,
    is_empty: bool,
) -> Result<bool> {
    // Snapshot of `depth` after entering this element: restored after a caught
    // error so a partially consumed nested element can never leak a leftover
    // increment into the caller's own depth bookkeeping.
    let depth_before = *depth;

    // Use name() instead of local_name() to preserve namespace prefixes
    //
    // NOTE: the tag lists below must stay in sync with the inner `match` in each
    // handler (parse_feed_core_text, parse_feed_link_or_category, parse_feed_person)
    // — a tag routed here that the handler doesn't also match falls through the
    // handler's silent `_ => {}` and consumes no events, desyncing the event stream.
    match element.name().as_ref() {
        tag @ (b"title" | b"subtitle" | b"tagline" | b"id" | b"updated" | b"modified"
        | b"published" | b"issued" | b"rights" | b"copyright" | b"generator" | b"icon"
        | b"logo")
            if !is_empty =>
        {
            if let Err(e) = parse_feed_core_text(ctx, element, feed) {
                recover_feed_error(ctx, tag, depth, depth_before, feed, &e);
            }
        }
        // NOTE: not guarded by `!is_empty` here (matches pre-existing behavior), but
        // `parse_feed_link_or_category` only calls `skip_to_end` — the sole source of
        // `Err` below — when `!is_empty` itself, so `recover_feed_field_error`'s drain
        // always has a matching closing tag to find when this arm's `Err` fires.
        tag @ (b"link" | b"category") => {
            if let Err(e) = parse_feed_link_or_category(ctx, element, feed, is_empty) {
                recover_feed_error(ctx, tag, depth, depth_before, feed, &e);
            }
        }
        b"author" | b"contributor" if !is_empty => {
            parse_feed_person(ctx, element, feed, depth)?;
        }
        b"entry" if !is_empty => {
            if !parse_feed_entry(ctx, element, feed, depth) {
                return Ok(false);
            }
        }
        tag => {
            if let Err(e) = parse_feed_namespace(ctx, tag, element, feed, depth, is_empty) {
                recover_feed_error(ctx, tag, depth, depth_before, feed, &e);
            }
        }
    }
    Ok(true)
}

/// Parse extension namespace tags at feed level (Dublin Core, Content, Media RSS,
/// Threading, iTunes, `GeoRSS`/Geo), in order.
fn parse_feed_namespace(
    ctx: &mut FeedCtx,
    tag: &[u8],
    element: &BytesStart<'_>,
    feed: &mut ParsedFeed,
    depth: &mut usize,
    is_empty: bool,
) -> Result<()> {
    // Check for namespace elements, in order
    let mut handled = parse_feed_ns_text(ctx, tag, feed, *depth, is_empty)?;
    if !handled {
        handled = parse_feed_media(ctx, tag, element, feed, *depth, is_empty)?;
    }
    if !handled {
        handled = parse_feed_ns_threading(ctx, tag, *depth, is_empty)?;
    }
    if !handled {
        handled = parse_feed_itunes_structured(ctx, tag, element, feed, depth, is_empty)?;
    }
    if !handled {
        handled = parse_feed_itunes_text(ctx, tag, feed, is_empty)?;
    }
    if !handled {
        handled = parse_feed_geo(ctx, tag, feed, *depth, is_empty)?;
    }

    if !handled && !is_empty {
        skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, *depth)?;
    }
    Ok(())
}

/// Parse core text-valued feed elements: title, subtitle/tagline, id, updated/modified,
/// published/issued, rights/copyright, generator, icon, logo.
fn parse_feed_core_text(
    ctx: &mut FeedCtx,
    element: &BytesStart<'_>,
    feed: &mut ParsedFeed,
) -> Result<()> {
    // keep tag list in sync with the dispatcher arm in parse_feed_element
    match element.name().as_ref() {
        b"title" => {
            let text = parse_text_construct(&mut ctx.xml, element, ctx.lang, ctx.base)?;
            feed.feed.set_title(text);
        }
        b"subtitle" | b"tagline" => {
            let text = parse_text_construct(&mut ctx.xml, element, ctx.lang, ctx.base)?;
            feed.feed.set_subtitle(text);
        }
        b"id" => {
            let (text, bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            if bozo {
                feed.bozo = true;
                feed.bozo_exception = Some("Unresolvable entity in feed id".to_string());
            }
            feed.feed.id = Some(text);
        }
        b"updated" | b"modified" => {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            feed.feed.updated = parse_date(&text);
            feed.feed.updated_str = Some(text);
        }
        b"published" | b"issued" => {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            feed.feed.published = parse_date(&text);
            feed.feed.published_str = Some(text);
        }
        b"generator" => {
            let generator = parse_generator(ctx.xml.reader, ctx.xml.buf, element, ctx.xml.limits)?;
            feed.feed.set_generator(generator);
        }
        b"icon" => {
            let url = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            feed.feed.icon = Some(ctx.base.resolve_safe(&url));
        }
        b"logo" => {
            let url = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            feed.feed.logo = Some(ctx.base.resolve_safe(&url));
        }
        b"rights" | b"copyright" => {
            let text = parse_text_construct(&mut ctx.xml, element, ctx.lang, ctx.base)?;
            feed.feed.set_rights(text);
        }
        _ => {}
    }
    Ok(())
}

/// Parse feed-level `<link>` and `<category>` elements.
fn parse_feed_link_or_category(
    ctx: &mut FeedCtx,
    element: &BytesStart<'_>,
    feed: &mut ParsedFeed,
    is_empty: bool,
) -> Result<()> {
    // keep tag list in sync with the dispatcher arm in parse_feed_element
    match element.name().as_ref() {
        b"link" => {
            if let Some(mut link) = Link::from_attributes(
                element.attributes().flatten(),
                ctx.xml.limits.max_attribute_length,
            ) {
                link.href = ctx.base.resolve_safe(&link.href).into();

                if feed.feed.link.is_none() && link.rel.as_deref() == Some("alternate") {
                    feed.feed.link = Some(link.href.to_string());
                }
                if feed.feed.license.is_none() && link.rel.as_deref() == Some("license") {
                    feed.feed.license = Some(link.href.to_string());
                }
                if feed.feed.next_url.is_none() && link.rel.as_deref() == Some("next") {
                    feed.feed.next_url = Some(link.href.to_string());
                }
                feed.feed
                    .links
                    .try_push_limited(link, ctx.xml.limits.max_links_per_feed);
            }
            if !is_empty {
                skip_to_end(ctx.xml.reader, ctx.xml.buf, b"link")?;
            }
        }
        b"category" => {
            if let Some(tag) = Tag::from_attributes(
                element.attributes().flatten(),
                ctx.xml.limits.max_attribute_length,
            ) {
                feed.feed
                    .tags
                    .try_push_limited(tag, ctx.xml.limits.max_tags);
            }
            if !is_empty {
                skip_to_end(ctx.xml.reader, ctx.xml.buf, b"category")?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Parse feed-level `<author>` and `<contributor>` elements.
#[allow(clippy::unnecessary_wraps)]
fn parse_feed_person(
    ctx: &mut FeedCtx,
    element: &BytesStart<'_>,
    feed: &mut ParsedFeed,
    depth: &mut usize,
) -> Result<()> {
    // keep tag list in sync with the dispatcher arm in parse_feed_element
    match element.name().as_ref() {
        b"author" => {
            if let Ok(person) = parse_person(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth) {
                if feed.feed.author.is_none() {
                    feed.feed.set_author(person.clone());
                }
                feed.feed
                    .authors
                    .try_push_limited(person, ctx.xml.limits.max_authors);
            }
        }
        b"contributor" => {
            if let Ok(person) = parse_person(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth) {
                feed.feed
                    .contributors
                    .try_push_limited(person, ctx.xml.limits.max_contributors);
            }
        }
        _ => {}
    }
    Ok(())
}

/// Parse a feed-level `<entry>` element and push it onto `feed.entries`.
///
/// Returns `false` when the caller must `continue` the event loop WITHOUT
/// decrementing depth and WITHOUT `buf.clear()` — this function has already
/// decremented depth internally, either because the entry limit was hit
/// (`check_entry_limit` returned `Ok(false)`) or because `check_entry_limit`'s
/// internal `skip_element` itself failed while skipping an over-limit entry
/// (caught here as bozo instead of propagated, per #463).
fn parse_feed_entry(
    ctx: &mut FeedCtx,
    element: &BytesStart<'_>,
    feed: &mut ParsedFeed,
    depth: &mut usize,
) -> bool {
    match feed.check_entry_limit(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth) {
        Ok(true) => {}
        Ok(false) => return false,
        Err(e) => {
            // `skip_element` inside `check_entry_limit` failed (nesting overflow or
            // ill-formed XML while skipping an over-limit entry) — record bozo and
            // resync to </entry> instead of letting the error abort the whole feed
            // (same invariant as #463's field-level fix, reached via a different
            // path). `check_entry_limit` already set `feed.bozo` before attempting
            // the skip, so first-error-wins naturally keeps "Entry limit exceeded".
            if !feed.bozo {
                feed.bozo_exception = Some(e.to_string());
            }
            feed.bozo = true;
            skip_to_end_qualified(ctx.xml.reader, ctx.xml.buf, b"entry");
            *depth = depth.saturating_sub(1);
            return false;
        }
    }

    let mut entry_ctx = ctx.base.child();
    if let Some(xml_base) = extract_xml_base(element, ctx.xml.limits.max_attribute_length) {
        entry_ctx.update_base(&xml_base);
    }

    // Entry-level xml:lang overrides feed-level; fall back to feed_lang.
    let entry_lang_owned = extract_xml_lang(element, ctx.xml.limits.max_attribute_length);
    let effective_lang = entry_lang_owned.as_deref().or(ctx.lang);

    let depth_before = *depth;
    match parse_entry(
        &mut ctx.xml,
        depth,
        &entry_ctx,
        effective_lang,
        &feed.namespaces,
    ) {
        Ok((mut entry, entry_bozo, bozo_reason)) => {
            if entry_bozo && !feed.bozo {
                feed.bozo = true;
                feed.bozo_exception = Some(
                    bozo_reason
                        .unwrap_or("Unresolvable entity in entry field")
                        .to_string(),
                );
            }
            if entry.summary.is_none()
                && let Some(content) = entry.content.first()
            {
                entry.summary = Some(content.value.clone());
                entry.summary_detail = Some(text_construct_from_content(content));
            }
            // #278: dc:creator fallback for author
            if entry.author.is_none()
                && let Some(dc) = &entry.dc_creator
            {
                entry.author = Some(dc.clone());
            }
            // #273: promote entry.id → entry.link when no explicit link
            promote_entry_id_to_link(&mut entry);
            // #275: fallback entry.updated from entry.published
            promote_entry_published_to_updated(&mut entry);
            // Post-process: iTunes subtitle/summary promotion (order-independent)
            apply_itunes_entry_promotions(&mut entry);
            feed.entries.push(entry);
        }
        Err(e) => {
            // `parse_entry` failed partway through the entry's own content (e.g. a
            // malformed entity in one of its fields) and left the reader positioned
            // mid-entry. Without draining, the entry's remaining children — several
            // of which share tag names with real feed-level fields (`title`, `link`,
            // `category`) — would be read next by the enclosing `parse_feed_element`
            // loop and misdispatched as feed siblings, corrupting feed metadata with
            // values ripped out of this entry (#463 S3).
            if !feed.bozo {
                feed.bozo_exception = Some(e.to_string());
            }
            feed.bozo = true;
            skip_to_end_qualified(ctx.xml.reader, ctx.xml.buf, b"entry");
            *depth = depth_before;
        }
    }
    true
}

/// Parse Dublin Core and Content namespace tags at feed level.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_feed_ns_text(
    ctx: &mut FeedCtx,
    tag: &[u8],
    feed: &mut ParsedFeed,
    depth: usize,
    is_empty: bool,
) -> Result<bool> {
    if let Some(dc_element) = is_dc_tag(tag, &feed.namespaces) {
        let dc_elem = dc_element.to_string();
        if !is_empty {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            dublin_core::handle_feed_element(&dc_elem, &text, &mut feed.feed);
        }
        Ok(true)
    } else if is_content_tag(tag).is_some() {
        // Content namespace - typically entry-level
        if !is_empty {
            skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse Media RSS namespace tags at feed level.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_feed_media(
    ctx: &mut FeedCtx,
    tag: &[u8],
    element: &BytesStart<'_>,
    feed: &mut ParsedFeed,
    depth: usize,
    is_empty: bool,
) -> Result<bool> {
    let Some(media_element) = is_media_tag(tag, &feed.namespaces) else {
        return Ok(false);
    };
    match media_element {
        "thumbnail" => {
            if let Some(thumb) = MediaThumbnail::from_attributes(
                element.attributes().flatten(),
                ctx.xml.limits.max_attribute_length,
            ) {
                feed.feed
                    .media_thumbnail
                    .try_push_limited(thumb, ctx.xml.limits.max_enclosures);
            }
            if !is_empty {
                skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
            }
        }
        "content" => {
            if let Some(content) = MediaContent::from_attributes(
                element.attributes().flatten(),
                ctx.xml.limits.max_attribute_length,
            ) {
                feed.feed
                    .media_content
                    .try_push_limited(content, ctx.xml.limits.max_enclosures);
            }
            if !is_empty {
                skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
            }
        }
        "rating" | "keywords" => {
            if !is_empty {
                let scheme = element
                    .attributes()
                    .flatten()
                    .find(|a| a.key.as_ref() == b"scheme")
                    .and_then(|a| {
                        a.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                            .ok()
                            .map(|v| truncate_to_length(&v, ctx.xml.limits.max_attribute_length))
                    });
                let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
                media_rss::handle_feed_element(
                    media_element,
                    scheme.as_deref(),
                    &text,
                    &mut feed.feed,
                );
            }
        }
        _ => {
            if !is_empty {
                skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
            }
        }
    }
    Ok(true)
}

/// Parse Atom Threading Extensions at feed level (unusual; recognized and skipped).
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_feed_ns_threading(
    ctx: &mut FeedCtx,
    tag: &[u8],
    depth: usize,
    is_empty: bool,
) -> Result<bool> {
    if is_thr_tag(tag).is_some() {
        if !is_empty {
            skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse structured iTunes namespace tags at feed level: image, category, owner.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_feed_itunes_structured(
    ctx: &mut FeedCtx,
    tag: &[u8],
    element: &BytesStart<'_>,
    feed: &mut ParsedFeed,
    depth: &mut usize,
    is_empty: bool,
) -> Result<bool> {
    if is_itunes_tag(tag, b"image", &feed.namespaces) {
        if let Some(url) = extract_href_attr(element, ctx.xml.limits) {
            itunes_feed_meta(&mut feed.feed).image = Some(url.clone().into());
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
            skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, *depth)?;
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"category", &feed.namespaces) {
        parse_atom_itunes_category(ctx, element, feed, is_empty)?;
        Ok(true)
    } else if is_itunes_tag(tag, b"owner", &feed.namespaces) && !is_empty {
        if let Ok(owner) =
            parse_atom_itunes_owner(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)
        {
            itunes_feed_meta(&mut feed.feed).owner = Some(owner);
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse text-valued iTunes namespace tags at feed level.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_feed_itunes_text(
    ctx: &mut FeedCtx,
    tag: &[u8],
    feed: &mut ParsedFeed,
    is_empty: bool,
) -> Result<bool> {
    if is_itunes_tag(tag, b"author", &feed.namespaces) && !is_empty {
        let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        if feed.feed.author.is_none() {
            feed.feed.set_author(Person::from_name(&text));
            feed.feed
                .authors
                .try_push_limited(Person::from_name(&text), ctx.xml.limits.max_authors);
        }
        itunes_feed_meta(&mut feed.feed).author = Some(text);
        Ok(true)
    } else if is_itunes_tag(tag, b"subtitle", &feed.namespaces) && !is_empty {
        let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        itunes_feed_meta(&mut feed.feed).subtitle = Some(text);
        Ok(true)
    } else if is_itunes_tag(tag, b"summary", &feed.namespaces) && !is_empty {
        let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        itunes_feed_meta(&mut feed.feed).summary = Some(text);
        Ok(true)
    } else if is_itunes_tag(tag, b"explicit", &feed.namespaces) && !is_empty {
        let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        itunes_feed_meta(&mut feed.feed).explicit = parse_explicit(&text);
        Ok(true)
    } else if is_itunes_tag(tag, b"keywords", &feed.namespaces) && !is_empty {
        let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        itunes_feed_meta(&mut feed.feed).keywords = text
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Ok(true)
    } else if is_itunes_tag(tag, b"type", &feed.namespaces) && !is_empty {
        let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        itunes_feed_meta(&mut feed.feed).podcast_type = Some(text);
        Ok(true)
    } else if is_itunes_tag(tag, b"complete", &feed.namespaces) && !is_empty {
        let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        itunes_feed_meta(&mut feed.feed).complete = Some(text.trim().to_string());
        Ok(true)
    } else if is_itunes_tag(tag, b"new-feed-url", &feed.namespaces) && !is_empty {
        let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        if !text.is_empty() {
            itunes_feed_meta(&mut feed.feed).new_feed_url = Some(text.trim().to_string().into());
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"block", &feed.namespaces) && !is_empty {
        let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        itunes_feed_meta(&mut feed.feed).block =
            Some(u8::from(text.trim().eq_ignore_ascii_case("yes")));
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse `GeoRSS` and W3C Geo namespace tags at feed level.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_feed_geo(
    ctx: &mut FeedCtx,
    tag: &[u8],
    feed: &mut ParsedFeed,
    depth: usize,
    is_empty: bool,
) -> Result<bool> {
    if let Some(georss_element) = is_georss_tag(tag) {
        if !is_empty {
            if georss_element == "where" {
                let (loc, had_bozo, bozo_reason) =
                    parse_georss_where(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
                if had_bozo && !feed.bozo {
                    feed.bozo = true;
                    feed.bozo_exception = Some(
                        bozo_reason
                            .unwrap_or("Unresolvable entity in feed field")
                            .to_string(),
                    );
                }
                if let Some(loc) = loc {
                    georss::merge_geometry(&mut feed.feed.r#where, loc);
                }
            } else {
                let georss_elem = georss_element.as_bytes().to_vec();
                let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
                georss::handle_feed_element(&georss_elem, &text, &mut feed.feed, ctx.xml.limits);
            }
        }
        Ok(true)
    } else if let Some(geo_element) = is_geo_tag(tag) {
        let geo_elem = geo_element.as_bytes().to_vec();
        if !is_empty {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            georss::handle_feed_geo_element(&geo_elem, &text, &mut feed.feed);
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse <entry> element
///
/// Returns `(entry, bozo, bozo_reason)`, where `bozo` reflects any
/// unresolved entity reference or GML coordinate/dims mismatch encountered
/// while reading this entry's text fields, and `bozo_reason` is a specific
/// description when available.
fn parse_entry(
    xml: &mut XmlCtx,
    depth: &mut usize,
    base_ctx: &BaseUrlContext,
    entry_lang: Option<&str>,
    namespaces: &HashMap<String, String>,
) -> Result<(Entry, bool, Option<&'static str>)> {
    let mut entry = Entry::with_capacity();
    let mut ctx = EntryCtx {
        xml: xml.reborrow(),
        base: base_ctx,
        lang: entry_lang,
        namespaces,
        bozo: false,
        bozo_reason: None,
    };

    loop {
        match ctx.xml.reader.read_event_into(ctx.xml.buf) {
            Ok(event @ (Event::Start(_) | Event::Empty(_))) => {
                let is_empty = matches!(event, Event::Empty(_));
                let (Event::Start(e) | Event::Empty(e)) = &event else {
                    unreachable!()
                };

                *depth += 1;
                check_depth(*depth, ctx.xml.limits.max_nesting_depth)?;

                let element = e.to_owned();
                // Use name() instead of local_name() to preserve namespace prefixes
                //
                // NOTE: the tag lists below must stay in sync with the inner `match` in
                // each handler (parse_entry_core_text, parse_entry_link_or_category,
                // parse_entry_person) — a tag routed here that the handler doesn't also
                // match falls through the handler's silent `_ => {}` and consumes no
                // events, desyncing the event stream.
                match element.name().as_ref() {
                    b"title" | b"id" | b"updated" | b"modified" | b"published" | b"issued"
                    | b"created" | b"subtitle" | b"tagline" | b"rights" | b"copyright"
                    | b"summary"
                        if !is_empty =>
                    {
                        parse_entry_core_text(&mut ctx, &element, &mut entry)?;
                    }
                    b"content" => {
                        parse_entry_content(&mut ctx, &element, &mut entry, is_empty)?;
                    }
                    b"link" | b"category" => {
                        parse_entry_link_or_category(&mut ctx, &element, &mut entry, is_empty)?;
                    }
                    b"author" | b"contributor" if !is_empty => {
                        parse_entry_person(&mut ctx, &element, &mut entry, depth)?;
                    }
                    b"source" if !is_empty => {
                        if let Ok(source) =
                            parse_atom_source(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)
                        {
                            entry.source = Some(source);
                        }
                    }
                    tag => {
                        parse_entry_namespace(
                            &mut ctx, tag, &element, &mut entry, depth, is_empty,
                        )?;
                    }
                }
                *depth = depth.saturating_sub(1);
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"entry" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        ctx.xml.buf.clear();
    }

    Ok((entry, ctx.bozo, ctx.bozo_reason))
}

/// Parse extension namespace tags at entry level (Dublin Core, Content, Media RSS,
/// Threading, iTunes, `GeoRSS`/Geo), in order.
fn parse_entry_namespace(
    ctx: &mut EntryCtx,
    tag: &[u8],
    element: &BytesStart<'_>,
    entry: &mut Entry,
    depth: &mut usize,
    is_empty: bool,
) -> Result<()> {
    let mut handled = parse_entry_ns_text(ctx, tag, entry, is_empty)?;
    if !handled {
        handled = parse_entry_media(ctx, tag, element, entry, depth, is_empty)?;
    }
    if !handled {
        handled = parse_entry_ns_threading(ctx, tag, element, entry, *depth, is_empty)?;
    }
    if !handled {
        handled = parse_entry_itunes(ctx, tag, element, entry, *depth, is_empty)?;
    }
    if !handled {
        handled = parse_entry_geo(ctx, tag, entry, *depth, is_empty)?;
    }

    if !handled && !is_empty {
        skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, *depth)?;
    }
    Ok(())
}

/// Parse core text-valued entry elements: title, id, updated/modified,
/// published/issued, created, subtitle/tagline, rights/copyright, summary.
fn parse_entry_core_text(
    ctx: &mut EntryCtx,
    element: &BytesStart<'_>,
    entry: &mut Entry,
) -> Result<()> {
    // keep tag list in sync with the dispatcher arm in parse_entry
    match element.name().as_ref() {
        b"title" => {
            let text = parse_text_construct(&mut ctx.xml, element, ctx.lang, ctx.base)?;
            entry.set_title(text);
        }
        b"id" => {
            let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            ctx.bozo |= had_bozo;
            entry.id = Some(text.into());
        }
        b"updated" | b"modified" => {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            entry.updated = parse_date(&text);
            entry.updated_str = Some(text);
        }
        b"published" | b"issued" => {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            entry.published = parse_date(&text);
            entry.published_str = Some(text);
        }
        b"created" => {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            entry.created = parse_date(&text);
            entry.created_str = Some(text);
        }
        b"subtitle" | b"tagline" => {
            let text = parse_text_construct(&mut ctx.xml, element, ctx.lang, ctx.base)?;
            entry.set_subtitle(text);
        }
        b"rights" | b"copyright" => {
            let text = parse_text_construct(&mut ctx.xml, element, ctx.lang, ctx.base)?;
            entry.set_rights(text);
        }
        b"summary" => {
            let text = parse_text_construct(&mut ctx.xml, element, ctx.lang, ctx.base)?;
            entry.set_summary(text);
        }
        _ => {}
    }
    Ok(())
}

/// Parse an entry-level `<content>` element (inline or out-of-line, RFC 4287 §4.1.3.2).
fn parse_entry_content(
    ctx: &mut EntryCtx,
    element: &BytesStart<'_>,
    entry: &mut Entry,
    is_empty: bool,
) -> Result<()> {
    if is_empty {
        if let Some(content) = parse_content_empty(element, ctx.xml.limits, ctx.lang, ctx.base) {
            entry
                .content
                .try_push_limited(content, ctx.xml.limits.max_content_blocks);
        }
    } else {
        let content = parse_content(ctx, element)?;
        entry
            .content
            .try_push_limited(content, ctx.xml.limits.max_content_blocks);
    }
    Ok(())
}

/// Parse entry-level `<link>` and `<category>` elements.
fn parse_entry_link_or_category(
    ctx: &mut EntryCtx,
    element: &BytesStart<'_>,
    entry: &mut Entry,
    is_empty: bool,
) -> Result<()> {
    // keep tag list in sync with the dispatcher arm in parse_entry
    match element.name().as_ref() {
        b"link" => {
            if let Some(mut link) = Link::from_attributes(
                element.attributes().flatten(),
                ctx.xml.limits.max_attribute_length,
            ) {
                link.href = ctx.base.resolve_safe(&link.href).into();

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
                            title: None,
                            duration: None,
                        },
                        ctx.xml.limits.max_enclosures,
                    );
                }
                entry
                    .links
                    .try_push_limited(link, ctx.xml.limits.max_links_per_entry);
            }
            if !is_empty {
                skip_to_end(ctx.xml.reader, ctx.xml.buf, b"link")?;
            }
        }
        b"category" => {
            if let Some(tag) = Tag::from_attributes(
                element.attributes().flatten(),
                ctx.xml.limits.max_attribute_length,
            ) {
                entry.tags.try_push_limited(tag, ctx.xml.limits.max_tags);
            }
            if !is_empty {
                skip_to_end(ctx.xml.reader, ctx.xml.buf, b"category")?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Parse entry-level `<author>` and `<contributor>` elements.
#[allow(clippy::unnecessary_wraps)]
fn parse_entry_person(
    ctx: &mut EntryCtx,
    element: &BytesStart<'_>,
    entry: &mut Entry,
    depth: &mut usize,
) -> Result<()> {
    // keep tag list in sync with the dispatcher arm in parse_entry
    match element.name().as_ref() {
        b"author" => {
            if let Ok(person) = parse_person(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth) {
                if entry.author.is_none() {
                    entry.set_author(person.clone());
                }
                entry
                    .authors
                    .try_push_limited(person, ctx.xml.limits.max_authors);
            }
        }
        b"contributor" => {
            if let Ok(person) = parse_person(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth) {
                entry
                    .contributors
                    .try_push_limited(person, ctx.xml.limits.max_contributors);
            }
        }
        _ => {}
    }
    Ok(())
}

/// Parse Dublin Core and Content namespace tags at entry level.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_entry_ns_text(
    ctx: &mut EntryCtx,
    tag: &[u8],
    entry: &mut Entry,
    is_empty: bool,
) -> Result<bool> {
    if let Some(dc_element) = is_dc_tag(tag, ctx.namespaces) {
        let dc_elem = dc_element.to_string();
        if !is_empty {
            let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            ctx.bozo |= had_bozo;
            dublin_core::handle_entry_element(&dc_elem, &text, entry);
        }
        Ok(true)
    } else if let Some(content_element) = is_content_tag(tag) {
        let content_elem = content_element.to_string();
        if !is_empty {
            let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            ctx.bozo |= had_bozo;
            content::handle_entry_element(&content_elem, &text, entry, ctx.lang, ctx.base.base());
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse Media RSS namespace tags at entry level.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_entry_media(
    ctx: &mut EntryCtx,
    tag: &[u8],
    element: &BytesStart<'_>,
    entry: &mut Entry,
    depth: &mut usize,
    is_empty: bool,
) -> Result<bool> {
    let Some(media_element) = is_media_tag(tag, ctx.namespaces) else {
        return Ok(false);
    };
    if parse_entry_media_object(ctx, media_element, element, entry, depth, is_empty)? {
        return Ok(true);
    }
    if parse_entry_media_meta(ctx, media_element, element, entry, *depth, is_empty)? {
        return Ok(true);
    }
    parse_entry_media_text(ctx, media_element, element, entry, is_empty)?;
    Ok(true)
}

/// Parse structured `media:*` elements that contain their own child content:
/// thumbnail, content, group.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_entry_media_object(
    ctx: &mut EntryCtx,
    media_element: &str,
    element: &BytesStart<'_>,
    entry: &mut Entry,
    depth: &mut usize,
    is_empty: bool,
) -> Result<bool> {
    match media_element {
        "thumbnail" => {
            if let Some(thumbnail) = MediaThumbnail::from_attributes(
                element.attributes().flatten(),
                ctx.xml.limits.max_attribute_length,
            ) {
                entry
                    .media_thumbnail
                    .try_push_limited(thumbnail, ctx.xml.limits.max_enclosures);
            }
            if !is_empty {
                skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, *depth)?;
            }
            Ok(true)
        }
        "content" => {
            if let Some(media) = MediaContent::from_attributes(
                element.attributes().flatten(),
                ctx.xml.limits.max_attribute_length,
            ) {
                entry
                    .media_content
                    .try_push_limited(media, ctx.xml.limits.max_enclosures);
            }
            if !is_empty {
                parse_atom_media_content_children(ctx, entry, depth)?;
            }
            Ok(true)
        }
        "group" => {
            if !is_empty {
                // media:group is a transparent container; promote children to entry
                parse_atom_media_group(ctx, entry, depth)?;
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Parse `media:*` metadata elements: credit, copyright, rating.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_entry_media_meta(
    ctx: &mut EntryCtx,
    media_element: &str,
    element: &BytesStart<'_>,
    entry: &mut Entry,
    depth: usize,
    is_empty: bool,
) -> Result<bool> {
    match media_element {
        "credit" => {
            let role = attr_raw(element, b"role");
            let scheme = attr_raw(element, b"scheme");
            if !is_empty {
                let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
                ctx.bozo |= had_bozo;
                if !text.is_empty() {
                    entry.media_credit.try_push_limited(
                        MediaCredit {
                            role,
                            scheme,
                            content: text,
                        },
                        ctx.xml.limits.max_links_per_entry,
                    );
                }
            }
            Ok(true)
        }
        "copyright" => {
            let url = attr_raw(element, b"url");
            if !is_empty {
                skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
            }
            entry.media_copyright = Some(MediaCopyright { url });
            Ok(true)
        }
        "rating" => {
            if !is_empty {
                let scheme =
                    attr_normalized(element, b"scheme", ctx.xml.limits.max_attribute_length);
                let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
                media_rss::handle_entry_rating(scheme.as_deref(), &text, entry);
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Parse `media:*` text elements: description, title, keywords; falls back to the
/// generic Media RSS element handler for anything else.
fn parse_entry_media_text(
    ctx: &mut EntryCtx,
    media_element: &str,
    element: &BytesStart<'_>,
    entry: &mut Entry,
    is_empty: bool,
) -> Result<()> {
    match media_element {
        "description" => {
            let type_attr = attr_raw(element, b"type");
            if !is_empty {
                let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
                ctx.bozo |= had_bozo;
                let is_plain = type_attr.as_deref().is_none_or(|t| t == "plain");
                if is_plain && !text.is_empty() {
                    entry.media_description = Some(text.clone());
                }
                if entry.summary.is_none() && !text.is_empty() {
                    entry.summary = Some(text);
                }
            }
        }
        "title" => {
            let type_attr = attr_raw(element, b"type");
            if !is_empty {
                let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
                ctx.bozo |= had_bozo;
                let is_plain = type_attr.as_deref().is_none_or(|t| t == "plain");
                if is_plain && !text.is_empty() {
                    entry.media_title = Some(text.clone());
                }
                if entry.title.is_none() && !text.is_empty() {
                    entry.title = Some(text);
                }
            }
        }
        "keywords" => {
            if !is_empty {
                let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
                ctx.bozo |= had_bozo;
                media_rss::handle_entry_element("keywords", &text, entry);
            }
        }
        _ => {
            let media_elem = media_element.to_string();
            if !is_empty {
                let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
                ctx.bozo |= had_bozo;
                media_rss::handle_entry_element(&media_elem, &text, entry);
            }
        }
    }
    Ok(())
}

/// Parse Atom Threading Extensions (RFC 4685) at entry level: thr, plus Slash/WFW.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_entry_ns_threading(
    ctx: &mut EntryCtx,
    tag: &[u8],
    element: &BytesStart<'_>,
    entry: &mut Entry,
    depth: usize,
    is_empty: bool,
) -> Result<bool> {
    if let Some(thr_element) = is_thr_tag(tag) {
        match thr_element {
            "in-reply-to" => {
                if let Some(reply) = threading::parse_in_reply_to_from_attrs(
                    element.attributes().flatten(),
                    ctx.xml.limits.max_attribute_length,
                ) {
                    // Shares max_links_per_entry limit; split if needed later
                    entry
                        .in_reply_to
                        .try_push_limited(reply, ctx.xml.limits.max_links_per_entry);
                }
                if !is_empty {
                    skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
                }
            }
            "total" if !is_empty => {
                let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
                ctx.bozo |= had_bozo;
                threading::handle_total(&text, entry);
            }
            _ => {
                if !is_empty {
                    skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
                }
            }
        }
        Ok(true)
    } else if let Some(slash_element) = is_slash_tag(tag) {
        let slash_elem = slash_element.to_string();
        if !is_empty {
            let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            ctx.bozo |= had_bozo;
            slash::handle_slash_entry_element(&slash_elem, &text, entry);
        }
        Ok(true)
    } else if let Some(wfw_element) = is_wfw_tag(tag) {
        let wfw_elem = wfw_element.to_string();
        if !is_empty {
            let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            ctx.bozo |= had_bozo;
            slash::handle_wfw_entry_element(&wfw_elem, &text, entry);
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse iTunes namespace tags at entry level.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_entry_itunes(
    ctx: &mut EntryCtx,
    tag: &[u8],
    element: &BytesStart<'_>,
    entry: &mut Entry,
    depth: usize,
    is_empty: bool,
) -> Result<bool> {
    if is_itunes_tag(tag, b"image", ctx.namespaces) {
        if let Some(url) = extract_href_attr(element, ctx.xml.limits) {
            itunes_entry_meta(entry).image =
                Some(truncate_to_length(&url, ctx.xml.limits.max_attribute_length).into());
        }
        if !is_empty {
            skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"title", ctx.namespaces) && !is_empty {
        let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        ctx.bozo |= had_bozo;
        itunes_entry_meta(entry).title = Some(text);
        Ok(true)
    } else if is_itunes_tag(tag, b"author", ctx.namespaces) && !is_empty {
        let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        ctx.bozo |= had_bozo;
        if entry.author.is_none() {
            entry.set_author(Person::from_name(&text));
            entry
                .authors
                .try_push_limited(Person::from_name(&text), ctx.xml.limits.max_authors);
        }
        itunes_entry_meta(entry).author = Some(text);
        Ok(true)
    } else if is_itunes_tag(tag, b"subtitle", ctx.namespaces) && !is_empty {
        let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        ctx.bozo |= had_bozo;
        itunes_entry_meta(entry).subtitle = Some(text);
        Ok(true)
    } else if is_itunes_tag(tag, b"summary", ctx.namespaces) && !is_empty {
        let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        ctx.bozo |= had_bozo;
        itunes_entry_meta(entry).summary = Some(text);
        Ok(true)
    } else if is_itunes_tag(tag, b"duration", ctx.namespaces) && !is_empty {
        let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        ctx.bozo |= had_bozo;
        itunes_entry_meta(entry).duration = if text.is_empty() { None } else { Some(text) };
        Ok(true)
    } else if is_itunes_tag(tag, b"explicit", ctx.namespaces) && !is_empty {
        let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        ctx.bozo |= had_bozo;
        itunes_entry_meta(entry).explicit = parse_explicit(&text);
        Ok(true)
    } else if is_itunes_tag(tag, b"episode", ctx.namespaces) && !is_empty {
        let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        ctx.bozo |= had_bozo;
        itunes_entry_meta(entry).episode = text.trim().parse().ok();
        Ok(true)
    } else if is_itunes_tag(tag, b"season", ctx.namespaces) && !is_empty {
        let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        ctx.bozo |= had_bozo;
        itunes_entry_meta(entry).season = text.trim().parse().ok();
        Ok(true)
    } else if is_itunes_tag(tag, b"episodeType", ctx.namespaces) && !is_empty {
        let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        ctx.bozo |= had_bozo;
        itunes_entry_meta(entry).episode_type = Some(text);
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse `GeoRSS` and W3C Geo namespace tags at entry level.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_entry_geo(
    ctx: &mut EntryCtx,
    tag: &[u8],
    entry: &mut Entry,
    depth: usize,
    is_empty: bool,
) -> Result<bool> {
    if let Some(georss_element) = is_georss_tag(tag) {
        if !is_empty {
            if georss_element == "where" {
                let (loc, had_bozo, bozo_reason) =
                    parse_georss_where(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
                ctx.bozo |= had_bozo;
                if let Some(reason) = bozo_reason {
                    ctx.bozo_reason.get_or_insert(reason);
                }
                if let Some(loc) = loc {
                    georss::merge_geometry(&mut entry.r#where, loc);
                }
            } else {
                let georss_elem = georss_element.as_bytes().to_vec();
                let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
                ctx.bozo |= had_bozo;
                georss::handle_entry_element(&georss_elem, &text, entry, ctx.xml.limits);
            }
        }
        Ok(true)
    } else if let Some(geo_element) = is_geo_tag(tag) {
        let geo_elem = geo_element.as_bytes().to_vec();
        if !is_empty {
            let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            ctx.bozo |= had_bozo;
            georss::handle_entry_geo_element(&geo_elem, &text, entry);
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Read an attribute value as raw UTF-8 text, with no entity normalization or
/// truncation. Used where the original code did a plain `str::from_utf8` read.
fn attr_raw(element: &BytesStart<'_>, key: &[u8]) -> Option<String> {
    element
        .attributes()
        .flatten()
        .find(|a| a.key.as_ref() == key)
        .and_then(|a| std::str::from_utf8(&a.value).ok().map(str::to_owned))
}

/// Read an attribute value with entity normalization and truncation to `max` bytes.
/// Not interchangeable with [`attr_raw`] — used only where the original code applied
/// `normalized_value` + `truncate_to_length`.
fn attr_normalized(element: &BytesStart<'_>, key: &[u8], max: usize) -> Option<String> {
    element
        .attributes()
        .flatten()
        .find(|a| a.key.as_ref() == key)
        .and_then(|a| a.normalized_value(quick_xml::XmlVersion::Implicit1_0).ok())
        .map(|v| truncate_to_length(&v, max))
}

/// Promote entry.id → entry.link when no explicit `<link>` is present (#273).
///
/// Sets `guidislink = Some(true)` when link is promoted from id, `Some(false)` when
/// an explicit `<link>` was present. `guidislink` remains `None` when no `<id>` exists.
fn promote_entry_id_to_link(entry: &mut Entry) {
    if let Some(id) = entry.id.as_deref() {
        if entry.link.is_none() {
            entry.link = Some(id.to_string());
            entry.guidislink = Some(true);
        } else {
            entry.guidislink = Some(false);
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
///
/// Called from both the feed tier (`FeedCtx` has no `namespaces`) and the entry
/// tier, so it takes a bare `XmlCtx` rather than `EntryCtx`.
fn parse_text_construct(
    xml: &mut XmlCtx,
    e: &quick_xml::events::BytesStart,
    lang: Option<&str>,
    base_ctx: &BaseUrlContext,
) -> Result<TextConstruct> {
    // RFC 4287 §3.1.1 says an absent `type` attribute defaults to "text", but that
    // default is only meaningful as a *display* hint. For sanitization we do not
    // extend the same trust to an unlabeled field that we extend to one the feed
    // author explicitly marked `type="text"`: an absent attribute carries no
    // assertion at all, so it is treated the same as `html`/`xhtml` (fail-closed,
    // #438). Explicit `type="text"` below overrides this and is trusted verbatim.
    let mut content_type = TextType::Html;
    let mut elem_base: Option<String> = None;
    let mut elem_lang: Option<String> = None;

    for attr in e.attributes().flatten() {
        if attr.value.len() > xml.limits.max_attribute_length {
            continue;
        }
        match attr.key.as_ref() {
            b"type" => content_type = TextType::from_type_attr(&bytes_to_string(&attr.value)),
            b"xml:base" | b"base" => {
                if let Ok(v) = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                    && !v.is_empty()
                {
                    elem_base = base_ctx.child_with_base(&v).base().map(ToString::to_string);
                }
            }
            b"xml:lang" | b"lang" => {
                if let Ok(v) = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0) {
                    elem_lang = Some(v.to_string());
                }
            }
            _ => {}
        }
    }

    let value = match content_type {
        TextType::Xhtml => read_xhtml_content_str(xml.reader, xml.buf, xml.limits)?,
        _ => read_text_str(xml.reader, xml.buf, xml.limits)?,
    };

    // Element-level xml:lang overrides parent lang; empty string clears it (XML spec)
    let effective_lang = match &elem_lang {
        Some(l) if l.is_empty() => None,
        Some(l) => Some(l.as_str()),
        None => lang,
    };

    Ok(TextConstruct {
        value,
        content_type,
        language: effective_lang.filter(|s| !s.is_empty()).map(Into::into),
        base: elem_base.or_else(|| base_ctx.base().map(ToString::to_string)),
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

    Ok(Person {
        name,
        email,
        uri,
        avatar: None,
    })
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
///
/// Only called from the entry tier (single call site), so it takes `EntryCtx`
/// directly rather than a bare `XmlCtx` plus separate `lang`/`base` params.
fn parse_content(ctx: &mut EntryCtx, e: &quick_xml::events::BytesStart) -> Result<Content> {
    let mut content_type = None;
    let mut is_xhtml = false;
    let mut src = None;
    let mut elem_base: Option<String> = None;
    let mut elem_lang: Option<String> = None;

    for attr in e.attributes().flatten() {
        if attr.value.len() > ctx.xml.limits.max_attribute_length {
            continue;
        }
        match attr.key.as_ref() {
            b"type" => {
                let normalized = normalize_atom_content_type(&bytes_to_string(&attr.value));
                is_xhtml = normalized == "application/xhtml+xml";
                content_type = Some(normalized.into());
            }
            b"src" => src = Some(bytes_to_string(&attr.value)),
            b"xml:base" | b"base" => {
                if let Ok(v) = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                    && !v.is_empty()
                {
                    elem_base = ctx.base.child_with_base(&v).base().map(ToString::to_string);
                }
            }
            b"xml:lang" | b"lang" => {
                if let Ok(v) = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0) {
                    elem_lang = Some(v.to_string());
                }
            }
            _ => {}
        }
    }

    // RFC 4287 §4.1.3.1: absent `type` defaults to "text"; keep `Content.content_type`
    // populated so downstream sanitization is never bypassed by a missing type.
    let content_type = content_type.or_else(|| Some(MimeType::TEXT_PLAIN.into()));

    // Element-level xml:lang overrides parent lang; empty string clears it (XML spec)
    let effective_lang = match &elem_lang {
        Some(l) if l.is_empty() => None,
        Some(l) => Some(l.as_str()),
        None => ctx.lang,
    };
    let effective_base = elem_base.or_else(|| ctx.base.base().map(ToString::to_string));

    // RFC 4287 §4.1.3.2: when src is present, content is out-of-line; value is empty.
    if src.is_some() {
        skip_to_end(ctx.xml.reader, ctx.xml.buf, b"content")?;
        return Ok(Content {
            value: String::new(),
            content_type,
            language: effective_lang.filter(|s| !s.is_empty()).map(Into::into),
            base: effective_base,
            src,
        });
    }

    let value = if is_xhtml {
        read_xhtml_content_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?
    } else {
        read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?
    };

    Ok(Content {
        value,
        content_type,
        language: effective_lang.filter(|s| !s.is_empty()).map(Into::into),
        base: effective_base,
        src: None,
    })
}

/// Normalize an Atom `content`/`text` construct `type` attribute to a MIME string.
///
/// Case-insensitively maps the RFC 4287 keywords (`text`, `html`, `xhtml`) to their
/// MIME equivalents; any other value (including an already-MIME-spelled type like
/// `text/html`) is lowercased and passed through unchanged.
fn normalize_atom_content_type(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "text" => MimeType::TEXT_PLAIN.to_string(),
        "html" => MimeType::TEXT_HTML.to_string(),
        "xhtml" => "application/xhtml+xml".to_string(),
        other => other.to_string(),
    }
}

/// Parse children of `<media:group>` in an Atom entry.
///
/// `<media:group>` is a transparent container per the Media RSS spec; its children
/// are treated as if they appeared directly under the entry element.
///
/// The `handle_atom_media_group_child` calls below read `ctx.xml.limits` and
/// `ctx.namespaces` as disjoint fields rather than passing `ctx` itself:
/// `e` is bound directly from `read_event_into` (no `.to_owned()`), so it holds
/// a live borrow of `ctx.xml.buf` that a whole-`ctx` borrow would alias.
fn parse_atom_media_group(ctx: &mut EntryCtx, entry: &mut Entry, depth: &mut usize) -> Result<()> {
    loop {
        ctx.xml.buf.clear();
        match ctx.xml.reader.read_event_into(ctx.xml.buf) {
            Ok(Event::Empty(e)) => {
                let tag = e.name().as_ref().to_vec();
                handle_atom_media_group_child(&tag, &e, entry, ctx.xml.limits, ctx.namespaces);
            }
            Ok(Event::Start(e)) => {
                let tag = e.name().as_ref().to_vec();
                if is_media_tag(&tag, ctx.namespaces) == Some("title") {
                    let type_attr = e
                        .attributes()
                        .flatten()
                        .find(|a| a.key.as_ref() == b"type")
                        .and_then(|a| std::str::from_utf8(&a.value).ok().map(str::to_owned));
                    let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
                    ctx.bozo |= had_bozo;
                    let is_plain = type_attr.as_deref().is_none_or(|t| t == "plain");
                    if is_plain && !text.is_empty() {
                        entry.media_title = Some(text.clone());
                    }
                    if entry.title.is_none() && !text.is_empty() {
                        entry.title = Some(text);
                    }
                } else if is_media_tag(&tag, ctx.namespaces) == Some("description") {
                    let type_attr = e
                        .attributes()
                        .flatten()
                        .find(|a| a.key.as_ref() == b"type")
                        .and_then(|a| std::str::from_utf8(&a.value).ok().map(str::to_owned));
                    let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
                    ctx.bozo |= had_bozo;
                    let is_plain = type_attr.as_deref().is_none_or(|t| t == "plain");
                    if is_plain && !text.is_empty() {
                        entry.media_description = Some(text.clone());
                    }
                    if entry.summary.is_none() && !text.is_empty() {
                        entry.summary = Some(text);
                    }
                } else {
                    handle_atom_media_group_child(&tag, &e, entry, ctx.xml.limits, ctx.namespaces);
                    *depth += 1;
                    skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, *depth)?;
                    *depth = depth.saturating_sub(1);
                }
            }
            Ok(Event::End(_) | Event::Eof) => break,
            Err(_) => {
                ctx.bozo = true;
                break;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Parse children of `<media:content>`, collecting nested `media:thumbnail` elements.
///
/// Python feedparser collects all `media:thumbnail` elements into `entry.media_thumbnail`
/// regardless of whether they are nested inside `media:content`.
fn parse_atom_media_content_children(
    ctx: &mut EntryCtx,
    entry: &mut Entry,
    depth: &mut usize,
) -> Result<()> {
    loop {
        ctx.xml.buf.clear();
        match ctx.xml.reader.read_event_into(ctx.xml.buf) {
            Ok(Event::Empty(e)) => {
                let tag = e.name().as_ref().to_vec();
                if is_media_tag(&tag, ctx.namespaces) == Some("thumbnail") {
                    let thumbnail = MediaThumbnail::from_attributes(
                        e.attributes().flatten(),
                        ctx.xml.limits.max_attribute_length,
                    );
                    if let Some(thumbnail) = thumbnail {
                        entry
                            .media_thumbnail
                            .try_push_limited(thumbnail, ctx.xml.limits.max_enclosures);
                    }
                }
            }
            Ok(Event::Start(e)) => {
                let tag = e.name().as_ref().to_vec();
                if is_media_tag(&tag, ctx.namespaces) == Some("thumbnail") {
                    let thumbnail = MediaThumbnail::from_attributes(
                        e.attributes().flatten(),
                        ctx.xml.limits.max_attribute_length,
                    );
                    if let Some(thumbnail) = thumbnail {
                        entry
                            .media_thumbnail
                            .try_push_limited(thumbnail, ctx.xml.limits.max_enclosures);
                    }
                }
                *depth += 1;
                skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, *depth)?;
                *depth = depth.saturating_sub(1);
            }
            Ok(Event::End(_) | Event::Eof) => break,
            Err(_) => {
                ctx.bozo = true;
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
    namespaces: &HashMap<String, String>,
) {
    let Some(child_elem) = is_media_tag(tag, namespaces) else {
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
    base_ctx: &BaseUrlContext,
) -> Option<Content> {
    let mut content_type = None;
    let mut src = None;
    let mut elem_base: Option<String> = None;
    let mut elem_lang: Option<String> = None;

    for attr in e.attributes().flatten() {
        if attr.value.len() > limits.max_attribute_length {
            continue;
        }
        match attr.key.as_ref() {
            b"type" => {
                content_type =
                    Some(normalize_atom_content_type(&bytes_to_string(&attr.value)).into());
            }
            b"src" => src = Some(bytes_to_string(&attr.value)),
            b"xml:base" | b"base" => {
                if let Ok(v) = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                    && !v.is_empty()
                {
                    elem_base = base_ctx.child_with_base(&v).base().map(ToString::to_string);
                }
            }
            b"xml:lang" | b"lang" => {
                if let Ok(v) = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0) {
                    elem_lang = Some(v.to_string());
                }
            }
            _ => {}
        }
    }

    // RFC 4287 §4.1.3.1: absent `type` defaults to "text".
    let content_type = content_type.or_else(|| Some(MimeType::TEXT_PLAIN.into()));

    let effective_lang = match &elem_lang {
        Some(l) if l.is_empty() => None,
        Some(l) => Some(l.as_str()),
        None => lang,
    };
    let effective_base = elem_base.or_else(|| base_ctx.base().map(ToString::to_string));

    src.map(|src_val| Content {
        value: String::new(),
        content_type,
        language: effective_lang.filter(|s| !s.is_empty()).map(Into::into),
        base: effective_base,
        src: Some(src_val),
    })
}

/// Accumulated loop-local state for [`parse_atom_source`], moved into a struct to
/// keep the parse loop itself under the function-length budget.
#[derive(Default)]
struct SourceFields {
    title: Option<String>,
    link: Option<String>,
    first_link_href: Option<String>,
    id: Option<String>,
    links: Vec<Link>,
    updated: Option<DateTime<Utc>>,
    updated_str: Option<String>,
    rights: Option<String>,
    has_explicit_link: bool,
    author: Option<String>,
}

impl SourceFields {
    /// Handle a `<link>` child element: parse its attributes and record it.
    ///
    /// `has_explicit_link` is only set inside the `Some(lnk)` branch — a `<link>`
    /// whose attributes fail to parse must not set it, since it feeds `guidislink`
    /// in [`SourceFields::finish`].
    fn push_link(&mut self, element: &BytesStart<'_>, limits: &ParserLimits) {
        if let Some(lnk) =
            Link::from_attributes(element.attributes().flatten(), limits.max_attribute_length)
        {
            // Track first alternate rel for link; fall back to first link seen.
            if lnk.rel.as_deref() == Some("alternate") && self.link.is_none() {
                self.link = Some(lnk.href.to_string());
            }
            if self.first_link_href.is_none() {
                self.first_link_href = Some(lnk.href.to_string());
            }
            self.has_explicit_link = true;
            self.links.push(lnk);
        }
    }

    /// Resolve the link fallback and `guidislink`, then build the final [`Source`].
    fn finish(mut self) -> Source {
        // Fall back to first link of any rel if no alternate was found
        if self.link.is_none() {
            self.link = self.first_link_href;
        }

        // Compute guidislink per Python feedparser semantics:
        // - None when no <id> present
        // - Some(true) when <id> looks like a URL and no explicit <link> present
        // - Some(false) otherwise
        let guidislink = self.id.as_deref().map(|id_val| {
            let id_is_url = id_val.starts_with("http://")
                || id_val.starts_with("https://")
                || id_val.starts_with("ftp://");
            id_is_url && !self.has_explicit_link
        });

        // When guidislink is true, populate link from the id value (matching Python feedparser)
        if guidislink == Some(true) {
            self.link.clone_from(&self.id);
        }

        Source {
            title: self.title,
            href: None,
            link: self.link,
            author: self.author,
            id: self.id,
            links: self.links,
            updated: self.updated,
            updated_str: self.updated_str,
            rights: self.rights,
            guidislink,
        }
    }
}

/// Parse <source> element (renamed to avoid confusion with RSS source)
fn parse_atom_source(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
    depth: &mut usize,
) -> Result<Source> {
    let mut fields = SourceFields::default();

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
                        fields.title = Some(read_text_str(reader, buf, limits)?);
                    }
                    b"link" => {
                        fields.push_link(&element, limits);
                        if !is_empty {
                            skip_to_end(reader, buf, b"link")?;
                        }
                    }
                    b"id" if !is_empty => fields.id = Some(read_text_str(reader, buf, limits)?),
                    b"updated" | b"modified" if !is_empty => {
                        let text = read_text_str(reader, buf, limits)?;
                        fields.updated = parse_date(&text);
                        fields.updated_str = Some(text);
                    }
                    b"rights" if !is_empty => {
                        fields.rights = Some(read_text_str(reader, buf, limits)?);
                    }
                    b"author" if !is_empty => {
                        if let Ok(person) = parse_person(reader, buf, limits, depth) {
                            fields.author = person.flat_string().map(|s| s.to_string());
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

    Ok(fields.finish())
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
                if tag_name.as_ref() == b"name" {
                    owner.name = Some(read_text_str(reader, buf, limits)?);
                } else if tag_name.as_ref() == b"email" {
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
    ctx: &mut FeedCtx,
    element: &quick_xml::events::BytesStart<'_>,
    feed: &mut ParsedFeed,
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
            skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, 0)?;
        }
        return Ok(());
    }

    let mut subcategory: Option<String> = None;

    if !is_empty {
        loop {
            ctx.xml.buf.clear();
            match ctx.xml.reader.read_event_into(ctx.xml.buf) {
                Ok(Event::Empty(e))
                    if is_itunes_tag(e.name().as_ref(), b"category", &feed.namespaces) =>
                {
                    subcategory = e
                        .attributes()
                        .flatten()
                        .find(|a| a.key.as_ref() == b"text")
                        .and_then(|a| String::from_utf8(a.value.into_owned()).ok());
                }
                Ok(Event::Start(e))
                    if is_itunes_tag(e.name().as_ref(), b"category", &feed.namespaces) =>
                {
                    subcategory = e
                        .attributes()
                        .flatten()
                        .find(|a| a.key.as_ref() == b"text")
                        .and_then(|a| String::from_utf8(a.value.into_owned()).ok());
                    skip_to_end(ctx.xml.reader, ctx.xml.buf, b"category")?;
                }
                Ok(Event::End(_) | Event::Eof) => break,
                Err(e) => return Err(e.into()),
                _ => {}
            }
        }
    }

    feed.feed.tags.try_push_limited(
        Tag {
            term: text.as_str().into(),
            scheme: Some("http://www.itunes.com/".into()),
            label: None,
        },
        ctx.xml.limits.max_tags,
    );
    if let Some(ref sub) = subcategory {
        feed.feed.tags.try_push_limited(
            Tag {
                term: sub.as_str().into(),
                scheme: Some("http://www.itunes.com/".into()),
                label: None,
            },
            ctx.xml.limits.max_tags,
        );
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

    // Regression tests for #463: a malformed entity in a feed-level field must
    // not abort the whole parse — sibling <entry>s must still be recovered.

    #[test]
    fn test_feed_title_malformed_entity_recovers_entries() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <title>Fish & Chips</title>
            <entry><title>Entry 1</title><id>entry1</id></entry>
            <entry><title>Entry 2</title><id>entry2</id></entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(feed.bozo);
        assert!(feed.bozo_exception.is_some());
        assert_eq!(feed.entries.len(), 2);
        assert_eq!(feed.entries[0].title.as_deref(), Some("Entry 1"));
        assert_eq!(feed.entries[1].title.as_deref(), Some("Entry 2"));
    }

    #[test]
    fn test_feed_category_malformed_entity_recovers_entries() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <title>Test</title>
            <category term="x">Fish & Chips</category>
            <entry><title>Entry 1</title><id>entry1</id></entry>
            <entry><title>Entry 2</title><id>entry2</id></entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(feed.bozo);
        assert!(feed.bozo_exception.is_some());
        assert_eq!(feed.entries.len(), 2);
    }

    #[test]
    fn test_entry_malformed_entity_does_not_leak_into_feed_fields() {
        // #463 S3: a malformed entity inside a normal <entry> (not an over-limit
        // one) must not leak that entry's own <title> into the feed-level fields —
        // the pre-existing defect the issue used entry-level recovery as the
        // reference-correct behavior for, but which was itself still broken via
        // this path.
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <title>REAL FEED TITLE</title>
            <entry><summary>E & S</summary><title>ENTRY TITLE</title><id>entry1</id></entry>
            <entry><title>Entry Two OK</title><id>entry2</id></entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(feed.bozo);
        assert_eq!(feed.feed.title.as_deref(), Some("REAL FEED TITLE"));
        assert_eq!(feed.entries.len(), 1);
        assert_eq!(feed.entries[0].title.as_deref(), Some("Entry Two OK"));
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
        assert_eq!(itunes.complete.as_deref(), Some("Yes"));
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
    fn test_atom_entry_guidislink_true_when_id_promoted_to_link() {
        // #285: when id is promoted to link, guidislink must be Some(true)
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
        assert_eq!(feed.entries[0].guidislink, Some(true));
        assert_eq!(feed.entries[0].link.as_deref(), Some("urn:uuid:1234"));
    }

    #[test]
    fn test_atom_entry_guidislink_false_when_explicit_link_present() {
        // #285: when explicit <link> is present, guidislink must be Some(false)
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <title>Test</title>
            <entry>
                <title>Entry</title>
                <id>urn:uuid:1234</id>
                <link href="https://example.com/entry"/>
                <updated>2024-01-01T00:00:00Z</updated>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.entries[0].guidislink, Some(false));
        assert_eq!(
            feed.entries[0].link.as_deref(),
            Some("https://example.com/entry")
        );
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

    #[test]
    fn test_atom_entry_dc_creator_fallback_author() {
        // #278: dc:creator must be used as fallback for entry.author
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:dc="http://purl.org/dc/elements/1.1/">
            <title>Test</title>
            <entry>
                <title>Entry</title>
                <id>urn:uuid:1234</id>
                <updated>2024-01-01T00:00:00Z</updated>
                <dc:creator>Jane Doe</dc:creator>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.entries[0].author.as_deref(), Some("Jane Doe"));
    }

    #[test]
    fn test_atom_entry_author_takes_precedence_over_dc_creator() {
        // #278: explicit <author> must take precedence over dc:creator
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:dc="http://purl.org/dc/elements/1.1/">
            <title>Test</title>
            <entry>
                <title>Entry</title>
                <id>urn:uuid:1234</id>
                <updated>2024-01-01T00:00:00Z</updated>
                <author><name>John Smith</name></author>
                <dc:creator>Jane Doe</dc:creator>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.entries[0].author.as_deref(), Some("John Smith"));
    }

    // =========================================================================
    // Regression tests for #257, #281 (Atom-specific)
    // =========================================================================

    // TC-281-3: Atom itunes:complete 'Yes' returns raw string
    #[test]
    fn test_tc_281_3_atom_itunes_complete_raw_string() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <title>P</title>
            <itunes:complete>Yes</itunes:complete>
        </feed>"#;
        let feed = parse_atom10(xml).unwrap();
        assert_eq!(
            feed.feed.itunes.as_ref().unwrap().complete.as_deref(),
            Some("Yes")
        );
    }

    // TC-257-9: Atom entry itunes:subtitle promotes to entry.subtitle
    #[test]
    fn test_tc_257_9_atom_entry_itunes_subtitle_promotes() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <title>Feed</title>
            <entry>
                <title>E</title>
                <id>urn:test:1</id>
                <updated>2024-01-01T00:00:00Z</updated>
                <itunes:subtitle>Atom episode subtitle</itunes:subtitle>
            </entry>
        </feed>"#;
        let feed = parse_atom10(xml).unwrap();
        let entry = &feed.entries[0];
        assert_eq!(entry.subtitle.as_deref(), Some("Atom episode subtitle"));
        assert_eq!(
            entry.itunes.as_ref().unwrap().subtitle.as_deref(),
            Some("Atom episode subtitle")
        );
    }

    // TC-257-10: Atom feed: itunes:subtitle overrides <subtitle> regardless of order
    #[test]
    fn test_tc_257_10_atom_itunes_subtitle_overrides_subtitle() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <title>Feed</title>
            <subtitle>Atom subtitle</subtitle>
            <itunes:subtitle>iTunes subtitle</itunes:subtitle>
        </feed>"#;
        let feed = parse_atom10(xml).unwrap();
        assert_eq!(
            feed.feed.subtitle.as_deref(),
            Some("iTunes subtitle"),
            "Post-processing must override standard <subtitle> with itunes:subtitle"
        );
    }

    // TC-257-11: Atom feed: itunes:subtitle before <subtitle> — post-processing still wins
    #[test]
    fn test_tc_257_11_atom_itunes_subtitle_before_subtitle_post_processing_wins() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <title>Feed</title>
            <itunes:subtitle>iTunes subtitle</itunes:subtitle>
            <subtitle>Atom subtitle</subtitle>
        </feed>"#;
        let feed = parse_atom10(xml).unwrap();
        assert_eq!(
            feed.feed.subtitle.as_deref(),
            Some("iTunes subtitle"),
            "Reversed order — post-processing guarantees iTunes wins"
        );
    }

    // TC-257: Atom itunes:summary populates feed.summary
    #[test]
    fn test_tc_257_atom_itunes_summary_populates_feed_summary() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <title>Feed</title>
            <subtitle>Atom subtitle</subtitle>
            <itunes:summary>Podcast summary</itunes:summary>
        </feed>"#;
        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.feed.summary.as_deref(), Some("Podcast summary"));
        assert_eq!(feed.feed.subtitle.as_deref(), Some("Atom subtitle"));
    }

    // TC-257-12A: Atom — empty itunes:subtitle does NOT override valid <subtitle>
    #[test]
    fn test_tc_257_12a_atom_empty_itunes_subtitle_no_override() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <title>Feed</title>
            <subtitle>Valid Atom subtitle</subtitle>
            <itunes:subtitle></itunes:subtitle>
        </feed>"#;
        let feed = parse_atom10(xml).unwrap();
        assert_eq!(
            feed.feed.subtitle.as_deref(),
            Some("Valid Atom subtitle"),
            "Empty itunes:subtitle must not override valid Atom <subtitle>"
        );
    }

    // Bug #301: Atom 0.3 <created> element maps to entry.created / entry.created_str
    #[test]
    fn test_atom03_created_element() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://purl.org/atom/ns#">
            <title>Test</title>
            <modified>2025-01-01T00:00:00Z</modified>
            <entry>
                <title>Entry</title>
                <id>urn:test:1</id>
                <issued>2024-12-01T00:00:00Z</issued>
                <modified>2024-12-02T00:00:00Z</modified>
                <created>2024-11-30T00:00:00Z</created>
            </entry>
        </feed>"#;
        let feed = parse_atom10(xml).unwrap();
        let entry = &feed.entries[0];
        assert!(
            entry.created.is_some(),
            "entry.created must be set from <created>"
        );
        assert_eq!(
            entry.created_str.as_deref(),
            Some("2024-11-30T00:00:00Z"),
            "entry.created_str must preserve raw date string"
        );
        // 2024-11-30 00:00:00 UTC
        assert_eq!(entry.created.unwrap().timestamp(), 1_732_924_800);
    }

    // TC-257-12B: Atom — itunes:subtitle only present (no <subtitle>) sets feed.subtitle
    #[test]
    fn test_tc_257_12b_atom_itunes_subtitle_only_no_regression() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <title>Feed</title>
            <itunes:subtitle>Only iTunes subtitle</itunes:subtitle>
        </feed>"#;
        let feed = parse_atom10(xml).unwrap();
        assert_eq!(
            feed.feed.subtitle.as_deref(),
            Some("Only iTunes subtitle"),
            "itunes:subtitle must set feed.subtitle when no native <subtitle> present"
        );
    }

    #[test]
    fn test_atom03_tagline_maps_to_subtitle() {
        let xml = br#"<?xml version="1.0" encoding="utf-8"?>
        <feed xmlns="http://purl.org/atom/ns#" version="0.3">
            <title>Test Feed</title>
            <tagline>My tagline</tagline>
            <modified>2004-01-01T00:00:00Z</modified>
        </feed>"#;
        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.version, FeedVersion::Atom03);
        assert_eq!(feed.feed.subtitle.as_deref(), Some("My tagline"));
    }

    #[test]
    fn test_atom03_copyright_maps_to_rights() {
        let xml = br#"<?xml version="1.0" encoding="utf-8"?>
        <feed xmlns="http://purl.org/atom/ns#" version="0.3">
            <title>Test Feed</title>
            <copyright>CC BY 4.0</copyright>
            <modified>2004-01-01T00:00:00Z</modified>
        </feed>"#;
        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.version, FeedVersion::Atom03);
        assert_eq!(feed.feed.rights.as_deref(), Some("CC BY 4.0"));
    }

    #[test]
    fn test_atom_itunes_category_maps_to_tags() {
        let xml = br#"<?xml version="1.0" encoding="utf-8"?>
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <title>Podcast Feed</title>
            <itunes:category text="Technology">
                <itunes:category text="Software How-To"/>
            </itunes:category>
        </feed>"#;
        let feed = parse_atom10(xml).unwrap();
        let tech_tag = feed
            .feed
            .tags
            .iter()
            .find(|t| t.term == "Technology")
            .expect("Technology category must appear in tags");
        assert_eq!(
            tech_tag.scheme.as_deref(),
            Some("http://www.itunes.com/"),
            "itunes:category scheme must be http://www.itunes.com/"
        );
        assert!(
            tech_tag.label.is_none(),
            "itunes:category label must be None"
        );
        let sub_tag = feed
            .feed
            .tags
            .iter()
            .find(|t| t.term == "Software How-To")
            .expect("Software How-To subcategory must appear in tags");
        assert_eq!(sub_tag.scheme.as_deref(), Some("http://www.itunes.com/"));
        assert!(sub_tag.label.is_none());
        let itunes = feed.feed.itunes.as_ref().unwrap();
        assert_eq!(itunes.categories[0].text, "Technology");
        assert_eq!(
            itunes.categories[0].subcategory.as_deref(),
            Some("Software How-To")
        );
    }

    #[test]
    fn test_atom_base_propagates_to_text_constructs() {
        let xml = br#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://example.com/" xml:lang="en">
  <title>Test Feed</title>
  <entry>
    <id>1</id>
    <title>Entry Title</title>
    <summary>Entry Summary</summary>
    <updated>2024-01-01T00:00:00Z</updated>
  </entry>
</feed>"#;
        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        let entry = &feed.entries[0];
        assert_eq!(
            entry.title_detail.as_ref().unwrap().language.as_deref(),
            Some("en")
        );
        assert_eq!(
            entry.title_detail.as_ref().unwrap().base.as_deref(),
            Some("http://example.com/")
        );
        assert_eq!(
            entry.summary_detail.as_ref().unwrap().language.as_deref(),
            Some("en")
        );
        assert_eq!(
            entry.summary_detail.as_ref().unwrap().base.as_deref(),
            Some("http://example.com/")
        );
    }

    #[test]
    fn test_atom_entry_level_lang_overrides_feed_lang() {
        let xml = br#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom" xml:lang="en">
  <title>Feed</title>
  <entry xml:lang="fr">
    <id>1</id>
    <title>Titre</title>
    <updated>2024-01-01T00:00:00Z</updated>
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
            Some("fr")
        );
    }

    #[test]
    fn test_atom_feed_level_lang_on_title() {
        let xml = br#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom" xml:lang="de" xml:base="http://example.com/">
  <title>Feed Titel</title>
  <subtitle>Untertitel</subtitle>
</feed>"#;
        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        assert_eq!(
            feed.feed.title_detail.as_ref().unwrap().language.as_deref(),
            Some("de")
        );
        assert_eq!(
            feed.feed.title_detail.as_ref().unwrap().base.as_deref(),
            Some("http://example.com/")
        );
    }

    #[test]
    fn test_atom_no_lang_yields_none() {
        let xml = br#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>No Lang Feed</title>
  <entry>
    <id>1</id>
    <title>Entry</title>
    <updated>2024-01-01T00:00:00Z</updated>
  </entry>
</feed>"#;
        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        assert!(feed.feed.title_detail.as_ref().unwrap().language.is_none());
        assert!(feed.feed.title_detail.as_ref().unwrap().base.is_none());
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
    fn test_atom_element_level_empty_lang_clears_inherited() {
        // xml:lang="" on a text construct clears the parent-inherited language (XML spec).
        let xml = br#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom" xml:lang="en">
  <title xml:lang="">Cleared Lang Title</title>
  <entry>
    <id>1</id>
    <title xml:lang="">Cleared Entry Title</title>
    <summary>Inherited Summary</summary>
    <updated>2024-01-01T00:00:00Z</updated>
  </entry>
</feed>"#;
        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        // Element-level xml:lang="" must clear the inherited "en"
        assert!(
            feed.feed.title_detail.as_ref().unwrap().language.is_none(),
            "feed title with xml:lang=\"\" should have None language"
        );
        assert!(
            feed.entries[0]
                .title_detail
                .as_ref()
                .unwrap()
                .language
                .is_none(),
            "entry title with xml:lang=\"\" should have None language"
        );
        // summary has no xml:lang override -> inherits feed-level "en"
        assert_eq!(
            feed.entries[0]
                .summary_detail
                .as_ref()
                .unwrap()
                .language
                .as_deref(),
            Some("en"),
            "entry summary without xml:lang override should inherit feed lang"
        );
    }

    #[test]
    fn test_atom_entry_level_empty_lang_clears_inherited() {
        // xml:lang="" on <entry> clears feed-level inherited lang for all its children.
        let xml = br#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom" xml:lang="de">
  <title>Feed</title>
  <entry xml:lang="">
    <id>1</id>
    <title>Entry</title>
    <updated>2024-01-01T00:00:00Z</updated>
  </entry>
</feed>"#;
        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        // Entry with xml:lang="" should not inherit "de" from feed
        assert!(
            feed.entries[0]
                .title_detail
                .as_ref()
                .unwrap()
                .language
                .is_none(),
            "entry title under xml:lang=\"\" entry should have None language"
        );
    }

    #[test]
    fn test_atom_author_inheritance_from_feed() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <title>Test</title>
            <author><name>Feed Author</name><email>feed@example.com</email></author>
            <entry>
                <title>Entry without author</title>
                <id>entry1</id>
                <updated>2024-01-01T00:00:00Z</updated>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        assert_eq!(feed.entries.len(), 1);
        assert_eq!(feed.entries[0].authors.len(), 1);
        assert_eq!(
            feed.entries[0].authors[0].name.as_deref(),
            Some("Feed Author")
        );
        // author is flat_string() which may include email
        assert!(
            feed.entries[0]
                .author
                .as_deref()
                .unwrap_or("")
                .contains("Feed Author")
        );
    }

    #[test]
    fn test_atom_entry_author_takes_precedence() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <title>Test</title>
            <author><name>Feed Author</name></author>
            <entry>
                <title>Entry with own author</title>
                <id>entry1</id>
                <updated>2024-01-01T00:00:00Z</updated>
                <author><name>Entry Author</name></author>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        assert_eq!(feed.entries[0].authors.len(), 1);
        assert_eq!(
            feed.entries[0].authors[0].name.as_deref(),
            Some("Entry Author")
        );
        assert_eq!(feed.entries[0].author.as_deref(), Some("Entry Author"));
    }

    #[test]
    fn test_atom_author_inheritance_mixed_entries() {
        let xml = br#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
            <title>Test</title>
            <author><name>Feed Author</name></author>
            <entry>
                <title>Entry with own author</title>
                <id>entry1</id>
                <updated>2024-01-01T00:00:00Z</updated>
                <author><name>Entry Author</name></author>
            </entry>
            <entry>
                <title>Entry without author</title>
                <id>entry2</id>
                <updated>2024-01-02T00:00:00Z</updated>
            </entry>
        </feed>"#;

        let feed = parse_atom10(xml).unwrap();
        assert!(!feed.bozo);
        assert_eq!(feed.entries[0].author.as_deref(), Some("Entry Author"));
        assert_eq!(feed.entries[1].author.as_deref(), Some("Feed Author"));
    }

    #[test]
    fn test_atom_null_bytes_stripped_from_title() {
        let xml = b"<?xml version=\"1.0\"?>\
        <feed xmlns=\"http://www.w3.org/2005/Atom\">\
            <title>Hello\x00World</title>\
            <entry>\
                <title>Entry\x00Title</title>\
                <id>e1</id>\
                <updated>2024-01-01T00:00:00Z</updated>\
            </entry>\
        </feed>";

        let feed = parse_atom10(xml).unwrap();
        assert_eq!(feed.feed.title.as_deref(), Some("HelloWorld"));
        assert_eq!(feed.entries[0].title.as_deref(), Some("EntryTitle"));
    }
}
