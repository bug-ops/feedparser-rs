use feedparser_rs::ParsedFeed as CoreParsedFeed;
use pyo3::exceptions::{PyAttributeError, PyKeyError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use super::compat::CONTAINER_FIELD_MAP;
use super::entry::PyEntry;
use super::feed_meta::PyFeedMeta;

#[pyclass(name = "FeedParserDict", module = "feedparser_rs")]
pub struct PyParsedFeed {
    feed: Py<PyFeedMeta>,
    entries: Vec<Py<PyEntry>>,
    bozo: bool,
    bozo_exception: Option<String>,
    encoding: String,
    version: String,
    namespaces: Py<PyDict>,
    status: Option<u16>,
    href: Option<String>,
    etag: Option<String>,
    modified: Option<String>,
    #[cfg(feature = "http")]
    headers: Option<Py<PyDict>>,
}

impl PyParsedFeed {
    pub fn from_core(py: Python<'_>, core: CoreParsedFeed) -> PyResult<Self> {
        let feed = Py::new(py, PyFeedMeta::from_core(core.feed))?;

        let entries: PyResult<Vec<_>> = core
            .entries
            .into_iter()
            .map(|e| Py::new(py, PyEntry::from_core(e)))
            .collect();

        let namespaces = PyDict::new(py);
        for (prefix, uri) in core.namespaces {
            namespaces.set_item(prefix, uri)?;
        }

        #[cfg(feature = "http")]
        let headers = if let Some(headers_map) = core.headers {
            let headers_dict = PyDict::new(py);
            for (key, value) in headers_map {
                headers_dict.set_item(key, value)?;
            }
            Some(headers_dict.unbind())
        } else {
            None
        };

        Ok(Self {
            feed,
            entries: entries?,
            bozo: core.bozo,
            bozo_exception: core.bozo_exception,
            encoding: core.encoding,
            version: core.version.to_string(),
            namespaces: namespaces.unbind(),
            status: core.status,
            href: core.href,
            etag: core.etag,
            modified: core.modified,
            #[cfg(feature = "http")]
            headers,
        })
    }
}

#[pymethods]
impl PyParsedFeed {
    #[getter]
    fn feed(&self, py: Python<'_>) -> Py<PyFeedMeta> {
        self.feed.clone_ref(py)
    }

    #[getter]
    fn entries(&self, py: Python<'_>) -> Vec<Py<PyEntry>> {
        self.entries.iter().map(|e| e.clone_ref(py)).collect()
    }

    #[getter]
    fn bozo(&self) -> bool {
        self.bozo
    }

    #[getter]
    fn bozo_exception(&self) -> Option<&str> {
        self.bozo_exception.as_deref()
    }

    #[getter]
    fn encoding(&self) -> &str {
        &self.encoding
    }

    #[getter]
    fn version(&self) -> &str {
        &self.version
    }

    #[getter]
    fn namespaces(&self, py: Python<'_>) -> Py<PyDict> {
        self.namespaces.clone_ref(py)
    }

    #[getter]
    fn status(&self) -> Option<u16> {
        self.status
    }

    #[getter]
    fn href(&self) -> Option<&str> {
        self.href.as_deref()
    }

    #[getter]
    fn etag(&self) -> Option<&str> {
        self.etag.as_deref()
    }

    #[getter]
    fn modified(&self) -> Option<&str> {
        self.modified.as_deref()
    }

    #[cfg(feature = "http")]
    #[getter]
    fn headers(&self, py: Python<'_>) -> Option<Py<PyDict>> {
        self.headers.as_ref().map(|h| h.clone_ref(py))
    }

    fn __repr__(&self) -> String {
        format!(
            "FeedParserDict(version='{}', bozo={}, entries={})",
            self.version,
            self.bozo,
            self.entries.len()
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Provides backward compatibility for deprecated Python feedparser container names.
    ///
    /// Maps old container names to their modern equivalents:
    /// - `channel` → `feed` (RSS uses <channel>, Atom uses <feed>)
    /// - `items` → `entries` (RSS uses <item>, Atom uses <entry>)
    ///
    /// This method is called by Python when normal attribute lookup fails.
    fn __getattr__(&self, py: Python<'_>, name: &str) -> PyResult<Py<PyAny>> {
        // Check if this is a deprecated container name
        if let Some(new_name) = CONTAINER_FIELD_MAP.get(name) {
            match *new_name {
                "feed" => {
                    // Convert Py<PyFeedMeta> to Py<PyAny>
                    Ok(self.feed.clone_ref(py).into())
                }
                "entries" => {
                    // Convert Vec<Py<PyEntry>> to Py<PyAny> (as Python list)
                    let entries: Vec<_> = self.entries.iter().map(|e| e.clone_ref(py)).collect();
                    match entries.into_pyobject(py) {
                        Ok(list) => Ok(list.unbind()),
                        Err(e) => Err(e),
                    }
                }
                _ => Err(PyAttributeError::new_err(format!(
                    "'FeedParserDict' object has no attribute '{}'",
                    name
                ))),
            }
        } else {
            // Field not found - raise AttributeError
            Err(PyAttributeError::new_err(format!(
                "'FeedParserDict' object has no attribute '{}'",
                name
            )))
        }
    }

    /// Provides dict-style access to fields for Python feedparser compatibility.
    ///
    /// Supports both modern field names and deprecated aliases:
    /// - `d['feed']` → feed metadata
    /// - `d['entries']` → list of entries
    /// - `d['channel']` → feed (deprecated alias)
    /// - `d['items']` → entries (deprecated alias)
    /// - `d['version']`, `d['bozo']`, etc. → top-level fields
    ///
    /// This method is called by Python when using dict-style access: `d[key]`.
    fn __getitem__(&self, py: Python<'_>, key: &str) -> PyResult<Py<PyAny>> {
        // Check for known fields first
        match key {
            "feed" => Ok(self.feed.clone_ref(py).into()),
            "entries" => {
                let entries: Vec<_> = self.entries.iter().map(|e| e.clone_ref(py)).collect();
                Ok(entries.into_pyobject(py)?.into_any().unbind())
            }
            "bozo" => {
                let pybozo = self.bozo.into_pyobject(py)?.to_owned();
                Ok(pybozo.into_any().unbind())
            }
            "bozo_exception" => Ok(self
                .bozo_exception
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "encoding" => Ok(self
                .encoding
                .as_str()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "version" => Ok(self.version.as_str().into_pyobject(py)?.into_any().unbind()),
            "namespaces" => Ok(self.namespaces.clone_ref(py).into()),
            "status" => Ok(self.status.into_pyobject(py)?.into_any().unbind()),
            "href" => Ok(self.href.as_deref().into_pyobject(py)?.into_any().unbind()),
            "etag" => Ok(self.etag.as_deref().into_pyobject(py)?.into_any().unbind()),
            "modified" => Ok(self
                .modified
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            #[cfg(feature = "http")]
            "headers" => {
                if let Some(ref headers) = self.headers {
                    Ok(headers.clone_ref(py).into())
                } else {
                    Ok(py.None().into_pyobject(py)?.into_any().unbind())
                }
            }
            // Check for deprecated container name aliases
            _ => {
                if let Some(new_name) = CONTAINER_FIELD_MAP.get(key) {
                    match *new_name {
                        "feed" => Ok(self.feed.clone_ref(py).into()),
                        "entries" => {
                            let entries: Vec<_> =
                                self.entries.iter().map(|e| e.clone_ref(py)).collect();
                            Ok(entries.into_pyobject(py)?.into_any().unbind())
                        }
                        _ => Err(PyKeyError::new_err(format!("'{}'", key))),
                    }
                } else {
                    // Field not found - raise KeyError
                    Err(PyKeyError::new_err(format!("'{}'", key)))
                }
            }
        }
    }
}
