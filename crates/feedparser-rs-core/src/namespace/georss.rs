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
//! Also supports the GeoRSS GML profile (`georss:where` wrapping
//! `gml:Point`/`gml:LineString`/`gml:Polygon`/`gml:MultiSurface`/
//! `gml:Envelope`), including `srsName`-driven axis-order normalization.
//! XML traversal for the GML profile lives in the parser's internal
//! `common::parse_georss_where` since it needs the `quick-xml` reader; this
//! module provides the pure coordinate/axis-order logic it calls into.
//!
//! # Specification
//!
//! - GeoRSS Simple: <http://www.georss.org/simple>
//! - GeoRSS GML profile: <http://www.georss.org/gml>

use crate::limits::ParserLimits;
use crate::types::{Entry, FeedMeta};

/// `GeoRSS` namespace URI
pub const GEORSS: &str = "http://www.georss.org/georss";

/// GML (Geography Markup Language) namespace URI used by the `GeoRSS` GML profile
pub const GML: &str = "http://www.opengis.net/gml";

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

/// Merge a freshly parsed geometry into an entry/feed's `where` field.
///
/// The geometry fields (`geo_type`, `coordinates`, `srs_name`) are replaced
/// wholesale — including resetting `srs_name` to `None` if `loc` has none,
/// e.g. a `GeoRSS` Simple element following a GML one — since a geometry
/// and its CRS are one unit; extended attributes (`elev`, `featureName`,
/// etc.) that may have been set from elements appearing before or after the
/// geometry element are preserved. Shared by the `GeoRSS` Simple element
/// handlers below and by the parser's internal `common::parse_georss_where`
/// (GML profile).
pub fn merge_geometry(target: &mut Option<Box<GeoLocation>>, loc: GeoLocation) {
    let existing = target.get_or_insert_with(|| Box::new(GeoLocation::default()));
    existing.geo_type = loc.geo_type;
    existing.coordinates = loc.coordinates;
    existing.srs_name = loc.srs_name;
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
                merge_geometry(&mut entry.r#where, loc);
            }
            true
        }
        b"line" => {
            if let Some(loc) = parse_line(text) {
                merge_geometry(&mut entry.r#where, loc);
            }
            true
        }
        b"polygon" => {
            if let Some(loc) = parse_polygon(text) {
                merge_geometry(&mut entry.r#where, loc);
            }
            true
        }
        b"box" => {
            if let Some(loc) = parse_box(text) {
                merge_geometry(&mut entry.r#where, loc);
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
                merge_geometry(&mut feed.r#where, loc);
            }
            true
        }
        b"line" => {
            if let Some(loc) = parse_line(text) {
                merge_geometry(&mut feed.r#where, loc);
            }
            true
        }
        b"polygon" => {
            if let Some(loc) = parse_polygon(text) {
                merge_geometry(&mut feed.r#where, loc);
            }
            true
        }
        b"box" => {
            if let Some(loc) = parse_box(text) {
                merge_geometry(&mut feed.r#where, loc);
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
    parse_coordinates_ordered(text, true, 2).into_option()
}

/// Outcome of [`parse_coordinates_ordered`], distinguishing a coordinate
/// count that doesn't divide evenly by `dims` from other malformed input
/// (non-numeric tokens, out-of-range values, empty text) — the former is a
/// distinct, more specific "bozo" condition the GML profile callers surface
/// to the caller instead of collapsing to a generic failure.
enum CoordParse {
    /// Successfully parsed coordinate pairs.
    Ok(Vec<(f64, f64)>),
    /// Token count present but not a multiple of `dims`.
    DimsMismatch,
    /// Empty text, a non-numeric token, or an out-of-range/non-finite value.
    Invalid,
}

impl CoordParse {
    fn into_option(self) -> Option<Vec<(f64, f64)>> {
        match self {
            Self::Ok(coords) => Some(coords),
            Self::DimsMismatch | Self::Invalid => None,
        }
    }
}

/// Parse coordinate tuples, applying the given axis order and dimensionality.
///
/// When `lat_lon_order` is `true`, each tuple's first two values are read as
/// `(lat, lon)` directly and validated against the `[-90, 90]`/`[-180, 180]`
/// degree ranges — the convention `GeoRSS` Simple always uses, and GML uses
/// for geographic CRSes. When `false` (a projected/non-geographic GML CRS),
/// the first two values are read as `(lon, lat)` and swapped, and — since
/// projected coordinates are not degrees (typically meters) — only checked
/// for finiteness rather than the degree ranges.
///
/// `dims` is the coordinate dimensionality (`gml:srsDimension`, GML only):
/// `3` chunks by three and drops the third (elevation) component per tuple
/// rather than letting it corrupt the next tuple's latitude; any other
/// value (including the `GeoRSS` Simple default) chunks by two. Comma
/// separators (a common non-conformant real-world variant) are normalized
/// to whitespace before splitting.
fn parse_coordinates_ordered(text: &str, lat_lon_order: bool, dims: usize) -> CoordParse {
    let dims = if dims == 3 { 3 } else { 2 };
    let normalized = text.replace(',', " ");
    let parts: Vec<&str> = normalized.split_whitespace().collect();

    if parts.is_empty() {
        return CoordParse::Invalid;
    }
    if !parts.len().is_multiple_of(dims) {
        return CoordParse::DimsMismatch;
    }

    let mut coords = Vec::with_capacity(parts.len() / dims);

    for chunk in parts.chunks(dims) {
        let Ok(a) = chunk[0].parse::<f64>() else {
            return CoordParse::Invalid;
        };
        let Ok(b) = chunk[1].parse::<f64>() else {
            return CoordParse::Invalid;
        };
        // chunk[2] (present only when dims == 3) is the elevation component;
        // intentionally dropped here — GeoLocation has no z-coordinate slot
        // (see `georss:elev` for the crate's separate elevation field).
        let (lat, lon) = if lat_lon_order { (a, b) } else { (b, a) };

        if lat_lon_order {
            if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
                return CoordParse::Invalid;
            }
        } else if !lat.is_finite() || !lon.is_finite() {
            return CoordParse::Invalid;
        }

        coords.push((lat, lon));
    }

    CoordParse::Ok(coords)
}

/// EPSG codes for geographic (latitude/longitude-axis) coordinate reference
/// systems, per the axis order defined by the EPSG registry. Mirrors the
/// `_geogCS` table in Python feedparser's `GeoRSS` GML support, used to
/// decide whether `gml:pos`/`gml:posList` values need swapping to match
/// this crate's `(latitude, longitude)` `GeoLocation::coordinates` order.
const GEOGRAPHIC_EPSG_CODES: &[u32] = &[
    3819, 3821, 3824, 3889, 3906, 4001, 4002, 4003, 4004, 4005, 4006, 4007, 4008, 4009, 4010, 4011,
    4012, 4013, 4014, 4015, 4016, 4018, 4019, 4020, 4021, 4022, 4023, 4024, 4025, 4027, 4028, 4029,
    4030, 4031, 4032, 4033, 4034, 4035, 4036, 4041, 4042, 4043, 4044, 4045, 4046, 4047, 4052, 4053,
    4054, 4055, 4075, 4081, 4120, 4121, 4122, 4123, 4124, 4125, 4126, 4127, 4128, 4129, 4130, 4131,
    4132, 4133, 4134, 4135, 4136, 4137, 4138, 4139, 4140, 4141, 4142, 4143, 4144, 4145, 4146, 4147,
    4148, 4149, 4150, 4151, 4152, 4153, 4154, 4155, 4156, 4157, 4158, 4159, 4160, 4161, 4162, 4163,
    4164, 4165, 4166, 4167, 4168, 4169, 4170, 4171, 4172, 4173, 4174, 4175, 4176, 4178, 4179, 4180,
    4181, 4182, 4183, 4184, 4185, 4188, 4189, 4190, 4191, 4192, 4193, 4194, 4195, 4196, 4197, 4198,
    4199, 4200, 4201, 4202, 4203, 4204, 4205, 4206, 4207, 4208, 4209, 4210, 4211, 4212, 4213, 4214,
    4215, 4216, 4218, 4219, 4220, 4221, 4222, 4223, 4224, 4225, 4226, 4227, 4228, 4229, 4230, 4231,
    4232, 4233, 4234, 4235, 4236, 4237, 4238, 4239, 4240, 4241, 4242, 4243, 4244, 4245, 4246, 4247,
    4248, 4249, 4250, 4251, 4252, 4253, 4254, 4255, 4256, 4257, 4258, 4259, 4260, 4261, 4262, 4263,
    4264, 4265, 4266, 4267, 4268, 4269, 4270, 4271, 4272, 4273, 4274, 4275, 4276, 4277, 4278, 4279,
    4280, 4281, 4282, 4283, 4284, 4285, 4286, 4287, 4288, 4289, 4291, 4292, 4293, 4294, 4295, 4296,
    4297, 4298, 4299, 4300, 4301, 4302, 4303, 4304, 4306, 4307, 4308, 4309, 4310, 4311, 4312, 4313,
    4314, 4315, 4316, 4317, 4318, 4319, 4322, 4324, 4326, 4463, 4470, 4475, 4483, 4490, 4555, 4558,
    4600, 4601, 4602, 4603, 4604, 4605, 4606, 4607, 4608, 4609, 4610, 4611, 4612, 4613, 4614, 4615,
    4616, 4617, 4618, 4619, 4620, 4621, 4622, 4623, 4624, 4625, 4626, 4627, 4628, 4629, 4630, 4631,
    4632, 4633, 4634, 4635, 4636, 4637, 4638, 4639, 4640, 4641, 4642, 4643, 4644, 4645, 4646, 4657,
    4658, 4659, 4660, 4661, 4662, 4663, 4664, 4665, 4666, 4667, 4668, 4669, 4670, 4671, 4672, 4673,
    4674, 4675, 4676, 4677, 4678, 4679, 4680, 4681, 4682, 4683, 4684, 4685, 4686, 4687, 4688, 4689,
    4690, 4691, 4692, 4693, 4694, 4695, 4696, 4697, 4698, 4699, 4700, 4701, 4702, 4703, 4704, 4705,
    4706, 4707, 4708, 4709, 4710, 4711, 4712, 4713, 4714, 4715, 4716, 4717, 4718, 4719, 4720, 4721,
    4722, 4723, 4724, 4725, 4726, 4727, 4728, 4729, 4730, 4731, 4732, 4733, 4734, 4735, 4736, 4737,
    4738, 4739, 4740, 4741, 4742, 4743, 4744, 4745, 4746, 4747, 4748, 4749, 4750, 4751, 4752, 4753,
    4754, 4755, 4756, 4757, 4758, 4759, 4760, 4761, 4762, 4763, 4764, 4765, 4801, 4802, 4803, 4804,
    4805, 4806, 4807, 4808, 4809, 4810, 4811, 4813, 4814, 4815, 4816, 4817, 4818, 4819, 4820, 4821,
    4823, 4824, 4901, 4902, 4903, 4904, 4979,
];

/// Returns `true` if `code` is a known geographic (lat/lon-axis) EPSG CRS.
fn is_geographic_epsg(code: u32) -> bool {
    GEOGRAPHIC_EPSG_CODES.binary_search(&code).is_ok()
}

/// Extract a trailing numeric EPSG code from a `srsName` value.
///
/// Handles the common forms `"EPSG:4326"`, `"urn:ogc:def:crs:EPSG::4326"`,
/// `"http://www.opengis.net/def/crs/EPSG/0/4326"`, and the classic GML 2
/// fragment form `"http://www.opengis.net/gml/srs/epsg.xml#4326"`. Tolerates
/// leading/trailing whitespace (from XML attribute-value normalization of a
/// line-wrapped attribute). Returns `None` if the value doesn't mention EPSG
/// or has no trailing numeric segment.
fn extract_epsg_code(srs_name: &str) -> Option<u32> {
    let trimmed = srs_name.trim();
    if !trimmed.to_ascii_uppercase().contains("EPSG") {
        return None;
    }
    trimmed
        .rsplit([':', '/', '#'])
        .map(str::trim)
        .find(|segment| !segment.is_empty())
        .and_then(|segment| segment.parse().ok())
}

/// Determine whether `gml:pos`/`gml:posList` values for `srs_name` are
/// ordered `(latitude, longitude)` — matching this crate's
/// `GeoLocation::coordinates` order directly — or need swapping from
/// `(longitude, latitude)`.
///
/// Per the `GeoRSS` GML profile, geographic CRSes (including the implied
/// default, WGS84 / EPSG:4326) use `(lat, lon)` axis order; most projected
/// CRSes use easting/northing order instead. `OGC:CRS84` (and its `urn`/
/// `http` forms) is special-cased to `(lon, lat)`: it is WGS84 like
/// EPSG:4326, but defined with the opposite axis order, and carries no
/// `EPSG` token so it would otherwise fall through to the geographic
/// default. Defaults to `(lat, lon)` when `srs_name` is absent or doesn't
/// reference a recognized EPSG code or CRS84.
fn srs_uses_lat_lon_order(srs_name: Option<&str>) -> bool {
    match srs_name {
        None => true,
        Some(name) if name.to_ascii_uppercase().contains("CRS84") => false,
        Some(name) => extract_epsg_code(name).is_none_or(is_geographic_epsg),
    }
}

/// Marker error: a GML geometry's coordinate text was present and non-empty,
/// but its token count wasn't a multiple of the resolved `srsDimension`.
///
/// Returned by [`build_gml_geometry`] and [`build_gml_envelope`] as a
/// distinct outcome from `Ok(None)` (other malformed input, which stays
/// silent per the tolerant "bozo" pattern): this specific condition is one
/// the caller should surface as `bozo = true` with a description, since a
/// coordinate-count mismatch is otherwise indistinguishable from a feed with
/// no GML geometry at all (#478).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GmlDimsMismatch;

/// Build a `GeoLocation` from a parsed `GeoRSS` GML profile geometry.
///
/// `geo_type` must be `Point`, `Line`, or `Polygon` — `Box` (`gml:Envelope`)
/// is handled separately by [`build_gml_envelope`], since it has no
/// `gml:pos`/`gml:posList` coordinate text; passing `Box` here always
/// returns `Ok(None)`. `text` is the raw
/// `gml:pos`/`gml:posList` coordinate text; axis order is normalized to
/// `(latitude, longitude)` using `srs_name` per the referenced CRS's axis
/// order (geographic CRSes, including the WGS84 default, use `(lat, lon)`;
/// most projected CRSes use easting/northing order instead, and — since
/// those values are typically meters, not degrees — are only checked for
/// finiteness rather than the `[-90, 90]`/`[-180, 180]` degree ranges).
/// `dims` is `gml:srsDimension` (`3` for 3D
/// `gml:pos`/`gml:posList`; anything else, including the common absence of
/// the attribute, means 2D).
///
/// Returns `Ok(None)` — the tolerant "bozo" pattern — if the coordinate text
/// is malformed, out of range, or has too few points for `geo_type`; the
/// caller should skip the geometry (including `srs_name`, since there is no
/// geometry left to attach it to) rather than fail parsing. Returns
/// `Err(GmlDimsMismatch)` instead when the coordinate text's token count
/// wasn't a multiple of the resolved `srsDimension` — a distinct anomaly the
/// caller should surface as bozo, unlike the other malformed-input cases
/// collapsed into `Ok(None)`.
///
/// # Errors
///
/// Returns `Err(GmlDimsMismatch)` when the coordinate text's token count
/// isn't a multiple of the resolved `srsDimension` — see above.
///
/// # Examples
///
/// ```
/// use feedparser_rs::namespace::georss::{GeoType, GmlDimsMismatch, build_gml_geometry};
///
/// let loc =
///     build_gml_geometry(GeoType::Point, Some("EPSG:4326".to_string()), "45.256 -71.92", 2);
/// assert_eq!(loc.unwrap().unwrap().coordinates[0], (45.256, -71.92));
///
/// // A projected (non-geographic) EPSG CRS uses (lon, lat) order and gets swapped;
/// // note projected coordinates are typically meters, not degrees (EPSG:3857 here).
/// let loc =
///     build_gml_geometry(GeoType::Point, Some("EPSG:3857".to_string()), "-8004866.0 5675670.0", 2);
/// assert_eq!(loc.unwrap().unwrap().coordinates[0], (5_675_670.0, -8_004_866.0));
///
/// // srsDimension="3": the third (elevation) value per tuple is dropped, not
/// // misaligned into the next tuple's latitude.
/// let loc = build_gml_geometry(GeoType::Line, None, "45.0 -71.0 10.0 46.0 -72.0 20.0", 3);
/// assert_eq!(
///     loc.unwrap().unwrap().coordinates,
///     vec![(45.0, -71.0), (46.0, -72.0)]
/// );
///
/// // 5 values isn't a multiple of dims=3 — a distinct bozo condition.
/// let result = build_gml_geometry(GeoType::Point, None, "45.0 -71.0 10.0 46.0 -72.0", 3);
/// assert_eq!(result, Err(GmlDimsMismatch));
/// ```
pub fn build_gml_geometry(
    geo_type: GeoType,
    srs_name: Option<String>,
    text: &str,
    dims: usize,
) -> Result<Option<GeoLocation>, GmlDimsMismatch> {
    let min_points = match geo_type {
        GeoType::Point => 1,
        GeoType::Line => 2,
        GeoType::Polygon => 3,
        GeoType::Box => return Ok(None),
    };

    let lat_lon_order = srs_uses_lat_lon_order(srs_name.as_deref());
    let coords = match parse_coordinates_ordered(text, lat_lon_order, dims) {
        CoordParse::Ok(coords) => coords,
        CoordParse::DimsMismatch => return Err(GmlDimsMismatch),
        CoordParse::Invalid => return Ok(None),
    };
    if coords.len() < min_points || (geo_type == GeoType::Point && coords.len() != 1) {
        return Ok(None);
    }

    Ok(Some(GeoLocation {
        geo_type,
        coordinates: coords,
        srs_name,
        ..Default::default()
    }))
}

/// Build a `GeoLocation` (`GeoType::Box`) from a `GeoRSS` GML profile
/// `gml:Envelope`.
///
/// `lower_text`/`upper_text` are the raw `gml:lowerCorner`/`gml:upperCorner`
/// coordinate text, each a single coordinate tuple; axis order is
/// normalized to `(latitude, longitude)` using `srs_name`, the same rule
/// [`build_gml_geometry`] applies to `gml:pos`/`gml:posList`. `dims` is
/// `gml:srsDimension` (`3` drops the elevation component per corner;
/// anything else means 2D).
///
/// Returns `Ok(None)` — the tolerant "bozo" pattern — if either corner's
/// text is malformed, out of range, or not a single coordinate tuple; the
/// caller should skip the geometry rather than fail parsing. Returns
/// `Err(GmlDimsMismatch)` instead when a corner's token count wasn't a
/// multiple of the resolved `srsDimension` — a distinct anomaly the caller
/// should surface as bozo, unlike the other malformed-input cases collapsed
/// into `Ok(None)`.
///
/// # Errors
///
/// Returns `Err(GmlDimsMismatch)` when a corner's token count isn't a
/// multiple of the resolved `srsDimension` — see above.
///
/// # Examples
///
/// ```
/// use feedparser_rs::namespace::georss::build_gml_envelope;
///
/// let loc = build_gml_envelope(None, "42.9 -71.9", "43.1 -71.5", 2);
/// assert_eq!(
///     loc.unwrap().unwrap().coordinates,
///     vec![(42.9, -71.9), (43.1, -71.5)]
/// );
/// ```
pub fn build_gml_envelope(
    srs_name: Option<String>,
    lower_text: &str,
    upper_text: &str,
    dims: usize,
) -> Result<Option<GeoLocation>, GmlDimsMismatch> {
    let lat_lon_order = srs_uses_lat_lon_order(srs_name.as_deref());
    let lower = match parse_coordinates_ordered(lower_text, lat_lon_order, dims) {
        CoordParse::Ok(coords) => coords,
        CoordParse::DimsMismatch => return Err(GmlDimsMismatch),
        CoordParse::Invalid => return Ok(None),
    };
    let upper = match parse_coordinates_ordered(upper_text, lat_lon_order, dims) {
        CoordParse::Ok(coords) => coords,
        CoordParse::DimsMismatch => return Err(GmlDimsMismatch),
        CoordParse::Invalid => return Ok(None),
    };

    if lower.len() != 1 || upper.len() != 1 {
        return Ok(None);
    }

    Ok(Some(GeoLocation {
        geo_type: GeoType::Box,
        coordinates: vec![lower[0], upper[0]],
        srs_name,
        ..Default::default()
    }))
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

    #[test]
    fn test_extract_epsg_code() {
        assert_eq!(extract_epsg_code("EPSG:4326"), Some(4326));
        assert_eq!(extract_epsg_code("urn:ogc:def:crs:EPSG::4326"), Some(4326));
        assert_eq!(
            extract_epsg_code("http://www.opengis.net/def/crs/EPSG/0/4326"),
            Some(4326)
        );
        // Classic GML 2 fragment form (#452).
        assert_eq!(
            extract_epsg_code("http://www.opengis.net/gml/srs/epsg.xml#3857"),
            Some(3857)
        );
        // XML attribute-value normalization of a line-wrapped attribute (#452).
        assert_eq!(extract_epsg_code(" EPSG:3857 "), Some(3857));
        assert_eq!(extract_epsg_code("http://www.opengis.net/gml"), None);
        assert_eq!(extract_epsg_code("not-a-crs"), None);
    }

    #[test]
    fn test_srs_uses_lat_lon_order() {
        assert!(srs_uses_lat_lon_order(None));
        assert!(srs_uses_lat_lon_order(Some("EPSG:4326")));
        assert!(srs_uses_lat_lon_order(Some("urn:ogc:def:crs:EPSG::4326")));
        assert!(!srs_uses_lat_lon_order(Some("EPSG:3857")));
        assert!(srs_uses_lat_lon_order(Some("some-custom-crs")));
        // CRS84 is WGS84 with (lon, lat) axis order despite no EPSG token (#454).
        assert!(!srs_uses_lat_lon_order(Some(
            "urn:ogc:def:crs:OGC:1.3:CRS84"
        )));
        assert!(!srs_uses_lat_lon_order(Some("OGC:CRS84")));
    }

    #[test]
    fn test_build_gml_geometry_point_epsg4326() {
        let loc = build_gml_geometry(
            GeoType::Point,
            Some("EPSG:4326".to_string()),
            "45.256 -71.92",
            2,
        );
        let loc = loc.unwrap().unwrap();
        assert_eq!(loc.geo_type, GeoType::Point);
        assert_eq!(loc.coordinates, vec![(45.256, -71.92)]);
        assert_eq!(loc.srs_name.as_deref(), Some("EPSG:4326"));
    }

    #[test]
    fn test_build_gml_geometry_point_no_srs_name_defaults_lat_lon() {
        let loc = build_gml_geometry(GeoType::Point, None, "45.256 -71.92", 2);
        let loc = loc.unwrap().unwrap();
        assert_eq!(loc.coordinates, vec![(45.256, -71.92)]);
        assert_eq!(loc.srs_name, None);
    }

    #[test]
    fn test_build_gml_geometry_swaps_projected_crs_realistic_meters() {
        // EPSG:3857 (Web Mercator) is not geographic: raw order is (lon, lat),
        // and real values are meters, not degrees — this must not be rejected
        // by the [-90,90]/[-180,180] degree-range check (#454 / issue S5).
        let loc = build_gml_geometry(
            GeoType::Point,
            Some("EPSG:3857".to_string()),
            "-8004866.0 5675670.0",
            2,
        );
        assert_eq!(
            loc.unwrap().unwrap().coordinates,
            vec![(5_675_670.0, -8_004_866.0)]
        );
    }

    #[test]
    fn test_build_gml_geometry_projected_crs_rejects_non_finite() {
        let result =
            build_gml_geometry(GeoType::Point, Some("EPSG:3857".to_string()), "NaN NaN", 2);
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn test_build_gml_geometry_linestring() {
        let loc = build_gml_geometry(
            GeoType::Line,
            Some("urn:ogc:def:crs:EPSG::4326".to_string()),
            "45.256 -71.92 46.0 -72.0",
            2,
        );
        let loc = loc.unwrap().unwrap();
        assert_eq!(loc.geo_type, GeoType::Line);
        assert_eq!(loc.coordinates.len(), 2);
    }

    #[test]
    fn test_build_gml_geometry_srs_dimension_3_drops_elevation() {
        // C1: dims=3 must chunk by 3 and drop the elevation component,
        // never let it corrupt the next tuple's latitude.
        let loc = build_gml_geometry(GeoType::Line, None, "45.0 -71.0 10.0 46.0 -72.0 20.0", 3);
        assert_eq!(
            loc.unwrap().unwrap().coordinates,
            vec![(45.0, -71.0), (46.0, -72.0)]
        );
    }

    #[test]
    fn test_build_gml_geometry_srs_dimension_mismatch_sets_bozo() {
        // 5 values isn't a multiple of dims=3 — must not silently misalign,
        // and must surface as bozo instead of being indistinguishable from a
        // feed with no GML geometry at all (#478).
        let result = build_gml_geometry(GeoType::Point, None, "45.0 -71.0 10.0 46.0 -72.0", 3);
        assert_eq!(result, Err(GmlDimsMismatch));
    }

    #[test]
    fn test_build_gml_geometry_comma_separated_coordinates() {
        let loc = build_gml_geometry(GeoType::Point, None, "45.256,-71.92", 2);
        assert_eq!(loc.unwrap().unwrap().coordinates, vec![(45.256, -71.92)]);
    }

    #[test]
    fn test_build_gml_geometry_polygon_too_few_points() {
        let result = build_gml_geometry(GeoType::Polygon, None, "45.0 -71.0 46.0 -71.0", 2);
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn test_build_gml_geometry_box_unsupported() {
        let result = build_gml_geometry(GeoType::Box, None, "45.0 -71.0 46.0 -71.0", 2);
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn test_build_gml_geometry_malformed_text() {
        assert_eq!(
            build_gml_geometry(GeoType::Point, None, "not numbers", 2),
            Ok(None)
        );
        assert_eq!(build_gml_geometry(GeoType::Point, None, "", 2), Ok(None));
    }

    #[test]
    fn test_build_gml_envelope() {
        let loc = build_gml_envelope(None, "42.9 -71.9", "43.1 -71.5", 2);
        let loc = loc.unwrap().unwrap();
        assert_eq!(loc.geo_type, GeoType::Box);
        assert_eq!(loc.coordinates, vec![(42.9, -71.9), (43.1, -71.5)]);
    }

    #[test]
    fn test_build_gml_envelope_swaps_projected_crs() {
        let loc = build_gml_envelope(
            Some("EPSG:3857".to_string()),
            "-8004866.0 5675670.0",
            "-8000000.0 5680000.0",
            2,
        );
        assert_eq!(
            loc.unwrap().unwrap().coordinates,
            vec![(5_675_670.0, -8_004_866.0), (5_680_000.0, -8_000_000.0)]
        );
    }

    #[test]
    fn test_build_gml_envelope_srs_dimension_3_drops_elevation() {
        let loc = build_gml_envelope(None, "42.9 -71.9 10.0", "43.1 -71.5 20.0", 3);
        assert_eq!(
            loc.unwrap().unwrap().coordinates,
            vec![(42.9, -71.9), (43.1, -71.5)]
        );
    }

    #[test]
    fn test_build_gml_envelope_malformed_corner() {
        assert_eq!(
            build_gml_envelope(None, "not numbers", "43.1 -71.5", 2),
            Ok(None)
        );
        assert_eq!(build_gml_envelope(None, "42.9 -71.9", "", 2), Ok(None));
    }

    #[test]
    fn test_build_gml_envelope_corner_wrong_arity() {
        // Each corner must be exactly one coordinate tuple.
        let result = build_gml_envelope(None, "42.9 -71.9 43.1 -71.5", "43.1 -71.5", 2);
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn test_build_gml_envelope_dims_mismatch_sets_bozo() {
        // Lower corner has 2 values, not a multiple of dims=3 (#478).
        let result = build_gml_envelope(None, "42.9 -71.9", "43.1 -71.5 20.0", 3);
        assert_eq!(result, Err(GmlDimsMismatch));
    }
}
