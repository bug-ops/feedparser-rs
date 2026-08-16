#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for `GeoRSS` and Creative Commons namespace parsing.
//!
//! Exercises the full `parse()` path end-to-end for:
//! - `GeoRSS` Simple (`georss:point`, `georss:polygon`, feed-level geo, invalid coordinates)
//! - Creative Commons (`creativeCommons:license`, `cc:license`, both on same field)

use feedparser_rs::namespace::georss::GeoType;
use feedparser_rs::parse;
use std::fmt::Write as _;

// ──────────────────────────────────────────────────────────────────────────────
// GeoRSS integration tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_georss_point_in_entry() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:georss="http://www.georss.org/georss">
        <channel>
            <title>Geo Feed</title>
            <link>http://example.com</link>
            <item>
                <title>Location Post</title>
                <link>http://example.com/1</link>
                <georss:point>45.256 -71.92</georss:point>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    assert_eq!(feed.entries.len(), 1);

    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.geo should be set");
    assert_eq!(geo.geo_type, GeoType::Point);
    assert_eq!(geo.coordinates.len(), 1);
    assert_eq!(geo.coordinates[0], (45.256, -71.92));
}

#[test]
fn test_georss_polygon_in_entry() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:georss="http://www.georss.org/georss">
        <channel>
            <title>Geo Feed</title>
            <link>http://example.com</link>
            <item>
                <title>Region Post</title>
                <link>http://example.com/2</link>
                <georss:polygon>45.0 -71.0 46.0 -71.0 46.0 -72.0 45.0 -71.0</georss:polygon>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);

    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.geo should be set");
    assert_eq!(geo.geo_type, GeoType::Polygon);
    assert_eq!(geo.coordinates.len(), 4);
    assert_eq!(geo.coordinates[0], (45.0, -71.0));
    assert_eq!(geo.coordinates[3], (45.0, -71.0)); // closed polygon
}

#[test]
fn test_georss_point_at_feed_level() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:georss="http://www.georss.org/georss">
        <channel>
            <title>Station Feed</title>
            <link>http://example.com</link>
            <georss:point>51.5074 -0.1278</georss:point>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);

    let geo = feed.feed.r#where.as_ref().expect("feed.geo should be set");
    assert_eq!(geo.geo_type, GeoType::Point);
    assert_eq!(geo.coordinates.len(), 1);
    assert_eq!(geo.coordinates[0], (51.5074, -0.1278));
}

#[test]
fn test_georss_invalid_coordinates_no_panic() {
    // Out-of-range latitude (> 90) must not panic and must leave geo as None.
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:georss="http://www.georss.org/georss">
        <channel>
            <title>Bad Geo Feed</title>
            <link>http://example.com</link>
            <item>
                <title>Post</title>
                <georss:point>999.0 -71.92</georss:point>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(
        feed.entries[0].r#where.is_none(),
        "invalid coordinates must produce None, not panic"
    );
}

#[test]
fn test_georss_malformed_text_no_panic() {
    // Non-numeric content must not panic and must leave geo as None.
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:georss="http://www.georss.org/georss">
        <channel>
            <title>Bad Geo Feed</title>
            <link>http://example.com</link>
            <item>
                <title>Post</title>
                <georss:point>not a number at all</georss:point>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(
        feed.entries[0].r#where.is_none(),
        "malformed coordinates must produce None, not panic"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Creative Commons integration tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_creative_commons_license_on_feed() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:creativeCommons="http://backend.userland.com/creativeCommonsRssModule">
        <channel>
            <title>CC Feed</title>
            <link>http://example.com</link>
            <creativeCommons:license>https://creativecommons.org/licenses/by/4.0/</creativeCommons:license>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    assert_eq!(
        feed.feed.license.as_deref(),
        Some("https://creativecommons.org/licenses/by/4.0/")
    );
}

#[test]
fn test_cc_license_on_entry() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:creativeCommons="http://backend.userland.com/creativeCommonsRssModule">
        <channel>
            <title>CC Entry Feed</title>
            <link>http://example.com</link>
            <item>
                <title>Licensed Post</title>
                <link>http://example.com/post/1</link>
                <creativeCommons:license>https://creativecommons.org/licenses/by-sa/4.0/</creativeCommons:license>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    assert_eq!(feed.entries.len(), 1);
    assert_eq!(
        feed.entries[0].license.as_deref(),
        Some("https://creativecommons.org/licenses/by-sa/4.0/")
    );
}

