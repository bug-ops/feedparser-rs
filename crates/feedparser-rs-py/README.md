# feedparser-rs

[![PyPI](https://img.shields.io/pypi/v/feedparser-rs)](https://pypi.org/project/feedparser-rs/)
[![Python](https://img.shields.io/pypi/pyversions/feedparser-rs)](https://pypi.org/project/feedparser-rs/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE-MIT)

High-performance RSS/Atom/JSON Feed parser for Python with feedparser-compatible API.

## Features

- **Fast**: Native Rust implementation via PyO3
- **HTTP fetching**: Built-in URL fetching with compression (gzip, deflate, brotli)
- **Conditional GET**: ETag/Last-Modified support for efficient polling
- **Tolerant parsing**: Bozo flag for graceful handling of malformed feeds
- **Multi-format**: RSS 0.9x/1.0/2.0, Atom 0.3/1.0, JSON Feed 1.0/1.1
- **Podcast support**: iTunes and Podcast 2.0 namespace extensions
- **feedparser-compatible**: Dict-style access, field aliases, same API patterns
- **DoS protection**: Built-in resource limits

## Installation

```bash
pip install feedparser-rs
```

> [!IMPORTANT]
> Requires Python 3.10 or later.

## Usage

### Basic Parsing

```python
import feedparser_rs

# Parse from string, bytes, or URL (auto-detected)
d = feedparser_rs.parse('<rss>...</rss>')
d = feedparser_rs.parse(b'<rss>...</rss>')
d = feedparser_rs.parse('https://example.com/feed.xml')  # URL auto-detected

# Attribute-style access (feedparser-compatible)
print(d.feed.title)
print(d.version)  # "rss20", "atom10", etc.
print(d.bozo)     # True if parsing errors occurred

# Dict-style access (feedparser-compatible)
print(d['feed']['title'])
print(d['entries'][0]['link'])

for entry in d.entries:
    print(entry.title)
    print(entry.published_parsed)  # time.struct_time
```

> [!NOTE]
> Date fields like `published_parsed` return `time.struct_time` for feedparser compatibility.

### Fetching from URL

```python
import feedparser_rs

# Option 1: Auto-detection (recommended)
d = feedparser_rs.parse('https://example.com/feed.xml')

# Option 2: Explicit URL function
d = feedparser_rs.parse_url('https://example.com/feed.xml')

# With conditional GET for efficient polling
d = feedparser_rs.parse(
    'https://example.com/feed.xml',
    etag=cached_etag,
    modified=cached_modified
)
if d.status == 304:
    print("Feed not modified")

# With custom limits
limits = feedparser_rs.ParserLimits(max_entries=100)
d = feedparser_rs.parse_with_limits('https://example.com/feed.xml', limits=limits)
```

> [!TIP]
> URL fetching supports automatic compression (gzip, deflate, brotli) and follows redirects.

## Migration from feedparser

feedparser-rs is designed as a drop-in replacement for Python feedparser:

```python
# Drop-in replacement
import feedparser_rs as feedparser

# Same API patterns work
d = feedparser.parse('https://example.com/feed.xml')
print(d.feed.title)
print(d['feed']['title'])  # Dict-style access works too
print(d.entries[0].link)

# Deprecated field names supported
print(d.feed.description)  # → d.feed.subtitle
print(d.channel.title)     # → d.feed.title
print(d.items[0].guid)     # → d.entries[0].id
```

### Supported Field Aliases

| Old Name | Maps To |
|----------|---------|
| `feed.description` | `feed.subtitle` or `feed.summary` |
| `feed.tagline` | `feed.subtitle` |
| `feed.copyright` | `feed.rights` |
| `feed.modified` | `feed.updated` |
| `channel` | `feed` |
| `items` | `entries` |
| `entry.guid` | `entry.id` |
| `entry.description` | `entry.summary` |
| `entry.issued` | `entry.published` |

## Advanced Usage

### Custom Resource Limits

```python
import feedparser_rs

limits = feedparser_rs.ParserLimits(
    max_feed_size_bytes=50_000_000,  # 50 MB
    max_entries=5_000,
    max_authors=20,
    max_links_per_entry=50,
)

d = feedparser_rs.parse_with_limits(feed_data, limits=limits)
```

### Format Detection

```python
import feedparser_rs

version = feedparser_rs.detect_format(feed_data)
print(version)  # "rss20", "atom10", "json11", etc.
```

### Podcast Support

```python
import feedparser_rs

d = feedparser_rs.parse(podcast_feed)

# iTunes metadata
if d.feed.itunes:
    print(d.feed.itunes.author)
    print(d.feed.itunes.categories)

# Episode metadata
for entry in d.entries:
    if entry.itunes:
        print(f"Duration: {entry.itunes.duration}s")
```

## API Reference

### Functions

- `parse(source, etag=None, modified=None, user_agent=None)` — Parse feed from bytes, str, or URL (auto-detected)
- `parse_url(url, etag=None, modified=None, user_agent=None)` — Fetch and parse feed from URL
- `parse_with_limits(source, etag=None, modified=None, user_agent=None, limits=None)` — Parse with custom resource limits
- `parse_url_with_limits(url, etag=None, modified=None, user_agent=None, limits=None)` — Fetch and parse with custom limits
- `detect_format(source)` — Detect feed format without full parsing

### Classes

- `FeedParserDict` — Parsed feed result (supports both attribute and dict-style access)
  - `.feed` / `['feed']` — Feed metadata
  - `.entries` / `['entries']` — List of entries
  - `.bozo` — True if parsing errors occurred
  - `.version` — Feed version string
  - `.encoding` — Character encoding
  - `.status` — HTTP status code (for URL fetches)
  - `.etag` — ETag header (for conditional GET)
  - `.modified` — Last-Modified header (for conditional GET)

- `ParserLimits` — Resource limits configuration

## Performance

Benchmarks vs Python feedparser on Apple M1 Pro:

| Operation | feedparser-rs | Python feedparser | Speedup |
|-----------|---------------|-------------------|---------|
| Parse 2 KB RSS | 0.01 ms | 0.9 ms | **90x** |
| Parse 20 KB RSS | 0.09 ms | 8.5 ms | **94x** |
| Parse 200 KB RSS | 0.94 ms | 85 ms | **90x** |

> [!TIP]
> For maximum performance, pass `bytes` instead of `str` to avoid UTF-8 re-encoding.

## Platform Support

Pre-built wheels available for:

| Platform | Architecture |
|----------|--------------|
| macOS | Intel (x64), Apple Silicon (arm64) |
| Linux | x64, arm64 |
| Windows | x64 |

Supported Python versions: 3.10, 3.11, 3.12, 3.13, 3.14

## Development

```bash
git clone https://github.com/bug-ops/feedparser-rs
cd feedparser-rs/crates/feedparser-rs-py
pip install maturin
maturin develop
```

## License

Licensed under either of:

- [Apache License, Version 2.0](../../LICENSE-APACHE)
- [MIT License](../../LICENSE-MIT)

at your option.

## Links

- [GitHub](https://github.com/bug-ops/feedparser-rs)
- [PyPI](https://pypi.org/project/feedparser-rs/)
- [Rust API Documentation](https://docs.rs/feedparser-rs)
- [Changelog](../../CHANGELOG.md)
