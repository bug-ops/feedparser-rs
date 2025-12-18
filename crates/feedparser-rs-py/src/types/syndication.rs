use feedparser_rs::SyndicationMeta as CoreSyndicationMeta;
use pyo3::prelude::*;

/// Syndication module metadata
#[pyclass(name = "SyndicationMeta", module = "feedparser_rs")]
#[derive(Clone)]
pub struct PySyndicationMeta {
    inner: CoreSyndicationMeta,
}

impl PySyndicationMeta {
    pub fn from_core(core: CoreSyndicationMeta) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PySyndicationMeta {
    /// Update period (hourly, daily, weekly, monthly, yearly)
    #[getter]
    fn update_period(&self) -> Option<&str> {
        self.inner.update_period.as_ref().map(|p| p.as_str())
    }

    /// Number of times updated per period
    #[getter]
    fn update_frequency(&self) -> Option<u32> {
        self.inner.update_frequency
    }

    /// Base date for update schedule (ISO 8601)
    #[getter]
    fn update_base(&self) -> Option<&str> {
        self.inner.update_base.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "SyndicationMeta(update_period={:?}, update_frequency={:?}, update_base={:?})",
            self.inner.update_period.as_ref().map(|p| p.as_str()),
            self.inner.update_frequency,
            self.inner.update_base.as_deref()
        )
    }
}
