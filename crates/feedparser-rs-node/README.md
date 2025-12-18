# feedparser-rs

[![npm](https://img.shields.io/npm/v/feedparser-rs)](https://www.npmjs.com/package/feedparser-rs)
[![Node](https://img.shields.io/node/v/feedparser-rs)](https://www.npmjs.com/package/feedparser-rs)
[![License](https://img.shields.io/npm/l/feedparser-rs)](LICENSE)

High-performance RSS/Atom/JSON Feed parser for Node.js, written in Rust.

Drop-in replacement for Python's `feedparser` library, offering 10-100x performance improvement.

## Features

- **Fast**: Written in Rust, 10-100x faster than Python feedparser
- **Tolerant**: Handles malformed feeds with bozo flag (like feedparser)
- **Multi-format**: RSS 0.9x/1.0/2.0, Atom 0.3/1.0, JSON Feed 1.0/1.1
- **HTTP fetching**: Built-in URL fetching with compression support
- **TypeScript**: Full TypeScript definitions included
- **Zero-copy**: Efficient parsing with minimal allocations

## Installation

```bash
npm install feedparser-rs
# or
yarn add feedparser-rs
# or
pnpm add feedparser-rs
```

> [!IMPORTANT]
> Requires Node.js 18 or later.

## Quick Start

```javascript
import { parse } from 'feedparser-rs';

const feed = parse(`
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
`);

console.log(feed.feed.title);  // "My Blog"
console.log(feed.entries[0].title);  // "Hello World"
console.log(feed.version);  // "rss20"
```

## HTTP Fetching

Fetch and parse feeds directly from URLs:

```javascript
import { fetchAndParse } from 'feedparser-rs';

const feed = await fetchAndParse('https://example.com/feed.xml');
console.log(feed.feed.title);
console.log(`Fetched ${feed.entries.length} entries`);
```

> [!TIP]
> `fetchAndParse` automatically handles compression (gzip, deflate, brotli) and follows redirects.

### Parsing from Buffer

```javascript
import { parse } from 'feedparser-rs';

const response = await fetch('https://example.com/feed.xml');
const buffer = Buffer.from(await response.arrayBuffer());
const feed = parse(buffer);
```

## API

### `parse(source: Buffer | string | Uint8Array): ParsedFeed`

Parse a feed from bytes or string.

**Parameters:**
- `source` - Feed content as Buffer, string, or Uint8Array

**Returns:**
- `ParsedFeed` object with feed metadata and entries

**Throws:**
- `Error` if parsing fails catastrophically

### `fetchAndParse(url: string): Promise<ParsedFeed>`

Fetch and parse a feed from URL.

**Parameters:**
- `url` - Feed URL to fetch

**Returns:**
- Promise resolving to `ParsedFeed` object

### `detectFormat(source: Buffer | string | Uint8Array): string`

Detect feed format without full parsing.

**Returns:**
- Format string: `"rss20"`, `"atom10"`, `"json11"`, etc.

```javascript
const format = detectFormat('<feed xmlns="http://www.w3.org/2005/Atom">...</feed>');
console.log(format);  // "atom10"
```

## Types

### ParsedFeed

```typescript
interface ParsedFeed {
  feed: FeedMeta;
  entries: Entry[];
  bozo: boolean;
  bozo_exception?: string;
  encoding: string;
  version: string;
  namespaces: Record<string, string>;
}
```

### FeedMeta

```typescript
interface FeedMeta {
  title?: string;
  title_detail?: TextConstruct;
  link?: string;
  links: Link[];
  subtitle?: string;
  updated?: number;  // Milliseconds since epoch
  author?: string;
  authors: Person[];
  language?: string;
  image?: Image;
  tags: Tag[];
  id?: string;
  ttl?: number;
}
```

### Entry

```typescript
interface Entry {
  id?: string;
  title?: string;
  link?: string;
  links: Link[];
  summary?: string;
  content: Content[];
  published?: number;  // Milliseconds since epoch
  updated?: number;
  author?: string;
  authors: Person[];
  tags: Tag[];
  enclosures: Enclosure[];
}
```

> [!NOTE]
> See `index.d.ts` for complete type definitions including `Link`, `Person`, `Tag`, `Image`, `Enclosure`, and more.

## Error Handling

The library uses a "bozo" flag (like feedparser) to indicate parsing errors while still returning partial results:

```javascript
const feed = parse('<rss><channel><title>Broken</title></rss>');

if (feed.bozo) {
  console.warn('Feed has errors:', feed.bozo_exception);
}

// Still can access parsed data
console.log(feed.feed.title);  // "Broken"
```

## Dates

All date fields are returned as milliseconds since Unix epoch. Convert to JavaScript Date:

```javascript
const entry = feed.entries[0];
if (entry.published) {
  const date = new Date(entry.published);
  console.log(date.toISOString());
}
```

## Performance

Benchmarks on Apple M1 Pro:

| Feed Size | Time | Throughput |
|-----------|------|------------|
| Small (2 KB) | 0.01 ms | 187 MB/s |
| Medium (20 KB) | 0.09 ms | 214 MB/s |
| Large (200 KB) | 0.94 ms | 213 MB/s |

### vs Python feedparser

| Operation | feedparser-rs | Python feedparser | Speedup |
|-----------|---------------|-------------------|---------|
| Parse 20 KB RSS | 0.09 ms | 8.5 ms | **94x** |
| Parse 200 KB RSS | 0.94 ms | 85 ms | **90x** |

> [!TIP]
> For best performance, pass `Buffer` instead of `string` to avoid UTF-8 conversion overhead.

## Platform Support

Pre-built binaries available for:

| Platform | Architecture |
|----------|--------------|
| macOS | Intel (x64), Apple Silicon (arm64) |
| Linux | x64, arm64 |
| Windows | x64 |

Supported Node.js versions: 18, 20, 22+

## Development

```bash
# Install dependencies
npm install

# Build native module
npm run build

# Run tests
npm test

# Run tests with coverage
npm run test:coverage
```

## License

Licensed under either of:

- [Apache License, Version 2.0](../../LICENSE-APACHE)
- [MIT License](../../LICENSE-MIT)

at your option.

## Links

- [GitHub](https://github.com/bug-ops/feedparser-rs)
- [npm](https://www.npmjs.com/package/feedparser-rs)
- [Rust API Documentation](https://docs.rs/feedparser-rs)
- [Changelog](../../CHANGELOG.md)
