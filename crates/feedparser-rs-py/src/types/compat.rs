use once_cell::sync::Lazy;
/// Python feedparser backward compatibility field mappings.
///
/// This module provides field alias mappings for deprecated Python feedparser field names.
/// Old field names map to new field names for backward compatibility.
///
/// Example: `feed.description` → `feed.subtitle`
///          `entry.guid` → `entry.id`
use std::collections::HashMap;

/// Feed-level field mappings: old name → list of new names (tried in order).
///
/// Some aliases can map to multiple fields (e.g., description → subtitle OR summary).
/// The resolver tries each new field in order until it finds a non-None value.
pub static FEED_FIELD_MAP: Lazy<HashMap<&'static str, Vec<&'static str>>> = Lazy::new(|| {
    let mut map = HashMap::new();

    // Description aliases
    map.insert("description", vec!["subtitle", "summary"]);
    map.insert(
        "description_detail",
        vec!["subtitle_detail", "summary_detail"],
    );

    // Tagline aliases (old Atom 0.3 field)
    map.insert("tagline", vec!["subtitle"]);
    map.insert("tagline_detail", vec!["subtitle_detail"]);

    // Info alias (RSS 1.0)
    map.insert("info", vec!["subtitle"]);
    map.insert("info_detail", vec!["subtitle_detail"]);

    // Copyright alias
    map.insert("copyright", vec!["rights"]);
    map.insert("copyright_detail", vec!["rights_detail"]);

    // Modified alias
    map.insert("modified", vec!["updated"]);
    map.insert("modified_parsed", vec!["updated_parsed"]);

    // Date alias (generic fallback)
    map.insert("date", vec!["updated", "published"]);
    map.insert("date_parsed", vec!["updated_parsed", "published_parsed"]);

    // URL alias
    map.insert("url", vec!["link"]);

    map
});

/// Entry-level field mappings: old name → list of new names (tried in order).
pub static ENTRY_FIELD_MAP: Lazy<HashMap<&'static str, Vec<&'static str>>> = Lazy::new(|| {
    let mut map = HashMap::new();

    // GUID alias (RSS)
    map.insert("guid", vec!["id"]);

    // Description alias
    map.insert("description", vec!["summary"]);
    map.insert("description_detail", vec!["summary_detail"]);

    // Issued alias (old feedparser field)
    map.insert("issued", vec!["published"]);
    map.insert("issued_parsed", vec!["published_parsed"]);

    // Modified alias
    map.insert("modified", vec!["updated"]);
    map.insert("modified_parsed", vec!["updated_parsed"]);

    // Date alias (generic fallback)
    map.insert("date", vec!["updated", "published"]);
    map.insert("date_parsed", vec!["updated_parsed", "published_parsed"]);

    map
});

/// Container-level field mappings for PyParsedFeed.
pub static CONTAINER_FIELD_MAP: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut map = HashMap::new();

    // RSS uses <channel>, Atom uses <feed>
    map.insert("channel", "feed");

    // RSS uses <item>, Atom uses <entry>
    map.insert("items", "entries");

    map
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feed_field_map_description() {
        let targets = FEED_FIELD_MAP.get("description").unwrap();
        assert_eq!(targets, &vec!["subtitle", "summary"]);
    }

    #[test]
    fn test_feed_field_map_modified() {
        let targets = FEED_FIELD_MAP.get("modified").unwrap();
        assert_eq!(targets, &vec!["updated"]);
    }

    #[test]
    fn test_entry_field_map_guid() {
        let targets = ENTRY_FIELD_MAP.get("guid").unwrap();
        assert_eq!(targets, &vec!["id"]);
    }

    #[test]
    fn test_entry_field_map_issued() {
        let targets = ENTRY_FIELD_MAP.get("issued").unwrap();
        assert_eq!(targets, &vec!["published"]);
    }

    #[test]
    fn test_container_field_map_channel() {
        let target = CONTAINER_FIELD_MAP.get("channel").unwrap();
        assert_eq!(*target, "feed");
    }
}