#[test]
fn test_cc_license_both_feed_and_entry() {
    // Both feed-level and entry-level licenses must be parsed independently.
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:creativeCommons="http://backend.userland.com/creativeCommonsRssModule">
        <channel>
            <title>Mixed CC Feed</title>
            <link>http://example.com</link>
            <creativeCommons:license>https://creativecommons.org/licenses/by/4.0/</creativeCommons:license>
            <item>
                <title>Post with own license</title>
                <link>http://example.com/post/2</link>
                <creativeCommons:license>https://creativecommons.org/licenses/by-nc/4.0/</creativeCommons:license>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    assert_eq!(
        feed.feed.license.as_deref(),
        Some("https://creativecommons.org/licenses/by/4.0/")
    );
    assert_eq!(
        feed.entries[0].license.as_deref(),
        Some("https://creativecommons.org/licenses/by-nc/4.0/")
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Combined namespace test
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_georss_and_cc_together() {
    // A feed that uses both GeoRSS and Creative Commons simultaneously.
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:creativeCommons="http://backend.userland.com/creativeCommonsRssModule">
        <channel>
            <title>Geo + CC Feed</title>
            <link>http://example.com</link>
            <georss:point>48.8566 2.3522</georss:point>
            <creativeCommons:license>https://creativecommons.org/licenses/by/4.0/</creativeCommons:license>
            <item>
                <title>Paris Event</title>
                <link>http://example.com/paris</link>
                <georss:point>48.8566 2.3522</georss:point>
                <creativeCommons:license>https://creativecommons.org/licenses/by-sa/4.0/</creativeCommons:license>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);

    // Feed-level assertions
    let feed_geo = feed.feed.r#where.as_ref().expect("feed.geo should be set");
    assert_eq!(feed_geo.geo_type, GeoType::Point);
    assert_eq!(feed_geo.coordinates[0], (48.8566, 2.3522));
    assert_eq!(
        feed.feed.license.as_deref(),
        Some("https://creativecommons.org/licenses/by/4.0/")
    );

    // Entry-level assertions
    assert_eq!(feed.entries.len(), 1);
    let entry_geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.geo should be set");
    assert_eq!(entry_geo.geo_type, GeoType::Point);
    assert_eq!(entry_geo.coordinates[0], (48.8566, 2.3522));
    assert_eq!(
        feed.entries[0].license.as_deref(),
        Some("https://creativecommons.org/licenses/by-sa/4.0/")
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Issue #291: georss:point/polygon not parsed in Atom entries
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_issue_291_atom_georss_point_in_entry() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <feed xmlns="http://www.w3.org/2005/Atom"
          xmlns:georss="http://www.georss.org/georss">
      <title>GeoRSS Atom Test</title>
      <id>urn:uuid:test</id>
      <updated>2024-01-01T00:00:00Z</updated>
      <entry>
        <id>urn:uuid:entry-1</id>
        <title>Entry with georss:point</title>
        <link href="https://example.com/entry-1"/>
        <updated>2024-01-01T00:00:00Z</updated>
        <georss:point>45.256 -71.92</georss:point>
      </entry>
    </feed>"#;

    let feed = parse(xml).expect("parse failed");
    assert!(!feed.bozo, "valid feed must not set bozo");
    assert_eq!(feed.entries.len(), 1);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(geo.geo_type, GeoType::Point);
    assert_eq!(geo.coordinates[0], (45.256, -71.92));
}

#[test]
fn test_issue_291_atom_georss_polygon_in_entry() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <feed xmlns="http://www.w3.org/2005/Atom"
          xmlns:georss="http://www.georss.org/georss">
      <title>GeoRSS Polygon Atom Test</title>
      <id>urn:uuid:test</id>
      <updated>2024-01-01T00:00:00Z</updated>
      <entry>
        <id>urn:uuid:entry-1</id>
        <title>Entry with georss:polygon</title>
        <link href="https://example.com/entry-1"/>
        <updated>2024-01-01T00:00:00Z</updated>
        <georss:polygon>45.0 -71.0 46.0 -71.0 46.0 -72.0 45.0 -71.0</georss:polygon>
      </entry>
    </feed>"#;

    let feed = parse(xml).expect("parse failed");
    assert!(!feed.bozo, "valid feed must not set bozo");
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(geo.geo_type, GeoType::Polygon);
    assert_eq!(geo.coordinates.len(), 4);
}

// ──────────────────────────────────────────────────────────────────────────────
// Issue #248: geo:lat/geo:long (W3C Basic Geo) not implemented
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_issue_248_rss_geo_latlong_in_entry() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <rss version="2.0" xmlns:geo="http://www.w3.org/2003/01/geo/wgs84_pos#">
      <channel>
        <title>W3C Geo Feed</title>
        <link>https://example.com</link>
        <description>Feed with geo:lat/geo:long</description>
        <item>
          <title>Item at location</title>
          <link>https://example.com/item-1</link>
          <geo:lat>51.5074</geo:lat>
          <geo:long>-0.1278</geo:long>
        </item>
      </channel>
    </rss>"#;

    let feed = parse(xml).expect("parse failed");
    assert!(!feed.bozo, "valid feed must not set bozo");
    assert_eq!(feed.entries.len(), 1);
    assert_eq!(feed.entries[0].geo_lat.as_deref(), Some("51.5074"));
    assert_eq!(feed.entries[0].geo_long.as_deref(), Some("-0.1278"));
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be auto-constructed");
    assert_eq!(geo.geo_type, GeoType::Point);
    assert!((geo.coordinates[0].0 - 51.5074).abs() < 1e-6);
    assert!((geo.coordinates[0].1 - (-0.1278)).abs() < 1e-6);
}

#[test]
fn test_issue_248_atom_geo_latlong_in_entry() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <feed xmlns="http://www.w3.org/2005/Atom"
          xmlns:geo="http://www.w3.org/2003/01/geo/wgs84_pos#">
      <title>W3C Geo Atom Feed</title>
      <id>urn:uuid:atom-geo-test</id>
      <updated>2024-01-01T00:00:00Z</updated>
      <entry>
        <id>urn:uuid:entry-1</id>
        <title>Entry with geo:lat and geo:long</title>
        <link href="https://example.com/entry-1"/>
        <updated>2024-01-01T00:00:00Z</updated>
        <geo:lat>48.8566</geo:lat>
        <geo:long>2.3522</geo:long>
      </entry>
    </feed>"#;

    let feed = parse(xml).expect("parse failed");
    assert!(!feed.bozo, "valid feed must not set bozo");
    assert_eq!(feed.entries.len(), 1);
    assert_eq!(feed.entries[0].geo_lat.as_deref(), Some("48.8566"));
    assert_eq!(feed.entries[0].geo_long.as_deref(), Some("2.3522"));
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be auto-constructed");
    assert_eq!(geo.geo_type, GeoType::Point);
    assert!((geo.coordinates[0].0 - 48.8566).abs() < 1e-6);
    assert!((geo.coordinates[0].1 - 2.3522).abs() < 1e-6);
}

