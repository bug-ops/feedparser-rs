//! Server-Side Request Forgery (SSRF) validation shared by the HTTP fetcher
//! and `xml:base` URL resolution.
//!
//! This module is the single source of truth for what counts as a "safe"
//! destination (public host, no loopback/private/link-local/metadata
//! address). It has no dependency on the `http` feature so it is always
//! compiled, letting [`crate::util::base_url`] reuse it even when the
//! `reqwest`-based HTTP client is disabled.

use crate::error::{FeedError, Result};
#[cfg(feature = "http")]
use std::net::IpAddr;
use std::net::{Ipv4Addr, Ipv6Addr};
use url::Url;

/// Localhost variations that should be blocked.
const LOCALHOST_VARIANTS: &[&str] = &[
    "localhost",
    "localhost.localdomain",
    "127.0.0.1",
    "::1",
    "[::1]",
];

/// Internal TLDs that should be blocked.
const INTERNAL_TLDS: &[&str] = &[
    ".local",
    ".localhost",
    ".internal",
    ".intranet",
    ".corp",
    ".home",
    ".lan",
];

/// Cloud metadata endpoints that should be blocked.
const METADATA_DOMAINS: &[&str] = &[
    "metadata.google.internal",
    "169.254.169.254",
    "metadata",
    "metadata.azure.com",
];

/// Validates a URL to prevent Server-Side Request Forgery (SSRF) attacks.
///
/// This is the canonical check reused by [`crate::http::validation::validate_url`]
/// (initial request validation and redirect re-validation) and
/// [`crate::util::base_url::is_safe_url`] (`xml:base` resolution), so both call
/// sites enforce the same rule set.
///
/// # Errors
///
/// Returns `FeedError::Http` if the URL is malformed, uses an unsupported
/// scheme, or resolves to a private/loopback/link-local/metadata host.
pub fn validate_url(url_str: &str) -> Result<Url> {
    let url = Url::parse(url_str).map_err(|e| FeedError::Http {
        message: format!("Invalid URL: {e}"),
    })?;

    match url.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(FeedError::Http {
                message: format!(
                    "Unsupported URL scheme '{scheme}': only 'http' and 'https' are allowed"
                ),
            });
        }
    }

    let host = url.host().ok_or_else(|| FeedError::Http {
        message: "URL must have a host".to_string(),
    })?;

    match host {
        url::Host::Ipv4(ip) => validate_ipv4(ip)?,
        url::Host::Ipv6(ip) => validate_ipv6(ip)?,
        url::Host::Domain(domain) => validate_domain(domain)?,
    }

    Ok(url)
}

/// Validates a resolved socket-level IP address.
///
/// Used to re-check DNS resolution results at connect time, closing the DNS
/// rebinding gap where a domain passes [`validate_url`] with a public IP but
/// is later repointed to a private/internal address.
///
/// # Errors
///
/// Returns `FeedError::Http` if `ip` is a private, loopback, link-local, or
/// otherwise disallowed address.
///
/// Gated behind the `http` feature: it is only consumed by
/// `http::resolver::SsrfSafeResolver`, and would otherwise be dead code (and
/// fail a `-D warnings` build) with `http` disabled.
#[cfg(feature = "http")]
pub fn validate_ip_addr(ip: IpAddr) -> Result<()> {
    match ip {
        IpAddr::V4(ip) => validate_ipv4(ip),
        IpAddr::V6(ip) => validate_ipv6(ip),
    }
}

