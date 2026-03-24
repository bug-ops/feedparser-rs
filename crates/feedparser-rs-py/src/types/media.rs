use feedparser_rs::{
    MediaContent as CoreMediaContent, MediaRating as CoreMediaRating,
    MediaThumbnail as CoreMediaThumbnail,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Convert a `MediaRating` into a Python dict `{"scheme": ..., "content": ...}`.
pub fn media_rating_to_py_dict(py: Python<'_>, rating: &CoreMediaRating) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new(py);
    dict.set_item("scheme", rating.scheme.as_deref().into_pyobject(py)?)?;
    dict.set_item("content", &rating.content)?;
    Ok(dict.into_any().unbind())
}

/// Represents a Media RSS thumbnail image.
///
/// Media RSS (MRSS) is a namespace extension for RSS that provides richer media
/// content metadata. Thumbnails are preview images for media content.
#[pyclass(name = "MediaThumbnail", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyMediaThumbnail {
    inner: CoreMediaThumbnail,
}

impl PyMediaThumbnail {
    pub fn from_core(core: CoreMediaThumbnail) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyMediaThumbnail {
    #[getter]
    fn url(&self) -> &str {
        &self.inner.url
    }

    #[getter]
    fn width(&self) -> Option<&str> {
        self.inner.width.as_deref()
    }

    #[getter]
    fn height(&self) -> Option<&str> {
        self.inner.height.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "MediaThumbnail(url='{}', width={:?}, height={:?})",
            self.inner.url, self.inner.width, self.inner.height
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner.url == other.inner.url
            && self.inner.width == other.inner.width
            && self.inner.height == other.inner.height
    }

    fn __getitem__(&self, key: &str) -> PyResult<Option<String>> {
        match key {
            "url" => Ok(Some(self.inner.url.to_string())),
            "width" => Ok(self.inner.width.as_deref().map(str::to_owned)),
            "height" => Ok(self.inner.height.as_deref().map(str::to_owned)),
            _ => Err(pyo3::exceptions::PyKeyError::new_err(key.to_string())),
        }
    }

    fn __contains__(&self, key: &str) -> bool {
        matches!(key, "url" | "width" | "height")
    }

    #[pyo3(signature = (key, default = None))]
    fn get(&self, key: &str, default: Option<String>) -> Option<String> {
        match self.__getitem__(key) {
            Ok(v) => v.or(default),
            Err(_) => default,
        }
    }

    fn keys(&self) -> Vec<&'static str> {
        vec!["url", "width", "height"]
    }

    fn values(&self) -> Vec<Option<String>> {
        vec![
            Some(self.inner.url.to_string()),
            self.inner.width.as_deref().map(str::to_owned),
            self.inner.height.as_deref().map(str::to_owned),
        ]
    }

    fn items(&self) -> Vec<(String, Option<String>)> {
        self.keys()
            .into_iter()
            .zip(self.values())
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }
}

/// Represents a Media RSS content item.
///
/// Media RSS content elements describe actual media files (video, audio, images)
/// with metadata like MIME type, file size, dimensions, and duration.
#[pyclass(name = "MediaContent", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyMediaContent {
    inner: CoreMediaContent,
}

impl PyMediaContent {
    pub fn from_core(core: CoreMediaContent) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyMediaContent {
    #[getter]
    fn url(&self) -> &str {
        &self.inner.url
    }

    #[getter]
    #[pyo3(name = "type")]
    fn content_type(&self) -> Option<&str> {
        self.inner.content_type.as_deref()
    }

    #[getter]
    fn medium(&self) -> Option<&str> {
        self.inner.medium.as_deref()
    }

    #[getter]
    fn filesize(&self) -> Option<u64> {
        self.inner.filesize
    }

    #[getter]
    fn width(&self) -> Option<&str> {
        self.inner.width.as_deref()
    }

    #[getter]
    fn height(&self) -> Option<&str> {
        self.inner.height.as_deref()
    }

    #[getter]
    fn duration(&self) -> Option<&str> {
        self.inner.duration.as_deref()
    }

    #[getter]
    fn bitrate(&self) -> Option<&str> {
        self.inner.bitrate.as_deref()
    }

    #[getter]
    fn lang(&self) -> Option<&str> {
        self.inner.lang.as_deref()
    }

    #[getter]
    fn channels(&self) -> Option<&str> {
        self.inner.channels.as_deref()
    }

    #[getter]
    fn codec(&self) -> Option<&str> {
        self.inner.codec.as_deref()
    }

    #[getter]
    fn expression(&self) -> Option<&str> {
        self.inner.expression.as_deref()
    }

    #[getter]
    fn isdefault(&self) -> Option<&str> {
        self.inner.isdefault.as_deref()
    }

    #[getter]
    fn samplingrate(&self) -> Option<&str> {
        self.inner.samplingrate.as_deref()
    }

    #[getter]
    fn framerate(&self) -> Option<&str> {
        self.inner.framerate.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "MediaContent(url='{}', type='{}')",
            self.inner.url,
            self.inner.content_type.as_deref().unwrap_or("unknown")
        )
    }

