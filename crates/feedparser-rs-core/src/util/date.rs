//! Multi-format date parsing for RSS and Atom feeds

use jiff::{
    Timestamp,
    civil::Date,
    fmt::{
        rfc2822::DateTimeParser,
        strtime::{self, BrokenDownTime},
    },
    tz::Offset,
};

/// RFC 2822 parser configured to accept mismatched weekdays (#143).
static RFC2822_PARSER: DateTimeParser = DateTimeParser::new().relaxed_weekday(true);

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

/// Convert a `strtime::BrokenDownTime` to a UTC `Timestamp`, defaulting naive
/// datetimes to UTC and preserving parsed offsets (returning `None` on overflow).
fn broken_down_to_utc_timestamp(tm: &BrokenDownTime) -> Option<Timestamp> {
    let dt = tm.to_datetime().ok()?;
    tm.offset().unwrap_or(Offset::UTC).to_timestamp(dt).ok()
}

/// Parse ASCTIME format: `Www Mmm [D]D HH:MM:SS YYYY` where DD may have a leading space.
///
/// Example: `Mon Jan  6 12:30:00 2025` or `Mon Jan 16 12:30:00 2025`
fn parse_asctime(s: &str) -> Option<Timestamp> {
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
    // Strip weekday prefix so wrong weekdays (e.g. "Tue Jan 16 12:30:00 2025") still parse.
    let rest = &s[4..];
    let dt = strtime::parse("%b %e %H:%M:%S %Y", rest)
        .ok()?
        .to_datetime()
        .ok()?;
    Offset::UTC.to_timestamp(dt).ok()
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
/// * `Some(Timestamp)` - Successfully parsed date normalized to UTC
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
pub fn parse_date(input: &str) -> Option<Timestamp> {
    let input = input.trim();

    if input.is_empty() {
        return None;
    }

    // Try RFC 3339 / ISO 8601 with offset first (most common in Atom)
    if let Ok(ts) = input.parse::<Timestamp>() {
        return Some(ts);
    }

    // Try RFC 2822 (RSS pubDate format) with relaxed weekday validation (#143)
    if let Ok(ts) = RFC2822_PARSER.parse_timestamp(input) {
        return Some(ts);
    }

    // Special handling for year-only format (e.g., "2024")
    if let Ok(year) = input.parse::<i16>()
        && (1000..=9999).contains(&year)
        && let Ok(date) = Date::new(year, 1, 1)
    {
        return Offset::UTC.to_timestamp(date.at(0, 0, 0, 0)).ok();
    }

    // Special handling for year-month format (e.g., "2024-12")
    if input.len() == 7
        && input.is_ascii()
        && input.as_bytes()[4] == b'-'
        && let (Ok(year), Ok(month)) = (input[..4].parse::<i16>(), input[5..7].parse::<i8>())
        && (1000..=9999).contains(&year)
        && (1..=12).contains(&month)
        && let Ok(date) = Date::new(year, month, 1)
    {
        return Offset::UTC.to_timestamp(date.at(0, 0, 0, 0)).ok();
    }

    // Try ASCTIME format: "Mon Jan  6 12:30:00 2025"
    if let Some(ts) = parse_asctime(input) {
        return Some(ts);
    }

    // Try all format strings
    for fmt in DATE_FORMATS {
        if let Ok(tm) = strtime::parse(fmt, input)
            && let Some(ts) = broken_down_to_utc_timestamp(&tm)
        {
            return Some(ts);
        }
    }

    // Could not parse
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_utc(ts: Timestamp) -> jiff::civil::DateTime {
        Offset::UTC.to_datetime(ts)
    }

    #[test]
    fn test_rfc3339_with_timezone() {
        let dt = parse_date("2024-12-14T10:30:00+00:00");
        assert!(dt.is_some());
        let dt = to_utc(dt.unwrap());
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
        assert_eq!(to_utc(dt.unwrap()).year(), 2024);
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
        let dt = to_utc(dt.unwrap());
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
        let dt = to_utc(dt.unwrap());
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
        let dt = to_utc(parse_date("Mon, 15 Jan 2026 10:30:00 +0000").unwrap());
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 10);
    }

    #[test]
    fn test_rfc2822_wrong_weekday_new_year() {
        // Wed is wrong (actual day is Thu), but date should still parse (#143)
        let dt = to_utc(parse_date("Wed, 01 Jan 2026 00:00:00 +0000").unwrap());
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 1);
    }

    #[test]
    fn test_rfc2822_correct_weekday() {
        // Thu is correct for 2026-01-15
        let dt = to_utc(parse_date("Thu, 15 Jan 2026 10:30:00 +0000").unwrap());
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_rfc2822_no_weekday() {
        let dt = to_utc(parse_date("15 Jan 2026 10:30:00 +0000").unwrap());
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
        let dt = to_utc(parse_date("2024").unwrap());
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 1);
        assert_eq!(dt.hour(), 0);
    }

    #[test]
    fn test_year_month_format() {
        let dt = to_utc(parse_date("2024-12").unwrap());
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 1);
        assert_eq!(dt.hour(), 0);
    }

    #[test]
    fn test_all_format_strings() {
        // (input, expected_year, expected_month, expected_day)
        let cases: &[(&str, i16, i8, i8)] = &[
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
            let dt =
                to_utc(parse_date(input).unwrap_or_else(|| panic!("Failed to parse: {input}")));
            assert_eq!(dt.year(), year, "Year mismatch for: {input}");
            assert_eq!(dt.month(), month, "Month mismatch for: {input}");
            assert_eq!(dt.day(), day, "Day mismatch for: {input}");
        }
    }

    #[test]
    fn test_asctime_single_digit_day_space_padded() {
        // Bug #258: "Mon Jan  6 12:30:00 2025" — day padded with space
        let dt = to_utc(parse_date("Mon Jan  6 12:30:00 2025").unwrap());
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 6);
        assert_eq!(dt.hour(), 12);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 0);
    }

    #[test]
    fn test_asctime_double_digit_day() {
        let dt = to_utc(parse_date("Mon Jan 16 12:30:00 2025").unwrap());
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 16);
    }

    #[test]
    fn test_asctime_various_months() {
        let dt = to_utc(parse_date("Fri Dec 31 23:59:59 2021").unwrap());
        assert_eq!(dt.year(), 2021);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 31);
    }

    #[test]
    fn test_asctime_wrong_weekday_and_short_input() {
        let dt = to_utc(parse_date("Tue Jan 16 12:30:00 2025").unwrap());
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 16);
        assert_eq!(parse_date("abc"), None);
    }

    #[test]
    fn test_non_ascii_year_month_guard_and_over9_fractional() {
        assert_eq!(parse_date("abcä-1"), None);
        assert_eq!(parse_date("2024-12-14T10:30:00.1234567890Z"), None);
    }

    #[test]
    fn test_date_formats_offset_preservation_and_overflow() {
        let dt = to_utc(parse_date("2024-12-14 10:30:45+02:00").unwrap());
        assert_eq!(dt.hour(), 8);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 45);

        let dt_submin = to_utc(parse_date("2024-12-14 10:30:45+02:00:15").unwrap());
        assert_eq!(dt_submin.hour(), 8);
        assert_eq!(dt_submin.minute(), 30);
        assert_eq!(dt_submin.second(), 30);

        assert_eq!(parse_date("9999-12-30 20:00:00-05:00"), None);
    }
}
