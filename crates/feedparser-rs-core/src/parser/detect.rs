//! Feed format detection from XML/JSON content

use crate::types::FeedVersion;
use quick_xml::{Reader, events::Event};

/// H1: Maximum size for JSON detection to prevent memory exhaustion
/// We only need to read the "version" field which is at the start
const MAX_JSON_DETECTION_SIZE: usize = 1024 * 1024; // 1MB

/// Auto-detect feed format from raw data
///
/// Examines the input data to determine the feed format by analyzing:
/// 1. Whether it's JSON (starts with `{`) → JSON Feed
/// 2. Root XML element name and attributes → RSS or Atom
///
/// # Arguments
///
/// * `data` - Raw feed data (XML or JSON)
///
/// # Returns
///
/// * `FeedVersion` - Detected format, or `Unknown` if unrecognized
///
/// # Examples
///
/// ```
/// use feedparser_rs::{detect_format, FeedVersion};
///
/// let rss = br#"<?xml version="1.0"?><rss version="2.0"></rss>"#;
/// assert_eq!(detect_format(rss), FeedVersion::Rss20);
///
/// let atom = br#"<feed xmlns="http://www.w3.org/2005/Atom"></feed>"#;
/// assert_eq!(detect_format(atom), FeedVersion::Atom10);
/// ```
#[must_use]
pub fn detect_format(data: &[u8]) -> FeedVersion {
    // Check for JSON Feed (starts with '{')
    let first_non_whitespace = data.iter().find(|&&b| !b.is_ascii_whitespace()).copied();

    if first_non_whitespace == Some(b'{') {
        return detect_json_feed_version(data);
    }

    // Parse XML to find root element
    detect_xml_format(data)
}

/// Detect JSON Feed version from JSON data
///
/// H1: Uses size limit to prevent memory exhaustion from large JSON files.
fn detect_json_feed_version(data: &[u8]) -> FeedVersion {
    // H1: Check size limit before parsing to prevent memory exhaustion
    if data.len() > MAX_JSON_DETECTION_SIZE {
        // For detection, we only need to find the "version" field
        // If file is too large, try to parse just the first chunk
        let truncated = &data[..MAX_JSON_DETECTION_SIZE];
        // Try to find version in truncated data using simple search
        // This is a fallback - if we can't detect, return Unknown
        return detect_json_version_from_partial(truncated);
    }

    // Try to parse as JSON and check version field
    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(data)
        && let Some(version) = json.get("version").and_then(|v| v.as_str())
    {
        return match version {
            "https://jsonfeed.org/version/1" => FeedVersion::JsonFeed10,
            "https://jsonfeed.org/version/1.1" => FeedVersion::JsonFeed11,
            _ => FeedVersion::Unknown,
        };
    }
    FeedVersion::Unknown
}

/// Fallback detection for large JSON files using string search
fn detect_json_version_from_partial(data: &[u8]) -> FeedVersion {
    // Simple byte search for version field patterns
    // This is a heuristic fallback for oversized JSON
    let data_str = std::str::from_utf8(data).unwrap_or("");

    if data_str.contains("https://jsonfeed.org/version/1.1") {
        FeedVersion::JsonFeed11
    } else if data_str.contains("https://jsonfeed.org/version/1") {
        FeedVersion::JsonFeed10
    } else {
        FeedVersion::Unknown
    }
}

/// Returns true if the data contains the Netscape RSS 0.91 DOCTYPE declaration.
///
/// Python feedparser uses this to distinguish `rss091n` (Netscape) from `rss091u` (Userland).
fn has_netscape_rss091_doctype(data: &[u8]) -> bool {
    // Only scan the first 512 bytes — DOCTYPE always appears before the root element
    let probe = &data[..data.len().min(512)];
    // The canonical Netscape DOCTYPE system identifier
    probe
        .windows(b"Netscape Communications".len())
        .any(|w| w == b"Netscape Communications")
}

