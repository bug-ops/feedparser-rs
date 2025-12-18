use feedparser_rs::Entry as CoreEntry;
use pyo3::prelude::*;

use super::common::{PyContent, PyEnclosure, PyLink, PyPerson, PySource, PyTag, PyTextConstruct};
use super::datetime::optional_datetime_to_struct_time;
use super::podcast::{PyItunesEntryMeta, PyPodcastPerson, PyPodcastTranscript};

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
            .as_ref()
            .map(|i| PyItunesEntryMeta::from_core(i.clone()))
    }

    #[getter]
    fn podcast_transcripts(&self) -> Vec<PyPodcastTranscript> {
        self.inner
            .podcast_transcripts
            .iter()
            .map(|t| PyPodcastTranscript::from_core(t.clone()))
            .collect()
    }

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

    fn __repr__(&self) -> String {
        format!(
            "Entry(title='{}', id='{}')",
            self.inner.title.as_deref().unwrap_or("untitled"),
            self.inner.id.as_deref().unwrap_or("no-id")
        )
    }
}
