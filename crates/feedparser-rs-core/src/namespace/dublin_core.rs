/// Dublin Core Metadata Element Set (DCMES)
///
/// Namespace: <http://purl.org/dc/elements/1.1/>
/// Prefix: dc
///
/// This module provides parsing support for Dublin Core elements commonly
/// found in RSS 1.0 and other feeds.
///
/// Common mappings:
/// - `dc:creator` → author
/// - `dc:date` → updated/published
/// - `dc:subject` → tags
/// - `dc:description` → subtitle/summary
/// - `dc:publisher` → publisher
/// - `dc:rights` → rights
/// - `dc:title` → title (fallback if RSS/Atom title not present)
/// - `dc:language` → language
/// - `dc:identifier` → id (fallback)
use crate::types::{Entry, FeedMeta, Person, Tag};
use crate::util::date::parse_date;

/// Dublin Core namespace URI
pub const DC_NAMESPACE: &str = "http://purl.org/dc/elements/1.1/";

/// Handle Dublin Core element at feed level
///
/// # Arguments
///
/// * `element` - Local name of the element (without namespace prefix)
/// * `text` - Text content of the element
/// * `feed` - Feed metadata to update
pub fn handle_feed_element(element: &str, text: &str, feed: &mut FeedMeta) {
    match element {
        "creator" => {
            // dc:creator → author (always overrides)
            feed.author = Some(text.into());
            // Store in dc_creator field
            feed.dc_creator = Some(text.into());
            // Also add to authors list
            feed.authors.push(Person::from_name(text));
        }
        "date" => {
            // dc:date → updated (always overrides pubDate promotion)
            if let Some(dt) = parse_date(text) {
                feed.updated = Some(dt);
                feed.updated_str = Some(text.to_string());
            }
        }
        "subject" => {
            // dc:subject → tags
            feed.tags.push(Tag::new(text));
        }
        "description" => {
            // dc:description → subtitle (if not already set)
            if feed.subtitle.is_none() {
                feed.subtitle = Some(text.to_string());
            }
        }
        "publisher" => {
            // dc:publisher → publisher
            if feed.publisher.is_none() {
                feed.publisher = Some(text.into());
            }
            feed.dc_publisher = Some(text.into());
        }
        "rights" => {
            // dc:rights → rights (if not already set)
            if feed.rights.is_none() {
                feed.rights = Some(text.to_string());
            }
            feed.dc_rights = Some(text.to_string());
        }
        "title" => {
            // dc:title → title (fallback)
            if feed.title.is_none() {
                feed.title = Some(text.to_string());
            }
        }
        "language" => {
            // dc:language → language
            if feed.language.is_none() {
                feed.language = Some(text.into());
            }
        }
        "identifier" => {
            // dc:identifier → id (fallback)
            if feed.id.is_none() {
                feed.id = Some(text.to_string());
            }
        }
        "contributor" => {
            // dc:contributor → contributors
            feed.contributors.push(Person::from_name(text));
        }
        _ => {
            // Ignore unknown DC elements (source, type, format, coverage, etc.)
        }
    }
}

