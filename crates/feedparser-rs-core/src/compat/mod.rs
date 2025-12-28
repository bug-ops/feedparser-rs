//! Compatibility utilities for feedparser API
//!
//! This module provides utilities to ensure API compatibility with
//! Python's feedparser library.

use crate::types::FeedVersion;

/// Normalize feed type string to Python feedparser format
///
/// Converts version enum to Python feedparser-compatible string format:
/// - "RSS 2.0" -> "rss20"
/// - "Atom 1.0" -> "atom10"
/// - etc.
///
/// # Arguments
///
/// * `version` - Feed version to normalize
///
/// # Returns
///
/// Normalized version string compatible with Python feedparser
///
/// # Examples
///
/// ```
/// use feedparser_rs::{compat::normalize_version, FeedVersion};
///
/// assert_eq!(normalize_version(FeedVersion::Rss20), "rss20");
/// assert_eq!(normalize_version(FeedVersion::Atom10), "atom10");
/// assert_eq!(normalize_version(FeedVersion::Unknown), "");
/// ```
#[must_use]
pub fn normalize_version(version: FeedVersion) -> String {
    version.as_str().to_string()
}

/// Convert duration in seconds to HH:MM:SS format
///
/// Formats duration for display in podcast feeds and other contexts
/// where human-readable time format is needed.
///
/// # Arguments
///
/// * `seconds` - Duration in seconds
///
/// # Returns
///
/// Duration string in HH:MM:SS format
///
/// # Examples
///
/// ```
/// use feedparser_rs::compat::format_duration;
///
/// assert_eq!(format_duration(0), "0:00:00");
/// assert_eq!(format_duration(90), "0:01:30");
/// assert_eq!(format_duration(3661), "1:01:01");
/// assert_eq!(format_duration(36000), "10:00:00");
/// ```
#[must_use]
pub fn format_duration(seconds: u32) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    format!("{hours}:{minutes:02}:{secs:02}")
}

/// Check if a string is a valid feed version identifier
///
/// Validates whether a version string matches one of the known
/// feed format versions supported by feedparser.
///
/// # Arguments
///
/// * `version` - Version string to validate
///
/// # Returns
///
/// `true` if version is valid, `false` otherwise
///
/// # Examples
///
/// ```
/// use feedparser_rs::compat::is_valid_version;
///
/// assert!(is_valid_version("rss20"));
/// assert!(is_valid_version("atom10"));
/// assert!(is_valid_version("json11"));
/// assert!(!is_valid_version("invalid"));
/// assert!(!is_valid_version(""));
/// ```
#[must_use]
pub fn is_valid_version(version: &str) -> bool {
    matches!(
        version,
        "rss090"
            | "rss091"
            | "rss092"
            | "rss10"
            | "rss20"
            | "atom03"
            | "atom10"
            | "json10"
            | "json11"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_version() {
        assert_eq!(normalize_version(FeedVersion::Rss20), "rss20");
        assert_eq!(normalize_version(FeedVersion::Rss10), "rss10");
        assert_eq!(normalize_version(FeedVersion::Atom10), "atom10");
        assert_eq!(normalize_version(FeedVersion::Atom03), "atom03");
        assert_eq!(normalize_version(FeedVersion::JsonFeed10), "json10");
        assert_eq!(normalize_version(FeedVersion::JsonFeed11), "json11");
        assert_eq!(normalize_version(FeedVersion::Unknown), "");
    }

    #[test]
    fn test_format_duration_zero() {
        assert_eq!(format_duration(0), "0:00:00");
    }

    #[test]
    fn test_format_duration_seconds_only() {
        assert_eq!(format_duration(30), "0:00:30");
        assert_eq!(format_duration(59), "0:00:59");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(60), "0:01:00");
        assert_eq!(format_duration(90), "0:01:30");
        assert_eq!(format_duration(150), "0:02:30");
        assert_eq!(format_duration(3599), "0:59:59");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(3600), "1:00:00");
        assert_eq!(format_duration(3661), "1:01:01");
        assert_eq!(format_duration(7200), "2:00:00");
        assert_eq!(format_duration(36000), "10:00:00");
    }

    #[test]
    fn test_format_duration_large() {
        assert_eq!(format_duration(86399), "23:59:59");
        assert_eq!(format_duration(86400), "24:00:00");
        assert_eq!(format_duration(90061), "25:01:01");
    }

    #[test]
    fn test_is_valid_version_valid() {
        assert!(is_valid_version("rss090"));
        assert!(is_valid_version("rss091"));
        assert!(is_valid_version("rss092"));
        assert!(is_valid_version("rss10"));
        assert!(is_valid_version("rss20"));
        assert!(is_valid_version("atom03"));
        assert!(is_valid_version("atom10"));
        assert!(is_valid_version("json10"));
        assert!(is_valid_version("json11"));
    }

    #[test]
    fn test_is_valid_version_invalid() {
        assert!(!is_valid_version(""));
        assert!(!is_valid_version("invalid"));
        assert!(!is_valid_version("rss30"));
        assert!(!is_valid_version("atom20"));
        assert!(!is_valid_version("RSS20")); // Case sensitive
        assert!(!is_valid_version("json12"));
        assert!(!is_valid_version("rdf"));
    }

    #[test]
    fn test_is_valid_version_edge_cases() {
        assert!(!is_valid_version(" rss20"));
        assert!(!is_valid_version("rss20 "));
        assert!(!is_valid_version("rss 20"));
    }
}
