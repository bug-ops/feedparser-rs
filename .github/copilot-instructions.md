# feedparser-rs GitHub Copilot Instructions

## Project Mission

High-performance RSS/Atom/JSON Feed parser in Rust with Python (PyO3) and Node.js (napi-rs) bindings. This is a drop-in replacement for Python's `feedparser` library with 10-100x performance improvement.

**CRITICAL**: API compatibility with Python feedparser is the #1 priority. Field names, types, and behavior must match exactly.

**MSRV:** Rust 1.91.0 | **Edition:** 2024 | **License:** MIT/Apache-2.0

## Architecture Overview

### Workspace Structure
- **`crates/feedparser-rs-core`** — Pure Rust parser. All parsing logic lives here. NO dependencies on other workspace crates.
- **`crates/feedparser-rs-py`** — Python bindings via PyO3/maturin. Depends on core.
- **`crates/feedparser-rs-node`** — Node.js bindings via napi-rs. Depends on core.

### Parser Pipeline
1. **Format Detection** (`parser/detect.rs`) — Identifies RSS 0.9x/1.0/2.0, Atom 0.3/1.0, or JSON Feed 1.0/1.1
2. **Parsing** — Routes to `parser/rss.rs`, `parser/atom.rs`, or `parser/json.rs`
3. **Namespace Extraction** — Handlers in `namespace/` process iTunes, Dublin Core, Media RSS, Podcast 2.0
4. **Tolerant Error Handling** — Returns `ParsedFeed` with `bozo` flag set on errors, continues parsing

## Idiomatic Rust & Performance

### Type Safety First
- Prefer strong types over primitives: `FeedVersion` enum, not `&str`
- Use `Option<T>` and `Result<T, E>` — never sentinel values
- Leverage generics and trait bounds for reusable code:
```rust
fn collect_limited<T, I: Iterator<Item = T>>(iter: I, limit: usize) -> Vec<T> {
    iter.take(limit).collect()
}
```

### Zero-Cost Abstractions
- Use `&str` over `String` in function parameters
- Prefer iterators over index-based loops: `.iter().filter().map()`
- Use `Cow<'_, str>` when ownership is conditionally needed
- Avoid allocations in hot paths — reuse buffers where possible

### Edition 2024 Features
- Use `gen` blocks for custom iterators where applicable
- Leverage improved async patterns for HTTP module
- Apply new lifetime elision rules for cleaner signatures

### Safety Guidelines
- `#![warn(unsafe_code)]` is enabled — avoid `unsafe` unless absolutely necessary
- All public APIs must have doc comments (`#![warn(missing_docs)]`)
- Use `thiserror` for error types with proper `#[error]` attributes

## Critical Conventions

### Error Handling: Bozo Pattern (MANDATORY)
**Never panic or return errors for malformed input.** Set `bozo = true` and continue:
```rust
match parse_date(&text) {
    Some(dt) => entry.published = Some(dt),
    None => {
        feed.bozo = true;
        feed.bozo_exception = Some(format!("Invalid date: {text}"));
        // Continue parsing!
    }
}
```

### API Compatibility with Python feedparser
Field names must match `feedparser` exactly: `feed.title`, `entries[0].summary`, `version` returns `"rss20"`, `"atom10"`

### XML Parsing with quick-xml
Use tolerant mode — no strict validation:
```rust
let mut reader = Reader::from_reader(data);
reader.config_mut().trim_text(true);
// Do NOT enable check_end_names — tolerance over strictness
```

## Development Commands

All automation via `cargo-make`:

| Command | Purpose |
|---------|---------|
| `cargo make fmt` | Format with nightly rustfmt |
| `cargo make clippy` | Lint (excludes py bindings) |
| `cargo make test-rust` | Rust tests (nextest) |
| `cargo make pre-commit` | fmt + clippy + test-rust |
| `cargo make bench` | Criterion benchmarks |
| `cargo make msrv-check` | Verify MSRV 1.91.0 compatibility |

### Bindings
```bash
# Python
cd crates/feedparser-rs-py && maturin develop && pytest tests/ -v

# Node.js
cd crates/feedparser-rs-node && pnpm install && pnpm build && pnpm test
```

## Testing Patterns

