//! Encoding detection and conversion utilities
//!
//! This module provides functions for detecting character encoding
//! and converting to UTF-8.
//!
//! Encoding detection follows this priority order:
//! 1. BOM (Byte Order Mark) - highest priority
//! 2. HTTP Content-Type charset (if provided)
//! 3. XML declaration encoding attribute
//! 4. Default to UTF-8

use encoding_rs::{Encoding, UTF_8};

/// Detect character encoding from byte data
///
/// Detection order:
/// 1. BOM (Byte Order Mark)
/// 2. XML declaration (<?xml encoding="..."?>)
/// 3. Default to UTF-8
///
/// # Arguments
///
/// * `data` - Raw byte data
///
/// # Returns
///
/// Detected encoding name (e.g., "utf-8", "iso-8859-1")
///
/// # Examples
///
/// ```
/// use feedparser_rs::util::encoding::detect_encoding;
///
/// // UTF-8 with BOM
/// let data = b"\xEF\xBB\xBF<?xml version=\"1.0\"?>";
/// assert_eq!(detect_encoding(data), "UTF-8");
///
/// // XML declaration
/// let data = b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>";
/// assert_eq!(detect_encoding(data), "windows-1252");
/// ```
pub fn detect_encoding(data: &[u8]) -> &'static str {
    // Check BOM first
    if let Some(bom_encoding) = detect_bom(data) {
        return bom_encoding;
    }

    // Check XML declaration
    if let Some(encoding) = extract_xml_encoding(data) {
        return encoding;
    }

    // Default to UTF-8
    "UTF-8"
}

/// Extract encoding from XML declaration
///
/// Parses <?xml version="1.0" encoding="..."?> declaration.
///
/// Operates on raw bytes rather than converting to UTF-8 first, so it works
/// correctly even when non-ASCII bytes appear before the first 512 bytes of
/// content (e.g. ISO-8859-1 titles that begin within 512 bytes of the start).
/// The XML declaration itself is always ASCII, so a byte-level search is safe.
fn extract_xml_encoding(data: &[u8]) -> Option<&'static str> {
    // Only scan the opening portion of the document; the XML declaration must
    // appear before any non-ASCII content so 512 bytes is more than enough.
    let search_data = &data[..data.len().min(512)];

    // Locate b"encoding=" using a byte-level search.
    let needle = b"encoding=";
    let enc_pos = search_data
        .windows(needle.len())
        .position(|w| w == needle)?;

    let after_eq = &search_data[enc_pos + needle.len()..];
    let quote = *after_eq.first()?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }

    let value_bytes = &after_eq[1..];
    let quote_end = value_bytes.iter().position(|&b| b == quote)?;
    // The encoding name is always ASCII; reject if it contains non-ASCII bytes.
    let encoding_name = std::str::from_utf8(&value_bytes[..quote_end]).ok()?;

    normalize_encoding_name(encoding_name)
}

/// Normalize encoding name to `encoding_rs` canonical form
fn normalize_encoding_name(name: &str) -> Option<&'static str> {
    let normalized = name.trim().to_lowercase();
    Encoding::for_label(normalized.as_bytes()).map(encoding_rs::Encoding::name)
}

/// Convert data to UTF-8 from detected encoding
///
/// # Arguments
///
/// * `data` - Raw byte data in unknown encoding
/// * `encoding_name` - Encoding name (e.g., "iso-8859-1")
///
/// # Returns
///
/// * `Ok(String)` - UTF-8 string
/// * `Err(String)` - Error message if conversion failed
///
/// # Examples
///
/// ```
/// use feedparser_rs::util::encoding::convert_to_utf8;
///
/// let latin1 = b"\xE9"; // é in ISO-8859-1
/// let utf8 = convert_to_utf8(latin1, "iso-8859-1").unwrap();
/// assert_eq!(utf8, "é");
/// ```
///
/// # Errors
///
/// Returns an error if the encoding conversion encounters invalid byte sequences
/// that cannot be properly decoded.
pub fn convert_to_utf8(data: &[u8], encoding_name: &str) -> Result<String, String> {
    let encoding = Encoding::for_label(encoding_name.as_bytes()).unwrap_or(UTF_8);

    let (cow, _encoding_used, had_errors) = encoding.decode(data);

    if had_errors {
        Err(format!(
            "Encoding conversion from {encoding_name} had errors"
        ))
    } else {
        Ok(cow.into_owned())
    }
}

