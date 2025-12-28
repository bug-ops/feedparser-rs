//! SSRF Protection Tests for URL Resolution.
//!
//! These tests verify that malicious xml:base attributes cannot be used
//! to create Server-Side Request Forgery (SSRF) attacks.
#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use feedparser_rs::parse;

#[test]
fn test_ssrf_localhost_blocked() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://localhost/">
        <icon>admin/config</icon>
    </feed>"#;

    let feed = parse(xml).unwrap();
    let icon = feed.feed.icon.as_deref().unwrap();
    assert!(
        !icon.starts_with("http://localhost"),
        "SSRF to localhost should be blocked, got: {icon}"
    );
    // Should return original relative URL instead
    assert_eq!(icon, "admin/config");
}

#[test]
fn test_ssrf_localhost_domain_blocked() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://localhost:8080/">
        <logo>secret/api/key</logo>
    </feed>"#;

    let feed = parse(xml).unwrap();
    let logo = feed.feed.logo.as_deref().unwrap();
    assert!(
        !logo.contains("localhost"),
        "SSRF to localhost domain should be blocked, got: {logo}"
    );
}

#[test]
fn test_ssrf_loopback_ip_blocked() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://127.0.0.1/">
        <icon>config.php</icon>
    </feed>"#;

    let feed = parse(xml).unwrap();
    let icon = feed.feed.icon.as_deref().unwrap();
    assert!(
        !icon.contains("127.0.0.1"),
        "SSRF to loopback IP should be blocked, got: {icon}"
    );
}

#[test]
fn test_ssrf_private_ip_192_168_blocked() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://192.168.1.1/">
        <icon>config.php</icon>
    </feed>"#;

    let feed = parse(xml).unwrap();
    let icon = feed.feed.icon.as_deref().unwrap();
    assert!(
        !icon.contains("192.168"),
        "SSRF to private IP 192.168.x.x should be blocked, got: {icon}"
    );
}

#[test]
fn test_ssrf_private_ip_10_x_blocked() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://10.0.0.1/">
        <icon>admin/backup.sql</icon>
    </feed>"#;

    let feed = parse(xml).unwrap();
    let icon = feed.feed.icon.as_deref().unwrap();
    assert!(
        !icon.contains("10.0.0.1"),
        "SSRF to private IP 10.x.x.x should be blocked, got: {icon}"
    );
}

#[test]
fn test_ssrf_private_ip_172_16_blocked() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://172.20.0.1/">
        <icon>internal/service</icon>
    </feed>"#;

    let feed = parse(xml).unwrap();
    let icon = feed.feed.icon.as_deref().unwrap();
    assert!(
        !icon.contains("172.20"),
        "SSRF to private IP 172.16-31.x.x should be blocked, got: {icon}"
    );
}

#[test]
fn test_ssrf_metadata_endpoint_blocked() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom"
          xml:base="http://169.254.169.254/latest/">
        <icon>meta-data/iam/security-credentials/</icon>
    </feed>"#;

    let feed = parse(xml).unwrap();
    let icon = feed.feed.icon.as_deref().unwrap();
    assert!(
        !icon.contains("169.254.169.254"),
        "SSRF to AWS metadata endpoint should be blocked, got: {icon}"
    );
}

#[test]
fn test_ssrf_ipv6_loopback_blocked() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://[::1]/">
        <icon>admin/config</icon>
    </feed>"#;

    let feed = parse(xml).unwrap();
    let icon = feed.feed.icon.as_deref().unwrap();
    assert!(
        !icon.contains("[::1]"),
        "SSRF to IPv6 loopback should be blocked, got: {icon}"
    );
}

#[test]
fn test_safe_public_urls_work() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://example.com/">
        <icon>images/icon.png</icon>
        <logo>images/logo.png</logo>
    </feed>"#;

    let feed = parse(xml).unwrap();
    assert_eq!(
        feed.feed.icon.as_deref(),
        Some("http://example.com/images/icon.png")
    );
    assert_eq!(
        feed.feed.logo.as_deref(),
        Some("http://example.com/images/logo.png")
    );
}

#[test]
fn test_safe_https_urls_work() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="https://secure.example.com/">
        <icon>icon.png</icon>
    </feed>"#;

    let feed = parse(xml).unwrap();
    assert_eq!(
        feed.feed.icon.as_deref(),
        Some("https://secure.example.com/icon.png")
    );
}

#[test]
fn test_rss_enclosure_ssrf_blocked() {
    // In RSS, the channel link serves as the base URL
    // Test with malicious channel link
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Test</title>
            <link>http://192.168.1.1/</link>
            <item>
                <title>Test Item</title>
                <enclosure url="backup.sql" type="application/sql" length="1000" />
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    let enclosure = &feed.entries[0].enclosures[0];
    assert!(
        !enclosure.url.contains("192.168"),
        "SSRF in RSS enclosure should be blocked, got: {}",
        enclosure.url
    );
    // Should return original relative URL when base is malicious
    assert_eq!(enclosure.url, "backup.sql");
}

#[test]
fn test_rss_link_ssrf_blocked() {
    // In RSS, the channel link serves as the base URL
    let xml = br#"<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Test</title>
            <link>http://localhost/</link>
            <item>
                <title>Test Item</title>
                <link>admin/config.php</link>
            </item>
        </channel>
    </rss>"#;

    let feed = parse(xml).unwrap();
    let link = feed.entries[0].link.as_deref().unwrap();
    assert!(
        !link.contains("localhost"),
        "SSRF in RSS item link should be blocked, got: {link}"
    );
    // Should return original relative URL when base is malicious
    assert_eq!(link, "admin/config.php");
}

