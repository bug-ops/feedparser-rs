use feedparser_rs::FeedMeta as CoreFeedMeta;
use pyo3::prelude::*;

use super::common::{PyGenerator, PyImage, PyLink, PyPerson, PyTag, PyTextConstruct};
use super::datetime::optional_datetime_to_struct_time;
use super::geo::PyGeoLocation;
use super::podcast::{PyItunesFeedMeta, PyPodcastMeta};
use super::syndication::PySyndicationMeta;

#[pyclass(name = "FeedMeta", module = "feedparser_rs")]
#[derive(Clone)]
pub struct PyFeedMeta {
    inner: CoreFeedMeta,
}

impl PyFeedMeta {
    pub fn from_core(core: CoreFeedMeta) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyFeedMeta {
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
    fn subtitle(&self) -> Option<&str> {
        self.inner.subtitle.as_deref()
    }

    #[getter]
    fn subtitle_detail(&self) -> Option<PyTextConstruct> {
        self.inner
            .subtitle_detail
            .as_ref()
            .map(|tc| PyTextConstruct::from_core(tc.clone()))
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
    fn published(&self) -> Option<String> {
        self.inner.published.map(|dt| dt.to_rfc3339())
    }

    #[getter]
    fn published_parsed(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        optional_datetime_to_struct_time(py, &self.inner.published)
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
    fn language(&self) -> Option<&str> {
        self.inner.language.as_deref()
    }

    #[getter]
    fn rights(&self) -> Option<&str> {
        self.inner.rights.as_deref()
    }

    #[getter]
    fn rights_detail(&self) -> Option<PyTextConstruct> {
        self.inner
            .rights_detail
            .as_ref()
            .map(|tc| PyTextConstruct::from_core(tc.clone()))
    }

    #[getter]
    fn generator(&self) -> Option<&str> {
        self.inner.generator.as_deref()
    }

    #[getter]
    fn generator_detail(&self) -> Option<PyGenerator> {
        self.inner
            .generator_detail
            .as_ref()
            .map(|g| PyGenerator::from_core(g.clone()))
    }

    #[getter]
    fn image(&self) -> Option<PyImage> {
        self.inner
            .image
            .as_ref()
            .map(|i| PyImage::from_core(i.clone()))
    }

    #[getter]
    fn icon(&self) -> Option<&str> {
        self.inner.icon.as_deref()
    }

    #[getter]
    fn logo(&self) -> Option<&str> {
        self.inner.logo.as_deref()
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
    fn id(&self) -> Option<&str> {
        self.inner.id.as_deref()
    }

    #[getter]
    fn ttl(&self) -> Option<u32> {
        self.inner.ttl
    }

    #[getter]
    fn itunes(&self) -> Option<PyItunesFeedMeta> {
        self.inner
            .itunes
            .as_ref()
            .map(|i| PyItunesFeedMeta::from_core(i.clone()))
    }

    #[getter]
    fn podcast(&self) -> Option<PyPodcastMeta> {
        self.inner
            .podcast
            .as_ref()
            .map(|p| PyPodcastMeta::from_core(p.clone()))
    }

    #[getter]
    fn license(&self) -> Option<&str> {
        self.inner.license.as_deref()
    }

    #[getter]
    fn syndication(&self) -> Option<PySyndicationMeta> {
        self.inner
            .syndication
            .as_ref()
            .map(|s| PySyndicationMeta::from_core(s.clone()))
    }

    #[getter]
    fn dc_creator(&self) -> Option<&str> {
        self.inner.dc_creator.as_deref()
    }

    #[getter]
    fn dc_publisher(&self) -> Option<&str> {
        self.inner.dc_publisher.as_deref()
    }

    #[getter]
    fn dc_rights(&self) -> Option<&str> {
        self.inner.dc_rights.as_deref()
    }

    #[getter]
    fn geo(&self) -> Option<PyGeoLocation> {
        self.inner
            .geo
            .as_ref()
            .map(|g| PyGeoLocation::from_core(g.clone()))
    }

    fn __repr__(&self) -> String {
        format!(
            "FeedMeta(title='{}', link='{}')",
            self.inner.title.as_deref().unwrap_or("untitled"),
            self.inner.link.as_deref().unwrap_or("no-link")
        )
    }
}
