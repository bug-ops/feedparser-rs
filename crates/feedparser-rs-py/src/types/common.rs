use super::datetime::optional_datetime_to_struct_time;
use feedparser_rs::{
    Cloud as CoreCloud, Content as CoreContent, Enclosure as CoreEnclosure,
    Generator as CoreGenerator, Image as CoreImage, Link as CoreLink, Person as CorePerson,
    Source as CoreSource, Tag as CoreTag, TextConstruct as CoreTextConstruct,
    TextInput as CoreTextInput, TextType,
};
use pyo3::prelude::*;
use pyo3::types::PyList;

#[pyclass(name = "TextConstruct", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyTextConstruct {
    inner: CoreTextConstruct,
}

impl PyTextConstruct {
    pub fn from_core(core: CoreTextConstruct) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyTextConstruct {
    #[getter]
    fn value(&self) -> &str {
        &self.inner.value
    }

    #[getter]
    #[pyo3(name = "type")]
    fn content_type(&self) -> &str {
        match self.inner.content_type {
            TextType::Text => "text/plain",
            TextType::Html => "text/html",
            TextType::Xhtml => "application/xhtml+xml",
        }
    }

    #[getter]
    fn language(&self) -> Option<&str> {
        self.inner.language.as_deref()
    }

    #[getter]
    fn base(&self) -> Option<&str> {
        self.inner.base.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "TextConstruct(type='{}', value='{}')",
            self.content_type(),
            &self.inner.value.chars().take(50).collect::<String>()
        )
    }

    fn __getitem__(&self, key: &str) -> PyResult<Option<String>> {
        match key {
            "value" => Ok(Some(self.inner.value.clone())),
            "type" => Ok(Some(self.content_type().to_owned())),
            "language" => Ok(self.inner.language.as_deref().map(str::to_owned)),
            "base" => Ok(self.inner.base.as_deref().map(str::to_owned)),
            _ => Err(pyo3::exceptions::PyKeyError::new_err(key.to_string())),
        }
    }

    fn __contains__(&self, key: &str) -> bool {
        matches!(key, "value" | "type" | "language" | "base")
    }

    #[pyo3(signature = (key, default = None))]
    fn get(&self, key: &str, default: Option<String>) -> Option<String> {
        match self.__getitem__(key) {
            Ok(v) => v.or(default),
            Err(_) => default,
        }
    }