/// Validates an IPv4 address to prevent SSRF.
fn validate_ipv4(ip: Ipv4Addr) -> Result<()> {
    if ip.is_private() {
        return Err(FeedError::Http {
            message: format!("Private IP address not allowed: {ip} (RFC 1918)"),
        });
    }

    if ip.is_loopback() {
        return Err(FeedError::Http {
            message: format!("Loopback address not allowed: {ip}"),
        });
    }

    if ip.is_link_local() {
        return Err(FeedError::Http {
            message: format!("Link-local address not allowed: {ip} (169.254.0.0/16)"),
        });
    }

    if ip.is_broadcast() {
        return Err(FeedError::Http {
            message: format!("Broadcast address not allowed: {ip}"),
        });
    }

    if ip.is_documentation() {
        return Err(FeedError::Http {
            message: format!("Documentation IP not allowed: {ip} (RFC 5737)"),
        });
    }

    let octets = ip.octets();

    // Block cloud metadata endpoints specifically
    if octets[0] == 169 && octets[1] == 254 && octets[2] == 169 && octets[3] == 254 {
        return Err(FeedError::Http {
            message: "AWS metadata endpoint blocked: 169.254.169.254".to_string(),
        });
    }

    // Block carrier-grade NAT (100.64.0.0/10)
    if octets[0] == 100 && (octets[1] & 0xC0) == 64 {
        return Err(FeedError::Http {
            message: format!("Carrier-grade NAT address not allowed: {ip} (100.64.0.0/10)"),
        });
    }

    // Block 0.0.0.0/8
    if octets[0] == 0 {
        return Err(FeedError::Http {
            message: format!("0.0.0.0/8 range not allowed: {ip}"),
        });
    }

    Ok(())
}

/// Well-known NAT64 (RFC 6052) prefix `64:ff9b::/96`: the first 6 segments
/// of any address translated by a standard NAT64/DNS64 gateway.
const NAT64_PREFIX: [u16; 6] = [0x0064, 0xff9b, 0, 0, 0, 0];

/// The zero prefix shared by deprecated IPv4-compatible addresses
/// (`::a.b.c.d`), distinct from the IPv4-mapped prefix (`::ffff:a.b.c.d`),
/// which `Ipv6Addr::to_ipv4_mapped` already covers.
const IPV4_COMPATIBLE_PREFIX: [u16; 6] = [0, 0, 0, 0, 0, 0];

/// Extracts the IPv4 address embedded in the low 32 bits of a segment array.
const fn embedded_ipv4(segments: [u16; 8]) -> Ipv4Addr {
    Ipv4Addr::new(
        (segments[6] >> 8) as u8,
        (segments[6] & 0xFF) as u8,
        (segments[7] >> 8) as u8,
        (segments[7] & 0xFF) as u8,
    )
}

/// Validates an IPv6 address to prevent SSRF.
fn validate_ipv6(ip: Ipv6Addr) -> Result<()> {
    // IPv4-mapped addresses (::ffff:0:0/96) embed an IPv4 address that the
    // OS resolves to on the wire; e.g. `::ffff:127.0.0.1` is loopback but
    // none of the native IPv6 checks below recognize it as such, so it must
    // be unwrapped and validated against the IPv4 rules instead.
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return validate_ipv4(mapped);
    }

    let segments = ip.segments();

    // IPv4-compatible addresses (deprecated `::a.b.c.d` form, distinct from
    // the IPv4-mapped form above) similarly embed a real IPv4 target that
    // bypasses every native IPv6 check below. `::` and `::1` are excluded
    // since is_unspecified()/is_loopback() already classify those natively
    // (and correctly) further down.
    let low32 = (u32::from(segments[6]) << 16) | u32::from(segments[7]);
    if segments[..6] == IPV4_COMPATIBLE_PREFIX && low32 > 1 {
        return validate_ipv4(embedded_ipv4(segments));
    }

    // NAT64 (RFC 6052): IPv6-only networks with NAT64/DNS64 — standard on
    // modern cloud IPv6-only subnets — translate this prefix straight
    // through to the embedded IPv4 address (e.g. `64:ff9b::a9fe:a9fe`
    // reaches 169.254.169.254 on a real NAT64 gateway), bypassing every
    // native IPv6 check below just like the mapped/compatible forms.
    if segments[..6] == NAT64_PREFIX {
        return validate_ipv4(embedded_ipv4(segments));
    }

    // Known follow-up gap, deliberately out of scope here: 6to4 addresses
    // (`2002::/16`, RFC 3056) also embed an IPv4 address in the same way,
    // but 6to4 relay infrastructure has been widely decommissioned and the
    // range is not standard on any current cloud network, unlike the three
    // forms above (mapped/compatible are used by real dual-stack stacks,
    // NAT64 is standard on modern IPv6-only cloud subnets).

    if ip.is_loopback() {
        return Err(FeedError::Http {
            message: format!("IPv6 loopback address not allowed: {ip}"),
        });
    }

    if ip.is_unspecified() {
        return Err(FeedError::Http {
            message: format!("IPv6 unspecified address not allowed: {ip}"),
        });
    }

    if ip.is_unicast_link_local() {
        return Err(FeedError::Http {
            message: format!("IPv6 link-local address not allowed: {ip} (fe80::/10)"),
        });
    }

    // Check for Unique Local Addresses (ULA) - fc00::/7
    if (segments[0] & 0xFE00) == 0xFC00 {
        return Err(FeedError::Http {
            message: format!("IPv6 unique local address not allowed: {ip} (fc00::/7)"),
        });
    }

    if ip.is_multicast() {
        return Err(FeedError::Http {
            message: format!("IPv6 multicast address not allowed: {ip} (ff00::/8)"),
        });
    }

    Ok(())
}

