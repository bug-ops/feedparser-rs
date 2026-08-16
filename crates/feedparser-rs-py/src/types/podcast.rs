use feedparser_rs::types::{
    PodcastAlternateEnclosure as CorePodcastAlternateEnclosure,
    PodcastAlternateEnclosureSource as CorePodcastAlternateEnclosureSource,
    PodcastFollow as CorePodcastFollow, PodcastIntegrity as CorePodcastIntegrity,
    PodcastLocation as CorePodcastLocation, PodcastRemoteItem as CorePodcastRemoteItem,
    PodcastSocialInteract as CorePodcastSocialInteract, PodcastTxt as CorePodcastTxt,
    PodcastUpdateFrequency as CorePodcastUpdateFrequency,
};
use feedparser_rs::{
    ItunesCategory as CoreItunesCategory, ItunesEntryMeta as CoreItunesEntryMeta,
    ItunesFeedMeta as CoreItunesFeedMeta, ItunesOwner as CoreItunesOwner,
    PodcastChapters as CorePodcastChapters, PodcastChat as CorePodcastChat,
    PodcastEntryMeta as CorePodcastEntryMeta, PodcastFunding as CorePodcastFunding,
    PodcastMeta as CorePodcastMeta, PodcastPerson as CorePodcastPerson,
    PodcastSoundbite as CorePodcastSoundbite, PodcastTranscript as CorePodcastTranscript,
    PodcastValue as CorePodcastValue, PodcastValueRecipient as CorePodcastValueRecipient,
    PodcastValueTimeSplit as CorePodcastValueTimeSplit,
};
use pyo3::prelude::*;

#[pyclass(name = "ItunesFeedMeta", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyItunesFeedMeta {
    inner: CoreItunesFeedMeta,
}

impl PyItunesFeedMeta {
    pub fn from_core(core: CoreItunesFeedMeta) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyItunesFeedMeta {
    #[getter]
    fn author(&self) -> Option<&str> {
        self.inner.author.as_deref()
    }

    #[getter]
    fn owner(&self) -> Option<PyItunesOwner> {
        self.inner
            .owner
            .as_ref()
            .map(|o| PyItunesOwner::from_core(o.clone()))
    }

    #[getter]
    fn categories(&self) -> Vec<PyItunesCategory> {
        self.inner
            .categories
            .iter()
            .map(|c| PyItunesCategory::from_core(c.clone()))
            .collect()
    }

    #[getter]
    fn explicit(&self) -> Option<bool> {
        self.inner.explicit
    }

    #[getter]
    fn image(&self) -> Option<&str> {
        self.inner.image.as_deref()
    }

    #[getter]
    fn keywords(&self) -> Vec<String> {
        self.inner.keywords.clone()
    }

    #[getter]
    fn podcast_type(&self) -> Option<&str> {
        self.inner.podcast_type.as_deref()
    }

    #[getter]
    fn complete(&self) -> Option<&str> {
        self.inner.complete.as_deref()
    }

    #[getter]
    fn new_feed_url(&self) -> Option<&str> {
        self.inner.new_feed_url.as_deref()
    }

    #[getter]
    fn block(&self) -> Option<u8> {
        self.inner.block
    }

    #[getter]
    fn subtitle(&self) -> Option<&str> {
        self.inner.subtitle.as_deref()
    }

    #[getter]
    fn summary(&self) -> Option<&str> {
        self.inner.summary.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "ItunesFeedMeta(author='{}', categories={})",
            self.inner.author.as_deref().unwrap_or("unknown"),
            self.inner.categories.len()
        )
    }

    fn __getitem__(&self, py: Python<'_>, key: &str) -> PyResult<pyo3::Py<pyo3::PyAny>> {
        use pyo3::IntoPyObjectExt;
        match key {
            "author" => self.inner.author.as_deref().into_py_any(py),
            "explicit" => self.inner.explicit.into_py_any(py),
            "image" => self.inner.image.as_deref().into_py_any(py),
            "keywords" => self.inner.keywords.clone().into_py_any(py),
            "podcast_type" => self.inner.podcast_type.as_deref().into_py_any(py),
            "complete" => self.inner.complete.as_deref().into_py_any(py),
            "new_feed_url" => self.inner.new_feed_url.as_deref().into_py_any(py),
            "block" => self.inner.block.into_py_any(py),
            "subtitle" => self.inner.subtitle.as_deref().into_py_any(py),
            "summary" => self.inner.summary.as_deref().into_py_any(py),
            _ => Err(pyo3::exceptions::PyKeyError::new_err(key.to_string())),
        }
    }

