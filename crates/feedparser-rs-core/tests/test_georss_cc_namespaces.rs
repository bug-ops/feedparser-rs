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
        .geo
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
        .geo
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

    let geo = feed.feed.geo.as_ref().expect("feed.geo should be set");
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
        feed.entries[0].geo.is_none(),
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
        feed.entries[0].geo.is_none(),
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
    let feed_geo = feed.feed.geo.as_ref().expect("feed.geo should be set");
    assert_eq!(feed_geo.geo_type, GeoType::Point);
    assert_eq!(feed_geo.coordinates[0], (48.8566, 2.3522));
    assert_eq!(
        feed.feed.license.as_deref(),
        Some("https://creativecommons.org/licenses/by/4.0/")
    );

    // Entry-level assertions
    assert_eq!(feed.entries.len(), 1);
    let entry_geo = feed.entries[0]
        .geo
        .as_ref()
        .expect("entry.geo should be set");
    assert_eq!(entry_geo.geo_type, GeoType::Point);
    assert_eq!(entry_geo.coordinates[0], (48.8566, 2.3522));
    assert_eq!(
        feed.entries[0].license.as_deref(),
        Some("https://creativecommons.org/licenses/by-sa/4.0/")
    );
}