#[test]
fn test_issue_248_geo_lat_only_no_where_constructed() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <rss version="2.0" xmlns:geo="http://www.w3.org/2003/01/geo/wgs84_pos#">
      <channel>
        <title>Feed</title>
        <link>https://example.com</link>
        <item>
          <title>Item with lat only</title>
          <geo:lat>51.5074</geo:lat>
        </item>
      </channel>
    </rss>"#;

    let feed = parse(xml).expect("parse failed");
    assert_eq!(feed.entries[0].geo_lat.as_deref(), Some("51.5074"));
    assert!(feed.entries[0].geo_long.is_none());
    assert!(
        feed.entries[0].r#where.is_none(),
        "where should not be constructed without both lat and long"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// GeoRSS extended attributes tests (issue #355)
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_issue_355_georss_extended_attrs_fixture() {
    let xml = include_bytes!("../../../tests/fixtures/georss_extended_attributes.xml");
    let feed = parse(xml).expect("parse failed");
    assert!(!feed.bozo);
    assert_eq!(feed.entries.len(), 4);

    // Item 1: Mountain Peak — all extended attrs + geometry
    let geo0 = feed.entries[0]
        .r#where
        .as_ref()
        .expect("item 1 should have geo");
    assert_eq!(geo0.geo_type, GeoType::Point);
    assert_eq!(geo0.coordinates[0], (45.256, -71.92));
    assert_eq!(geo0.elev, Some(1337.5));
    assert_eq!(geo0.feature_type_tag.as_deref(), Some("mountain"));
    assert_eq!(geo0.feature_name.as_deref(), Some("Mont Mégantic"));
    assert_eq!(geo0.relationship_tag.as_deref(), Some("is-located-at"));

    // Item 2: City — partial extended attrs
    let geo1 = feed.entries[1]
        .r#where
        .as_ref()
        .expect("item 2 should have geo");
    assert_eq!(geo1.feature_type_tag.as_deref(), Some("city"));
    assert_eq!(geo1.feature_name.as_deref(), Some("Paris"));
    assert!(geo1.elev.is_none());

    // Item 3: Metadata Only — no geometry, only feature_name
    let geo2 = feed.entries[2]
        .r#where
        .as_ref()
        .expect("item 3 should have geo");
    assert_eq!(geo2.feature_name.as_deref(), Some("Unknown Location"));
    assert!(geo2.coordinates.is_empty());

    // Item 4: Attrs Before Geometry — regression test for merge pattern (C1)
    let geo3 = feed.entries[3]
        .r#where
        .as_ref()
        .expect("item 4 should have geo");
    assert_eq!(geo3.geo_type, GeoType::Point);
    assert_eq!(geo3.coordinates[0], (40.0, -74.0));
    assert_eq!(geo3.feature_name.as_deref(), Some("Reverse Order"));
    assert_eq!(geo3.elev, Some(500.0));
}

// ──────────────────────────────────────────────────────────────────────────────
// Issue #454: GeoRSS GML profile (gml:Point/LineString/Polygon, srsName)
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_issue_454_gml_point_epsg4326_short_form() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>GML Point Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:Point srsName="EPSG:4326">
                        <gml:pos>45.256 -71.92</gml:pos>
                    </gml:Point>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(geo.geo_type, GeoType::Point);
    assert_eq!(geo.coordinates[0], (45.256, -71.92));
    assert_eq!(geo.srs_name.as_deref(), Some("EPSG:4326"));
}

#[test]
fn test_issue_454_gml_point_epsg4326_urn_form() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>GML Point Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:Point srsName="urn:ogc:def:crs:EPSG::4326">
                        <gml:pos>45.256 -71.92</gml:pos>
                    </gml:Point>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(geo.coordinates[0], (45.256, -71.92));
    assert_eq!(geo.srs_name.as_deref(), Some("urn:ogc:def:crs:EPSG::4326"));
}

#[test]
fn test_issue_454_gml_point_no_srs_name_defaults_wgs84() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>GML Point Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:Point>
                        <gml:pos>45.256 -71.92</gml:pos>
                    </gml:Point>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(geo.coordinates[0], (45.256, -71.92));
    assert!(geo.srs_name.is_none());
}

#[test]
fn test_issue_454_gml_linestring() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>GML Line Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:LineString srsName="EPSG:4326">
                        <gml:posList>45.256 -71.92 46.0 -72.0</gml:posList>
                    </gml:LineString>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(geo.geo_type, GeoType::Line);
    assert_eq!(geo.coordinates, vec![(45.256, -71.92), (46.0, -72.0)]);
}

#[test]
fn test_issue_454_gml_polygon_via_exterior_linear_ring() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>GML Polygon Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:Polygon srsName="EPSG:4326">
                        <gml:exterior>
                            <gml:LinearRing>
                                <gml:posList>45.0 -71.0 46.0 -71.0 46.0 -72.0 45.0 -71.0</gml:posList>
                            </gml:LinearRing>
                        </gml:exterior>
                    </gml:Polygon>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(geo.geo_type, GeoType::Polygon);
    assert_eq!(geo.coordinates.len(), 4);
    assert_eq!(geo.coordinates[0], (45.0, -71.0));
    assert_eq!(geo.coordinates[3], (45.0, -71.0));
}

#[test]
fn test_issue_454_gml_point_projected_crs_axis_swap() {
    // EPSG:3857 (Web Mercator) is a projected, non-geographic CRS: raw pos
    // order is (x, y) i.e. (lon-like, lat-like), so it must be swapped to
    // match this crate's (lat, lon) `GeoLocation::coordinates` convention.
    // Real EPSG:3857 values are meters, not degrees — must not be rejected
    // by lat/lon-range validation (issue #454 follow-up finding S5).
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>GML Point Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:Point srsName="EPSG:3857">
                        <gml:pos>-8004866.0 5675670.0</gml:pos>
                    </gml:Point>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(geo.coordinates[0], (5_675_670.0, -8_004_866.0));
    assert_eq!(geo.srs_name.as_deref(), Some("EPSG:3857"));
}

#[test]
fn test_issue_454_gml_srs_name_gml2_fragment_form() {
    // Classic GML 2 srsName form: "...epsg.xml#3857" (fragment, not colon-separated).
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>GML Point Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:Point srsName="http://www.opengis.net/gml/srs/epsg.xml#3857">
                        <gml:pos>-8004866.0 5675670.0</gml:pos>
                    </gml:Point>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(geo.coordinates[0], (5_675_670.0, -8_004_866.0));
}