    fn __contains__(&self, key: &str) -> bool {
        matches!(
            key,
            "author"
                | "explicit"
                | "image"
                | "keywords"
                | "podcast_type"
                | "complete"
                | "new_feed_url"
                | "block"
                | "subtitle"
                | "summary"
        )
    }

    #[pyo3(signature = (key, default = None))]
    fn get(
        &self,
        py: Python<'_>,
        key: &str,
        default: Option<pyo3::Py<pyo3::PyAny>>,
    ) -> PyResult<pyo3::Py<pyo3::PyAny>> {
        match self.__getitem__(py, key) {
            Ok(v) if v.is_none(py) => Ok(default.unwrap_or_else(|| py.None())),
            Ok(v) => Ok(v),
            Err(_) => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    fn keys(&self) -> Vec<&'static str> {
        vec![
            "author",
            "explicit",
            "image",
            "keywords",
            "podcast_type",
            "complete",
            "new_feed_url",
            "block",
            "subtitle",
            "summary",
        ]
    }

    fn values(&self, py: Python<'_>) -> PyResult<Vec<pyo3::Py<pyo3::PyAny>>> {
        self.keys()
            .into_iter()
            .map(|key| self.__getitem__(py, key))
            .collect()
    }

    fn items(&self, py: Python<'_>) -> PyResult<Vec<(String, pyo3::Py<pyo3::PyAny>)>> {
        self.keys()
            .into_iter()
            .map(|key| Ok((key.to_string(), self.__getitem__(py, key)?)))
            .collect()
    }
}

#[pyclass(name = "ItunesEntryMeta", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyItunesEntryMeta {
    inner: CoreItunesEntryMeta,
}

