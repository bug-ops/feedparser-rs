# feedparser-rs

[![Crates.io](https://img.shields.io/crates/v/feedparser-rs)](https://crates.io/crates/feedparser-rs)
[![docs.rs](https://img.shields.io/docsrs/feedparser-rs)](https://docs.rs/feedparser-rs)
[![PyPI](https://img.shields.io/pypi/v/feedparser-rs)](https://pypi.org/project/feedparser-rs/)
[![npm](https://img.shields.io/npm/v/feedparser-rs)](https://www.npmjs.com/package/feedparser-rs)
[![CI](https://img.shields.io/github/actions/workflow/status/bug-ops/feedparser-rs/ci.yml?branch=main)](https://github.com/bug-ops/feedparser-rs/actions)
[![codecov](https://codecov.io/gh/bug-ops/feedparser-rs/graph/badge.svg)](https://codecov.io/gh/bug-ops/feedparser-rs)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE-MIT)

High-performance RSS/Atom/JSON Feed parser written in Rust, with Python and Node.js bindings.

## Features

- **Multi-format support** — RSS 0.9x, 1.0, 2.0 / Atom 0.3, 1.0 / JSON Feed 1.0, 1.1
- **Tolerant parsing** — Handles malformed feeds gracefully with `bozo` flag pattern
- **HTTP fetching** — Built-in URL fetching with compression (gzip, deflate, brotli)
- **Conditional GET** — ETag/Last-Modified support for bandwidth-efficient polling
- **Podcast support** — iTunes and Podcast 2.0 namespace extensions
- **Multi-language bindings** — Native Python (PyO3) and Node.js (napi-rs) bindings
- **Familiar API** — Inspired by Python's feedparser, easy to migrate existing code

## Supported Formats

| Format | Versions | Status |
|--------|----------|--------|
| RSS | 0.90, 0.91, 0.92, 1.0, 2.0 | ✅ Full support |
| Atom | 0.3, 1.0 | ✅ Full support |
| JSON Feed | 1.0, 1.1 | ✅ Full support |

### Namespace Extensions

| Namespace | Description |
|-----------|-------------|
| Dublin Core | Creator, date, rights metadata |
| Content | Encoded HTML content |
| Media RSS | Media attachments and metadata |
| iTunes | Podcast metadata (author, duration, explicit) |
| Podcast 2.0 | Chapters, transcripts, funding |
| Syndication | Update schedule (period, frequency, base) |
| GeoRSS | Geographic location data (point, line, polygon, box) |
| Creative Commons | License information with `rel="license"` links |

## Installation

### Rust

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

### Node.js

```bash
npm install feedparser-rs
# or
yarn add feedparser-rs
# or
pnpm add feedparser-rs
```

### Python

```bash
pip install feedparser-rs
```

## Usage

### Rust

```rust
use feedparser_rs::parse;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"
        <?xml version="1.0"?>
        <rss version="2.0">
            <channel>
                <title>Example Feed</title>
                <link>https://example.com</link>
                <item>
                    <title>First Post</title>
                    <link>https://example.com/post/1</link>
                </item>
            </channel>
        </rss>
    "#;

    let feed = parse(xml.as_bytes())?;

    println!("Version: {}", feed.version.as_str());  // "rss20"
    println!("Title: {:?}", feed.feed.title);
    println!("Entries: {}", feed.entries.len());

    for entry in &feed.entries {
        println!("  - {:?}", entry.title);
    }

    Ok(())
}
```

#### Fetching from URL

```rust
use feedparser_rs::fetch_and_parse;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let feed = fetch_and_parse("https://example.com/feed.xml")?;
    println!("Fetched {} entries", feed.entries.len());
    Ok(())
}
```

> [!TIP]
> Use `fetch_and_parse` for URL fetching with automatic compression handling (gzip, deflate, brotli).

### Node.js

```javascript
import { parse, fetchAndParse } from 'feedparser-rs';

// Parse from string
const feed = parse('<rss version="2.0">...</rss>');
console.log(feed.version);  // 'rss20'
console.log(feed.feed.title);
console.log(feed.entries.length);

// Fetch from URL
const remoteFeed = await fetchAndParse('https://example.com/feed.xml');
```

See [Node.js API documentation](crates/feedparser-rs-node/README.md) for complete reference.

### Python

```python
import feedparser_rs

# Parse from bytes or string
d = feedparser_rs.parse(b'<rss>...</rss>')
print(d.version)       # 'rss20'
print(d.feed.title)
print(d.bozo)          # True if parsing had issues
print(d.entries[0].published_parsed)  # time.struct_time
```

> [!NOTE]
> Python bindings provide `time.struct_time` for date fields, matching feedparser's API for easy migration.

## Cargo Features

| Feature | Description | Default |
|---------|-------------|---------|
| `http` | Enable URL fetching with reqwest (gzip/deflate/brotli support) | Yes |

To disable HTTP support and reduce dependencies:

```toml
[dependencies]
feedparser-rs = { version = "0.2", default-features = false }
```

## Workspace Structure

| Crate | Description | Package |
|-------|-------------|---------|
| [`feedparser-rs`](crates/feedparser-rs-core) | Core Rust parser | [crates.io](https://crates.io/crates/feedparser-rs) |
| [`feedparser-rs-node`](crates/feedparser-rs-node) | Node.js bindings | [npm](https://www.npmjs.com/package/feedparser-rs) |
| [`feedparser-rs-py`](crates/feedparser-rs-py) | Python bindings | [PyPI](https://pypi.org/project/feedparser-rs) |

## Development

```bash
# Install cargo-make
cargo install cargo-make

# Run all checks (format, lint, test)
cargo make ci-all

# Run tests with coverage
cargo make coverage

# Run benchmarks
cargo make bench
```

See all available tasks:

```bash
cargo make --list-all-steps
```

## Benchmarks

Measured on Apple M1 Pro, parsing real-world RSS feeds:

| Feed Size | Time | Throughput |
|-----------|------|------------|
| Small (2 KB) | **10.7 µs** | 187 MB/s |
| Medium (20 KB) | **93.6 µs** | 214 MB/s |
| Large (200 KB) | **939 µs** | 213 MB/s |

Format detection: **128 ns** (near-instant)

### vs Python feedparser

| Operation | feedparser-rs | Python feedparser | Speedup |
|-----------|---------------|-------------------|---------|
| Parse 20 KB RSS | 0.09 ms | 8.5 ms | **94x** |
| Parse 200 KB RSS | 0.94 ms | 85 ms | **90x** |

> [!TIP]
> Run your own benchmarks with `cargo bench` or compare against Python with `cargo make bench-compare`.

## MSRV Policy

Minimum Supported Rust Version: **1.88.0** (edition 2024).

MSRV increases are considered breaking changes and will result in a minor version bump.

## License

Licensed under either of:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.

## Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) before submitting a pull request.

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).