#[test]
fn test_issue_454_gml_crs84_lon_lat_order() {
    // OGC:CRS84 is WGS84 with (lon, lat) axis order — the opposite of EPSG:4326.
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>GML Point Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:Point srsName="urn:ogc:def:crs:OGC:1.3:CRS84">
                        <gml:pos>-71.92 45.256</gml:pos>
                    </gml:Point>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(geo.coordinates[0], (45.256, -71.92));
}

#[test]
fn test_issue_454_gml_srs_dimension_3_drops_elevation() {
    // C1: srsDimension="3" must chunk gml:posList by 3 and drop the
    // elevation component per tuple, not misalign it into the next
    // tuple's latitude.
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>GML Line Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:LineString srsName="EPSG:4326" srsDimension="3">
                        <gml:posList>45.0 -71.0 10.0 46.0 -72.0 20.0</gml:posList>
                    </gml:LineString>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(geo.coordinates, vec![(45.0, -71.0), (46.0, -72.0)]);
}

#[test]
fn test_issue_454_gml_comma_separated_pos() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>GML Point Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:Point srsName="EPSG:4326">
                        <gml:pos>45.256,-71.92</gml:pos>
                    </gml:Point>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(geo.coordinates[0], (45.256, -71.92));
}

#[test]
fn test_issue_454_gml_pos_entity_error_sets_bozo() {
    // S3: an unresolvable entity inside gml:pos must set bozo, matching the
    // equivalent georss:point Simple-profile behavior.
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>GML Point Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:Point srsName="EPSG:4326">
                        <gml:pos>45.0 &bogus; -71.0</gml:pos>
                    </gml:Point>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(feed.bozo, "unresolvable entity in gml:pos must set bozo");
    // Regression (#478 S1): the unresolved entity leaves an odd token count
    // ("45.0", "&bogus;", "-71.0" = 3 tokens against the default dims=2), but
    // the real defect is the entity, not a srsDimension mismatch -- no
    // srsDimension attribute even appears in this feed. Must not be
    // misdiagnosed with the dims-mismatch description.
    assert_eq!(
        feed.bozo_exception.as_deref(),
        Some("Unresolvable entity in entry field")
    );
}

#[test]
fn test_issue_454_gml_malformed_pos_no_panic() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>GML Point Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:Point srsName="EPSG:4326">
                        <gml:pos>not numbers</gml:pos>
                    </gml:Point>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(
        feed.entries[0].r#where.is_none(),
        "malformed gml:pos text must produce None, not panic"
    );
}

#[test]
fn test_issue_454_gml_missing_pos_no_panic() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>Empty Geometry Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:Point srsName="EPSG:4326"/>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    assert!(
        feed.entries[0].r#where.is_none(),
        "geometry without gml:pos must produce None, not panic"
    );
}

#[test]
fn test_issue_454_gml_point_at_feed_level() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <georss:where>
                <gml:Point srsName="EPSG:4326">
                    <gml:pos>51.5074 -0.1278</gml:pos>
                </gml:Point>
            </georss:where>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed
        .feed
        .r#where
        .as_ref()
        .expect("feed.where should be set");
    assert_eq!(geo.geo_type, GeoType::Point);
    assert_eq!(geo.coordinates[0], (51.5074, -0.1278));
    assert_eq!(geo.srs_name.as_deref(), Some("EPSG:4326"));
}

#[test]
fn test_issue_454_gml_point_in_atom_entry() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <feed xmlns="http://www.w3.org/2005/Atom"
          xmlns:georss="http://www.georss.org/georss"
          xmlns:gml="http://www.opengis.net/gml">
      <title>GML Atom Test</title>
      <id>urn:uuid:test</id>
      <updated>2024-01-01T00:00:00Z</updated>
      <entry>
        <id>urn:uuid:entry-1</id>
        <title>Entry with GML point</title>
        <link href="https://example.com/entry-1"/>
        <updated>2024-01-01T00:00:00Z</updated>
        <georss:where>
            <gml:Point srsName="EPSG:4326">
                <gml:pos>45.256 -71.92</gml:pos>
            </gml:Point>
        </georss:where>
      </entry>
    </feed>"#;

    let feed = parse(xml).expect("parse failed");
    assert!(!feed.bozo, "valid feed must not set bozo");
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(geo.geo_type, GeoType::Point);
    assert_eq!(geo.coordinates[0], (45.256, -71.92));
    assert_eq!(geo.srs_name.as_deref(), Some("EPSG:4326"));
}

#[test]
fn test_issue_454_gml_point_in_rss10_item() {
    let xml = br#"<?xml version="1.0"?>
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns="http://purl.org/rss/1.0/"
             xmlns:georss="http://www.georss.org/georss"
             xmlns:gml="http://www.opengis.net/gml">
        <channel rdf:about="http://example.com/">
            <title>GML RSS 1.0 Feed</title>
            <link>http://example.com</link>
            <description>Feed with a GML point item</description>
        </channel>
        <item rdf:about="http://example.com/article1">
            <title>Article with location</title>
            <link>http://example.com/article1</link>
            <georss:where>
                <gml:Point srsName="EPSG:4326">
                    <gml:pos>45.256 -71.92</gml:pos>
                </gml:Point>
            </georss:where>
        </item>
    </rdf:RDF>"#;

    let feed = parse(xml).expect("parse failed");
    assert!(!feed.bozo, "valid feed must not set bozo");
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(geo.geo_type, GeoType::Point);
    assert_eq!(geo.coordinates[0], (45.256, -71.92));
    assert_eq!(geo.srs_name.as_deref(), Some("EPSG:4326"));
}

#[test]
fn test_issue_454_georss_simple_point_srs_name_still_none() {
    // Regression: GeoRSS Simple parsing must not start setting srs_name.
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0" xmlns:georss="http://www.georss.org/georss">
        <channel>
            <title>Geo Feed</title>
            <link>http://example.com</link>
            <item>
                <title>Location Post</title>
                <link>http://example.com/1</link>
                <georss:point>45.256 -71.92</georss:point>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(geo.coordinates[0], (45.256, -71.92));
    assert!(geo.srs_name.is_none());
}

