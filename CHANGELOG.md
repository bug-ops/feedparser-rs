# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Core: syndication module (`syn:`/`sy:` namespace) is now parsed in RSS 2.0 feeds; previously only RSS 1.0 feeds were supported — RSS 2.0 feeds with `<syn:updatePeriod>` etc. returned `feed.syndication = None` (#237)
- Core: `syn:updateFrequency` / `sy:updateFrequency` now returns the raw string value (e.g. `"2"`) instead of an integer, matching Python feedparser behavior (#268, #220)
- Python bindings: expose `thr:in-reply-to` as `entry['thr_in-reply-to']` returning the first element as a plain dict with keys `ref`, `href`, `type`, `source` (non-None only), matching Python feedparser API; `entry.thr_in_reply_to` (underscore) retains the full list of `InReplyTo` objects (#267, #245)
- Core, Python, Node.js bindings: Atom `<source><link href="..."/>` now populates `entry.source.link` (new field); `entry.source.href` remains for RSS `<source url="...">` only (#262)
- Core, Python, Node.js bindings: Atom `<source><author>` is now exposed as `entry.source.author` flat string in `"Name (email)"` format (#262)
- Core, Python, Node.js bindings: Atom `<content src="...">` (out-of-line content per RFC 4287 §4.1.3.2) is now parsed — `content.src` is set to the URL, `content.value` is empty string, `content.type` is set from the `type` attribute (#252)
- Core, Python, Node.js bindings: Atom flat `author` string now uses `"Name (email)"` format when email is present; previously only the name was used (#251)

### Changed

- **BREAKING**: `entry.source.link` renamed to `entry.source.href` in core Rust type and Node.js bindings for Python feedparser API compatibility; Python binding retains `source.link` as an alias for `source.href` (#240)

### Added

- Core, Python, Node.js bindings: `entry.source` now exposes `links` (all link elements), `updated`/`updated_parsed`, `rights`, and `guidislink` fields for Atom `<source>` elements, matching Python feedparser (#242, #214)
- Core: `entry.source.guidislink` is `Some(true)` when the Atom `<source>` `<id>` looks like a URL and no explicit `<link>` is present; `Some(false)` when an explicit `<link>` is present or the id is not a URL; `None` for RSS sources

### Fixed

- Core: Atom entries without an explicit `<link>` now have `entry.link` promoted from `entry.id`, and `entry.guidislink` set to `true`; when an explicit `<link>` is present, `entry.guidislink` is `false`, matching Python feedparser behavior (#273)
- Core: Atom feeds without an explicit `<link>` now have `feed.link` promoted from `feed.id`, matching Python feedparser behavior (#274)
- Core: Atom entries with `<published>` but no `<updated>` now have `entry.updated` and `entry.updated_str` set from `entry.published`, matching Python feedparser behavior (#275)
- Core, Python, Node.js bindings: `feed.image` now exposes `url` (alias for `href`), `description` (alias for `subtitle`), `subtitle_detail` and `title_detail` (`TextConstruct` with `type='text/plain'`), and `links` (synthesized `[{rel:'alternate', href:<url>}]`) to match Python feedparser API (#216, #239, #276)
- Core, Python, Node.js bindings: `itunes_duration` now returns the raw XML string (e.g. `"1:23:45"`, `"83:45"`, `"5025"`) instead of converting to seconds as an integer; `itunes_episode` and `itunes_season` now return strings (e.g. `"42"`, `"3"`) instead of integers, matching Python feedparser behavior (#224, #225)
- RSS `<pubDate>` now mirrored to `entry.updated`/`entry.updated_parsed` and `feed.updated`/`feed.updated_parsed` when no other update date is present (#201, #250)
- `dc:date` now takes precedence over `pubDate`-promoted `updated` field when both are present
- Core: RSS `entry.guidislink` is now `false` when an explicit `<link>` element is present in the item, regardless of `isPermaLink` attribute value; element order (`<guid>` before or after `<link>`) no longer affects the result (#231)
- Core: Atom `entry.guidislink` is now `Some(false)` when `<id>` is present (was always `None`), matching Python feedparser behavior (#256)
- Python: all nested struct types (Image, Enclosure, Link, Person, TextConstruct, Generator, Tag, Source, Content, MediaThumbnail, MediaContent, ItunesFeedMeta, ItunesEntryMeta, GeoLocation) now implement the full dict protocol: `get()`, `keys()`, `values()`, `items()`, `dict()`, `in` operator, and `__getitem__` (#264, #222)
- Python: geo location field renamed from `where_` to `where` to match Python feedparser API (`entry.where`, `feed.where`) (#249)

## [0.5.0] - 2026-03-24

### Fixed
- Core: RSS `<enclosure>` elements are now added to both `entry.enclosures` and `entry.links` (with `rel='enclosure'`), matching Python feedparser behavior (#192)
- Core, Python, Node.js bindings: Atom parser now recognizes the `itunes:` namespace; `feed.itunes`, `feed.image` (promoted from `itunes:image`), and per-entry `entry.itunes` fields are now populated in Atom feeds, matching existing RSS behavior (#194)
- Core, Python, Node.js bindings: `itunes:block` is now parsed at feed level and exposed as `itunes.block` (integer: `1` for "yes", `0` otherwise); `itunes:complete`, `itunes:type`, and `itunes:new-feed-url` were already supported in RSS and are now also recognized in Atom feeds (#233)
- Core, Python, Node.js bindings: `itunes_explicit` now returns `None` (`null` / `None`) for "no"/"false"/"clean" and absent values, and `True` only for "yes"/"true"/"explicit"; previously returned `False` for "no", which does not match Python feedparser behavior (#234)
- Core: syndication namespace elements (`updatePeriod`, `updateFrequency`, `updateBase`) are now recognized with both `sy:` and `syn:` prefixes; previously only `syn:` was recognized, causing `feed.syndication` to return `None` for feeds using the more common `sy:` prefix (#191)
- Core, Python, Node.js bindings: GeoRSS location is now exposed as `entry.where` / `entry.where_` (was `entry.geo`), matching Python feedparser field name; coordinates now follow GeoJSON order `(lon, lat)` (was `(lat, lon)`); type names use GeoJSON capitalization (`Point`, `LineString`, `Polygon`); Python and Node.js bindings return a dict `{'type': 'Point', 'coordinates': (lon, lat)}` instead of a custom GeoLocation object (#185)
- Core, Python, Node.js bindings: `MediaContent` now exposes `bitrate`, `lang`, `channels`, `codec`, `expression`, `isdefault`, `samplingrate` attributes; `duration` changed from integer to raw string matching Python feedparser behavior (#190)
- Core: `itunes:author`, `itunes:subtitle`, and `itunes:summary` are now promoted to the corresponding standard feed/entry fields (`author`/`author_detail`/`authors`, `subtitle`/`subtitle_detail`, `summary`/`summary_detail`) when those fields are absent; existing standard-field values are never overwritten (#188)
- Core: `entry.podcast.transcript` and `entry.podcast.person` are now populated from the same data as `entry.podcast_transcripts` and `entry.podcast_persons`; `entry.podcast` is now non-None whenever any Podcast 2.0 namespace element (transcript, person, soundbite, or chapters) is present (#183)
- Core, Python, Node.js bindings: expose `entry.guidislink` (`bool`) indicating whether an RSS `<guid>` has `isPermaLink="true"` (or attribute absent, which defaults to true per RSS 2.0 spec); when `guidislink` is true and no `<link>` element is present, `entry.link` now falls back to the guid URL, matching Python feedparser behavior (#179)
- Core: `<media:content>` elements nested inside a `<media:group>` wrapper are now parsed into `entry.media_content` (and `entry.media_thumbnail` for `<media:thumbnail>`) for both RSS and Atom feeds; previously only top-level `<media:content>` elements were recognized (#184)
- Core: `dc:date` in RSS 1.0 (RDF) feeds now maps to `entry.updated`/`entry.updated_parsed` instead of `entry.published`/`entry.published_parsed`, matching Python feedparser behavior (#175)
- Core, Python, Node.js bindings: `feed.ttl` now returns a `str` instead of an integer, matching Python feedparser behavior; `feed.docs` field added to expose the RSS `<docs>` channel element (#181)
- Core, Python, Node.js bindings: `Link.length`, `Enclosure.length`, `MediaContent.width`, `MediaContent.height`, `MediaThumbnail.width`, and `MediaThumbnail.height` now return `str` (raw XML attribute value) instead of an integer, matching Python feedparser behavior; non-numeric values are preserved as-is rather than silently dropped (#173)
- Core: JSON Feed `icon` field now maps to `feed.image` (large timeline image) and `favicon` maps to `feed.icon` (browser favicon), per JSON Feed 1.1 spec; previously the two fields were swapped (#176)
- Core: when an RSS entry (or feed) has both `<author>` and `<dc:creator>`, `entry.author` now returns the `dc:creator` value (dc:creator always wins); previously dc:creator only set `entry.author` if it was not already set (#172)
- Core: when an RSS `<author>` element contains the `email@x.com (Name)` format, `entry.author` now returns the raw string (e.g. `"email@x.com (Name)"`); `entry.author_detail` still contains the parsed name and email fields; previously only the parsed name was stored in `entry.author` (#172)
- Atom feeds with `type="xhtml"` content now preserve inner HTML markup; the outer `<div xmlns="...xhtml">` wrapper is stripped per RFC 4287 §3.1.1.3; applies to `content`, `summary`, `title`, `rights`, and `subtitle` fields; previously all tags were stripped leaving bare concatenated text (#169)
- Atom `<source>`: self-closing `<link href="..."/>` before `<id>` no longer causes `source.id` to return `None`; `skip_to_end` is now skipped for `Event::Empty` elements in `parse_atom_source`, matching the pattern used elsewhere in the Atom parser (#174)
- Atom `<content type="xhtml">` now normalizes the `content_type` field to `"application/xhtml+xml"`; `"html"` normalizes to `"text/html"` and `"text"` to `"text/plain"`, matching Python feedparser MIME type output (#170)
- Python and Node.js bindings: `entry.slash_comments` now returns a string (e.g. `'42'`) instead of an integer, matching Python feedparser behavior and the existing `thr_total` string convention (#168)
- Core: truncated or unclosed XML feeds (RSS 2.0, Atom, RSS 1.0) now set `bozo=true` with `bozo_exception="Feed is truncated or has unclosed XML elements"`; previously the parsers silently ignored EOF without inspecting whether open elements remained unclosed (#165)
- Populate `ParsedFeed.namespaces` HashMap from `xmlns:` declarations on root elements (`<rss>`, `<channel>`, `<feed>`, `<rdf:RDF>`); default namespace `xmlns=""` uses key `""`, prefixed namespaces `xmlns:dc=""` use the prefix as key; enforces `max_namespaces` and `max_attribute_length` limits with bozo flag on overflow (#163)
- Python bindings: `entry['itunes_duration']`, `entry['itunes_episode']`, `entry['itunes_season']`, `entry['itunes_explicit']`, `entry['itunes_episodetype']`, `entry['itunes_author']`, `entry['itunes_title']`, `entry['itunes_image']` now work via `__getitem__`, matching Python feedparser flat key access; feed-level `feed['itunes_author']`, `feed['itunes_explicit']`, `feed['itunes_image']` also supported (#164)
- Python bindings: `FeedMeta`, `Entry`, and `FeedParserDict` now expose `.get(key, default=None)`, `.keys()`, `.values()`, and `.items()` methods, matching the `FeedParserDict` dict-compatible API from Python feedparser; `.get()` never raises `KeyError` (#162)
- Core: `entry.rights_detail.value` was always empty due to `std::mem::take` consuming the value before assigning it to `rights_detail`; fixed by cloning the value instead (#161)
- `media:content` elements now expose the `medium` attribute (`video`, `audio`, `image`, `document`, `executable`) in core, Python, and Node.js bindings, matching Python feedparser behavior (#158)
- Rename `media_thumbnails` to `media_thumbnail` (singular) across core, Python, and Node.js bindings to match Python feedparser API (#157)
- Map Atom `xml:lang` attribute on `<feed>` to `feed.language`; propagate to `TextConstruct.language` and `Content.language` on feed-level and entry-level constructs; entry-level `xml:lang` overrides feed-level (#149)
- Whitespace adjacent to XML entity sequences (`&lt;`, `&gt;`, `&amp;`, etc.) in RSS/Atom description and summary fields is now preserved; previously spaces immediately before or after entities were stripped because `trim_text` was applied per-token rather than to the final collected text (#152)
- Core: when an RSS item has only `content:encoded` (no `<description>`), or an Atom entry has only `<content>` (no `<summary>`), set `entry.summary` from `content[0].value` as a fallback, matching Python feedparser behavior (#150)
- Map RSS 2.0 `<lastBuildDate>` channel element to `feed.updated` / `feed.updated_parsed`, matching Python feedparser behavior (#147)
- Atom `<link>` elements without an explicit `type` attribute now get a default MIME type based on `rel`: `text/html` for `alternate`, `hub`, `enclosure`, and unknown relations; `application/atom+xml` for `self` — matching Python feedparser behavior (#146)
- RFC2822 date parsing now tolerates incorrect day-of-week names, matching Python feedparser behavior (#143)
- Node.js bindings: `entry.updated`, `entry.published`, `entry.created`, `entry.expired`, `entry.dcDate`, `feed.updated`, `feed.published` now return the original timezone-preserving date string from the feed; added corresponding `*Parsed` fields (`updatedParsed`, `publishedParsed`, `createdParsed`, `expiredParsed`, `dcDateParsed`) returning `number` (ms since epoch) for use with `new Date(ms)` (#141)
- Core/bindings: `updated` and `published` string fields now preserve the original timezone string from the feed instead of normalizing to UTC; `*_parsed` fields remain correctly normalized (#140)
- Map RSS 2.0 `<copyright>` channel element to `feed.rights` (#144)
- Parse RSS 2.0 `<source url="...">Title</source>` into `entry.source.link` and `entry.source.title` (#145)
- Parse Atom `<rights>` at entry level into `entry.rights` and `entry.rights_detail`; map `dc:rights` on entries to `entry.rights` (when not already set by Atom) and `entry.dc_rights`; expose `rights`, `rights_detail`, `copyright`, `copyright_detail` in Python bindings and `rights`, `rightsDetail` in Node.js bindings (#139)
- `TextConstruct.value` (`title_detail.value`, `summary_detail.value`, `subtitle_detail.value`, `rights_detail.value`) was always empty due to `mem::take` moving the string into the shorthand field; fixed by cloning instead (#136)
- `TextConstruct.type` now returns MIME types matching Python feedparser: `text/plain`, `text/html`, `application/xhtml+xml` instead of short forms `text`, `html`, `xhtml` (#136)
- Python binding: nested objects (`Enclosure`, `Tag`, `Image`, `Content`, `Generator`, `Link`, `Source`) now support dict-like subscript access (`obj['key']`), matching Python feedparser `FeedParserDict` behaviour; unknown keys raise `KeyError` (#134)
- `generator_detail.name` now contains the generator text content (previously the field was named `value` and was empty after `set_generator` consumed it via `mem::take`) (#132)
- `generator_detail.href` replaces `generator_detail.uri` to match Python feedparser API (`generator_detail['href']`) (#132)
- Python bindings: `PyGenerator` now exposes `.name` and `.href` getters; `.value` is kept as a backward-compatibility alias for `.name` (#132)
- Node.js bindings: `Generator` object now has `name` and `href` fields instead of `value` and `uri` (#132)
- Python/Node.js bindings: rename `Enclosure.url` → `href`, `Image.url` → `href`, `Image.description` → `subtitle` for feedparser API compatibility (#130)
- RSS `<author>`, `<managingEditor>`, and `<webMaster>` now parse `email (Name)` and `Name <email>` formats into structured `author_detail` / `publisher_detail` (`Person` with `name` and `email`), matching Python feedparser behavior (#128)
- Python binding: `Person.uri` renamed to `Person.href` for feedparser API compatibility; added `__getitem__` for dict-like access (`person['href']`, `person['name']`, `person['email']`) (#126)
- Node.js binding: `Person.uri` renamed to `Person.href` for feedparser API compatibility (#126)
- `author_detail.name` (and `publisher_detail.name`) was always `None` due to `Person.name` being moved via `.take()` into the shorthand field instead of cloned; affected `Entry` and `FeedMeta` setters (`set_author`, `set_publisher`) (#127)

## [0.4.8] - 2026-03-23

### Added
- feat(core): populate `feed.next_url` from Atom/RSS `<link rel="next">` per RFC 5005 (#120)
- Atom Threading Extensions (RFC 4685) support: parse `thr:in-reply-to` and `thr:total` elements in Atom 1.0, RSS 2.0, and RSS 1.0 feeds (#111)
  - New `InReplyTo` struct with fields `ref_`, `href`, `type_`, `source` (all `Option`)
  - New `Entry` fields: `in_reply_to: Vec<InReplyTo>` and `thr_total: Option<u32>`
  - Tolerant parsing: empty attribute values normalized to `None`; missing `ref` attribute accepted; all-empty `thr:in-reply-to` elements skipped; malformed/negative/overflow `thr:total` silently ignored
  - Python bindings: `PyInReplyTo` class with attribute and dict-style access; `thr_in_reply_to` and `thr_total` getters on entry (both `thr_in_reply_to` and `thr_in-reply-to` dict keys supported)
  - Node.js bindings: `InReplyTo` object type with `thrInReplyTo` and `thrTotal` fields on `Entry`
  - Filed #118 as follow-up for `thr:count` and `thr:updated` on `<link rel="replies">` (RFC 4685 §4)
- `slash:comments` (integer comment count) and `wfw:commentRss` (comment feed URL) namespace support for RSS and Atom feeds; exposed as `entry.slash_comments: Option<u32>` and `entry.wfw_comment_rss: Option<String>` in core, `entry.slash_comments` / `entry.wfw_commentrss` in Python bindings, and `entry.slashComments` / `entry.wfwCommentRss` in Node.js bindings (#109)
- JSON Feed 1.1: parse `next_url` feed-level field into `FeedMeta.next_url: Option<String>` (#112)
- JSON Feed 1.1: parse `banner_image` entry-level field, stored as `Link` with `rel="banner"` in `entry.links` (#112)
- `Link::banner()` constructor for creating banner image links (project-internal convention)
- Expose `next_url` in Python bindings via `#[getter]`, `__getattr__`, and `__getitem__`
- Expose `next_url` in Node.js bindings as `FeedMeta.next_url`
- Parse `<subtitle>` element at the Atom entry level: `Entry` now exposes `subtitle: Option<String>` and `subtitle_detail: Option<TextConstruct>`, mirroring the existing feed-level subtitle fields (#110)
- Expose `subtitle` and `subtitle_detail` on `Entry` in Python (PyO3) and Node.js (napi-rs) bindings (#110)
- RFC 4685 Atom Threading Extensions: parse `thr:count` (reply count) and `thr:updated` (last reply datetime) attributes on `<link>` elements; exposed as `link.thr_count: Option<u32>` and `link.thr_updated: Option<DateTime<Utc>>` in core, `link.thr_count` / `link.thr_updated` / `link.thr_updated_parsed` in Python bindings, and `link.thrCount` / `link.thrUpdated` in Node.js bindings (#118)
- Map Atom `<link rel="enclosure">` to `entry.enclosures` for API parity with Python feedparser; enclosure links are dual-populated into both `entry.links` and `entry.enclosures`, with optional `type` and `length` attributes silently mapped to `None` when absent or invalid (#119)

### Fixed
- Parse RSS `<category domain="...">` attribute as `Tag.scheme` (#116)
- Add `tests/fixtures/**` to the `rust-core` paths-filter group so fixture-only PRs correctly trigger Rust test jobs (#107)
- Fix encoding detection for non-UTF-8 feeds: `extract_xml_encoding` now performs a byte-level search for the XML declaration instead of calling `str::from_utf8` on the full search buffer, which failed when non-ASCII bytes appeared within the first 512 bytes (#95)
- `parse()` and `parse_with_limits()` now detect and convert non-UTF-8 feeds (ISO-8859-1, Windows-1252, UTF-16 LE/BE, UTF-8 BOM) to UTF-8 before parsing, and set `feed.encoding` to the detected encoding label (#95)
- Atom 0.3 feeds now correctly report `version = "atom03"` instead of `"atom10"` (#91)
- Atom 0.3 `<modified>` and `<issued>` elements are now mapped to `updated` and `published` fields respectively (#91)
- Unrecognized feed format now sets `bozo = true` and `version = Unknown` instead of silently returning an empty RSS 2.0 feed (#100)

### Tests
- Add adversarial input tests covering DOCTYPE entity injection, XML bomb patterns, NUL bytes, long attributes, and malformed namespace URIs; verify no panics and correct bozo behavior (#101)
- Add integration tests for non-UTF-8 feed parsing: ISO-8859-1, Windows-1252, UTF-8 BOM, and UTF-16 LE BOM feeds (#95)
- Add integration tests for all 16 previously untested `ParserLimits` fields (#94): `max_links_per_feed`, `max_links_per_entry`, `max_authors`, `max_contributors`, `max_tags`, `max_content_blocks`, `max_enclosures`, `max_namespaces`, `max_text_length`, `max_feed_size_bytes`, `max_attribute_length`, `max_podcast_soundbites`, `max_podcast_transcripts`, `max_podcast_funding`, `max_podcast_persons`, `max_value_recipients`
- Add integration test fixture and tests for Atom 0.3 feed parsing (#91)
- Expand date parsing unit tests to cover all 27 format strings and 2 special cases (#92)
- Add end-to-end integration tests for GeoRSS (`georss:point`, `georss:polygon`, feed-level geo, invalid coordinates) and Creative Commons (`creativeCommons:license`, `cc:license`) namespace parsing (#93)
- Add Rust integration tests for GUID XML entity decoding (REG-002): `&amp;`, `&#038;`, `&#x26;`, multiple entities, unknown/malformed entities (#103)
- Strengthen edge-case assertions for empty, whitespace-only, invalid XML, DOCTYPE-only, and invalid UTF-8 input (#100)

## [0.4.7] - 2026-03-21

### Dependencies

- Bump `once_cell` in the patch-updates group (#87)
- Bump `@biomejs/biome` (#86, #83)
- Bump `dorny/paths-filter` from 3 to 4 (#85)
- Bump `quinn-proto` from 0.11.13 to 0.11.14 (#84)
- Bump dependency versions in Cargo.lock

## [0.4.6] - 2026-03-05

### Fixed
- Propagate bozo flag from entry-level fields: thread bozo signal through `parse_item`, `parse_entry`, and `parse_rss10_item` (#75)

### Security
- Bump `aws-lc-sys` from 0.37.1 to 0.38.0 (via `aws-lc-rs` 1.16.1) to fix three HIGH severity advisories: PKCS7_verify Certificate Chain Validation Bypass (GHSA-vw5v-4f2q-w9xf), Timing Side-Channel in AES-CCM Tag Verification (GHSA-65p9-r9h6-22vj), PKCS7_verify Signature Validation Bypass (GHSA-hfpc-8r3f-gw53) (#81)

### Dependencies
- Bump `chrono` in the patch-updates group (#80)
- Bump `minimatch` from 10.2.2 to 10.2.4 in Node.js bindings (#77)
- Bump patch-updates group with 2 updates (#76)
- Bump `@biomejs/biome` (#74)
- Bump `actions/checkout` from 4 to 6 (#73)
- Bump `github/codeql-action` from 3 to 4 (#72)
- Bump `actions/upload-artifact` from 6 to 7 (#78)
- Bump `actions/download-artifact` from 7 to 8 (#79)

## [0.4.5] - 2026-02-20

### Fixed
- Regenerate Node.js `package-lock.json` to fix npm release CI failures (#67)
- Add CodeQL security scanning workflow
- Make entity resolution bozo-tolerant: `resolve_entity` preserves malformed entities as-is instead of failing (#64)
- Propagate bozo flag from `read_text` when encountering unresolvable entities in feed-level fields (#64)

### Added
- Edge-case tests for invalid numeric refs, malformed entity syntax, unknown named entities, and mixed valid/invalid entities (#64)

## [0.4.4] - 2026-02-20

### Fixed
- Handle XML entity references (e.g. `&#038;`) in element text, matching Python feedparser behavior (#59, #60)

### Changed
- Update `pyo3` from 0.27.2 to 0.28.x to fix memory corruption vulnerability RUSTSEC-2026-0013 (#51, #62)
- Switch CI security audit from `npm audit` to `pnpm audit` for correct override handling (#62)
- Bump `minimatch` to >=10.2.1 to resolve ReDoS vulnerability GHSA-3ppc-4f35-3m26 (#62)
- Upgrade `@biomejs/biome` to 2.4.0 (#49, #54, #57)
- Make Rust coverage upload non-blocking in CI
- Bump `lewagon/wait-on-check-action` from 1.3.4 to 1.5.0 (#50)
- Add dependabot auto-merge workflow

### Dependencies
- Bump `bytes` from 1.11.0 to 1.11.1 (#52)
- Bump `memchr` from 2.7.6 to 2.8.0 (#56)
- Bump `thiserror` in the patch-updates group (#48)
- Bump patch-updates group with multiple updates (#53, #55, #58)

## [0.4.3] - 2026-01-15

### Added
- Add `ruff` linter and formatter for Python bindings
- Add `biome` linter and formatter for Node.js bindings
- Add `npm audit` to CI security checks (alongside `cargo deny`)

### Changed
- Drop Python 3.9 support (dependencies require 3.10+)
- Add Python 3.14 support
- Build wheels for all supported Python versions (3.10-3.14)

## [0.4.2] - 2026-01-14

### Fixed
- RSS 2.0 feeds with self-closing XML elements (e.g., `<atom:link ... />`) now parse items correctly (#45)
- Empty elements at both channel and item level are handled properly
- Self-closing enclosure elements no longer break item parsing
- Empty `itunes:image` elements now populate `feed.feed.image`

## [0.4.1] - 2025-01-12

### Changed
- Updated `quick-xml` from 0.38 to 0.39
- Updated `reqwest` from 0.12 to 0.13 (switched from `rustls-tls` to `rustls` feature)
- Added OpenSSL license to `deny.toml` for `aws-lc-sys` (rustls crypto backend)

## [0.4.0] - 2025-12-28

### Added
- **Python feedparser compatibility improvements**:
  - Field alias mappings for deprecated field names (`description` → `subtitle`, `guid` → `id`, etc.)
  - Dict-style access on feed objects (`d['feed']['title']`, `d['entries'][0]['link']`)
  - Container aliases (`channel` → `feed`, `items` → `entries`)
  - Auto-URL detection in `parse()` function (URLs are automatically fetched when http feature enabled)
  - Optional HTTP parameters (`etag`, `modified`, `user_agent`) for `parse()` and `parse_with_limits()`

### Changed
- `parse_with_limits()` now uses keyword-only `limits` parameter for consistency

## [0.3.0] - 2025-12-18

### Added
- Syndication Module namespace support (`syn:updatePeriod`, `syn:updateFrequency`, `syn:updateBase`)
- `feed.published` field for Atom feeds and RSS channel `pubDate`
- `xml:base` URL resolution for relative URLs in Atom and RSS feeds
- `xml:lang` attribute tracking for feed and entry language detection
- Creative Commons `license` field extraction from `rel="license"` links
- Comprehensive RSS 1.0 integration tests (12+ test cases)
- Syndication metadata exposed in Python and Node.js bindings
- Dublin Core fields (`dc_creator`, `dc_publisher`, `dc_rights`) in bindings
- Benchmark results in all README files

### Changed
- Improved test coverage from 83% to 91%+
- Optimized Python bindings to return `&str` instead of `String` for enum values
- Simplified Node.js Entry conversion using idiomatic `.collect()` pattern
- Updated documentation with performance benchmarks (90-94x faster than Python feedparser)

### Fixed
- Performance issue with unnecessary string allocations in Python `__repr__` methods

## [0.2.1] - 2025-12-16

### Changed
- crates.io publishing now uses OIDC trusted publishing (no tokens required)
- Updated crate READMEs with GitHub callouts and consistent formatting

## [0.2.0] - 2025-12-16

### Added
- RSS 1.0 (RDF) parser support with full namespace handling
- GeoRSS namespace support (point, line, polygon, box geometries)
- Creative Commons namespace support with license links (`rel="license"`)
- `ParseOptions` API with `strict()`, `permissive()`, `default()` presets
- Base URL resolution (`xml:base`) for relative URLs in Atom feeds
- HTTP `Content-Type` charset extraction for encoding detection
- Year-only (`2024`) and year-month (`2024-12`) date format parsing
- GitHub Copilot code review agents configuration

### Fixed
- Critical SSRF vulnerabilities with URL validation and domain allowlisting
- Input validation for parser limits to prevent DoS attacks

### Changed
- Refactored parsers with `collect_attributes` and `find_attribute` helpers
- npm publishing now uses OIDC trusted publishing with provenance attestations
- Improved test coverage to 83.78%

### Security
- Added SSRF protection for HTTP fetching with configurable domain restrictions
- Strengthened input validation for all parser limit parameters

## [0.1.8] - 2025-12-16

### Added
- Export `parse_url` and `parse_url_with_limits` in Python bindings
- Supported Formats and Namespace Extensions tables in README

### Fixed
- Python README now documents URL fetching (was marked as not implemented)
- Repository URLs in Python package metadata

### Changed
- Improved test coverage to 83.78%

## [0.1.7] - 2025-12-16

### Changed
- Merged all release workflows into single `release.yml` for reliable GitHub Release creation
- All platforms (crates.io, PyPI, npm) now build and publish in a single coordinated workflow

## [0.1.6] - 2025-12-16

### Fixed
- Unified GitHub Release workflow to prevent overwrites between crates.io/PyPI/npm releases
- Synchronized version numbers across Cargo.toml, pyproject.toml, and package.json

## [0.1.5] - 2025-12-16

### Fixed
- Switched from native-tls to rustls-tls to eliminate OpenSSL dependency for cross-compilation
- Use native ARM runners (ubuntu-24.04-arm) instead of cross-compilation for aarch64
- Fixed deprecated macos-13 runner by using macos-latest with cross-compilation
- Fixed Windows PowerShell compatibility in npm release workflow

## [0.1.4] - 2025-12-16

### Fixed
- Added package-lock.json for Node.js release workflow

## [0.1.3] - 2025-12-16

### Fixed
- Fixed package name in release-crates.yml workflow (feedparser-rs-core → feedparser-rs)
- Switched to PyO3/maturin-action for PyPI releases

## [0.1.2] - 2025-12-16

### Fixed
- Fixed GitHub Actions artifact versions (v7 → v4)

### Added
- PyPI badge in README

## [0.1.1] - 2025-12-16

### Added
- HTTP fetching with `http` feature (enabled by default)
- `parse_url` and `parse_url_with_limits` functions for URL fetching
- Conditional GET support (ETag, Last-Modified) for bandwidth-efficient caching
- Automatic compression handling (gzip, deflate, brotli)
- Node.js `fetchAndParse` function for URL fetching
- Podcast namespace support (iTunes and Podcast 2.0)
- CONTRIBUTING.md guide
- GitHub issue and PR templates
- Codecov badge in README

### Changed
- Renamed crate from `feedparser-rs-core` to `feedparser-rs`
- Default features now include `http` for URL fetching support
- Migrated to cargo-make for task automation
- Updated documentation with more accurate claims

## [0.1.0] - 2025-12-14

### Added
- Initial release
- RSS 2.0, 1.0, 0.9x parsing
- Atom 1.0, 0.3 parsing
- JSON Feed 1.0, 1.1 parsing
- Multi-format date parsing
- HTML sanitization
- Encoding detection
- Tolerant parsing with bozo flag
- Rust core library
- Parser limits for security (max nesting depth, entry count, etc.)
- Comprehensive test coverage
- Documentation with examples

[Unreleased]: https://github.com/bug-ops/feedparser-rs/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/bug-ops/feedparser-rs/compare/v0.4.8...v0.5.0
[0.4.8]: https://github.com/bug-ops/feedparser-rs/compare/v0.4.7...v0.4.8
[0.4.7]: https://github.com/bug-ops/feedparser-rs/compare/v0.4.6...v0.4.7
[0.4.6]: https://github.com/bug-ops/feedparser-rs/compare/v0.4.5...v0.4.6
[0.4.5]: https://github.com/bug-ops/feedparser-rs/compare/v0.4.4...v0.4.5
[0.4.4]: https://github.com/bug-ops/feedparser-rs/compare/v0.4.3...v0.4.4
[0.4.3]: https://github.com/bug-ops/feedparser-rs/compare/v0.4.2...v0.4.3
[0.4.2]: https://github.com/bug-ops/feedparser-rs/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/bug-ops/feedparser-rs/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/bug-ops/feedparser-rs/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/bug-ops/feedparser-rs/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/bug-ops/feedparser-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/bug-ops/feedparser-rs/compare/v0.1.8...v0.2.0
[0.1.8]: https://github.com/bug-ops/feedparser-rs/compare/v0.1.7...v0.1.8
[0.1.7]: https://github.com/bug-ops/feedparser-rs/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/bug-ops/feedparser-rs/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/bug-ops/feedparser-rs/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/bug-ops/feedparser-rs/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/bug-ops/feedparser-rs/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/bug-ops/feedparser-rs/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/bug-ops/feedparser-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/bug-ops/feedparser-rs/releases/tag/v0.1.0
