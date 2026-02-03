use feedparser_rs::{
    ItunesCategory as CoreItunesCategory, ItunesEntryMeta as CoreItunesEntryMeta,
    ItunesFeedMeta as CoreItunesFeedMeta, ItunesOwner as CoreItunesOwner,
    PodcastChapters as CorePodcastChapters, PodcastEntryMeta as CorePodcastEntryMeta,
    PodcastFunding as CorePodcastFunding, PodcastMeta as CorePodcastMeta,
    PodcastPerson as CorePodcastPerson, PodcastSoundbite as CorePodcastSoundbite,
    PodcastTranscript as CorePodcastTranscript,
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

    fn __repr__(&self) -> String {
        format!(
            "ItunesFeedMeta(author='{}', categories={})",
            self.inner.author.as_deref().unwrap_or("unknown"),
            self.inner.categories.len()
        )
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
    fn duration(&self) -> Option<u32> {
        self.inner.duration
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
    fn episode(&self) -> Option<u32> {
        self.inner.episode
    }

    #[getter]
    fn season(&self) -> Option<u32> {
        self.inner.season
    }

    #[getter]
    fn episode_type(&self) -> Option<&str> {
        self.inner.episode_type.as_deref()
    }

    fn __repr__(&self) -> String {
        if let (Some(season), Some(episode)) = (self.inner.season, self.inner.episode) {
            format!("ItunesEntryMeta(season={}, episode={})", season, episode)
        } else {
            "ItunesEntryMeta()".to_string()
        }
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

    fn __repr__(&self) -> String {
        format!(
            "PodcastMeta(guid='{}', persons={})",
            self.inner.guid.as_deref().unwrap_or("none"),
            self.inner.persons.len()
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
    fn person(&self) -> Vec<PyPodcastPerson> {
        self.inner
            .person
            .iter()
            .map(|p| PyPodcastPerson::from_core(p.clone()))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "PodcastEntryMeta(transcripts={}, chapters={}, soundbites={}, persons={})",
            self.inner.transcript.len(),
            if self.inner.chapters.is_some() {
                "present"
            } else {
                "none"
            },
            self.inner.soundbite.len(),
            self.inner.person.len()
        )
    }
}
