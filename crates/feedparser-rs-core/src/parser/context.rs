//! Shared parse-context types.
//!
//! Bundles the XML event-loop plumbing (`XmlCtx`) and per-item/per-entry
//! parsing state (`EntryCtx`) that would otherwise be threaded as separate
//! parameters through every RSS/Atom/RSS 1.0 helper function, keeping their
//! signatures under clippy's `too_many_arguments` threshold.

use std::collections::HashMap;

use quick_xml::Reader;

use crate::ParserLimits;
use crate::util::base_url::BaseUrlContext;

/// XML event-loop plumbing shared by every parser helper.
///
/// Carries the reader, its reusable event buffer, and the active
/// [`ParserLimits`]. Every helper that consumes XML events borrows the
/// `reader`/`buf` pair from here instead of taking them as separate
/// parameters.
pub struct XmlCtx<'r, 'd> {
    /// The XML event reader positioned at the current parse location.
    pub reader: &'r mut Reader<&'d [u8]>,
    /// Reusable buffer for `reader.read_event_into`; cleared between events.
    pub buf: &'r mut Vec<u8>,
    /// Parser resource limits (nesting depth, text length, entry count, ...).
    pub limits: &'r ParserLimits,
}

impl<'d> XmlCtx<'_, 'd> {
    /// Reborrow this context for a nested call.
    ///
    /// Required because [`EntryCtx`] holds `XmlCtx` by value while the
    /// caller must retain use of its own `reader`/`buf` afterwards.
    pub const fn reborrow(&mut self) -> XmlCtx<'_, 'd> {
        XmlCtx {
            reader: &mut *self.reader,
            buf: &mut *self.buf,
            limits: self.limits,
        }
    }
}

/// Per-item (RSS/RSS 1.0) or per-entry (Atom) parsing context.
///
/// Built at the top of `parse_item`/`parse_entry` — never by the caller,
/// since the caller holds `feed: &mut ParsedFeed` and this context borrows
/// `&feed.namespaces` for its lifetime. Carries the read-only context
/// (namespaces, xml:base, xml:lang) plus accumulators written while walking
/// an item/entry's children.
pub struct EntryCtx<'r, 'd, 'p> {
    /// XML event-loop plumbing (reader, buffer, limits).
    pub xml: XmlCtx<'r, 'd>,
    /// The xml:base resolution context inherited from the enclosing feed/channel.
    pub base: &'p BaseUrlContext,
    /// The effective xml:lang for this item/entry, if any.
    pub lang: Option<&'p str>,
    /// Declared namespace prefix -> URI mapping for the whole feed.
    pub namespaces: &'p HashMap<String, String>,
    /// Entity-resolution bozo accumulator, returned to the caller once the
    /// item/entry has been fully parsed.
    pub bozo: bool,
    /// RSS 2.0 only: whether an explicit `<link>` element was seen (unused
    /// by Atom and RSS 1.0, which have no equivalent guid-fallback rule).
    pub has_explicit_link: bool,
    /// RSS 2.0 only: the `isPermaLink` attribute of `<guid>`, if present
    /// (unused by Atom and RSS 1.0).
    pub guid_is_permalink: Option<bool>,
}
