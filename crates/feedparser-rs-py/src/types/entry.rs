use feedparser_rs::Entry as CoreEntry;
use feedparser_rs::namespace::georss::GeoLocation as CoreGeoLocation;
use pyo3::exceptions::{PyAttributeError, PyKeyError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use super::common::{PyContent, PyEnclosure, PyLink, PyPerson, PySource, PyTag, PyTextConstruct};
use super::compat::ENTRY_FIELD_MAP;
use super::datetime::optional_datetime_to_struct_time;
use super::media::{
    PyMediaContent, PyMediaCopyright, PyMediaCredit, PyMediaRating, PyMediaThumbnail,
};
use super::podcast::{PyItunesEntryMeta, PyPodcastEntryMeta, PyPodcastPerson, PyPodcastTranscript};
use super::thread::PyInReplyTo;

/// Convert a [`CoreGeoLocation`] to a Python dict matching the Python feedparser `where` format:
/// `{'type': 'Point', 'coordinates': (lon, lat)}` (GeoJSON coordinate order).
fn geo_location_to_py_dict(py: Python<'_>, geo: &CoreGeoLocation) -> PyResult<Py<PyAny>> {
    use feedparser_rs::namespace::georss::GeoType;
    let dict = PyDict::new(py);
    let type_str = match geo.geo_type {
        GeoType::Point => "Point",
        GeoType::Line => "LineString",
        GeoType::Polygon => "Polygon",
        GeoType::Box => "Box",
    };
    dict.set_item("type", type_str)?;
    match geo.geo_type {
        GeoType::Point => {
            if let Some(&(lat, lon)) = geo.coordinates.first() {
                dict.set_item("coordinates", (lon, lat))?;
            } else {
                dict.set_item("coordinates", py.None())?;
            }
        }
        _ => {
            let coords: Vec<(f64, f64)> = geo
                .coordinates
                .iter()
                .map(|&(lat, lon)| (lon, lat))
                .collect();
            dict.set_item("coordinates", coords)?;
        }
    }
    Ok(dict.into_any().unbind())
}

#[pyclass(name = "Entry", module = "feedparser_rs", from_py_object)]
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
    fn published(&self) -> Option<&str> {
        self.inner.published_str.as_deref()
    }

    #[getter]
    fn published_parsed(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        optional_datetime_to_struct_time(py, &self.inner.published)
    }

    #[getter]
    fn updated(&self) -> Option<&str> {
        self.inner.updated_str.as_deref()
    }

    #[getter]
    fn updated_parsed(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        optional_datetime_to_struct_time(py, &self.inner.updated)
    }

    #[getter]
    fn created(&self) -> Option<&str> {
        self.inner.created_str.as_deref()
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
    #[pyo3(name = "where")]
    fn where_field(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        self.inner
            .r#where
            .as_deref()
            .map(|g| geo_location_to_py_dict(py, g))
            .transpose()
    }

    #[getter]
    fn geo_lat(&self) -> Option<&str> {
        self.inner.geo_lat.as_deref()
    }

    #[getter]
    fn geo_long(&self) -> Option<&str> {
        self.inner.geo_long.as_deref()
    }

    #[getter]
    fn dc_creator(&self) -> Option<&str> {
        self.inner.dc_creator.as_deref()
    }

    #[getter]
    fn slash_comments(&self) -> Option<String> {
        self.inner.slash_comments.map(|n| n.to_string())
    }

    #[getter]
    fn slash_hit_parade(&self) -> Option<&str> {
        self.inner.slash_hit_parade.as_deref()
    }

    #[getter]
    fn wfw_commentrss(&self) -> Option<&str> {
        self.inner.wfw_comment_rss.as_deref()
    }

    #[getter]
    fn guidislink(&self) -> Option<bool> {
        self.inner.guidislink
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
    fn dc_rights(&self) -> Option<&str> {
        self.inner.dc_rights.as_deref()
    }

    #[getter]
    fn dc_subject(&self) -> Vec<String> {
        self.inner.dc_subject.clone()
    }

    #[getter]
    fn media_thumbnail(&self) -> Vec<PyMediaThumbnail> {
        self.inner
            .media_thumbnail
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
    fn media_credit(&self) -> Vec<PyMediaCredit> {
        self.inner
            .media_credit
            .iter()
            .map(|c| PyMediaCredit::from_core(c.clone()))
            .collect()
    }

    #[getter]
    fn media_copyright(&self) -> Option<PyMediaCopyright> {
        self.inner
            .media_copyright
            .as_ref()
            .map(|c| PyMediaCopyright::from_core(c.clone()))
    }

    #[getter]
    fn media_rating(&self) -> Option<PyMediaRating> {
        self.inner
            .media_rating
            .as_ref()
            .map(|r| PyMediaRating::from_core(r.clone()))
    }

    #[getter]
    fn media_keywords(&self) -> Option<&str> {
        self.inner.media_keywords.as_deref()
    }

    #[getter]
    fn media_description(&self) -> Option<&str> {
        self.inner.media_description.as_deref()
    }

    #[getter]
    fn podcast(&self) -> Option<PyPodcastEntryMeta> {
        self.inner
            .podcast
            .as_deref()
            .map(|p| PyPodcastEntryMeta::from_core(p.clone()))
    }

    /// Atom Threading Extensions: entries this entry is a reply to.
    ///
    /// Returns a list of `InReplyTo` objects, one per `thr:in-reply-to` element.
    /// Python feedparser exposes this as `entry.thr_in_reply_to`.
    #[getter]
    fn thr_in_reply_to(&self) -> Vec<PyInReplyTo> {
        self.inner
            .in_reply_to
            .iter()
            .map(|r| PyInReplyTo::from_core(r.clone()))
            .collect()
    }

    /// Atom Threading Extensions: total response count.
    ///
    /// Returns the `thr:total` value as a string for Python feedparser compatibility.
    /// Python feedparser returns this as a string (e.g., `"5"`), not an integer.
    #[getter]
    fn thr_total(&self) -> Option<String> {
        self.inner.thr_total.map(|n| n.to_string())
    }

    /// Returns the value for `key` if present, otherwise returns `default` (None if omitted).
    ///
    /// Provides `dict.get()` compatibility for Python feedparser consumers.
    /// Unlike `__getitem__`, this method never raises `KeyError`.
    #[pyo3(signature = (key, default = None))]
    fn get(&self, py: Python<'_>, key: &str, default: Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
        match self.__getitem__(py, key) {
            Ok(v) if v.is_none(py) => Ok(default.unwrap_or_else(|| py.None())),
            Ok(v) => Ok(v),
            Err(_) => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    /// Returns a list of field names whose values are not None.
    fn keys(&self, py: Python<'_>) -> PyResult<Vec<&'static str>> {
        const ALL_KEYS: &[&str] = &[
            "id",
            "title",
            "title_detail",
            "subtitle",
            "subtitle_detail",
            "link",
            "links",
            "summary",
            "summary_detail",
            "content",
            "published",
            "published_parsed",
            "updated",
            "updated_parsed",
            "created",
            "created_parsed",
            "expired",
            "expired_parsed",
            "author",
            "author_detail",
            "authors",
            "contributors",
            "publisher",
            "publisher_detail",
            "tags",
            "enclosures",
            "comments",
            "source",
            "itunes",
            "podcast_transcripts",
            "podcast_persons",
            "license",
            "where",
            "dc_creator",
            "dc_date",
            "dc_date_parsed",
            "slash_comments",
            "slash_hit_parade",
            "wfw_commentrss",
            "rights",
            "rights_detail",
            "dc_rights",
            "dc_subject",
            "media_thumbnail",
            "media_content",
            "media_credit",
            "media_copyright",
            "media_rating",
            "media_keywords",
            "media_description",
            "podcast",
            "thr_in_reply_to",
            "thr_total",
            "guidislink",
        ];
        let mut result = Vec::new();
        for &key in ALL_KEYS {
            let value = self.__getitem__(py, key)?;
            if !value.is_none(py) {
                result.push(key);
            }
        }
        Ok(result)
    }

    /// Returns a list of field values for all non-None fields.
    fn values(&self, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
        let keys = self.keys(py)?;
        keys.into_iter()
            .map(|key| self.__getitem__(py, key))
            .collect()
    }

    /// Returns a list of `(key, value)` pairs for all non-None fields.
    fn items(&self, py: Python<'_>) -> PyResult<Vec<(String, Py<PyAny>)>> {
        let keys = self.keys(py)?;
        keys.into_iter()
            .map(|key| Ok((key.to_string(), self.__getitem__(py, key)?)))
            .collect()
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
                    "id" => self
                        .inner
                        .id
                        .as_deref()
                        .and_then(|v| v.into_pyobject(py).map(|o| o.unbind().into()).ok()),
                    "summary" => self
                        .inner
                        .summary
                        .as_deref()
                        .and_then(|v| v.into_pyobject(py).map(|o| o.unbind().into()).ok()),
                    "summary_detail" => self.inner.summary_detail.as_ref().and_then(|tc| {
                        Py::new(py, PyTextConstruct::from_core(tc.clone()))
                            .ok()
                            .map(|p: Py<PyTextConstruct>| p.into_any())
                    }),
                    "published" => self
                        .inner
                        .published_str
                        .as_deref()
                        .and_then(|v| v.into_pyobject(py).map(|o| o.unbind().into()).ok()),
                    "published_parsed" => {
                        optional_datetime_to_struct_time(py, &self.inner.published)
                            .ok()
                            .flatten()
                    }
                    "updated" => self
                        .inner
                        .updated_str
                        .as_deref()
                        .and_then(|v| v.into_pyobject(py).map(|o| o.unbind().into()).ok()),
                    "updated_parsed" => optional_datetime_to_struct_time(py, &self.inner.updated)
                        .ok()
                        .flatten(),
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

    /// Provides dict-style access to fields for Python feedparser compatibility.
    ///
    /// Supports both modern field names and deprecated aliases.
    /// This method is called by Python when using dict-style access: `entry['title']`.
    ///
    /// Raises KeyError for unknown keys (unlike __getattr__ which raises AttributeError).
    fn __getitem__(&self, py: Python<'_>, key: &str) -> PyResult<Py<PyAny>> {
        // Check for known fields first
        match key {
            "id" => Ok(self
                .inner
                .id
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
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
            "summary" => Ok(self
                .inner
                .summary
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "summary_detail" => {
                if let Some(ref tc) = self.inner.summary_detail {
                    Ok(Py::new(py, PyTextConstruct::from_core(tc.clone()))?.into_any())
                } else {
                    Ok(py.None())
                }
            }
            "content" => {
                let content: Vec<_> = self
                    .inner
                    .content
                    .iter()
                    .map(|c| PyContent::from_core(c.clone()))
                    .collect();
                Ok(content.into_pyobject(py)?.into_any().unbind())
            }
            "published" => Ok(self
                .inner
                .published_str
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "published_parsed" => Ok(optional_datetime_to_struct_time(py, &self.inner.published)?
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "updated" => Ok(self
                .inner
                .updated_str
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "updated_parsed" => Ok(optional_datetime_to_struct_time(py, &self.inner.updated)?
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "created" => Ok(self
                .inner
                .created
                .map(|dt| dt.to_rfc3339())
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "created_parsed" => Ok(optional_datetime_to_struct_time(py, &self.inner.created)?
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "expired" => Ok(self
                .inner
                .expired
                .map(|dt| dt.to_rfc3339())
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "expired_parsed" => Ok(optional_datetime_to_struct_time(py, &self.inner.expired)?
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
            "tags" => {
                let tags: Vec<_> = self
                    .inner
                    .tags
                    .iter()
                    .map(|t| PyTag::from_core(t.clone()))
                    .collect();
                Ok(tags.into_pyobject(py)?.into_any().unbind())
            }
            "enclosures" => {
                let enclosures: Vec<_> = self
                    .inner
                    .enclosures
                    .iter()
                    .map(|e| PyEnclosure::from_core(e.clone()))
                    .collect();
                Ok(enclosures.into_pyobject(py)?.into_any().unbind())
            }
            "comments" => Ok(self
                .inner
                .comments
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "source" => {
                if let Some(ref s) = self.inner.source {
                    Ok(Py::new(py, PySource::from_core(s.clone()))?.into_any())
                } else {
                    Ok(py.None())
                }
            }
            "itunes" => {
                if let Some(ref i) = self.inner.itunes {
                    Ok(Py::new(py, PyItunesEntryMeta::from_core(i.as_ref().clone()))?.into_any())
                } else {
                    Ok(py.None())
                }
            }
            // Flat itunes_* keys for Python feedparser compatibility
            "itunes_author" => Ok(self
                .inner
                .itunes
                .as_ref()
                .and_then(|i| i.author.as_deref())
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "itunes_duration" => Ok(self
                .inner
                .itunes
                .as_ref()
                .and_then(|i| i.duration.as_deref())
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "itunes_episode" => Ok(self
                .inner
                .itunes
                .as_ref()
                .and_then(|i| i.episode.as_deref())
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "itunes_season" => Ok(self
                .inner
                .itunes
                .as_ref()
                .and_then(|i| i.season.as_deref())
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "itunes_explicit" => Ok(self
                .inner
                .itunes
                .as_ref()
                .and_then(|i| i.explicit)
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "itunes_episodetype" => Ok(self
                .inner
                .itunes
                .as_ref()
                .and_then(|i| i.episode_type.as_deref())
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "itunes_image" => Ok(self
                .inner
                .itunes
                .as_ref()
                .and_then(|i| i.image.as_deref())
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "itunes_title" => Ok(self
                .inner
                .itunes
                .as_ref()
                .and_then(|i| i.title.as_deref())
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "podcast_transcripts" => {
                let transcripts: Vec<_> = self
                    .inner
                    .podcast_transcripts
                    .iter()
                    .map(|t| PyPodcastTranscript::from_core(t.clone()))
                    .collect();
                Ok(transcripts.into_pyobject(py)?.into_any().unbind())
            }
            "podcast_persons" => {
                let persons: Vec<_> = self
                    .inner
                    .podcast_persons
                    .iter()
                    .map(|p| PyPodcastPerson::from_core(p.clone()))
                    .collect();
                Ok(persons.into_pyobject(py)?.into_any().unbind())
            }
            "license" => Ok(self
                .inner
                .license
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "where" | "where_" => {
                if let Some(ref g) = self.inner.r#where {
                    Ok(geo_location_to_py_dict(py, g.as_ref())?)
                } else {
                    Ok(py.None())
                }
            }
            "geo_lat" => Ok(self
                .inner
                .geo_lat
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "geo_long" => Ok(self
                .inner
                .geo_long
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "dc_creator" => Ok(self
                .inner
                .dc_creator
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "dc_date" => Ok(self
                .inner
                .dc_date
                .map(|dt| dt.to_rfc3339())
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "dc_date_parsed" => Ok(optional_datetime_to_struct_time(py, &self.inner.dc_date)?
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "slash_comments" => Ok(self
                .inner
                .slash_comments
                .map(|n| n.to_string())
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "slash_hit_parade" => Ok(self
                .inner
                .slash_hit_parade
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "wfw_commentrss" => Ok(self
                .inner
                .wfw_comment_rss
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
            "copyright" => Ok(self
                .inner
                .rights
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "copyright_detail" => {
                if let Some(ref tc) = self.inner.rights_detail {
                    Ok(Py::new(py, PyTextConstruct::from_core(tc.clone()))?.into_any())
                } else {
                    Ok(py.None())
                }
            }
            "dc_rights" => Ok(self
                .inner
                .dc_rights
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "dc_subject" => Ok(self
                .inner
                .dc_subject
                .clone()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "media_thumbnail" => {
                let thumbnails: Vec<_> = self
                    .inner
                    .media_thumbnail
                    .iter()
                    .map(|t| PyMediaThumbnail::from_core(t.clone()))
                    .collect();
                Ok(thumbnails.into_pyobject(py)?.into_any().unbind())
            }
            "media_content" => {
                let content: Vec<_> = self
                    .inner
                    .media_content
                    .iter()
                    .map(|c| PyMediaContent::from_core(c.clone()))
                    .collect();
                Ok(content.into_pyobject(py)?.into_any().unbind())
            }
            "media_credit" => {
                let credits: Vec<_> = self
                    .inner
                    .media_credit
                    .iter()
                    .map(|c| PyMediaCredit::from_core(c.clone()))
                    .collect();
                Ok(credits.into_pyobject(py)?.into_any().unbind())
            }
            "media_copyright" => {
                if let Some(ref c) = self.inner.media_copyright {
                    Ok(Py::new(py, PyMediaCopyright::from_core(c.clone()))?.into_any())
                } else {
                    Ok(py.None())
                }
            }
            "media_rating" => {
                if let Some(ref r) = self.inner.media_rating {
                    Ok(Py::new(py, PyMediaRating::from_core(r.clone()))?.into_any())
                } else {
                    Ok(py.None())
                }
            }
            "media_keywords" => Ok(self
                .inner
                .media_keywords
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "media_description" => Ok(self
                .inner
                .media_description
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "podcast" => {
                if let Some(ref p) = self.inner.podcast {
                    Ok(Py::new(py, PyPodcastEntryMeta::from_core(p.as_ref().clone()))?.into_any())
                } else {
                    Ok(py.None())
                }
            }
            // `thr_in_reply_to` (underscore, rs-native): returns list of all InReplyTo objects
            "thr_in_reply_to" => {
                let replies: Vec<_> = self
                    .inner
                    .in_reply_to
                    .iter()
                    .map(|r| PyInReplyTo::from_core(r.clone()))
                    .collect();
                Ok(replies.into_pyobject(py)?.into_any().unbind())
            }
            // `thr_in-reply-to` (hyphen): Python feedparser compat — returns first element as dict
            "thr_in-reply-to" => {
                let Some(first) = self.inner.in_reply_to.first() else {
                    return Ok(py.None());
                };
                let dict = pyo3::types::PyDict::new(py);
                if let Some(v) = first.ref_.as_deref() {
                    dict.set_item("ref", v)?;
                }
                if let Some(v) = first.href.as_deref() {
                    dict.set_item("href", v)?;
                }
                if let Some(v) = first.type_.as_deref() {
                    dict.set_item("type", v)?;
                }
                if let Some(v) = first.source.as_deref() {
                    dict.set_item("source", v)?;
                }
                Ok(dict.into_any().unbind())
            }
            "thr_total" => {
                // Return as string for Python feedparser compatibility
                let value = self.inner.thr_total.map(|n| n.to_string());
                Ok(value.into_pyobject(py)?.into_any().unbind())
            }
            "guidislink" => Ok(self.inner.guidislink.into_pyobject(py)?.into_any().unbind()),
            // Check for deprecated field name aliases
            _ => {
                if let Some(new_names) = ENTRY_FIELD_MAP.get(key) {
                    // Try each new field name in order
                    for new_name in new_names {
                        let value: Option<Py<PyAny>> =
                            match *new_name {
                                "id" => self.inner.id.as_deref().and_then(|v| {
                                    v.into_pyobject(py).map(|o| o.unbind().into()).ok()
                                }),
                                "summary" => self.inner.summary.as_deref().and_then(|v| {
                                    v.into_pyobject(py).map(|o| o.unbind().into()).ok()
                                }),
                                "summary_detail" => {
                                    self.inner.summary_detail.as_ref().and_then(|tc| {
                                        Py::new(py, PyTextConstruct::from_core(tc.clone()))
                                            .ok()
                                            .map(|p: Py<PyTextConstruct>| p.into_any())
                                    })
                                }
                                "published" => self.inner.published_str.as_deref().and_then(|v| {
                                    v.into_pyobject(py).map(|o| o.unbind().into()).ok()
                                }),
                                "published_parsed" => {
                                    optional_datetime_to_struct_time(py, &self.inner.published)
                                        .ok()
                                        .flatten()
                                }
                                "updated" => self.inner.updated_str.as_deref().and_then(|v| {
                                    v.into_pyobject(py).map(|o| o.unbind().into()).ok()
                                }),
                                "updated_parsed" => {
                                    optional_datetime_to_struct_time(py, &self.inner.updated)
                                        .ok()
                                        .flatten()
                                }
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
