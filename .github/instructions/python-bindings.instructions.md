---
applyTo: "crates/feedparser-rs-py/**"
---

# Python Bindings Instructions

## Mission-Critical: API Compatibility

These bindings MUST be a drop-in replacement for Python's `feedparser` library. Every function, class, attribute, and return type must match exactly.

**Target API:**
```python
import feedparser

# Parse from various sources
d = feedparser.parse('https://example.com/feed.xml')
d = feedparser.parse(open('feed.xml').read())
d = feedparser.parse(b'<rss>...</rss>')

# Access fields (these names are MANDATORY)
d.version           # 'rss20', 'atom10', etc.
d.bozo              # True/False
d.bozo_exception    # String or None
d.encoding          # 'utf-8', etc.
d.feed.title        # Feed title
d.feed.link         # Feed link
d.entries[0].title  # Entry title
d.entries[0].published_parsed  # time.struct_time (NOT DateTime!)
```

## PyO3 Fundamentals

### Module Setup

**Located in:** `src/lib.rs`

```rust
use pyo3::prelude::*;

#[pymodule]
fn feedparser_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(parse_url, m)?)?;
    m.add_class::<PyParsedFeed>()?;
    m.add_class::<PyFeedMeta>()?;
    m.add_class::<PyEntry>()?;
    // ... other classes
    Ok(())
}
```

**Module name MUST be `feedparser_rs`** (matches PyPI package name)

### Main Parse Function

```rust
/// Parse an RSS/Atom feed from bytes, string, or URL
#[pyfunction]
#[pyo3(signature = (source, /))]
pub fn parse(py: Python<'_>, source: &Bound<'_, PyAny>) -> PyResult<PyParsedFeed> {
    let data: Vec<u8> = if let Ok(s) = source.extract::<String>() {
        // String - could be URL or XML content
        if s.starts_with("http://") || s.starts_with("https://") {
            // HTTP fetching (if feature enabled)
            #[cfg(feature = "http")]
            {
                return parse_url_impl(py, &s);
            }
            #[cfg(not(feature = "http"))]
            {
                return Err(PyNotImplementedError::new_err(
                    "URL fetching not enabled. Install with 'pip install feedparser-rs[http]'"
                ));
            }
        }
        s.into_bytes()
    } else if let Ok(b) = source.extract::<Vec<u8>>() {
        b
    } else {
        return Err(PyTypeError::new_err("source must be str or bytes"));
    };

    let result = feedparser_rs_core::parse(&data)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    Ok(PyParsedFeed::from(result))
}
```

**Rules:**
1. Accept `str` (URL or XML) and `bytes`
2. Return `PyResult<PyParsedFeed>` (never panic)
3. Use `PyValueError` for parsing errors (not `RuntimeError`)

## Python Class Mapping

### ParsedFeed (FeedParserDict)

**Located in:** `src/types/parsed_feed.rs`

```rust
/// Main parsing result (equivalent to feedparser.FeedParserDict)
#[pyclass(name = "FeedParserDict")]
#[derive(Clone)]
pub struct PyParsedFeed {
    inner: Arc<ParsedFeed>,  // Use Arc for cheap clones
}

#[pymethods]
impl PyParsedFeed {
    #[getter]
    fn feed(&self) -> PyFeedMeta {
        PyFeedMeta {
            inner: Arc::clone(&self.inner.feed),
        }
    }

    #[getter]
    fn entries(&self) -> Vec<PyEntry> {
        self.inner
            .entries
            .iter()
            .map(|e| PyEntry {
                inner: Arc::new(e.clone()),
            })
            .collect()
    }

    #[getter]
    fn bozo(&self) -> bool {
        self.inner.bozo
    }

    #[getter]
    fn bozo_exception(&self) -> Option<String> {
        self.inner.bozo_exception.clone()
    }

    #[getter]
    fn encoding(&self) -> &str {
        &self.inner.encoding
    }

    #[getter]
    fn version(&self) -> &str {
        self.inner.version.as_str()  // Returns "rss20", "atom10", etc.
    }

    #[getter]
    fn namespaces(&self) -> HashMap<String, String> {
        self.inner.namespaces.clone()
    }

    // Python repr for debugging
    fn __repr__(&self) -> String {
        format!(
            "FeedParserDict(version={:?}, bozo={}, entries={})",
            self.version(),
            self.bozo(),
            self.entries().len()
        )
    }
}
```

**CRITICAL**: Class name MUST be `"FeedParserDict"` (matches Python feedparser)

### FeedMeta

**Located in:** `src/types/feed_meta.rs`

```rust
#[pyclass]
#[derive(Clone)]
pub struct PyFeedMeta {
    inner: Arc<FeedMeta>,
}

#[pymethods]
impl PyFeedMeta {
    #[getter]
    fn title(&self) -> Option<&str> {
        self.inner.title.as_deref()
    }

    #[getter]
    fn link(&self) -> Option<&str> {
        self.inner.link.as_deref()
    }

    #[getter]
    fn subtitle(&self) -> Option<&str> {
        self.inner.subtitle.as_deref()
    }

    #[getter]
    fn language(&self) -> Option<&str> {
        self.inner.language.as_deref()
    }

    // ... other getters
}
```