Use `include_str!()` for fixtures in `tests/fixtures/`:
```rust
#[test]
fn test_rss20_basic() {
    let xml = include_str!("../../tests/fixtures/rss/example.xml");
    let feed = parse(xml.as_bytes()).unwrap();
    assert!(!feed.bozo);
}
```

Always verify malformed feeds set bozo but still parse:
```rust
#[test]
fn test_malformed_sets_bozo() {
    let xml = b"<rss><channel><title>Broken</title></rss>";
    let feed = parse(xml).unwrap();
    assert!(feed.bozo);
    assert_eq!(feed.feed.title.as_deref(), Some("Broken")); // Still parsed!
}
```

## Security Requirements

### SSRF Protection (CRITICAL for HTTP Module)
Block these URL patterns before fetching:
- Localhost/loopback: `127.0.0.1`, `[::1]`, `localhost`
- Private networks: `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`
- Link-local: `169.254.0.0/16` (AWS/GCP metadata endpoints), `fe80::/10`
- Special addresses: `0.0.0.0/8`, `255.255.255.255`, `::/128`

Always validate URLs through `is_safe_url()` before HTTP requests.

### XSS Protection (HTML Sanitization)
Use `ammonia` for HTML content from feeds:
- Allowed tags: `a, abbr, b, blockquote, br, code, div, em, h1-h6, hr, i, img, li, ol, p, pre, span, strong, ul`
- Enforce `rel="nofollow noopener"` on links
- Allow only `http`, `https`, `mailto` URL schemes
- Never pass raw HTML to Python/Node.js bindings without sanitization

### DoS Protection
Apply limits via `ParserLimits`:
- `max_feed_size`: Default 50MB
- `max_nesting_depth`: Default 100 levels
- `max_entries`: Default 10,000 items
- `max_text_length`: Default 1MB per text field
- `max_attribute_length`: Default 10KB per attribute

## Code Quality Standards

### Function Length Guidelines
- **Target**: Functions should be <50 lines
- **Maximum**: NEVER exceed 100 lines
- **If >50 lines**: Extract inline logic to helper functions

Example refactoring pattern:
```rust
// Before: 200+ line function
fn parse_channel(...) {
    match tag {
        b"itunes:category" => { /* 80 lines inline */ }
        // ...
    }
}

// After: Delegate to helpers
fn parse_channel(...) {
    match tag {
        tag if is_itunes_tag_any(tag) => parse_channel_itunes(tag, ...)?,
        // ...
    }
}
```

### Documentation Requirements
All public APIs must have doc comments:
```rust
/// Parses an RSS/Atom feed from bytes.
///
/// # Arguments
/// * `data` - Raw feed content as bytes
///
/// # Returns
/// Returns `ParsedFeed` with extracted metadata. If parsing encounters errors,
/// `bozo` flag is set to `true` and `bozo_exception` contains the error description.
///
/// # Examples
/// ```
/// let xml = b"<rss version=\"2.0\">...</rss>";
/// let feed = parse(xml)?;
/// assert_eq!(feed.version, FeedVersion::Rss20);
/// ```
pub fn parse(data: &[u8]) -> Result<ParsedFeed> { ... }
```

### Inline Comments
Minimize inline comments. Use comments ONLY for:
1. **Why** decisions (not **what** the code does)
2. Non-obvious constraints or workarounds
3. References to specifications (RFC 4287 section 4.1.2, etc.)

## Commit & Branch Conventions
- Branch: `feat/`, `fix/`, `docs/`, `refactor/`, `test/`
- Commits: [Conventional Commits](https://conventionalcommits.org/)
- Never mention "Claude" or "co-authored" in commit messages

## What NOT to Do
- ❌ Don't use `.unwrap()` or `.expect()` in parser code — use bozo pattern
- ❌ Don't add dependencies without workspace-level declaration in root `Cargo.toml`
- ❌ Don't skip `--exclude feedparser-rs-py` in workspace-wide Rust commands (PyO3 needs special handling)
- ❌ Don't break API compatibility with Python feedparser field names
- ❌ Don't panic on malformed feeds — set `bozo = true` and continue parsing
- ❌ Don't fetch URLs without SSRF validation (`is_safe_url()`)
- ❌ Don't pass raw HTML to bindings without sanitization (`sanitize_html()`)
- ❌ Don't create functions >100 lines — extract helpers
- ❌ Don't use generic names like `utils`, `helpers`, `common` for modules
- ❌ Don't add emojis to code or comments
