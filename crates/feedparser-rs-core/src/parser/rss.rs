//! RSS 2.0 parser implementation

use std::collections::HashMap;

use crate::{
    ParserLimits,
    error::{FeedError, Result},
    namespace::{content, dublin_core, georss, media_rss, slash, syndication, threading},
    types::{
        Enclosure, Entry, FeedMeta, FeedVersion, Image, ItunesCategory, ItunesFeedMeta,
        ItunesOwner, Link, MediaCopyright, MediaCredit, MediaThumbnail, ParsedFeed, Person,
        PodcastAlternateEnclosure, PodcastAlternateEnclosureSource, PodcastChapters, PodcastChat,
        PodcastEntryMeta, PodcastFollow, PodcastFunding, PodcastIntegrity, PodcastLocation,
        PodcastMeta, PodcastPerson, PodcastRemoteItem, PodcastSocialInteract, PodcastSoundbite,
        PodcastTranscript, PodcastTxt, PodcastUpdateFrequency, PodcastValue, PodcastValueRecipient,
        PodcastValueTimeSplit, Source, Tag, TextConstruct, TextType, parse_explicit,
    },
    util::{
        base_url::BaseUrlContext,
        parse_date,
        text::{parse_rss_person, truncate_to_length},
    },
};
use quick_xml::{Reader, events::Event};

use super::common::{
    EVENT_BUFFER_CAPACITY, LimitedCollectionExt, build_media_content, build_media_thumbnail,
    check_depth, extract_namespaces, extract_xml_lang, find_attribute, init_feed, is_content_tag,
    is_dc_tag, is_geo_tag, is_georss_tag, is_itunes_tag, is_media_tag, is_slash_tag, is_syn_tag,
    is_thr_tag, is_wfw_tag, itunes_entry_meta, itunes_feed_meta, parse_georss_where,
    podcast_feed_meta, read_text, read_text_str, skip_element, skip_to_end_qualified,
    text_construct_from_content,
};
use super::context::{EntryCtx, XmlCtx};

/// Channel-tier parse context: XML plumbing plus the xml:base/xml:lang state
/// accumulated while parsing `<channel>` children.
///
/// `base` is `&mut` (unlike the read-only `FeedCtx`/`RdfCtx` outer tiers) because
/// `parse_channel_text` mutates it in place when `<link>` establishes the base URL.
struct ChannelCtx<'r, 'd, 'p> {
    /// XML event-loop plumbing (reader, buffer, limits).
    xml: XmlCtx<'r, 'd>,
    /// xml:base resolution context; mutated when `<link>` sets the channel base URL.
    base: &'p mut BaseUrlContext,
    /// xml:lang inherited by elements that don't declare their own.
    lang: Option<&'p str>,
}

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
                if let Ok(v) = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0) {
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
///
/// Relative URI resolution is always enabled; use [`parse_rss20_with_options`] to
/// control it.
pub fn parse_rss20_with_limits(data: &[u8], limits: ParserLimits) -> Result<ParsedFeed> {
    parse_rss20_with_options(data, limits, true)
}

/// Parse RSS 2.0 with custom parser limits and relative URI resolution control
pub fn parse_rss20_with_options(
    data: &[u8],
    limits: ParserLimits,
    resolve_relative_uris: bool,
) -> Result<ParsedFeed> {
    limits
        .check_feed_size(data.len())
        .map_err(|e| FeedError::InvalidFormat(e.to_string()))?;

    let mut reader = Reader::from_reader(data);

    let mut feed = init_feed(FeedVersion::Rss20, limits.max_entries);
    let mut buf = Vec::with_capacity(EVENT_BUFFER_CAPACITY);
    let mut depth: usize = 1;
    let mut base_ctx = BaseUrlContext::new().with_resolve(resolve_relative_uris);
    let mut found_channel = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.local_name().as_ref() == b"rss" => {
                extract_namespaces(&e, &mut feed, &limits);
            }
            Ok(Event::Start(e)) if e.local_name().as_ref() == b"channel" => {
                // Some feeds declare xmlns on <channel> rather than <rss>
                extract_namespaces(&e, &mut feed, &limits);
                let channel_lang =
                    extract_xml_lang(&e, limits.max_attribute_length).filter(|s| !s.is_empty());
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

/// Apply iTunes promotion rules to feed-level fields.
///
/// Called after all XML elements in a channel/feed have been parsed, so promotion
/// is order-independent regardless of where itunes: elements appear in the document.
/// Empty/whitespace-only iTunes values are filtered and do not override valid data.
fn apply_itunes_feed_promotions(feed: &mut FeedMeta) {
    // Clone to avoid simultaneous immutable + mutable borrow on `feed`.
    let subtitle = feed.itunes.as_ref().and_then(|it| it.subtitle.clone());
    let summary = feed.itunes.as_ref().and_then(|it| it.summary.clone());
    let itunes_image = feed.itunes.as_ref().and_then(|it| it.image.clone());
    let itunes_author = feed.itunes.as_ref().and_then(|it| it.author.clone());
    let owner_name = feed
        .itunes
        .as_ref()
        .and_then(|it| it.owner.as_ref())
        .and_then(|o| o.name.clone());
    let owner_email = feed
        .itunes
        .as_ref()
        .and_then(|it| it.owner.as_ref())
        .and_then(|o| o.email.clone());

    if let Some(ref s) = subtitle
        && !s.trim().is_empty()
    {
        feed.set_subtitle(TextConstruct::html(s));
    }
    if let Some(ref s) = summary
        && !s.trim().is_empty()
    {
        feed.set_summary(TextConstruct::html(s));
    }

    // itunes:image href always wins over RSS <image> (#287).
    if let Some(ref url) = itunes_image
        && !url.trim().is_empty()
    {
        feed.image = Some(Image {
            url: url.as_str().into(),
            title: None,
            link: None,
            width: None,
            height: None,
            description: None,
        });
    }

    // Author promotion priority (#297): managingEditor > itunes:owner.name > itunes:author.
    // managingEditor is set inline as a native RSS field before this function runs.
    if feed.author.is_none() {
        if let Some(ref name) = owner_name
            && !name.trim().is_empty()
        {
            let person = Person {
                name: Some(name.as_str().into()),
                email: owner_email.as_deref().map(crate::types::Email::new),
                uri: None,
                avatar: None,
            };
            // #317: feed.author must be name-only (no email in parentheses).
            // author_detail still carries the full person (name + email).
            feed.author.clone_from(&person.name);
            feed.author_detail = Some(person.clone());
            feed.authors.try_push_limited(person, usize::MAX);
        } else if let Some(ref name) = itunes_author
            && !name.trim().is_empty()
        {
            let person = Person::from_name(name);
            feed.set_author(person.clone());
            feed.authors.try_push_limited(person, usize::MAX);
        }
    }
}

/// Apply iTunes subtitle/summary promotions to entry-level fields.
///
/// Called after all XML elements in an item/entry have been parsed. In RSS, entry.subtitle
/// is iTunes-only (no native RSS <subtitle> for entries), so this always sets a previously-None
/// field when itunes:subtitle is present. Entry.summary may already be set from <description>.
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
    let mut ctx = ChannelCtx {
        xml: XmlCtx {
            reader,
            buf: &mut buf,
            limits,
        },
        base: base_ctx,
        lang: channel_lang,
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
                let item_lang = extract_xml_lang(e, ctx.xml.limits.max_attribute_length)
                    .filter(|s| !s.is_empty());

                dispatch_channel_tag(
                    &mut ctx,
                    &tag,
                    &attrs,
                    item_lang.as_deref(),
                    feed,
                    depth,
                    is_empty,
                );
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
        ctx.xml.buf.clear();
    }

    // Post-process: iTunes subtitle/summary always win over <description> (order-independent)
    apply_itunes_feed_promotions(&mut feed.feed);

    Ok(())
}

/// Record a channel/feed-level field parse failure as bozo instead of propagating it.
///
/// Invariant: channel/feed-level field parse failures must not abort sibling
/// item/entry recovery — mirrors the catch-and-continue already applied to
/// `<item>`/`<entry>` parsing (see `parse_channel_item`).
///
/// First error wins: `bozo_exception` is only set the first time, matching the
/// convention used by `parse_channel_item`/`parse_rdf_item`, so a later, less
/// informative failure does not clobber the original diagnostic.
fn record_channel_bozo(feed: &mut ParsedFeed, err: &FeedError) {
    if !feed.bozo {
        feed.bozo_exception = Some(err.to_string());
    }
    feed.bozo = true;
}

/// Recover from a channel-level field parse error caught by `dispatch_channel_tag`.
///
/// Records the error as bozo (see [`record_channel_bozo`]), then drains the reader
/// to `tag`'s own closing tag and restores `depth` to its pre-dispatch value. Without
/// draining, a partially-consumed container's leftover children (e.g. `<textInput>`'s
/// own `<title>`/`<description>`) would be read next by the enclosing `parse_channel`
/// loop and misdispatched as if they were real channel-level siblings, silently
/// overwriting unrelated feed metadata (#463).
fn recover_channel_field_error(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    tag: &[u8],
    depth: &mut usize,
    depth_before: usize,
    feed: &mut ParsedFeed,
    err: &FeedError,
) {
    record_channel_bozo(feed, err);
    skip_to_end_qualified(reader, buf, tag);
    *depth = depth_before;
}

/// `ChannelCtx`-taking wrapper around [`recover_channel_field_error`], so
/// `dispatch_channel_tag`'s call sites don't each need to spell out
/// `ctx.xml.reader, ctx.xml.buf` inline.
fn recover_channel_error(
    ctx: &mut ChannelCtx,
    tag: &[u8],
    depth: &mut usize,
    depth_before: usize,
    feed: &mut ParsedFeed,
    err: &FeedError,
) {
    recover_channel_field_error(
        ctx.xml.reader,
        ctx.xml.buf,
        tag,
        depth,
        depth_before,
        feed,
        err,
    );
}

/// Dispatch a single `<channel>` child element to its handler.
fn dispatch_channel_tag(
    ctx: &mut ChannelCtx,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    item_lang: Option<&str>,
    feed: &mut ParsedFeed,
    depth: &mut usize,
    is_empty: bool,
) {
    // Snapshot of `depth` on entry: restored after a caught error so a partially
    // consumed nested element (e.g. `<image>`'s children) can never leak a leftover
    // increment into the enclosing `parse_channel` loop's own depth bookkeeping.
    let depth_before = *depth;

    // Use full qualified name to distinguish standard RSS tags from namespaced tags
    match tag {
        // `category` is folded in here (rather than kept as its own arm) specifically
        // to pick up the `!is_empty` guard: `parse_channel_standard`'s text read and
        // `recover_channel_field_error`'s drain both assume a matching closing tag
        // exists, which a self-closing `<category/>` doesn't have.
        b"title" | b"link" | b"description" | b"language" | b"pubDate" | b"lastBuildDate"
        | b"managingEditor" | b"webMaster" | b"generator" | b"ttl" | b"docs" | b"copyright"
        | b"category"
            if !is_empty =>
        {
            if let Err(e) = parse_channel_standard(ctx, tag, attrs, feed) {
                recover_channel_error(ctx, tag, depth, depth_before, feed, &e);
            }
        }
        b"image" if !is_empty => {
            match parse_image(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth) {
                Ok(image) => feed.feed.image = Some(image),
                Err(e) => recover_channel_error(ctx, tag, depth, depth_before, feed, &e),
            }
        }
        b"cloud" => {
            feed.feed.cloud = Some(parse_cloud(attrs));
        }
        b"textInput" if !is_empty => {
            match parse_text_input(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits) {
                Ok(ti) => feed.feed.textinput = Some(ti),
                Err(e) => recover_channel_error(ctx, tag, depth, depth_before, feed, &e),
            }
        }
        b"skipHours" if !is_empty => {
            if let Err(e) = parse_skip_hours(
                ctx.xml.reader,
                ctx.xml.buf,
                ctx.xml.limits,
                &mut feed.feed.skiphours,
            ) {
                recover_channel_error(ctx, tag, depth, depth_before, feed, &e);
            }
        }
        b"skipDays" if !is_empty => {
            if let Err(e) = parse_skip_days(
                ctx.xml.reader,
                ctx.xml.buf,
                ctx.xml.limits,
                &mut feed.feed.skipdays,
            ) {
                recover_channel_error(ctx, tag, depth, depth_before, feed, &e);
            }
        }
        b"item" if !is_empty => {
            parse_channel_item(ctx, item_lang, feed, depth);
        }
        _ => {
            if let Err(e) = parse_channel_extension(ctx, tag, attrs, feed, depth, is_empty) {
                recover_channel_error(ctx, tag, depth, depth_before, feed, &e);
            }
        }
    }
}

/// Parse <item> element within channel
#[inline]
fn parse_channel_item(
    ctx: &mut ChannelCtx,
    item_lang: Option<&str>,
    feed: &mut ParsedFeed,
    depth: &mut usize,
) {
    match feed.check_entry_limit(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth) {
        Ok(true) => {}
        Ok(false) => return,
        Err(e) => {
            // `skip_element` inside `check_entry_limit` failed (nesting overflow or
            // ill-formed XML while skipping an over-limit item) — record bozo and
            // resync to </item> instead of letting the error abort the whole channel
            // (same invariant as #463's field-level fix, reached via a different
            // path). `check_entry_limit` already set `feed.bozo` before attempting
            // the skip, so first-error-wins naturally keeps "Entry limit exceeded".
            if !feed.bozo {
                feed.bozo_exception = Some(e.to_string());
            }
            feed.bozo = true;
            skip_to_end_qualified(ctx.xml.reader, ctx.xml.buf, b"item");
            *depth = depth.saturating_sub(1);
            return;
        }
    }

    let effective_lang = item_lang.or(ctx.lang);
    let depth_before = *depth;

    match parse_item(
        &mut ctx.xml,
        depth,
        ctx.base,
        effective_lang,
        &feed.namespaces,
    ) {
        Ok((mut entry, has_attr_errors, has_entity_bozo)) => {
            if has_attr_errors {
                feed.bozo = true;
                feed.bozo_exception = Some(MALFORMED_ATTRIBUTES_ERROR.to_string());
            }
            if has_entity_bozo && !feed.bozo {
                feed.bozo = true;
                feed.bozo_exception = Some("Unresolvable entity in entry field".to_string());
            }
            if entry.summary.is_none()
                && let Some(content) = entry.content.first()
            {
                entry.summary = Some(content.value.clone());
                entry.summary_detail = Some(text_construct_from_content(content));
            }
            // Post-process: iTunes subtitle/summary promotion (order-independent).
            // Note: RSS entry.subtitle is iTunes-only — there is no native RSS <subtitle> for
            // entries. Unlike feed-level where <description> maps to feed.subtitle, entry
            // <description> maps to entry.summary, so entry.subtitle is always None before
            // itunes post-processing.
            apply_itunes_entry_promotions(&mut entry);
            feed.entries.push(entry);
        }
        Err(e) => {
            // `parse_item` failed partway through the item's own content (e.g. a
            // malformed entity in one of its fields) and left the reader positioned
            // mid-item. Without draining, the item's remaining children — several of
            // which share tag names with real channel-level fields (`title`, `link`,
            // `description`, `pubDate`, `category`) — would be read next by the
            // enclosing `parse_channel` loop and misdispatched as channel siblings,
            // corrupting feed metadata with values ripped out of this item (#463 S3).
            if !feed.bozo {
                feed.bozo_exception = Some(e.to_string());
            }
            feed.bozo = true;
            skip_to_end_qualified(ctx.xml.reader, ctx.xml.buf, b"item");
            *depth = depth_before;
        }
    }
}

/// Parse channel extension elements (iTunes, Podcast, namespaces)
#[inline]
fn parse_channel_extension(
    ctx: &mut ChannelCtx,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    depth: &mut usize,
    is_empty: bool,
) -> Result<()> {
    if tag == b"atom:link" || tag == b"atom10:link" {
        let mut href = String::new();
        let mut rel: Option<String> = None;
        let mut link_type: Option<String> = None;
        for (key, value) in attrs {
            match key.as_slice() {
                b"href" => href = truncate_to_length(value, ctx.xml.limits.max_attribute_length),
                b"rel" => {
                    rel = Some(truncate_to_length(
                        value,
                        ctx.xml.limits.max_attribute_length,
                    ));
                }
                b"type" => {
                    link_type = Some(truncate_to_length(
                        value,
                        ctx.xml.limits.max_attribute_length,
                    ));
                }
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
                .try_push_limited(link, ctx.xml.limits.max_links_per_feed);
        }
        return Ok(());
    }
    let mut handled = parse_channel_itunes(ctx, tag, attrs, feed, depth, is_empty)?;
    if !handled {
        handled = parse_channel_podcast(ctx, tag, attrs, feed, *depth, is_empty)?;
    }
    if !handled {
        handled = parse_channel_namespace(ctx, tag, attrs, feed, *depth, is_empty)?;
    }

    // Only skip element content if this is NOT an empty element
    if !handled && !is_empty {
        skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, *depth)?;
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
            title: None,
            duration: None,
        })
    }
}

/// Parse standard RSS 2.0 channel elements
#[inline]
fn parse_channel_standard(
    ctx: &mut ChannelCtx,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
) -> Result<()> {
    if parse_channel_text(ctx, tag, feed)? {
        return Ok(());
    }
    if parse_channel_dates(ctx, tag, feed)? {
        return Ok(());
    }
    parse_channel_people_and_tags(ctx, tag, attrs, feed)?;
    Ok(())
}

