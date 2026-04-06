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
    /// Elevation in meters (from `georss:elev`)
    pub elev: Option<f64>,
    /// Feature type classification (from `georss:featuretypetag`)
    pub feature_type_tag: Option<String>,
    /// Human-readable place name (from `georss:featurename`)
    pub feature_name: Option<String>,
    /// Relationship type (from `georss:relationshiptag`)
    pub relationship_tag: Option<String>,
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
            ..Default::default()
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
    pub fn line(coords: Vec<(f64, f64)>) -> Self {
        Self {
            geo_type: GeoType::Line,
            coordinates: coords,
            ..Default::default()
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
    pub fn polygon(coords: Vec<(f64, f64)>) -> Self {
        Self {
            geo_type: GeoType::Polygon,
            coordinates: coords,
            ..Default::default()
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
            ..Default::default()
        }
    }
}

/// Parse W3C Basic Geo element and update entry
///
/// Handles `geo:lat` and `geo:long` elements. When both are present,
/// auto-constructs `entry.r#where` as a point location.
///
/// # Arguments
///
/// * `tag` - Element local name (e.g., "lat", "long", "lon")
/// * `text` - Element text content
/// * `entry` - Entry to update
///
/// # Returns
///
/// `true` if element was recognized and handled, `false` otherwise
pub fn handle_entry_geo_element(tag: &[u8], text: &str, entry: &mut Entry) -> bool {
    match tag {
        b"lat" => {
            entry.geo_lat = Some(text.to_string());
            try_build_entry_where(entry);
            true
        }
        b"long" | b"lon" => {
            entry.geo_long = Some(text.to_string());
            try_build_entry_where(entry);
            true
        }
        _ => false,
    }
}

/// Parse W3C Basic Geo element and update feed metadata
///
/// Handles `geo:lat` and `geo:long` elements. When both are present,
/// auto-constructs `feed.r#where` as a point location.
///
/// # Arguments
///
/// * `tag` - Element local name (e.g., "lat", "long", "lon")
/// * `text` - Element text content
/// * `feed` - Feed metadata to update
///
/// # Returns
///
/// `true` if element was recognized and handled, `false` otherwise
pub fn handle_feed_geo_element(tag: &[u8], text: &str, feed: &mut FeedMeta) -> bool {
    match tag {
        b"lat" => {
            feed.geo_lat = Some(text.to_string());
            try_build_feed_where(feed);
            true
        }
        b"long" | b"lon" => {
            feed.geo_long = Some(text.to_string());
            try_build_feed_where(feed);
            true
        }
        _ => false,
    }
}

fn try_build_entry_where(entry: &mut Entry) {
    if let (Some(lat_str), Some(lon_str)) = (entry.geo_lat.as_deref(), entry.geo_long.as_deref())
        && let (Ok(lat), Ok(lon)) = (lat_str.parse::<f64>(), lon_str.parse::<f64>())
        && (-90.0..=90.0).contains(&lat)
        && (-180.0..=180.0).contains(&lon)
    {
        entry.r#where = Some(Box::new(GeoLocation::point(lat, lon)));
    }
}

fn try_build_feed_where(feed: &mut FeedMeta) {
    if let (Some(lat_str), Some(lon_str)) = (feed.geo_lat.as_deref(), feed.geo_long.as_deref())
        && let (Ok(lat), Ok(lon)) = (lat_str.parse::<f64>(), lon_str.parse::<f64>())
        && (-90.0..=90.0).contains(&lat)
        && (-180.0..=180.0).contains(&lon)
    {
        feed.r#where = Some(Box::new(GeoLocation::point(lat, lon)));
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
                let existing = entry
                    .r#where
                    .get_or_insert_with(|| Box::new(GeoLocation::default()));
                existing.geo_type = loc.geo_type;
                existing.coordinates = loc.coordinates;
                existing.srs_name = loc.srs_name;
            }
            true
        }
        b"line" => {
            if let Some(loc) = parse_line(text) {
                let existing = entry
                    .r#where
                    .get_or_insert_with(|| Box::new(GeoLocation::default()));
                existing.geo_type = loc.geo_type;
                existing.coordinates = loc.coordinates;
                existing.srs_name = loc.srs_name;
            }
            true
        }
        b"polygon" => {
            if let Some(loc) = parse_polygon(text) {
                let existing = entry
                    .r#where
                    .get_or_insert_with(|| Box::new(GeoLocation::default()));
                existing.geo_type = loc.geo_type;
                existing.coordinates = loc.coordinates;
                existing.srs_name = loc.srs_name;
            }
            true
        }
        b"box" => {
            if let Some(loc) = parse_box(text) {
                let existing = entry
                    .r#where
                    .get_or_insert_with(|| Box::new(GeoLocation::default()));
                existing.geo_type = loc.geo_type;
                existing.coordinates = loc.coordinates;
                existing.srs_name = loc.srs_name;
            }
            true
        }
        b"elev" => {
            if let Ok(v) = text.trim().parse::<f64>()
                && v.is_finite()
            {
                entry
                    .r#where
                    .get_or_insert_with(|| Box::new(GeoLocation::default()))
                    .elev = Some(v);
            }
            true
        }
        b"featuretypetag" => {
            entry
                .r#where
                .get_or_insert_with(|| Box::new(GeoLocation::default()))
                .feature_type_tag = Some(text.to_string());
            true
        }
        b"featurename" => {
            entry
                .r#where
                .get_or_insert_with(|| Box::new(GeoLocation::default()))
                .feature_name = Some(text.to_string());
            true
        }
        b"relationshiptag" => {
            entry
                .r#where
                .get_or_insert_with(|| Box::new(GeoLocation::default()))
                .relationship_tag = Some(text.to_string());
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
                let existing = feed
                    .r#where
                    .get_or_insert_with(|| Box::new(GeoLocation::default()));
                existing.geo_type = loc.geo_type;
                existing.coordinates = loc.coordinates;
                existing.srs_name = loc.srs_name;
            }
            true
        }
        b"line" => {
            if let Some(loc) = parse_line(text) {
                let existing = feed
                    .r#where
                    .get_or_insert_with(|| Box::new(GeoLocation::default()));
                existing.geo_type = loc.geo_type;
                existing.coordinates = loc.coordinates;
                existing.srs_name = loc.srs_name;
            }
            true
        }
        b"polygon" => {
            if let Some(loc) = parse_polygon(text) {
                let existing = feed
                    .r#where
                    .get_or_insert_with(|| Box::new(GeoLocation::default()));
                existing.geo_type = loc.geo_type;
                existing.coordinates = loc.coordinates;
                existing.srs_name = loc.srs_name;
            }
            true
        }
        b"box" => {
            if let Some(loc) = parse_box(text) {
                let existing = feed
                    .r#where
                    .get_or_insert_with(|| Box::new(GeoLocation::default()));
                existing.geo_type = loc.geo_type;
                existing.coordinates = loc.coordinates;
                existing.srs_name = loc.srs_name;
            }
            true
        }
        b"elev" => {
            if let Ok(v) = text.trim().parse::<f64>()
                && v.is_finite()
            {
                feed.r#where
                    .get_or_insert_with(|| Box::new(GeoLocation::default()))
                    .elev = Some(v);
            }
            true
        }
        b"featuretypetag" => {
            feed.r#where
                .get_or_insert_with(|| Box::new(GeoLocation::default()))
                .feature_type_tag = Some(text.to_string());
            true
        }
        b"featurename" => {
            feed.r#where
                .get_or_insert_with(|| Box::new(GeoLocation::default()))
                .feature_name = Some(text.to_string());
            true
        }
        b"relationshiptag" => {
            feed.r#where
                .get_or_insert_with(|| Box::new(GeoLocation::default()))
                .relationship_tag = Some(text.to_string());
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
        assert!(entry.r#where.is_some());

        let geo = entry.r#where.as_ref().unwrap();
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
        assert!(entry.r#where.is_some());
        assert_eq!(entry.r#where.as_ref().unwrap().geo_type, GeoType::Line);
    }

    #[test]
    fn test_handle_entry_element_unknown() {
        let mut entry = Entry::default();
        let limits = ParserLimits::default();

        let handled = handle_entry_element(b"unknown", "data", &mut entry, &limits);
        assert!(!handled);
        assert!(entry.r#where.is_none());
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
        assert!(feed.r#where.is_some());

        let geo = feed.r#where.as_ref().unwrap();
        assert_eq!(geo.geo_type, GeoType::Point);
        assert_eq!(geo.coordinates[0], (45.256, -71.92));
    }

    #[test]
    fn test_handle_feed_element_line() {
        let mut feed = FeedMeta::default();
        let limits = ParserLimits::default();

        let handled = handle_feed_element(b"line", "45.256 -71.92 46.0 -72.0", &mut feed, &limits);
        assert!(handled);
        assert!(feed.r#where.is_some());
        assert_eq!(feed.r#where.as_ref().unwrap().geo_type, GeoType::Line);
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
        assert!(feed.r#where.is_some());
        assert_eq!(feed.r#where.as_ref().unwrap().geo_type, GeoType::Polygon);
    }

    #[test]
    fn test_handle_feed_element_box() {
        let mut feed = FeedMeta::default();
        let limits = ParserLimits::default();

        let handled = handle_feed_element(b"box", "45.0 -72.0 46.0 -71.0", &mut feed, &limits);
        assert!(handled);
        assert!(feed.r#where.is_some());
        assert_eq!(feed.r#where.as_ref().unwrap().geo_type, GeoType::Box);
    }

    #[test]
    fn test_handle_feed_element_unknown() {
        let mut feed = FeedMeta::default();
        let limits = ParserLimits::default();

        let handled = handle_feed_element(b"unknown", "data", &mut feed, &limits);
        assert!(!handled);
        assert!(feed.r#where.is_none());
    }

    #[test]
    fn test_handle_feed_element_invalid_data() {
        let mut feed = FeedMeta::default();
        let limits = ParserLimits::default();

        let handled = handle_feed_element(b"point", "invalid data", &mut feed, &limits);
        assert!(handled);
        assert!(feed.r#where.is_none());
    }

    #[test]
    fn test_handle_entry_element_elev() {
        let mut entry = Entry::default();
        let limits = ParserLimits::default();

        let handled = handle_entry_element(b"elev", "1337.5", &mut entry, &limits);
        assert!(handled);
        let geo = entry.r#where.as_ref().unwrap();
        assert_eq!(geo.elev, Some(1337.5));
    }

    #[test]
    fn test_handle_entry_element_feature_name() {
        let mut entry = Entry::default();
        let limits = ParserLimits::default();

        let handled = handle_entry_element(b"featurename", "Mont Mégantic", &mut entry, &limits);
        assert!(handled);
        let geo = entry.r#where.as_ref().unwrap();
        assert_eq!(geo.feature_name.as_deref(), Some("Mont Mégantic"));
    }

    #[test]
    fn test_handle_entry_element_feature_type_tag() {
        let mut entry = Entry::default();
        let limits = ParserLimits::default();

        let handled = handle_entry_element(b"featuretypetag", "mountain", &mut entry, &limits);
        assert!(handled);
        let geo = entry.r#where.as_ref().unwrap();
        assert_eq!(geo.feature_type_tag.as_deref(), Some("mountain"));
    }

    #[test]
    fn test_handle_entry_element_relationship_tag() {
        let mut entry = Entry::default();
        let limits = ParserLimits::default();

        let handled =
            handle_entry_element(b"relationshiptag", "is-located-at", &mut entry, &limits);
        assert!(handled);
        let geo = entry.r#where.as_ref().unwrap();
        assert_eq!(geo.relationship_tag.as_deref(), Some("is-located-at"));
    }

    #[test]
    fn test_extended_attrs_without_geometry() {
        let mut entry = Entry::default();
        let limits = ParserLimits::default();

        handle_entry_element(b"featurename", "Unknown Location", &mut entry, &limits);
        let geo = entry.r#where.as_ref().unwrap();
        assert_eq!(geo.feature_name.as_deref(), Some("Unknown Location"));
        assert!(geo.coordinates.is_empty());
    }

    #[test]
    fn test_extended_attrs_invalid_elev() {
        let mut entry = Entry::default();
        let limits = ParserLimits::default();

        let handled = handle_entry_element(b"elev", "not-a-number", &mut entry, &limits);
        assert!(handled);
        // GeoLocation not created because elev parse failed
        assert!(entry.r#where.is_none());
    }

    #[test]
    fn test_extended_attrs_elev_non_finite_ignored() {
        let limits = ParserLimits::default();

        for value in ["NaN", "Infinity", "-Infinity"] {
            let mut entry = Entry::default();
            let handled = handle_entry_element(b"elev", value, &mut entry, &limits);
            assert!(handled, "element must be recognized for value {value}");
            assert!(
                entry.r#where.is_none(),
                "non-finite elev '{value}' must not create GeoLocation"
            );
        }
    }

    #[test]
    fn test_extended_attrs_before_geometry() {
        let mut entry = Entry::default();
        let limits = ParserLimits::default();

        handle_entry_element(b"featurename", "Reverse Order", &mut entry, &limits);
        handle_entry_element(b"elev", "500.0", &mut entry, &limits);
        handle_entry_element(b"point", "40.0 -74.0", &mut entry, &limits);

        let geo = entry.r#where.as_ref().unwrap();
        assert_eq!(geo.geo_type, GeoType::Point);
        assert_eq!(geo.coordinates[0], (40.0, -74.0));
        assert_eq!(geo.feature_name.as_deref(), Some("Reverse Order"));
        assert_eq!(geo.elev, Some(500.0));
    }

    #[test]
    fn test_extended_attrs_after_geometry() {
        let mut entry = Entry::default();
        let limits = ParserLimits::default();

        handle_entry_element(b"point", "45.256 -71.92", &mut entry, &limits);
        handle_entry_element(b"featurename", "Mont Mégantic", &mut entry, &limits);
        handle_entry_element(b"elev", "1337.5", &mut entry, &limits);

        let geo = entry.r#where.as_ref().unwrap();
        assert_eq!(geo.geo_type, GeoType::Point);
        assert_eq!(geo.coordinates[0], (45.256, -71.92));
        assert_eq!(geo.feature_name.as_deref(), Some("Mont Mégantic"));
        assert_eq!(geo.elev, Some(1337.5));
    }
}
