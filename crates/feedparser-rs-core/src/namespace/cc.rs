//! Creative Commons namespace support for license information
//!
//! Handles Creative Commons license metadata in RSS and Atom feeds.
//! Supports both the modern `cc:license` (with `rdf:resource` attribute)
//! and legacy `creativeCommons:license` text elements.
//!
//! # Supported Elements
//!
//! - `cc:license` (with `rdf:resource` attribute) - Modern CC namespace
//! - `creativeCommons:license` (text element) - Legacy Userland namespace
//!
//! # Specification
//!
//! Creative Commons: <http://creativecommons.org/ns>
//! Legacy: <http://backend.userland.com/creativeCommonsRssModule>

use crate::Entry;
use crate::limits::ParserLimits;
use crate::types::generics::LimitedCollectionExt;
use crate::types::{FeedMeta, Link};

/// Creative Commons namespace URI (modern)
pub const CC: &str = "http://creativecommons.org/ns#";

/// Creative Commons legacy namespace URI (Userland)
pub const CREATIVE_COMMONS: &str = "http://backend.userland.com/creativeCommonsRssModule";

/// Handle Creative Commons element at feed level
///
/// Converts CC license information to a link with `rel="license"`
/// and adds it to the feed's links collection.
///
/// # Arguments
///
/// * `tag` - Element local name (e.g., "license")
/// * `attrs` - Element attributes as (name, value) pairs
/// * `text` - Element text content
/// * `feed` - Feed metadata to update
/// * `limits` - Parser limits for bounded collections
///
/// # Returns
///
/// `true` if element was recognized and handled, `false` otherwise
pub fn handle_feed_element(
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    text: &str,
    feed: &mut FeedMeta,
    limits: &ParserLimits,
) -> bool {
    match tag {
        b"license" => {
            if let Some(license_url) = extract_license_url(attrs, text) {
                feed.links.try_push_limited(
                    Link {
                        href: license_url.into(),
                        rel: Some("license".into()),
                        ..Default::default()
                    },
                    limits.max_links_per_feed,
                );
            }
            true
        }
        _ => false,
    }
}

/// Handle Creative Commons element at entry level
///
/// Converts CC license information to a link with `rel="license"`
/// and adds it to the entry's links collection.
///
/// # Arguments
///
/// * `tag` - Element local name (e.g., "license")
/// * `attrs` - Element attributes as (name, value) pairs
/// * `text` - Element text content
/// * `entry` - Entry to update
/// * `limits` - Parser limits for bounded collections
///
/// # Returns
///
/// `true` if element was recognized and handled, `false` otherwise
pub fn handle_entry_element(
    tag: &[u8],
    attrs: &[(Vec<u8>, String)],
    text: &str,
    entry: &mut Entry,
    limits: &ParserLimits,
) -> bool {
    match tag {
        b"license" => {
            if let Some(license_url) = extract_license_url(attrs, text) {
                entry.links.try_push_limited(
                    Link {
                        href: license_url.into(),
                        rel: Some("license".into()),
                        ..Default::default()
                    },
                    limits.max_links_per_entry,
                );
            }
            true
        }
        _ => false,
    }
}