/// Parse text-valued standard RSS 2.0 channel elements.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_channel_text(ctx: &mut ChannelCtx, tag: &[u8], feed: &mut ParsedFeed) -> Result<bool> {
    match tag {
        b"title" => {
            let (text, bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            if bozo {
                feed.bozo = true;
                feed.bozo_exception = Some("Unresolvable entity in channel title".into());
            }
            // RSS has no per-element type attribute; real-world feeds routinely embed
            // HTML in <title>, so it is treated as potentially unsafe (fail-closed),
            // matching the treatment of <description> below (#438).
            feed.feed.set_title(TextConstruct {
                value: text,
                content_type: TextType::Html,
                language: ctx.lang.map(std::convert::Into::into),
                base: ctx.base.base().map(String::from),
            });
        }
        b"link" => {
            let link_text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            feed.feed
                .set_alternate_link(link_text.clone(), ctx.xml.limits.max_links_per_feed);

            if ctx.base.base().is_none() {
                ctx.base.update_base(&link_text);
            }
        }
        b"description" => {
            let (text, bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            if bozo {
                feed.bozo = true;
                feed.bozo_exception =
                    Some("Unresolvable entity in channel description".to_string());
            }
            feed.feed.set_subtitle(TextConstruct {
                value: text,
                content_type: TextType::Html,
                language: ctx.lang.map(std::convert::Into::into),
                base: ctx.base.base().map(String::from),
            });
        }
        b"language" => {
            feed.feed.language =
                Some(read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?.into());
        }
        b"copyright" => {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            if !text.is_empty() {
                feed.feed.rights = Some(text);
            }
        }
        b"generator" => {
            let name = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            if !name.is_empty() {
                feed.feed.set_generator(crate::types::Generator {
                    name,
                    href: None,
                    version: None,
                });
            }
        }
        b"ttl" => {
            feed.feed.ttl = Some(read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?);
        }
        b"docs" => {
            feed.feed.docs = Some(read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?);
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// Parse date-valued standard RSS 2.0 channel elements: pubDate, lastBuildDate.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_channel_dates(ctx: &mut ChannelCtx, tag: &[u8], feed: &mut ParsedFeed) -> Result<bool> {
    match tag {
        b"pubDate" => {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
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
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
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
        _ => return Ok(false),
    }
    Ok(true)
}

/// Parse people- and tag-valued standard RSS 2.0 channel elements: managingEditor,
/// webMaster, category.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_channel_people_and_tags(
    ctx: &mut ChannelCtx,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
) -> Result<bool> {
    match tag {
        b"managingEditor" => {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            feed.feed.set_author(parse_rss_person(&text));
            // Preserve raw string in author flat field; author_detail has parsed fields
            feed.feed.author = Some(text.into());
        }
        b"webMaster" => {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            feed.feed.set_publisher(parse_rss_person(&text));
            // Preserve raw string in publisher flat field; publisher_detail has parsed fields
            feed.feed.publisher = Some(text.into());
        }
        b"category" => {
            let scheme = find_attribute(attrs, b"domain").map(|s| s.to_owned().into());
            let term = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            if !term.is_empty() || scheme.is_some() {
                feed.feed.tags.try_push_limited(
                    Tag {
                        term: term.into(),
                        scheme,
                        label: None,
                    },
                    ctx.xml.limits.max_tags,
                );
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// Parse iTunes namespace tags at channel level
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_channel_itunes(
    ctx: &mut ChannelCtx,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    depth: &mut usize,
    is_empty: bool,
) -> Result<bool> {
    if parse_channel_itunes_structured(ctx, tag, attrs, feed, depth, is_empty) {
        Ok(true)
    } else {
        parse_channel_itunes_text(ctx, tag, feed, is_empty)
    }
}

/// Parse structured iTunes namespace tags at channel level (owner, category, image).
///
/// Returns `true` if the tag was recognized and handled, `false` if not recognized.
fn parse_channel_itunes_structured(
    ctx: &mut ChannelCtx,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    depth: &mut usize,
    is_empty: bool,
) -> bool {
    if is_itunes_tag(tag, b"owner", &feed.namespaces) {
        if !is_empty
            && let Ok(owner) =
                parse_itunes_owner(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)
        {
            // Promote to publisher_detail if not already set
            if feed.feed.publisher_detail.is_none() {
                let person = Person {
                    name: owner.name.as_deref().map(Into::into),
                    email: owner.email.as_deref().map(crate::types::Email::new),
                    uri: None,
                    avatar: None,
                };
                feed.feed.set_publisher(person);
            }
            itunes_feed_meta(&mut feed.feed).owner = Some(owner);
        }
        true
    } else if is_itunes_tag(tag, b"category", &feed.namespaces) {
        parse_itunes_category(ctx, attrs, feed, is_empty);
        true
    } else if is_itunes_tag(tag, b"image", &feed.namespaces) {
        if let Some(value) = find_attribute(attrs, b"href") {
            let url = truncate_to_length(value, ctx.xml.limits.max_attribute_length);
            itunes_feed_meta(&mut feed.feed).image = Some(url.clone().into());
            // itunes:image always wins over RSS <image> (order-independent via post-processing
            // in apply_itunes_feed_promotions called after the channel element loop).
            feed.feed.image = Some(Image {
                url: url.into(),
                title: None,
                link: None,
                width: None,
                height: None,
                description: None,
            });
        }
        true
    } else {
        false
    }
}

/// Parse text-valued iTunes namespace tags at channel level.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_channel_itunes_text(
    ctx: &mut ChannelCtx,
    tag: &[u8],
    feed: &mut ParsedFeed,
    is_empty: bool,
) -> Result<bool> {
    if is_itunes_tag(tag, b"author", &feed.namespaces) {
        if !is_empty {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            // Store raw value; author promotion happens in apply_itunes_feed_promotions
            // so that itunes:owner.name priority over itunes:author is order-independent.
            itunes_feed_meta(&mut feed.feed).author = Some(text);
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"subtitle", &feed.namespaces) {
        if !is_empty {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            itunes_feed_meta(&mut feed.feed).subtitle = Some(text);
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"summary", &feed.namespaces) {
        if !is_empty {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            itunes_feed_meta(&mut feed.feed).summary = Some(text);
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"explicit", &feed.namespaces) {
        if !is_empty {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            itunes_feed_meta(&mut feed.feed).explicit = parse_explicit(&text);
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"keywords", &feed.namespaces) {
        if !is_empty {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            itunes_feed_meta(&mut feed.feed).keywords = text
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"type", &feed.namespaces) {
        if !is_empty {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            itunes_feed_meta(&mut feed.feed).podcast_type = Some(text);
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"complete", &feed.namespaces) {
        if !is_empty {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            itunes_feed_meta(&mut feed.feed).complete = Some(text.trim().to_string());
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"new-feed-url", &feed.namespaces) {
        if !is_empty {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            if !text.is_empty() {
                itunes_feed_meta(&mut feed.feed).new_feed_url =
                    Some(text.trim().to_string().into());
            }
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"block", &feed.namespaces) {
        if !is_empty {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            itunes_feed_meta(&mut feed.feed).block =
                Some(u8::from(text.trim().eq_ignore_ascii_case("yes")));
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse iTunes category with potential subcategory
fn parse_itunes_category(
    ctx: &mut ChannelCtx,
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    is_empty: bool,
) {
    let category_text = find_attribute(attrs, b"text")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length))
        .unwrap_or_default();

    // Parse potential nested subcategory (only if not an empty element)
    let mut subcategory_text: Option<String> = None;
    if !is_empty {
        let mut nesting = 0;
        loop {
            match ctx.xml.reader.read_event_into(ctx.xml.buf) {
                Ok(Event::Start(sub_e))
                    if is_itunes_tag(sub_e.name().as_ref(), b"category", &feed.namespaces) =>
                {
                    nesting += 1;
                    if nesting == 1 {
                        for attr in sub_e.attributes().flatten() {
                            if attr.key.as_ref() == b"text"
                                && let Ok(value) =
                                    attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                            {
                                subcategory_text = Some(
                                    value
                                        .chars()
                                        .take(ctx.xml.limits.max_attribute_length)
                                        .collect(),
                                );
                                break;
                            }
                        }
                    }
                }
                Ok(Event::Empty(sub_e))
                    if is_itunes_tag(sub_e.name().as_ref(), b"category", &feed.namespaces)
                        && subcategory_text.is_none() =>
                {
                    for attr in sub_e.attributes().flatten() {
                        if attr.key.as_ref() == b"text"
                            && let Ok(value) =
                                attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                        {
                            subcategory_text = Some(
                                value
                                    .chars()
                                    .take(ctx.xml.limits.max_attribute_length)
                                    .collect(),
                            );
                            break;
                        }
                    }
                }
                Ok(Event::End(end_e))
                    if is_itunes_tag(end_e.name().as_ref(), b"category", &feed.namespaces) =>
                {
                    if nesting == 0 {
                        break;
                    }
                    nesting -= 1;
                }
                Ok(Event::Eof) | Err(_) => break,
                _ => {}
            }
            ctx.xml.buf.clear();
        }
    }

    feed.feed.tags.try_push_limited(
        Tag {
            term: category_text.as_str().into(),
            scheme: Some("http://www.itunes.com/".into()),
            label: None,
        },
        ctx.xml.limits.max_tags,
    );
    if let Some(ref sub) = subcategory_text {
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
    ctx: &mut ChannelCtx,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    depth: usize,
    is_empty: bool,
) -> Result<bool> {
    if parse_channel_podcast_a(ctx, tag, attrs, feed, depth, is_empty)?
        || parse_channel_podcast_b(ctx, tag, attrs, feed, is_empty)?
    {
        return Ok(true);
    }
    parse_channel_podcast_c(ctx, tag, attrs, feed, is_empty)
}

/// Parse Podcast 2.0 namespace tags at channel level: guid, funding, value.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_channel_podcast_a(
    ctx: &mut ChannelCtx,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    depth: usize,
    is_empty: bool,
) -> Result<bool> {
    if tag.starts_with(b"podcast:guid") {
        if !is_empty {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            podcast_feed_meta(&mut feed.feed).guid = Some(text);
        }
        Ok(true)
    } else if tag.starts_with(b"podcast:funding") {
        let url = find_attribute(attrs, b"url")
            .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length))
            .unwrap_or_default();
        let message = if is_empty {
            None
        } else {
            let message_text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            if message_text.is_empty() {
                None
            } else {
                Some(message_text)
            }
        };
        if !url.is_empty() {
            podcast_feed_meta(&mut feed.feed).funding.try_push_limited(
                PodcastFunding {
                    url: url.into(),
                    message,
                },
                ctx.xml.limits.max_podcast_funding,
            );
        }
        Ok(true)
    } else if tag == b"podcast:value" {
        if !is_empty {
            parse_podcast_value(ctx, attrs, feed, depth)?;
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse Podcast 2.0 namespace tags at channel level: person, medium, locked.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_channel_podcast_b(
    ctx: &mut ChannelCtx,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    is_empty: bool,
) -> Result<bool> {
    if tag.starts_with(b"podcast:person") {
        if !is_empty {
            let role = Some(find_attribute(attrs, b"role").map_or_else(
                || "host".to_string(),
                |v| truncate_to_length(v, ctx.xml.limits.max_attribute_length),
            ));
            let group = find_attribute(attrs, b"group")
                .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length));
            let img = find_attribute(attrs, b"img")
                .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length));
            let href = find_attribute(attrs, b"href")
                .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length));
            let name = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            if !name.is_empty() {
                let person = PodcastPerson {
                    name,
                    role,
                    group,
                    img: img.map(Into::into),
                    href: href.map(Into::into),
                };
                podcast_feed_meta(&mut feed.feed)
                    .persons
                    .try_push_limited(person, ctx.xml.limits.max_podcast_persons);
            }
        }
        Ok(true)
    } else if tag.starts_with(b"podcast:medium") {
        if !is_empty {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            if !text.is_empty() {
                podcast_feed_meta(&mut feed.feed).medium = Some(text);
            }
        }
        Ok(true)
    } else if tag.starts_with(b"podcast:locked") {
        if !is_empty {
            let owner = find_attribute(attrs, b"owner")
                .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length));
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            let podcast = podcast_feed_meta(&mut feed.feed);
            if !text.is_empty() {
                podcast.locked = Some(text);
            }
            if let Some(o) = owner.filter(|o| !o.is_empty()) {
                podcast.locked_owner = Some(o);
            }
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse Podcast 2.0 namespace tags at channel level: location, podroll, txt,
/// updateFrequency, follow.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_channel_podcast_c(
    ctx: &mut ChannelCtx,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    is_empty: bool,
) -> Result<bool> {
    if tag.starts_with(b"podcast:location") {
        parse_podcast_channel_location(ctx, attrs, feed, is_empty)?;
        Ok(true)
    } else if tag.starts_with(b"podcast:podroll") {
        if !is_empty {
            parse_podcast_podroll(ctx.xml.reader, ctx.xml.buf, feed, ctx.xml.limits)?;
        }
        Ok(true)
    } else if tag.starts_with(b"podcast:txt") {
        parse_podcast_channel_txt(ctx, attrs, feed, is_empty)?;
        Ok(true)
    } else if tag.starts_with(b"podcast:updateFrequency") {
        parse_podcast_update_frequency(ctx, attrs, feed, is_empty)?;
        Ok(true)
    } else if tag.starts_with(b"podcast:follow") {
        parse_podcast_channel_follow(attrs, feed, ctx.xml.limits);
        Ok(true)
    } else if tag.starts_with(b"podcast:chat") {
        parse_podcast_channel_chat(attrs, feed, ctx.xml.limits);
        Ok(true)
    } else if tag.starts_with(b"podcast:podping") {
        let uses_podping = find_attribute(attrs, b"usesPodping").and_then(|v| {
            if v.eq_ignore_ascii_case("true") {
                Some(true)
            } else if v.eq_ignore_ascii_case("false") {
                Some(false)
            } else {
                None
            }
        });
        if uses_podping.is_some() {
            let podcast = podcast_feed_meta(&mut feed.feed);
            if podcast.podping_uses_podping.is_none() {
                podcast.podping_uses_podping = uses_podping;
            }
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse Dublin Core, Content, `GeoRSS`, and Media RSS namespace tags at channel level
#[inline]
fn parse_channel_namespace(
    ctx: &mut ChannelCtx,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    depth: usize,
    is_empty: bool,
) -> Result<bool> {
    if let Some(dc_element) = is_dc_tag(tag, &feed.namespaces) {
        if !is_empty {
            let dc_elem = dc_element.to_string();
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            dublin_core::handle_feed_element(&dc_elem, &text, &mut feed.feed);
        }
        Ok(true)
    } else if let Some(_content_element) = is_content_tag(tag) {
        if !is_empty {
            skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
        }
        Ok(true)
    } else if let Some(media_element) = is_media_tag(tag, &feed.namespaces) {
        parse_channel_media(ctx, media_element, attrs, feed, depth, is_empty)?;
        Ok(true)
    } else if let Some(georss_element) = is_georss_tag(tag) {
        if !is_empty {
            if georss_element == "where" {
                let (loc, _had_bozo) =
                    parse_georss_where(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
                if let Some(loc) = loc {
                    georss::merge_geometry(&mut feed.feed.r#where, loc);
                }
            } else {
                let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
                georss::handle_feed_element(
                    georss_element.as_bytes(),
                    &text,
                    &mut feed.feed,
                    ctx.xml.limits,
                );
            }
        }
        Ok(true)
    } else if let Some(geo_element) = is_geo_tag(tag) {
        if !is_empty {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            georss::handle_feed_geo_element(geo_element.as_bytes(), &text, &mut feed.feed);
        }
        Ok(true)
    } else if tag.starts_with(b"creativeCommons:license") || tag == b"license" {
        if !is_empty {
            feed.feed.license = Some(read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?);
        }
        Ok(true)
    } else if is_thr_tag(tag).is_some() {
        // Atom Threading Extensions at channel level — recognize and skip
        if !is_empty {
            skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
        }
        Ok(true)
    } else if let Some(syn_element) = is_syn_tag(tag) {
        if !is_empty {
            let syn_elem = syn_element.to_string();
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            syndication::handle_feed_element(&syn_elem, &text, &mut feed.feed);
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse Media RSS namespace elements at channel level.
fn parse_channel_media(
    ctx: &mut ChannelCtx,
    media_element: &str,
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    depth: usize,
    is_empty: bool,
) -> Result<()> {
    match media_element {
        "thumbnail" => {
            if let Some(thumb) = build_media_thumbnail(attrs, ctx.xml.limits) {
                feed.feed
                    .media_thumbnail
                    .try_push_limited(thumb, ctx.xml.limits.max_enclosures);
            }
            if !is_empty {
                skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
            }
        }
        "content" => {
            if let Some(content) = build_media_content(attrs, ctx.xml.limits) {
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
                let scheme = find_attribute(attrs, b"scheme").map(str::to_owned);
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
    Ok(())
}

/// Parse <item> element (entry)
///
/// Returns a tuple where:
/// - First element: the parsed `Entry`
/// - Second element: `bool` indicating whether attribute parsing errors occurred
/// - Third element: `bool` indicating whether unresolvable entities were encountered
fn parse_item(
    xml: &mut XmlCtx,
    depth: &mut usize,
    base_ctx: &BaseUrlContext,
    item_lang: Option<&str>,
    namespaces: &HashMap<String, String>,
) -> Result<(Entry, bool, bool)> {
    let mut entry = Entry::with_capacity();
    let mut has_attr_errors = false;
    let mut ctx = EntryCtx {
        xml: xml.reborrow(),
        base: base_ctx,
        lang: item_lang,
        namespaces,
        bozo: false,
        has_explicit_link: false,
        guid_is_permalink: None,
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

                // NOTE: Allocation here is necessary due to borrow checker constraints.
                // We need owned tag data to pass &mut buf to helper functions simultaneously.
                // Potential future optimization: restructure helpers to avoid this allocation.
                let tag = e.name().as_ref().to_vec();
                let (attrs, attr_error) = collect_attributes(e);
                if attr_error {
                    has_attr_errors = true;
                }

                dispatch_item_tag(&mut ctx, &tag, &attrs, &mut entry, *depth, is_empty)?;
                *depth = depth.saturating_sub(1);
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"item" => {
                break;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        ctx.xml.buf.clear();
    }

    // guidislink = isPermaLink AND no explicit <link> element was present in the item.
    // Per Python feedparser semantics: when <link> exists, guid is just an identifier.
    if let Some(is_permalink) = ctx.guid_is_permalink {
        entry.guidislink = Some(is_permalink && !ctx.has_explicit_link);
        // If an explicit <link> was present, remove the guid-as-link fallback from entry.link
        // only if it was set as fallback (i.e., entry.link equals entry.id). We detect this
        // by checking: if has_explicit_link, entry.link was already overwritten by <link> arm.
        // No additional cleanup needed — <link> arm overwrites entry.link directly.
    }

    // dc:creator takes precedence over <author>
    if let Some(dc) = &entry.dc_creator {
        entry.author = Some(dc.clone());
    }

    Ok((entry, has_attr_errors, ctx.bozo))
}

/// Dispatch a single `<item>` child element to its handler.
fn dispatch_item_tag(
    ctx: &mut EntryCtx,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    depth: usize,
    is_empty: bool,
) -> Result<()> {
    // Use full qualified name to distinguish standard RSS tags from namespaced tags
    match tag {
        b"title" | b"link" | b"description" | b"guid" | b"pubDate" | b"author" | b"category"
        | b"comments" => {
            parse_item_standard(ctx, tag, attrs, entry)?;
        }
        b"enclosure" => {
            if let Some(mut enclosure) = parse_enclosure(attrs, ctx.xml.limits) {
                enclosure.url = ctx.base.resolve_safe(&enclosure.url).into();
                let link = Link {
                    href: enclosure.url.clone(),
                    rel: Some("enclosure".into()),
                    link_type: enclosure.enclosure_type.clone(),
                    length: enclosure.length.clone(),
                    ..Link::default()
                };
                entry
                    .links
                    .try_push_limited(link, ctx.xml.limits.max_links_per_entry);
                entry
                    .enclosures
                    .try_push_limited(enclosure, ctx.xml.limits.max_enclosures);
            }
            if !is_empty {
                skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
            }
        }
        b"source" => {
            if let Ok(source) = parse_source(ctx.xml.reader, ctx.xml.buf, attrs, ctx.xml.limits) {
                entry.source = Some(source);
            }
        }
        _ => {
            parse_item_extension(ctx, tag, attrs, entry, depth, is_empty)?;
        }
    }
    Ok(())
}

/// Parse extension namespace tags at item level (iTunes, Podcast, other namespaces).
fn parse_item_extension(
    ctx: &mut EntryCtx,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    depth: usize,
    is_empty: bool,
) -> Result<()> {
    let mut handled = parse_item_itunes(ctx, tag, attrs, entry, depth, is_empty)?;
    if !handled {
        handled = parse_item_podcast(ctx, tag, attrs, entry, depth, is_empty)?;
    }
    if !handled {
        handled = parse_item_namespace(ctx, tag, attrs, entry, depth, is_empty)?;
    }

    if !handled && !is_empty {
        skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
    }

    Ok(())
}

/// Parse standard RSS 2.0 item elements
#[inline]
fn parse_item_standard(
    ctx: &mut EntryCtx,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
) -> Result<()> {
    if parse_item_text(ctx, tag, entry)? {
        return Ok(());
    }
    if parse_item_links(ctx, tag, attrs, entry)? {
        return Ok(());
    }
    parse_item_meta(ctx, tag, attrs, entry)?;
    Ok(())
}

/// Parse text-valued standard RSS 2.0 item elements: title, description, comments.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_item_text(ctx: &mut EntryCtx, tag: &[u8], entry: &mut Entry) -> Result<bool> {
    match tag {
        b"title" => {
            let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            ctx.bozo |= had_bozo;
            // See feed-level <title> above: no type attribute, so treated as
            // potentially unsafe HTML by default (fail-closed, #438).
            entry.set_title(TextConstruct {
                value: text,
                content_type: TextType::Html,
                language: ctx.lang.map(std::convert::Into::into),
                base: ctx.base.base().map(String::from),
            });
        }
        b"description" => {
            let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            ctx.bozo |= had_bozo;
            entry.set_summary(TextConstruct {
                value: text,
                content_type: TextType::Html,
                language: ctx.lang.map(std::convert::Into::into),
                base: ctx.base.base().map(String::from),
            });
        }
        b"comments" => {
            entry.comments = Some(read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?);
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// Parse link-valued standard RSS 2.0 item elements: link, guid.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_item_links(
    ctx: &mut EntryCtx,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
) -> Result<bool> {
    match tag {
        b"link" => {
            let link_text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            let resolved_link = ctx.base.resolve_safe(&link_text);
            entry.link = Some(resolved_link.clone());
            entry.links.try_push_limited(
                Link {
                    href: resolved_link.into(),
                    rel: Some("alternate".into()),
                    ..Default::default()
                },
                ctx.xml.limits.max_links_per_entry,
            );
            ctx.has_explicit_link = true;
        }
        b"guid" => {
            // isPermaLink defaults to true per RSS 2.0 spec when attribute is absent
            let is_permalink = find_attribute(attrs, b"isPermaLink")
                .is_none_or(|v| v.eq_ignore_ascii_case("true"));
            let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            ctx.bozo |= had_bozo;
            entry.id = Some(text.clone().into());
            // Defer guidislink computation: need to know if explicit <link> was present.
            // guidislink = isPermaLink AND no explicit <link> element in the item.
            // Order is not guaranteed, so we track in guid_is_permalink and compute after loop.
            ctx.guid_is_permalink = Some(is_permalink);
            // Use guid as entry.link fallback when it is a permalink and no <link> present yet.
            // If <link> appears after <guid>, the fallback will be overwritten — that is correct.
            if is_permalink && entry.link.is_none() {
                let resolved = ctx.base.resolve_safe(&text);
                entry.link = Some(resolved.clone());
                entry.links.try_push_limited(
                    Link {
                        href: resolved.into(),
                        rel: Some("alternate".into()),
                        ..Default::default()
                    },
                    ctx.xml.limits.max_links_per_entry,
                );
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// Parse metadata-valued standard RSS 2.0 item elements: pubDate, author, category.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
fn parse_item_meta(
    ctx: &mut EntryCtx,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
) -> Result<bool> {
    match tag {
        b"pubDate" => {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            let dt = parse_date(&text);
            entry.published = dt;
            entry.published_str = Some(text.clone());
            if entry.updated.is_none() {
                entry.updated = dt;
                entry.updated_str = Some(text);
            }
        }
        b"author" => {
            let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            ctx.bozo |= had_bozo;
            entry.set_author(parse_rss_person(&text));
            // Preserve raw string in author flat field; author_detail has parsed fields
            entry.author = Some(text.into());
        }
        b"category" => {
            let scheme = find_attribute(attrs, b"domain").map(|s| s.to_owned().into());
            let (term, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            ctx.bozo |= had_bozo;
            entry.tags.try_push_limited(
                Tag {
                    term: term.into(),
                    scheme,
                    label: None,
                },
                ctx.xml.limits.max_tags,
            );
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// Parse iTunes namespace tags at item level
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
#[inline]
fn parse_item_itunes(
    ctx: &mut EntryCtx,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    depth: usize,
    is_empty: bool,
) -> Result<bool> {
    if is_itunes_tag(tag, b"title", ctx.namespaces) {
        let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        ctx.bozo |= had_bozo;
        itunes_entry_meta(entry).title = Some(text);
        Ok(true)
    } else if is_itunes_tag(tag, b"author", ctx.namespaces) {
        let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        ctx.bozo |= had_bozo;
        // Promote to entry.author if not already set
        if entry.author.is_none() {
            entry.set_author(Person::from_name(&text));
            entry
                .authors
                .try_push_limited(Person::from_name(&text), ctx.xml.limits.max_authors);
        }
        itunes_entry_meta(entry).author = Some(text);
        Ok(true)
    } else if is_itunes_tag(tag, b"subtitle", ctx.namespaces) {
        let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        ctx.bozo |= had_bozo;
        itunes_entry_meta(entry).subtitle = Some(text);
        Ok(true)
    } else if is_itunes_tag(tag, b"summary", ctx.namespaces) {
        let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        ctx.bozo |= had_bozo;
        itunes_entry_meta(entry).summary = Some(text);
        Ok(true)
    } else if is_itunes_tag(tag, b"duration", ctx.namespaces) {
        let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        let itunes = itunes_entry_meta(entry);
        if !text.is_empty() {
            itunes.duration = Some(text);
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"explicit", ctx.namespaces) {
        let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        itunes_entry_meta(entry).explicit = parse_explicit(&text);
        Ok(true)
    } else if is_itunes_tag(tag, b"image", ctx.namespaces) {
        if let Some(value) = find_attribute(attrs, b"href") {
            itunes_entry_meta(entry).image =
                Some(truncate_to_length(value, ctx.xml.limits.max_attribute_length).into());
        }
        if !is_empty {
            skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"episode", ctx.namespaces) {
        let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        let itunes = itunes_entry_meta(entry);
        if !text.is_empty() {
            itunes.episode = Some(text);
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"season", ctx.namespaces) {
        let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        let itunes = itunes_entry_meta(entry);
        if !text.is_empty() {
            itunes.season = Some(text);
        }
        Ok(true)
    } else if is_itunes_tag(tag, b"episodeType", ctx.namespaces) {
        let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        itunes_entry_meta(entry).episode_type = Some(text);
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse Podcast 2.0 namespace tags at item level
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
#[inline]
fn parse_item_podcast(
    ctx: &mut EntryCtx,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    depth: usize,
    is_empty: bool,
) -> Result<bool> {
    if parse_item_podcast_a(ctx, tag, attrs, entry, depth, is_empty)? {
        return Ok(true);
    }
    parse_item_podcast_b(ctx, tag, attrs, entry, depth, is_empty)
}

/// Parse Podcast 2.0 namespace tags at item level: transcript, person, chapters,
/// soundbite, medium, season, episode.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
#[inline]
fn parse_item_podcast_a(
    ctx: &mut EntryCtx,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    depth: usize,
    is_empty: bool,
) -> Result<bool> {
    if tag.starts_with(b"podcast:transcript") {
        parse_podcast_transcript(ctx, attrs, entry, depth, is_empty)?;
        Ok(true)
    } else if tag.starts_with(b"podcast:person") {
        parse_podcast_person(ctx, attrs, entry)?;
        Ok(true)
    } else if tag.starts_with(b"podcast:chapters") {
        parse_podcast_chapters(ctx, attrs, entry, depth, is_empty)?;
        Ok(true)
    } else if tag.starts_with(b"podcast:soundbite") {
        parse_podcast_soundbite(ctx, attrs, entry, depth, is_empty)?;
        Ok(true)
    } else if tag.starts_with(b"podcast:medium") {
        if !is_empty {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            if !text.is_empty() {
                let podcast = entry
                    .podcast
                    .get_or_insert_with(|| Box::new(PodcastEntryMeta::default()));
                podcast.medium = Some(text);
            }
        }
        Ok(true)
    } else if tag.starts_with(b"podcast:season") {
        let value = if is_empty {
            find_attribute(attrs, b"number")
                .map(|n| truncate_to_length(n, ctx.xml.limits.max_attribute_length))
        } else {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            if text.is_empty() {
                find_attribute(attrs, b"number")
                    .map(|n| truncate_to_length(n, ctx.xml.limits.max_attribute_length))
            } else {
                Some(text)
            }
        };
        if let Some(v) = value {
            entry
                .podcast
                .get_or_insert_with(|| Box::new(PodcastEntryMeta::default()))
                .season = Some(v);
        }
        Ok(true)
    } else if tag.starts_with(b"podcast:episode") {
        let value = if is_empty {
            find_attribute(attrs, b"number")
                .map(|n| truncate_to_length(n, ctx.xml.limits.max_attribute_length))
        } else {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            if text.is_empty() {
                find_attribute(attrs, b"number")
                    .map(|n| truncate_to_length(n, ctx.xml.limits.max_attribute_length))
            } else {
                Some(text)
            }
        };
        if let Some(v) = value {
            entry
                .podcast
                .get_or_insert_with(|| Box::new(PodcastEntryMeta::default()))
                .episode = Some(v);
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse Podcast 2.0 namespace tags at item level: alternateEnclosure, location,
/// socialInteract, txt, follow, chat, value.
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
#[inline]
fn parse_item_podcast_b(
    ctx: &mut EntryCtx,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    depth: usize,
    is_empty: bool,
) -> Result<bool> {
    if tag.starts_with(b"podcast:alternateEnclosure") {
        if !is_empty {
            parse_podcast_alternate_enclosure(ctx, attrs, entry)?;
        }
        Ok(true)
    } else if tag.starts_with(b"podcast:location") {
        parse_podcast_item_location(ctx, attrs, entry, is_empty)?;
        Ok(true)
    } else if tag.starts_with(b"podcast:socialInteract") {
        parse_podcast_social_interact(ctx, attrs, entry, depth, is_empty)?;
        Ok(true)
    } else if tag.starts_with(b"podcast:txt") {
        parse_podcast_item_txt(ctx, attrs, entry, is_empty)?;
        Ok(true)
    } else if tag.starts_with(b"podcast:follow") {
        parse_podcast_item_follow(attrs, entry, ctx.xml.limits);
        Ok(true)
    } else if tag.starts_with(b"podcast:chat") {
        parse_podcast_item_chat(attrs, entry, ctx.xml.limits);
        Ok(true)
    } else if tag == b"podcast:value" {
        if !is_empty {
            parse_item_podcast_value(ctx, attrs, entry, depth)?;
        }
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
    ctx: &mut EntryCtx,
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    depth: usize,
    is_empty: bool,
) -> Result<()> {
    let url = find_attribute(attrs, b"url")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length))
        .unwrap_or_default();
    let transcript_type = find_attribute(attrs, b"type")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length));
    let language = find_attribute(attrs, b"language")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length));
    let rel = find_attribute(attrs, b"rel")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length));

    if !url.is_empty() {
        let transcript = PodcastTranscript {
            url: url.into(),
            transcript_type: transcript_type.map(Into::into),
            language,
            rel,
        };
        entry
            .podcast_transcripts
            .try_push_limited(transcript.clone(), ctx.xml.limits.max_podcast_transcripts);
        let podcast = entry
            .podcast
            .get_or_insert_with(|| Box::new(PodcastEntryMeta::default()));
        podcast
            .transcript
            .try_push_limited(transcript, ctx.xml.limits.max_podcast_transcripts);
    }

    if !is_empty {
        skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
    }

    Ok(())
}

/// Parse Podcast 2.0 person element
fn parse_podcast_person(
    ctx: &mut EntryCtx,
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
) -> Result<()> {
    let role = Some(find_attribute(attrs, b"role").map_or_else(
        || "host".to_string(),
        |v| truncate_to_length(v, ctx.xml.limits.max_attribute_length),
    ));
    let group = find_attribute(attrs, b"group")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length));
    let img = find_attribute(attrs, b"img")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length));
    let href = find_attribute(attrs, b"href")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length));

    let name = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
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
            .try_push_limited(person.clone(), ctx.xml.limits.max_podcast_persons);
        let podcast = entry
            .podcast
            .get_or_insert_with(|| Box::new(PodcastEntryMeta::default()));
        podcast
            .persons
            .try_push_limited(person, ctx.xml.limits.max_podcast_persons);
    }

    Ok(())
}

/// Parse Podcast 2.0 chapters element
fn parse_podcast_chapters(
    ctx: &mut EntryCtx,
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    depth: usize,
    is_empty: bool,
) -> Result<()> {
    let url = find_attribute(attrs, b"url")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length))
        .unwrap_or_default();
    let type_ = find_attribute(attrs, b"type")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length))
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
        skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
    }

    Ok(())
}

/// Parse Podcast 2.0 soundbite element
fn parse_podcast_soundbite(
    ctx: &mut EntryCtx,
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    depth: usize,
    is_empty: bool,
) -> Result<()> {
    let start_time = find_attribute(attrs, b"startTime").and_then(|v| v.parse::<f64>().ok());
    let duration = find_attribute(attrs, b"duration").and_then(|v| v.parse::<f64>().ok());

    if let (Some(start_time), Some(duration)) = (start_time, duration) {
        let title = if is_empty {
            None
        } else {
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
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
            ctx.xml.limits.max_podcast_soundbites,
        );
    } else if !is_empty {
        skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
    }

    Ok(())
}

/// Parse podcast:location at channel level
fn parse_podcast_channel_location(
    ctx: &mut ChannelCtx,
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    is_empty: bool,
) -> Result<()> {
    let geo = find_attribute(attrs, b"geo")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length));
    let osm = find_attribute(attrs, b"osm")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length));
    let name = if is_empty {
        String::new()
    } else {
        read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?
    };
    if !name.is_empty() {
        let podcast = feed
            .feed
            .podcast
            .get_or_insert_with(|| Box::new(PodcastMeta::default()));
        podcast.location = Some(PodcastLocation { name, geo, osm });
    }
    Ok(())
}

/// Parse a `podcast:remoteItem` element's attributes into a [`PodcastRemoteItem`]
///
/// Does not apply any validity guard (e.g. requiring `feedGuid`/`feedUrl`) — a
/// `podcast:remoteItem` nested inside a `podcast:valueTimeSplit` may legitimately
/// carry only `itemGuid`. Callers that need the podroll-specific "must have a feed
/// reference" guard apply it themselves at the call site.
fn parse_remote_item_attrs(
    attrs: &[(Vec<u8>, String)],
    limits: &ParserLimits,
) -> PodcastRemoteItem {
    let feed_guid = find_attribute(attrs, b"feedGuid")
        .map(|v| truncate_to_length(v, limits.max_attribute_length));
    let feed_url = find_attribute(attrs, b"feedUrl")
        .map(|v| truncate_to_length(v, limits.max_attribute_length));
    let item_guid = find_attribute(attrs, b"itemGuid")
        .map(|v| truncate_to_length(v, limits.max_attribute_length));
    let medium = find_attribute(attrs, b"medium")
        .map(|v| truncate_to_length(v, limits.max_attribute_length));
    let title =
        find_attribute(attrs, b"title").map(|v| truncate_to_length(v, limits.max_attribute_length));
    PodcastRemoteItem {
        feed_guid,
        feed_url: feed_url.map(Into::into),
        item_guid,
        medium,
        title,
    }
}

/// Parse podcast:podroll container at channel level
fn parse_podcast_podroll(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    feed: &mut ParsedFeed,
    limits: &ParserLimits,
) -> Result<()> {
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e) | Event::Empty(e))
                if e.name().as_ref().starts_with(b"podcast:remoteItem") =>
            {
                let (item_attrs, _) = collect_attributes(&e);
                let item = parse_remote_item_attrs(&item_attrs, limits);
                if item.feed_guid.is_some() || item.feed_url.is_some() {
                    let podcast = feed
                        .feed
                        .podcast
                        .get_or_insert_with(|| Box::new(PodcastMeta::default()));
                    podcast
                        .podroll
                        .try_push_limited(item, limits.max_podcast_podroll);
                }
            }
            Ok(Event::End(e)) if e.name().as_ref().starts_with(b"podcast:podroll") => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

/// Parse podcast:txt at channel level
fn parse_podcast_channel_txt(
    ctx: &mut ChannelCtx,
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    is_empty: bool,
) -> Result<()> {
    let purpose = find_attribute(attrs, b"purpose")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length));
    let value = if is_empty {
        String::new()
    } else {
        read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?
    };
    if !value.is_empty() {
        let podcast = feed
            .feed
            .podcast
            .get_or_insert_with(|| Box::new(PodcastMeta::default()));
        podcast.txt.try_push_limited(
            PodcastTxt { purpose, value },
            ctx.xml.limits.max_podcast_txt,
        );
    }
    Ok(())
}

/// Parse podcast:updateFrequency at channel level
fn parse_podcast_update_frequency(
    ctx: &mut ChannelCtx,
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    is_empty: bool,
) -> Result<()> {
    let rrule = find_attribute(attrs, b"rrule")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length));
    let complete = find_attribute(attrs, b"complete").and_then(|v| {
        if v.eq_ignore_ascii_case("true") {
            Some(true)
        } else if v.eq_ignore_ascii_case("false") {
            Some(false)
        } else {
            None
        }
    });
    let dtstart = find_attribute(attrs, b"dtstart")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length));
    let label = if is_empty {
        None
    } else {
        let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        if text.is_empty() { None } else { Some(text) }
    };
    let podcast = feed
        .feed
        .podcast
        .get_or_insert_with(|| Box::new(PodcastMeta::default()));
    podcast.update_frequency = Some(PodcastUpdateFrequency {
        rrule,
        complete,
        dtstart,
        label,
    });
    Ok(())
}

/// Parse podcast:follow at channel level
fn parse_podcast_channel_follow(
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    limits: &ParserLimits,
) {
    let url = find_attribute(attrs, b"url")
        .map(|v| truncate_to_length(v, limits.max_attribute_length))
        .unwrap_or_default();
    if !url.is_empty() {
        let platform = find_attribute(attrs, b"platform")
            .map(|v| truncate_to_length(v, limits.max_attribute_length));
        let podcast = feed
            .feed
            .podcast
            .get_or_insert_with(|| Box::new(PodcastMeta::default()));
        podcast.follow.try_push_limited(
            PodcastFollow {
                url: url.into(),
                platform,
            },
            limits.max_podcast_follow,
        );
    }
}

/// Parse podcast:chat at channel level
fn parse_podcast_channel_chat(
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    limits: &ParserLimits,
) {
    let server = find_attribute(attrs, b"server")
        .map(|v| truncate_to_length(v, limits.max_attribute_length))
        .unwrap_or_default();
    let protocol = find_attribute(attrs, b"protocol")
        .map(|v| truncate_to_length(v, limits.max_attribute_length))
        .unwrap_or_default();
    if !server.is_empty() && !protocol.is_empty() {
        let account_id = find_attribute(attrs, b"accountId")
            .map(|v| truncate_to_length(v, limits.max_attribute_length));
        let space = find_attribute(attrs, b"space")
            .map(|v| truncate_to_length(v, limits.max_attribute_length));
        let podcast = feed
            .feed
            .podcast
            .get_or_insert_with(|| Box::new(PodcastMeta::default()));
        podcast.chat.try_push_limited(
            PodcastChat {
                server,
                protocol,
                account_id,
                space,
            },
            limits.max_podcast_chat,
        );
    }
}

fn parse_alternate_enclosure_attrs(
    attrs: &[(Vec<u8>, String)],
    limits: &ParserLimits,
) -> Option<PodcastAlternateEnclosure> {
    let type_ = find_attribute(attrs, b"type")
        .map(|v| truncate_to_length(v, limits.max_attribute_length))
        .unwrap_or_default();
    if type_.is_empty() {
        return None;
    }
    let length = find_attribute(attrs, b"length").and_then(|v| v.parse::<u64>().ok());
    let bitrate = find_attribute(attrs, b"bitrate").and_then(|v| v.parse::<f64>().ok());
    let height = find_attribute(attrs, b"height").and_then(|v| v.parse::<u32>().ok());
    let lang =
        find_attribute(attrs, b"lang").map(|v| truncate_to_length(v, limits.max_attribute_length));
    let title =
        find_attribute(attrs, b"title").map(|v| truncate_to_length(v, limits.max_attribute_length));
    let rel =
        find_attribute(attrs, b"rel").map(|v| truncate_to_length(v, limits.max_attribute_length));
    let codecs = find_attribute(attrs, b"codecs")
        .map(|v| truncate_to_length(v, limits.max_attribute_length));
    let default_val = find_attribute(attrs, b"default").and_then(|v| {
        if v.eq_ignore_ascii_case("true") {
            Some(true)
        } else if v.eq_ignore_ascii_case("false") {
            Some(false)
        } else {
            None
        }
    });
    Some(PodcastAlternateEnclosure {
        type_: type_.into(),
        length,
        bitrate,
        height,
        lang,
        title,
        rel,
        codecs,
        default: default_val,
        sources: Vec::with_capacity(limits.max_podcast_alternate_enclosure_sources.min(4)),
        integrity: None,
    })
}

fn parse_alternate_enclosure_children(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    enc: &mut PodcastAlternateEnclosure,
    limits: &ParserLimits,
) -> Result<()> {
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                let tag_name = e.name();
                let tag_bytes = tag_name.as_ref();
                if tag_bytes.starts_with(b"podcast:source") {
                    let (src_attrs, _) = collect_attributes(&e);
                    let uri = find_attribute(&src_attrs, b"uri")
                        .map(|v| truncate_to_length(v, limits.max_attribute_length))
                        .unwrap_or_default();
                    if !uri.is_empty() {
                        let content_type = find_attribute(&src_attrs, b"contentType")
                            .map(|v| truncate_to_length(v, limits.max_attribute_length));
                        enc.sources.try_push_limited(
                            PodcastAlternateEnclosureSource {
                                uri: uri.into(),
                                content_type: content_type.map(Into::into),
                            },
                            limits.max_podcast_alternate_enclosure_sources,
                        );
                    }
                } else if tag_bytes.starts_with(b"podcast:integrity") {
                    let (int_attrs, is_empty_int) = collect_attributes(&e);
                    let int_type = find_attribute(&int_attrs, b"type")
                        .map(|v| truncate_to_length(v, limits.max_attribute_length))
                        .unwrap_or_default();
                    let value = if is_empty_int {
                        String::new()
                    } else {
                        buf.clear();
                        read_text_str(reader, buf, limits)?
                    };
                    if !int_type.is_empty() && !value.is_empty() {
                        enc.integrity = Some(PodcastIntegrity {
                            type_: int_type,
                            value,
                        });
                    }
                }
            }
            Ok(Event::End(e)) if e.name().as_ref().starts_with(b"podcast:alternateEnclosure") => {
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

/// Parse podcast:alternateEnclosure container at item level
fn parse_podcast_alternate_enclosure(
    ctx: &mut EntryCtx,
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
) -> Result<()> {
    let Some(mut enc) = parse_alternate_enclosure_attrs(attrs, ctx.xml.limits) else {
        return Ok(());
    };
    parse_alternate_enclosure_children(ctx.xml.reader, ctx.xml.buf, &mut enc, ctx.xml.limits)?;
    let podcast = entry
        .podcast
        .get_or_insert_with(|| Box::new(PodcastEntryMeta::default()));
    podcast
        .alternate_enclosures
        .try_push_limited(enc, ctx.xml.limits.max_podcast_alternate_enclosures);
    Ok(())
}

/// Parse podcast:location at item level
fn parse_podcast_item_location(
    ctx: &mut EntryCtx,
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    is_empty: bool,
) -> Result<()> {
    let geo = find_attribute(attrs, b"geo")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length));
    let osm = find_attribute(attrs, b"osm")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length));
    let name = if is_empty {
        String::new()
    } else {
        read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?
    };
    if !name.is_empty() {
        let podcast = entry
            .podcast
            .get_or_insert_with(|| Box::new(PodcastEntryMeta::default()));
        podcast.location = Some(PodcastLocation { name, geo, osm });
    }
    Ok(())
}

/// Parse podcast:socialInteract at item level
fn parse_podcast_social_interact(
    ctx: &mut EntryCtx,
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    depth: usize,
    is_empty: bool,
) -> Result<()> {
    let uri = find_attribute(attrs, b"uri")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length))
        .unwrap_or_default();
    if uri.is_empty() {
        if !is_empty {
            skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
        }
        return Ok(());
    }
    let protocol = find_attribute(attrs, b"protocol")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length));
    let account_id = find_attribute(attrs, b"accountId")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length));
    let account_url = find_attribute(attrs, b"accountUrl")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length));
    let priority = find_attribute(attrs, b"priority").and_then(|v| v.parse::<u32>().ok());

    if !is_empty {
        skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
    }

    let podcast = entry
        .podcast
        .get_or_insert_with(|| Box::new(PodcastEntryMeta::default()));
    podcast.social_interact.try_push_limited(
        PodcastSocialInteract {
            uri: uri.into(),
            protocol,
            account_id,
            account_url: account_url.map(Into::into),
            priority,
        },
        ctx.xml.limits.max_podcast_social_interact,
    );
    Ok(())
}

