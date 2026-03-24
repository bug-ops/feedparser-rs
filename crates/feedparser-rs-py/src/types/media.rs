use feedparser_rs::{MediaContent as CoreMediaContent, MediaThumbnail as CoreMediaThumbnail};
use pyo3::prelude::*;

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

    fn __repr__(&self) -> String {
        format!(
            "MediaContent(url='{}', type='{}')",
            self.inner.url,
            self.inner.content_type.as_deref().unwrap_or("unknown")
        )
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
    }
}