/// Extract license URL from element
///
/// Tries two methods in order:
/// 1. `rdf:resource` attribute (modern cc:license format)
/// 2. Text content (legacy creativeCommons:license format)
///
/// # Arguments
///
/// * `attrs` - Element attributes
/// * `text` - Element text content
///
/// # Returns
///
/// License URL if found, `None` otherwise
fn extract_license_url(attrs: &[(Vec<u8>, String)], text: &str) -> Option<String> {
    // Try rdf:resource attribute first (modern format)
    // <cc:license rdf:resource="http://creativecommons.org/licenses/by/4.0/" />
    for (name, value) in attrs {
        if (name == b"resource" || name.ends_with(b":resource")) && !value.is_empty() {
            return Some(value.clone());
        }
    }

    // Fall back to text content (legacy format)
    // <creativeCommons:license>http://creativecommons.org/licenses/by/4.0/</creativeCommons:license>
    let trimmed = text.trim();
    if !trimmed.is_empty() {
        return Some(trimmed.to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_license_url_from_attribute() {
        let attrs = vec![(
            b"resource".to_vec(),
            "http://creativecommons.org/licenses/by/4.0/".to_string(),
        )];
        let url = extract_license_url(&attrs, "").unwrap();
        assert_eq!(url, "http://creativecommons.org/licenses/by/4.0/");
    }

    #[test]
    fn test_extract_license_url_from_namespaced_attribute() {
        let attrs = vec![(
            b"rdf:resource".to_vec(),
            "http://creativecommons.org/licenses/by-sa/4.0/".to_string(),
        )];
        let url = extract_license_url(&attrs, "").unwrap();
        assert_eq!(url, "http://creativecommons.org/licenses/by-sa/4.0/");
    }

    #[test]
    fn test_extract_license_url_from_text() {
        let url =
            extract_license_url(&[], "http://creativecommons.org/licenses/by-nc/4.0/").unwrap();
        assert_eq!(url, "http://creativecommons.org/licenses/by-nc/4.0/");
    }

    #[test]
    fn test_extract_license_url_from_text_with_whitespace() {
        let url =
            extract_license_url(&[], "  http://creativecommons.org/licenses/by-nd/4.0/  ").unwrap();
        assert_eq!(url, "http://creativecommons.org/licenses/by-nd/4.0/");
    }

    #[test]
    fn test_extract_license_url_prefers_attribute() {
        // If both attribute and text present, attribute wins
        let attrs = vec![(
            b"rdf:resource".to_vec(),
            "http://creativecommons.org/licenses/by/4.0/".to_string(),
        )];
        let url =
            extract_license_url(&attrs, "http://creativecommons.org/licenses/by-sa/4.0/").unwrap();
        assert_eq!(url, "http://creativecommons.org/licenses/by/4.0/");
    }

    #[test]
    fn test_extract_license_url_empty() {
        assert!(extract_license_url(&[], "").is_none());
        assert!(extract_license_url(&[], "   ").is_none());
    }

    #[test]
    fn test_handle_feed_element_license() {
        let mut feed = FeedMeta::default();
        let limits = ParserLimits::default();

        let attrs = vec![(
            b"rdf:resource".to_vec(),
            "http://creativecommons.org/licenses/by/4.0/".to_string(),
        )];

        let handled = handle_feed_element(b"license", &attrs, "", &mut feed, &limits);
        assert!(handled);
        assert_eq!(feed.links.len(), 1);
        assert_eq!(
            feed.links[0].href,
            "http://creativecommons.org/licenses/by/4.0/"
        );
        assert_eq!(feed.links[0].rel.as_deref(), Some("license"));
    }

    #[test]
    fn test_handle_entry_element_license() {
        let mut entry = Entry::default();
        let limits = ParserLimits::default();

        let handled = handle_entry_element(
            b"license",
            &[],
            "http://creativecommons.org/licenses/by-sa/4.0/",
            &mut entry,
            &limits,
        );
        assert!(handled);
        assert_eq!(entry.links.len(), 1);
        assert_eq!(
            entry.links[0].href,
            "http://creativecommons.org/licenses/by-sa/4.0/"
        );
        assert_eq!(entry.links[0].rel.as_deref(), Some("license"));
    }

    #[test]
    fn test_handle_feed_element_unknown() {
        let mut feed = FeedMeta::default();
        let limits = ParserLimits::default();

        let handled = handle_feed_element(b"unknown", &[], "", &mut feed, &limits);
        assert!(!handled);
    }

    #[test]
    fn test_handle_entry_element_unknown() {
        let mut entry = Entry::default();
        let limits = ParserLimits::default();

        let handled = handle_entry_element(b"unknown", &[], "", &mut entry, &limits);
        assert!(!handled);
    }

    #[test]
    fn test_multiple_licenses() {
        let mut feed = FeedMeta::default();
        let limits = ParserLimits::default();

        let attrs1 = vec![(
            b"rdf:resource".to_vec(),
            "http://creativecommons.org/licenses/by/4.0/".to_string(),
        )];
        handle_feed_element(b"license", &attrs1, "", &mut feed, &limits);

        let attrs2 = vec![(
            b"rdf:resource".to_vec(),
            "http://creativecommons.org/licenses/by-sa/4.0/".to_string(),
        )];
        handle_feed_element(b"license", &attrs2, "", &mut feed, &limits);

        assert_eq!(feed.links.len(), 2);
        assert_eq!(feed.links[0].rel.as_deref(), Some("license"));
        assert_eq!(feed.links[1].rel.as_deref(), Some("license"));
    }
}