/// Detect XML-based feed format (RSS or Atom)
fn detect_xml_format(data: &[u8]) -> FeedVersion {
    let has_netscape_doctype = has_netscape_rss091_doctype(data);

    let mut reader = Reader::from_reader(data);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();

    // Read events until we find the root element
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                let name = e.local_name();

                match name.as_ref() {
                    b"rss" => {
                        // Check version attribute
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"version" {
                                return match attr.value.as_ref() {
                                    b"0.90" => FeedVersion::Rss090,
                                    b"0.91" => {
                                        if has_netscape_doctype {
                                            FeedVersion::Rss091Netscape
                                        } else {
                                            FeedVersion::Rss091Userland
                                        }
                                    }
                                    b"0.92" => FeedVersion::Rss092,
                                    b"2.0" => FeedVersion::Rss20,
                                    _ => FeedVersion::Unknown,
                                };
                            }
                        }
                        // No version attribute, assume 2.0
                        return FeedVersion::Rss20;
                    }
                    b"rdf:RDF" | b"RDF" => {
                        // RSS 1.0 uses RDF
                        return FeedVersion::Rss10;
                    }
                    b"feed" => {
                        // Atom - check xmlns attribute
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"xmlns" {
                                let ns = attr.value.as_ref();
                                if ns == b"http://www.w3.org/2005/Atom" {
                                    return FeedVersion::Atom10;
                                } else if ns == b"http://purl.org/atom/ns#" {
                                    return FeedVersion::Atom03;
                                }
                            }
                        }
                        // No xmlns or unknown, assume Atom 1.0
                        return FeedVersion::Atom10;
                    }
                    _ => {
                        // Unknown root element
                        return FeedVersion::Unknown;
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => {
                // XML parsing error, can't detect
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    FeedVersion::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_rss20() {
        let xml = br#"<?xml version="1.0"?><rss version="2.0"></rss>"#;
        assert_eq!(detect_format(xml), FeedVersion::Rss20);
    }

    #[test]
    fn test_detect_rss20_no_version() {
        let xml = br#"<?xml version="1.0"?><rss></rss>"#;
        assert_eq!(detect_format(xml), FeedVersion::Rss20);
    }

    #[test]
    fn test_detect_rss091_userland() {
        let xml = br#"<rss version="0.91"></rss>"#;
        assert_eq!(detect_format(xml), FeedVersion::Rss091Userland);
    }

    #[test]
    fn test_detect_rss091_netscape() {
        let xml = br#"<?xml version="1.0"?>
<!DOCTYPE rss PUBLIC "-//Netscape Communications//DTD RSS 0.91//EN"
    "http://my.netscape.com/publish/formats/rss-0.91.dtd">
<rss version="0.91"></rss>"#;
        assert_eq!(detect_format(xml), FeedVersion::Rss091Netscape);
    }

    #[test]
    fn test_detect_rss092() {
        let xml = br#"<rss version="0.92"></rss>"#;
        assert_eq!(detect_format(xml), FeedVersion::Rss092);
    }

    #[test]
    fn test_detect_rss090() {
        let xml = br#"<rss version="0.90"></rss>"#;
        assert_eq!(detect_format(xml), FeedVersion::Rss090);
    }

    #[test]
    fn test_detect_rss10_rdf() {
        let xml = br#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"></rdf:RDF>"#;
        assert_eq!(detect_format(xml), FeedVersion::Rss10);
    }

    #[test]
    fn test_detect_rss10_rdf_uppercase() {
        let xml = br#"<RDF xmlns="http://www.w3.org/1999/02/22-rdf-syntax-ns#"></RDF>"#;
        assert_eq!(detect_format(xml), FeedVersion::Rss10);
    }

    #[test]
    fn test_detect_atom10() {
        let xml = br#"<feed xmlns="http://www.w3.org/2005/Atom"></feed>"#;
        assert_eq!(detect_format(xml), FeedVersion::Atom10);
    }

    #[test]
    fn test_detect_atom10_no_xmlns() {
        let xml = br"<feed></feed>";
        assert_eq!(detect_format(xml), FeedVersion::Atom10);
    }

    #[test]
    fn test_detect_atom03() {
        let xml = br#"<feed xmlns="http://purl.org/atom/ns#"></feed>"#;
        assert_eq!(detect_format(xml), FeedVersion::Atom03);
    }

    #[test]
    fn test_detect_json_feed_10() {
        let json = br#"{"version": "https://jsonfeed.org/version/1", "title": "Test"}"#;
        assert_eq!(detect_format(json), FeedVersion::JsonFeed10);
    }

    #[test]
    fn test_detect_json_feed_11() {
        let json = br#"{"version": "https://jsonfeed.org/version/1.1"}"#;
        assert_eq!(detect_format(json), FeedVersion::JsonFeed11);
    }

    #[test]
    fn test_detect_unknown_xml() {
        let xml = br"<unknown></unknown>";
        assert_eq!(detect_format(xml), FeedVersion::Unknown);
    }

    #[test]
    fn test_detect_invalid_xml() {
        let xml = b"not xml at all";
        assert_eq!(detect_format(xml), FeedVersion::Unknown);
    }

    #[test]
    fn test_detect_whitespace_before_json() {
        let json = b"  \n  {\"version\": \"https://jsonfeed.org/version/1.1\"}";
        assert_eq!(detect_format(json), FeedVersion::JsonFeed11);
    }

    #[test]
    fn test_detect_whitespace_before_xml() {
        let xml = b"  \n  <?xml version=\"1.0\"?><rss version=\"2.0\"></rss>";
        assert_eq!(detect_format(xml), FeedVersion::Rss20);
    }

    #[test]
    fn test_detect_empty_data() {
        let data = b"";
        assert_eq!(detect_format(data), FeedVersion::Unknown);
    }

    #[test]
    fn test_detect_json_version_from_partial() {
        // Test the fallback detection using string search
        use super::detect_json_version_from_partial;

        let json_11 = br#"{"version": "https://jsonfeed.org/version/1.1", "title": "Test"}"#;
        assert_eq!(
            detect_json_version_from_partial(json_11),
            FeedVersion::JsonFeed11
        );

        let json_10 = br#"{"version": "https://jsonfeed.org/version/1", "title": "Test"}"#;
        assert_eq!(
            detect_json_version_from_partial(json_10),
            FeedVersion::JsonFeed10
        );

        let unknown = br#"{"title": "No version field"}"#;
        assert_eq!(
            detect_json_version_from_partial(unknown),
            FeedVersion::Unknown
        );
    }
}