#[test]
fn test_issue_454_gml_deeply_nested_sets_bozo_no_panic() {
    // Adversarial input: nest wrapper elements inside gml:Polygon past
    // max_nesting_depth so the GML tree-walk's recursive coordinate search
    // (find_gml_coord_text) must hit check_depth and bail out via bozo,
    // rather than recursing unboundedly or panicking.
    let mut xml = String::from(
        r#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>Deeply Nested GML</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:Polygon srsName="EPSG:4326">
                        <gml:exterior>"#,
    );

    for i in 0..150 {
        write!(&mut xml, "<gml:wrap{i}>").unwrap();
    }
    xml.push_str("<gml:posList>45.0 -71.0 46.0 -71.0 46.0 -72.0 45.0 -71.0</gml:posList>");
    for i in (0..150).rev() {
        write!(&mut xml, "</gml:wrap{i}>").unwrap();
    }

    xml.push_str(
        r"
                        </gml:exterior>
                    </gml:Polygon>
                </georss:where>
            </item>
        </channel>
    </rss>",
    );

    let feed = parse(xml.as_bytes()).expect("should handle deep GML nesting without panicking");

    // The item that hit the depth limit is dropped entirely (matches the
    // existing depth-limit error-handling contract), but the parse itself
    // must complete without panicking and must flag bozo.
    assert!(
        feed.bozo,
        "should set bozo flag for excessive GML nesting depth"
    );
    assert!(feed.entries.is_empty());
}

// ──────────────────────────────────────────────────────────────────────────────
// Issue #461: GeoRSS GML profile — gml:Envelope and gml:MultiSurface
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_issue_461_gml_envelope() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>GML Envelope Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:Envelope srsName="EPSG:4326">
                        <gml:lowerCorner>42.9 -71.9</gml:lowerCorner>
                        <gml:upperCorner>43.1 -71.5</gml:upperCorner>
                    </gml:Envelope>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(geo.geo_type, GeoType::Box);
    assert_eq!(geo.coordinates, vec![(42.9, -71.9), (43.1, -71.5)]);
    assert_eq!(geo.srs_name.as_deref(), Some("EPSG:4326"));
}

#[test]
fn test_issue_461_gml_multi_surface_wrapping_polygon() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>GML MultiSurface Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:MultiSurface srsName="EPSG:4326">
                        <gml:surfaceMember>
                            <gml:Polygon>
                                <gml:exterior>
                                    <gml:LinearRing>
                                        <gml:posList>45.0 -71.0 46.0 -71.0 46.0 -72.0 45.0 -71.0</gml:posList>
                                    </gml:LinearRing>
                                </gml:exterior>
                            </gml:Polygon>
                        </gml:surfaceMember>
                    </gml:MultiSurface>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(geo.geo_type, GeoType::Polygon);
    assert_eq!(geo.coordinates.len(), 4);
    assert_eq!(geo.coordinates[0], (45.0, -71.0));
    assert_eq!(geo.coordinates[3], (45.0, -71.0));
}

#[test]
fn test_issue_461_gml_envelope_missing_corner_no_panic() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>Incomplete Envelope</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:Envelope srsName="EPSG:4326">
                        <gml:lowerCorner>42.9 -71.9</gml:lowerCorner>
                    </gml:Envelope>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).expect("must not panic on missing upperCorner");
    assert!(
        feed.entries[0].r#where.is_none(),
        "envelope missing a corner must produce no geometry, not panic"
    );
}

#[test]
fn test_issue_461_gml_envelope_malformed_corner_no_panic() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>Malformed Envelope</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:Envelope srsName="EPSG:4326">
                        <gml:lowerCorner>not numbers</gml:lowerCorner>
                        <gml:upperCorner>43.1 -71.5</gml:upperCorner>
                    </gml:Envelope>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).expect("must not panic on malformed corner text");
    assert!(
        feed.entries[0].r#where.is_none(),
        "malformed corner text must produce no geometry, not panic"
    );
}

#[test]
fn test_issue_461_gml_envelope_srs_dimension_on_corner_elements() {
    // srsDimension can be placed directly on gml:lowerCorner/gml:upperCorner
    // rather than only on the gml:Envelope root — a common real-world
    // placement per the GML spec. It must still be honored.
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>3D Envelope Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:Envelope srsName="EPSG:4326">
                        <gml:lowerCorner srsDimension="3">42.9 -71.9 10.0</gml:lowerCorner>
                        <gml:upperCorner srsDimension="3">43.1 -71.5 20.0</gml:upperCorner>
                    </gml:Envelope>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(geo.geo_type, GeoType::Box);
    assert_eq!(geo.coordinates, vec![(42.9, -71.9), (43.1, -71.5)]);
}

#[test]
fn test_issue_461_gml_multi_surface_multiple_members_uses_first_only() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>GML MultiSurface Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:MultiSurface srsName="EPSG:4326">
                        <gml:surfaceMember>
                            <gml:Polygon>
                                <gml:exterior>
                                    <gml:LinearRing>
                                        <gml:posList>45.0 -71.0 46.0 -71.0 46.0 -72.0 45.0 -71.0</gml:posList>
                                    </gml:LinearRing>
                                </gml:exterior>
                            </gml:Polygon>
                        </gml:surfaceMember>
                        <gml:surfaceMember>
                            <gml:Polygon>
                                <gml:exterior>
                                    <gml:LinearRing>
                                        <gml:posList>10.0 -20.0 11.0 -20.0 11.0 -21.0 10.0 -20.0</gml:posList>
                                    </gml:LinearRing>
                                </gml:exterior>
                            </gml:Polygon>
                        </gml:surfaceMember>
                    </gml:MultiSurface>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(geo.geo_type, GeoType::Polygon);
    // Only the first surfaceMember's coordinates are used; the second
    // member's ring (10.0 -20.0 ...) is discarded.
    assert_eq!(geo.coordinates.len(), 4);
    assert_eq!(geo.coordinates[0], (45.0, -71.0));
    assert_eq!(geo.coordinates[3], (45.0, -71.0));
}