    fn __getitem__(&self, key: &str) -> PyResult<Option<String>> {
        match key {
            "url" => Ok(Some(self.inner.url.to_string())),
            "type" => Ok(self.inner.content_type.as_deref().map(str::to_owned)),
            "medium" => Ok(self.inner.medium.as_deref().map(str::to_owned)),
            "width" => Ok(self.inner.width.as_deref().map(str::to_owned)),
            "height" => Ok(self.inner.height.as_deref().map(str::to_owned)),
            "duration" => Ok(self.inner.duration.as_deref().map(str::to_owned)),
            "filesize" => Ok(self.inner.filesize.map(|v| v.to_string())),
            "bitrate" => Ok(self.inner.bitrate.as_deref().map(str::to_owned)),
            "channels" => Ok(self.inner.channels.as_deref().map(str::to_owned)),
            "samplingrate" => Ok(self.inner.samplingrate.as_deref().map(str::to_owned)),
            "framerate" => Ok(self.inner.framerate.as_deref().map(str::to_owned)),
            "lang" => Ok(self.inner.lang.as_deref().map(str::to_owned)),
            "codec" => Ok(self.inner.codec.as_deref().map(str::to_owned)),
            "expression" => Ok(self.inner.expression.as_deref().map(str::to_owned)),
            "isdefault" => Ok(self.inner.isdefault.as_deref().map(str::to_owned)),
            _ => Err(pyo3::exceptions::PyKeyError::new_err(key.to_string())),
        }
    }

    fn __contains__(&self, key: &str) -> bool {
        matches!(
            key,
            "url"
                | "type"
                | "medium"
                | "width"
                | "height"
                | "duration"
                | "filesize"
                | "bitrate"
                | "channels"
                | "samplingrate"
                | "framerate"
                | "lang"
                | "codec"
                | "expression"
                | "isdefault"
        )
    }

    #[pyo3(signature = (key, default = None))]
    fn get(&self, key: &str, default: Option<String>) -> Option<String> {
        match self.__getitem__(key) {
            Ok(v) => v.or(default),
            Err(_) => default,
        }
    }

    fn keys(&self) -> Vec<&'static str> {
        vec![
            "url",
            "type",
            "medium",
            "width",
            "height",
            "duration",
            "filesize",
            "bitrate",
            "channels",
            "samplingrate",
            "framerate",
            "lang",
            "codec",
            "expression",
            "isdefault",
        ]
    }

    fn values(&self) -> Vec<Option<String>> {
        vec![
            Some(self.inner.url.to_string()),
            self.inner.content_type.as_deref().map(str::to_owned),
            self.inner.medium.as_deref().map(str::to_owned),
            self.inner.width.as_deref().map(str::to_owned),
            self.inner.height.as_deref().map(str::to_owned),
            self.inner.duration.as_deref().map(str::to_owned),
            self.inner.filesize.map(|v| v.to_string()),
            self.inner.bitrate.as_deref().map(str::to_owned),
            self.inner.channels.as_deref().map(str::to_owned),
            self.inner.samplingrate.as_deref().map(str::to_owned),
            self.inner.framerate.as_deref().map(str::to_owned),
            self.inner.lang.as_deref().map(str::to_owned),
            self.inner.codec.as_deref().map(str::to_owned),
            self.inner.expression.as_deref().map(str::to_owned),
            self.inner.isdefault.as_deref().map(str::to_owned),
        ]
    }

    fn items(&self) -> Vec<(String, Option<String>)> {
        self.keys()
            .into_iter()
            .zip(self.values())
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner.url == other.inner.url
            && self.inner.content_type == other.inner.content_type
            && self.inner.medium == other.inner.medium
            && self.inner.filesize == other.inner.filesize
            && self.inner.width == other.inner.width
            && self.inner.height == other.inner.height
            && self.inner.duration == other.inner.duration
            && self.inner.bitrate == other.inner.bitrate
            && self.inner.lang == other.inner.lang
            && self.inner.channels == other.inner.channels
            && self.inner.codec == other.inner.codec
            && self.inner.expression == other.inner.expression
            && self.inner.isdefault == other.inner.isdefault
            && self.inner.samplingrate == other.inner.samplingrate
            && self.inner.framerate == other.inner.framerate
    }
}