impl PyItunesEntryMeta {
    pub fn from_core(core: CoreItunesEntryMeta) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyItunesEntryMeta {
    #[getter]
    fn title(&self) -> Option<&str> {
        self.inner.title.as_deref()
    }

    #[getter]
    fn author(&self) -> Option<&str> {
        self.inner.author.as_deref()
    }

    #[getter]
    fn duration(&self) -> Option<&str> {
        self.inner.duration.as_deref()
    }

    #[getter]
    fn explicit(&self) -> Option<bool> {
        self.inner.explicit
    }

    #[getter]
    fn image(&self) -> Option<&str> {
        self.inner.image.as_deref()
    }

    #[getter]
    fn episode(&self) -> Option<&str> {
        self.inner.episode.as_deref()
    }

    #[getter]
    fn season(&self) -> Option<&str> {
        self.inner.season.as_deref()
    }

    #[getter]
    fn episode_type(&self) -> Option<&str> {
        self.inner.episode_type.as_deref()
    }

    #[getter]
    fn subtitle(&self) -> Option<&str> {
        self.inner.subtitle.as_deref()
    }

    #[getter]
    fn summary(&self) -> Option<&str> {
        self.inner.summary.as_deref()
    }

    fn __repr__(&self) -> String {
        if let (Some(season), Some(episode)) =
            (self.inner.season.as_deref(), self.inner.episode.as_deref())
        {
            format!("ItunesEntryMeta(season={season}, episode={episode})")
        } else {
            "ItunesEntryMeta()".to_string()
        }
    }

    fn __getitem__(&self, py: Python<'_>, key: &str) -> PyResult<pyo3::Py<pyo3::PyAny>> {
        use pyo3::IntoPyObjectExt;
        match key {
            "title" => self.inner.title.as_deref().into_py_any(py),
            "author" => self.inner.author.as_deref().into_py_any(py),
            "duration" => self.inner.duration.as_deref().into_py_any(py),
            "explicit" => self.inner.explicit.into_py_any(py),
            "image" => self.inner.image.as_deref().into_py_any(py),
            "episode" => self.inner.episode.as_deref().into_py_any(py),
            "season" => self.inner.season.as_deref().into_py_any(py),
            "episode_type" => self.inner.episode_type.as_deref().into_py_any(py),
            "subtitle" => self.inner.subtitle.as_deref().into_py_any(py),
            "summary" => self.inner.summary.as_deref().into_py_any(py),
            _ => Err(pyo3::exceptions::PyKeyError::new_err(key.to_string())),
        }
    }

    fn __contains__(&self, key: &str) -> bool {
        matches!(
            key,
            "title"
                | "author"
                | "duration"
                | "explicit"
                | "image"
                | "episode"
                | "season"
                | "episode_type"
                | "subtitle"
                | "summary"
        )
    }

    #[pyo3(signature = (key, default = None))]
    fn get(
        &self,
        py: Python<'_>,
        key: &str,
        default: Option<pyo3::Py<pyo3::PyAny>>,
    ) -> PyResult<pyo3::Py<pyo3::PyAny>> {
        match self.__getitem__(py, key) {
            Ok(v) if v.is_none(py) => Ok(default.unwrap_or_else(|| py.None())),
            Ok(v) => Ok(v),
            Err(_) => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    fn keys(&self) -> Vec<&'static str> {
        vec![
            "title",
            "author",
            "duration",
            "explicit",
            "image",
            "episode",
            "season",
            "episode_type",
            "subtitle",
            "summary",
        ]
    }

    fn values(&self, py: Python<'_>) -> PyResult<Vec<pyo3::Py<pyo3::PyAny>>> {
        self.keys()
            .into_iter()
            .map(|key| self.__getitem__(py, key))
            .collect()
    }

    fn items(&self, py: Python<'_>) -> PyResult<Vec<(String, pyo3::Py<pyo3::PyAny>)>> {
        self.keys()
            .into_iter()
            .map(|key| Ok((key.to_string(), self.__getitem__(py, key)?)))
            .collect()
    }
}

#[pyclass(name = "ItunesOwner", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyItunesOwner {
    inner: CoreItunesOwner,
}

impl PyItunesOwner {
    pub fn from_core(core: CoreItunesOwner) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyItunesOwner {
    #[getter]
    fn name(&self) -> Option<&str> {
        self.inner.name.as_deref()
    }

    #[getter]
    fn email(&self) -> Option<&str> {
        self.inner.email.as_deref()
    }

    fn __repr__(&self) -> String {
        if let Some(name) = &self.inner.name {
            format!("ItunesOwner(name='{}')", name)
        } else {
            "ItunesOwner()".to_string()
        }
    }
}

#[pyclass(name = "ItunesCategory", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyItunesCategory {
    inner: CoreItunesCategory,
}

impl PyItunesCategory {
    pub fn from_core(core: CoreItunesCategory) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyItunesCategory {
    #[getter]
    fn text(&self) -> &str {
        &self.inner.text
    }

    #[getter]
    fn subcategory(&self) -> Option<&str> {
        self.inner.subcategory.as_deref()
    }

    fn __repr__(&self) -> String {
        if let Some(sub) = &self.inner.subcategory {
            format!(
                "ItunesCategory(text='{}', subcategory='{}')",
                self.inner.text, sub
            )
        } else {
            format!("ItunesCategory(text='{}')", self.inner.text)
        }
    }
}

#[pyclass(name = "PodcastMeta", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyPodcastMeta {
    inner: CorePodcastMeta,
}

impl PyPodcastMeta {
    pub fn from_core(core: CorePodcastMeta) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyPodcastMeta {
    /// Returns podcast transcripts at feed level.
    ///
    /// Note: Field is named `transcripts` (plural) at feed level,
    /// but `transcript` (singular) at entry level in PodcastEntryMeta.
    /// This follows the core Rust types and Podcast 2.0 namespace conventions.
    #[getter]
    fn transcripts(&self) -> Vec<PyPodcastTranscript> {
        self.inner
            .transcripts
            .iter()
            .map(|t| PyPodcastTranscript::from_core(t.clone()))
            .collect()
    }

    #[getter]
    fn funding(&self) -> Vec<PyPodcastFunding> {
        self.inner
            .funding
            .iter()
            .map(|f| PyPodcastFunding::from_core(f.clone()))
            .collect()
    }

    #[getter]
    fn persons(&self) -> Vec<PyPodcastPerson> {
        self.inner
            .persons
            .iter()
            .map(|p| PyPodcastPerson::from_core(p.clone()))
            .collect()
    }

    #[getter]
    fn guid(&self) -> Option<&str> {
        self.inner.guid.as_deref()
    }

    #[getter]
    fn medium(&self) -> Option<&str> {
        self.inner.medium.as_deref()
    }

    #[getter]
    fn locked(&self) -> Option<&str> {
        self.inner.locked.as_deref()
    }

    #[getter]
    fn locked_owner(&self) -> Option<&str> {
        self.inner.locked_owner.as_deref()
    }

    #[getter]
    fn value(&self) -> Option<PyPodcastValue> {
        self.inner
            .value
            .as_ref()
            .map(|v| PyPodcastValue::from_core(v.clone()))
    }

    #[getter]
    fn chat(&self) -> Vec<PyPodcastChat> {
        self.inner
            .chat
            .iter()
            .map(|c| PyPodcastChat::from_core(c.clone()))
            .collect()
    }

    #[getter]
    fn podping_uses_podping(&self) -> Option<bool> {
        self.inner.podping_uses_podping
    }

    #[getter]
    fn podroll(&self) -> Vec<PyPodcastRemoteItem> {
        self.inner
            .podroll
            .iter()
            .map(|r| PyPodcastRemoteItem::from_core(r.clone()))
            .collect()
    }

    #[getter]
    fn location(&self) -> Option<PyPodcastLocation> {
        self.inner
            .location
            .as_ref()
            .map(|l| PyPodcastLocation::from_core(l.clone()))
    }

    #[getter]
    fn txt(&self) -> Vec<PyPodcastTxt> {
        self.inner
            .txt
            .iter()
            .map(|t| PyPodcastTxt::from_core(t.clone()))
            .collect()
    }

    #[getter]
    fn update_frequency(&self) -> Option<PyPodcastUpdateFrequency> {
        self.inner
            .update_frequency
            .as_ref()
            .map(|u| PyPodcastUpdateFrequency::from_core(u.clone()))
    }

    #[getter]
    fn follow(&self) -> Vec<PyPodcastFollow> {
        self.inner
            .follow
            .iter()
            .map(|f| PyPodcastFollow::from_core(f.clone()))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "PodcastMeta(guid='{}', persons={}, medium='{}')",
            self.inner.guid.as_deref().unwrap_or("none"),
            self.inner.persons.len(),
            self.inner.medium.as_deref().unwrap_or("none"),
        )
    }
}

#[pyclass(name = "PodcastTranscript", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyPodcastTranscript {
    inner: CorePodcastTranscript,
}

