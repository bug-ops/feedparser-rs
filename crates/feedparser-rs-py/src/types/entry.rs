use feedparser_rs::Entry as CoreEntry;
use pyo3::prelude::*;
use pyo3::exceptions::PyAttributeError;

use super::compat::ENTRY_FIELD_MAP;
use super::common::{PyContent, PyEnclosure, PyLink, PyPerson, PySource, PyTag, PyTextConstruct};
use super::datetime::optional_datetime_to_struct_time;
use super::geo::PyGeoLocation;
use super::media::{PyMediaContent, PyMediaThumbnail};
use super::podcast::{PyItunesEntryMeta, PyPodcastEntryMeta, PyPodcastPerson, PyPodcastTranscript};

#[pyclass(name = "Entry", module = "feedparser_rs")]
#[derive(Clone)]
pub struct PyEntry {
    inner: CoreEntry,
}

impl PyEntry {
    pub fn from_core(core: CoreEntry) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyEntry {
    #[getter]
    fn id(&self) -> Option<&str> {
        self.inner.id.as_deref()
    }

    #[getter]
    fn title(&self) -> Option<&str> {
        self.inner.title.as_deref()
    }

    #[getter]
    fn title_detail(&self) -> Option<PyTextConstruct> {
        self.inner
            .title_detail
            .as_ref()
            .map(|tc| PyTextConstruct::from_core(tc.clone()))
    }

    #[getter]
    fn link(&self) -> Option<&str> {
        self.inner.link.as_deref()
    }

    #[getter]
    fn links(&self) -> Vec<PyLink> {
        self.inner
            .links
            .iter()
            .map(|l| PyLink::from_core(l.clone()))
            .collect()
    }

    #[getter]
    fn summary(&self) -> Option<&str> {
        self.inner.summary.as_deref()
    }

    #[getter]
    fn summary_detail(&self) -> Option<PyTextConstruct> {
        self.inner
            .summary_detail
            .as_ref()
            .map(|tc| PyTextConstruct::from_core(tc.clone()))
    }

    #[getter]
    fn content(&self) -> Vec<PyContent> {
        self.inner
            .content
            .iter()
            .map(|c| PyContent::from_core(c.clone()))
            .collect()
    }

    #[getter]
    fn published(&self) -> Option<String> {
        self.inner.published.map(|dt| dt.to_rfc3339())
    }

    #[getter]
    fn published_parsed(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        optional_datetime_to_struct_time(py, &self.inner.published)
    }

    #[getter]
    fn updated(&self) -> Option<String> {
        self.inner.updated.map(|dt| dt.to_rfc3339())
    }

    #[getter]
    fn updated_parsed(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        optional_datetime_to_struct_time(py, &self.inner.updated)
    }

    #[getter]
    fn created(&self) -> Option<String> {
        self.inner.created.map(|dt| dt.to_rfc3339())
    }

    #[getter]
    fn created_parsed(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        optional_datetime_to_struct_time(py, &self.inner.created)
    }

    #[getter]
    fn expired(&self) -> Option<String> {
        self.inner.expired.map(|dt| dt.to_rfc3339())
    }

    #[getter]
    fn expired_parsed(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        optional_datetime_to_struct_time(py, &self.inner.expired)
    }

    #[getter]
    fn author(&self) -> Option<&str> {
        self.inner.author.as_deref()
    }

    #[getter]
    fn author_detail(&self) -> Option<PyPerson> {
        self.inner
            .author_detail
            .as_ref()
            .map(|p| PyPerson::from_core(p.clone()))
    }

    #[getter]
    fn authors(&self) -> Vec<PyPerson> {
        self.inner
            .authors
            .iter()
            .map(|p| PyPerson::from_core(p.clone()))
            .collect()
    }

    #[getter]
    fn contributors(&self) -> Vec<PyPerson> {
        self.inner
            .contributors
            .iter()
            .map(|p| PyPerson::from_core(p.clone()))
            .collect()
    }

    #[getter]
    fn publisher(&self) -> Option<&str> {
        self.inner.publisher.as_deref()
    }

    #[getter]
    fn publisher_detail(&self) -> Option<PyPerson> {
        self.inner
            .publisher_detail
            .as_ref()
            .map(|p| PyPerson::from_core(p.clone()))
    }

    #[getter]
    fn tags(&self) -> Vec<PyTag> {
        self.inner
            .tags
            .iter()
            .map(|t| PyTag::from_core(t.clone()))
            .collect()
    }

    #[getter]
    fn enclosures(&self) -> Vec<PyEnclosure> {
        self.inner
            .enclosures
            .iter()
            .map(|e| PyEnclosure::from_core(e.clone()))
            .collect()
    }

    #[getter]
    fn comments(&self) -> Option<&str> {
        self.inner.comments.as_deref()
    }

    #[getter]
    fn source(&self) -> Option<PySource> {
        self.inner
            .source
            .as_ref()
            .map(|s| PySource::from_core(s.clone()))
    }

    #[getter]
    fn itunes(&self) -> Option<PyItunesEntryMeta> {
        self.inner
            .itunes
            .as_deref()
            .map(|i| PyItunesEntryMeta::from_core(i.clone()))
    }

    /// Returns podcast transcripts for this entry.
    ///
    /// Dual access pattern for feedparser compatibility:
    /// - `entry.podcast_transcripts` - Direct access (this method)
    /// - `entry.podcast.transcript` - Nested access via PodcastEntryMeta
    ///
    /// Both provide the same data. Use whichever pattern matches your code style.
    #[getter]
    fn podcast_transcripts(&self) -> Vec<PyPodcastTranscript> {
        self.inner
            .podcast_transcripts
            .iter()
            .map(|t| PyPodcastTranscript::from_core(t.clone()))
            .collect()
    }

    /// Returns podcast persons for this entry.
    ///
    /// Dual access pattern for feedparser compatibility:
    /// - `entry.podcast_persons` - Direct access (this method)
    /// - `entry.podcast.person` - Nested access via PodcastEntryMeta
    ///
    /// Both provide the same data. Use whichever pattern matches your code style.
    #[getter]
    fn podcast_persons(&self) -> Vec<PyPodcastPerson> {
        self.inner
            .podcast_persons
            .iter()
            .map(|p| PyPodcastPerson::from_core(p.clone()))
            .collect()
    }

    #[getter]
    fn license(&self) -> Option<&str> {
        self.inner.license.as_deref()
    }

    #[getter]
    fn geo(&self) -> Option<PyGeoLocation> {
        self.inner
            .geo
            .as_deref()
            .map(|g| PyGeoLocation::from_core(g.clone()))
    }

    #[getter]
    fn dc_creator(&self) -> Option<&str> {
        self.inner.dc_creator.as_deref()
    }

    #[getter]
    fn dc_date(&self) -> Option<String> {
        self.inner.dc_date.map(|dt| dt.to_rfc3339())
    }

    #[getter]
    fn dc_date_parsed(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        optional_datetime_to_struct_time(py, &self.inner.dc_date)
    }

    #[getter]
    fn dc_rights(&self) -> Option<&str> {
        self.inner.dc_rights.as_deref()
    }

    #[getter]
    fn dc_subject(&self) -> Vec<String> {
        self.inner.dc_subject.clone()
    }

    #[getter]
    fn media_thumbnails(&self) -> Vec<PyMediaThumbnail> {
        self.inner
            .media_thumbnails
            .iter()
            .map(|t| PyMediaThumbnail::from_core(t.clone()))
            .collect()
    }

    #[getter]
    fn media_content(&self) -> Vec<PyMediaContent> {
        self.inner
            .media_content
            .iter()
            .map(|c| PyMediaContent::from_core(c.clone()))
            .collect()
    }

    #[getter]
    fn podcast(&self) -> Option<PyPodcastEntryMeta> {
        self.inner
            .podcast
            .as_deref()
            .map(|p| PyPodcastEntryMeta::from_core(p.clone()))
    }

    fn __repr__(&self) -> String {
        format!(
            "Entry(title='{}', id='{}')",
            self.inner.title.as_deref().unwrap_or("untitled"),
            self.inner.id.as_deref().unwrap_or("no-id")
        )
    }

    /// Provides backward compatibility for deprecated Python feedparser field names.
    ///
    /// Maps old field names to their modern equivalents:
    /// - `guid` → `id`
    /// - `description` → `summary`
    /// - `issued` → `published`
    /// - `modified` → `updated`
    /// - `date` → `updated` (or `published` as fallback)
    ///
    /// This method is called by Python when normal attribute lookup fails.
    fn __getattr__(&self, py: Python<'_>, name: &str) -> PyResult<Py<PyAny>> {
        // Check if this is a deprecated field name
        if let Some(new_names) = ENTRY_FIELD_MAP.get(name) {
            // Try each new field name in order
            for new_name in new_names {
                let value: Option<Py<PyAny>> = match *new_name {
                    "id" => self.inner.id.as_deref().and_then(|v| {
                        v.into_pyobject(py).map(|o| o.unbind().into()).ok()
                    }),
                    "summary" => self.inner.summary.as_deref().and_then(|v| {
                        v.into_pyobject(py).map(|o| o.unbind().into()).ok()
                    }),
                    "summary_detail" => self.inner.summary_detail.as_ref().and_then(|tc| {
                        Py::new(py, PyTextConstruct::from_core(tc.clone())).ok().map(|p: Py<PyTextConstruct>| p.into_any())
                    }),
                    "published" => self.inner.published.and_then(|dt| {
                        dt.to_rfc3339().into_pyobject(py).map(|o| o.unbind().into()).ok()
                    }),
                    "published_parsed" => {
                        optional_datetime_to_struct_time(py, &self.inner.published).ok().flatten()
                    },
                    "updated" => self.inner.updated.and_then(|dt| {
                        dt.to_rfc3339().into_pyobject(py).map(|o| o.unbind().into()).ok()
                    }),
                    "updated_parsed" => {
                        optional_datetime_to_struct_time(py, &self.inner.updated).ok().flatten()
                    },
                    _ => None,
                };

                // If we found a value, return it
                if let Some(v) = value {
                    return Ok(v);
                }
            }
        }

        // Field not found - raise AttributeError
        Err(PyAttributeError::new_err(format!(
            "'Entry' object has no attribute '{}'",
            name
        )))
    }
}
