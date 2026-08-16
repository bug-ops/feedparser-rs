# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- `feedparser-rs-py`: migrated `compat.rs`'s field-alias maps from `once_cell::sync::Lazy` to `std::sync::LazyLock`, matching the pattern already used in `feedparser-rs-core`. `LazyLock` has been stable since Rust 1.80, well below this project's MSRV (1.88.0), so `once_cell` is no longer a direct dependency of any workspace crate and was removed from `[workspace.dependencies]` (#489).
- **Breaking**: raised the workspace MSRV from 1.88.0 to 1.91.0 (#490). Per this project's policy, an MSRV bump requires a minor version bump, so this must ship as **0.7.0**, not 0.6.1 — a consumer pinned to `feedparser-rs = "0.6"` will not pull this in via a semver-compatible upgrade. This unlocks `Ipv4Addr::from_octets`/`Ipv6Addr::from_segments`, now used in `util/ssrf.rs` in place of `Ipv4Addr::new`/`Ipv6Addr::new` for a clearer octet/segment-array round-trip with the existing `octets()`/`segments()` getters, and `Duration::from_mins`/`from_hours` in a couple of test-only call sites in `http/client.rs`/`tests/http_integration.rs` (both surfaced by the new clippy `duration_suboptimal_units` lint now in-MSRV); behavior is unchanged in all cases.

### Fixed

- CI: the `release.yml` npm build/publish jobs used `npm ci` against a stale `package-lock.json` that had drifted from `package.json`'s devDependencies, failing every npm platform build during the v0.6.0 release. Switched those jobs to `pnpm` (matching `ci.yml` and the `cargo-make` Node.js tasks) and removed `package-lock.json`, so the workspace has a single Node.js lockfile (`pnpm-lock.yaml`) instead of two that can drift out of sync. Also switched the `security` CI job's `npm audit` back to `pnpm audit`, which had regressed to `npm audit` despite `pnpm` already being installed in that job.

## [0.6.0] - 2026-08-17

### Security

- Close an SSRF protection bypass in `FeedHttpClient` (CWE-918, #436): HTTP redirects (`Location` headers) are now re-validated against the same SSRF checks as the initial request, and DNS resolution results are re-checked at connect time via a custom resolver, closing a DNS-rebinding gap where a domain resolving to a public IP at validation time could be repointed to a private/loopback/link-local/metadata address before the connection was made. Also consolidated `util::base_url::is_safe_url` to reuse the same validation rules as `http::validation::validate_url` instead of a separate, weaker, duplicated implementation.
- `FeedHttpClient` no longer honors the system/environment HTTP proxy (`HTTP_PROXY`/`HTTPS_PROXY`/`ALL_PROXY`): a proxy resolves its own target hostname, which silently bypassed the DNS-rebinding-safe resolver above.
- Close an IPv6 embedded-IPv4 SSRF bypass in `validate_ipv6`: IPv4-mapped (`::ffff:127.0.0.1`), deprecated IPv4-compatible (`::127.0.0.1`), and NAT64 (`64:ff9b::/96`, RFC 6052) address forms are now unwrapped to their embedded IPv4 address and validated against the same IPv4 rules, instead of silently passing every native IPv6 check. 6to4 (`2002::/16`, RFC 3056) is a known, deliberately deferred follow-up: relay infrastructure for it has been widely decommissioned and it is not standard on any current cloud network, unlike the three forms fixed here.
- **[P1]** Close a trailing-dot FQDN SSRF bypass in `validate_domain` (#452): one or more trailing DNS root-label dots (e.g. `http://metadata.google.internal./`, `http://localhost../`) resolve identically to the un-dotted form via every standard stub resolver, but previously bypassed every domain-string blocklist check (`LOCALHOST_VARIANTS`, `INTERNAL_TLDS`, `METADATA_DOMAINS`) because the trailing dot(s) broke exact-match and `ends_with` comparisons. The domain is now normalized by stripping all trailing `.` characters before those checks run.
- Add several missing IANA special-purpose address ranges to `validate_ipv4`/`validate_ipv6` as defense-in-depth (#453): IPv4 multicast (`224.0.0.0/4`), reserved (`240.0.0.0/4`), IETF protocol assignments (`192.0.0.0/24`), 6to4 relay anycast (`192.88.99.0/24`), and benchmarking (`198.18.0.0/15`); IPv6 Teredo (`2001::/32`), ORCHIDv2 (`2001:20::/28`), documentation (`2001:db8::/32`), the discard-only prefix (`100::/64`), and the RFC 8215 NAT64 local-use prefix (`64:ff9b:1::/48`).
- **[P4]** Add the remaining IANA AS112/AMT/PCP anycast sub-ranges to `validate_ipv4`/`validate_ipv6` — newly-blocked IANA special-purpose ranges not covered by the existing IETF-assignment-block checks, closing the registry-completeness gap left by #453/#455 (#462): IPv4 AS112-v4 (`192.31.196.0/24`, RFC 7535), AMT (`192.52.193.0/24`, RFC 7450), and Direct Delegation AS112 Service (`192.175.48.0/24`, RFC 7534); IPv6 PCP Anycast (`2001:1::1/128`, RFC 7723), TURN Relay Anycast (`2001:1::2/128`, RFC 8155), AMT (`2001:3::/32`, RFC 7450), AS112-v6 (`2001:4:112::/48`, RFC 7535), and Direct Delegation AS112 Service (`2620:4f:8000::/48`, RFC 7534). These are publicly routed anycast service addresses, not private/internal ranges — no attack scenario, registry completeness only.
- **[P3]** Block `2001:1::3/128` (DNS-SD Service Registration Protocol Anycast, RFC 9665) and `100:0:0:1::/64` (Dummy IPv6 Prefix, RFC 9780) in `validate_ipv6_special_purpose` (#471): both addresses were registered in the IANA IPv6 Special-Purpose Address Registry after #469 shipped and previously fell through as allowed, sitting directly inside/adjacent to ranges the registry now reserves. Registry-completeness only, same as #462 — no attack scenario.
- **[P4]** Add the remaining uncovered IANA IPv6 special-purpose ranges to `validate_ipv6` (#471): Benchmarking (`2001:2::/48`, RFC 5180), the deprecated (previously ORCHID) range (`2001:10::/28`, RFC 4843) and DRIP Entity Tags (`2001:30::/28`, RFC 9374), the documentation range extension (`3fff::/20`, RFC 9637), and Segment Routing (SRv6) SIDs (`5f00::/16`, RFC 9602). `2002::/16` (6to4) remains deliberately out of scope; see the #469 entry above.
- **[P2]** Collapsed the individual `2001::/23` IETF Protocol Assignments sub-range checks (Teredo, ORCHIDv2, the deprecated ORCHID range, DRIP, PCP/TURN/DNS-SD anycast, AMT, AS112-v6) in `validate_ipv6_special_purpose`/`validate_ipv6_special_purpose_extended` into a single `segments[0] == 0x2001 && (segments[1] & 0xFE00) == 0` check (#474). Enumerating sub-ranges one at a time kept reopening the same registry-completeness gap (#453/#455/#462/#471) every time IANA registered a new address inside the block; blocking the whole `2001::/23` block outright closes it structurally instead. `2001:1::` itself (no anycast suffix) is now blocked too — previously allowed despite being unassigned within the block. `2001:db8::/32` (documentation, RFC 3849) is a separate top-level IANA allocation, not a sub-range of `2001::/23`, and keeps its own explicit check.
- SSRF rejection reasons (from redirect re-validation or DNS-rebinding checks) are no longer swallowed by `reqwest::Error`'s `Display`, which only prints the outer error kind and URL — `FeedHttpClient::get` now walks the full error source chain so callers see the actual reason a request was blocked.
- **[P0]** `ParseOptions.sanitize_html` (documented default: `true`) was never consumed by any parse entry point, so HTML-bearing feed fields (titles, summaries, `content:encoded`, etc.) reached applications unsanitized — a stored XSS vector for any consumer that renders parsed feed content. Added a fail-closed `sanitize_feed` post-parse pass (`util::sanitize::sanitize_feed`) that walks every HTML-bearing field of `ParsedFeed`, matching Python feedparser's `can_contain_dangerous_markup` coverage, and wired it into `parse`/`parse_with_limits`/`parse_with_options` and their `parse_url*` counterparts (#438)
  - **Behavior change**: HTML entities are now escaped and unrecognized tags dropped by default, e.g. an RSS title `Rust & C++: a <comparison>` becomes `Rust &amp; C++: a ` — this matches Python feedparser's sanitizer. Pass `sanitize_html: false` (Rust/Python) or `{ sanitizeHtml: false }` (Node) to opt out for fully-trusted feed sources. Sanitization is idempotent
  - Fixed a related bypass: Atom `type="text/html"`, `type="application/xhtml+xml"`, and case variants like `type="HTML"` previously fell through to the plain-text default and were never sanitized even before this fix; the `type` attribute is now parsed case-insensitively with unrecognized values treated as HTML (fail-closed)
  - `title`/`subtitle`/`summary` fields on RSS 2.0, RSS 1.0, and JSON Feed have no per-element type indicator and were initially (during development of this fix) still mislabeled as always-safe plain text, leaving the P0 partially open for those fields; they are now treated as potentially unsafe HTML by default, matching how `<description>` was already handled
  - An Atom text construct (`<title>`, `<summary>`, `<subtitle>`, `<rights>`) with **no** `type` attribute at all is now sanitized too. RFC 4287 §3.1.1 defaults an absent `type` to "text" for *display* purposes, but that default carries no safety assertion — only an *explicit* `type="text"` is now trusted to skip sanitization
  - `sanitize_html`'s underlying HTML5 tree builder is quadratic-time on pathologically deep tag nesting within a single field; added `ParserLimits::max_html_nesting_depth` (default 100) so content nested deeper is escaped as plain text in O(n) instead of being handed to the sanitizer unbounded, closing a remote CPU-DoS on the newly-default-on sanitization path. The depth estimate tracks matched tag *names* on a small stack, scope-aware per HTML5's "has an element in scope" algorithm (a closing tag cannot pop through a `<table>`/`<td>`/`<th>`/`<caption>` scope barrier to reach a same-named ancestor — this is what makes `("<div><table></div>")*n` genuinely nest rather than closing cleanly each iteration), and recognizes HTML void elements (`<br>`, `<img>`, ...), auto-closing elements (`<li>`, `<p>`, `<td>`, `<th>`, `<tr>`, `<option>`), and HTML5 "formatting elements" (`<b>`, `<font>`, `<i>`, ..., which — verified empirically — do not create the same pathological nesting a structural element does even when repeated thousands of times unclosed). An additional O(1) backstop caps total tags per field at 10,000, bounding worst-case behavior independent of how closely this model tracks html5ever's real semantics. Together these mean ordinary valid or mildly malformed HTML — image galleries, unclosed `<li>`/`<p>` runs, tables, runs of unclosed inline formatting — is never misjudged as pathologically nested, while every discovered way to genuinely hide deep nesting from the guard is closed
  - Hoisted the sanitizer's tag/attribute allow-lists and `ammonia::Builder` to a `LazyLock` built once, instead of rebuilding them on every call — this was a measurable regression once sanitization went from zero call sites to dozens per entry
- Added `resolve: bool` to `BaseUrlContext` so `ParseOptions::strict()`'s `resolve_relative_uris: false` now actually disables relative URL resolution (previously a no-op). The safety checks (dangerous-scheme filter, SSRF/private-IP validation) always run regardless of this setting — disabling resolution only skips joining a relative URL against the feed's base URL, so an absolute dangerous URL supplied directly by the feed is still blocked

### Added

- `parse_with_options(data, &ParseOptions)` and `parse_url_with_options(url, etag, modified, user_agent, &ParseOptions)` — full control over HTML sanitization, relative URI resolution, and parser limits in a single call. `parse`/`parse_with_limits` and `parse_url`/`parse_url_with_limits` are now thin wrappers over these
- CI: `test-node` job now fails if `pnpm run build` regenerates `crates/feedparser-rs-node/index.d.ts`/`index.js` with content differing from the committed files, closing the gap that let those generated files drift from the actual napi-exported API without CI catching it (#450). The check runs once per CI run (on the `ubuntu-latest` / Node 22 matrix leg) via a new `test-node-verify-generated` `cargo-make` task
- Node binding: documented error `.code` property (`'InvalidArg'` | `'GenericFailure'`) with usage example and tests covering both paths (#444)
- Podcast 2.0 `podcast:chat`, `podcast:podping`, and `podcast:valueTimeSplit` support (#437): `PodcastChat` (channel-level `PodcastMeta.chat` and item-level `PodcastEntryMeta.chat`), `PodcastMeta.podping_uses_podping`, and `PodcastValueTimeSplit` (`PodcastValue.time_splits`), plus `ParserLimits::max_podcast_chat` (default 20) and `max_podcast_value_time_splits` (default 20). `podcast:valueTimeSplit` is parsed only from channel-level `<podcast:value>` in this release — item-level `<podcast:value>` is not yet parsed, tracked in a follow-up issue. **Breaking**: `ParserLimits` is not `#[non_exhaustive]`, so adding `max_podcast_chat` and `max_podcast_value_time_splits` breaks any downstream exhaustive `ParserLimits { .. }` literal construction (this PR itself had to patch `crates/feedparser-rs-py/src/limits.rs` for exactly that reason)
- Item-level `podcast:value`/`podcast:valueTimeSplit` support (#466): `PodcastEntryMeta.value: Option<PodcastValue>` is now populated from `<podcast:value>` when it appears inside `<item>`, reusing the same parsing and `ParserLimits` bounds (`max_value_recipients`, `max_podcast_value_time_splits`) as the channel-level tag added in #437. This was flagged during #465's review as materially limiting, since `podcast:valueTimeSplit` is inherently episode-scoped and real-world feeds are expected to place it inside `<item>` rather than only `<channel>`. Node.js/Python binding parity remains deferred (tracked in #480). **Breaking**: `PodcastEntryMeta` is not `#[non_exhaustive]`, so adding `value` breaks any downstream exhaustive `PodcastEntryMeta { .. }` struct literal or destructuring pattern, same class of break as the `ParserLimits` note above
- Node.js and Python binding parity for the channel-level `podcast:chat`, `podcast:podping`, and `podcast:valueTimeSplit` tags (#467): `PodcastMeta.chat`/`PodcastEntryMeta.chat` (`PodcastChat`), `PodcastMeta.podpingUsesPodping`/`podping_uses_podping`, and `PodcastValue.timeSplits`/`time_splits` (`PodcastValueTimeSplit`, including its `remoteItem`/`remote_item` via the newly-mirrored `PodcastRemoteItem`) are now exposed through both bindings, closing the parity gap left open by #465. Item-level `podcast:value`/`valueTimeSplit` (#466) and `podcast:alternateEnclosure` binding parity remain deferred, tracked in #480
- Node.js and Python binding parity for the remaining `PodcastRemoteItem` use sites (#473): `PodcastRemoteItem` was previously reachable only via `PodcastValueTimeSplit.remoteItem`/`remote_item` (#467). Now also exposed via `PodcastMeta.podroll` (feed level) and newly-mirrored feed-level fields `PodcastMeta.location`/`txt`/`updateFrequency`(`update_frequency`)/`follow`, plus entry-level `PodcastEntryMeta.alternateEnclosures`(`alternate_enclosures`)/`location`/`socialInteract`(`social_interact`)/`txt`/`follow`. Adds the following newly-mirrored types to both bindings: `PodcastLocation`, `PodcastTxt`, `PodcastUpdateFrequency`, `PodcastFollow`, `PodcastSocialInteract`, `PodcastAlternateEnclosure`, `PodcastAlternateEnclosureSource`, `PodcastIntegrity`. Note: `PodcastAlternateEnclosure.length` is exposed as `f64` in the Node.js binding (napi has no `FromNapiValue` support for `u64`), exact up to 2^53 bytes.
- Node.js and Python binding parity for item-level `podcast:value`/`podcast:valueTimeSplit` (#466, #480): `PodcastEntryMeta.value`/`value` (reusing `PodcastValue`) is now exposed through both bindings, reusing the `PodcastAlternateEnclosure` mirror types added above for `PodcastEntryMeta.alternateEnclosures`/`alternate_enclosures`, which #473 had already exposed independently

### Changed

- **BREAKING**: `GeoLocation.crs` renamed to `srsName` in the Node.js binding to match the `srs_name` field used by core and the Python binding (#441)
- **Breaking (Node.js)**: `parseWithOptions`/`parseUrlWithOptions` now take a `ParseOptions` object (`{ maxSize?, sanitizeHtml?, resolveRelativeUris? }`) instead of a positional `maxSize` number — update call sites from `parseWithOptions(source, 1024)` to `parseWithOptions(source, { maxSize: 1024 })`
- Python `parse()`/`parse_with_limits()`/`parse_url()`/`parse_url_with_limits()` gained `sanitize_html: bool = True` and `resolve_relative_uris: bool = True` keyword arguments
- Internal: split the parser functions that exceeded the project's 100-line function-length limit (`atom.rs::parse_entry`/`parse_feed_element`, `rss.rs`'s channel/item family, `rss10.rs::parse_rss10_with_options`/`parse_item`/`parse_channel`, `json.rs::parse_item`) into smaller per-namespace helpers, following the decomposition pattern already used elsewhere in `rss.rs` (#433). Output is byte-identical (verified via differential testing against the pre-refactor baseline across the full fixture corpus plus truncation/self-closing-tag mutations); RSS 1.0 (RDF) parsing has a measured ~2-4% throughput cost from three new per-element allocations required by the split's handler-function signature convention.
- Internal: enabled the workspace's `clippy::pedantic`/`clippy::nursery` lint gate (`[lints] workspace = true`) on `feedparser-rs-node`, which previously had no lint enforcement of its own beyond a redundant crate-level `#![deny(clippy::all)]` (removed, since the workspace lint table now covers it) (#435). Fixed the resulting warnings: missing crate/field doc comments, missing doc-markdown backticks, missing `# Errors` sections, redundant `String` clones and closures, an `i64`-to-`f64` timestamp cast centralized into one documented helper, and a lossless `u16`-to-`u32` status-code cast changed to `u32::from`. Added a direct `chrono` dependency (previously only reached transitively through `feedparser-rs`) to name `DateTime<Utc>` for the new timestamp helper and to reference `DateTime::to_rfc3339` directly. Purely internal code shape changes — no public API behavior changed; regenerated `index.d.ts` to pick up the updated TSDoc comments (`napi build --platform`). Four of the five `#[napi]`-exported functions keep owned `String`/`Either<Buffer, String>` parameters (`#[allow(clippy::needless_pass_by_value)]`, annotated) since `napi-rs` cannot bind JS string/enum arguments by reference; `parse` needs no such allow since it fully consumes its argument.
- **Breaking**: `FeedError` no longer derives `Clone`, and `XmlError`/`IoError`/`JsonError`/`UrlError` now carry the concrete source error (`quick_xml::Error`, `std::io::Error`, `serde_json::Error`, `url::ParseError`) instead of a pre-formatted `String`. `FeedError::source()` now returns the real underlying error, supporting `downcast_ref` on it, instead of always returning `None` (#440)
- **Breaking**: `PodcastValue` no longer derives `Eq` (gained a `Vec<PodcastValueTimeSplit>` field, which is not `Eq` due to its `f64` fields); `PartialEq` is retained
- `quick_xml`, `serde_json`, and `url` are now effectively public dependencies of `feedparser-rs`'s API surface via `FeedError`'s variants — a semver-major bump of any of the three in a future release becomes a breaking change for consumers pattern-matching on those `FeedError` variants (#440)
- Internal (#460): eliminated all `#[allow(clippy::too_many_arguments)]` in `feedparser-rs-core`'s RSS/Atom/RSS 1.0 parsers by introducing shared `XmlCtx`/`EntryCtx` context structs (new `parser/context.rs`) plus small per-format outer-tier structs (`ChannelCtx` in `rss.rs`, `FeedCtx` in `atom.rs`, `RdfCtx` in `rss10.rs`) that bundle the reader/buffer/limits/base/lang/namespace state previously threaded as individual parameters. Removed the dead, unused `ParseContext` type from `parser/common.rs`. Added a crate-level `#![forbid(clippy::too_many_arguments)]` in `feedparser-rs-core/src/lib.rs` (not a Cargo.toml lints-table entry, so `[lints] workspace = true` keeps inheriting the full workspace rust+clippy lint sets unmodified) so the count cannot silently regress; `feedparser-rs-node`'s `#[napi]`-generated code emits its own `#[allow(clippy::all)]`, which is incompatible with a workspace- or Cargo.toml-level `forbid`, hence the crate-level attribute instead. Pure refactor: parser semantics and output are unchanged, verified by the full existing test suite passing with zero test-file edits.
- Internal (#484): moved `EntryCtx`'s RSS 2.0-only `has_explicit_link`/`guid_is_permalink` fields out into a new `RssGuidCtx`, private to `rss.rs` and local to `parse_item`, threaded only through the `dispatch_item_tag` -> `parse_item_standard` -> `parse_item_links` call chain that actually uses them. `EntryCtx` (shared by Atom, RSS 1.0, and RSS 2.0) no longer carries dead fields that only one of its three consumers ever read or wrote. Pure refactor: parser semantics and output are unchanged.

### Fixed

- **[P1]** A GeoRSS GML geometry whose coordinate text was present and non-empty but whose length didn't cleanly divide by the resolved `srsDimension` (`gml:pos`/`gml:posList`, and `gml:lowerCorner`/`gml:upperCorner` on `gml:Envelope`) was silently dropped — `entry.where`/`feed.where` ended up `None` with `bozo` left `false`, indistinguishable from a feed with no GML geometry at all (#478). `build_gml_geometry`/`build_gml_envelope` now distinguish this specific coordinate-count/dims mismatch from other malformed-input cases and surface it as `bozo = true` with a description ("GML coordinate list length is not a multiple of resolved srsDimension") at both feed and entry/item level, across RSS 2.0, RSS 1.0, and Atom. The geometry itself is still omitted, since there is no safe way to interpret the mismatched tuple. Also fixed: an unresolvable entity inside `gml:pos`/`gml:posList`/`gml:lowerCorner`/`gml:upperCorner` text is no longer misdiagnosed with the dims-mismatch description when it happens to leave an odd token count; and feed/channel-level `<georss:where>` (RSS 2.0, RSS 1.0, Atom) previously discarded its bozo signal entirely — an entity-resolution error at feed level is now surfaced too, matching existing entry-level behavior.
  - **Breaking**: `namespace::georss::build_gml_geometry`/`build_gml_envelope` (public API, pre-1.0) changed their return type from `Option<GeoLocation>` to `Result<Option<GeoLocation>, GmlDimsMismatch>` — `Ok(None)` for the pre-existing tolerant-skip cases, `Err(GmlDimsMismatch)` for the new coordinate-count/dims mismatch signal.
- **[P1]** A malformed entity (e.g. a bare `&`) in a channel/feed-level field (`<title>`, `<description>`, `<image>`, `<textInput>`, extension namespaces) aborted the entire RSS 2.0/Atom 1.0 parse, discarding every `<item>`/`<entry>` — even well-formed ones later in the document (#463). Item/entry-level recovery, previously believed to already handle this correctly, had the same defect on multi-field items/entries: a malformed entity in one field (e.g. `<description>`) left the reader positioned mid-item, and the item's *other* fields (`<title>`, `<link>`, which share tag names with real channel/feed fields) leaked into and silently overwrote the real channel/feed metadata instead of being dropped along with the rest of that item. A third variant hit the entry-limit path: skipping a malformed item/entry past `ParserLimits::max_entries` could itself fail and abort the whole parse. All three are fixed by draining the reader to the failing element's own closing tag (nesting-aware, so a same-named nested descendant doesn't stop the drain early, with a bounded iteration cap) before recording `bozo` and continuing, instead of letting the error propagate.
- **[P1]** `FeedHttpClient::with_timeout` had no effect on requests (#451): it only wrote to a `timeout` struct field that `get()` never read, so the underlying `reqwest::blocking::Client` kept its hardcoded 30-second timeout regardless of what callers configured. `get()` now applies the configured timeout per-request via `RequestBuilder::timeout`, and `with_timeout` clamps absurdly large durations (e.g. `Duration::MAX`) to a 1-hour ceiling to avoid an internal overflow panic in `reqwest`'s blocking wait.
  - **Behavior change**: the default 30-second timeout is now also a true total deadline covering connect, all redirect hops, and the full response body, enforced by `reqwest`'s async layer. Previously, with no timeout ever reaching that layer, only the blocking wrapper bounded things — 30s for connect/headers plus a **separate, fresh** 30s for the body read, i.e. up to ~60s wall clock in the worst case. Callers who never call `with_timeout` will now see requests fail faster under slow-body conditions.
- **[P2]** Implemented the GeoRSS GML profile (`georss:where` wrapping `gml:Point`/`gml:LineString`/`gml:Polygon`, via `gml:pos`/`gml:posList`, including `gml:exterior`/`gml:LinearRing` for polygons) across RSS 2.0, RSS 1.0, and Atom parsing (#454). Previously only GeoRSS Simple (`georss:point`/`line`/`polygon`/`box`) was parsed, and `GeoLocation.srs_name` was structurally dead — the field existed but nothing ever set it. The `srsName` attribute is now captured (matched case-insensitively; whitespace-trimmed) and drives axis-order normalization to this crate's `(latitude, longitude)` coordinate convention: geographic CRSes (including the implied WGS84/EPSG:4326 default) and `OGC:CRS84`'s `(lon, lat)` special case are handled explicitly; other/projected EPSG CRSes not in the geographic-axis registry are swapped from `(lon, lat)` and validated as finite rather than degree-ranged, since projected values are typically meters. `srsName` accepts the `EPSG:nnnn`, `urn:ogc:def:crs:EPSG::nnnn`, `http://.../EPSG/0/nnnn`, and classic GML 2 `...epsg.xml#nnnn` forms. `gml:srsDimension="3"` is honored so 3D `gml:posList` values chunk correctly instead of an elevation component silently corrupting the next coordinate pair's latitude. Comma-separated coordinate lists are tolerated (normalized to whitespace), matching GeoRSS Simple's existing tolerance. Malformed or missing GML content is skipped tolerantly (no panic, no bozo) per the existing GeoRSS extended-attribute pattern, except unresolvable entities in `gml:pos`/`gml:posList` text, which set `bozo` at entry level, matching GeoRSS Simple's existing behavior for the same input. `gml:Envelope` and `gml:MultiSurface` were out of scope for this fix; see the #461 entry below for the follow-up implementation.
- **[P2]** Implemented `gml:Envelope` and `gml:MultiSurface` support in the GeoRSS GML profile, closing the gap left by #454 (#461). `gml:Envelope` (via `gml:lowerCorner`/`gml:upperCorner`) is now parsed into a `GeoLocation { geo_type: GeoType::Box, .. }`, the GML equivalent of `georss:box`; each corner is axis-order-normalized independently using the same `srsName`/EPSG-registry rules as `gml:pos`. `gml:MultiSurface` wrapping `gml:surfaceMember`/`gml:Polygon` is now recognized at the `georss:where` dispatch level and descends through the wrapper to the inner polygon using the existing recursive coordinate search. As with the rest of the GML profile, malformed or missing corner text is skipped tolerantly (no panic; the geometry is simply omitted).
- **[P1]** `srsDimension` was only honored when set on the `gml:Point`/`gml:LineString`/`gml:Polygon` root element, silently ignored when set on `gml:pos`/`gml:posList` (or any intermediate wrapper, e.g. `gml:LinearRing`, `gml:Polygon` under `gml:surfaceMember`) — the canonical GML `SRSReferenceGroup` placement and what real-world WFS/GeoServer/INSPIRE feed producers overwhelmingly emit (#470). 3D coordinate text was chunked as 2D pairs instead, corrupting every coordinate after the first tuple without setting `bozo`, since the misaligned values still happened to land inside valid degree ranges. `find_gml_coord_text` now resolves `srsDimension` at every GML element it descends through, nearest-declaring-ancestor wins (a value on `gml:pos`/`gml:posList` beats one on an enclosing wrapper, which beats the geometry root's), matching the precedence `gml:Envelope`'s `gml:lowerCorner`/`gml:upperCorner` already had from #461. A syntactically valid but out-of-range override (anything other than `2`/`3`, e.g. `srsDimension="0"`) is now rejected rather than silently replacing a correct inherited value — this clamp also closes a latent gap in the #461 corner-level precedence, which previously had the same issue. Found during review of this fix: an empty subtree carrying its own valid `srsDimension` (e.g. a `gml:MultiSurface` member with no actual coordinates) could leak that dimensionality into a later, unrelated sibling's dims — `find_gml_coord_text` now only adopts a recursive subtree's resolved `srsDimension` when that subtree actually found coordinate text, not merely because it declared one.
- `parse_podcast_value`'s `</podcast:value>` end guard matched on `starts_with(b"podcast:value")` rather than an exact tag match, so it also matched `</podcast:valueRecipient>` and (once added) `</podcast:valueTimeSplit>` — any feed writing an explicitly-closed `<podcast:valueRecipient>...</podcast:valueRecipient>` (rather than self-closing) silently truncated the rest of its `<podcast:value>` block, dropping any sibling elements that followed. Found while implementing #437; the guard now compares tag names for exact equality
- Node.js binding: regenerated `index.d.ts`/`index.js` via `napi build` to match the actual napi-exported API — the committed `.d.ts` had drifted and was missing the `Cloud`, `MediaCredit`, `MediaRating`, and `TextInput` interfaces along with several fields (`rights`/`rightsDetail`, `guidislink`, `mediaCredit`, `mediaRating`, `avatar`, `docs`, iTunes/Podcast season/episode, and more) on `Entry`, `FeedMeta`, `Image`, `Link`, `MediaContent`, `MediaThumbnail`, `Source`, `ItunesEntryMeta`, `ItunesFeedMeta`, `SyndicationMeta`, and `Person` (#448). The regeneration also fixed a stale hardcoded native-binding version-check string in `index.js` (52 occurrences comparing against `0.4.8` instead of the actual published `0.5.6`), which could throw a spurious "version mismatch" error under `NAPI_RS_ENFORCE_VERSION_CHECK=1`
- Node.js binding: `crates/feedparser-rs-node/package.json`'s `test`/`test:coverage` scripts used an unquoted `__test__/*.spec.mjs` glob, which `node --test` does not expand itself — relying on shell glob expansion silently breaks on `cmd.exe` (Windows CI, `release.yml` pins Node 18/20). Replaced with an explicit file list that works identically across shells and Node versions
- Node.js binding tests: `phase3-fields.spec.mjs` and `syndication.spec.mjs` were never wired into the `test` npm script and were silently skipped in CI; both are now included, and the assertions that only surfaced once the files actually ran have been corrected to match the real binding output (GeoRSS location exposed as `where`, not `geo`; `itunes:explicit` only maps to `true` for "yes"/"true"/"explicit" values, never `false`; `dcDate` is an RFC 3339 string, with the millisecond timestamp on `dcDateParsed`; Media RSS `width`/`height`/`filesize`/`duration`/`updateFrequency` are raw strings, not numbers) (#445)
- Node.js binding: added a structured `FeedError`-to-`napi::Error` conversion layer (`error.rs`), mirroring the Python binding's `convert_feed_error`, so format/encoding/URL errors map to `Status::InvalidArg` and I/O/unknown errors map to `Status::GenericFailure` instead of every call site inlining its own generic error message (#439)
- Node.js binding: regenerated `index.d.ts` via `napi build` to pick up an intra-doc-link fix already present in `src/lib.rs` (`` `core::ParseOptions::default` `` → `` [`core::ParseOptions::default`] ``) — this drift reproduced immediately after #449 merged and is exactly what the new CI guard (#450, above) now prevents from recurring silently
- CI: bump Node.js to 22 in the `npm: Publish` release job; `npm install -g npm@latest` now requires Node `^22.22.2 || ^24.15.0 || >=26.0.0` and failed with `EBADENGINE` under Node 20
- `feedparser-rs-py` now inherits `edition`/`rust-version` from the workspace instead of hardcoding stale values (#434)
- `parse_podcast_podroll`'s and the alternate-enclosure children parser's `</podcast:podroll>`/`</podcast:alternateEnclosure>` end-tag guards matched via `starts_with` rather than an exact tag comparison, the same bug class already fixed for `parse_podcast_value` above — now compare tag names for exact equality (#468)
- `crates/feedparser-rs-core/src/error.rs::test_error_display` and `crates/feedparser-rs-node/src/error.rs::preserves_error_message` asserted on quick-xml's exact `Display` wording, which is outside its semver guarantee and could break on an unrelated quick-xml upgrade; they now check the error variant / a stable, crate-owned message prefix instead (#468)

### Documentation

- `PodcastValue.time_splits`'s field doc now notes that a self-closing `<podcast:valueTimeSplit/>` is silently dropped rather than producing an empty entry, mirroring the existing parser-function doc (#468)

## [0.5.6] - 2026-07-27

### Security

- Update `brace-expansion` to 5.0.8 and `js-yaml` to 4.3.0 (transitive Node.js dev dependencies) to address high-severity `npm audit` advisories (DoS via numeric-range expansion and YAML merge-key chains) (#415)
- Update `ammonia` to 4.1.4 to address RUSTSEC-2026-0213 (XSS via SVG `animate`/`set` animation tags with `javascript:` scheme) (#415)
- Update `quinn-proto` (transitive, via `reqwest`) to 0.11.15 to address RUSTSEC-2026-0185/GHSA-4w2j-m93h-cj5j (remote memory exhaustion from unbounded out-of-order stream reassembly) (#420)

### Fixed

- Root and binding READMEs: corrected nonexistent `fetch_and_parse`/`fetchAndParse` API references to the real `parse_url`/`parseUrl` functions, fixed Node.js binding docs (synchronous `parseUrl`, `bozoException` naming, date field shapes, missing `parseWithOptions`/`parseUrlWithOptions` and HTTP fields, supported Node.js versions), fixed the Python binding's `itunes.duration` example, and bumped a stale version pin in the core crate README (#411)

### Changed

- Bump `compact_str` from 0.9.1 to 0.10.0 (#426)
- Bump `napi` from 3.10.3 to 3.11.0 and `napi-derive` from 3.5.9 to 3.6.0 (#419, #425)
- Bump `regex` from 1.12.4 to 1.13.1 (#414, #419)
- Bump `anyhow`, `memchr`, `serde`, `serde_json`, and `thiserror` in the patch-updates group (#419)
- Bump `@biomejs/biome`, `@napi-rs/cli`, and `c8` (Node.js dev tooling) (#415, #424)
- Bump `actions/labeler` from 6 to 7 (#423)
- Bump `actions/setup-python` from 6 to 7 (#421)
- Bump `actions/setup-node` from 6 to 7 (#416)
- Bump `lewagon/wait-on-check-action` from 1.8.1 to 1.9.0 (#422)

## [0.5.5] - 2026-07-07

### Security

- Update `quick-xml` to 0.41.0 to address RUSTSEC-2026-0195 (unbounded namespace-declaration allocation in `NsReader` enabling memory-exhaustion DoS) (#408)
- Update `ammonia` to 4.1.3 to address an mXSS bypass via MathML `annotation-xml` encoding strip (#408)
- Update `crossbeam-epoch` (dev dependency, via `criterion`) to 0.9.20 to address RUSTSEC-2026-0204 (#408)

### Changed

- Bump `pyo3` from 0.28.3 to 0.29.0 (#395)
- Bump `napi` from 3.9.0 to 3.10.3 across the patch- and minor-updates groups (#398, #400, #402, #406)
- Bump `napi-derive` from 3.5.6 to 3.5.9 (#402, #409)
- Bump `chrono` in the patch-updates group (#394)
- Bump `reqwest`, `compact_str`, and `memchr` in the patch-updates group (#392)
- Bump `memchr`, `regex`, and `napi-sys` in the patch-updates group (#398)
- Bump `anyhow` in the patch-updates group (#402)
- Bump `html-escape` in the patch-updates group (#409)
- Bump `@biomejs/biome` and `@napi-rs/cli` (Node.js dev tooling) (#391, #397, #401, #404)
- Bump `lewagon/wait-on-check-action` from 1.7.0 to 1.8.1 (#396, #403)
- Bump `actions/checkout` from 6 to 7 (#399)
- Bump `codecov/codecov-action` from 6 to 7 (#393)

## [0.5.4] - 2026-05-26

### Changed

- Bump `napi` to 3.9.0 in the Node.js bindings (#387)
- Bump `quick-xml` from 0.39.4 to 0.40.1 (#388)
- Bump `serde_json` patch version (#389)
- Bump `napi-build` to 2.3 and `napi-derive` to 3.5.6 in the Node.js bindings (#386)
- Bump `quick-xml` patch version (#385)
- Bump `@biomejs/biome` (Node.js dev tooling) (#381, #382, #384)
- Various transitive dependency patch updates via Dependabot (#383)

## [0.5.3] - 2026-04-24

### Added

- feat(core): parse `media:title` element into `entry.media_title` for RSS and Atom feeds (including inside `<media:group>`); exposed in Python and Node.js bindings (#363)

### Security

- Update `rustls-webpki` to 0.103.13 to address RUSTSEC-2026-0104

## [0.5.2] - 2026-04-06

### Added

- Core: parse 8 additional Podcast 2.0 namespace elements: `podcast:transcript`, `podcast:alternateEnclosure` (with `podcast:source` and `podcast:integrity` children), `podcast:location`, `podcast:podroll` (with `podcast:remoteItem` children), `podcast:socialInteract`, `podcast:txt`, `podcast:updateFrequency`, `podcast:follow`; new types `PodcastAlternateEnclosure`, `PodcastAlternateEnclosureSource`, `PodcastIntegrity`, `PodcastLocation`, `PodcastRemoteItem`, `PodcastSocialInteract`, `PodcastTxt`, `PodcastUpdateFrequency`, `PodcastFollow` added to public API (#351)
- JSON Feed 1.1 `hubs` array is now parsed into `feed.links` with `rel="hub"`; hub `type` field is stored as `link_type` (#359)
- Core, Python, Node.js bindings: `GeoLocation` now exposes `elev` (elevation in meters), `feature_type_tag`, `feature_name`, and `relationship_tag` fields from `georss:elev`, `georss:featuretypetag`, `georss:featurename`, and `georss:relationshiptag` elements; geometry handlers use merge pattern to preserve extended attributes regardless of element order (#355)
- Core, Python, Node.js bindings: `PodcastEntryMeta.season` and `PodcastEntryMeta.episode` fields parse `podcast:season` and `podcast:episode` elements via the `number` attribute (Podcast 2.0 spec) (#332)
- Python binding: `PyPodcastValue` and `PyPodcastValueRecipient` classes expose existing `podcast:value` data to Python; `PodcastMeta.value` getter added to `PyPodcastMeta` (#337)

### Changed

- `feed.skiphours` returns `Vec<u32>` with correctly parsed hour values and `feed.skipdays` returns `Vec<String>` with day names, versus Python feedparser which incorrectly returns empty strings for both fields. `feed.textinput` returns a populated `TextInput` struct with all child fields, versus Python feedparser which returns an empty dict. This is an intentional improvement over Python feedparser's behavior (#336).

### Fixed

- Dublin Core: `dc:date` now sets `entry.published` (as fallback when not already set) in addition to `entry.updated`, matching Python feedparser behavior (#354)
- JSON Feed: entries without `authors` now inherit feed-level `authors` per the JSON Feed spec (#356)
- RSS 1.0: self-closing `<image rdf:resource="..."/>` and `<textinput rdf:resource="..."/>` inside `<channel>` no longer consume subsequent events; `skip_element` is now skipped for `Event::Empty` throughout the RSS 1.0 parser, fixing item loss when these reference elements appear in the channel block (#345)
- Python binding: `Cloud.registerprocedure` (no underscore) now correctly exposed as `registerprocedure` to match Python feedparser API (#335)
- Node.js binding: `cloud.registerprocedure` (no underscore) now correctly exposed as `registerprocedure` instead of `registerProcedure` for Python feedparser compatibility (#335)
- Namespace extension parsers (Dublin Core, Media RSS, iTunes) now resolve namespace URIs instead of matching only hardcoded prefixes; feeds using non-standard prefixes (e.g. `xmlns:dublin="http://purl.org/dc/elements/1.1/"`) are correctly parsed (#334)
- `podcast:season` and `podcast:episode` now read element text content as primary value per Podcast 2.0 spec, with `number` attribute as fallback; fixes feeds like TWiT that use text content (#348)
- Atom entries with no `<author>` element now inherit feed-level authors per RFC 4287 §4.1.2; previously entries were returned with empty authors when the feed defined authors (#353)
- Null bytes (U+0000) in XML text content are now silently stripped from all parsed text fields (titles, descriptions, authors, etc.) per XML 1.0 §2.2 (#352)

## [0.5.1] - 2026-03-24

### Added

- Core, Python, Node.js bindings: `PodcastEntryMeta.medium` field exposes `podcast:medium` at entry/item level (#320)
- Core, Python, Node.js bindings: `PodcastMeta.locked` and `PodcastMeta.locked_owner` fields parse `podcast:locked` feed-level element (Podcast 2.0 spec) (#213)
- Core, Python, Node.js bindings: `PodcastMeta.medium` field exposes `podcast:medium` feed-level element (Podcast 2.0 spec: content type string such as `podcast`, `music`, `video`, etc.) (#255)
- Core: `podcast:person` elements at feed/channel level are now collected into `feed.podcast.persons`; previously only entry-level persons were parsed (#292)
- Core, Python, Node.js bindings: `FeedMeta.summary` and `FeedMeta.summary_detail` fields populated from `itunes:summary` (#257)
- Python binding: `feed.summary` and `feed["summary"]` now return the `itunes:summary` value (#257)
- Core, Python, Node.js bindings: `entry.external_url` populated from JSON Feed `item.external_url` (#196)
- Core, Python, Node.js bindings: `entry.language` populated from JSON Feed `item.language` (#227)
- Core: `language` and `base` fields on `TextConstruct` and `Content` are now populated from `xml:lang`/`xml:base` in Atom feeds, including element-level overrides (#137)
- Core: `language` and `base` fields on `TextConstruct` and `Content` are now populated in RSS 1.0 (RDF) feeds from `<rdf:RDF xml:lang>` and `<item xml:lang>` attributes (#137)
- Core: `content:encoded` elements in RSS 2.0 and RSS 1.0 now carry `language` and `base` from the surrounding parse context (#137)
- Core: JSON Feed feed-level `language` field now propagates to item `TextConstruct`/`Content` language when item lacks its own `language` (#137)
- Core: JSON Feed feed-level `language` now propagates to `feed.title_detail` and `feed.subtitle_detail` (#137)
- Core, Python, Node.js bindings: `enclosure.title` populated from JSON Feed attachment `title` (#196)
- Core, Python, Node.js bindings: `enclosure.duration` populated from JSON Feed attachment `duration_in_seconds` as raw string (#196)
- Core, Python, Node.js bindings: `Person.avatar` field populated from JSON Feed author `avatar` URL (#210)
- Core, Python, Node.js bindings: `entry.source` now exposes `links` (all link elements), `updated`/`updated_parsed`, `rights`, and `guidislink` fields for Atom `<source>` elements, matching Python feedparser (#242, #214)
- Core: `entry.source.guidislink` is `Some(true)` when the Atom `<source>` `<id>` looks like a URL and no explicit `<link>` is present; `Some(false)` when an explicit `<link>` is present or the id is not a URL; `None` for RSS sources

### Changed

- **BREAKING**: `PodcastEntryMeta.person` renamed to `PodcastEntryMeta.persons` for consistency with feed-level `PodcastMeta.persons` (#320)
- **BREAKING**: `ItunesFeedMeta.complete` changed from `Option<bool>` to `Option<String>` to return the raw XML text value (e.g. `"Yes"`, `"No"`) instead of a parsed boolean (#281)
- `itunes:subtitle` now always overrides `<description>` for `feed.subtitle` regardless of XML element order; previously it only set subtitle when absent (#257)
- `itunes:summary` populates new `feed.summary` field instead of aliasing to `feed.subtitle` (#257)
- Entry-level `itunes:subtitle` and `itunes:summary` promotion is now order-independent via post-processing (#257)
- Atom entry `itunes:subtitle` now promotes to `entry.subtitle` (was missing) (#257)
- **BREAKING**: `entry.source.link` renamed to `entry.source.href` in core Rust type and Node.js bindings for Python feedparser API compatibility; Python binding retains `source.link` as an alias for `source.href` (#240)

### Fixed

- Core: `itunes:category` tags now have `scheme='http://www.itunes.com/'` and `label=None`, matching Python feedparser; previously `scheme` was `None` and `label` was a copy of `term` (#325)
- Core: `itunes:category` elements at channel/feed level are now mapped to `feed.tags` (term and label set to category text, scheme None), matching Python feedparser behavior (#204)
- Core: Atom 0.3 `<tagline>` is now mapped to `feed.subtitle`/`feed.subtitle_detail` and `<copyright>` to `feed.rights`/`feed.rights_detail` (#203)
- Core: RSS `<generator>` now populates `feed.generator_detail` with `name` set (matching Python feedparser behavior); previously only `feed.generator` was set (#254)
- Core: `slash:hit_parade` is now parsed from RSS entries into `entry.slash_hit_parade`; also fixed `extract_ns_local_name` to allow underscores in namespace-local tag names (#244)
- Core, Python, Node.js bindings: RSS 2.0 optional channel elements `<cloud>`, `<textInput>`, `<skipHours>`, `<skipDays>` are now parsed and exposed as `feed.cloud`, `feed.textinput`, `feed.skiphours`, `feed.skipdays` (#200)
- Core: `podcast:person` default role changed from `"unknown"` to `"host"` per Podcast 2.0 spec; Python binding now returns `"host"` instead of `None` when no `role` attribute is present (#236)
- Core: `itunes:summary`-only no longer incorrectly sets `feed.subtitle`; only `itunes:subtitle` promotes to `feed.subtitle` (#308)
- Core: JSON Feed `icon` field now correctly maps to `feed.icon`; `favicon` field now correctly maps to `feed.logo` (previously `icon` was mapped to `feed.image` and `favicon` to `feed.icon`) (#329)
- Core: `feed.author` now uses `itunes:owner.name` (with email in `author_detail`) when both `itunes:owner` and `itunes:author` are present, matching Python feedparser priority (#297)
- Core: `feed.author` from `itunes:owner` now contains name only (no `"Name (email)"` format); `feed.author_detail` still carries both name and email (#317)
- Core: `&apos;` and `&quot;` entity references inside xhtml content now decode to literal `'` and `"` characters instead of passing through as escaped entity refs (#316)
- Core: `itunes:image` now overrides RSS `<image>` for `feed.image` regardless of element order, matching Python feedparser behavior (#287)
- Core, Python, Node.js bindings: `MediaContent.filesize` is now a `String` (was `u64`/`i64`), matching Python feedparser which preserves raw attribute values; non-numeric values like `"not_a_number"` are now retained as-is (#221)
- Core, Python, Node.js bindings: `media:credit`, `media:copyright`, `media:rating`, `media:keywords`, and `media:description` are now parsed from RSS and Atom feeds and exposed on `Entry` as `media_credit`, `media_copyright`, `media_rating`, `media_keywords`, and `media_description` (#246, #288)
- Core, Python, Node.js bindings: `media:rating` and `media:keywords` are now parsed at feed level and exposed as `feed.media_rating` / `feed.media_keywords` (#302, #208)
- Core, Python, Node.js bindings: `media:thumbnail` now parses the `time` attribute (NTP offset string) and exposes it as `MediaThumbnail.time: Option<String>` (#229)
- Core: XHTML serializer now correctly re-emits entity references (`&amp;`, `&lt;`, `&gt;`) that quick-xml emits as `GeneralRef` events; previously they were silently dropped producing bare `&` / `<` in output (#215)
- Core, Python, Node.js bindings: Atom 0.3 `<created>` element now maps to `entry.created` / `entry.created_str` (raw date string) consistent with `published_str` / `updated_str`; previously the field was always `None` (#301)
- Core: date parser now handles ASCTIME format `Www Mmm [D]D HH:MM:SS YYYY` with optional space-padded single-digit day (e.g. `Mon Jan  6 12:30:00 2025`) (#258)
- Core: JSON Feed `icon` correctly maps to `feed.image` (large timeline image) and `favicon` to `feed.icon` (small browser icon); previously the mapping was reversed (#323)
- Core: `Entry.created_str` field added to preserve raw Atom 0.3 `<created>` date string (#301)
- Core: `media:content` attributes `bitrate`, `channels`, `samplingrate`, and `framerate` are now parsed and exposed as strings on `MediaContent`, matching Python feedparser behavior (#294, #253)
- Core, Python, Node.js bindings: `media:thumbnail` elements nested inside `<media:content>` are now collected into `entry.media_thumbnail` alongside top-level thumbnails, matching Python feedparser behavior (#270)
- Python bindings: `bitrate`, `channels`, `samplingrate`, `framerate`, `lang`, `codec`, `expression`, and `isdefault` attributes are now accessible via `MediaContent.__getitem__` / `__contains__` / `keys` / `values` dict protocol (#294)
- Core: `feed.publisher` flat field is now populated with the raw `<webMaster>` string (e.g. `"webmaster@example.com (Web Master)"`) matching how `feed.author` is populated from `<managingEditor>`; previously `feed.publisher` was `None` (#277, #218)
- Core: `itunes:owner` in RSS feeds now promotes to `feed.publisher_detail` (name + email) if no publisher is already set; existing publisher from `<webMaster>` is not overridden (#280)
- Core: `itunes:owner` was already parsed in Atom feeds with both `<itunes:name>` and `<itunes:email>` children; no change needed (#266)
- Core: `itunes:explicit` values `"false"`, `"no"`, `"clean"` now return `None` (not `Some(false)`); only `"yes"`, `"true"`, `"explicit"` return `Some(true)` — matches Python feedparser behavior (#206)
- Python bindings: expose flat `itunes_block`, `itunes_complete`, `itunes_type`, `itunes_new-feed-url` fields directly on feed dict, matching Python feedparser API (#232)
- Python bindings: expose flat `sy_updateperiod`, `sy_updatefrequency`, `sy_updatebase` fields directly on feed dict, matching Python feedparser API (#293)
- Python bindings: `PodcastPerson` now supports dict protocol (`__getitem__`, `get`, `keys`, `values`, `items`), consistent with other struct types (#300)
- `itunes:duration` confirmed as string type in core and all bindings (regression test added) (#265)
- Core: syndication module (`syn:`/`sy:` namespace) is now parsed in RSS 2.0 feeds; previously only RSS 1.0 feeds were supported — RSS 2.0 feeds with `<syn:updatePeriod>` etc. returned `feed.syndication = None` (#237)
- Core: `syn:updateFrequency` / `sy:updateFrequency` now returns the raw string value (e.g. `"2"`) instead of an integer, matching Python feedparser behavior (#268, #220)
- Python bindings: expose `thr:in-reply-to` as `entry['thr_in-reply-to']` returning the first element as a plain dict with keys `ref`, `href`, `type`, `source` (non-None only), matching Python feedparser API; `entry.thr_in_reply_to` (underscore) retains the full list of `InReplyTo` objects (#267, #245)
- Core, Python, Node.js bindings: Atom `<source><link href="..."/>` now populates `entry.source.link` (new field); `entry.source.href` remains for RSS `<source url="...">` only (#262)
- Core, Python, Node.js bindings: Atom `<source><author>` is now exposed as `entry.source.author` flat string in `"Name (email)"` format (#262)
- Core, Python, Node.js bindings: Atom `<content src="...">` (out-of-line content per RFC 4287 §4.1.3.2) is now parsed — `content.src` is set to the URL, `content.value` is empty string, `content.type` is set from the `type` attribute (#252)
- Core, Python, Node.js bindings: Atom flat `author` string now uses `"Name (email)"` format when email is present; previously only the name was used (#251)
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
- Core: Atom `entry.guidislink` is now `Some(true)` when `entry.link` is promoted from `entry.id` (no explicit `<link>`), and `Some(false)` when an explicit `<link>` is present; previously always hardcoded to `Some(false)` (#285)
- Core: `dc:creator` in Atom entries is now used as fallback for `entry.author` when no `<author>` element is present, matching RSS behavior and Python feedparser (#278)
- Core: RSS 0.92 and 0.90 feeds now report `"rss092"` and `"rss090"` instead of `"rss20"`; RSS 0.91 with Netscape DOCTYPE reports `"rss091n"`, without DOCTYPE reports `"rss091u"`, matching Python feedparser behavior (#283)
- Core: `georss:point`, `georss:polygon`, and `georss:line` are now parsed in Atom `<entry>` elements and populate `entry.where`; previously only RSS `<item>` elements were supported (#291)
- Core, Python, Node.js bindings: `geo:lat` and `geo:long` (W3C Basic Geo namespace) are now parsed at feed and entry level; `feed.geo_lat`, `feed.geo_long`, `entry.geo_lat`, `entry.geo_long` are exposed as flat strings; `feed.where`/`entry.where` are auto-constructed as a GeoJSON Point when both are present (#248)

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

[Unreleased]: https://github.com/bug-ops/feedparser-rs/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/bug-ops/feedparser-rs/compare/v0.5.6...v0.6.0
[0.5.6]: https://github.com/bug-ops/feedparser-rs/compare/v0.5.5...v0.5.6
[0.5.5]: https://github.com/bug-ops/feedparser-rs/compare/v0.5.4...v0.5.5
[0.5.4]: https://github.com/bug-ops/feedparser-rs/compare/v0.5.3...v0.5.4
[0.5.3]: https://github.com/bug-ops/feedparser-rs/compare/v0.5.2...v0.5.3
[0.5.2]: https://github.com/bug-ops/feedparser-rs/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/bug-ops/feedparser-rs/compare/v0.5.0...v0.5.1
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