#[test]
fn test_issue_470_gml_pos_list_srs_dimension_on_element() {
    // srsDimension placed on gml:posList itself (the canonical GML
    // placement, and what real-world WFS/GeoServer/INSPIRE producers emit)
    // must be honored even though the gml:LineString root carries none.
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>3D LineString Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:LineString>
                        <gml:posList srsDimension="3">45.0 -71.0 5.0 46.0 -71.0 5.0 46.0 -72.0 5.0 45.0 -71.0 5.0</gml:posList>
                    </gml:LineString>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(
        geo.coordinates,
        vec![(45.0, -71.0), (46.0, -71.0), (46.0, -72.0), (45.0, -71.0)]
    );
}

#[test]
fn test_issue_470_gml_pos_srs_dimension_on_root_only() {
    // Regression check: srsDimension on the geometry root element (no
    // per-element override) must still work after threading dims through
    // find_gml_coord_text.
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>3D Point Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:Point srsDimension="3">
                        <gml:pos>45.256 -71.92 100.0</gml:pos>
                    </gml:Point>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(geo.coordinates[0], (45.256, -71.92));
}

#[test]
fn test_issue_470_gml_pos_list_srs_dimension_element_takes_precedence() {
    // When srsDimension appears on both the root element and gml:posList,
    // the per-element value must win.
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>Conflicting Dimension Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:LineString srsDimension="2">
                        <gml:posList srsDimension="3">45.0 -71.0 5.0 46.0 -71.0 5.0 46.0 -72.0 5.0 45.0 -71.0 5.0</gml:posList>
                    </gml:LineString>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(
        geo.coordinates,
        vec![(45.0, -71.0), (46.0, -71.0), (46.0, -72.0), (45.0, -71.0)]
    );
}

#[test]
fn test_issue_470_gml_pos_list_srs_dimension_defaults_to_2d() {
    // Neither root nor gml:posList specify srsDimension: default to 2D.
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>2D LineString Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:LineString>
                        <gml:posList>45.0 -71.0 46.0 -71.0 46.0 -72.0 45.0 -71.0</gml:posList>
                    </gml:LineString>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(
        geo.coordinates,
        vec![(45.0, -71.0), (46.0, -71.0), (46.0, -72.0), (45.0, -71.0)]
    );
}

#[test]
fn test_issue_470_gml_pos_srs_dimension_element_only_no_root() {
    // gml:pos carries srsDimension with no dims anywhere on the root.
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>3D Point Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:Point>
                        <gml:pos srsDimension="3">45.256 -71.92 100.0</gml:pos>
                    </gml:Point>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(geo.coordinates[0], (45.256, -71.92));
}

#[test]
fn test_issue_470_gml_polygon_exterior_linear_ring_pos_list_srs_dimension() {
    // srsDimension on the nested gml:posList must survive threading through
    // the gml:exterior/gml:LinearRing wrapper recursion.
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>3D Polygon Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:Polygon>
                        <gml:exterior>
                            <gml:LinearRing>
                                <gml:posList srsDimension="3">45.0 -71.0 5.0 46.0 -71.0 5.0 46.0 -72.0 5.0 45.0 -71.0 5.0</gml:posList>
                            </gml:LinearRing>
                        </gml:exterior>
                    </gml:Polygon>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(
        geo.coordinates,
        vec![(45.0, -71.0), (46.0, -71.0), (46.0, -72.0), (45.0, -71.0)]
    );
}

#[test]
fn test_issue_470_gml_polygon_linear_ring_srs_dimension_on_wrapper() {
    // srsDimension declared on the gml:LinearRing wrapper itself (not on
    // posList, not on the geometry root) must still be honored.
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>3D Polygon Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:Polygon>
                        <gml:exterior>
                            <gml:LinearRing srsDimension="3">
                                <gml:posList>45.0 -71.0 5.0 46.0 -71.0 5.0 46.0 -72.0 5.0 45.0 -71.0 5.0</gml:posList>
                            </gml:LinearRing>
                        </gml:exterior>
                    </gml:Polygon>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(
        geo.coordinates,
        vec![(45.0, -71.0), (46.0, -71.0), (46.0, -72.0), (45.0, -71.0)]
    );
}

#[test]
fn test_issue_470_gml_multi_surface_srs_dimension_on_innermost_pos_list() {
    // Deepest nesting from #461 (MultiSurface > surfaceMember > Polygon >
    // exterior > LinearRing > posList) with srsDimension on the innermost
    // posList must still resolve through every recursion level.
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>3D MultiSurface Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:MultiSurface>
                        <gml:surfaceMember>
                            <gml:Polygon>
                                <gml:exterior>
                                    <gml:LinearRing>
                                        <gml:posList srsDimension="3">45.0 -71.0 5.0 46.0 -71.0 5.0 46.0 -72.0 5.0 45.0 -71.0 5.0</gml:posList>
                                    </gml:LinearRing>
                                </gml:exterior>
                            </gml:Polygon>
                        </gml:surfaceMember>
                    </gml:MultiSurface>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set");
    assert_eq!(
        geo.coordinates,
        vec![(45.0, -71.0), (46.0, -71.0), (46.0, -72.0), (45.0, -71.0)]
    );
}

#[test]
fn test_issue_470_gml_pos_list_invalid_element_srs_dimension_falls_back_to_root() {
    // Out-of-range element-level srsDimension (only 2 and 3 are valid per
    // GML) must not silently override a valid root-level value -- it must
    // fall back to the inherited dims, not collapse to 2D.
    for invalid in ["0", "1", "4"] {
        let xml = format!(
            r#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>Invalid Dimension Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:LineString srsDimension="3">
                        <gml:posList srsDimension="{invalid}">45.0 -71.0 5.0 46.0 -71.0 5.0 46.0 -72.0 5.0 45.0 -71.0 5.0</gml:posList>
                    </gml:LineString>
                </georss:where>
            </item>
        </channel>
    </rss>"#
        );

        let feed = parse(xml.as_bytes()).unwrap();
        assert!(
            !feed.bozo,
            "invalid srsDimension={invalid} must not set bozo"
        );
        let geo = feed.entries[0]
            .r#where
            .as_ref()
            .expect("entry.where should be set");
        assert_eq!(
            geo.coordinates,
            vec![(45.0, -71.0), (46.0, -71.0), (46.0, -72.0), (45.0, -71.0)],
            "invalid srsDimension={invalid} must fall back to root dims=3, not clobber it"
        );
    }
}

