#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for `GeoRSS` and Creative Commons namespace parsing.
//!
//! Exercises the full `parse()` path end-to-end for:
//! - `GeoRSS` Simple (`georss:point`, `georss:polygon`, feed-level geo, invalid coordinates)
//! - Creative Commons (`creativeCommons:license`, `cc:license`, both on same field)

use feedparser_rs::namespace::georss::GeoType;
use feedparser_rs::parse;

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