/// Validates a domain name to prevent SSRF.
fn validate_domain(domain: &str) -> Result<()> {
    let domain_lower = domain.to_lowercase();

    if LOCALHOST_VARIANTS.contains(&domain_lower.as_str()) {
        return Err(FeedError::Http {
            message: format!("Localhost domain not allowed: {domain}"),
        });
    }

    for tld in INTERNAL_TLDS {
        if domain_lower.ends_with(tld) {
            return Err(FeedError::Http {
                message: format!("Internal domain TLD not allowed: {domain}"),
            });
        }
    }

    if METADATA_DOMAINS.contains(&domain_lower.as_str()) {
        return Err(FeedError::Http {
            message: format!("Cloud metadata domain not allowed: {domain}"),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_validate_url_public_ok() {
        assert!(validate_url("https://example.com/feed.xml").is_ok());
    }

    #[test]
    fn test_validate_url_rejects_private_ip() {
        assert!(validate_url("http://10.0.0.1/").is_err());
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_validate_ip_addr_rejects_loopback() {
        assert!(validate_ip_addr(IpAddr::V4(Ipv4Addr::LOCALHOST)).is_err());
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_validate_ip_addr_rejects_metadata() {
        assert!(validate_ip_addr(IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))).is_err());
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_validate_ip_addr_accepts_public() {
        assert!(validate_ip_addr(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))).is_ok());
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_validate_ip_addr_rejects_ipv6_unspecified() {
        assert!(validate_ip_addr(IpAddr::V6(Ipv6Addr::UNSPECIFIED)).is_err());
    }

    #[test]
    fn test_validate_url_rejects_ipv4_mapped_loopback() {
        // ::ffff:127.0.0.1 is not caught by is_loopback()/is_unicast_link_local()
        // on Ipv6Addr; it must be unwrapped via to_ipv4_mapped() and re-checked.
        assert!(validate_url("http://[::ffff:127.0.0.1]/").is_err());
    }

    #[test]
    fn test_validate_url_rejects_ipv4_mapped_private() {
        assert!(validate_url("http://[::ffff:10.0.0.1]/").is_err());
    }

    #[test]
    fn test_validate_url_allows_ipv4_mapped_public() {
        assert!(validate_url("http://[::ffff:8.8.8.8]/").is_ok());
    }

    #[test]
    fn test_validate_url_rejects_ipv4_compatible_loopback() {
        // ::127.0.0.1 (deprecated IPv4-compatible form) is distinct from the
        // literal ::1 that is_loopback() recognizes.
        assert!(validate_url("http://[::127.0.0.1]/").is_err());
    }

    #[test]
    fn test_validate_url_rejects_ipv4_compatible_metadata() {
        assert!(validate_url("http://[::169.254.169.254]/").is_err());
    }

    #[test]
    fn test_validate_url_allows_ipv4_compatible_public() {
        assert!(validate_url("http://[::8.8.8.8]/").is_ok());
    }

    #[test]
    fn test_validate_url_rejects_nat64_metadata() {
        // 64:ff9b::a9fe:a9fe is the RFC 6052 NAT64 encoding of 169.254.169.254.
        assert!(validate_url("http://[64:ff9b::a9fe:a9fe]/").is_err());
    }

    #[test]
    fn test_validate_url_rejects_nat64_private() {
        // 64:ff9b::a00:1 encodes 10.0.0.1.
        assert!(validate_url("http://[64:ff9b::a00:1]/").is_err());
    }

    #[test]
    fn test_validate_url_allows_nat64_public() {
        // 64:ff9b::808:808 encodes 8.8.8.8.
        assert!(validate_url("http://[64:ff9b::808:808]/").is_ok());
    }
}