impl PyPodcastTranscript {
    pub fn from_core(core: CorePodcastTranscript) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyPodcastTranscript {
    #[getter]
    fn url(&self) -> &str {
        &self.inner.url
    }

    #[getter]
    #[pyo3(name = "type")]
    fn transcript_type(&self) -> Option<&str> {
        self.inner.transcript_type.as_deref()
    }

    #[getter]
    fn language(&self) -> Option<&str> {
        self.inner.language.as_deref()
    }

    #[getter]
    fn rel(&self) -> Option<&str> {
        self.inner.rel.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "PodcastTranscript(url='{}', type='{}')",
            &self.inner.url,
            self.inner.transcript_type.as_deref().unwrap_or("unknown")
        )
    }
}

#[pyclass(name = "PodcastFunding", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyPodcastFunding {
    inner: CorePodcastFunding,
}

impl PyPodcastFunding {
    pub fn from_core(core: CorePodcastFunding) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyPodcastFunding {
    #[getter]
    fn url(&self) -> &str {
        &self.inner.url
    }

    #[getter]
    fn message(&self) -> Option<&str> {
        self.inner.message.as_deref()
    }

    fn __repr__(&self) -> String {
        format!("PodcastFunding(url='{}')", &self.inner.url)
    }
}

#[pyclass(name = "PodcastPerson", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyPodcastPerson {
    inner: CorePodcastPerson,
}

impl PyPodcastPerson {
    pub fn from_core(core: CorePodcastPerson) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyPodcastPerson {
    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    #[getter]
    fn role(&self) -> Option<&str> {
        self.inner.role.as_deref()
    }

    #[getter]
    fn group(&self) -> Option<&str> {
        self.inner.group.as_deref()
    }

    #[getter]
    fn img(&self) -> Option<&str> {
        self.inner.img.as_deref()
    }

    #[getter]
    fn href(&self) -> Option<&str> {
        self.inner.href.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "PodcastPerson(name='{}', role='{}')",
            &self.inner.name,
            self.inner.role.as_deref().unwrap_or("unknown")
        )
    }

    fn __getitem__(&self, py: Python<'_>, key: &str) -> PyResult<pyo3::Py<pyo3::PyAny>> {
        use pyo3::IntoPyObjectExt;
        match key {
            "name" => self.inner.name.as_str().into_py_any(py),
            "role" => self.inner.role.as_deref().into_py_any(py),
            "group" => self.inner.group.as_deref().into_py_any(py),
            "img" => self.inner.img.as_deref().into_py_any(py),
            "href" => self.inner.href.as_deref().into_py_any(py),
            _ => Err(pyo3::exceptions::PyKeyError::new_err(key.to_string())),
        }
    }

