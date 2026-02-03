use feedparser_rs::{
    Content as CoreContent, Enclosure as CoreEnclosure, Generator as CoreGenerator,
    Image as CoreImage, Link as CoreLink, Person as CorePerson, Source as CoreSource,
    Tag as CoreTag, TextConstruct as CoreTextConstruct, TextType,
};
use pyo3::prelude::*;

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
            TextType::Text => "text",
            TextType::Html => "html",
            TextType::Xhtml => "xhtml",
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
    fn length(&self) -> Option<u64> {
        self.inner.length
    }

    #[getter]
    fn hreflang(&self) -> Option<&str> {
        self.inner.hreflang.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "Link(href='{}', rel='{}')",
            &self.inner.href,
            self.inner.rel.as_deref().unwrap_or("alternate")
        )
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
    fn uri(&self) -> Option<&str> {
        self.inner.uri.as_deref()
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
    fn url(&self) -> &str {
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
    fn description(&self) -> Option<&str> {
        self.inner.description.as_deref()
    }

    fn __repr__(&self) -> String {
        format!("Image(url='{}')", &self.inner.url)
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
    fn url(&self) -> &str {
        &self.inner.url
    }

    #[getter]
    fn length(&self) -> Option<u64> {
        self.inner.length
    }

    #[getter]
    #[pyo3(name = "type")]
    fn enclosure_type(&self) -> Option<&str> {
        self.inner.enclosure_type.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "Enclosure(url='{}', type='{}')",
            &self.inner.url,
            self.inner.enclosure_type.as_deref().unwrap_or("unknown")
        )
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

    fn __repr__(&self) -> String {
        format!(
            "Content(type='{}', value='{}')",
            self.inner.content_type.as_deref().unwrap_or("text/plain"),
            &self.inner.value.chars().take(50).collect::<String>()
        )
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
    #[getter]
    fn value(&self) -> &str {
        &self.inner.value
    }

    #[getter]
    fn uri(&self) -> Option<&str> {
        self.inner.uri.as_deref()
    }

    #[getter]
    fn version(&self) -> Option<&str> {
        self.inner.version.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "Generator(value='{}', version='{}')",
            &self.inner.value,
            self.inner.version.as_deref().unwrap_or("unknown")
        )
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

    #[getter]
    fn link(&self) -> Option<&str> {
        self.inner.link.as_deref()
    }

    #[getter]
    fn id(&self) -> Option<&str> {
        self.inner.id.as_deref()
    }

    fn __repr__(&self) -> String {
        if let Some(title) = &self.inner.title {
            format!("Source(title='{}')", title)
        } else {
            "Source()".to_string()
        }
    }
}
