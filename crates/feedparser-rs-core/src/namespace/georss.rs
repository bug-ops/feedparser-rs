//! GeoRSS namespace support for geographic location data
//!
//! Supports parsing GeoRSS Simple elements for specifying geographic locations
//! in RSS and Atom feeds. GeoRSS is commonly used in mapping applications,
//! location-based services, and geocoded content.
//!
//! # Supported Elements
//!
//! - `georss:point` - Single latitude/longitude point
//! - `georss:line` - Line string (multiple points)
//! - `georss:polygon` - Polygon (closed shape)
//! - `georss:box` - Bounding box (lower-left + upper-right)
//!
//! # Specification
//!
//! GeoRSS Simple: <http://www.georss.org/simple>

use crate::limits::ParserLimits;
use crate::types::{Entry, FeedMeta};

/// `GeoRSS` namespace URI
pub const GEORSS: &str = "http://www.georss.org/georss";

/// Type of geographic shape
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GeoType {
    /// Single point (latitude, longitude)
    #[default]
    Point,
    /// Line connecting multiple points
    Line,
    /// Closed polygon shape
    Polygon,
    /// Bounding box (lower-left, upper-right corners)
    Box,
}

/// Geographic location data from `GeoRSS`
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GeoLocation {
    /// Type of geographic shape
    pub geo_type: GeoType,
    /// Coordinate pairs as (latitude, longitude)
    ///
    /// - Point: 1 coordinate pair
    /// - Line: 2+ coordinate pairs
    /// - Polygon: 3+ coordinate pairs (first == last for closed polygon)
    /// - Box: 2 coordinate pairs (lower-left, upper-right)
    pub coordinates: Vec<(f64, f64)>,
    /// Coordinate reference system (e.g., "EPSG:4326" for WGS84)
    ///
    /// Default is WGS84 (latitude/longitude) if not specified
    pub srs_name: Option<String>,
}

impl GeoLocation {
    /// Creates new point location
    ///
    /// # Arguments
    ///
    /// * `lat` - Latitude in decimal degrees
    /// * `lon` - Longitude in decimal degrees
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::namespace::georss::GeoLocation;
    ///
    /// let loc = GeoLocation::point(45.256, -71.92);
    /// assert_eq!(loc.coordinates.len(), 1);
    /// ```
    #[must_use]
    pub fn point(lat: f64, lon: f64) -> Self {
        Self {
            geo_type: GeoType::Point,
            coordinates: vec![(lat, lon)],
            srs_name: None,
        }
    }

    /// Creates new line location
    ///
    /// # Arguments
    ///
    /// * `coords` - Vector of (latitude, longitude) pairs
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::namespace::georss::GeoLocation;
    ///
    /// let coords = vec![(45.256, -71.92), (46.0, -72.0)];
    /// let loc = GeoLocation::line(coords);
    /// assert_eq!(loc.coordinates.len(), 2);
    /// ```
    #[must_use]
    pub const fn line(coords: Vec<(f64, f64)>) -> Self {
        Self {
            geo_type: GeoType::Line,
            coordinates: coords,
            srs_name: None,
        }
    }

    /// Creates new polygon location
    ///
    /// # Arguments
    ///
    /// * `coords` - Vector of (latitude, longitude) pairs
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::namespace::georss::GeoLocation;
    ///
    /// let coords = vec![
    ///     (45.0, -71.0),
    ///     (46.0, -71.0),
    ///     (46.0, -72.0),
    ///     (45.0, -71.0), // Close the polygon
    /// ];
    /// let loc = GeoLocation::polygon(coords);
    /// ```
    #[must_use]
    pub const fn polygon(coords: Vec<(f64, f64)>) -> Self {
        Self {
            geo_type: GeoType::Polygon,
            coordinates: coords,
            srs_name: None,
        }
    }

    /// Creates new bounding box location
    ///
    /// # Arguments
    ///
    /// * `lower_lat` - Lower-left latitude
    /// * `lower_lon` - Lower-left longitude
    /// * `upper_lat` - Upper-right latitude
    /// * `upper_lon` - Upper-right longitude
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::namespace::georss::GeoLocation;
    ///
    /// let loc = GeoLocation::bbox(45.0, -72.0, 46.0, -71.0);
    /// assert_eq!(loc.coordinates.len(), 2);
    /// ```
    #[must_use]
    pub fn bbox(lower_lat: f64, lower_lon: f64, upper_lat: f64, upper_lon: f64) -> Self {
        Self {
            geo_type: GeoType::Box,
            coordinates: vec![(lower_lat, lower_lon), (upper_lat, upper_lon)],
            srs_name: None,
        }
    }
}

