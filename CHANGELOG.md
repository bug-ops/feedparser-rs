# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/bug-ops/feedparser-rs/compare/v0.4.0...HEAD
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