#[test]
fn test_issue_470_gml_pos_list_malformed_element_srs_dimension_falls_back_to_root() {
    // Non-numeric element-level srsDimension must not panic and must fall
    // back to the inherited (root) dims.
    for malformed in ["abc", "3.5", "-3"] {
        let xml = format!(
            r#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>Malformed Dimension Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:LineString srsDimension="3">
                        <gml:posList srsDimension="{malformed}">45.0 -71.0 5.0 46.0 -71.0 5.0 46.0 -72.0 5.0 45.0 -71.0 5.0</gml:posList>
                    </gml:LineString>
                </georss:where>
            </item>
        </channel>
    </rss>"#
        );

        let feed = parse(xml.as_bytes()).expect("must not panic on malformed srsDimension");
        assert!(
            !feed.bozo,
            "malformed srsDimension={malformed} must not set bozo"
        );
        let geo = feed.entries[0]
            .r#where
            .as_ref()
            .expect("entry.where should be set");
        assert_eq!(
            geo.coordinates,
            vec![(45.0, -71.0), (46.0, -71.0), (46.0, -72.0), (45.0, -71.0)],
            "malformed srsDimension={malformed} must fall back to root dims=3"
        );
    }
}

#[test]
fn test_issue_470_gml_multi_surface_empty_first_member_srs_dimension_does_not_leak_to_sibling() {
    // Regression for a dims-leak found during review: the first
    // gml:surfaceMember's gml:Polygon declares srsDimension="3" but has no
    // actual coordinate text (an empty gml:LinearRing). That must not leak
    // dims=3 into the *sibling* gml:surfaceMember, whose plain 2D posList
    // carries no override of its own. Before the fix, the leaked dims=3
    // made the second member's 8-number posList fail the "len % dims == 0"
    // check (8 is not a multiple of 3), silently producing no geometry at
    // all (entry.where == None) with bozo still false.
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>MultiSurface Leak Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:MultiSurface>
                        <gml:surfaceMember>
                            <gml:Polygon srsDimension="3">
                                <gml:exterior>
                                    <gml:LinearRing>
                                    </gml:LinearRing>
                                </gml:exterior>
                            </gml:Polygon>
                        </gml:surfaceMember>
                        <gml:surfaceMember>
                            <gml:Polygon>
                                <gml:exterior>
                                    <gml:LinearRing>
                                        <gml:posList>45.0 -71.0 46.0 -71.0 46.0 -72.0 45.0 -71.0</gml:posList>
                                    </gml:LinearRing>
                                </gml:exterior>
                            </gml:Polygon>
                        </gml:surfaceMember>
                    </gml:MultiSurface>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(!feed.bozo);
    let geo = feed.entries[0]
        .r#where
        .as_ref()
        .expect("entry.where should be set -- dims must not leak from the empty first member");
    assert_eq!(
        geo.coordinates,
        vec![(45.0, -71.0), (46.0, -71.0), (46.0, -72.0), (45.0, -71.0)]
    );
}

#[test]
fn test_issue_478_gml_dims_mismatch_sets_bozo_in_entry() {
    // A posList length that isn't a multiple of the resolved srsDimension
    // must set bozo with a specific description, not silently leave
    // entry.where == None indistinguishable from "no GML geometry" (#478).
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>GML Line Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:LineString srsName="EPSG:4326" srsDimension="3">
                        <gml:posList>45.0 -71.0 10.0 46.0 -72.0</gml:posList>
                    </gml:LineString>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(feed.bozo, "srsDimension mismatch must set bozo");
    assert_eq!(
        feed.bozo_exception.as_deref(),
        Some("GML coordinate list length is not a multiple of resolved srsDimension")
    );
    assert!(
        feed.entries[0].r#where.is_none(),
        "mismatched coordinate count must not produce a geometry"
    );
}

#[test]
fn test_issue_478_gml_dims_mismatch_sets_bozo_at_feed_level() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <georss:where>
                <gml:LineString srsName="EPSG:4326" srsDimension="3">
                    <gml:posList>45.0 -71.0 10.0 46.0 -72.0</gml:posList>
                </gml:LineString>
            </georss:where>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(feed.bozo, "srsDimension mismatch must set bozo");
    assert_eq!(
        feed.bozo_exception.as_deref(),
        Some("GML coordinate list length is not a multiple of resolved srsDimension")
    );
    assert!(
        feed.feed.r#where.is_none(),
        "mismatched coordinate count must not produce a geometry"
    );
}