    #[pyo3(signature = (key, default = None))]
    fn get(
        &self,
        py: Python<'_>,
        key: &str,
        default: Option<pyo3::Py<pyo3::PyAny>>,
    ) -> PyResult<pyo3::Py<pyo3::PyAny>> {
        match self.__getitem__(py, key) {
            Ok(v) if v.is_none(py) => Ok(default.unwrap_or_else(|| py.None())),
            Ok(v) => Ok(v),
            Err(_) => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    fn keys(&self) -> Vec<&'static str> {
        vec!["name", "role", "group", "img", "href"]
    }

    fn values(&self, py: Python<'_>) -> PyResult<Vec<pyo3::Py<pyo3::PyAny>>> {
        self.keys()
            .into_iter()
            .map(|key| self.__getitem__(py, key))
            .collect()
    }

    fn items(&self, py: Python<'_>) -> PyResult<Vec<(String, pyo3::Py<pyo3::PyAny>)>> {
        self.keys()
            .into_iter()
            .map(|key| Ok((key.to_string(), self.__getitem__(py, key)?)))
            .collect()
    }
}

#[pyclass(name = "PodcastChapters", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyPodcastChapters {
    inner: CorePodcastChapters,
}

impl PyPodcastChapters {
    pub fn from_core(core: CorePodcastChapters) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyPodcastChapters {
    #[getter]
    fn url(&self) -> &str {
        &self.inner.url
    }

    #[getter]
    #[pyo3(name = "type")]
    fn chapters_type(&self) -> &str {
        &self.inner.type_
    }

    fn __repr__(&self) -> String {
        format!(
            "PodcastChapters(url='{}', type='{}')",
            &self.inner.url, &self.inner.type_
        )
    }
}

#[pyclass(name = "PodcastSoundbite", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyPodcastSoundbite {
    inner: CorePodcastSoundbite,
}

impl PyPodcastSoundbite {
    pub fn from_core(core: CorePodcastSoundbite) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyPodcastSoundbite {
    #[getter]
    fn start_time(&self) -> f64 {
        self.inner.start_time
    }

    #[getter]
    fn duration(&self) -> f64 {
        self.inner.duration
    }

    #[getter]
    fn title(&self) -> Option<&str> {
        self.inner.title.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "PodcastSoundbite(start_time={}, duration={})",
            self.inner.start_time, self.inner.duration
        )
    }
}

#[pyclass(name = "PodcastEntryMeta", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyPodcastEntryMeta {
    inner: CorePodcastEntryMeta,
}

impl PyPodcastEntryMeta {
    pub fn from_core(core: CorePodcastEntryMeta) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyPodcastEntryMeta {
    /// Returns podcast transcripts at entry level.
    ///
    /// Note: Field is named `transcript` (singular) at entry level,
    /// but `transcripts` (plural) at feed level in PodcastMeta.
    /// This follows the core Rust types and Podcast 2.0 namespace conventions.
    #[getter]
    fn transcript(&self) -> Vec<PyPodcastTranscript> {
        self.inner
            .transcript
            .iter()
            .map(|t| PyPodcastTranscript::from_core(t.clone()))
            .collect()
    }

    #[getter]
    fn chapters(&self) -> Option<PyPodcastChapters> {
        self.inner
            .chapters
            .as_ref()
            .map(|c| PyPodcastChapters::from_core(c.clone()))
    }

    #[getter]
    fn soundbite(&self) -> Vec<PyPodcastSoundbite> {
        self.inner
            .soundbite
            .iter()
            .map(|s| PyPodcastSoundbite::from_core(s.clone()))
            .collect()
    }

    #[getter]
    fn persons(&self) -> Vec<PyPodcastPerson> {
        self.inner
            .persons
            .iter()
            .map(|p| PyPodcastPerson::from_core(p.clone()))
            .collect()
    }

    #[getter]
    fn medium(&self) -> Option<&str> {
        self.inner.medium.as_deref()
    }

    #[getter]
    fn season(&self) -> Option<&str> {
        self.inner.season.as_deref()
    }

    #[getter]
    fn episode(&self) -> Option<&str> {
        self.inner.episode.as_deref()
    }

    #[getter]
    fn chat(&self) -> Vec<PyPodcastChat> {
        self.inner
            .chat
            .iter()
            .map(|c| PyPodcastChat::from_core(c.clone()))
            .collect()
    }

    #[getter]
    fn alternate_enclosures(&self) -> Vec<PyPodcastAlternateEnclosure> {
        self.inner
            .alternate_enclosures
            .iter()
            .map(|e| PyPodcastAlternateEnclosure::from_core(e.clone()))
            .collect()
    }

    #[getter]
    fn location(&self) -> Option<PyPodcastLocation> {
        self.inner
            .location
            .as_ref()
            .map(|l| PyPodcastLocation::from_core(l.clone()))
    }

    #[getter]
    fn social_interact(&self) -> Vec<PyPodcastSocialInteract> {
        self.inner
            .social_interact
            .iter()
            .map(|s| PyPodcastSocialInteract::from_core(s.clone()))
            .collect()
    }

    #[getter]
    fn txt(&self) -> Vec<PyPodcastTxt> {
        self.inner
            .txt
            .iter()
            .map(|t| PyPodcastTxt::from_core(t.clone()))
            .collect()
    }

    #[getter]
    fn follow(&self) -> Vec<PyPodcastFollow> {
        self.inner
            .follow
            .iter()
            .map(|f| PyPodcastFollow::from_core(f.clone()))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "PodcastEntryMeta(transcripts={}, chapters={}, soundbites={}, persons={}, season={}, episode={})",
            self.inner.transcript.len(),
            if self.inner.chapters.is_some() {
                "present"
            } else {
                "none"
            },
            self.inner.soundbite.len(),
            self.inner.persons.len(),
            self.inner.season.as_deref().unwrap_or("none"),
            self.inner.episode.as_deref().unwrap_or("none"),
        )
    }
}