/// Parse `GeoRSS` element and update entry
///
/// # Arguments
///
/// * `tag` - Element local name (e.g., "point", "line", "polygon", "box")
/// * `text` - Element text content
/// * `entry` - Entry to update
/// * `_limits` - Parser limits (unused but kept for API consistency)
///
/// # Returns
///
/// `true` if element was recognized and handled, `false` otherwise
pub fn handle_entry_element(
    tag: &[u8],
    text: &str,
    entry: &mut Entry,
    _limits: &ParserLimits,
) -> bool {
    match tag {
        b"point" => {
            if let Some(loc) = parse_point(text) {
                entry.geo = Some(Box::new(loc));
            }
            true
        }
        b"line" => {
            if let Some(loc) = parse_line(text) {
                entry.geo = Some(Box::new(loc));
            }
            true
        }
        b"polygon" => {
            if let Some(loc) = parse_polygon(text) {
                entry.geo = Some(Box::new(loc));
            }
            true
        }
        b"box" => {
            if let Some(loc) = parse_box(text) {
                entry.geo = Some(Box::new(loc));
            }
            true
        }
        _ => false,
    }
}

/// Parse `GeoRSS` element and update feed metadata
///
/// # Arguments
///
/// * `tag` - Element local name (e.g., "point", "line", "polygon", "box")
/// * `text` - Element text content
/// * `feed` - Feed metadata to update
/// * `_limits` - Parser limits (unused but kept for API consistency)
///
/// # Returns
///
/// `true` if element was recognized and handled, `false` otherwise
pub fn handle_feed_element(
    tag: &[u8],
    text: &str,
    feed: &mut FeedMeta,
    _limits: &ParserLimits,
) -> bool {
    match tag {
        b"point" => {
            if let Some(loc) = parse_point(text) {
                feed.geo = Some(Box::new(loc));
            }
            true
        }
        b"line" => {
            if let Some(loc) = parse_line(text) {
                feed.geo = Some(Box::new(loc));
            }
            true
        }
        b"polygon" => {
            if let Some(loc) = parse_polygon(text) {
                feed.geo = Some(Box::new(loc));
            }
            true
        }
        b"box" => {
            if let Some(loc) = parse_box(text) {
                feed.geo = Some(Box::new(loc));
            }
            true
        }
        _ => false,
    }
}

/// Parse georss:point element
///
/// Format: "lat lon" (space-separated)
/// Example: "45.256 -71.92"
fn parse_point(text: &str) -> Option<GeoLocation> {
    let coords = parse_coordinates(text)?;
    if coords.len() == 1 {
        Some(GeoLocation {
            geo_type: GeoType::Point,
            coordinates: coords,
            srs_name: None,
        })
    } else {
        None
    }
}

/// Parse georss:line element
///
/// Format: "lat1 lon1 lat2 lon2 ..." (space-separated)
/// Example: "45.256 -71.92 46.0 -72.0"
fn parse_line(text: &str) -> Option<GeoLocation> {
    let coords = parse_coordinates(text)?;
    if coords.len() >= 2 {
        Some(GeoLocation {
            geo_type: GeoType::Line,
            coordinates: coords,
            srs_name: None,
        })
    } else {
        None
    }
}

/// Parse georss:polygon element
///
/// Format: "lat1 lon1 lat2 lon2 lat3 lon3 ..." (space-separated)
/// Example: "45.0 -71.0 46.0 -71.0 46.0 -72.0 45.0 -71.0"
fn parse_polygon(text: &str) -> Option<GeoLocation> {
    let coords = parse_coordinates(text)?;
    if coords.len() >= 3 {
        Some(GeoLocation {
            geo_type: GeoType::Polygon,
            coordinates: coords,
            srs_name: None,
        })
    } else {
        None
    }
}

