# feedparser-rs

[![Crates.io](https://img.shields.io/crates/v/feedparser-rs)](https://crates.io/crates/feedparser-rs)
[![docs.rs](https://img.shields.io/docsrs/feedparser-rs)](https://docs.rs/feedparser-rs)
[![MSRV](https://img.shields.io/crates/msrv/feedparser-rs)](https://github.com/bug-ops/feedparser-rs)
[![License](https://img.shields.io/crates/l/feedparser-rs)](LICENSE)

High-performance RSS/Atom/JSON Feed parser written in Rust.

This is the core parsing library that powers the Python and Node.js bindings.

## Features

- **Multi-format**: RSS 0.9x/1.0/2.0, Atom 0.3/1.0, JSON Feed 1.0/1.1
- **Tolerant**: Bozo flag for graceful handling of malformed feeds
- **Fast**: Native Rust performance (200+ MB/s throughput)
- **Safe**: No unsafe code, comprehensive error handling
- **HTTP support**: Fetch feeds from URLs with compression and conditional GET
- **Podcast support**: iTunes and Podcast 2.0 namespace extensions
- **Namespace extensions**: Dublin Core, Media RSS, Syndication, GeoRSS, Creative Commons
- **Well-tested**: 91%+ test coverage with real-world feed fixtures

## Installation

```bash
cargo add feedparser-rs
```

Or add to your `Cargo.toml`:

```toml
[dependencies]
feedparser-rs = "0.2"
```

> [!IMPORTANT]
> Requires Rust 1.88.0 or later (edition 2024).

## Quick Start

```rust
use feedparser_rs::parse;

let xml = r#"
    <?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>My Blog</title>
            <item>
                <title>Hello World</title>
                <link>https://example.com/1</link>
            </item>
        </channel>
    </rss>
"#;

let feed = parse(xml.as_bytes())?;
assert_eq!(feed.feed.title.as_deref(), Some("My Blog"));
assert_eq!(feed.entries.len(), 1);
# Ok::<(), feedparser_rs::FeedError>(())
```

## HTTP Fetching

Fetch feeds directly from URLs with automatic compression handling:

```rust
use feedparser_rs::parse_url;

let feed = parse_url("https://example.com/feed.xml", None, None, None)?;
println!("Title: {:?}", feed.feed.title);
println!("Entries: {}", feed.entries.len());

// Subsequent fetch with caching (uses ETag/Last-Modified)
let feed2 = parse_url(
    "https://example.com/feed.xml",
    feed.etag.as_deref(),
    feed.modified.as_deref(),
    None
)?;

if feed2.status == Some(304) {
    println!("Not modified, use cached version");
}
# Ok::<(), feedparser_rs::FeedError>(())
```

> [!TIP]
> Use conditional GET with ETag/Last-Modified to minimize bandwidth when polling feeds.

To disable HTTP support and reduce dependencies:

```toml
[dependencies]
feedparser-rs = { version = "0.2", default-features = false }
```

## Cargo Features

| Feature | Description | Default |
|---------|-------------|---------|
| `http` | URL fetching with reqwest (gzip/deflate/brotli) | Yes |

## Error Handling

The library uses a "bozo" flag (like Python's feedparser) to indicate parsing errors while still returning partial results:

```rust
use feedparser_rs::parse;

let malformed = b"<rss><channel><title>Broken</title></rss>";
let feed = parse(malformed)?;

assert!(feed.bozo);
assert!(feed.bozo_exception.is_some());
// Still can access parsed data
assert_eq!(feed.feed.title.as_deref(), Some("Broken"));
# Ok::<(), feedparser_rs::FeedError>(())
```

## Parser Limits

To prevent resource exhaustion (DoS protection), the parser enforces configurable limits:

```rust
use feedparser_rs::{parse_with_limits, ParserLimits};

let limits = ParserLimits {
    max_entries: 100,
    max_nesting_depth: 20,
    ..Default::default()
};

let feed = parse_with_limits(xml.as_bytes(), limits)?;
# Ok::<(), feedparser_rs::FeedError>(())
```

> [!NOTE]
> Default limits are generous for typical feeds. Use `ParserLimits::strict()` for untrusted input.

## Benchmarks

Measured on Apple M1 Pro:

| Feed Size | Time | Throughput |
|-----------|------|------------|
| Small (2 KB) | 10.7 µs | 187 MB/s |
| Medium (20 KB) | 93.6 µs | 214 MB/s |
| Large (200 KB) | 939 µs | 213 MB/s |

Format detection: 128 ns

Run benchmarks:

```bash
cargo bench
```

## Platform Bindings

- **Node.js**: [`feedparser-rs`](https://www.npmjs.com/package/feedparser-rs) on npm
- **Python**: [`feedparser-rs`](https://pypi.org/project/feedparser-rs/) on PyPI

## MSRV Policy

Minimum Supported Rust Version: **1.88.0** (edition 2024).

MSRV increases are considered breaking changes and will result in a minor version bump.

## License

Licensed under either of:

- [Apache License, Version 2.0](../../LICENSE-APACHE)
- [MIT License](../../LICENSE-MIT)

at your option.

## Links

- [GitHub](https://github.com/bug-ops/feedparser-rs)
- [API Documentation](https://docs.rs/feedparser-rs)
- [Changelog](../../CHANGELOG.md)