#[pyclass(name = "PodcastValue", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyPodcastValue {
    inner: CorePodcastValue,
}

impl PyPodcastValue {
    pub fn from_core(core: CorePodcastValue) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyPodcastValue {
    #[getter]
    #[pyo3(name = "type")]
    fn type_(&self) -> &str {
        &self.inner.type_
    }

    #[getter]
    fn method(&self) -> &str {
        &self.inner.method
    }

    #[getter]
    fn suggested(&self) -> Option<&str> {
        self.inner.suggested.as_deref()
    }

    #[getter]
    fn recipients(&self) -> Vec<PyPodcastValueRecipient> {
        self.inner
            .recipients
            .iter()
            .map(|r| PyPodcastValueRecipient::from_core(r.clone()))
            .collect()
    }

    #[getter]
    fn time_splits(&self) -> Vec<PyPodcastValueTimeSplit> {
        self.inner
            .time_splits
            .iter()
            .map(|s| PyPodcastValueTimeSplit::from_core(s.clone()))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "PodcastValue(type='{}', method='{}', recipients={})",
            &self.inner.type_,
            &self.inner.method,
            self.inner.recipients.len()
        )
    }
}

#[pyclass(
    name = "PodcastValueRecipient",
    module = "feedparser_rs",
    from_py_object
)]
#[derive(Clone)]
pub struct PyPodcastValueRecipient {
    inner: CorePodcastValueRecipient,
}

impl PyPodcastValueRecipient {
    pub fn from_core(core: CorePodcastValueRecipient) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyPodcastValueRecipient {
    #[getter]
    fn name(&self) -> Option<&str> {
        self.inner.name.as_deref()
    }

    #[getter]
    #[pyo3(name = "type")]
    fn type_(&self) -> &str {
        &self.inner.type_
    }

    #[getter]
    fn address(&self) -> &str {
        &self.inner.address
    }

    #[getter]
    fn split(&self) -> u32 {
        self.inner.split
    }

    #[getter]
    fn fee(&self) -> Option<bool> {
        self.inner.fee
    }

    fn __repr__(&self) -> String {
        format!(
            "PodcastValueRecipient(name='{}', split={})",
            self.inner.name.as_deref().unwrap_or("unknown"),
            self.inner.split
        )
    }
}

#[pyclass(
    name = "PodcastValueTimeSplit",
    module = "feedparser_rs",
    from_py_object
)]
#[derive(Clone)]
pub struct PyPodcastValueTimeSplit {
    inner: CorePodcastValueTimeSplit,
}

