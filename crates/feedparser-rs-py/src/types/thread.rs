use feedparser_rs::InReplyTo as CoreInReplyTo;
use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;

/// Atom Threading Extensions in-reply-to reference.
///
/// Represents a single `thr:in-reply-to` element from RFC 4685.
/// Provides both attribute access and dict-style access for Python feedparser compatibility.
#[pyclass(name = "InReplyTo", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyInReplyTo {
    inner: CoreInReplyTo,
}

impl PyInReplyTo {
    pub fn from_core(core: CoreInReplyTo) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyInReplyTo {
    /// IRI of the entry being replied to (ref attribute)
    ///
    /// Named `ref_` to avoid Rust keyword conflict; Python attribute name is `ref`.
    #[getter]
    #[pyo3(name = "ref")]
    fn ref_(&self) -> Option<&str> {
        self.inner.ref_.as_deref()
    }

    /// URL where the referenced entry can be found (href attribute)
    #[getter]
    fn href(&self) -> Option<&str> {
        self.inner.href.as_deref()
    }

    /// MIME type of the linked resource (type attribute)
    ///
    /// Named `type_` to avoid Rust keyword conflict; Python attribute name is `type`.
    #[getter]
    #[pyo3(name = "type")]
    fn type_(&self) -> Option<&str> {
        self.inner.type_.as_deref()
    }

    /// IRI of the feed containing the referenced entry (source attribute)
    #[getter]
    fn source(&self) -> Option<&str> {
        self.inner.source.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "InReplyTo(ref='{}')",
            self.inner.ref_.as_deref().unwrap_or("")
        )
    }

    /// Dict-style access for Python feedparser compatibility.
    ///
    /// Supports keys: `ref`, `href`, `type`, `source`.
    fn __getitem__(&self, py: Python<'_>, key: &str) -> PyResult<Py<PyAny>> {
        match key {
            "ref" => Ok(self
                .inner
                .ref_
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "href" => Ok(self
                .inner
                .href
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "type" => Ok(self
                .inner
                .type_
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "source" => Ok(self
                .inner
                .source
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            _ => Err(PyKeyError::new_err(format!(
                "'InReplyTo' has no key '{}'",
                key
            ))),
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
