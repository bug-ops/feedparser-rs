# feedparser-rs

[![npm](https://img.shields.io/npm/v/feedparser-rs)](https://www.npmjs.com/package/feedparser-rs)
[![Node](https://img.shields.io/node/v/feedparser-rs)](https://www.npmjs.com/package/feedparser-rs)
[![License](https://img.shields.io/npm/l/feedparser-rs)](LICENSE)

High-performance RSS/Atom/JSON Feed parser for Node.js, written in Rust.

Drop-in replacement for Python's `feedparser` library, offering 10-100x performance improvement.

## Features

- **Fast**: Written in Rust, 10-100x faster than Python feedparser
- **Tolerant**: Handles malformed feeds with bozo flag (like feedparser)
- **Sanitized by default**: HTML-bearing fields are sanitized against XSS unless explicitly disabled via `parseWithOptions`
- **Multi-format**: RSS 0.9x/1.0/2.0, Atom 0.3/1.0, JSON Feed 1.0/1.1
- **JSON Feed extensions**: `_`-prefixed custom objects (e.g. `_cast`) captured as `feed.jsonExtensions`/`entry.jsonExtensions`
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

> **Important:** Requires Node.js 18 or later.

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
import { parseUrl } from 'feedparser-rs';

const feed = parseUrl('https://example.com/feed.xml');
console.log(feed.feed.title);
console.log(`Fetched ${feed.entries.length} entries`);

// Subsequent fetch with conditional GET (uses ETag/Last-Modified)
const feed2 = parseUrl('https://example.com/feed.xml', feed.etag, feed.modified);
if (feed2.status === 304) {
  console.log('Not modified, use cached version');
}
```

> **Important:** `parseUrl` is synchronous — it blocks the event loop for the duration of the HTTP request. Run it inside a worker thread if you need to avoid blocking on slow feeds.

> **Tip:** `parseUrl` automatically handles compression (gzip, deflate, brotli) and follows redirects.

### Parsing from Buffer

```javascript
import { parse } from 'feedparser-rs';

const response = await fetch('https://example.com/feed.xml');
const buffer = Buffer.from(await response.arrayBuffer());
const feed = parse(buffer);
```

## API

### `parse(source: Buffer | string): ParsedFeed`

Parse a feed from bytes or string.

**Parameters:**
- `source` - Feed content as Buffer or string

**Returns:**
- `ParsedFeed` object with feed metadata and entries

**Throws:**
- `Error` if the input exceeds the size limit or parsing fails catastrophically

### `parseWithOptions(source: Buffer | string, options?: ParseOptions): ParsedFeed`

Like `parse`, with full control over parsing behavior via a `ParseOptions` object:

```typescript
interface ParseOptions {
  maxSize?: number;             // Maximum feed size in bytes (default: 100 MB)
  sanitizeHtml?: boolean;       // Sanitize HTML-bearing fields (default: true)
  resolveRelativeUris?: boolean; // Resolve relative URLs against the feed's base URL (default: true)
}
```

```javascript
const feed = parseWithOptions(xml, { maxSize: 10_485_760, sanitizeHtml: false });
```

> **Important:** All fields are optional and independently overridable; omitted fields keep
> their default. `sanitizeHtml` defaults to `true` — disable it only for feed sources you fully
> trust.

### `parseUrl(url: string, etag?: string, modified?: string, userAgent?: string): ParsedFeed`

Fetch and parse a feed from an HTTP/HTTPS URL, with conditional GET support. **Synchronous**
— blocks the event loop for the duration of the request.

**Parameters:**
- `url` - Feed URL to fetch
- `etag` - ETag from a previous fetch, for conditional GET
- `modified` - Last-Modified value from a previous fetch, for conditional GET
- `userAgent` - Custom `User-Agent` header

**Returns:**
- `ParsedFeed` object with HTTP metadata (`status`, `href`, `etag`, `modified`, `headers`) populated. On a `304 Not Modified` response, `entries` is empty and `status` is `304`.

> **Note:** Only available when the `http` Cargo feature is enabled (the default for the published npm package).

### `parseUrlWithOptions(url: string, etag?: string, modified?: string, userAgent?: string, options?: ParseOptions): ParsedFeed`

Like `parseUrl`, with full control over parsing behavior via the same `ParseOptions` object as
`parseWithOptions` above (`maxSize`, `sanitizeHtml`, `resolveRelativeUris`).

### `detectFormat(source: Buffer | string): string`

Detect feed format without full parsing.

**Returns:**
- Format string: `"rss20"`, `"atom10"`, `"json11"`, etc.

```javascript
const format = detectFormat('<feed xmlns="http://www.w3.org/2005/Atom">...</feed>');
console.log(format);  // "atom10"
```

## Types

> **Note:** Field names follow napi-rs's automatic snake_case → camelCase conversion. Fields below are a representative subset — see `index.d.ts` for the complete definitions, including `Link`, `Person`, `Tag`, `Image`, `Enclosure`, `Itunes*`, `Podcast*`, `Media*`, and Dublin Core/GeoRSS fields.

### ParsedFeed

```typescript
interface ParsedFeed {
  feed: FeedMeta;
  entries: Entry[];
  bozo: boolean;
  bozoException?: string;
  encoding: string;
  version: string;
  namespaces: Record<string, string>;
  status?: number;                   // HTTP status code (parseUrl only)
  href?: string;                      // Final URL after redirects (parseUrl only)
  etag?: string;                      // For conditional GET (parseUrl only)
  modified?: string;                  // Last-Modified header (parseUrl only)
  headers?: Record<string, string>;   // Full response headers (parseUrl only)
}
```

### FeedMeta

Dates are exposed as a raw string (original value, timezone preserved) plus a `*Parsed`
variant with milliseconds since epoch.

```typescript
interface FeedMeta {
  title?: string;
  titleDetail?: TextConstruct;
  link?: string;
  links: Link[];
  subtitle?: string;
  updated?: string;          // Raw date string
  updatedParsed?: number;    // Milliseconds since epoch
  published?: string;
  publishedParsed?: number;
  author?: string;
  authors: Person[];
  language?: string;
  image?: Image;
  tags: Tag[];
  id?: string;
  ttl?: string;               // Kept as string for feedparser compatibility
  itunes?: ItunesFeedMeta;
  podcast?: PodcastMeta;
  jsonExtensions: Record<string, unknown>;  // JSON Feed `_`-prefixed custom objects; empty for RSS/Atom
  // ...and more: subtitleDetail, summary, contributors, publisher, rights,
  // generator, icon, logo, syndication, media*, cloud, textinput, etc.
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
  published?: string;         // Raw date string
  publishedParsed?: number;   // Milliseconds since epoch
  updated?: string;
  updatedParsed?: number;
  author?: string;
  authors: Person[];
  tags: Tag[];
  enclosures: Enclosure[];
  mediaTitle?: string;
  itunes?: ItunesEntryMeta;   // Episode duration, explicit, image, episode/season number
  podcast?: PodcastEntryMeta; // Podcast 2.0: transcripts, chapters, soundbites, persons
  jsonExtensions: Record<string, unknown>;  // JSON Feed `_`-prefixed custom objects; empty for RSS/Atom
  // ...and more: titleDetail, subtitleDetail, rights, created*, expired*,
  // publisher, contributors, comments, source, media*, thr*, dc*, etc.
}
```

> **Note:** See `index.d.ts` for complete type definitions.

## Error Handling

The library uses a "bozo" flag (like feedparser) to indicate parsing errors while still returning partial results:

```javascript
const feed = parse('<rss><channel><title>Broken</title></rss>');

if (feed.bozo) {
  console.warn('Feed has errors:', feed.bozoException);
}

// Still can access parsed data
console.log(feed.feed.title);  // "Broken"
```

### Thrown Errors

Some operations throw errors for catastrophic failures (e.g., input exceeding size limits, network errors):

```javascript
try {
  const feed = parseWithOptions(largeXml, { maxSize: 10 });
} catch (err) {
  // err.code indicates the error category:
  // 'InvalidArg' = bad input (XML/JSON parse error, invalid URL, encoding error)
  // 'GenericFailure' = I/O or network error (file/socket error, HTTP failure)
  if (err.code === 'InvalidArg') {
    console.error('Invalid input:', err.message);
  } else if (err.code === 'GenericFailure') {
    console.error('I/O or network error:', err.message);
  }
}
```

**Note:** The thrown `Error` object's `.code` property is a string (`'InvalidArg'` | `'GenericFailure'`). Use it to distinguish input validation errors from transient I/O failures when deciding whether to retry.

## Dates

Each date field is exposed twice: the raw string as it appeared in the feed (timezone
preserved) and a `*Parsed` variant with milliseconds since Unix epoch, ready for `Date`:

```javascript
const entry = feed.entries[0];
console.log(entry.published);        // e.g. "Mon, 06 Jul 2026 10:00:00 -0800" (raw string)
if (entry.publishedParsed) {
  const date = new Date(entry.publishedParsed);
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

> **Tip:** For best performance, pass `Buffer` instead of `string` to avoid UTF-8 conversion overhead.

## Platform Support

Pre-built binaries available for:

| Platform | Architecture |
|----------|--------------|
| macOS | Intel (x64), Apple Silicon (arm64) |
| Linux | x64, arm64 |
| Windows | x64 |

Minimum Node.js version: 18 (per `package.json` `engines`). CI tests run on Node.js 22 and 24.

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
