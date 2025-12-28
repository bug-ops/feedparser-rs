use feedparser_rs::FeedMeta as CoreFeedMeta;
use pyo3::exceptions::{PyAttributeError, PyKeyError};
use pyo3::prelude::*;

use super::common::{PyGenerator, PyImage, PyLink, PyPerson, PyTag, PyTextConstruct};
use super::compat::FEED_FIELD_MAP;
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
            .as_deref()
            .map(|i| PyItunesFeedMeta::from_core(i.clone()))
    }

    #[getter]
    fn podcast(&self) -> Option<PyPodcastMeta> {
        self.inner
            .podcast
            .as_deref()
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
            .as_deref()
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
            .as_deref()
            .map(|g| PyGeoLocation::from_core(g.clone()))
    }

    fn __repr__(&self) -> String {
        format!(
            "FeedMeta(title='{}', link='{}')",
            self.inner.title.as_deref().unwrap_or("untitled"),
            self.inner.link.as_deref().unwrap_or("no-link")
        )
    }

    /// Provides backward compatibility for deprecated Python feedparser field names.
    ///
    /// Maps old field names to their modern equivalents:
    /// - `description` → `subtitle` (or `summary` as fallback)
    /// - `tagline` → `subtitle`
    /// - `modified` → `updated`
    /// - `copyright` → `rights`
    /// - `date` → `updated` (or `published` as fallback)
    /// - `url` → `link`
    ///
    /// This method is called by Python when normal attribute lookup fails.
    fn __getattr__(&self, py: Python<'_>, name: &str) -> PyResult<Py<PyAny>> {
        // Check if this is a deprecated field name
        if let Some(new_names) = FEED_FIELD_MAP.get(name) {
            // Try each new field name in order
            for new_name in new_names {
                let value: Option<Py<PyAny>> = match *new_name {
                    "subtitle" => self
                        .inner
                        .subtitle
                        .as_deref()
                        .and_then(|v| v.into_pyobject(py).map(|o| o.unbind().into()).ok()),
                    "subtitle_detail" => self.inner.subtitle_detail.as_ref().and_then(|tc| {
                        Py::new(py, PyTextConstruct::from_core(tc.clone()))
                            .ok()
                            .map(|p: Py<PyTextConstruct>| p.into_any())
                    }),
                    "summary" => self
                        .inner
                        .subtitle
                        .as_deref()
                        .and_then(|v| v.into_pyobject(py).map(|o| o.unbind().into()).ok()),
                    "summary_detail" => self.inner.subtitle_detail.as_ref().and_then(|tc| {
                        Py::new(py, PyTextConstruct::from_core(tc.clone()))
                            .ok()
                            .map(|p: Py<PyTextConstruct>| p.into_any())
                    }),
                    "rights" => self
                        .inner
                        .rights
                        .as_deref()
                        .and_then(|v| v.into_pyobject(py).map(|o| o.unbind().into()).ok()),
                    "rights_detail" => self.inner.rights_detail.as_ref().and_then(|tc| {
                        Py::new(py, PyTextConstruct::from_core(tc.clone()))
                            .ok()
                            .map(|p: Py<PyTextConstruct>| p.into_any())
                    }),
                    "updated" => self.inner.updated.and_then(|dt| {
                        dt.to_rfc3339()
                            .into_pyobject(py)
                            .map(|o| o.unbind().into())
                            .ok()
                    }),
                    "updated_parsed" => optional_datetime_to_struct_time(py, &self.inner.updated)
                        .ok()
                        .flatten(),
                    "published" => self.inner.published.and_then(|dt| {
                        dt.to_rfc3339()
                            .into_pyobject(py)
                            .map(|o| o.unbind().into())
                            .ok()
                    }),
                    "published_parsed" => {
                        optional_datetime_to_struct_time(py, &self.inner.published)
                            .ok()
                            .flatten()
                    }
                    "link" => self
                        .inner
                        .link
                        .as_deref()
                        .and_then(|v| v.into_pyobject(py).map(|o| o.unbind().into()).ok()),
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
            "'FeedMeta' object has no attribute '{}'",
            name
        )))
    }

    /// Provides dict-style access to fields for Python feedparser compatibility.
    ///
    /// Supports both modern field names and deprecated aliases.
    /// This method is called by Python when using dict-style access: `feed['title']`.
    ///
    /// Raises KeyError for unknown keys (unlike __getattr__ which raises AttributeError).
    fn __getitem__(&self, py: Python<'_>, key: &str) -> PyResult<Py<PyAny>> {
        // Check for known fields first
        match key {
            "title" => Ok(self
                .inner
                .title
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "title_detail" => {
                if let Some(ref tc) = self.inner.title_detail {
                    Ok(Py::new(py, PyTextConstruct::from_core(tc.clone()))?.into_any())
                } else {
                    Ok(py.None())
                }
            }
            "link" => Ok(self
                .inner
                .link
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "links" => {
                let links: Vec<_> = self
                    .inner
                    .links
                    .iter()
                    .map(|l| PyLink::from_core(l.clone()))
                    .collect();
                Ok(links.into_pyobject(py)?.into_any().unbind())
            }
            "subtitle" => Ok(self
                .inner
                .subtitle
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "subtitle_detail" => {
                if let Some(ref tc) = self.inner.subtitle_detail {
                    Ok(Py::new(py, PyTextConstruct::from_core(tc.clone()))?.into_any())
                } else {
                    Ok(py.None())
                }
            }
            "updated" => Ok(self
                .inner
                .updated
                .map(|dt| dt.to_rfc3339())
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "updated_parsed" => Ok(optional_datetime_to_struct_time(py, &self.inner.updated)?
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "published" => Ok(self
                .inner
                .published
                .map(|dt| dt.to_rfc3339())
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "published_parsed" => Ok(optional_datetime_to_struct_time(py, &self.inner.published)?
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "author" => Ok(self
                .inner
                .author
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "author_detail" => {
                if let Some(ref p) = self.inner.author_detail {
                    Ok(Py::new(py, PyPerson::from_core(p.clone()))?.into_any())
                } else {
                    Ok(py.None())
                }
            }
            "authors" => {
                let authors: Vec<_> = self
                    .inner
                    .authors
                    .iter()
                    .map(|p| PyPerson::from_core(p.clone()))
                    .collect();
                Ok(authors.into_pyobject(py)?.into_any().unbind())
            }
            "contributors" => {
                let contributors: Vec<_> = self
                    .inner
                    .contributors
                    .iter()
                    .map(|p| PyPerson::from_core(p.clone()))
                    .collect();
                Ok(contributors.into_pyobject(py)?.into_any().unbind())
            }
            "publisher" => Ok(self
                .inner
                .publisher
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "publisher_detail" => {
                if let Some(ref p) = self.inner.publisher_detail {
                    Ok(Py::new(py, PyPerson::from_core(p.clone()))?.into_any())
                } else {
                    Ok(py.None())
                }
            }
            "language" => Ok(self
                .inner
                .language
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "rights" => Ok(self
                .inner
                .rights
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "rights_detail" => {
                if let Some(ref tc) = self.inner.rights_detail {
                    Ok(Py::new(py, PyTextConstruct::from_core(tc.clone()))?.into_any())
                } else {
                    Ok(py.None())
                }
            }
            "generator" => Ok(self
                .inner
                .generator
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "generator_detail" => {
                if let Some(ref g) = self.inner.generator_detail {
                    Ok(Py::new(py, PyGenerator::from_core(g.clone()))?.into_any())
                } else {
                    Ok(py.None())
                }
            }
            "image" => {
                if let Some(ref i) = self.inner.image {
                    Ok(Py::new(py, PyImage::from_core(i.clone()))?.into_any())
                } else {
                    Ok(py.None())
                }
            }
            "icon" => Ok(self
                .inner
                .icon
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "logo" => Ok(self
                .inner
                .logo
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "tags" => {
                let tags: Vec<_> = self
                    .inner
                    .tags
                    .iter()
                    .map(|t| PyTag::from_core(t.clone()))
                    .collect();
                Ok(tags.into_pyobject(py)?.into_any().unbind())
            }
            "id" => Ok(self
                .inner
                .id
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "ttl" => Ok(self.inner.ttl.into_pyobject(py)?.into_any().unbind()),
            "itunes" => {
                if let Some(ref i) = self.inner.itunes {
                    Ok(Py::new(py, PyItunesFeedMeta::from_core(i.as_ref().clone()))?.into_any())
                } else {
                    Ok(py.None())
                }
            }
            "podcast" => {
                if let Some(ref p) = self.inner.podcast {
                    Ok(Py::new(py, PyPodcastMeta::from_core(p.as_ref().clone()))?.into_any())
                } else {
                    Ok(py.None())
                }
            }
            "license" => Ok(self
                .inner
                .license
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "syndication" => {
                if let Some(ref s) = self.inner.syndication {
                    Ok(Py::new(py, PySyndicationMeta::from_core(s.as_ref().clone()))?.into_any())
                } else {
                    Ok(py.None())
                }
            }
            "dc_creator" => Ok(self
                .inner
                .dc_creator
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "dc_publisher" => Ok(self
                .inner
                .dc_publisher
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "dc_rights" => Ok(self
                .inner
                .dc_rights
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "geo" => {
                if let Some(ref g) = self.inner.geo {
                    Ok(Py::new(py, PyGeoLocation::from_core(g.as_ref().clone()))?.into_any())
                } else {
                    Ok(py.None())
                }
            }
            // Check for deprecated field name aliases
            _ => {
                if let Some(new_names) = FEED_FIELD_MAP.get(key) {
                    // Try each new field name in order
                    for new_name in new_names {
                        let value: Option<Py<PyAny>> =
                            match *new_name {
                                "subtitle" => self.inner.subtitle.as_deref().and_then(|v| {
                                    v.into_pyobject(py).map(|o| o.unbind().into()).ok()
                                }),
                                "subtitle_detail" => {
                                    self.inner.subtitle_detail.as_ref().and_then(|tc| {
                                        Py::new(py, PyTextConstruct::from_core(tc.clone()))
                                            .ok()
                                            .map(|p: Py<PyTextConstruct>| p.into_any())
                                    })
                                }
                                "summary" => self.inner.subtitle.as_deref().and_then(|v| {
                                    v.into_pyobject(py).map(|o| o.unbind().into()).ok()
                                }),
                                "summary_detail" => {
                                    self.inner.subtitle_detail.as_ref().and_then(|tc| {
                                        Py::new(py, PyTextConstruct::from_core(tc.clone()))
                                            .ok()
                                            .map(|p: Py<PyTextConstruct>| p.into_any())
                                    })
                                }
                                "rights" => self.inner.rights.as_deref().and_then(|v| {
                                    v.into_pyobject(py).map(|o| o.unbind().into()).ok()
                                }),
                                "rights_detail" => {
                                    self.inner.rights_detail.as_ref().and_then(|tc| {
                                        Py::new(py, PyTextConstruct::from_core(tc.clone()))
                                            .ok()
                                            .map(|p: Py<PyTextConstruct>| p.into_any())
                                    })
                                }
                                "updated" => self.inner.updated.and_then(|dt| {
                                    dt.to_rfc3339()
                                        .into_pyobject(py)
                                        .map(|o| o.unbind().into())
                                        .ok()
                                }),
                                "updated_parsed" => {
                                    optional_datetime_to_struct_time(py, &self.inner.updated)
                                        .ok()
                                        .flatten()
                                }
                                "published" => self.inner.published.and_then(|dt| {
                                    dt.to_rfc3339()
                                        .into_pyobject(py)
                                        .map(|o| o.unbind().into())
                                        .ok()
                                }),
                                "published_parsed" => {
                                    optional_datetime_to_struct_time(py, &self.inner.published)
                                        .ok()
                                        .flatten()
                                }
                                "link" => self.inner.link.as_deref().and_then(|v| {
                                    v.into_pyobject(py).map(|o| o.unbind().into()).ok()
                                }),
                                _ => None,
                            };

                        // If we found a value, return it
                        if let Some(v) = value {
                            return Ok(v);
                        }
                    }
                }
                // Field not found - raise KeyError
                Err(PyKeyError::new_err(format!("'{}'", key)))
            }
        }
    }
}