#[test]
fn test_issue_478_gml_envelope_dims_mismatch_sets_bozo() {
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>GML Envelope Post</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:Envelope srsName="EPSG:4326" srsDimension="3">
                        <gml:lowerCorner>42.9 -71.9</gml:lowerCorner>
                        <gml:upperCorner>43.1 -71.5 20.0</gml:upperCorner>
                    </gml:Envelope>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(feed.bozo, "srsDimension mismatch on a corner must set bozo");
    assert_eq!(
        feed.bozo_exception.as_deref(),
        Some("GML coordinate list length is not a multiple of resolved srsDimension")
    );
    assert!(feed.entries[0].r#where.is_none());
}

#[test]
fn test_issue_478_gml_dims_mismatch_sets_bozo_in_rss10_item() {
    let xml = br#"<?xml version="1.0"?>
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns="http://purl.org/rss/1.0/"
             xmlns:georss="http://www.georss.org/georss"
             xmlns:gml="http://www.opengis.net/gml">
        <channel rdf:about="http://example.com/">
            <title>GML RSS 1.0 Feed</title>
            <link>http://example.com</link>
            <description>Feed with a mismatched GML item</description>
        </channel>
        <item rdf:about="http://example.com/article1">
            <title>Article with location</title>
            <link>http://example.com/article1</link>
            <georss:where>
                <gml:LineString srsName="EPSG:4326" srsDimension="3">
                    <gml:posList>45.0 -71.0 10.0 46.0 -72.0</gml:posList>
                </gml:LineString>
            </georss:where>
        </item>
    </rdf:RDF>"#;

    let feed = parse(xml).unwrap();
    assert!(feed.bozo, "srsDimension mismatch must set bozo");
    assert_eq!(
        feed.bozo_exception.as_deref(),
        Some("GML coordinate list length is not a multiple of resolved srsDimension")
    );
    assert!(feed.entries[0].r#where.is_none());
}

#[test]
fn test_issue_478_gml_dims_mismatch_sets_bozo_at_rss10_feed_level() {
    let xml = br#"<?xml version="1.0"?>
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns="http://purl.org/rss/1.0/"
             xmlns:georss="http://www.georss.org/georss"
             xmlns:gml="http://www.opengis.net/gml">
        <channel rdf:about="http://example.com/">
            <title>GML RSS 1.0 Feed</title>
            <link>http://example.com</link>
            <description>Feed-level mismatched GML</description>
            <georss:where>
                <gml:LineString srsName="EPSG:4326" srsDimension="3">
                    <gml:posList>45.0 -71.0 10.0 46.0 -72.0</gml:posList>
                </gml:LineString>
            </georss:where>
        </channel>
    </rdf:RDF>"#;

    let feed = parse(xml).unwrap();
    assert!(feed.bozo, "srsDimension mismatch must set bozo");
    assert_eq!(
        feed.bozo_exception.as_deref(),
        Some("GML coordinate list length is not a multiple of resolved srsDimension")
    );
    assert!(feed.feed.r#where.is_none());
}

#[test]
fn test_issue_478_gml_dims_mismatch_sets_bozo_in_atom_entry() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <feed xmlns="http://www.w3.org/2005/Atom"
          xmlns:georss="http://www.georss.org/georss"
          xmlns:gml="http://www.opengis.net/gml">
      <title>GML Atom Test</title>
      <id>urn:uuid:test</id>
      <updated>2024-01-01T00:00:00Z</updated>
      <entry>
        <id>urn:uuid:entry-1</id>
        <title>Entry with mismatched GML</title>
        <link href="https://example.com/entry-1"/>
        <updated>2024-01-01T00:00:00Z</updated>
        <georss:where>
            <gml:LineString srsName="EPSG:4326" srsDimension="3">
                <gml:posList>45.0 -71.0 10.0 46.0 -72.0</gml:posList>
            </gml:LineString>
        </georss:where>
      </entry>
    </feed>"#;

    let feed = parse(xml).unwrap();
    assert!(feed.bozo, "srsDimension mismatch must set bozo");
    assert_eq!(
        feed.bozo_exception.as_deref(),
        Some("GML coordinate list length is not a multiple of resolved srsDimension")
    );
    assert!(feed.entries[0].r#where.is_none());
}

#[test]
fn test_issue_478_gml_dims_mismatch_sets_bozo_at_atom_feed_level() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <feed xmlns="http://www.w3.org/2005/Atom"
          xmlns:georss="http://www.georss.org/georss"
          xmlns:gml="http://www.opengis.net/gml">
      <title>GML Atom Test</title>
      <id>urn:uuid:test</id>
      <updated>2024-01-01T00:00:00Z</updated>
      <georss:where>
          <gml:LineString srsName="EPSG:4326" srsDimension="3">
              <gml:posList>45.0 -71.0 10.0 46.0 -72.0</gml:posList>
          </gml:LineString>
      </georss:where>
    </feed>"#;

    let feed = parse(xml).unwrap();
    assert!(feed.bozo, "srsDimension mismatch must set bozo");
    assert_eq!(
        feed.bozo_exception.as_deref(),
        Some("GML coordinate list length is not a multiple of resolved srsDimension")
    );
    assert!(feed.feed.r#where.is_none());
}

#[test]
fn test_issue_478_feed_level_entity_error_in_gml_pos_sets_bozo() {
    // Regression (#478 S2): feed/channel-level georss:where previously
    // discarded parse_georss_where's bozo signal entirely, so an
    // unresolvable entity in the coordinate text at feed level was silently
    // dropped -- exactly the #478 symptom, at the level the fix itself
    // touched.
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <georss:where>
                <gml:Point srsName="EPSG:4326">
                    <gml:pos>&bogus;45.0 -71.0</gml:pos>
                </gml:Point>
            </georss:where>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(
        feed.bozo,
        "unresolvable entity in feed-level gml:pos must set bozo"
    );
    assert!(feed.feed.r#where.is_none());
}

#[test]
fn test_issue_478_gml_dims_mismatch_wins_over_unrelated_entity_bozo_in_same_entry() {
    // Documents actual precedence (#478 M2): EntryCtx::bozo_reason is set
    // only by the GML dims-mismatch case, applied as an override over the
    // generic "unresolvable entity" fallback at flush time -- it is not a
    // general first-set-wins arbiter across every bozo cause in the entry.
    // Here the entity error in <title> happens first in document order, but
    // the GML-specific message still wins because it is the only writer of
    // `bozo_reason`.
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0"
         xmlns:georss="http://www.georss.org/georss"
         xmlns:gml="http://www.opengis.net/gml">
        <channel>
            <title>GML Feed</title>
            <link>http://example.com</link>
            <item>
                <title>Bad &bogus; Title</title>
                <link>http://example.com/1</link>
                <georss:where>
                    <gml:LineString srsName="EPSG:4326" srsDimension="3">
                        <gml:posList>45.0 -71.0 10.0 46.0 -72.0</gml:posList>
                    </gml:LineString>
                </georss:where>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    assert!(feed.bozo);
    assert_eq!(
        feed.bozo_exception.as_deref(),
        Some("GML coordinate list length is not a multiple of resolved srsDimension"),
        "GML-specific description must win over the generic entity-bozo fallback"
    );
}