/// Parse podcast:txt at item level
fn parse_podcast_item_txt(
    ctx: &mut EntryCtx,
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    is_empty: bool,
) -> Result<()> {
    let purpose = find_attribute(attrs, b"purpose")
        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length));
    let value = if is_empty {
        String::new()
    } else {
        read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?
    };
    if !value.is_empty() {
        let podcast = entry
            .podcast
            .get_or_insert_with(|| Box::new(PodcastEntryMeta::default()));
        podcast.txt.try_push_limited(
            PodcastTxt { purpose, value },
            ctx.xml.limits.max_podcast_txt,
        );
    }
    Ok(())
}

/// Parse podcast:follow at item level
fn parse_podcast_item_follow(
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    limits: &ParserLimits,
) {
    let url = find_attribute(attrs, b"url")
        .map(|v| truncate_to_length(v, limits.max_attribute_length))
        .unwrap_or_default();
    if !url.is_empty() {
        let platform = find_attribute(attrs, b"platform")
            .map(|v| truncate_to_length(v, limits.max_attribute_length));
        let podcast = entry
            .podcast
            .get_or_insert_with(|| Box::new(PodcastEntryMeta::default()));
        podcast.follow.try_push_limited(
            PodcastFollow {
                url: url.into(),
                platform,
            },
            limits.max_podcast_follow,
        );
    }
}