#[test]
fn test_atom_link_ssrf_blocked() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://10.0.0.1/">
        <id>test</id>
        <title>Test</title>
        <updated>2024-01-01T00:00:00Z</updated>
        <link href="admin/panel" rel="alternate" />
    </feed>"#;

    let feed = parse(xml).unwrap();
    let link_href = &feed.feed.links[0].href;
    assert!(
        !link_href.contains("10.0.0.1"),
        "SSRF in Atom link should be blocked, got: {link_href}"
    );
    assert_eq!(link_href, "admin/panel");
}

#[test]
fn test_nested_xml_base_ssrf_blocked() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://example.com/">
        <entry xml:base="http://192.168.1.100/">
            <id>test</id>
            <title>Test</title>
            <updated>2024-01-01T00:00:00Z</updated>
            <link href="secret/data" />
        </entry>
    </feed>"#;

    let feed = parse(xml).unwrap();
    let entry_link = &feed.entries[0].links[0].href;
    assert!(
        !entry_link.contains("192.168"),
        "SSRF via nested xml:base should be blocked, got: {entry_link}"
    );
}

#[test]
fn test_absolute_urls_bypass_malicious_base() {
    // Even with malicious base, absolute URLs should work
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://localhost/">
        <icon>http://cdn.example.com/icon.png</icon>
    </feed>"#;

    let feed = parse(xml).unwrap();
    assert_eq!(
        feed.feed.icon.as_deref(),
        Some("http://cdn.example.com/icon.png"),
        "Absolute URLs should override base"
    );
}

#[test]
fn test_relative_urls_without_base_unchanged() {
    // Without xml:base, relative URLs should remain relative
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom">
        <icon>images/icon.png</icon>
    </feed>"#;

    let feed = parse(xml).unwrap();
    assert_eq!(
        feed.feed.icon.as_deref(),
        Some("images/icon.png"),
        "Relative URLs without base should remain unchanged"
    );
}

#[test]
fn test_special_schemes_unaffected() {
    // mailto: and tel: should pass through
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://localhost/">
        <link href="mailto:admin@example.com" />
    </feed>"#;

    let feed = parse(xml).unwrap();
    assert_eq!(
        feed.feed.links[0].href, "mailto:admin@example.com",
        "Special schemes should not be affected by base"
    );
}

#[test]
fn test_file_scheme_protection() {
    // file:// schemes in xml:base should not resolve because resolve_url
    // only works with http/https bases. Non-HTTP schemes result in the
    // original href being returned unchanged.
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="file:///etc/">
        <icon>passwd</icon>
    </feed>"#;

    let feed = parse(xml).unwrap();
    let icon = feed.feed.icon.as_deref().unwrap();
    // file:// base URL parsing fails in url::Url::parse, so original href is returned
    assert_eq!(icon, "passwd", "file:// base should not resolve");
    assert!(
        !icon.starts_with("file://"),
        "file:// scheme should not be in result"
    );
}

#[test]
fn test_google_metadata_domain_blocked() {
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom"
          xml:base="http://metadata.google.internal/">
        <icon>computeMetadata/v1/instance/service-accounts/default/token</icon>
    </feed>"#;

    let feed = parse(xml).unwrap();
    let icon = feed.feed.icon.as_deref().unwrap();
    assert!(
        !icon.contains("metadata.google.internal"),
        "Google metadata endpoint should be blocked, got: {icon}"
    );
}

#[test]
fn test_absolute_malicious_url_in_href_blocked() {
    // If href itself is an absolute malicious URL, it should be blocked
    // even when base URL is safe (or when there's no base URL)
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://example.com/">
        <icon>http://localhost/admin/config</icon>
    </feed>"#;

    let feed = parse(xml).unwrap();
    let icon = feed.feed.icon.as_deref().unwrap_or("");
    assert!(
        !icon.contains("localhost"),
        "Absolute malicious URL in href should be blocked, got: {icon}"
    );
    // Should return empty string for dangerous absolute URLs
    assert!(
        icon.is_empty(),
        "Dangerous absolute URL should result in empty string, got: {icon}"
    );
}

#[test]
fn test_absolute_malicious_private_ip_in_href_blocked() {
    // Private IP in href should be blocked
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom">
        <link href="http://192.168.1.1/internal/api" rel="alternate" />
    </feed>"#;

    let feed = parse(xml).unwrap();
    let link_href = &feed.feed.links[0].href;
    assert!(
        !link_href.contains("192.168"),
        "Absolute malicious private IP in href should be blocked, got: {link_href}"
    );
    // Should return empty string for dangerous absolute URLs
    assert!(
        link_href.is_empty(),
        "Dangerous absolute URL should result in empty string, got: {link_href}"
    );
}

#[test]
fn test_case_insensitive_scheme_bypass_blocked() {
    // Uppercase schemes should also be blocked (RFC 3986 - schemes are case-insensitive)
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="FILE:///etc/">
        <icon>passwd</icon>
    </feed>"#;

    let feed = parse(xml).unwrap();
    let icon = feed.feed.icon.as_deref().unwrap();
    assert!(
        !icon.to_lowercase().starts_with("file://"),
        "Uppercase FILE:// scheme should be blocked, got: {icon}"
    );
}

#[test]
fn test_mixed_case_javascript_scheme_blocked() {
    // Mixed case javascript: should be blocked
    let xml = br#"<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="JaVaScRiPt:alert(1)//">
        <icon>test</icon>
    </feed>"#;

    let feed = parse(xml).unwrap();
    let icon = feed.feed.icon.as_deref().unwrap();
    assert!(
        !icon.to_lowercase().contains("javascript"),
        "Mixed case javascript: scheme should be blocked, got: {icon}"
    );
}