**Rules:**
1. All getters return Python-compatible types (`Option<&str>`, not `Option<String>`)
2. Use `as_deref()` for `Option<String>` → `Option<&str>` conversion
3. Clone only when necessary (prefer references)

## Date Conversion (CRITICAL)

### time.struct_time Requirement

Python feedparser returns `time.struct_time` for `*_parsed` fields. This is MANDATORY for compatibility.

```rust
use pyo3::types::PyTuple;

#[pymethods]
impl PyEntry {
    #[getter]
    fn published(&self) -> Option<String> {
        self.inner.published.map(|dt| dt.to_rfc3339())
    }

    #[getter]
    fn published_parsed(&self, py: Python<'_>) -> PyResult<Option<PyObject>> {
        match &self.inner.published {
            Some(dt) => {
                let time_module = py.import_bound("time")?;
                let struct_time = time_module.getattr("struct_time")?;

                let tuple = PyTuple::new_bound(
                    py,
                    &[
                        dt.year(),
                        dt.month() as i32,
                        dt.day() as i32,
                        dt.hour() as i32,
                        dt.minute() as i32,
                        dt.second() as i32,
                        dt.weekday().num_days_from_monday() as i32,
                        dt.ordinal() as i32,
                        0,  // tm_isdst (daylight savings time flag, -1 = unknown)
                    ],
                );

                Ok(Some(struct_time.call1((tuple,))?.into()))
            }
            None => Ok(None),
        }
    }
}
```

**CRITICAL**:
- `published` → RFC 3339 string
- `published_parsed` → `time.struct_time` tuple
- Apply same pattern to `updated`, `created`, `expired` fields

### time.struct_time Format

```python
# Python time.struct_time fields
(
    tm_year,    # e.g., 2024
    tm_mon,     # 1-12
    tm_mday,    # 1-31
    tm_hour,    # 0-23
    tm_min,     # 0-59
    tm_sec,     # 0-59
    tm_wday,    # 0-6 (Monday=0)
    tm_yday,    # 1-366 (day of year)
    tm_isdst,   # 0, 1, or -1
)
```

**Chrono mapping:**
- `dt.year()` → `tm_year`
- `dt.month()` → `tm_mon`
- `dt.day()` → `tm_mday`
- `dt.hour()` → `tm_hour`
- `dt.minute()` → `tm_min`
- `dt.second()` → `tm_sec`
- `dt.weekday().num_days_from_monday()` → `tm_wday` (Monday=0)
- `dt.ordinal()` → `tm_yday` (day of year)
- `0` or `-1` → `tm_isdst` (DST flag, use -1 for unknown)

## Error Handling

### Exception Mapping

```rust
use pyo3::exceptions::{PyTypeError, PyValueError, PyNotImplementedError};

// Type errors (wrong input type)
if source.is_none() {
    return Err(PyTypeError::new_err("source must be str or bytes"));
}

// Value errors (malformed content)
let result = feedparser_rs_core::parse(&data)
    .map_err(|e| PyValueError::new_err(e.to_string()))?;

// Not implemented (missing feature)
#[cfg(not(feature = "http"))]
{
    return Err(PyNotImplementedError::new_err("URL fetching not enabled"));
}
```

**Rules:**
1. Use `PyTypeError` for wrong Python types
2. Use `PyValueError` for parsing/content errors
3. Use `PyNotImplementedError` for unimplemented features
4. NEVER panic (always return `PyResult`)

## Memory Management

### Use Arc for Shared Ownership

```rust
use std::sync::Arc;

#[pyclass]
pub struct PyEntry {
    inner: Arc<Entry>,  // Shared ownership, cheap clones
}

// Cloning is O(1)
let cloned = PyEntry {
    inner: Arc::clone(&original.inner),
};
```

**Why?** Python objects can be shared/copied freely. Using `Arc` avoids expensive deep clones.

### Avoid Lifetime Parameters

```rust
// ❌ WRONG - Lifetimes don't work with PyO3
#[pyclass]
pub struct PyEntry<'a> {
    inner: &'a Entry,
}

// ✅ CORRECT - Owned data
#[pyclass]
pub struct PyEntry {
    inner: Arc<Entry>,
}
```

## Testing Python Bindings

### pytest Setup

**Located in:** `tests/test_basic.py`

```python
import pytest
import feedparser_rs

def test_parse_rss20():
    xml = b"""<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Test Feed</title>
            <link>http://example.com</link>
        </channel>
    </rss>"""

    feed = feedparser_rs.parse(xml)

    assert feed.version == "rss20"
    assert not feed.bozo
    assert feed.feed.title == "Test Feed"
    assert feed.feed.link == "http://example.com"

def test_bozo_flag_on_malformed():
    xml = b"<rss><channel><title>Broken</channel></rss>"  # Missing </title>

    feed = feedparser_rs.parse(xml)

    assert feed.bozo is True
    assert feed.bozo_exception is not None
    assert feed.feed.title == "Broken"  # Still parsed!

def test_published_parsed_returns_struct_time():
    xml = b"""<rss version="2.0"><channel><item>
        <pubDate>Mon, 01 Jan 2024 12:00:00 GMT</pubDate>
    </item></channel></rss>"""

    feed = feedparser_rs.parse(xml)
    entry = feed.entries[0]

    assert entry.published_parsed is not None
    assert entry.published_parsed.tm_year == 2024
    assert entry.published_parsed.tm_mon == 1
    assert entry.published_parsed.tm_mday == 1
```

