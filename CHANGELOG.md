# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- JSON Feed 1.1: parse `next_url` feed-level field into `FeedMeta.next_url: Option<String>` (#112)
- JSON Feed 1.1: parse `banner_image` entry-level field, stored as `Link` with `rel="banner"` in `entry.links` (#112)
- `Link::banner()` constructor for creating banner image links (project-internal convention)
- Expose `next_url` in Python bindings via `#[getter]`, `__getattr__`, and `__getitem__`
- Expose `next_url` in Node.js bindings as `FeedMeta.next_url`
- Parse `<subtitle>` element at the Atom entry level: `Entry` now exposes `subtitle: Option<String>` and `subtitle_detail: Option<TextConstruct>`, mirroring the existing feed-level subtitle fields (#110)
- Expose `subtitle` and `subtitle_detail` on `Entry` in Python (PyO3) and Node.js (napi-rs) bindings (#110)

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

[Unreleased]: https://github.com/bug-ops/feedparser-rs/compare/v0.4.7...HEAD
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
