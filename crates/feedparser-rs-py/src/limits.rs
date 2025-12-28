use feedparser_rs::ParserLimits as CoreParserLimits;
use pyo3::prelude::*;

/// Resource limits for feed parsing (DoS protection)
#[pyclass(name = "ParserLimits", module = "feedparser_rs")]
#[derive(Clone)]
pub struct PyParserLimits {
    max_feed_size_bytes: usize,
    max_entries: usize,
    max_links_per_feed: usize,
    max_links_per_entry: usize,
    max_authors: usize,
    max_contributors: usize,
    max_tags: usize,
    max_content_blocks: usize,
    max_enclosures: usize,
}

#[pymethods]
impl PyParserLimits {
    #[new]
    #[pyo3(signature = (
        max_feed_size_bytes=100_000_000,
        max_entries=10_000,
        max_links_per_feed=100,
        max_links_per_entry=50,
        max_authors=20,
        max_contributors=20,
        max_tags=100,
        max_content_blocks=10,
        max_enclosures=20
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        max_feed_size_bytes: usize,
        max_entries: usize,
        max_links_per_feed: usize,
        max_links_per_entry: usize,
        max_authors: usize,
        max_contributors: usize,
        max_tags: usize,
        max_content_blocks: usize,
        max_enclosures: usize,
    ) -> Self {
        Self {
            max_feed_size_bytes,
            max_entries,
            max_links_per_feed,
            max_links_per_entry,
            max_authors,
            max_contributors,
            max_tags,
            max_content_blocks,
            max_enclosures,
        }
    }

    #[getter]
    fn max_feed_size_bytes(&self) -> usize {
        self.max_feed_size_bytes
    }

    #[getter]
    fn max_entries(&self) -> usize {
        self.max_entries
    }

    #[getter]
    fn max_links_per_feed(&self) -> usize {
        self.max_links_per_feed
    }

    #[getter]
    fn max_links_per_entry(&self) -> usize {
        self.max_links_per_entry
    }

    #[getter]
    fn max_authors(&self) -> usize {
        self.max_authors
    }

    #[getter]
    fn max_contributors(&self) -> usize {
        self.max_contributors
    }

    #[getter]
    fn max_tags(&self) -> usize {
        self.max_tags
    }

    #[getter]
    fn max_content_blocks(&self) -> usize {
        self.max_content_blocks
    }

    #[getter]
    fn max_enclosures(&self) -> usize {
        self.max_enclosures
    }

    fn __repr__(&self) -> String {
        format!(
            "ParserLimits(max_feed_size_bytes={}, max_entries={})",
            self.max_feed_size_bytes, self.max_entries
        )
    }
}

impl PyParserLimits {
    /// Convert to core ParserLimits
    pub(crate) fn to_core_limits(&self) -> CoreParserLimits {
        CoreParserLimits {
            max_feed_size_bytes: self.max_feed_size_bytes,
            max_entries: self.max_entries,
            max_links_per_feed: self.max_links_per_feed,
            max_links_per_entry: self.max_links_per_entry,
            max_authors: self.max_authors,
            max_contributors: self.max_contributors,
            max_tags: self.max_tags,
            max_content_blocks: self.max_content_blocks,
            max_enclosures: self.max_enclosures,
            max_namespaces: 100,               // Use default
            max_nesting_depth: 100,            // Use default
            max_text_length: 10 * 1024 * 1024, // 10 MB
            max_attribute_length: 64 * 1024,   // 64 KB
            max_podcast_soundbites: 10,        // Use default
            max_podcast_transcripts: 20,       // Use default
            max_podcast_funding: 20,           // Use default
            max_podcast_persons: 50,           // Use default
            max_value_recipients: 20,          // Use default
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_limits_defaults() {
        let limits = PyParserLimits::new(100_000_000, 10_000, 100, 50, 20, 20, 100, 10, 20);

        assert_eq!(limits.max_feed_size_bytes(), 100_000_000);
        assert_eq!(limits.max_entries(), 10_000);
        assert_eq!(limits.max_links_per_feed(), 100);
        assert_eq!(limits.max_links_per_entry(), 50);
        assert_eq!(limits.max_authors(), 20);
        assert_eq!(limits.max_contributors(), 20);
        assert_eq!(limits.max_tags(), 100);
        assert_eq!(limits.max_content_blocks(), 10);
        assert_eq!(limits.max_enclosures(), 20);
    }

    #[test]
    fn test_parser_limits_custom() {
        let limits = PyParserLimits::new(50_000_000, 5_000, 50, 25, 10, 10, 50, 5, 10);

        assert_eq!(limits.max_feed_size_bytes(), 50_000_000);
        assert_eq!(limits.max_entries(), 5_000);
        assert_eq!(limits.max_links_per_feed(), 50);
        assert_eq!(limits.max_links_per_entry(), 25);
        assert_eq!(limits.max_authors(), 10);
        assert_eq!(limits.max_contributors(), 10);
        assert_eq!(limits.max_tags(), 50);
        assert_eq!(limits.max_content_blocks(), 5);
        assert_eq!(limits.max_enclosures(), 10);
    }

    #[test]
    fn test_to_core_limits() {
        let py_limits = PyParserLimits::new(50_000_000, 5_000, 50, 25, 10, 10, 50, 5, 10);

        let core_limits = py_limits.to_core_limits();

        assert_eq!(core_limits.max_feed_size_bytes, 50_000_000);
        assert_eq!(core_limits.max_entries, 5_000);
        assert_eq!(core_limits.max_links_per_feed, 50);
        assert_eq!(core_limits.max_links_per_entry, 25);
        assert_eq!(core_limits.max_authors, 10);
        assert_eq!(core_limits.max_contributors, 10);
        assert_eq!(core_limits.max_tags, 50);
        assert_eq!(core_limits.max_content_blocks, 5);
        assert_eq!(core_limits.max_enclosures, 10);
        // Check default values
        assert_eq!(core_limits.max_namespaces, 100);
        assert_eq!(core_limits.max_nesting_depth, 100);
        assert_eq!(core_limits.max_text_length, 10 * 1024 * 1024);
        assert_eq!(core_limits.max_attribute_length, 64 * 1024);
    }

    #[test]
    fn test_repr() {
        let limits = PyParserLimits::new(100_000_000, 10_000, 100, 50, 20, 20, 100, 10, 20);

        let repr = limits.__repr__();
        assert!(repr.contains("ParserLimits"));
        assert!(repr.contains("100000000"));
        assert!(repr.contains("10000"));
    }
}
