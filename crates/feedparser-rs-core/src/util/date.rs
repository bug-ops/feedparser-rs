//! Multi-format date parsing for RSS and Atom feeds

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

/// Date format strings to try, in priority order
///
/// Order matters: more specific formats first, then fallbacks
const DATE_FORMATS: &[&str] = &[
    // ISO 8601 / RFC 3339 variants (Atom)
    "%Y-%m-%dT%H:%M:%S%.f%:z", // 2024-12-14T10:30:45.123+00:00
    "%Y-%m-%dT%H:%M:%S%:z",    // 2024-12-14T10:30:45+00:00
    "%Y-%m-%dT%H:%M:%S%.fZ",   // 2024-12-14T10:30:45.123Z
    "%Y-%m-%dT%H:%M:%SZ",      // 2024-12-14T10:30:45Z
    "%Y-%m-%dT%H:%M:%S",       // 2024-12-14T10:30:45 (no timezone)
    "%Y-%m-%d %H:%M:%S",       // 2024-12-14 10:30:45
    "%Y-%m-%d",                // 2024-12-14
    // W3C Date-Time variants
    "%Y-%m-%d %H:%M:%S%:z", // 2024-12-14 10:30:45+00:00
    "%Y/%m/%d %H:%M:%S",    // 2024/12/14 10:30:45
    "%Y/%m/%d",             // 2024/12/14
    // RFC 822 variants (RSS pubDate)
    "%d %b %Y %H:%M:%S", // 14 Dec 2024 10:30:45
    "%d %b %Y",          // 14 Dec 2024
    "%d %B %Y %H:%M:%S", // 14 December 2024 10:30:45
    "%d %B %Y",          // 14 December 2024
    // US date formats
    "%B %d, %Y %H:%M:%S", // December 14, 2024 10:30:45
    "%B %d, %Y",          // December 14, 2024
    "%b %d, %Y %H:%M:%S", // Dec 14, 2024 10:30:45
    "%b %d, %Y",          // Dec 14, 2024
    "%m/%d/%Y %H:%M:%S",  // 12/14/2024 10:30:45
    "%m/%d/%Y",           // 12/14/2024
    "%m-%d-%Y",           // 12-14-2024
    // EU date formats
    "%d.%m.%Y %H:%M:%S", // 14.12.2024 10:30:45
    "%d.%m.%Y",          // 14.12.2024
    "%d/%m/%Y %H:%M:%S", // 14/12/2024 10:30:45
    "%d/%m/%Y",          // 14/12/2024
    "%d-%b-%Y",          // 14-Dec-2024
    "%d-%B-%Y",          // 14-December-2024
];

/// Parse ASCTIME format: `Www Mmm [D]D HH:MM:SS YYYY` where DD may have a leading space.
///
/// Example: `Mon Jan  6 12:30:00 2025` or `Mon Jan 16 12:30:00 2025`
fn parse_asctime(s: &str) -> Option<NaiveDateTime> {
    // Expected: 3 alpha (weekday) + space + 3 alpha (month) + space + 1-2 digit day
    //           (possibly space-padded) + space + HH:MM:SS + space + YYYY
    let b = s.as_bytes();
    if b.len() < 24 {
        return None;
    }
    // Weekday: bytes 0..3 must be alpha
    if !b[..3].iter().all(u8::is_ascii_alphabetic) || b[3] != b' ' {
        return None;
    }
    // Strip weekday prefix
    let rest = &s[4..];
    // Normalize: collapse double-space before single-digit day to single space
    // "Jan  6" → "Jan 6"
    let normalized = if rest.len() > 4 && rest.as_bytes()[4] == b' ' && rest.as_bytes()[3] == b' ' {
        // "Mmm  D ..." → "Mmm D ..."
        let mut n = String::with_capacity(rest.len());
        n.push_str(&rest[..3]); // month
        n.push(' ');
        n.push_str(rest[4..].trim_start_matches(' '));
        n
    } else {
        rest.to_string()
    };
    NaiveDateTime::parse_from_str(&normalized, "%b %e %H:%M:%S %Y")
        .or_else(|_| NaiveDateTime::parse_from_str(&normalized, "%b %d %H:%M:%S %Y"))
        .ok()
}