impl PyPodcastValueTimeSplit {
    pub fn from_core(core: CorePodcastValueTimeSplit) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyPodcastValueTimeSplit {
    #[getter]
    fn start_time(&self) -> f64 {
        self.inner.start_time
    }

    #[getter]
    fn duration(&self) -> f64 {
        self.inner.duration
    }

    #[getter]
    fn remote_start_time(&self) -> f64 {
        self.inner.remote_start_time
    }

    #[getter]
    fn remote_percentage(&self) -> f64 {
        self.inner.remote_percentage
    }

    #[getter]
    fn recipients(&self) -> Vec<PyPodcastValueRecipient> {
        self.inner
            .recipients
            .iter()
            .map(|r| PyPodcastValueRecipient::from_core(r.clone()))
            .collect()
    }

    #[getter]
    fn remote_item(&self) -> Option<PyPodcastRemoteItem> {
        self.inner
            .remote_item
            .as_ref()
            .map(|r| PyPodcastRemoteItem::from_core(r.clone()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PodcastValueTimeSplit(start_time={}, duration={}, remote_percentage={})",
            self.inner.start_time, self.inner.duration, self.inner.remote_percentage
        )
    }
}

#[pyclass(name = "PodcastRemoteItem", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyPodcastRemoteItem {
    inner: CorePodcastRemoteItem,
}

impl PyPodcastRemoteItem {
    pub fn from_core(core: CorePodcastRemoteItem) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyPodcastRemoteItem {
    #[getter]
    fn feed_guid(&self) -> Option<&str> {
        self.inner.feed_guid.as_deref()
    }

    #[getter]
    fn feed_url(&self) -> Option<&str> {
        self.inner.feed_url.as_deref()
    }

    #[getter]
    fn item_guid(&self) -> Option<&str> {
        self.inner.item_guid.as_deref()
    }

    #[getter]
    fn medium(&self) -> Option<&str> {
        self.inner.medium.as_deref()
    }

    #[getter]
    fn title(&self) -> Option<&str> {
        self.inner.title.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "PodcastRemoteItem(feed_guid='{}', title='{}')",
            self.inner.feed_guid.as_deref().unwrap_or("none"),
            self.inner.title.as_deref().unwrap_or("none")
        )
    }
}

#[pyclass(name = "PodcastChat", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyPodcastChat {
    inner: CorePodcastChat,
}

impl PyPodcastChat {
    pub fn from_core(core: CorePodcastChat) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyPodcastChat {
    #[getter]
    fn server(&self) -> &str {
        &self.inner.server
    }

    #[getter]
    fn protocol(&self) -> &str {
        &self.inner.protocol
    }

    #[getter]
    fn account_id(&self) -> Option<&str> {
        self.inner.account_id.as_deref()
    }

    #[getter]
    fn space(&self) -> Option<&str> {
        self.inner.space.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "PodcastChat(server='{}', protocol='{}')",
            self.inner.server, self.inner.protocol
        )
    }
}

#[pyclass(name = "PodcastLocation", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyPodcastLocation {
    inner: CorePodcastLocation,
}

impl PyPodcastLocation {
    pub fn from_core(core: CorePodcastLocation) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyPodcastLocation {
    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    #[getter]
    fn geo(&self) -> Option<&str> {
        self.inner.geo.as_deref()
    }

    #[getter]
    fn osm(&self) -> Option<&str> {
        self.inner.osm.as_deref()
    }

    fn __repr__(&self) -> String {
        format!("PodcastLocation(name='{}')", &self.inner.name)
    }
}

#[pyclass(name = "PodcastTxt", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyPodcastTxt {
    inner: CorePodcastTxt,
}

impl PyPodcastTxt {
    pub fn from_core(core: CorePodcastTxt) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyPodcastTxt {
    #[getter]
    fn purpose(&self) -> Option<&str> {
        self.inner.purpose.as_deref()
    }

    #[getter]
    fn value(&self) -> &str {
        &self.inner.value
    }

    fn __repr__(&self) -> String {
        format!(
            "PodcastTxt(purpose='{}', value='{}')",
            self.inner.purpose.as_deref().unwrap_or("none"),
            &self.inner.value
        )
    }
}

#[pyclass(
    name = "PodcastUpdateFrequency",
    module = "feedparser_rs",
    from_py_object
)]
#[derive(Clone)]
pub struct PyPodcastUpdateFrequency {
    inner: CorePodcastUpdateFrequency,
}

impl PyPodcastUpdateFrequency {
    pub fn from_core(core: CorePodcastUpdateFrequency) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyPodcastUpdateFrequency {
    #[getter]
    fn rrule(&self) -> Option<&str> {
        self.inner.rrule.as_deref()
    }

    #[getter]
    fn complete(&self) -> Option<bool> {
        self.inner.complete
    }

    #[getter]
    fn dtstart(&self) -> Option<&str> {
        self.inner.dtstart.as_deref()
    }

    #[getter]
    fn label(&self) -> Option<&str> {
        self.inner.label.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "PodcastUpdateFrequency(rrule='{}')",
            self.inner.rrule.as_deref().unwrap_or("none")
        )
    }
}

#[pyclass(name = "PodcastFollow", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyPodcastFollow {
    inner: CorePodcastFollow,
}