### Compatibility Tests

Test against actual Python feedparser:

```python
import feedparser  # Original library
import feedparser_rs

def test_compatibility_with_feedparser():
    xml = open("tests/fixtures/rss/example.xml").read()

    # Parse with both libraries
    fp_result = feedparser.parse(xml)
    fprs_result = feedparser_rs.parse(xml)

    # Compare results
    assert fprs_result.version == fp_result.version
    assert fprs_result.feed.title == fp_result.feed.title
    assert len(fprs_result.entries) == len(fp_result.entries)
```

## Performance Optimization

### Release GIL for CPU-Intensive Work

```rust
use pyo3::Python;

#[pyfunction]
pub fn parse(py: Python<'_>, source: &Bound<'_, PyAny>) -> PyResult<PyParsedFeed> {
    let data = extract_bytes(source)?;

    // Release GIL during parsing (CPU-intensive)
    let result = py.allow_threads(|| feedparser_rs_core::parse(&data))
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    Ok(PyParsedFeed::from(result))
}
```

**Why?** Parsing is CPU-bound. Releasing the GIL allows other Python threads to run.

### Lazy Conversion

Don't convert Rust → Python types eagerly:

```rust
// ❌ BAD - Convert all entries immediately
#[getter]
fn entries(&self) -> Vec<PyEntry> {
    self.inner.entries.iter().map(|e| PyEntry::from(e)).collect()
}

// ✅ GOOD - Return iterator or convert lazily
#[getter]
fn entries(&self) -> Vec<PyEntry> {
    // Still eager, but with Arc (cheap clones)
    self.inner.entries.iter().map(|e| PyEntry {
        inner: Arc::new(e.clone())
    }).collect()
}
```

## maturin Configuration

**Located in:** `pyproject.toml`

```toml
[build-system]
requires = ["maturin>=1.7,<2.0"]
build-backend = "maturin"

[project]
name = "feedparser-rs"
version = "0.1.8"
description = "High-performance RSS/Atom feed parser (drop-in feedparser replacement)"
readme = "README.md"
requires-python = ">=3.10"
classifiers = [
    "Programming Language :: Rust",
    "Programming Language :: Python :: 3",
]

[tool.maturin]
features = ["pyo3/extension-module"]
module-name = "feedparser_rs"  # Must match #[pymodule] name
```

## Development Workflow

```bash
# Install development environment
cd crates/feedparser-rs-py
python -m venv .venv
source .venv/bin/activate  # or .venv\Scripts\activate on Windows
pip install maturin pytest

# Build and install locally
maturin develop --release

# Run tests
pytest tests/ -v

# Build wheel
maturin build --release
```

## Common Pitfalls

### Don't Return Rust Types Directly

```rust
// ❌ WRONG - Can't return Rust type to Python
#[pyfunction]
fn parse(data: &[u8]) -> ParsedFeed {
    feedparser_rs_core::parse(data).unwrap()
}

// ✅ CORRECT - Wrap in PyO3 class
#[pyfunction]
fn parse(data: &[u8]) -> PyResult<PyParsedFeed> {
    let result = feedparser_rs_core::parse(data)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(PyParsedFeed::from(result))
}
```

### Don't Use .unwrap() or .expect()

```rust
// ❌ WRONG - Panics crash Python
let dt = parse_date(&text).unwrap();

// ✅ CORRECT - Return PyResult
let dt = parse_date(&text)
    .ok_or_else(|| PyValueError::new_err("Invalid date"))?;
```

### Don't Forget #[getter] Attribute

```rust
// ❌ WRONG - Not accessible from Python
#[pymethods]
impl PyEntry {
    fn title(&self) -> Option<&str> {  // Missing #[getter]
        self.inner.title.as_deref()
    }
}

// ✅ CORRECT
#[pymethods]
impl PyEntry {
    #[getter]
    fn title(&self) -> Option<&str> {
        self.inner.title.as_deref()
    }
}
```

### Don't Break Python's __repr__

```rust
// ✅ GOOD - Provide helpful __repr__
#[pymethods]
impl PyEntry {
    fn __repr__(&self) -> String {
        format!(
            "Entry(title={:?}, link={:?})",
            self.inner.title,
            self.inner.link
        )
    }
}
```

## References

- PyO3 documentation: https://pyo3.rs/
- Python feedparser API: https://feedparser.readthedocs.io/
- maturin guide: https://www.maturin.rs/
- time.struct_time: https://docs.python.org/3/library/time.html#time.struct_time