/// Detect encoding and convert to UTF-8 in one step
///
/// # Examples
///
/// ```
/// use feedparser_rs::util::encoding::detect_and_convert;
///
/// let data = b"<?xml version=\"1.0\"?><root>Test</root>";
/// let (utf8, detected_encoding) = detect_and_convert(data).unwrap();
/// assert_eq!(detected_encoding, "UTF-8");
/// assert!(utf8.contains("Test"));
/// ```
///
/// # Errors
///
/// Returns an error if the encoding conversion encounters invalid byte sequences
/// that cannot be properly decoded.
pub fn detect_and_convert(data: &[u8]) -> Result<(String, &'static str), String> {
    let encoding_name = detect_encoding(data);
    let utf8_string = convert_to_utf8(data, encoding_name)?;
    Ok((utf8_string, encoding_name))
}

/// Extract charset from HTTP Content-Type header
///
/// Parses the charset parameter from Content-Type headers like:
/// - `text/xml; charset=utf-8`
/// - `application/xml;charset=ISO-8859-1`
/// - `text/html; charset="UTF-8"`
///
/// # Arguments
///
/// * `content_type` - The Content-Type header value
///
/// # Returns
///
/// The charset value if found, or None
///
/// # Examples
///
/// ```
/// use feedparser_rs::util::encoding::extract_charset_from_content_type;
///
/// assert_eq!(
///     extract_charset_from_content_type("text/xml; charset=utf-8"),
///     Some("UTF-8")
/// );
/// assert_eq!(
///     extract_charset_from_content_type("text/html"),
///     None
/// );
/// ```
#[must_use]
pub fn extract_charset_from_content_type(content_type: &str) -> Option<&'static str> {
    let lowercase = content_type.to_lowercase();

    // Find charset= parameter
    let charset_start = lowercase.find("charset=")?;
    let value_start = charset_start + 8;
    let rest = &content_type[value_start..];

    // Handle quoted values: charset="UTF-8"
    let charset_value = if rest.starts_with('"') || rest.starts_with('\'') {
        let quote = rest.chars().next()?;
        let end = rest[1..].find(quote)?;
        &rest[1..=end]
    } else {
        // Unquoted value: charset=UTF-8
        // End at semicolon, space, or end of string
        let end = rest
            .find(|c: char| c == ';' || c.is_whitespace())
            .unwrap_or(rest.len());
        &rest[..end]
    };

    normalize_encoding_name(charset_value)
}

/// Detect encoding with optional HTTP Content-Type hint
///
/// This is the preferred function when parsing feeds from HTTP responses,
/// as it considers the Content-Type charset parameter in addition to
/// BOM and XML declaration detection.
///
/// # Priority Order
///
/// 1. BOM (Byte Order Mark) - highest priority, cannot be wrong
/// 2. HTTP Content-Type charset (if provided)
/// 3. XML declaration encoding attribute
/// 4. Default to UTF-8
///
/// # Arguments
///
/// * `data` - Raw byte data
/// * `content_type` - Optional HTTP Content-Type header value
///
/// # Returns
///
/// Detected encoding name
///
/// # Examples
///
/// ```
/// use feedparser_rs::util::encoding::detect_encoding_with_hint;
///
/// // BOM takes priority over Content-Type
/// let data = b"\xEF\xBB\xBF<?xml version=\"1.0\"?>";
/// assert_eq!(
///     detect_encoding_with_hint(data, Some("text/xml; charset=ISO-8859-1")),
///     "UTF-8"
/// );
///
/// // Content-Type is used when no BOM
/// let data = b"<?xml version=\"1.0\"?>";
/// assert_eq!(
///     detect_encoding_with_hint(data, Some("text/xml; charset=ISO-8859-1")),
///     "windows-1252"
/// );
///
/// // Falls back to XML declaration when no Content-Type
/// let data = b"<?xml version=\"1.0\" encoding=\"UTF-16\"?>";
/// assert_eq!(detect_encoding_with_hint(data, None), "UTF-16LE");
/// ```
pub fn detect_encoding_with_hint(data: &[u8], content_type: Option<&str>) -> &'static str {
    // Check BOM first - highest priority
    if let Some(bom_encoding) = detect_bom(data) {
        return bom_encoding;
    }

    // Check Content-Type charset if provided
    if let Some(ct) = content_type
        && let Some(charset) = extract_charset_from_content_type(ct)
    {
        return charset;
    }

    // Check XML declaration
    if let Some(encoding) = extract_xml_encoding(data) {
        return encoding;
    }

    // Default to UTF-8
    "UTF-8"
}