impl PyPodcastFollow {
    pub fn from_core(core: CorePodcastFollow) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyPodcastFollow {
    #[getter]
    fn url(&self) -> &str {
        &self.inner.url
    }

    #[getter]
    fn platform(&self) -> Option<&str> {
        self.inner.platform.as_deref()
    }

    fn __repr__(&self) -> String {
        format!("PodcastFollow(url='{}')", &self.inner.url)
    }
}

#[pyclass(
    name = "PodcastSocialInteract",
    module = "feedparser_rs",
    from_py_object
)]
#[derive(Clone)]
pub struct PyPodcastSocialInteract {
    inner: CorePodcastSocialInteract,
}

impl PyPodcastSocialInteract {
    pub fn from_core(core: CorePodcastSocialInteract) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyPodcastSocialInteract {
    #[getter]
    fn uri(&self) -> &str {
        &self.inner.uri
    }

    #[getter]
    fn protocol(&self) -> Option<&str> {
        self.inner.protocol.as_deref()
    }

    #[getter]
    fn account_id(&self) -> Option<&str> {
        self.inner.account_id.as_deref()
    }

    #[getter]
    fn account_url(&self) -> Option<&str> {
        self.inner.account_url.as_deref()
    }

    #[getter]
    fn priority(&self) -> Option<u32> {
        self.inner.priority
    }

    fn __repr__(&self) -> String {
        format!("PodcastSocialInteract(uri='{}')", &self.inner.uri)
    }
}

#[pyclass(
    name = "PodcastAlternateEnclosure",
    module = "feedparser_rs",
    from_py_object
)]
#[derive(Clone)]
pub struct PyPodcastAlternateEnclosure {
    inner: CorePodcastAlternateEnclosure,
}

impl PyPodcastAlternateEnclosure {
    pub fn from_core(core: CorePodcastAlternateEnclosure) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyPodcastAlternateEnclosure {
    #[getter]
    #[pyo3(name = "type")]
    fn type_(&self) -> &str {
        &self.inner.type_
    }

    #[getter]
    fn length(&self) -> Option<u64> {
        self.inner.length
    }

    #[getter]
    fn bitrate(&self) -> Option<f64> {
        self.inner.bitrate
    }

    #[getter]
    fn height(&self) -> Option<u32> {
        self.inner.height
    }

    #[getter]
    fn lang(&self) -> Option<&str> {
        self.inner.lang.as_deref()
    }

    #[getter]
    fn title(&self) -> Option<&str> {
        self.inner.title.as_deref()
    }

    #[getter]
    fn rel(&self) -> Option<&str> {
        self.inner.rel.as_deref()
    }

    #[getter]
    fn codecs(&self) -> Option<&str> {
        self.inner.codecs.as_deref()
    }

    #[getter]
    #[pyo3(name = "default")]
    fn is_default(&self) -> Option<bool> {
        self.inner.default
    }

    #[getter]
    fn sources(&self) -> Vec<PyPodcastAlternateEnclosureSource> {
        self.inner
            .sources
            .iter()
            .map(|s| PyPodcastAlternateEnclosureSource::from_core(s.clone()))
            .collect()
    }

    #[getter]
    fn integrity(&self) -> Option<PyPodcastIntegrity> {
        self.inner
            .integrity
            .as_ref()
            .map(|i| PyPodcastIntegrity::from_core(i.clone()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PodcastAlternateEnclosure(type='{}', sources={})",
            &self.inner.type_,
            self.inner.sources.len()
        )
    }
}

#[pyclass(
    name = "PodcastAlternateEnclosureSource",
    module = "feedparser_rs",
    from_py_object
)]
#[derive(Clone)]
pub struct PyPodcastAlternateEnclosureSource {
    inner: CorePodcastAlternateEnclosureSource,
}

impl PyPodcastAlternateEnclosureSource {
    pub fn from_core(core: CorePodcastAlternateEnclosureSource) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyPodcastAlternateEnclosureSource {
    #[getter]
    fn uri(&self) -> &str {
        &self.inner.uri
    }

    #[getter]
    fn content_type(&self) -> Option<&str> {
        self.inner.content_type.as_deref()
    }

    fn __repr__(&self) -> String {
        format!("PodcastAlternateEnclosureSource(uri='{}')", &self.inner.uri)
    }
}

#[pyclass(name = "PodcastIntegrity", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyPodcastIntegrity {
    inner: CorePodcastIntegrity,
}

impl PyPodcastIntegrity {
    pub fn from_core(core: CorePodcastIntegrity) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyPodcastIntegrity {
    #[getter]
    #[pyo3(name = "type")]
    fn type_(&self) -> &str {
        &self.inner.type_
    }

    #[getter]
    fn value(&self) -> &str {
        &self.inner.value
    }

    fn __repr__(&self) -> String {
        format!("PodcastIntegrity(type='{}')", &self.inner.type_)
    }
}
