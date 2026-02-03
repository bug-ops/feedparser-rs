use feedparser_rs::namespace::georss::{GeoLocation as CoreGeoLocation, GeoType};
use pyo3::prelude::*;

/// Represents a GeoRSS geographic location.
///
/// GeoRSS is a namespace extension that adds geographic information to RSS feeds.
/// Supports points, lines, polygons, and bounding boxes with coordinate data.
#[pyclass(name = "GeoLocation", module = "feedparser_rs", from_py_object)]
#[derive(Clone)]
pub struct PyGeoLocation {
    inner: CoreGeoLocation,
}

impl PyGeoLocation {
    pub fn from_core(core: CoreGeoLocation) -> Self {
        Self { inner: core }
    }
}

#[pymethods]
impl PyGeoLocation {
    #[getter]
    fn geo_type(&self) -> &str {
        match self.inner.geo_type {
            GeoType::Point => "point",
            GeoType::Line => "line",
            GeoType::Polygon => "polygon",
            GeoType::Box => "box",
        }
    }

    #[getter]
    fn coordinates(&self) -> Vec<(f64, f64)> {
        self.inner.coordinates.clone()
    }

    #[getter]
    fn srs_name(&self) -> Option<&str> {
        self.inner.srs_name.as_deref()
    }

    fn __repr__(&self) -> String {
        match self.inner.geo_type {
            GeoType::Point => {
                if let Some(coord) = self.inner.coordinates.first() {
                    format!(
                        "GeoLocation(geo_type='point', coordinates=[({}, {})])",
                        coord.0, coord.1
                    )
                } else {
                    "GeoLocation(geo_type='point', coordinates=[])".to_string()
                }
            }
            _ => format!(
                "GeoLocation(geo_type='{}', coordinates={})",
                self.geo_type(),
                self.inner.coordinates.len()
            ),
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner.geo_type == other.inner.geo_type
            && self.inner.coordinates == other.inner.coordinates
            && self.inner.srs_name == other.inner.srs_name
    }
}
