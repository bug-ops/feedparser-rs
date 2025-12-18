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
- **Familiar API**: Inspired by feedparser, easy migration path
- **DoS protection**: Built-in resource limits

## Installation

```bash
pip install feedparser-rs
```

> [!IMPORTANT]
> Requires Python 3.9 or later.

## Usage

### Basic Parsing

```python
import feedparser_rs

# Parse from string or bytes
d = feedparser_rs.parse('<rss>...</rss>')
d = feedparser_rs.parse(b'<rss>...</rss>')

# Access data
print(d.feed.title)
print(d.version)  # "rss20", "atom10", etc.
print(d.bozo)     # True if parsing errors occurred

for entry in d.entries:
    print(entry.title)
    print(entry.published_parsed)  # time.struct_time
```

> [!NOTE]
> Date fields like `published_parsed` return `time.struct_time` for feedparser compatibility.

### Fetching from URL

```python
import feedparser_rs

# Fetch and parse in one call
d = feedparser_rs.parse_url('https://example.com/feed.xml')

print(d.feed.title)
print(f"Fetched {len(d.entries)} entries")

# With custom limits
limits = feedparser_rs.ParserLimits(max_entries=100)
d = feedparser_rs.parse_url_with_limits('https://example.com/feed.xml', limits)
```

> [!TIP]
> `parse_url` supports automatic compression (gzip, deflate, brotli) and follows redirects.

## Migration from feedparser

```python
# Option 1: alias import
import feedparser_rs as feedparser
d = feedparser.parse(feed_content)

# Option 2: direct import
import feedparser_rs
d = feedparser_rs.parse(feed_content)

# Option 3: URL fetching (new!)
d = feedparser_rs.parse_url('https://example.com/feed.xml')
```

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

d = feedparser_rs.parse_with_limits(feed_data, limits)
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

- `parse(source)` — Parse feed from bytes or str
- `parse_url(url)` — Fetch and parse feed from URL
- `parse_with_limits(source, limits)` — Parse with custom resource limits
- `parse_url_with_limits(url, limits)` — Fetch and parse with custom limits
- `detect_format(source)` — Detect feed format without full parsing

### Classes

- `FeedParserDict` — Parsed feed result
  - `.feed` — Feed metadata
  - `.entries` — List of entries
  - `.bozo` — True if parsing errors occurred
  - `.version` — Feed version string
  - `.encoding` — Character encoding

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

Supported Python versions: 3.9, 3.10, 3.11, 3.12, 3.13

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