    fn keys(&self) -> Vec<&'static str> {
        vec!["value", "type", "language", "base"]
    }

    fn values(&self) -> Vec<Option<String>> {
        vec![
            Some(self.inner.value.clone()),
            Some(self.content_type().to_owned()),
            self.inner.language.as_deref().map(str::to_owned),
            self.inner.base.as_deref().map(str::to_owned),
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

#[pyclass(name = "Link", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyLink {
    inner: CoreLink,
}

impl PyLink {
    pub fn from_core(core: CoreLink) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyLink {
    #[getter]
    fn href(&self) -> &str {
        &self.inner.href
    }

    #[getter]
    fn rel(&self) -> Option<&str> {
        self.inner.rel.as_deref()
    }

    #[getter]
    #[pyo3(name = "type")]
    fn link_type(&self) -> Option<&str> {
        self.inner.link_type.as_deref()
    }

    #[getter]
    fn title(&self) -> Option<&str> {
        self.inner.title.as_deref()
    }

    #[getter]
    fn length(&self) -> Option<&str> {
        self.inner.length.as_deref()
    }

    #[getter]
    fn hreflang(&self) -> Option<&str> {
        self.inner.hreflang.as_deref()
    }

    #[getter]
    fn thr_count(&self) -> Option<u32> {
        self.inner.thr_count
    }

    #[getter]
    fn thr_updated(&self) -> Option<String> {
        self.inner.thr_updated.map(|dt| dt.to_rfc3339())
    }

    #[getter]
    fn thr_updated_parsed(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        optional_datetime_to_struct_time(py, &self.inner.thr_updated)
    }

    fn __repr__(&self) -> String {
        format!(
            "Link(href='{}', rel='{}')",
            &self.inner.href,
            self.inner.rel.as_deref().unwrap_or("alternate")
        )
    }

    fn __getitem__(&self, key: &str) -> PyResult<Option<String>> {
        match key {
            "href" => Ok(Some(self.inner.href.to_string())),
            "rel" => Ok(self.inner.rel.as_deref().map(str::to_owned)),
            "type" => Ok(self.inner.link_type.as_deref().map(str::to_owned)),
            "title" => Ok(self.inner.title.as_deref().map(str::to_owned)),
            "hreflang" => Ok(self.inner.hreflang.as_deref().map(str::to_owned)),
            _ => Err(pyo3::exceptions::PyKeyError::new_err(key.to_string())),
        }
    }

    fn __contains__(&self, key: &str) -> bool {
        matches!(key, "href" | "rel" | "type" | "title" | "hreflang")
    }

    #[pyo3(signature = (key, default = None))]
    fn get(&self, key: &str, default: Option<String>) -> Option<String> {
        match self.__getitem__(key) {
            Ok(v) => v.or(default),
            Err(_) => default,
        }
    }

    fn keys(&self) -> Vec<&'static str> {
        vec!["href", "rel", "type", "title", "hreflang"]
    }

    fn values(&self) -> Vec<Option<String>> {
        vec![
            Some(self.inner.href.to_string()),
            self.inner.rel.as_deref().map(str::to_owned),
            self.inner.link_type.as_deref().map(str::to_owned),
            self.inner.title.as_deref().map(str::to_owned),
            self.inner.hreflang.as_deref().map(str::to_owned),
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

#[pyclass(name = "Person", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyPerson {
    inner: CorePerson,
}

impl PyPerson {
    pub fn from_core(core: CorePerson) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyPerson {
    #[getter]
    fn name(&self) -> Option<&str> {
        self.inner.name.as_deref()
    }

    #[getter]
    fn email(&self) -> Option<&str> {
        self.inner.email.as_deref()
    }

    #[getter]
    fn href(&self) -> Option<&str> {
        self.inner.uri.as_deref()
    }

    #[getter]
    fn avatar(&self) -> Option<&str> {
        self.inner.avatar.as_deref()
    }

    fn __getitem__(&self, key: &str) -> PyResult<Option<String>> {
        match key {
            "name" => Ok(self.inner.name.as_deref().map(str::to_owned)),
            "email" => Ok(self.inner.email.as_deref().map(str::to_owned)),
            "href" => Ok(self.inner.uri.as_deref().map(str::to_owned)),
            "avatar" => Ok(self.inner.avatar.clone()),
            _ => Err(pyo3::exceptions::PyKeyError::new_err(key.to_string())),
        }
    }

    fn __contains__(&self, key: &str) -> bool {
        matches!(key, "name" | "email" | "href" | "avatar")
    }

    #[pyo3(signature = (key, default = None))]
    fn get(&self, key: &str, default: Option<String>) -> Option<String> {
        match self.__getitem__(key) {
            Ok(v) => v.or(default),
            Err(_) => default,
        }
    }

    fn keys(&self) -> Vec<&'static str> {
        vec!["name", "email", "href", "avatar"]
    }

    fn values(&self) -> Vec<Option<String>> {
        vec![
            self.inner.name.as_deref().map(str::to_owned),
            self.inner.email.as_deref().map(str::to_owned),
            self.inner.uri.as_deref().map(str::to_owned),
            self.inner.avatar.clone(),
        ]
    }

    fn items(&self) -> Vec<(String, Option<String>)> {
        self.keys()
            .into_iter()
            .zip(self.values())
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }

    fn __repr__(&self) -> String {
        if let Some(name) = &self.inner.name {
            format!("Person(name='{}')", name)
        } else if let Some(email) = &self.inner.email {
            format!("Person(email='{}')", email)
        } else {
            "Person()".to_string()
        }
    }
}

#[pyclass(name = "Tag", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyTag {
    inner: CoreTag,
}

impl PyTag {
    pub fn from_core(core: CoreTag) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyTag {
    #[getter]
    fn term(&self) -> &str {
        &self.inner.term
    }

    #[getter]
    fn scheme(&self) -> Option<&str> {
        self.inner.scheme.as_deref()
    }

    #[getter]
    fn label(&self) -> Option<&str> {
        self.inner.label.as_deref()
    }

    fn __repr__(&self) -> String {
        format!("Tag(term='{}')", &self.inner.term)
    }

    fn __getitem__(&self, key: &str) -> PyResult<Option<String>> {
        match key {
            "term" => Ok(Some(self.inner.term.to_string())),
            "scheme" => Ok(self.inner.scheme.as_deref().map(str::to_owned)),
            "label" => Ok(self.inner.label.as_deref().map(str::to_owned)),
            _ => Err(pyo3::exceptions::PyKeyError::new_err(key.to_string())),
        }
    }

    fn __contains__(&self, key: &str) -> bool {
        matches!(key, "term" | "scheme" | "label")
    }

    #[pyo3(signature = (key, default = None))]
    fn get(&self, key: &str, default: Option<String>) -> Option<String> {
        match self.__getitem__(key) {
            Ok(v) => v.or(default),
            Err(_) => default,
        }
    }

    fn keys(&self) -> Vec<&'static str> {
        vec!["term", "scheme", "label"]
    }

    fn values(&self) -> Vec<Option<String>> {
        vec![
            Some(self.inner.term.to_string()),
            self.inner.scheme.as_deref().map(str::to_owned),
            self.inner.label.as_deref().map(str::to_owned),
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

#[pyclass(name = "Image", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyImage {
    inner: CoreImage,
}

impl PyImage {
    pub fn from_core(core: CoreImage) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyImage {
    #[getter]
    fn href(&self) -> &str {
        &self.inner.url
    }

    #[getter]
    fn title(&self) -> Option<&str> {
        self.inner.title.as_deref()
    }

    #[getter]
    fn link(&self) -> Option<&str> {
        self.inner.link.as_deref()
    }

    #[getter]
    fn width(&self) -> Option<u32> {
        self.inner.width
    }

    #[getter]
    fn height(&self) -> Option<u32> {
        self.inner.height
    }

    #[getter]
    fn url(&self) -> &str {
        &self.inner.url
    }

    #[getter]
    fn description(&self) -> Option<&str> {
        self.inner.description.as_deref()
    }

    #[getter]
    fn subtitle(&self) -> Option<&str> {
        self.inner.description.as_deref()
    }

    #[getter]
    fn subtitle_detail(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        let Some(desc) = self.inner.description.as_deref() else {
            return Ok(None);
        };
        let tc = PyTextConstruct::from_core(CoreTextConstruct::text(desc));
        Ok(Some(tc.into_pyobject(py)?.into_any().unbind()))
    }

    #[getter]
    fn title_detail(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        let Some(title) = self.inner.title.as_deref() else {
            return Ok(None);
        };
        let tc = PyTextConstruct::from_core(CoreTextConstruct::text(title));
        Ok(Some(tc.into_pyobject(py)?.into_any().unbind()))
    }

    #[getter]
    fn links(&self, py: Python<'_>) -> PyResult<Py<PyList>> {
        use feedparser_rs::Link as CoreLink;
        let link = CoreLink::alternate(self.inner.url.clone());
        let py_link = PyLink::from_core(link);
        let list = PyList::new(py, [py_link])?;
        Ok(list.unbind())
    }

    fn __repr__(&self) -> String {
        format!("Image(href='{}')", &self.inner.url)
    }

    fn __getitem__(&self, key: &str) -> PyResult<Option<String>> {
        match key {
            "href" | "url" => Ok(Some(self.inner.url.to_string())),
            "title" => Ok(self.inner.title.as_deref().map(str::to_owned)),
            "link" => Ok(self.inner.link.as_deref().map(str::to_owned)),
            "description" | "subtitle" => Ok(self.inner.description.as_deref().map(str::to_owned)),
            "width" => Ok(self.inner.width.map(|v| v.to_string())),
            "height" => Ok(self.inner.height.map(|v| v.to_string())),
            _ => Err(pyo3::exceptions::PyKeyError::new_err(key.to_string())),
        }
    }

    fn __contains__(&self, key: &str) -> bool {
        matches!(
            key,
            "href" | "title" | "link" | "description" | "width" | "height"
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
        vec!["href", "title", "link", "description", "width", "height"]
    }

    fn values(&self) -> Vec<Option<String>> {
        vec![
            Some(self.inner.url.to_string()),
            self.inner.title.as_deref().map(str::to_owned),
            self.inner.link.as_deref().map(str::to_owned),
            self.inner.description.as_deref().map(str::to_owned),
            self.inner.width.map(|v| v.to_string()),
            self.inner.height.map(|v| v.to_string()),
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

#[pyclass(name = "Enclosure", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyEnclosure {
    inner: CoreEnclosure,
}

impl PyEnclosure {
    pub fn from_core(core: CoreEnclosure) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyEnclosure {
    #[getter]
    fn href(&self) -> &str {
        &self.inner.url
    }

    #[getter]
    fn length(&self) -> Option<&str> {
        self.inner.length.as_deref()
    }

    #[getter]
    #[pyo3(name = "type")]
    fn enclosure_type(&self) -> Option<&str> {
        self.inner.enclosure_type.as_deref()
    }

    #[getter]
    fn title(&self) -> Option<&str> {
        self.inner.title.as_deref()
    }

    #[getter]
    fn duration(&self) -> Option<&str> {
        self.inner.duration.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "Enclosure(href='{}', type='{}')",
            &self.inner.url,
            self.inner.enclosure_type.as_deref().unwrap_or("unknown")
        )
    }

    fn __getitem__(&self, key: &str) -> PyResult<Option<String>> {
        match key {
            "href" => Ok(Some(self.inner.url.to_string())),
            "type" => Ok(self.inner.enclosure_type.as_deref().map(str::to_owned)),
            "length" => Ok(self.inner.length.clone()),
            "title" => Ok(self.inner.title.clone()),
            "duration" => Ok(self.inner.duration.clone()),
            _ => Err(pyo3::exceptions::PyKeyError::new_err(key.to_string())),
        }
    }

    fn __contains__(&self, key: &str) -> bool {
        matches!(key, "href" | "type" | "length" | "title" | "duration")
    }

    #[pyo3(signature = (key, default = None))]
    fn get(&self, key: &str, default: Option<String>) -> Option<String> {
        match self.__getitem__(key) {
            Ok(v) => v.or(default),
            Err(_) => default,
        }
    }

    fn keys(&self) -> Vec<&'static str> {
        vec!["href", "type", "length", "title", "duration"]
    }

    fn values(&self) -> Vec<Option<String>> {
        vec![
            Some(self.inner.url.to_string()),
            self.inner.enclosure_type.as_deref().map(str::to_owned),
            self.inner.length.clone(),
            self.inner.title.clone(),
            self.inner.duration.clone(),
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

#[pyclass(name = "Content", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyContent {
    inner: CoreContent,
}

impl PyContent {
    pub fn from_core(core: CoreContent) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyContent {
    #[getter]
    fn value(&self) -> &str {
        &self.inner.value
    }

    #[getter]
    #[pyo3(name = "type")]
    fn content_type(&self) -> Option<&str> {
        self.inner.content_type.as_deref()
    }

    #[getter]
    fn language(&self) -> Option<&str> {
        self.inner.language.as_deref()
    }

    #[getter]
    fn base(&self) -> Option<&str> {
        self.inner.base.as_deref()
    }

    /// Out-of-line content URL (Atom `<content src="...">`, RFC 4287 §4.1.3.2)
    #[getter]
    fn src(&self) -> Option<&str> {
        self.inner.src.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "Content(type='{}', value='{}')",
            self.inner.content_type.as_deref().unwrap_or("text/plain"),
            &self.inner.value.chars().take(50).collect::<String>()
        )
    }

    fn __getitem__(&self, key: &str) -> PyResult<Option<String>> {
        match key {
            "value" => Ok(Some(self.inner.value.clone())),
            "type" => Ok(self.inner.content_type.as_deref().map(str::to_owned)),
            "language" => Ok(self.inner.language.as_deref().map(str::to_owned)),
            "base" => Ok(self.inner.base.as_deref().map(str::to_owned)),
            "src" => Ok(self.inner.src.as_deref().map(str::to_owned)),
            _ => Err(pyo3::exceptions::PyKeyError::new_err(key.to_string())),
        }
    }

    fn __contains__(&self, key: &str) -> bool {
        matches!(key, "value" | "type" | "language" | "base")
    }

    #[pyo3(signature = (key, default = None))]
    fn get(&self, key: &str, default: Option<String>) -> Option<String> {
        match self.__getitem__(key) {
            Ok(v) => v.or(default),
            Err(_) => default,
        }
    }

    fn keys(&self) -> Vec<&'static str> {
        vec!["value", "type", "language", "base"]
    }

    fn values(&self) -> Vec<Option<String>> {
        vec![
            Some(self.inner.value.clone()),
            self.inner.content_type.as_deref().map(str::to_owned),
            self.inner.language.as_deref().map(str::to_owned),
            self.inner.base.as_deref().map(str::to_owned),
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

#[pyclass(name = "Generator", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyGenerator {
    inner: CoreGenerator,
}

impl PyGenerator {
    pub fn from_core(core: CoreGenerator) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyGenerator {
    /// Generator name (text content of the `<generator>` element)
    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    /// Generator URI (feedparser compatibility alias for `name`)
    #[getter]
    fn value(&self) -> &str {
        &self.inner.name
    }

    /// Generator URI (`href` attribute, matching Python feedparser API)
    #[getter]
    fn href(&self) -> Option<&str> {
        self.inner.href.as_deref()
    }

    #[getter]
    fn version(&self) -> Option<&str> {
        self.inner.version.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "Generator(name='{}', version='{}')",
            &self.inner.name,
            self.inner.version.as_deref().unwrap_or("unknown")
        )
    }

    fn __getitem__(&self, key: &str) -> PyResult<Option<String>> {
        match key {
            "name" => Ok(Some(self.inner.name.clone())),
            "href" => Ok(self.inner.href.as_deref().map(str::to_owned)),
            "version" => Ok(self.inner.version.as_deref().map(str::to_owned)),
            _ => Err(pyo3::exceptions::PyKeyError::new_err(key.to_string())),
        }
    }

    fn __contains__(&self, key: &str) -> bool {
        matches!(key, "name" | "href" | "version")
    }

    #[pyo3(signature = (key, default = None))]
    fn get(&self, key: &str, default: Option<String>) -> Option<String> {
        match self.__getitem__(key) {
            Ok(v) => v.or(default),
            Err(_) => default,
        }
    }

    fn keys(&self) -> Vec<&'static str> {
        vec!["name", "href", "version"]
    }

    fn values(&self) -> Vec<Option<String>> {
        vec![
            Some(self.inner.name.clone()),
            self.inner.href.as_deref().map(str::to_owned),
            self.inner.version.as_deref().map(str::to_owned),
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

#[pyclass(name = "Source", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PySource {
    inner: CoreSource,
}

impl PySource {
    pub fn from_core(core: CoreSource) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PySource {
    #[getter]
    fn title(&self) -> Option<&str> {
        self.inner.title.as_deref()
    }

    /// Primary source URL (Python feedparser API name).
    #[getter]
    fn href(&self) -> Option<&str> {
        self.inner.href.as_deref()
    }

    /// Primary source URL: Atom `<source><link href="..."/>` or RSS `<source url="...">` fallback.
    #[getter]
    fn link(&self) -> Option<&str> {
        self.inner.link.as_deref().or(self.inner.href.as_deref())
    }

    /// Source author flat string (Atom `<source><author>`)
    #[getter]
    fn author(&self) -> Option<&str> {
        self.inner.author.as_deref()
    }

    #[getter]
    fn id(&self) -> Option<&str> {
        self.inner.id.as_deref()
    }

    #[getter]
    fn links(&self) -> Vec<PyLink> {
        self.inner
            .links
            .iter()
            .map(|l| PyLink::from_core(l.clone()))
            .collect()
    }

    /// Raw updated date string (timezone preserved).
    #[getter]
    fn updated(&self) -> Option<&str> {
        self.inner.updated_str.as_deref()
    }

    /// Parsed updated date as `time.struct_time`.
    #[getter]
    fn updated_parsed(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        optional_datetime_to_struct_time(py, &self.inner.updated)
    }

    #[getter]
    fn rights(&self) -> Option<&str> {
        self.inner.rights.as_deref()
    }

    #[getter]
    fn guidislink(&self) -> Option<bool> {
        self.inner.guidislink
    }

    fn __repr__(&self) -> String {
        if let Some(title) = &self.inner.title {
            format!("Source(title='{}')", title)
        } else {
            "Source()".to_string()
        }
    }

    fn __contains__(&self, key: &str) -> bool {
        matches!(
            key,
            "title"
                | "href"
                | "link"
                | "id"
                | "links"
                | "updated"
                | "updated_parsed"
                | "rights"
                | "guidislink"
        )
    }

    /// Returns the value for `key` if present, otherwise returns `default` (None if omitted).
    #[pyo3(signature = (key, default = None))]
    fn get(&self, py: Python<'_>, key: &str, default: Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
        match self.__getitem__(py, key) {
            Ok(v) if v.is_none(py) => Ok(default.unwrap_or_else(|| py.None())),
            Ok(v) => Ok(v),
            Err(_) => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    fn keys(&self) -> Vec<&'static str> {
        vec![
            "title",
            "href",
            "link",
            "id",
            "links",
            "updated",
            "updated_parsed",
            "rights",
            "guidislink",
        ]
    }

    fn values(&self, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
        self.keys()
            .into_iter()
            .map(|key| self.__getitem__(py, key))
            .collect()
    }

    fn items(&self, py: Python<'_>) -> PyResult<Vec<(String, Py<PyAny>)>> {
        self.keys()
            .into_iter()
            .map(|key| Ok((key.to_string(), self.__getitem__(py, key)?)))
            .collect()
    }

    fn __getitem__(&self, py: Python<'_>, key: &str) -> PyResult<Py<PyAny>> {
        match key {
            "title" => Ok(self
                .inner
                .title
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
            "link" => Ok(self
                .inner
                .link
                .as_deref()
                .or(self.inner.href.as_deref())
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
            "id" => Ok(self
                .inner
                .id
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "links" => {
                let py_links: Vec<PyLink> = self
                    .inner
                    .links
                    .iter()
                    .map(|l| PyLink::from_core(l.clone()))
                    .collect();
                Ok(py_links.into_pyobject(py)?.into_any().unbind())
            }
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
            "rights" => Ok(self
                .inner
                .rights
                .as_deref()
                .into_pyobject(py)?
                .into_any()
                .unbind()),
            "guidislink" => Ok(self.inner.guidislink.into_pyobject(py)?.into_any().unbind()),
            _ => Err(pyo3::exceptions::PyKeyError::new_err(key.to_string())),
        }
    }
}

#[pyclass(name = "Cloud", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyCloud {
    inner: CoreCloud,
}

impl PyCloud {
    pub fn from_core(core: CoreCloud) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyCloud {
    #[getter]
    fn domain(&self) -> Option<&str> {
        self.inner.domain.as_deref()
    }

    #[getter]
    fn port(&self) -> Option<&str> {
        self.inner.port.as_deref()
    }

    #[getter]
    fn path(&self) -> Option<&str> {
        self.inner.path.as_deref()
    }

    #[getter]
    fn register_procedure(&self) -> Option<&str> {
        self.inner.register_procedure.as_deref()
    }

    #[getter]
    fn protocol(&self) -> Option<&str> {
        self.inner.protocol.as_deref()
    }
}

#[pyclass(name = "TextInput", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyTextInput {
    inner: CoreTextInput,
}

impl PyTextInput {
    pub fn from_core(core: CoreTextInput) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyTextInput {
    #[getter]
    fn title(&self) -> Option<&str> {
        self.inner.title.as_deref()
    }

    #[getter]
    fn description(&self) -> Option<&str> {
        self.inner.description.as_deref()
    }

    #[getter]
    fn name(&self) -> Option<&str> {
        self.inner.name.as_deref()
    }

    #[getter]
    fn link(&self) -> Option<&str> {
        self.inner.link.as_deref()
    }
}