/// Parse georss:box element
///
/// Format: space-separated values (lower-left, upper-right)
/// Example: "45.0 -72.0 46.0 -71.0"
fn parse_box(text: &str) -> Option<GeoLocation> {
    let coords = parse_coordinates(text)?;
    if coords.len() == 2 {
        Some(GeoLocation {
            geo_type: GeoType::Box,
            coordinates: coords,
            srs_name: None,
        })
    } else {
        None
    }
}

/// Parse space-separated coordinate pairs
///
/// Format: "lat1 lon1 lat2 lon2 ..." (pairs of floats)
fn parse_coordinates(text: &str) -> Option<Vec<(f64, f64)>> {
    let parts: Vec<&str> = text.split_whitespace().collect();

    // Must have even number of values (lat/lon pairs)
    if parts.is_empty() || !parts.len().is_multiple_of(2) {
        return None;
    }

    let mut coords = Vec::with_capacity(parts.len() / 2);

    for chunk in parts.chunks(2) {
        let lat = chunk[0].parse::<f64>().ok()?;
        let lon = chunk[1].parse::<f64>().ok()?;

        // Basic validation: latitude should be -90 to 90, longitude -180 to 180
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            return None;
        }

        coords.push((lat, lon));
    }

    Some(coords)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_point() {
        let loc = parse_point("45.256 -71.92").unwrap();
        assert_eq!(loc.geo_type, GeoType::Point);
        assert_eq!(loc.coordinates.len(), 1);
        assert_eq!(loc.coordinates[0], (45.256, -71.92));
    }

    #[test]
    fn test_parse_point_invalid() {
        assert!(parse_point("45.256").is_none());
        assert!(parse_point("45.256 -71.92 extra").is_none());
        assert!(parse_point("not numbers").is_none());
        assert!(parse_point("").is_none());
    }

    #[test]
    fn test_parse_line() {
        let loc = parse_line("45.256 -71.92 46.0 -72.0").unwrap();
        assert_eq!(loc.geo_type, GeoType::Line);
        assert_eq!(loc.coordinates.len(), 2);
        assert_eq!(loc.coordinates[0], (45.256, -71.92));
        assert_eq!(loc.coordinates[1], (46.0, -72.0));
    }

    #[test]
    fn test_parse_line_single_point() {
        // Line needs at least 2 points
        assert!(parse_line("45.256 -71.92").is_none());
    }

    #[test]
    fn test_parse_polygon() {
        let loc = parse_polygon("45.0 -71.0 46.0 -71.0 46.0 -72.0 45.0 -71.0").unwrap();
        assert_eq!(loc.geo_type, GeoType::Polygon);
        assert_eq!(loc.coordinates.len(), 4);
        assert_eq!(loc.coordinates[0], (45.0, -71.0));
        assert_eq!(loc.coordinates[3], (45.0, -71.0)); // Closed polygon
    }

    #[test]
    fn test_parse_box() {
        let loc = parse_box("45.0 -72.0 46.0 -71.0").unwrap();
        assert_eq!(loc.geo_type, GeoType::Box);
        assert_eq!(loc.coordinates.len(), 2);
        assert_eq!(loc.coordinates[0], (45.0, -72.0)); // Lower-left
        assert_eq!(loc.coordinates[1], (46.0, -71.0)); // Upper-right
    }

    #[test]
    fn test_parse_box_invalid() {
        // Box needs exactly 2 points (4 values)
        assert!(parse_box("45.0 -72.0").is_none());
        assert!(parse_box("45.0 -72.0 46.0 -71.0 extra values").is_none());
    }

    #[test]
    fn test_coordinate_validation() {
        // Invalid latitude (> 90)
        assert!(parse_point("91.0 0.0").is_none());
        // Invalid latitude (< -90)
        assert!(parse_point("-91.0 0.0").is_none());
        // Invalid longitude (> 180)
        assert!(parse_point("0.0 181.0").is_none());
        // Invalid longitude (< -180)
        assert!(parse_point("0.0 -181.0").is_none());
    }

    #[test]
    fn test_handle_entry_element_point() {
        let mut entry = Entry::default();
        let limits = ParserLimits::default();

        let handled = handle_entry_element(b"point", "45.256 -71.92", &mut entry, &limits);
        assert!(handled);
        assert!(entry.geo.is_some());

        let geo = entry.geo.as_ref().unwrap();
        assert_eq!(geo.geo_type, GeoType::Point);
        assert_eq!(geo.coordinates[0], (45.256, -71.92));
    }

    #[test]
    fn test_handle_entry_element_line() {
        let mut entry = Entry::default();
        let limits = ParserLimits::default();

        let handled =
            handle_entry_element(b"line", "45.256 -71.92 46.0 -72.0", &mut entry, &limits);
        assert!(handled);
        assert!(entry.geo.is_some());
        assert_eq!(entry.geo.as_ref().unwrap().geo_type, GeoType::Line);
    }

    #[test]
    fn test_handle_entry_element_unknown() {
        let mut entry = Entry::default();
        let limits = ParserLimits::default();

        let handled = handle_entry_element(b"unknown", "data", &mut entry, &limits);
        assert!(!handled);
        assert!(entry.geo.is_none());
    }

    #[test]
    fn test_geo_location_constructors() {
        let point = GeoLocation::point(45.0, -71.0);
        assert_eq!(point.geo_type, GeoType::Point);
        assert_eq!(point.coordinates.len(), 1);

        let line = GeoLocation::line(vec![(45.0, -71.0), (46.0, -72.0)]);
        assert_eq!(line.geo_type, GeoType::Line);
        assert_eq!(line.coordinates.len(), 2);

        let polygon = GeoLocation::polygon(vec![(45.0, -71.0), (46.0, -71.0), (45.0, -71.0)]);
        assert_eq!(polygon.geo_type, GeoType::Polygon);
        assert_eq!(polygon.coordinates.len(), 3);

        let bbox = GeoLocation::bbox(45.0, -72.0, 46.0, -71.0);
        assert_eq!(bbox.geo_type, GeoType::Box);
        assert_eq!(bbox.coordinates.len(), 2);
    }

    #[test]
    fn test_whitespace_handling() {
        let loc = parse_point("  45.256   -71.92  ").unwrap();
        assert_eq!(loc.coordinates[0], (45.256, -71.92));
    }

    #[test]
    fn test_handle_feed_element_point() {
        let mut feed = FeedMeta::default();
        let limits = ParserLimits::default();

        let handled = handle_feed_element(b"point", "45.256 -71.92", &mut feed, &limits);
        assert!(handled);
        assert!(feed.geo.is_some());

        let geo = feed.geo.as_ref().unwrap();
        assert_eq!(geo.geo_type, GeoType::Point);
        assert_eq!(geo.coordinates[0], (45.256, -71.92));
    }

    #[test]
    fn test_handle_feed_element_line() {
        let mut feed = FeedMeta::default();
        let limits = ParserLimits::default();

        let handled = handle_feed_element(b"line", "45.256 -71.92 46.0 -72.0", &mut feed, &limits);
        assert!(handled);
        assert!(feed.geo.is_some());
        assert_eq!(feed.geo.as_ref().unwrap().geo_type, GeoType::Line);
    }

    #[test]
    fn test_handle_feed_element_polygon() {
        let mut feed = FeedMeta::default();
        let limits = ParserLimits::default();

        let handled = handle_feed_element(
            b"polygon",
            "45.0 -71.0 46.0 -71.0 46.0 -72.0 45.0 -71.0",
            &mut feed,
            &limits,
        );
        assert!(handled);
        assert!(feed.geo.is_some());
        assert_eq!(feed.geo.as_ref().unwrap().geo_type, GeoType::Polygon);
    }

    #[test]
    fn test_handle_feed_element_box() {
        let mut feed = FeedMeta::default();
        let limits = ParserLimits::default();

        let handled = handle_feed_element(b"box", "45.0 -72.0 46.0 -71.0", &mut feed, &limits);
        assert!(handled);
        assert!(feed.geo.is_some());
        assert_eq!(feed.geo.as_ref().unwrap().geo_type, GeoType::Box);
    }

    #[test]
    fn test_handle_feed_element_unknown() {
        let mut feed = FeedMeta::default();
        let limits = ParserLimits::default();

        let handled = handle_feed_element(b"unknown", "data", &mut feed, &limits);
        assert!(!handled);
        assert!(feed.geo.is_none());
    }

    #[test]
    fn test_handle_feed_element_invalid_data() {
        let mut feed = FeedMeta::default();
        let limits = ParserLimits::default();

        let handled = handle_feed_element(b"point", "invalid data", &mut feed, &limits);
        assert!(handled);
        assert!(feed.geo.is_none());
    }
}