/// Strip a leading weekday prefix of the form `"Www, "` (3 ASCII alpha chars + ", ").
fn strip_weekday_prefix(s: &str) -> Option<&str> {
    let b = s.as_bytes();
    if b.len() > 5 && b[3] == b',' && b[4] == b' ' && b[..3].iter().all(u8::is_ascii_alphabetic) {
        Some(&s[5..])
    } else {
        None
    }
}

/// Parse date from string, trying multiple formats
///
/// This function attempts to parse dates in the following order:
/// 1. RFC 3339 (Atom standard: 2024-12-14T10:30:00Z)
/// 2. RFC 2822 (RSS standard: Sat, 14 Dec 2024 10:30:00 +0000)
/// 3. Common format strings (ISO 8601 variants, US/EU formats)
///
/// # Arguments
///
/// * `input` - Date string to parse
///
/// # Returns
///
/// * `Some(DateTime<Utc>)` - Successfully parsed date
/// * `None` - Could not parse date
///
/// # Examples
///
/// ```
/// use feedparser_rs::util::date::parse_date;
///
/// // RFC 3339 (Atom)
/// assert!(parse_date("2024-12-14T10:30:00Z").is_some());
///
/// // RFC 2822 (RSS)
/// assert!(parse_date("Sat, 14 Dec 2024 10:30:00 +0000").is_some());
///
/// // ISO 8601 date-only
/// assert!(parse_date("2024-12-14").is_some());
///
/// // Invalid date
/// assert!(parse_date("not a date").is_none());
/// ```
#[must_use]
pub fn parse_date(input: &str) -> Option<DateTime<Utc>> {
    let input = input.trim();

    if input.is_empty() {
        return None;
    }

    // Try RFC 3339 first (most common in Atom)
    if let Ok(dt) = DateTime::parse_from_rfc3339(input) {
        return Some(dt.with_timezone(&Utc));
    }

    // Try RFC 2822 (RSS pubDate format)
    if let Ok(dt) = DateTime::parse_from_rfc2822(input) {
        return Some(dt.with_timezone(&Utc));
    }

    // Retry RFC 2822 with weekday prefix stripped — chrono validates the weekday
    // strictly, but Python feedparser accepts wrong day-of-week names (#143)
    if let Some(stripped) = strip_weekday_prefix(input)
        && let Ok(dt) = DateTime::parse_from_rfc2822(stripped)
    {
        return Some(dt.with_timezone(&Utc));
    }

    // Special handling for year-only format (e.g., "2024")
    if let Ok(year) = input.parse::<i32>()
        && (1000..=9999).contains(&year)
    {
        return NaiveDate::from_ymd_opt(year, 1, 1)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| dt.and_utc());
    }

    // Special handling for year-month format (e.g., "2024-12")
    if input.len() == 7
        && input.chars().nth(4) == Some('-')
        && let (Ok(year), Ok(month)) = (input[..4].parse::<i32>(), input[5..7].parse::<u32>())
        && (1000..=9999).contains(&year)
        && (1..=12).contains(&month)
    {
        return NaiveDate::from_ymd_opt(year, month, 1)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| dt.and_utc());
    }

    // Try ASCTIME format: "Mon Jan  6 12:30:00 2025"
    if let Some(dt) = parse_asctime(input) {
        return Some(dt.and_utc());
    }

    // Try all format strings
    for fmt in DATE_FORMATS {
        // Try parsing with time component
        if let Ok(dt) = NaiveDateTime::parse_from_str(input, fmt) {
            return Some(dt.and_utc());
        }

        // Try parsing date-only, assume midnight UTC
        if let Ok(date) = NaiveDate::parse_from_str(input, fmt) {
            return date.and_hms_opt(0, 0, 0).map(|dt| dt.and_utc());
        }
    }

    // Could not parse
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_rfc3339_with_timezone() {
        let dt = parse_date("2024-12-14T10:30:00+00:00");
        assert!(dt.is_some());
        let dt = dt.unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 14);
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
    }

    #[test]
    fn test_rfc3339_z_suffix() {
        let dt = parse_date("2024-12-14T10:30:00Z");
        assert!(dt.is_some());
        assert_eq!(dt.unwrap().year(), 2024);
    }

    #[test]
    fn test_rfc3339_with_milliseconds() {
        let dt = parse_date("2024-12-14T10:30:00.123Z");
        assert!(dt.is_some());
    }

    #[test]
    fn test_rfc2822_format() {
        let dt = parse_date("Sat, 14 Dec 2024 10:30:00 +0000");
        assert!(dt.is_some());
        let dt = dt.unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 12);
    }

    #[test]
    fn test_rfc2822_gmt() {
        let dt = parse_date("Sat, 14 Dec 2024 10:30:00 GMT");
        assert!(dt.is_some());
    }

    #[test]
    fn test_iso8601_date_only() {
        let dt = parse_date("2024-12-14");
        assert!(dt.is_some());
        let dt = dt.unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 14);
        assert_eq!(dt.hour(), 0); // Midnight
    }

    #[test]
    fn test_us_format_long_month() {
        let dt = parse_date("December 14, 2024");
        assert!(dt.is_some());
    }

    #[test]
    fn test_us_format_short_month() {
        let dt = parse_date("Dec 14, 2024");
        assert!(dt.is_some());
    }

    #[test]
    fn test_invalid_date() {
        let dt = parse_date("not a date");
        assert!(dt.is_none());
    }

    #[test]
    fn test_empty_string() {
        let dt = parse_date("");
        assert!(dt.is_none());
    }

    #[test]
    fn test_whitespace_only() {
        let dt = parse_date("   ");
        assert!(dt.is_none());
    }

    #[test]
    fn test_partial_date_invalid() {
        // Invalid partial dates should fail
        let dt = parse_date("2024-13"); // Invalid month
        assert!(dt.is_none());
        let dt = parse_date("abcd-12");
        assert!(dt.is_none());
    }

    #[test]
    fn test_us_date_slash_format() {
        let dt = parse_date("12/14/2024");
        assert!(dt.is_some());
    }

    #[test]
    fn test_eu_date_dot_format() {
        let dt = parse_date("14.12.2024");
        assert!(dt.is_some());
    }

    #[test]
    fn test_rfc822_without_day() {
        let dt = parse_date("14 Dec 2024");
        assert!(dt.is_some());
    }

    #[test]
    fn test_rfc822_long_month() {
        let dt = parse_date("14 December 2024");
        assert!(dt.is_some());
    }

    #[test]
    fn test_year_slash_format() {
        let dt = parse_date("2024/12/14");
        assert!(dt.is_some());
    }

    #[test]
    fn test_dash_month_format() {
        let dt = parse_date("14-Dec-2024");
        assert!(dt.is_some());
    }

    #[test]
    fn test_us_dash_format() {
        let dt = parse_date("12-14-2024");
        assert!(dt.is_some());
    }

    #[test]
    fn test_eu_slash_with_time() {
        let dt = parse_date("14/12/2024 10:30:45");
        assert!(dt.is_some());
    }

    #[test]
    fn test_multiple_formats_dont_panic() {
        let dates = vec![
            "2024-12-14T10:30:00Z",
            "Sat, 14 Dec 2024 10:30:00 GMT",
            "14 Dec 2024",
            "December 14, 2024",
            "12/14/2024",
            "14.12.2024",
            "2024/12/14",
            "14-Dec-2024",
            "not a date",
            "",
            "2024",
            "12/2024",
        ];

        for date_str in dates {
            let _ = parse_date(date_str);
        }
    }

    #[test]
    fn test_rfc2822_wrong_weekday() {
        // Mon is wrong (actual day is Thu), but date should still parse (#143)
        let dt = parse_date("Mon, 15 Jan 2026 10:30:00 +0000").unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 10);
    }

    #[test]
    fn test_rfc2822_wrong_weekday_new_year() {
        // Wed is wrong (actual day is Thu), but date should still parse (#143)
        let dt = parse_date("Wed, 01 Jan 2026 00:00:00 +0000").unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 1);
    }

    #[test]
    fn test_rfc2822_correct_weekday() {
        // Thu is correct for 2026-01-15
        let dt = parse_date("Thu, 15 Jan 2026 10:30:00 +0000").unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_rfc2822_no_weekday() {
        let dt = parse_date("15 Jan 2026 10:30:00 +0000").unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_edge_case_leap_year() {
        let dt = parse_date("2024-02-29");
        assert!(dt.is_some());
    }

    #[test]
    fn test_edge_case_invalid_date() {
        let dt = parse_date("2023-02-29");
        assert!(dt.is_none());
    }

    #[test]
    fn test_year_only_format() {
        let dt = parse_date("2024").unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 1);
        assert_eq!(dt.hour(), 0);
    }

    #[test]
    fn test_year_month_format() {
        let dt = parse_date("2024-12").unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 1);
        assert_eq!(dt.hour(), 0);
    }

    #[test]
    fn test_all_format_strings() {
        // (input, expected_year, expected_month, expected_day)
        let cases: &[(&str, i32, u32, u32)] = &[
            // ISO 8601 / RFC 3339 variants
            ("2024-12-14T10:30:45.123+00:00", 2024, 12, 14),
            ("2024-12-14T10:30:45+00:00", 2024, 12, 14),
            ("2024-12-14T10:30:45.123Z", 2024, 12, 14),
            ("2024-12-14T10:30:45Z", 2024, 12, 14),
            ("2024-12-14T10:30:45", 2024, 12, 14),
            ("2024-12-14 10:30:45", 2024, 12, 14),
            ("2024-12-14", 2024, 12, 14),
            // W3C Date-Time variants
            ("2024-12-14 10:30:45+00:00", 2024, 12, 14),
            ("2024/12/14 10:30:45", 2024, 12, 14),
            ("2024/12/14", 2024, 12, 14),
            // RFC 822 variants
            ("14 Dec 2024 10:30:45", 2024, 12, 14),
            ("14 Dec 2024", 2024, 12, 14),
            ("14 December 2024 10:30:45", 2024, 12, 14),
            ("14 December 2024", 2024, 12, 14),
            // US date formats
            ("December 14, 2024 10:30:45", 2024, 12, 14),
            ("December 14, 2024", 2024, 12, 14),
            ("Dec 14, 2024 10:30:45", 2024, 12, 14),
            ("Dec 14, 2024", 2024, 12, 14),
            ("12/14/2024 10:30:45", 2024, 12, 14),
            ("12/14/2024", 2024, 12, 14),
            ("12-14-2024", 2024, 12, 14),
            // EU date formats
            ("14.12.2024 10:30:45", 2024, 12, 14),
            ("14.12.2024", 2024, 12, 14),
            ("14/12/2024 10:30:45", 2024, 12, 14),
            ("14/12/2024", 2024, 12, 14),
            ("14-Dec-2024", 2024, 12, 14),
            ("14-December-2024", 2024, 12, 14),
            // Special cases
            ("2024", 2024, 1, 1),
            ("2024-12", 2024, 12, 1),
        ];

        for &(input, year, month, day) in cases {
            let dt = parse_date(input).unwrap_or_else(|| panic!("Failed to parse: {input}"));
            assert_eq!(dt.year(), year, "Year mismatch for: {input}");
            assert_eq!(dt.month(), month, "Month mismatch for: {input}");
            assert_eq!(dt.day(), day, "Day mismatch for: {input}");
        }
    }

    #[test]
    fn test_asctime_single_digit_day_space_padded() {
        // Bug #258: "Mon Jan  6 12:30:00 2025" — day padded with space
        let dt = parse_date("Mon Jan  6 12:30:00 2025").unwrap();
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 6);
        assert_eq!(dt.hour(), 12);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 0);
    }

    #[test]
    fn test_asctime_double_digit_day() {
        let dt = parse_date("Mon Jan 16 12:30:00 2025").unwrap();
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 16);
    }

    #[test]
    fn test_asctime_various_months() {
        let dt = parse_date("Fri Dec 31 23:59:59 2021").unwrap();
        assert_eq!(dt.year(), 2021);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 31);
    }
}