/// Detect encoding from BOM only
///
/// Returns the encoding if a BOM is present, None otherwise.
fn detect_bom(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Some("UTF-8");
    }
    // UTF-32 BOMs must be checked BEFORE UTF-16 BOMs
    // because UTF-32LE BOM (FF FE 00 00) starts with UTF-16LE BOM (FF FE)
    if data.starts_with(&[0x00, 0x00, 0xFE, 0xFF]) {
        return Some("UTF-32BE");
    }
    if data.starts_with(&[0xFF, 0xFE, 0x00, 0x00]) {
        return Some("UTF-32LE");
    }
    if data.starts_with(&[0xFF, 0xFE]) {
        return Some("UTF-16LE");
    }
    if data.starts_with(&[0xFE, 0xFF]) {
        return Some("UTF-16BE");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_utf8_bom() {
        let data = b"\xEF\xBB\xBF<?xml version=\"1.0\"?>";
        assert_eq!(detect_encoding(data), "UTF-8");
    }

    #[test]
    fn test_detect_utf16le_bom() {
        let data = b"\xFF\xFE<\x00?\x00x\x00m\x00l\x00";
        assert_eq!(detect_encoding(data), "UTF-16LE");
    }

    #[test]
    fn test_detect_utf16be_bom() {
        let data = b"\xFE\xFF\x00<\x00?\x00x\x00m\x00l";
        assert_eq!(detect_encoding(data), "UTF-16BE");
    }

    #[test]
    fn test_detect_from_xml_declaration() {
        let data = b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>";
        assert_eq!(detect_encoding(data).to_lowercase(), "windows-1252");
    }

    #[test]
    fn test_detect_from_xml_declaration_single_quotes() {
        let data = b"<?xml version='1.0' encoding='UTF-8'?>";
        assert_eq!(detect_encoding(data), "UTF-8");
    }

    #[test]
    fn test_detect_default_utf8() {
        let data = b"<?xml version=\"1.0\"?>";
        assert_eq!(detect_encoding(data), "UTF-8");
    }

    #[test]
    fn test_convert_iso8859_1() {
        let data = b"\xE9";
        let utf8 = convert_to_utf8(data, "iso-8859-1").unwrap();
        assert_eq!(utf8, "é");
    }

    #[test]
    fn test_convert_windows1252() {
        let data = b"\x93Hello\x94";
        let utf8 = convert_to_utf8(data, "windows-1252").unwrap();
        assert!(utf8.contains("Hello"));
    }

    #[test]
    fn test_detect_and_convert() {
        let data = b"<?xml version=\"1.0\"?><root>Test</root>";
        let (utf8, encoding) = detect_and_convert(data).unwrap();
        assert_eq!(encoding, "UTF-8");
        assert!(utf8.contains("Test"));
    }

    #[test]
    fn test_extract_xml_encoding_double_quotes() {
        let data = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>";
        assert!(extract_xml_encoding(data).is_some());
    }

    #[test]
    fn test_extract_xml_encoding_single_quotes() {
        let data = b"<?xml version='1.0' encoding='UTF-8'?>";
        assert!(extract_xml_encoding(data).is_some());
    }

    #[test]
    fn test_extract_xml_encoding_none() {
        let data = b"<?xml version=\"1.0\"?>";
        assert!(extract_xml_encoding(data).is_none());
    }

    #[test]
    fn test_normalize_encoding_name() {
        assert_eq!(normalize_encoding_name("UTF-8"), Some("UTF-8"));
        assert_eq!(normalize_encoding_name("utf-8"), Some("UTF-8"));
        assert_eq!(normalize_encoding_name("  UTF-8  "), Some("UTF-8"));
        assert_eq!(normalize_encoding_name("ISO-8859-1"), Some("windows-1252"));
    }

    #[test]
    fn test_convert_utf8_to_utf8() {
        let data = b"Hello";
        let result = convert_to_utf8(data, "utf-8").unwrap();
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_detect_no_encoding_declaration() {
        let data = b"<rss><channel></channel></rss>";
        assert_eq!(detect_encoding(data), "UTF-8");
    }

    #[test]
    fn test_empty_data() {
        let data = b"";
        assert_eq!(detect_encoding(data), "UTF-8");
    }

    // Tests for Content-Type charset extraction

    #[test]
    fn test_extract_charset_basic() {
        assert_eq!(
            extract_charset_from_content_type("text/xml; charset=utf-8"),
            Some("UTF-8")
        );
    }

    #[test]
    fn test_extract_charset_no_space() {
        assert_eq!(
            extract_charset_from_content_type("text/xml;charset=utf-8"),
            Some("UTF-8")
        );
    }

    #[test]
    fn test_extract_charset_quoted() {
        assert_eq!(
            extract_charset_from_content_type("text/xml; charset=\"UTF-8\""),
            Some("UTF-8")
        );
    }

    #[test]
    fn test_extract_charset_single_quoted() {
        assert_eq!(
            extract_charset_from_content_type("text/xml; charset='UTF-8'"),
            Some("UTF-8")
        );
    }

    #[test]
    fn test_extract_charset_uppercase() {
        assert_eq!(
            extract_charset_from_content_type("TEXT/XML; CHARSET=UTF-8"),
            Some("UTF-8")
        );
    }

    #[test]
    fn test_extract_charset_iso8859() {
        assert_eq!(
            extract_charset_from_content_type("text/html; charset=iso-8859-1"),
            Some("windows-1252")
        );
    }

    #[test]
    fn test_extract_charset_none() {
        assert_eq!(extract_charset_from_content_type("text/xml"), None);
    }

    #[test]
    fn test_extract_charset_empty() {
        assert_eq!(extract_charset_from_content_type(""), None);
    }

    #[test]
    fn test_extract_charset_with_boundary() {
        // Content-Type with multiple parameters
        assert_eq!(
            extract_charset_from_content_type("multipart/form-data; boundary=----; charset=utf-8"),
            Some("UTF-8")
        );
    }

    // Tests for detect_encoding_with_hint

    #[test]
    fn test_hint_bom_priority() {
        // BOM takes priority over Content-Type
        let data = b"\xEF\xBB\xBF<?xml version=\"1.0\"?>";
        assert_eq!(
            detect_encoding_with_hint(data, Some("text/xml; charset=ISO-8859-1")),
            "UTF-8"
        );
    }

    #[test]
    fn test_hint_content_type_used() {
        // Content-Type is used when no BOM
        let data = b"<?xml version=\"1.0\"?>";
        assert_eq!(
            detect_encoding_with_hint(data, Some("text/xml; charset=ISO-8859-1")),
            "windows-1252"
        );
    }

    #[test]
    fn test_hint_xml_declaration_fallback() {
        // Falls back to XML declaration when no Content-Type charset
        let data = b"<?xml version=\"1.0\" encoding=\"windows-1252\"?>";
        assert_eq!(detect_encoding_with_hint(data, None), "windows-1252");
    }

    #[test]
    fn test_hint_default_utf8() {
        // Default to UTF-8 when no hints
        let data = b"<rss><channel></channel></rss>";
        assert_eq!(detect_encoding_with_hint(data, None), "UTF-8");
    }

    #[test]
    fn test_hint_content_type_without_charset() {
        // Content-Type without charset falls through to XML declaration
        let data = b"<?xml version=\"1.0\" encoding=\"windows-1252\"?>";
        assert_eq!(
            detect_encoding_with_hint(data, Some("text/xml")),
            "windows-1252"
        );
    }

    // Tests for detect_bom

    #[test]
    fn test_detect_bom_utf8() {
        assert_eq!(detect_bom(b"\xEF\xBB\xBF"), Some("UTF-8"));
    }

    #[test]
    fn test_detect_bom_utf16le() {
        assert_eq!(detect_bom(b"\xFF\xFE"), Some("UTF-16LE"));
    }

    #[test]
    fn test_detect_bom_utf16be() {
        assert_eq!(detect_bom(b"\xFE\xFF"), Some("UTF-16BE"));
    }

    #[test]
    fn test_detect_bom_utf32le() {
        assert_eq!(detect_bom(b"\xFF\xFE\x00\x00"), Some("UTF-32LE"));
    }

    #[test]
    fn test_detect_bom_utf32be() {
        assert_eq!(detect_bom(b"\x00\x00\xFE\xFF"), Some("UTF-32BE"));
    }

    #[test]
    fn test_detect_bom_none() {
        assert_eq!(detect_bom(b"<?xml"), None);
        assert_eq!(detect_bom(b""), None);
    }
}