/// Handle Dublin Core element at entry level
///
/// # Arguments
///
/// * `element` - Local name of the element (without namespace prefix)
/// * `text` - Text content of the element
/// * `entry` - Entry to update
pub fn handle_entry_element(element: &str, text: &str, entry: &mut Entry) {
    match element {
        "creator" => {
            // dc:creator takes precedence over <author>; reconciled at </item>
            entry.dc_creator = Some(text.into());
            entry.authors.push(Person::from_name(text));
        }
        "date" => {
            if let Some(dt) = parse_date(text) {
                entry.dc_date = Some(dt);
                // dc:date is more authoritative than pubDate for updated; always override
                entry.updated = Some(dt);
                entry.updated_str = Some(text.to_string());
            }
        }
        "subject" => {
            entry.dc_subject.push(text.to_string());
            entry.tags.push(Tag::new(text));
        }
        "description" => {
            if entry.summary.is_none() {
                entry.summary = Some(text.to_string());
            }
        }
        "title" => {
            if entry.title.is_none() {
                entry.title = Some(text.to_string());
            }
        }
        "identifier" => {
            if entry.id.is_none() {
                entry.id = Some(text.into());
            }
        }
        "contributor" => {
            entry.contributors.push(Person::from_name(text));
        }
        "rights" => {
            if entry.rights.is_none() {
                entry.rights = Some(text.to_string());
            }
            entry.dc_rights = Some(text.to_string());
        }
        _ => {
            // Ignore unknown DC elements
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dc_creator_feed() {
        let mut feed = FeedMeta::default();
        handle_feed_element("creator", "John Doe", &mut feed);

        assert_eq!(feed.author.as_deref(), Some("John Doe"));
        assert_eq!(feed.authors.len(), 1);
        assert_eq!(feed.authors[0].name.as_deref(), Some("John Doe"));
    }

    #[test]
    fn test_dc_multiple_creators() {
        let mut feed = FeedMeta::default();
        handle_feed_element("creator", "Alice", &mut feed);
        handle_feed_element("creator", "Bob", &mut feed);

        // Last dc:creator wins (always overrides)
        assert_eq!(feed.author.as_deref(), Some("Bob"));
        // Both are in authors list
        assert_eq!(feed.authors.len(), 2);
    }

    #[test]
    fn test_dc_date() {
        let mut feed = FeedMeta::default();
        handle_feed_element("date", "2024-01-15T10:30:00Z", &mut feed);

        assert!(feed.updated.is_some());
    }

    #[test]
    fn test_dc_subject() {
        let mut feed = FeedMeta::default();
        handle_feed_element("subject", "Technology", &mut feed);
        handle_feed_element("subject", "Programming", &mut feed);

        assert_eq!(feed.tags.len(), 2);
        assert_eq!(feed.tags[0].term, "Technology");
        assert_eq!(feed.tags[1].term, "Programming");
    }

    #[test]
    fn test_dc_description() {
        let mut feed = FeedMeta::default();
        handle_feed_element("description", "Test description", &mut feed);

        assert_eq!(feed.subtitle.as_deref(), Some("Test description"));
    }

    #[test]
    fn test_dc_fallback_title() {
        let mut feed = FeedMeta::default();
        handle_feed_element("title", "DC Title", &mut feed);

        assert_eq!(feed.title.as_deref(), Some("DC Title"));

        // Set RSS title - should not be overwritten by DC
        feed.title = Some("RSS Title".to_string());
        handle_feed_element("title", "DC Title 2", &mut feed);

        assert_eq!(feed.title.as_deref(), Some("RSS Title"));
    }

    #[test]
    fn test_entry_dc_elements() {
        let mut entry = Entry::default();

        handle_entry_element("creator", "Jane Doe", &mut entry);
        handle_entry_element("subject", "Tech", &mut entry);
        handle_entry_element("description", "Entry summary", &mut entry);

        // dc:creator is stored in dc_creator; entry.author is reconciled at </item> in rss.rs
        assert_eq!(entry.dc_creator.as_deref(), Some("Jane Doe"));
        assert_eq!(entry.tags.len(), 1);
        assert_eq!(entry.summary.as_deref(), Some("Entry summary"));
    }

    #[test]
    fn test_entry_updated_from_dc_date() {
        let mut entry = Entry::default();
        handle_entry_element("date", "2024-01-15T10:30:00Z", &mut entry);

        assert!(entry.updated.is_some());
        assert!(entry.published.is_none());
    }

    #[test]
    fn test_entry_dc_rights_sets_both_fields() {
        let mut entry = Entry::default();
        handle_entry_element("rights", "Copyright 2024 Example", &mut entry);

        assert_eq!(entry.rights.as_deref(), Some("Copyright 2024 Example"));
        assert_eq!(entry.dc_rights.as_deref(), Some("Copyright 2024 Example"));
    }

    #[test]
    fn test_entry_dc_rights_does_not_overwrite_rights() {
        let mut entry = Entry {
            rights: Some("Atom rights".to_string()),
            ..Default::default()
        };
        handle_entry_element("rights", "DC rights", &mut entry);

        // dc:rights should not overwrite rights already set (e.g. by Atom <rights>)
        assert_eq!(entry.rights.as_deref(), Some("Atom rights"));
        assert_eq!(entry.dc_rights.as_deref(), Some("DC rights"));
    }
}