/// Parse podcast:chat at item level
fn parse_podcast_item_chat(attrs: &[(Vec<u8>, String)], entry: &mut Entry, limits: &ParserLimits) {
    let server = find_attribute(attrs, b"server")
        .map(|v| truncate_to_length(v, limits.max_attribute_length))
        .unwrap_or_default();
    let protocol = find_attribute(attrs, b"protocol")
        .map(|v| truncate_to_length(v, limits.max_attribute_length))
        .unwrap_or_default();
    if !server.is_empty() && !protocol.is_empty() {
        let account_id = find_attribute(attrs, b"accountId")
            .map(|v| truncate_to_length(v, limits.max_attribute_length));
        let space = find_attribute(attrs, b"space")
            .map(|v| truncate_to_length(v, limits.max_attribute_length));
        let podcast = entry
            .podcast
            .get_or_insert_with(|| Box::new(PodcastEntryMeta::default()));
        podcast.chat.try_push_limited(
            PodcastChat {
                server,
                protocol,
                account_id,
                space,
            },
            limits.max_podcast_chat,
        );
    }
}

/// Parse Dublin Core, Content, and Media RSS namespace tags at item level
///
/// Returns `Ok(true)` if the tag was recognized and handled, `Ok(false)` if not recognized.
#[inline]
fn parse_item_namespace(
    ctx: &mut EntryCtx,
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    depth: usize,
    is_empty: bool,
) -> Result<bool> {
    if let Some(dc_element) = is_dc_tag(tag, ctx.namespaces) {
        let dc_elem = dc_element.to_string();
        let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        ctx.bozo |= had_bozo;
        dublin_core::handle_entry_element(&dc_elem, &text, entry);
        Ok(true)
    } else if let Some(content_element) = is_content_tag(tag) {
        let content_elem = content_element.to_string();
        let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        ctx.bozo |= had_bozo;
        content::handle_entry_element(&content_elem, &text, entry, ctx.lang, ctx.base.base());
        Ok(true)
    } else if let Some(georss_element) = is_georss_tag(tag) {
        if georss_element == "where" {
            if !is_empty {
                let (loc, had_bozo) =
                    parse_georss_where(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
                ctx.bozo |= had_bozo;
                if let Some(loc) = loc {
                    georss::merge_geometry(&mut entry.r#where, loc);
                }
            }
        } else {
            let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            ctx.bozo |= had_bozo;
            georss::handle_entry_element(georss_element.as_bytes(), &text, entry, ctx.xml.limits);
        }
        Ok(true)
    } else if let Some(geo_element) = is_geo_tag(tag) {
        if !is_empty {
            let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            ctx.bozo |= had_bozo;
            georss::handle_entry_geo_element(geo_element.as_bytes(), &text, entry);
        }
        Ok(true)
    } else if let Some(slash_element) = is_slash_tag(tag) {
        let slash_elem = slash_element.to_string();
        let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        ctx.bozo |= had_bozo;
        slash::handle_slash_entry_element(&slash_elem, &text, entry);
        Ok(true)
    } else if let Some(wfw_element) = is_wfw_tag(tag) {
        let wfw_elem = wfw_element.to_string();
        let (text, had_bozo) = read_text(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
        ctx.bozo |= had_bozo;
        slash::handle_wfw_entry_element(&wfw_elem, &text, entry);
        Ok(true)
    } else if let Some(media_element) = is_media_tag(tag, ctx.namespaces) {
        parse_item_media(ctx, media_element, attrs, entry, depth, is_empty)?;
        Ok(true)
    } else if tag.starts_with(b"creativeCommons:license") || tag == b"license" {
        entry.license = Some(read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?);
        Ok(true)
    } else if let Some(thr_element) = is_thr_tag(tag) {
        // Atom Threading Extensions (RFC 4685)
        match thr_element {
            "in-reply-to" => {
                if let Some(reply) = threading::parse_in_reply_to_from_collected(
                    attrs,
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
    } else {
        Ok(false)
    }
}

/// Parse Media RSS namespace elements
fn parse_item_media(
    ctx: &mut EntryCtx,
    media_element: &str,
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    depth: usize,
    is_empty: bool,
) -> Result<()> {
    if parse_item_media_object(ctx, media_element, attrs, entry, depth, is_empty)? {
        return Ok(());
    }
    if parse_item_media_meta(ctx, media_element, attrs, entry, is_empty)? {
        return Ok(());
    }
    parse_item_media_text(ctx, media_element, attrs, entry, is_empty)
}

/// Parse structured `media:*` elements that contain their own child content:
/// thumbnail, content, group.
///
/// Every arm returns `Ok(true)` unconditionally; `is_empty` is only tested inside
/// arm bodies. This must NOT use match guards — the `_` fallback in
/// [`parse_item_media_text`] performs an unguarded `read_text_str`, so a
/// self-closing element must never fall through to it (see #433 review notes).
fn parse_item_media_object(
    ctx: &mut EntryCtx,
    media_element: &str,
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    depth: usize,
    is_empty: bool,
) -> Result<bool> {
    match media_element {
        "thumbnail" => {
            if let Some(thumb) = build_media_thumbnail(attrs, ctx.xml.limits) {
                entry
                    .media_thumbnail
                    .try_push_limited(thumb, ctx.xml.limits.max_enclosures);
            }
            if !is_empty {
                skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, depth)?;
            }
            Ok(true)
        }
        "content" => {
            if let Some(content) = build_media_content(attrs, ctx.xml.limits) {
                entry
                    .media_content
                    .try_push_limited(content, ctx.xml.limits.max_enclosures);
            }
            if !is_empty {
                parse_media_content_children(ctx, entry, depth)?;
            }
            Ok(true)
        }
        "group" => {
            // media:group is a transparent container; parse its children as if they
            // were direct siblings of the enclosing item.
            if !is_empty {
                parse_media_group(ctx, entry, depth)?;
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Parse `media:*` metadata elements: rating, credit, copyright.
///
/// Every arm returns `Ok(true)` unconditionally; `is_empty` is only tested inside
/// arm bodies (see [`parse_item_media_object`] for why match guards are unsafe here).
fn parse_item_media_meta(
    ctx: &mut EntryCtx,
    media_element: &str,
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    is_empty: bool,
) -> Result<bool> {
    match media_element {
        "rating" => {
            if !is_empty {
                let scheme = find_attribute(attrs, b"scheme").map(str::to_owned);
                let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
                media_rss::handle_entry_rating(scheme.as_deref(), &text, entry);
            }
            Ok(true)
        }
        "credit" => {
            let role = find_attribute(attrs, b"role").map(str::to_owned);
            let scheme = find_attribute(attrs, b"scheme").map(str::to_owned);
            let text = if is_empty {
                String::new()
            } else {
                read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?
            };
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
            Ok(true)
        }
        "copyright" => {
            let url = find_attribute(attrs, b"url").map(str::to_owned);
            if !is_empty {
                read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            }
            entry.media_copyright = Some(MediaCopyright { url });
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Parse `media:*` text elements: description, title; falls back to the generic
/// Media RSS element handler for anything else.
///
/// The `_` fallback performs an unguarded `read_text_str`, matching today's
/// behavior exactly — see [`parse_item_media_object`] for why the caller must
/// never let a self-closing structured element reach here.
fn parse_item_media_text(
    ctx: &mut EntryCtx,
    media_element: &str,
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    is_empty: bool,
) -> Result<()> {
    match media_element {
        "description" => {
            let type_attr = find_attribute(attrs, b"type").map(str::to_owned);
            let text = if is_empty {
                String::new()
            } else {
                read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?
            };
            let is_plain = type_attr.as_deref().is_none_or(|t| t == "plain");
            if is_plain && !text.is_empty() {
                entry.media_description = Some(text.clone());
            }
            if entry.summary.is_none() && !text.is_empty() {
                entry.summary = Some(text);
            }
        }
        "title" => {
            let type_attr = find_attribute(attrs, b"type").map(str::to_owned);
            let text = if is_empty {
                String::new()
            } else {
                read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?
            };
            let is_plain = type_attr.as_deref().is_none_or(|t| t == "plain");
            if is_plain && !text.is_empty() {
                entry.media_title = Some(text.clone());
            }
            if entry.title.is_none() && !text.is_empty() {
                entry.title = Some(text);
            }
        }
        _ => {
            let media_elem = media_element.to_string();
            let text = read_text_str(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits)?;
            media_rss::handle_entry_element(&media_elem, &text, entry);
        }
    }
    Ok(())
}

/// Parse children of `<media:group>`, treating the group as a transparent container.
///
/// Per the Media RSS spec, `<media:group>` groups related media objects but has no
/// semantic meaning of its own; its children are promoted to the parent item level.
fn parse_media_group(ctx: &mut EntryCtx, entry: &mut Entry, depth: usize) -> Result<()> {
    let mut inner_depth = depth;
    loop {
        ctx.xml.buf.clear();
        match ctx.xml.reader.read_event_into(ctx.xml.buf) {
            Ok(Event::Start(e)) => {
                inner_depth += 1;
                check_depth(inner_depth, ctx.xml.limits.max_nesting_depth)?;
                let tag = e.name().as_ref().to_vec();
                let (attrs, _) = collect_attributes(&e);
                if let Some(media_element) = is_media_tag(&tag, ctx.namespaces) {
                    parse_item_media(ctx, media_element, &attrs, entry, inner_depth, false)?;
                } else {
                    skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, inner_depth)?;
                }
                inner_depth = inner_depth.saturating_sub(1);
            }
            Ok(Event::Empty(e)) => {
                let tag = e.name().as_ref().to_vec();
                let (attrs, _) = collect_attributes(&e);
                if let Some(media_element) = is_media_tag(&tag, ctx.namespaces) {
                    parse_item_media(ctx, media_element, &attrs, entry, inner_depth, true)?;
                }
            }
            Ok(Event::End(_) | Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    Ok(())
}

/// Parse children of `<media:content>`, collecting nested `media:thumbnail` elements.
///
/// Python feedparser collects all `media:thumbnail` elements into `entry.media_thumbnail`
/// regardless of whether they are top-level or nested inside `media:content`.
fn parse_media_content_children(ctx: &mut EntryCtx, entry: &mut Entry, depth: usize) -> Result<()> {
    let mut inner_depth = depth;
    loop {
        ctx.xml.buf.clear();
        match ctx.xml.reader.read_event_into(ctx.xml.buf) {
            Ok(Event::Start(e)) => {
                inner_depth += 1;
                check_depth(inner_depth, ctx.xml.limits.max_nesting_depth)?;
                let tag = e.name().as_ref().to_vec();
                if is_media_tag(&tag, ctx.namespaces) == Some("thumbnail") {
                    let (attrs, _) = collect_attributes(&e);
                    let url = find_attribute(&attrs, b"url")
                        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length))
                        .unwrap_or_default();
                    let width = find_attribute(&attrs, b"width").map(str::to_owned);
                    let height = find_attribute(&attrs, b"height").map(str::to_owned);
                    let time = find_attribute(&attrs, b"time").map(str::to_owned);
                    if !url.is_empty() {
                        entry.media_thumbnail.try_push_limited(
                            MediaThumbnail {
                                url: url.into(),
                                width,
                                height,
                                time,
                            },
                            ctx.xml.limits.max_enclosures,
                        );
                    }
                }
                skip_element(ctx.xml.reader, ctx.xml.buf, ctx.xml.limits, inner_depth)?;
                inner_depth = inner_depth.saturating_sub(1);
            }
            Ok(Event::Empty(e)) => {
                let tag = e.name().as_ref().to_vec();
                if is_media_tag(&tag, ctx.namespaces) == Some("thumbnail") {
                    let (attrs, _) = collect_attributes(&e);
                    let url = find_attribute(&attrs, b"url")
                        .map(|v| truncate_to_length(v, ctx.xml.limits.max_attribute_length))
                        .unwrap_or_default();
                    let width = find_attribute(&attrs, b"width").map(str::to_owned);
                    let height = find_attribute(&attrs, b"height").map(str::to_owned);
                    let time = find_attribute(&attrs, b"time").map(str::to_owned);
                    if !url.is_empty() {
                        entry.media_thumbnail.try_push_limited(
                            MediaThumbnail {
                                url: url.into(),
                                width,
                                height,
                                time,
                            },
                            ctx.xml.limits.max_enclosures,
                        );
                    }
                }
            }
            Ok(Event::End(_) | Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    Ok(())
}

/// Parse `<cloud>` attributes into a `Cloud` struct (self-closing element)
fn parse_cloud(attrs: &[(Vec<u8>, String)]) -> crate::types::Cloud {
    crate::types::Cloud {
        domain: find_attribute(attrs, b"domain").map(str::to_string),
        port: find_attribute(attrs, b"port").map(str::to_string),
        path: find_attribute(attrs, b"path").map(str::to_string),
        register_procedure: find_attribute(attrs, b"registerProcedure").map(str::to_string),
        protocol: find_attribute(attrs, b"protocol").map(str::to_string),
    }
}

/// Parse `<textInput>` child elements into a `TextInput` struct
fn parse_text_input(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
) -> Result<crate::types::TextInput> {
    let mut ti = crate::types::TextInput::default();
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) => {
                let tag = e.local_name().as_ref().to_vec();
                match tag.as_slice() {
                    b"title" => ti.title = Some(read_text_str(reader, buf, limits)?),
                    b"description" => ti.description = Some(read_text_str(reader, buf, limits)?),
                    b"name" => ti.name = Some(read_text_str(reader, buf, limits)?),
                    b"link" => ti.link = Some(read_text_str(reader, buf, limits)?),
                    _ => {}
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"textInput" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }
    Ok(ti)
}

/// Parse `<skipHours>` child `<hour>` elements (values 0–23)
fn parse_skip_hours(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
    hours: &mut Vec<u32>,
) -> Result<()> {
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) if e.local_name().as_ref() == b"hour" => {
                let text = read_text_str(reader, buf, limits)?;
                if let Ok(h) = text.trim().parse::<u32>()
                    && h <= 23
                    && !hours.contains(&h)
                {
                    hours.push(h);
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"skipHours" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

/// Parse `<skipDays>` child `<day>` elements
fn parse_skip_days(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    limits: &ParserLimits,
    days: &mut Vec<String>,
) -> Result<()> {
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) if e.local_name().as_ref() == b"day" => {
                let text = read_text_str(reader, buf, limits)?;
                let day = text.trim().to_string();
                if !day.is_empty() && !days.contains(&day) {
                    days.push(day);
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"skipDays" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
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
        buf.clear();
    }

    Ok(owner)
}

/// Parse a `podcast:valueRecipient` element's attributes into a [`PodcastValueRecipient`]
fn parse_value_recipient_attrs(
    attrs: &[(Vec<u8>, String)],
    limits: &ParserLimits,
) -> PodcastValueRecipient {
    let name =
        find_attribute(attrs, b"name").map(|v| truncate_to_length(v, limits.max_attribute_length));
    let type_ = find_attribute(attrs, b"type")
        .map(|v| truncate_to_length(v, limits.max_attribute_length))
        .unwrap_or_default();
    let address = find_attribute(attrs, b"address")
        .map(|v| truncate_to_length(v, limits.max_attribute_length))
        .unwrap_or_default();
    let split = find_attribute(attrs, b"split")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);
    let fee = find_attribute(attrs, b"fee").and_then(|v| {
        if v.eq_ignore_ascii_case("true") {
            Some(true)
        } else if v.eq_ignore_ascii_case("false") {
            Some(false)
        } else {
            None
        }
    });
    PodcastValueRecipient {
        name,
        type_,
        address,
        split,
        fee,
    }
}

/// Parse a `podcast:valueTimeSplit` element into a [`PodcastValueTimeSplit`]
///
/// Consumes events up to and including the matching `</podcast:valueTimeSplit>`
/// End event (or Eof) regardless of whether `startTime`/`duration` were present,
/// then decides what to return — there is no early return once the loop starts.
///
/// A self-closing `<podcast:valueTimeSplit/>` has no matching End event (this
/// reader does not expand empty elements), so entering the loop for one would
/// read past the rest of the feed looking for an End that will never come.
/// The caller passes `is_empty` so this returns `Ok(None)` immediately without
/// touching the reader in that case.
fn parse_podcast_value_time_split(
    ctx: &mut XmlCtx,
    attrs: &[(Vec<u8>, String)],
    is_empty: bool,
    depth: usize,
) -> Result<Option<PodcastValueTimeSplit>> {
    if is_empty {
        return Ok(None);
    }

    let start_time = find_attribute(attrs, b"startTime")
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite());
    let duration = find_attribute(attrs, b"duration")
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite());
    let remote_start_time = find_attribute(attrs, b"remoteStartTime")
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite())
        .unwrap_or(0.0);
    let remote_percentage = find_attribute(attrs, b"remotePercentage")
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite())
        .unwrap_or(100.0)
        .clamp(0.0, 100.0);

    let mut recipients = Vec::with_capacity(2);
    let mut remote_item = None;

    loop {
        match ctx.reader.read_event_into(ctx.buf) {
            Ok(event @ (Event::Start(_) | Event::Empty(_))) => {
                let is_empty_child = matches!(event, Event::Empty(_));
                let (Event::Start(e) | Event::Empty(e)) = &event else {
                    unreachable!()
                };
                let child_tag = e.name();
                let next_depth = depth + 1;
                check_depth(next_depth, ctx.limits.max_nesting_depth)?;

                if child_tag.as_ref().starts_with(b"podcast:valueRecipient") {
                    let (recipient_attrs, _) = collect_attributes(e);
                    recipients.try_push_limited(
                        parse_value_recipient_attrs(&recipient_attrs, ctx.limits),
                        ctx.limits.max_value_recipients,
                    );
                } else if remote_item.is_none()
                    && child_tag.as_ref().starts_with(b"podcast:remoteItem")
                {
                    let (item_attrs, _) = collect_attributes(e);
                    remote_item = Some(parse_remote_item_attrs(&item_attrs, ctx.limits));
                } else if !is_empty_child {
                    skip_element(ctx.reader, ctx.buf, ctx.limits, next_depth)?;
                }
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"podcast:valueTimeSplit" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        ctx.buf.clear();
    }

    Ok(match (start_time, duration) {
        (Some(start_time), Some(duration)) => Some(PodcastValueTimeSplit {
            start_time,
            duration,
            remote_start_time,
            remote_percentage,
            recipients,
            remote_item,
        }),
        _ => None,
    })
}

/// Parse a `<podcast:value>` element's attributes and children into a [`PodcastValue`]
///
/// Shared by channel-level and item-level `podcast:value` parsing. Consumes events up to
/// and including the matching `</podcast:value>`, an `Eof`, or a propagated read/depth
/// error — whichever comes first. `depth` is the nesting depth of the `<podcast:value>`
/// element itself; children (recognized or not) are checked and, if unrecognized, skipped
/// at `depth + 1`, matching the `parse_image`/`parse_georss_where` convention used
/// elsewhere in this parser.
fn parse_podcast_value_content(
    ctx: &mut XmlCtx,
    attrs: &[(Vec<u8>, String)],
    depth: usize,
) -> Result<PodcastValue> {
    let type_ = find_attribute(attrs, b"type")
        .map(|v| truncate_to_length(v, ctx.limits.max_attribute_length))
        .unwrap_or_default();
    let method = find_attribute(attrs, b"method")
        .map(|v| truncate_to_length(v, ctx.limits.max_attribute_length))
        .unwrap_or_default();
    let suggested = find_attribute(attrs, b"suggested")
        .map(|v| truncate_to_length(v, ctx.limits.max_attribute_length));

    let mut recipients = Vec::with_capacity(2);
    let mut time_splits = Vec::with_capacity(2);

    loop {
        match ctx.reader.read_event_into(ctx.buf) {
            Ok(event @ (Event::Start(_) | Event::Empty(_))) => {
                let is_empty = matches!(event, Event::Empty(_));
                let (Event::Start(e) | Event::Empty(e)) = &event else {
                    unreachable!()
                };
                // NOTE: Allocation here is necessary due to borrow checker constraints
                // (see parse_channel/parse_item for the same pattern) — we need owned
                // tag/attribute data to call the sub-parsers with `reader`/`buf` free.
                let tag = e.name().as_ref().to_vec();
                let (child_attrs, _) = collect_attributes(e);

                let next_depth = depth + 1;
                check_depth(next_depth, ctx.limits.max_nesting_depth)?;

                if tag.starts_with(b"podcast:valueRecipient") {
                    recipients.try_push_limited(
                        parse_value_recipient_attrs(&child_attrs, ctx.limits),
                        ctx.limits.max_value_recipients,
                    );
                } else if tag.starts_with(b"podcast:valueTimeSplit") {
                    if let Some(split) =
                        parse_podcast_value_time_split(ctx, &child_attrs, is_empty, next_depth)?
                    {
                        time_splits
                            .try_push_limited(split, ctx.limits.max_podcast_value_time_splits);
                    }
                } else if !is_empty {
                    skip_element(ctx.reader, ctx.buf, ctx.limits, next_depth)?;
                }
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"podcast:value" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        ctx.buf.clear();
    }

    Ok(PodcastValue {
        type_,
        method,
        suggested,
        recipients,
        time_splits,
    })
}

/// Parse Podcast 2.0 value element from channel-level <podcast:value> element
///
/// Parses value-for-value payment information including payment type, method,
/// suggested amount, nested valueRecipient elements, and valueTimeSplit elements.
fn parse_podcast_value(
    ctx: &mut ChannelCtx,
    attrs: &[(Vec<u8>, String)],
    feed: &mut ParsedFeed,
    depth: usize,
) -> Result<()> {
    let value = parse_podcast_value_content(&mut ctx.xml, attrs, depth)?;
    let podcast = feed
        .feed
        .podcast
        .get_or_insert_with(|| Box::new(PodcastMeta::default()));
    podcast.value = Some(value);
    Ok(())
}

/// Parse Podcast 2.0 value element from item-level <podcast:value> element
///
/// Mirrors [`parse_podcast_value`] but stores the result on the entry's
/// [`PodcastEntryMeta`] instead of the feed-level [`PodcastMeta`].
fn parse_item_podcast_value(
    ctx: &mut EntryCtx,
    attrs: &[(Vec<u8>, String)],
    entry: &mut Entry,
    depth: usize,
) -> Result<()> {
    let value = parse_podcast_value_content(&mut ctx.xml, attrs, depth)?;
    let podcast = entry
        .podcast
        .get_or_insert_with(|| Box::new(PodcastEntryMeta::default()));
    podcast.value = Some(value);
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

    // Regression tests for #463: a malformed entity in a channel-level field must
    // not abort the whole parse — sibling <item>s must still be recovered.

    #[test]
    fn test_channel_title_malformed_entity_recovers_items() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title>Fish & Chips</title>
                <item><title>Item 1</title></item>
                <item><title>Item 2</title></item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(feed.bozo);
        assert!(feed.bozo_exception.is_some());
        assert_eq!(feed.entries.len(), 2);
        assert_eq!(feed.entries[0].title.as_deref(), Some("Item 1"));
        assert_eq!(feed.entries[1].title.as_deref(), Some("Item 2"));
    }

    #[test]
    fn test_channel_description_malformed_entity_recovers_items() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title>Test</title>
                <description>Fish & Chips</description>
                <item><title>Item 1</title></item>
                <item><title>Item 2</title></item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(feed.bozo);
        assert!(feed.bozo_exception.is_some());
        assert_eq!(feed.entries.len(), 2);
    }

    #[test]
    fn test_channel_image_malformed_entity_recovers_items() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title>Test</title>
                <image>
                    <url>http://example.com/img.png</url>
                    <title>Fish & Chips</title>
                </image>
                <item><title>Item 1</title></item>
                <item><title>Item 2</title></item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(feed.bozo);
        assert!(feed.bozo_exception.is_some());
        assert!(feed.feed.image.is_none());
        // #463 C1: the failed <image>'s own <title> ("Fish & Chips") must not leak
        // into the channel-level title.
        assert_eq!(feed.feed.title.as_deref(), Some("Test"));
        assert_eq!(feed.entries.len(), 2);
    }

    #[test]
    fn test_channel_self_closing_category_does_not_consume_following_items() {
        // A self-closing <category/> has no matching closing tag; the `category`
        // arm in `dispatch_channel_tag` must be guarded by `!is_empty` (like the
        // other standard-field arms) so it never attempts to read text or drain to
        // a `</category>` that doesn't exist, which would otherwise consume the
        // following well-formed <item>s looking for one.
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title>Test</title>
                <category domain="d"/>
                <item><title>Item 1</title></item>
                <item><title>Item 2</title></item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.entries.len(), 2);
        assert_eq!(feed.entries[0].title.as_deref(), Some("Item 1"));
        assert_eq!(feed.entries[1].title.as_deref(), Some("Item 2"));
    }

    #[test]
    fn test_channel_textinput_malformed_entity_does_not_leak_into_channel_fields() {
        // #463 C1: <textInput>'s own <title>/<description> must not overwrite the
        // real channel-level <title>/<description> after a malformed entity in an
        // earlier textInput field (<name>) aborts textInput parsing partway through.
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title>REAL</title>
                <description>REAL DESC</description>
                <textInput>
                    <name>a & b</name>
                    <title>TI TITLE</title>
                    <description>TI DESC</description>
                    <link>http://example.com</link>
                </textInput>
                <item><title>Item 1</title></item>
                <item><title>Item 2</title></item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(feed.bozo);
        assert!(feed.bozo_exception.is_some());
        assert_eq!(feed.feed.title.as_deref(), Some("REAL"));
        assert_eq!(feed.feed.subtitle.as_deref(), Some("REAL DESC"));
        assert_eq!(feed.entries.len(), 2);
    }

    #[test]
    fn test_channel_textinput_nested_same_name_does_not_stop_drain_early() {
        // #463 S4: a same-named nested <textInput> inside the failing <textInput>
        // must not stop the error-recovery drain at the inner </textInput>, which
        // would otherwise leak the outer textInput's own remaining <title>/
        // <description> into the channel-level fields.
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title>REAL</title>
                <description>REAL DESC</description>
                <textInput>
                    <name>a & b</name>
                    <textInput>junk</textInput>
                    <title>LEAKED TITLE</title>
                    <description>LEAKED DESC</description>
                    <link>http://example.com</link>
                </textInput>
                <item><title>Item 1</title></item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(feed.bozo);
        assert_eq!(feed.feed.title.as_deref(), Some("REAL"));
        assert_eq!(feed.feed.subtitle.as_deref(), Some("REAL DESC"));
        assert_eq!(feed.entries.len(), 1);
    }

    #[test]
    fn test_item_malformed_entity_does_not_leak_into_channel_fields() {
        // #463 S3: a malformed entity inside a normal <item> (not an over-limit one)
        // must not leak that item's own <title>/<link> into the channel-level
        // fields — the pre-existing defect the issue used item-level recovery as
        // the reference-correct behavior for, but which was itself still broken via
        // this path.
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title>REAL CHANNEL TITLE</title>
                <link>http://real-channel.example/</link>
                <item>
                    <description>ITEM & DESC</description>
                    <title>ITEM TITLE</title>
                    <link>http://item.link/</link>
                </item>
                <item><title>Item Two OK</title></item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(feed.bozo);
        assert_eq!(feed.feed.title.as_deref(), Some("REAL CHANNEL TITLE"));
        assert_eq!(
            feed.feed.link.as_deref(),
            Some("http://real-channel.example/")
        );
        assert_eq!(feed.entries.len(), 1);
        assert_eq!(feed.entries[0].title.as_deref(), Some("Item Two OK"));
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

    #[test]
    fn test_rss_itunes_owner_promotes_to_publisher_detail() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Test Podcast</title>
                <itunes:owner>
                    <itunes:name>Jane Smith</itunes:name>
                    <itunes:email>jane@example.com</itunes:email>
                </itunes:owner>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        let pub_detail = feed.feed.publisher_detail.as_ref().unwrap();
        assert_eq!(pub_detail.name.as_deref(), Some("Jane Smith"));
        assert_eq!(pub_detail.email.as_deref(), Some("jane@example.com"));
    }

    #[test]
    fn test_rss_itunes_owner_does_not_override_existing_publisher() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Test Podcast</title>
                <webMaster>webmaster@example.com (Web Master)</webMaster>
                <itunes:owner>
                    <itunes:name>iTunes Owner</itunes:name>
                    <itunes:email>owner@example.com</itunes:email>
                </itunes:owner>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        // webMaster sets publisher_detail first — itunes:owner must not override it
        let pub_detail = feed.feed.publisher_detail.as_ref().unwrap();
        assert_eq!(pub_detail.name.as_deref(), Some("Web Master"));
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
        assert_eq!(podcast.persons.len(), 2);
        assert_eq!(podcast.persons[0].name, "Jane Doe");
        assert_eq!(podcast.persons[0].role.as_deref(), Some("host"));
        assert_eq!(
            podcast.persons[0].href.as_deref(),
            Some("https://example.com/host")
        );
        assert_eq!(
            podcast.persons[0].img.as_deref(),
            Some("https://example.com/host.jpg")
        );
        assert_eq!(podcast.persons[1].name, "John Smith");
        assert_eq!(podcast.persons[1].role.as_deref(), Some("guest"));
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
        assert_eq!(podcast.persons.len(), 1);
        assert!(podcast.chapters.is_none());
        assert!(podcast.soundbite.is_empty());
    }

    #[test]
    fn test_podcast_season_episode_parsed() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 42</title>
                    <podcast:season number="3">Season Three</podcast:season>
                    <podcast:episode number="42">Bonus</podcast:episode>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        let entry = &feed.entries[0];
        let podcast = entry
            .podcast
            .as_deref()
            .expect("entry.podcast must be Some");
        // text content takes priority over number attribute
        assert_eq!(podcast.season.as_deref(), Some("Season Three"));
        assert_eq!(podcast.episode.as_deref(), Some("Bonus"));
    }

    #[test]
    fn test_podcast_season_episode_no_number_attr() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:transcript url="https://example.com/t.txt" type="text/plain"/>
                    <podcast:season>3</podcast:season>
                    <podcast:episode>42</podcast:episode>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        let entry = &feed.entries[0];
        // text content is used when number attribute is absent
        let podcast = entry
            .podcast
            .as_deref()
            .expect("entry.podcast must be Some due to transcript element");
        assert_eq!(podcast.season.as_deref(), Some("3"));
        assert_eq!(podcast.episode.as_deref(), Some("42"));
    }

    #[test]
    fn test_podcast_season_episode_bozo() {
        let xml = b"<rss version=\"2.0\" xmlns:podcast=\"https://podcastindex.org/namespace/1.0\"><channel><item><podcast:season number=\"3\">Season Three</channel></rss>";

        let feed = parse_rss20(xml).unwrap();
        assert!(feed.bozo);
    }

    #[test]
    fn test_podcast_season_episode_missing() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:transcript url="https://example.com/t.txt" type="text/plain"/>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        let entry = &feed.entries[0];
        let podcast = entry
            .podcast
            .as_deref()
            .expect("entry.podcast must be Some");
        assert!(podcast.season.is_none());
        assert!(podcast.episode.is_none());
    }

    #[test]
    fn test_podcast_location_channel() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Geo Feed</title>
                <podcast:location geo="geo:37.786971,-122.399677" osm="R113314">San Francisco</podcast:location>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let podcast = feed
            .feed
            .podcast
            .as_deref()
            .expect("podcast should be Some");
        let loc = podcast.location.as_ref().expect("location should be Some");
        assert_eq!(loc.name, "San Francisco");
        assert_eq!(loc.geo.as_deref(), Some("geo:37.786971,-122.399677"));
        assert_eq!(loc.osm.as_deref(), Some("R113314"));
    }

    #[test]
    fn test_podcast_location_channel_missing_name_skipped() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:location geo="geo:0,0"></podcast:location>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let podcast = feed.feed.podcast.as_deref();
        assert!(podcast.is_none_or(|p| p.location.is_none()));
    }

    #[test]
    fn test_podcast_podroll_channel() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:podroll>
                    <podcast:remoteItem feedGuid="abc123" feedUrl="https://example.com/feed.xml" title="Example Podcast"/>
                    <podcast:remoteItem feedGuid="def456" medium="podcast"/>
                </podcast:podroll>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let podcast = feed
            .feed
            .podcast
            .as_deref()
            .expect("podcast should be Some");
        assert_eq!(podcast.podroll.len(), 2);
        assert_eq!(podcast.podroll[0].feed_guid.as_deref(), Some("abc123"));
        assert_eq!(
            podcast.podroll[0].feed_url.as_deref(),
            Some("https://example.com/feed.xml")
        );
        assert_eq!(podcast.podroll[0].title.as_deref(), Some("Example Podcast"));
        assert_eq!(podcast.podroll[1].feed_guid.as_deref(), Some("def456"));
        assert_eq!(podcast.podroll[1].medium.as_deref(), Some("podcast"));
    }

    #[test]
    fn test_podcast_podroll_skips_items_without_guid_or_url() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:podroll>
                    <podcast:remoteItem medium="podcast"/>
                </podcast:podroll>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let podcast = feed.feed.podcast.as_deref();
        assert!(podcast.is_none_or(|p| p.podroll.is_empty()));
    }

    #[test]
    fn test_podcast_txt_channel() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:txt purpose="verify">abc123verify</podcast:txt>
                <podcast:txt>plain text record</podcast:txt>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let podcast = feed
            .feed
            .podcast
            .as_deref()
            .expect("podcast should be Some");
        assert_eq!(podcast.txt.len(), 2);
        assert_eq!(podcast.txt[0].purpose.as_deref(), Some("verify"));
        assert_eq!(podcast.txt[0].value, "abc123verify");
        assert!(podcast.txt[1].purpose.is_none());
        assert_eq!(podcast.txt[1].value, "plain text record");
    }

    #[test]
    fn test_podcast_update_frequency_channel() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:updateFrequency rrule="FREQ=WEEKLY" dtstart="2023-01-01T00:00:00Z" complete="false">weekly</podcast:updateFrequency>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let podcast = feed
            .feed
            .podcast
            .as_deref()
            .expect("podcast should be Some");
        let uf = podcast
            .update_frequency
            .as_ref()
            .expect("update_frequency should be Some");
        assert_eq!(uf.rrule.as_deref(), Some("FREQ=WEEKLY"));
        assert_eq!(uf.dtstart.as_deref(), Some("2023-01-01T00:00:00Z"));
        assert_eq!(uf.complete, Some(false));
        assert_eq!(uf.label.as_deref(), Some("weekly"));
    }

    #[test]
    fn test_podcast_follow_channel() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:follow url="https://mastodon.social/@podcast" platform="activitypub"/>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let podcast = feed
            .feed
            .podcast
            .as_deref()
            .expect("podcast should be Some");
        assert_eq!(podcast.follow.len(), 1);
        assert_eq!(podcast.follow[0].url, "https://mastodon.social/@podcast");
        assert_eq!(podcast.follow[0].platform.as_deref(), Some("activitypub"));
    }

    #[test]
    fn test_podcast_follow_channel_missing_url_skipped() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:follow platform="activitypub"/>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let podcast = feed.feed.podcast.as_deref();
        assert!(podcast.is_none_or(|p| p.follow.is_empty()));
    }

    #[test]
    fn test_podcast_alternate_enclosure_item() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:alternateEnclosure type="audio/mpeg" length="12345" bitrate="128" default="true">
                        <podcast:source uri="https://example.com/ep1.mp3"/>
                        <podcast:source uri="https://cdn.example.com/ep1.mp3" contentType="audio/mpeg"/>
                    </podcast:alternateEnclosure>
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let entry = &feed.entries[0];
        let podcast = entry
            .podcast
            .as_deref()
            .expect("entry podcast should be Some");
        assert_eq!(podcast.alternate_enclosures.len(), 1);
        let ae = &podcast.alternate_enclosures[0];
        assert_eq!(ae.type_.as_ref(), "audio/mpeg");
        assert_eq!(ae.length, Some(12345));
        assert_eq!(ae.bitrate, Some(128.0));
        assert_eq!(ae.default, Some(true));
        assert_eq!(ae.sources.len(), 2);
        assert_eq!(ae.sources[0].uri, "https://example.com/ep1.mp3");
        assert_eq!(ae.sources[1].uri, "https://cdn.example.com/ep1.mp3");
        assert_eq!(ae.sources[1].content_type.as_deref(), Some("audio/mpeg"));
    }

    #[test]
    fn test_podcast_alternate_enclosure_missing_type_skipped() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:alternateEnclosure length="12345">
                        <podcast:source uri="https://example.com/ep1.mp3"/>
                    </podcast:alternateEnclosure>
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let entry = &feed.entries[0];
        let podcast = entry.podcast.as_deref();
        assert!(podcast.is_none_or(|p| p.alternate_enclosures.is_empty()));
    }

    #[test]
    fn test_podcast_location_item() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:location geo="geo:40.7128,-74.0060">New York</podcast:location>
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let podcast = feed.entries[0]
            .podcast
            .as_deref()
            .expect("podcast should be Some");
        let loc = podcast.location.as_ref().expect("location should be Some");
        assert_eq!(loc.name, "New York");
        assert_eq!(loc.geo.as_deref(), Some("geo:40.7128,-74.0060"));
        assert!(loc.osm.is_none());
    }

    #[test]
    fn test_podcast_social_interact_item() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:socialInteract uri="https://mastodon.social/@host/status/1" protocol="activitypub" accountId="@host@mastodon.social" priority="1"/>
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let podcast = feed.entries[0]
            .podcast
            .as_deref()
            .expect("podcast should be Some");
        assert_eq!(podcast.social_interact.len(), 1);
        let si = &podcast.social_interact[0];
        assert_eq!(si.uri, "https://mastodon.social/@host/status/1");
        assert_eq!(si.protocol.as_deref(), Some("activitypub"));
        assert_eq!(si.account_id.as_deref(), Some("@host@mastodon.social"));
        assert_eq!(si.priority, Some(1));
    }

    #[test]
    fn test_podcast_social_interact_missing_uri_skipped() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:socialInteract protocol="activitypub"/>
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let podcast = feed.entries[0].podcast.as_deref();
        assert!(podcast.is_none_or(|p| p.social_interact.is_empty()));
    }

    #[test]
    fn test_podcast_txt_item() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:txt purpose="license">MIT</podcast:txt>
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let podcast = feed.entries[0]
            .podcast
            .as_deref()
            .expect("podcast should be Some");
        assert_eq!(podcast.txt.len(), 1);
        assert_eq!(podcast.txt[0].purpose.as_deref(), Some("license"));
        assert_eq!(podcast.txt[0].value, "MIT");
    }

    #[test]
    fn test_podcast_follow_item() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:follow url="https://twitter.com/podcast" platform="twitter"/>
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let podcast = feed.entries[0]
            .podcast
            .as_deref()
            .expect("podcast should be Some");
        assert_eq!(podcast.follow.len(), 1);
        assert_eq!(podcast.follow[0].url, "https://twitter.com/podcast");
        assert_eq!(podcast.follow[0].platform.as_deref(), Some("twitter"));
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

        // Empty xml:lang="" must be treated as None per XML spec
        if let Some(ref title_detail) = feed.feed.title_detail {
            assert!(title_detail.language.is_none());
        }

        assert_eq!(feed.entries.len(), 1);
        if let Some(ref title_detail) = feed.entries[0].title_detail {
            assert!(title_detail.language.is_none());
        }
    }

    #[test]
    fn test_parse_rss_xml_base_from_link() {
        // In RSS 2.0, xml:base on <channel> is not extracted by the current implementation.
        // base is populated from <link> instead.
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel xml:lang="en">
                <title>Base Test</title>
                <link>http://example.com/</link>
                <description>Description</description>
                <item>
                    <title>Item Title</title>
                    <description>Item Desc</description>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);

        // base is populated from <link>, not xml:base attribute
        let title_detail = feed.feed.title_detail.as_ref().unwrap();
        // Note: title is parsed before link, so base may be None on title_detail
        assert_eq!(title_detail.language.as_deref(), Some("en"));

        // subtitle_detail (description) is parsed after link -> should have base
        let subtitle_detail = feed.feed.subtitle_detail.as_ref().unwrap();
        assert_eq!(subtitle_detail.base.as_deref(), Some("http://example.com/"));
        assert_eq!(subtitle_detail.language.as_deref(), Some("en"));
    }

    #[test]
    fn test_parse_rss_content_encoded_carries_lang_and_base() {
        // content:encoded in an RSS item should carry the item-level lang and base.
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:content="http://purl.org/rss/1.0/modules/content/">
            <channel xml:lang="en" xml:base="http://example.com/">
                <title>Feed</title>
                <item xml:lang="fr">
                    <title>Item</title>
                    <content:encoded><![CDATA[<p>Content</p>]]></content:encoded>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);

        let entry = &feed.entries[0];
        assert!(
            !entry.content.is_empty(),
            "content:encoded should be parsed"
        );
        assert_eq!(entry.content[0].language.as_deref(), Some("fr"));
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
        assert_eq!(
            feed.feed.publisher.as_deref(),
            Some("webmaster@example.com (Web Master)")
        );
    }

    #[test]
    fn test_rss_webmaster_email_only() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title>T</title>
                <link>http://x.com</link>
                <webMaster>webmaster@example.com</webMaster>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);

        let pub_detail = feed.feed.publisher_detail.as_ref().unwrap();
        assert_eq!(pub_detail.email.as_deref(), Some("webmaster@example.com"));
        assert!(pub_detail.name.is_none());
        assert_eq!(
            feed.feed.publisher.as_deref(),
            Some("webmaster@example.com")
        );
    }

    #[test]
    fn test_rss_webmaster_missing() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title>T</title>
                <link>http://x.com</link>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        assert!(feed.feed.publisher.is_none());
        assert!(feed.feed.publisher_detail.is_none());
    }

    #[test]
    fn test_rss_webmaster_malformed_no_email() {
        // No @ sign — treated as plain name, publisher raw string still set
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title>T</title>
                <link>http://x.com</link>
                <webMaster>not-an-email</webMaster>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        assert_eq!(feed.feed.publisher.as_deref(), Some("not-an-email"));
        let pub_detail = feed.feed.publisher_detail.as_ref().unwrap();
        assert_eq!(pub_detail.name.as_deref(), Some("not-an-email"));
        assert!(pub_detail.email.is_none());
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

    // #297: itunes:owner.name must win over itunes:author for feed.author
    #[test]
    fn test_itunes_owner_name_wins_over_itunes_author() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Podcast</title>
                <itunes:author>Show Author</itunes:author>
                <itunes:owner>
                    <itunes:name>Owner Name</itunes:name>
                    <itunes:email>owner@example.com</itunes:email>
                </itunes:owner>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        // #317: feed.author must be name-only, no email in parentheses
        assert_eq!(
            feed.feed.author.as_deref(),
            Some("Owner Name"),
            "itunes:owner.name must win over itunes:author"
        );
        let detail = feed.feed.author_detail.as_ref().unwrap();
        assert_eq!(detail.name.as_deref(), Some("Owner Name"));
        assert_eq!(detail.email.as_deref(), Some("owner@example.com"));
    }

    // #297: itunes:owner before itunes:author — order must not matter
    #[test]
    fn test_itunes_owner_before_author_still_wins() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Podcast</title>
                <itunes:owner>
                    <itunes:name>Owner Name</itunes:name>
                    <itunes:email>owner@example.com</itunes:email>
                </itunes:owner>
                <itunes:author>Show Author</itunes:author>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        // #317: feed.author must be name-only
        assert_eq!(
            feed.feed.author.as_deref(),
            Some("Owner Name"),
            "itunes:owner.name must win regardless of element order"
        );
    }

    // #317: itunes:owner with name+email — feed.author is name only, author_detail has both
    #[test]
    fn test_itunes_owner_author_name_only_no_email() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Podcast</title>
                <itunes:owner>
                    <itunes:name>Jane Doe</itunes:name>
                    <itunes:email>jane@example.com</itunes:email>
                </itunes:owner>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert_eq!(
            feed.feed.author.as_deref(),
            Some("Jane Doe"),
            "feed.author must be name only — no email in parentheses"
        );
        let detail = feed.feed.author_detail.as_ref().unwrap();
        assert_eq!(detail.name.as_deref(), Some("Jane Doe"));
        assert_eq!(detail.email.as_deref(), Some("jane@example.com"));
    }

    // #287: itunes:image must override RSS <image>
    #[test]
    fn test_itunes_image_overrides_rss_image() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Podcast</title>
                <image>
                    <url>https://example.com/rss-logo.png</url>
                    <title>RSS Logo</title>
                    <link>https://example.com</link>
                </image>
                <itunes:image href="https://example.com/itunes-cover.jpg"/>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert_eq!(
            feed.feed.image.as_ref().map(|i| i.url.as_str()),
            Some("https://example.com/itunes-cover.jpg"),
            "itunes:image must override RSS <image>"
        );
    }

    // #287: itunes:image before RSS <image> — order must not matter
    #[test]
    fn test_itunes_image_before_rss_image_still_wins() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Podcast</title>
                <itunes:image href="https://example.com/itunes-cover.jpg"/>
                <image>
                    <url>https://example.com/rss-logo.png</url>
                    <title>RSS Logo</title>
                    <link>https://example.com</link>
                </image>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert_eq!(
            feed.feed.image.as_ref().map(|i| i.url.as_str()),
            Some("https://example.com/itunes-cover.jpg"),
            "itunes:image must win regardless of element order"
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
    fn test_itunes_subtitle_overrides_existing_feed_subtitle() {
        // itunes:subtitle always wins over <description> (post-processing is order-independent)
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Podcast</title>
                <description>Existing Subtitle</description>
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
    fn test_itunes_summary_does_not_promote_to_feed_subtitle() {
        // #308: itunes:summary must NOT set feed.subtitle — only itunes:subtitle does that.
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Podcast</title>
                <itunes:summary>iTunes Summary</itunes:summary>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.feed.subtitle.as_deref(), None);
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
    fn test_entry_itunes_summary_overrides_existing() {
        // itunes:summary always wins over <description> for entry.summary (post-processing)
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
        assert_eq!(entry.summary.as_deref(), Some("iTunes Summary"));
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

    // =========================================================================
    // Regression tests for #257, #281, #265
    // =========================================================================

    // TC-257-1: itunes:subtitle overrides <description> (description BEFORE subtitle)
    #[test]
    fn test_tc_257_1_itunes_subtitle_overrides_description_after() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <description>RSS description</description>
                <itunes:subtitle>iTunes subtitle</itunes:subtitle>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.feed.subtitle.as_deref(), Some("iTunes subtitle"));
        assert_eq!(
            feed.feed.itunes.as_ref().unwrap().subtitle.as_deref(),
            Some("iTunes subtitle")
        );
    }

    // TC-257-2: itunes:subtitle overrides even when it appears BEFORE description
    #[test]
    fn test_tc_257_2_itunes_subtitle_before_description_post_processing() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <itunes:subtitle>iTunes subtitle</itunes:subtitle>
                <description>RSS description</description>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert_eq!(
            feed.feed.subtitle.as_deref(),
            Some("iTunes subtitle"),
            "Post-processing must override <description> with itunes:subtitle"
        );
    }

    // TC-257-3: itunes:summary populates feed.summary, not feed.subtitle
    #[test]
    fn test_tc_257_3_itunes_summary_populates_feed_summary() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <description>RSS description</description>
                <itunes:summary>iTunes summary</itunes:summary>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.feed.summary.as_deref(), Some("iTunes summary"));
        assert_eq!(
            feed.feed.subtitle.as_deref(),
            Some("RSS description"),
            "subtitle stays as <description> when itunes:subtitle is absent"
        );
    }

    // TC-257-4: itunes:summary populates feed.summary only, never feed.subtitle (#308)
    #[test]
    fn test_tc_257_4_itunes_summary_only_sets_feed_summary() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <itunes:summary>iTunes summary</itunes:summary>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.feed.summary.as_deref(), Some("iTunes summary"));
        assert_eq!(
            feed.feed.subtitle.as_deref(),
            None,
            "itunes:summary must not set feed.subtitle — only itunes:subtitle does that"
        );
    }

    // TC-257-5: All three coexist — subtitle != summary != description
    #[test]
    fn test_tc_257_5_all_three_fields_coexist() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <description>Channel description</description>
                <itunes:subtitle>Podcast subtitle</itunes:subtitle>
                <itunes:summary>Podcast summary</itunes:summary>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.feed.subtitle.as_deref(), Some("Podcast subtitle"));
        assert_eq!(feed.feed.summary.as_deref(), Some("Podcast summary"));
    }

    // TC-257-6: Empty itunes:subtitle does NOT override valid <description>
    #[test]
    fn test_tc_257_6_empty_itunes_subtitle_no_override() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <description>Valid description</description>
                <itunes:subtitle></itunes:subtitle>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert_eq!(
            feed.feed.subtitle.as_deref(),
            Some("Valid description"),
            "Empty itunes:subtitle must not override valid <description>"
        );
    }

    // TC-257-7: Whitespace-only itunes:subtitle does NOT override valid <description>
    #[test]
    fn test_tc_257_7_whitespace_itunes_subtitle_no_override() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <description>Valid description</description>
                <itunes:subtitle>   </itunes:subtitle>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert_eq!(
            feed.feed.subtitle.as_deref(),
            Some("Valid description"),
            "Whitespace-only itunes:subtitle must not override valid <description>"
        );
    }

    // TC-257-8: Entry itunes:subtitle overrides <description> for entry.subtitle
    #[test]
    fn test_tc_257_8_entry_itunes_subtitle_overrides_description() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel><item>
                <description>Item description</description>
                <itunes:subtitle>Episode subtitle</itunes:subtitle>
            </item></channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let entry = &feed.entries[0];
        assert_eq!(entry.subtitle.as_deref(), Some("Episode subtitle"));
        // <description> maps to entry.summary, itunes:subtitle maps to entry.subtitle
        assert_eq!(entry.summary.as_deref(), Some("Item description"));
    }

    // TC-257-M3: entry itunes:summary overrides <description> for entry.summary
    #[test]
    fn test_tc_257_m3_entry_itunes_summary_overrides_description() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel><item>
                <description>Item description</description>
                <itunes:summary>Episode summary</itunes:summary>
            </item></channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        let entry = &feed.entries[0];
        assert_eq!(
            entry.summary.as_deref(),
            Some("Episode summary"),
            "itunes:summary must override <description> for entry.summary"
        );
    }

    // TC-281-1: itunes:complete 'Yes' returns raw string
    #[test]
    fn test_tc_281_1_itunes_complete_yes_raw_string() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel><title>P</title><itunes:complete>Yes</itunes:complete></channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert_eq!(
            feed.feed.itunes.as_ref().unwrap().complete.as_deref(),
            Some("Yes")
        );
    }

    // TC-281-2: itunes:complete 'no' returns raw string
    #[test]
    fn test_tc_281_2_itunes_complete_no_raw_string() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel><title>P</title><itunes:complete>no</itunes:complete></channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert_eq!(
            feed.feed.itunes.as_ref().unwrap().complete.as_deref(),
            Some("no")
        );
    }

    // TC-265-1: itunes:duration numeric stays as string (regression)
    #[test]
    fn test_tc_265_1_itunes_duration_numeric_stays_string() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel><item><itunes:duration>3600</itunes:duration></item></channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert_eq!(
            feed.entries[0].itunes.as_ref().unwrap().duration.as_deref(),
            Some("3600")
        );
    }

    // TC-265-2: itunes:duration H:M:S format stays as string (regression)
    #[test]
    fn test_tc_265_2_itunes_duration_hms_stays_string() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel><item><itunes:duration>1:00:00</itunes:duration></item></channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert_eq!(
            feed.entries[0].itunes.as_ref().unwrap().duration.as_deref(),
            Some("1:00:00")
        );
    }

    // TC-257-12: Integration — full podcast feed with all three fields
    #[test]
    fn test_tc_257_12_integration_full_podcast_feed() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <description>Channel description</description>
                <itunes:subtitle>Podcast subtitle</itunes:subtitle>
                <itunes:summary>Podcast summary</itunes:summary>
                <itunes:complete>Yes</itunes:complete>
                <item>
                    <description>Episode description</description>
                    <itunes:subtitle>Episode subtitle</itunes:subtitle>
                    <itunes:summary>Episode summary</itunes:summary>
                    <itunes:duration>1:23:45</itunes:duration>
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert_eq!(feed.feed.subtitle.as_deref(), Some("Podcast subtitle"));
        assert_eq!(feed.feed.summary.as_deref(), Some("Podcast summary"));
        assert_eq!(
            feed.feed.itunes.as_ref().unwrap().complete.as_deref(),
            Some("Yes")
        );
        let entry = &feed.entries[0];
        assert_eq!(entry.subtitle.as_deref(), Some("Episode subtitle"));
        assert_eq!(entry.summary.as_deref(), Some("Episode summary"));
        assert_eq!(
            entry.itunes.as_ref().unwrap().duration.as_deref(),
            Some("1:23:45")
        );
    }

    #[test]
    fn test_parse_rss_podcast_person_feed_level() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Test Podcast</title>
                <podcast:person role="host" img="https://example.com/host.jpg">Jane Doe</podcast:person>
                <podcast:person role="guest">John Smith</podcast:person>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        let persons = &feed.feed.podcast.as_ref().unwrap().persons;
        assert_eq!(persons.len(), 2);
        assert_eq!(persons[0].name, "Jane Doe");
        assert_eq!(persons[0].role.as_deref(), Some("host"));
        assert_eq!(
            persons[0].img.as_deref(),
            Some("https://example.com/host.jpg")
        );
        assert_eq!(persons[1].name, "John Smith");
        assert_eq!(persons[1].role.as_deref(), Some("guest"));
    }

    #[test]
    fn test_parse_rss_podcast_person_default_role_host() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Test Podcast</title>
                <item>
                    <title>Episode</title>
                    <podcast:person>Jane Doe</podcast:person>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        let person = &feed.entries[0].podcast_persons[0];
        assert_eq!(person.name, "Jane Doe");
        assert_eq!(person.role.as_deref(), Some("host"));
    }

    #[test]
    fn test_parse_rss_podcast_person_default_role_host_feed_level() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Test Podcast</title>
                <podcast:person>Jane Doe</podcast:person>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        let person = &feed.feed.podcast.as_ref().unwrap().persons[0];
        assert_eq!(person.name, "Jane Doe");
        assert_eq!(person.role.as_deref(), Some("host"));
    }

    #[test]
    fn test_parse_rss_podcast_medium() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Test Podcast</title>
                <podcast:medium>podcast</podcast:medium>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(
            feed.feed.podcast.as_ref().unwrap().medium.as_deref(),
            Some("podcast")
        );
    }

    #[test]
    fn test_parse_rss_podcast_medium_music() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Test Podcast</title>
                <podcast:medium>music</podcast:medium>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert_eq!(
            feed.feed.podcast.as_ref().unwrap().medium.as_deref(),
            Some("music")
        );
    }

    #[test]
    fn test_rss_itunes_category_maps_to_tags() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
            <channel>
                <title>Podcast</title>
                <link>http://example.com</link>
                <description>A podcast</description>
                <itunes:category text="Technology">
                    <itunes:category text="Software How-To"/>
                </itunes:category>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
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
    fn test_parse_rss_podcast_medium_entry_level() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Test Podcast</title>
                <item>
                    <title>Episode 1</title>
                    <podcast:medium>music</podcast:medium>
                </item>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        let podcast = feed.entries[0].podcast.as_deref().unwrap();
        assert_eq!(podcast.medium.as_deref(), Some("music"));
    }

    #[test]
    fn test_parse_rss_podcast_locked() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Test Podcast</title>
                <podcast:locked owner="owner@example.com">yes</podcast:locked>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        let podcast = feed.feed.podcast.as_ref().unwrap();
        assert_eq!(podcast.locked.as_deref(), Some("yes"));
        assert_eq!(podcast.locked_owner.as_deref(), Some("owner@example.com"));
    }

    #[test]
    fn test_parse_rss_podcast_locked_no_owner() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Test Podcast</title>
                <podcast:locked>no</podcast:locked>
            </channel>
        </rss>"#;

        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        let podcast = feed.feed.podcast.as_ref().unwrap();
        assert_eq!(podcast.locked.as_deref(), Some("no"));
        assert!(podcast.locked_owner.is_none());
    }

    #[test]
    fn test_rss20_custom_dc_prefix() {
        let xml = include_bytes!("../../../../tests/fixtures/rss/custom_ns_dc.xml");
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo, "bozo_exception: {:?}", feed.bozo_exception);
        // dc:creator at channel level should be parsed
        assert_eq!(feed.feed.author.as_deref(), Some("Test Author"));
        // dc:creator at item level should be parsed
        assert!(!feed.entries.is_empty());
        assert_eq!(feed.entries[0].author.as_deref(), Some("Item Author"));
        // mrss:thumbnail should be parsed
        assert!(!feed.entries[0].media_thumbnail.is_empty());
    }

    #[test]
    fn test_rss20_custom_itunes_prefix() {
        let xml = include_bytes!("../../../../tests/fixtures/rss/custom_ns_itunes.xml");
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo, "bozo_exception: {:?}", feed.bozo_exception);
        // it:author at channel level should be parsed
        let itunes = feed
            .feed
            .itunes
            .as_ref()
            .expect("itunes metadata should be present");
        assert_eq!(itunes.author.as_deref(), Some("John Doe"));
        // it:duration at item level should be parsed
        assert!(!feed.entries.is_empty());
        let entry_itunes = feed.entries[0]
            .itunes
            .as_ref()
            .expect("entry itunes metadata");
        assert_eq!(entry_itunes.duration.as_deref(), Some("30:00"));
        assert_eq!(entry_itunes.author.as_deref(), Some("Jane Smith"));
    }

    // --- podcast:chat / podcast:podping / podcast:valueTimeSplit (issue #437) ---

    #[test]
    fn test_podcast_chat_channel() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:chat server="matrix.example.com" protocol="matrix" accountId="@podcast:example.com" space="!room:example.com"/>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        let podcast = feed
            .feed
            .podcast
            .as_deref()
            .expect("podcast should be Some");
        assert_eq!(podcast.chat.len(), 1);
        assert_eq!(podcast.chat[0].server, "matrix.example.com");
        assert_eq!(podcast.chat[0].protocol, "matrix");
        assert_eq!(
            podcast.chat[0].account_id.as_deref(),
            Some("@podcast:example.com")
        );
        assert_eq!(podcast.chat[0].space.as_deref(), Some("!room:example.com"));
    }

    #[test]
    fn test_podcast_chat_item() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:chat server="xmpp.example.com" protocol="xmpp"/>
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        assert_eq!(feed.entries.len(), 1);
        let podcast = feed.entries[0]
            .podcast
            .as_deref()
            .expect("entry podcast should be Some");
        assert_eq!(podcast.chat.len(), 1);
        assert_eq!(podcast.chat[0].server, "xmpp.example.com");
        assert_eq!(podcast.chat[0].protocol, "xmpp");
        assert!(podcast.chat[0].account_id.is_none());
    }

    #[test]
    fn test_podcast_chat_channel_missing_server_skipped() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:chat protocol="matrix"/>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        let podcast = feed.feed.podcast.as_deref();
        assert!(podcast.is_none_or(|p| p.chat.is_empty()));
    }

    #[test]
    fn test_podcast_chat_channel_missing_protocol_skipped() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:chat server="matrix.example.com"/>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        let podcast = feed.feed.podcast.as_deref();
        assert!(podcast.is_none_or(|p| p.chat.is_empty()));
    }

    #[test]
    fn test_podcast_chat_channel_respects_limit() {
        let mut xml = String::from(
            r#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>"#,
        );
        for i in 0..10 {
            use std::fmt::Write;
            let _ = write!(
                xml,
                r#"<podcast:chat server="server{i}.example.com" protocol="matrix"/>"#
            );
        }
        xml.push_str("</channel></rss>");

        let limits = ParserLimits {
            max_podcast_chat: 3,
            ..Default::default()
        };
        let feed = parse_rss20_with_limits(xml.as_bytes(), limits).unwrap();
        let podcast = feed
            .feed
            .podcast
            .as_deref()
            .expect("podcast should be Some");
        assert_eq!(podcast.chat.len(), 3, "should respect max_podcast_chat");
    }

    #[test]
    fn test_podcast_podping_uses_podping_true() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:podping usesPodping="true"/>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        let podcast = feed
            .feed
            .podcast
            .as_deref()
            .expect("podcast should be Some");
        assert_eq!(podcast.podping_uses_podping, Some(true));
    }

    #[test]
    fn test_podcast_podping_unknown_value_is_none() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:podping usesPodping="maybe"/>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        let podcast = feed.feed.podcast.as_deref();
        assert!(podcast.is_none_or(|p| p.podping_uses_podping.is_none()));
    }

    #[test]
    fn test_podcast_value_time_split_with_recipients() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:value type="lightning" method="keysend">
                    <podcast:valueTimeSplit startTime="60" duration="30">
                        <podcast:valueRecipient type="node" address="addr1" split="100"/>
                    </podcast:valueTimeSplit>
                </podcast:value>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        let value = feed.feed.podcast.as_ref().unwrap().value.as_ref().unwrap();
        assert_eq!(value.time_splits.len(), 1);
        let split = &value.time_splits[0];
        assert!((split.start_time - 60.0).abs() < f64::EPSILON);
        assert!((split.duration - 30.0).abs() < f64::EPSILON);
        assert!((split.remote_percentage - 100.0).abs() < f64::EPSILON);
        assert_eq!(split.recipients.len(), 1);
        assert_eq!(split.recipients[0].address, "addr1");
        assert!(split.remote_item.is_none());
    }

    #[test]
    fn test_podcast_value_time_split_with_remote_item() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:value type="lightning" method="keysend">
                    <podcast:valueTimeSplit startTime="10" duration="20" remoteStartTime="5" remotePercentage="50">
                        <podcast:remoteItem itemGuid="abc123"/>
                    </podcast:valueTimeSplit>
                </podcast:value>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        let value = feed.feed.podcast.as_ref().unwrap().value.as_ref().unwrap();
        assert_eq!(value.time_splits.len(), 1);
        let split = &value.time_splits[0];
        assert!((split.remote_start_time - 5.0).abs() < f64::EPSILON);
        assert!((split.remote_percentage - 50.0).abs() < f64::EPSILON);
        assert!(split.recipients.is_empty());
        let remote_item = split
            .remote_item
            .as_ref()
            .expect("remote_item should be Some");
        assert_eq!(remote_item.item_guid.as_deref(), Some("abc123"));
    }

    #[test]
    fn test_podcast_value_time_split_missing_start_time_dropped() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:value type="lightning" method="keysend">
                    <podcast:valueTimeSplit duration="30">
                        <podcast:valueRecipient type="node" address="addr1" split="100"/>
                    </podcast:valueTimeSplit>
                </podcast:value>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        let value = feed.feed.podcast.as_ref().unwrap().value.as_ref().unwrap();
        assert!(value.time_splits.is_empty());
        assert_eq!(value.type_, "lightning");
    }

    #[test]
    fn test_podcast_value_time_split_stray_channel_level_ignored() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:valueTimeSplit startTime="1" duration="2"></podcast:valueTimeSplit>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo);
        let podcast = feed.feed.podcast.as_deref();
        assert!(podcast.is_none_or(|p| p.value.is_none()));
    }

    #[test]
    fn test_podcast_value_time_split_self_closing_does_not_swallow_feed() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:value type="lightning" method="keysend">
                    <podcast:valueTimeSplit startTime="1" duration="2"/>
                    <podcast:valueRecipient type="node" address="addr1" split="100"/>
                </podcast:value>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo, "bozo_exception: {:?}", feed.bozo_exception);
        let value = feed.feed.podcast.as_ref().unwrap().value.as_ref().unwrap();
        assert!(
            value.time_splits.is_empty(),
            "self-closing valueTimeSplit carries no children, so it is dropped"
        );
        assert_eq!(
            value.recipients.len(),
            1,
            "the recipient after the self-closing split must still be collected"
        );
    }

    #[test]
    fn test_podcast_value_time_split_nested_explicit_close_pins_tolerant_behavior() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:value type="lightning" method="keysend">
                    <podcast:valueTimeSplit startTime="1" duration="2">
                        <podcast:valueTimeSplit startTime="10" duration="20"></podcast:valueTimeSplit>
                    </podcast:valueTimeSplit>
                    <podcast:valueRecipient type="node" address="addrX" split="100"/>
                </podcast:value>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo, "bozo_exception: {:?}", feed.bozo_exception);
        let value = feed.feed.podcast.as_ref().unwrap().value.as_ref().unwrap();
        assert_eq!(
            value.time_splits.len(),
            1,
            "the inner End closes the outer split; nested Start is ignored"
        );
        assert!((value.time_splits[0].start_time - 1.0).abs() < f64::EPSILON);
        assert!((value.time_splits[0].duration - 2.0).abs() < f64::EPSILON);
        assert_eq!(
            value.recipients.len(),
            1,
            "trailing content after the nested split leaks back and is still parsed"
        );
    }

    #[test]
    fn test_podcast_value_time_split_respects_limit() {
        let mut xml = String::from(
            r#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:value type="lightning" method="keysend">"#,
        );
        for i in 0..10 {
            use std::fmt::Write;
            let _ = write!(
                xml,
                r#"<podcast:valueTimeSplit startTime="{i}" duration="10"></podcast:valueTimeSplit>"#
            );
        }
        xml.push_str("</podcast:value></channel></rss>");

        let limits = ParserLimits {
            max_podcast_value_time_splits: 4,
            ..Default::default()
        };
        let feed = parse_rss20_with_limits(xml.as_bytes(), limits).unwrap();
        let value = feed.feed.podcast.as_ref().unwrap().value.as_ref().unwrap();
        assert_eq!(
            value.time_splits.len(),
            4,
            "should respect max_podcast_value_time_splits"
        );
    }

    #[test]
    fn test_podcast_value_end_guard_regression_two_time_splits_then_recipient() {
        // Regression: the end-guard for </podcast:value> used to be a
        // starts_with prefix match, which also matched
        // </podcast:valueTimeSplit>, truncating the value block after the
        // first explicitly-closed split.
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:value type="lightning" method="keysend">
                    <podcast:valueTimeSplit startTime="1" duration="2"></podcast:valueTimeSplit>
                    <podcast:valueTimeSplit startTime="3" duration="4"></podcast:valueTimeSplit>
                    <podcast:valueRecipient type="node" address="addr1" split="100"/>
                </podcast:value>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo, "bozo_exception: {:?}", feed.bozo_exception);
        let value = feed.feed.podcast.as_ref().unwrap().value.as_ref().unwrap();
        assert_eq!(value.time_splits.len(), 2);
        assert_eq!(value.recipients.len(), 1);
    }

    #[test]
    fn test_podcast_value_end_guard_regression_explicit_closed_recipients() {
        // Regression (pre-existing bug, predates this feature): the same
        // prefix-match end guard also matched </podcast:valueRecipient>,
        // so an explicitly-closed recipient truncated the value block
        // before a following sibling recipient was parsed.
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:value type="lightning" method="keysend">
                    <podcast:valueRecipient type="node" address="addr1" split="50"></podcast:valueRecipient>
                    <podcast:valueRecipient type="node" address="addr2" split="50"/>
                </podcast:value>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo, "bozo_exception: {:?}", feed.bozo_exception);
        let value = feed.feed.podcast.as_ref().unwrap().value.as_ref().unwrap();
        assert_eq!(value.recipients.len(), 2);
    }

    // --- item-level podcast:value / podcast:valueTimeSplit (issue #466) ---

    #[test]
    fn test_podcast_value_item_lightning() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
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
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo, "bozo_exception: {:?}", feed.bozo_exception);
        assert_eq!(feed.entries.len(), 1);

        let podcast = feed.entries[0]
            .podcast
            .as_deref()
            .expect("entry podcast should be Some");
        let value = podcast.value.as_ref().expect("value should be Some");

        assert_eq!(value.type_, "lightning");
        assert_eq!(value.method, "keysend");
        assert_eq!(value.suggested.as_deref(), Some("0.00000005000"));
        assert_eq!(value.recipients.len(), 2);
        assert_eq!(value.recipients[0].name.as_deref(), Some("Host"));
        assert_eq!(value.recipients[0].split, 90);
        assert_eq!(value.recipients[1].name.as_deref(), Some("Producer"));
        assert_eq!(value.recipients[1].split, 10);

        // Channel-level podcast metadata is untouched by item-level parsing.
        assert!(feed.feed.podcast.is_none_or(|p| p.value.is_none()));
    }

    #[test]
    fn test_podcast_value_item_time_split_with_remote_item() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:value type="lightning" method="keysend">
                        <podcast:valueTimeSplit startTime="10" duration="20" remoteStartTime="5" remotePercentage="50">
                            <podcast:remoteItem itemGuid="abc123"/>
                        </podcast:valueTimeSplit>
                        <podcast:valueRecipient type="node" address="addr1" split="100"/>
                    </podcast:value>
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo, "bozo_exception: {:?}", feed.bozo_exception);

        let value = feed.entries[0]
            .podcast
            .as_deref()
            .and_then(|p| p.value.as_ref())
            .expect("entry value should be Some");

        assert_eq!(value.time_splits.len(), 1);
        let split = &value.time_splits[0];
        assert!((split.start_time - 10.0).abs() < f64::EPSILON);
        assert!((split.duration - 20.0).abs() < f64::EPSILON);
        assert!((split.remote_start_time - 5.0).abs() < f64::EPSILON);
        assert!((split.remote_percentage - 50.0).abs() < f64::EPSILON);
        let remote_item = split
            .remote_item
            .as_ref()
            .expect("remote_item should be Some");
        assert_eq!(remote_item.item_guid.as_deref(), Some("abc123"));

        assert_eq!(value.recipients.len(), 1);
        assert_eq!(value.recipients[0].address, "addr1");
    }

    #[test]
    fn test_podcast_value_item_time_split_missing_start_time_dropped() {
        // Malformed valueTimeSplit (missing required startTime) must not panic
        // or set bozo — it is simply dropped, matching channel-level tolerance.
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:value type="lightning" method="keysend">
                        <podcast:valueTimeSplit duration="30">
                            <podcast:valueRecipient type="node" address="addr1" split="100"/>
                        </podcast:valueTimeSplit>
                    </podcast:value>
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo, "bozo_exception: {:?}", feed.bozo_exception);

        let value = feed.entries[0]
            .podcast
            .as_deref()
            .and_then(|p| p.value.as_ref())
            .expect("entry value should be Some");
        assert!(value.time_splits.is_empty());
        assert_eq!(value.type_, "lightning");
    }

    #[test]
    fn test_podcast_value_item_respects_limits() {
        let mut xml = String::from(
            r#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:value type="lightning" method="keysend">"#,
        );
        for i in 0..25 {
            use std::fmt::Write;
            let _ = write!(
                xml,
                r#"<podcast:valueRecipient type="node" address="addr_{i}" split="4"/>"#
            );
        }
        xml.push_str("</podcast:value></item></channel></rss>");

        let limits = ParserLimits {
            max_value_recipients: 5,
            ..Default::default()
        };
        let feed = parse_rss20_with_limits(xml.as_bytes(), limits).unwrap();
        let value = feed.entries[0]
            .podcast
            .as_deref()
            .and_then(|p| p.value.as_ref())
            .expect("entry value should be Some");
        assert_eq!(
            value.recipients.len(),
            5,
            "should respect max_value_recipients limit for item-level value"
        );
    }

    #[test]
    fn test_podcast_value_item_time_split_respects_limit() {
        let mut xml = String::from(
            r#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:value type="lightning" method="keysend">"#,
        );
        for i in 0..10 {
            use std::fmt::Write;
            let _ = write!(
                xml,
                r#"<podcast:valueTimeSplit startTime="{i}" duration="10"></podcast:valueTimeSplit>"#
            );
        }
        xml.push_str("</podcast:value></item></channel></rss>");

        let limits = ParserLimits {
            max_podcast_value_time_splits: 4,
            ..Default::default()
        };
        let feed = parse_rss20_with_limits(xml.as_bytes(), limits).unwrap();
        let value = feed.entries[0]
            .podcast
            .as_deref()
            .and_then(|p| p.value.as_ref())
            .expect("entry value should be Some");
        assert_eq!(
            value.time_splits.len(),
            4,
            "should respect max_podcast_value_time_splits limit for item-level value"
        );
    }

    #[test]
    fn test_podcast_value_item_no_leak_between_items() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:value type="lightning" method="keysend">
                        <podcast:valueRecipient type="node" address="addr1" split="100"/>
                    </podcast:value>
                </item>
                <item>
                    <title>Episode 2</title>
                    <podcast:value type="hive" method="keysend">
                        <podcast:valueRecipient type="node" address="addr2" split="50"/>
                        <podcast:valueRecipient type="node" address="addr3" split="50"/>
                    </podcast:value>
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo, "bozo_exception: {:?}", feed.bozo_exception);
        assert_eq!(feed.entries.len(), 2);

        let value1 = feed.entries[0]
            .podcast
            .as_deref()
            .and_then(|p| p.value.as_ref())
            .expect("entry 1 value should be Some");
        let value2 = feed.entries[1]
            .podcast
            .as_deref()
            .and_then(|p| p.value.as_ref())
            .expect("entry 2 value should be Some");

        assert_eq!(value1.type_, "lightning");
        assert_eq!(value1.recipients.len(), 1);
        assert_eq!(value1.recipients[0].address, "addr1");

        assert_eq!(value2.type_, "hive");
        assert_eq!(value2.recipients.len(), 2);
        assert_eq!(value2.recipients[0].address, "addr2");
        assert_eq!(value2.recipients[1].address, "addr3");
    }

    #[test]
    fn test_podcast_value_item_absent_is_none() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:season number="2"/>
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo, "bozo_exception: {:?}", feed.bozo_exception);
        let podcast = feed.entries[0]
            .podcast
            .as_deref()
            .expect("entry podcast should be Some (season was set)");
        assert!(podcast.value.is_none());
    }

    #[test]
    fn test_podcast_value_item_self_closing_is_none() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:value/>
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo, "bozo_exception: {:?}", feed.bozo_exception);
        let podcast = feed.entries[0].podcast.as_deref();
        assert!(podcast.is_none_or(|p| p.value.is_none()));
    }

    #[test]
    fn test_podcast_value_item_duplicate_last_wins() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:value type="lightning" method="keysend">
                        <podcast:valueRecipient type="node" address="first" split="100"/>
                    </podcast:value>
                    <podcast:value type="hive" method="keysend">
                        <podcast:valueRecipient type="node" address="second" split="100"/>
                    </podcast:value>
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo, "bozo_exception: {:?}", feed.bozo_exception);
        let value = feed.entries[0]
            .podcast
            .as_deref()
            .and_then(|p| p.value.as_ref())
            .expect("entry value should be Some");
        assert_eq!(value.type_, "hive", "the last podcast:value should win");
        assert_eq!(value.recipients[0].address, "second");
    }

    #[test]
    fn test_podcast_value_time_split_stray_item_level_ignored() {
        let xml = br#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:valueTimeSplit startTime="1" duration="2"></podcast:valueTimeSplit>
                </item>
            </channel>
        </rss>"#;
        let feed = parse_rss20(xml).unwrap();
        assert!(!feed.bozo, "bozo_exception: {:?}", feed.bozo_exception);
        let podcast = feed.entries[0].podcast.as_deref();
        assert!(podcast.is_none_or(|p| p.value.is_none()));
    }

    #[test]
    fn test_podcast_value_item_deep_nesting_sets_bozo() {
        // Regression (C1): deeply nested unrecognized children under an item-level
        // <podcast:value> must be depth-checked and set bozo, exactly like every
        // other recursive sub-parser in this module (parse_image, parse_georss_where,
        // etc.) — not silently accepted as an unbounded-nesting bypass.
        let mut xml = String::from(
            r#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <item>
                    <title>Episode 1</title>
                    <podcast:value type="lightning" method="keysend">"#,
        );
        for _ in 0..200 {
            xml.push_str("<x>");
        }
        for _ in 0..200 {
            xml.push_str("</x>");
        }
        xml.push_str("</podcast:value></item></channel></rss>");

        let limits = ParserLimits {
            max_nesting_depth: 100,
            ..Default::default()
        };
        let feed = parse_rss20_with_limits(xml.as_bytes(), limits).unwrap();
        assert!(
            feed.bozo,
            "deep nesting under item-level podcast:value must set bozo"
        );
        assert!(
            feed.bozo_exception
                .as_deref()
                .is_some_and(|e| e.contains("nesting depth")),
            "bozo_exception should mention nesting depth: {:?}",
            feed.bozo_exception
        );
    }

    #[test]
    fn test_podcast_value_channel_deep_nesting_sets_bozo() {
        // Companion to the item-level C1 regression test above: the shared
        // parse_podcast_value_content helper must enforce the same depth limit
        // regardless of which call site (channel or item) invokes it.
        let mut xml = String::from(
            r#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
            <channel>
                <title>Feed</title>
                <podcast:value type="lightning" method="keysend">"#,
        );
        for _ in 0..200 {
            xml.push_str("<x>");
        }
        for _ in 0..200 {
            xml.push_str("</x>");
        }
        xml.push_str("</podcast:value></channel></rss>");

        let limits = ParserLimits {
            max_nesting_depth: 100,
            ..Default::default()
        };
        let feed = parse_rss20_with_limits(xml.as_bytes(), limits).unwrap();
        assert!(
            feed.bozo,
            "deep nesting under channel-level podcast:value must set bozo"
        );
        assert!(
            feed.bozo_exception
                .as_deref()
                .is_some_and(|e| e.contains("nesting depth")),
            "bozo_exception should mention nesting depth: {:?}",
            feed.bozo_exception
        );
    }
}
