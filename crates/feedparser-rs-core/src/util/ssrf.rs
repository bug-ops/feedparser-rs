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

    if ip.is_multicast() {
        return Err(FeedError::Http {
            message: format!("Multicast address not allowed: {ip} (224.0.0.0/4)"),
        });
    }

    validate_ipv4_special_purpose(ip, ip.octets())
}

/// Validates IANA special-purpose IPv4 ranges not covered by `Ipv4Addr`'s
/// built-in classifiers, plus cloud-metadata/CGN/this-network/AS112/AMT
/// blocks. Split out of [`validate_ipv4`] to keep that function close to the
/// project's function-length target.
fn validate_ipv4_special_purpose(ip: Ipv4Addr, octets: [u8; 4]) -> Result<()> {
    // Reserved for future use (RFC 1112), excluding 255.255.255.255 which
    // is already covered by the broadcast check in `validate_ipv4`.
    if octets[0] >= 240 {
        return Err(FeedError::Http {
            message: format!("Reserved address not allowed: {ip} (240.0.0.0/4)"),
        });
    }

    // IETF Protocol Assignments (RFC 6890)
    if octets[0] == 192 && octets[1] == 0 && octets[2] == 0 {
        return Err(FeedError::Http {
            message: format!("IETF protocol assignment address not allowed: {ip} (192.0.0.0/24)"),
        });
    }

    // 6to4 relay anycast (RFC 3068); deprecated by RFC 7526 but still seen
    // in older configurations.
    if octets[0] == 192 && octets[1] == 88 && octets[2] == 99 {
        return Err(FeedError::Http {
            message: format!("6to4 relay anycast address not allowed: {ip} (192.88.99.0/24)"),
        });
    }

    // Benchmarking (RFC 2544)
    if octets[0] == 198 && (octets[1] & 0xFE) == 18 {
        return Err(FeedError::Http {
            message: format!("Benchmarking address not allowed: {ip} (198.18.0.0/15)"),
        });
    }

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

    // AS112-v4 (RFC 7535)
    if octets[0] == 192 && octets[1] == 31 && octets[2] == 196 {
        return Err(FeedError::Http {
            message: format!("AS112-v4 address not allowed: {ip} (192.31.196.0/24)"),
        });
    }

    // AMT (RFC 7450)
    if octets[0] == 192 && octets[1] == 52 && octets[2] == 193 {
        return Err(FeedError::Http {
            message: format!("AMT address not allowed: {ip} (192.52.193.0/24)"),
        });
    }

    // Direct Delegation AS112 Service (RFC 7534)
    if octets[0] == 192 && octets[1] == 175 && octets[2] == 48 {
        return Err(FeedError::Http {
            message: format!(
                "Direct Delegation AS112 Service address not allowed: {ip} (192.175.48.0/24)"
            ),
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

    validate_ipv6_special_purpose(ip, segments)?;
    validate_ipv6_special_purpose_extended(ip, segments)?;

    if ip.is_multicast() {
        return Err(FeedError::Http {
            message: format!("IPv6 multicast address not allowed: {ip} (ff00::/8)"),
        });
    }

    Ok(())
}

/// Validates IANA special-purpose IPv6 ranges not covered by `Ipv6Addr`'s
/// built-in classifiers (Teredo, `ORCHIDv2`, documentation, discard-only, the
/// RFC 9780 dummy prefix, the RFC 8215 NAT64 local-use prefix, PCP/TURN/DNS-SD
/// anycast, AMT, and the AS112 service ranges). Split out of
/// [`validate_ipv6`] to keep that function under the project's
/// function-length limit.
fn validate_ipv6_special_purpose(ip: Ipv6Addr, segments: [u16; 8]) -> Result<()> {
    // Teredo tunneling (RFC 4380)
    if segments[0] == 0x2001 && segments[1] == 0 {
        return Err(FeedError::Http {
            message: format!("IPv6 Teredo address not allowed: {ip} (2001::/32)"),
        });
    }

    // ORCHIDv2 (RFC 7343)
    if segments[0] == 0x2001 && (segments[1] & 0xFFF0) == 0x0020 {
        return Err(FeedError::Http {
            message: format!("IPv6 ORCHIDv2 address not allowed: {ip} (2001:20::/28)"),
        });
    }

    // Documentation range (RFC 3849)
    if segments[0] == 0x2001 && segments[1] == 0x0db8 {
        return Err(FeedError::Http {
            message: format!("IPv6 documentation address not allowed: {ip} (2001:db8::/32)"),
        });
    }

    // Discard-only prefix (RFC 6666)
    if segments[0] == 0x0100 && segments[1] == 0 && segments[2] == 0 && segments[3] == 0 {
        return Err(FeedError::Http {
            message: format!("IPv6 discard-only address not allowed: {ip} (100::/64)"),
        });
    }

    // Dummy IPv6 Prefix (RFC 9780), distinct from the discard-only prefix
    // above — the IANA registry lists these as two independent /64 entries,
    // not sub-ranges of a common parent block.
    if segments[0] == 0x0100 && segments[1] == 0 && segments[2] == 0 && segments[3] == 1 {
        return Err(FeedError::Http {
            message: format!("IPv6 dummy prefix address not allowed: {ip} (100:0:0:1::/64)"),
        });
    }

    // NAT64 local-use prefix (RFC 8215), distinct from the well-known
    // 64:ff9b::/96 prefix unwrapped to its embedded IPv4 address in
    // `validate_ipv6`.
    if segments[0] == 0x0064 && segments[1] == 0xff9b && segments[2] == 0x0001 {
        return Err(FeedError::Http {
            message: format!("IPv6 NAT64 local-use address not allowed: {ip} (64:ff9b:1::/48)"),
        });
    }

    // PCP Anycast (RFC 7723)
    if segments == [0x2001, 1, 0, 0, 0, 0, 0, 1] {
        return Err(FeedError::Http {
            message: format!("IPv6 PCP Anycast address not allowed: {ip} (2001:1::1/128)"),
        });
    }

    // TURN Relay Anycast (RFC 8155)
    if segments == [0x2001, 1, 0, 0, 0, 0, 0, 2] {
        return Err(FeedError::Http {
            message: format!("IPv6 TURN Relay Anycast address not allowed: {ip} (2001:1::2/128)"),
        });
    }

    // DNS-SD Service Registration Protocol Anycast (RFC 9665)
    if segments == [0x2001, 1, 0, 0, 0, 0, 0, 3] {
        return Err(FeedError::Http {
            message: format!("IPv6 DNS-SD Anycast address not allowed: {ip} (2001:1::3/128)"),
        });
    }

    // AMT (RFC 7450)
    if segments[0] == 0x2001 && segments[1] == 3 {
        return Err(FeedError::Http {
            message: format!("IPv6 AMT address not allowed: {ip} (2001:3::/32)"),
        });
    }

    // AS112-v6 (RFC 7535)
    if segments[0] == 0x2001 && segments[1] == 4 && segments[2] == 0x0112 {
        return Err(FeedError::Http {
            message: format!("IPv6 AS112-v6 address not allowed: {ip} (2001:4:112::/48)"),
        });
    }

    // Direct Delegation AS112 Service (RFC 7534)
    if segments[0] == 0x2620 && segments[1] == 0x004f && segments[2] == 0x8000 {
        return Err(FeedError::Http {
            message: format!(
                "IPv6 Direct Delegation AS112 Service address not allowed: {ip} (2620:4f:8000::/48)"
            ),
        });
    }

    Ok(())
}

/// Validates additional IANA IPv6 special-purpose ranges not covered by
/// [`validate_ipv6_special_purpose`]: IPv6 Benchmarking (RFC 5180), the
/// deprecated ORCHID range (RFC 4843) and DRIP Entity Tags (RFC 9374) under
/// `2001::/23`, and the RFC 9637/9602 documentation and Segment Routing
/// ranges. Kept separate to keep both functions under the project's
/// function-length limit.
///
/// `2002::/16` (6to4) is deliberately not blocked here either; see the
/// comment in [`validate_ipv6`].
fn validate_ipv6_special_purpose_extended(ip: Ipv6Addr, segments: [u16; 8]) -> Result<()> {
    // Benchmarking (RFC 5180), 2001:2::/48. Distinct from 2001:1::/32 above.
    if segments[0] == 0x2001 && segments[1] == 2 && segments[2] == 0 {
        return Err(FeedError::Http {
            message: format!("IPv6 benchmarking address not allowed: {ip} (2001:2::/48)"),
        });
    }

    // Deprecated (previously ORCHID) (RFC 4843, deprecated by RFC 7343)
    if segments[0] == 0x2001 && (segments[1] & 0xFFF0) == 0x0010 {
        return Err(FeedError::Http {
            message: format!("IPv6 deprecated ORCHID address not allowed: {ip} (2001:10::/28)"),
        });
    }

    // DRIP Entity Tags (DETs) (RFC 9374)
    if segments[0] == 0x2001 && (segments[1] & 0xFFF0) == 0x0030 {
        return Err(FeedError::Http {
            message: format!("IPv6 DRIP Entity Tag address not allowed: {ip} (2001:30::/28)"),
        });
    }

    // Documentation range (RFC 9637)
    if segments[0] == 0x3fff && (segments[1] & 0xF000) == 0 {
        return Err(FeedError::Http {
            message: format!("IPv6 documentation address not allowed: {ip} (3fff::/20)"),
        });
    }

    // Segment Routing (SRv6) SIDs (RFC 9602)
    if segments[0] == 0x5f00 {
        return Err(FeedError::Http {
            message: format!("IPv6 segment routing address not allowed: {ip} (5f00::/16)"),
        });
    }

    Ok(())
}

/// Validates a domain name to prevent SSRF.
fn validate_domain(domain: &str) -> Result<()> {
    let domain_lower = domain.to_lowercase();
    // One or more trailing root-label dots (RFC 1035 §3.1, e.g. `localhost.`,
    // `localhost..`) resolve identically to the un-dotted form via every
    // standard stub resolver, so they must be stripped before the blocklist
    // checks below or they silently break the exact-match and `ends_with`
    // comparisons.
    let normalized = domain_lower.trim_end_matches('.');

    if LOCALHOST_VARIANTS.contains(&normalized) {
        return Err(FeedError::Http {
            message: format!("Localhost domain not allowed: {domain}"),
        });
    }

    for tld in INTERNAL_TLDS {
        if normalized.ends_with(tld) {
            return Err(FeedError::Http {
                message: format!("Internal domain TLD not allowed: {domain}"),
            });
        }
    }

    if METADATA_DOMAINS.contains(&normalized) {
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

    // --- #452: trailing-dot FQDN normalization ---

    #[test]
    fn test_validate_domain_rejects_trailing_dot_metadata_google() {
        assert!(validate_domain("metadata.google.internal.").is_err());
    }

    #[test]
    fn test_validate_domain_rejects_trailing_dot_localhost() {
        assert!(validate_domain("localhost.").is_err());
    }

    #[test]
    fn test_validate_domain_rejects_trailing_dot_metadata() {
        assert!(validate_domain("metadata.").is_err());
    }

    #[test]
    fn test_validate_domain_rejects_trailing_dot_internal_tld() {
        assert!(validate_domain("server.local.").is_err());
        assert!(validate_domain("server.internal.").is_err());
    }

    #[test]
    fn test_validate_domain_allows_trailing_dot_public() {
        assert!(validate_domain("example.com.").is_ok());
    }

    #[test]
    fn test_validate_domain_rejects_multiple_trailing_dots() {
        assert!(validate_domain("localhost..").is_err());
        assert!(validate_domain("metadata.google.internal..").is_err());
        assert!(validate_domain("server.internal..").is_err());
    }

    #[test]
    fn test_validate_url_rejects_trailing_dot_metadata_google() {
        assert!(validate_url("http://metadata.google.internal./").is_err());
    }

    // --- #453: additional IANA special-purpose IPv4 ranges ---

    #[test]
    fn test_validate_ipv4_rejects_multicast_boundaries() {
        assert!(validate_ipv4(Ipv4Addr::new(224, 0, 0, 0)).is_err());
        assert!(validate_ipv4(Ipv4Addr::new(239, 255, 255, 255)).is_err());
        assert!(validate_ipv4(Ipv4Addr::new(223, 255, 255, 255)).is_ok());
    }

    #[test]
    fn test_validate_ipv4_rejects_reserved_boundaries() {
        assert!(validate_ipv4(Ipv4Addr::new(240, 0, 0, 0)).is_err());
        assert!(validate_ipv4(Ipv4Addr::new(255, 255, 255, 254)).is_err());
    }

    #[test]
    fn test_validate_ipv4_rejects_ietf_protocol_assignment_boundaries() {
        assert!(validate_ipv4(Ipv4Addr::new(192, 0, 0, 0)).is_err());
        assert!(validate_ipv4(Ipv4Addr::new(192, 0, 0, 255)).is_err());
        assert!(validate_ipv4(Ipv4Addr::new(191, 255, 255, 255)).is_ok());
        assert!(validate_ipv4(Ipv4Addr::new(192, 0, 1, 0)).is_ok());
    }

    #[test]
    fn test_validate_ipv4_rejects_6to4_relay_anycast_boundaries() {
        assert!(validate_ipv4(Ipv4Addr::new(192, 88, 99, 0)).is_err());
        assert!(validate_ipv4(Ipv4Addr::new(192, 88, 99, 255)).is_err());
        assert!(validate_ipv4(Ipv4Addr::new(192, 88, 98, 255)).is_ok());
        assert!(validate_ipv4(Ipv4Addr::new(192, 88, 100, 0)).is_ok());
    }

    #[test]
    fn test_validate_ipv4_rejects_benchmarking_boundaries() {
        assert!(validate_ipv4(Ipv4Addr::new(198, 18, 0, 0)).is_err());
        assert!(validate_ipv4(Ipv4Addr::new(198, 19, 255, 255)).is_err());
        assert!(validate_ipv4(Ipv4Addr::new(198, 17, 255, 255)).is_ok());
        assert!(validate_ipv4(Ipv4Addr::new(198, 20, 0, 0)).is_ok());
    }

    // --- #453: additional IANA special-purpose IPv6 ranges ---

    #[test]
    fn test_validate_ipv6_rejects_teredo_boundaries() {
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 0, 0, 0, 0, 0, 0, 0)).is_err());
        assert!(
            validate_ipv6(Ipv6Addr::new(
                0x2001, 0, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff
            ))
            .is_err()
        );
        assert!(
            validate_ipv6(Ipv6Addr::new(
                0x2000, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff
            ))
            .is_ok()
        );
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 1, 0, 0, 0, 0, 0, 0)).is_ok());
    }

    #[test]
    fn test_validate_ipv6_rejects_orchidv2_boundaries() {
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 0x0020, 0, 0, 0, 0, 0, 0)).is_err());
        assert!(
            validate_ipv6(Ipv6Addr::new(
                0x2001, 0x002f, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff
            ))
            .is_err()
        );
        // 2001:1f::/28 and 2001:30::/28 are contiguous with ORCHIDv2 on
        // either side (2001:10::/28, 2001:30::/28; see #471), so they are
        // blocked too, just for a different reason. See
        // test_validate_ipv6_rejects_protocol_assignment_10_boundaries for
        // the genuinely free address immediately below 2001:10::/28.
        assert!(
            validate_ipv6(Ipv6Addr::new(
                0x2001, 0x001f, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff
            ))
            .is_err()
        );
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 0x0030, 0, 0, 0, 0, 0, 0)).is_err());
    }

    #[test]
    fn test_validate_ipv6_rejects_documentation_boundaries() {
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 0)).is_err());
        assert!(
            validate_ipv6(Ipv6Addr::new(
                0x2001, 0x0db8, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff
            ))
            .is_err()
        );
        assert!(
            validate_ipv6(Ipv6Addr::new(
                0x2001, 0x0db7, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff
            ))
            .is_ok()
        );
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 0x0db9, 0, 0, 0, 0, 0, 0)).is_ok());
    }

    #[test]
    fn test_validate_ipv6_rejects_discard_only_boundaries() {
        assert!(validate_ipv6(Ipv6Addr::new(0x0100, 0, 0, 0, 0, 0, 0, 0)).is_err());
        assert!(
            validate_ipv6(Ipv6Addr::new(
                0x0100, 0, 0, 0, 0xffff, 0xffff, 0xffff, 0xffff
            ))
            .is_err()
        );
        assert!(
            validate_ipv6(Ipv6Addr::new(
                0x00ff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff
            ))
            .is_ok()
        );
        // 0x0100:0:0:1::/64 is the RFC 9780 Dummy IPv6 Prefix (see #471),
        // contiguous with the discard-only prefix above; blocked too.
        assert!(validate_ipv6(Ipv6Addr::new(0x0100, 0, 0, 1, 0, 0, 0, 0)).is_err());
        assert!(validate_ipv6(Ipv6Addr::new(0x0100, 0, 0, 2, 0, 0, 0, 0)).is_ok());
    }

    #[test]
    fn test_validate_ipv6_rejects_nat64_local_use_boundaries() {
        assert!(validate_ipv6(Ipv6Addr::new(0x0064, 0xff9b, 1, 0, 0, 0, 0, 0)).is_err());
        assert!(
            validate_ipv6(Ipv6Addr::new(
                0x0064, 0xff9b, 1, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff
            ))
            .is_err()
        );
        assert!(
            validate_ipv6(Ipv6Addr::new(
                0x0064, 0xff9b, 0, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff
            ))
            .is_ok()
        );
        assert!(validate_ipv6(Ipv6Addr::new(0x0064, 0xff9b, 2, 0, 0, 0, 0, 0)).is_ok());
    }

    #[test]
    fn test_validate_ipv6_allows_6to4_deliberately_out_of_scope() {
        // 2002::/16 (RFC 3056) is deliberately not blocked; see the comment
        // in `validate_ipv6`. Pinned here as a regression guard.
        assert!(validate_ipv6(Ipv6Addr::new(0x2002, 0, 0, 0, 0, 0, 0, 0)).is_ok());
        assert!(validate_ipv6(Ipv6Addr::new(0x2002, 0x0a00, 0x0001, 0, 0, 0, 0, 0)).is_ok());
    }

    // --- #462: IANA AS112/AMT/PCP anycast sub-ranges ---

    #[test]
    fn test_validate_ipv4_rejects_as112_v4_boundaries() {
        assert!(validate_ipv4(Ipv4Addr::new(192, 31, 196, 0)).is_err());
        assert!(validate_ipv4(Ipv4Addr::new(192, 31, 196, 255)).is_err());
        assert!(validate_ipv4(Ipv4Addr::new(192, 31, 195, 255)).is_ok());
        assert!(validate_ipv4(Ipv4Addr::new(192, 31, 197, 0)).is_ok());
    }

    #[test]
    fn test_validate_ipv4_rejects_amt_boundaries() {
        assert!(validate_ipv4(Ipv4Addr::new(192, 52, 193, 0)).is_err());
        assert!(validate_ipv4(Ipv4Addr::new(192, 52, 193, 255)).is_err());
        assert!(validate_ipv4(Ipv4Addr::new(192, 52, 192, 255)).is_ok());
        assert!(validate_ipv4(Ipv4Addr::new(192, 52, 194, 0)).is_ok());
    }

    #[test]
    fn test_validate_ipv4_rejects_direct_delegation_as112_boundaries() {
        assert!(validate_ipv4(Ipv4Addr::new(192, 175, 48, 0)).is_err());
        assert!(validate_ipv4(Ipv4Addr::new(192, 175, 48, 255)).is_err());
        assert!(validate_ipv4(Ipv4Addr::new(192, 175, 47, 255)).is_ok());
        assert!(validate_ipv4(Ipv4Addr::new(192, 175, 49, 0)).is_ok());
    }

    #[test]
    fn test_validate_ipv6_rejects_pcp_anycast() {
        // No "allowed" assertion against another single address in
        // 2001:1::/32 here on purpose: that's exactly the failure mode
        // #471 was filed for (RFC 9665 registered 2001:1::3 after PR #469
        // asserted it was allowed) — a future IANA registration would
        // silently break it again.
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 1, 0, 0, 0, 0, 0, 1)).is_err());
    }

    #[test]
    fn test_validate_ipv6_rejects_turn_relay_anycast() {
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 1, 0, 0, 0, 0, 0, 2)).is_err());
    }

    // --- #471: stale assertions + remaining IANA IPv6 special-purpose gaps ---

    #[test]
    fn test_validate_ipv6_rejects_dns_sd_anycast() {
        // 2001:1::3, RFC 9665: registered after PR #469 blocked ::1/::2.
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 1, 0, 0, 0, 0, 0, 3)).is_err());
    }

    #[test]
    fn test_validate_ipv6_rejects_dummy_prefix_boundaries() {
        assert!(validate_ipv6(Ipv6Addr::new(0x0100, 0, 0, 1, 0, 0, 0, 0)).is_err());
        assert!(
            validate_ipv6(Ipv6Addr::new(
                0x0100, 0, 0, 1, 0xffff, 0xffff, 0xffff, 0xffff
            ))
            .is_err()
        );
        assert!(validate_ipv6(Ipv6Addr::new(0x0100, 0, 0, 2, 0, 0, 0, 0)).is_ok());
    }

    #[test]
    fn test_validate_ipv6_rejects_benchmarking_boundaries() {
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 2, 0, 0, 0, 0, 0, 0)).is_err());
        assert!(
            validate_ipv6(Ipv6Addr::new(
                0x2001, 2, 0, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff
            ))
            .is_err()
        );
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 2, 1, 0, 0, 0, 0, 0)).is_ok());
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 1, 0, 0, 0, 0, 0, 0)).is_ok());
    }

    #[test]
    fn test_validate_ipv6_rejects_protocol_assignment_10_boundaries() {
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 0x0010, 0, 0, 0, 0, 0, 0)).is_err());
        assert!(
            validate_ipv6(Ipv6Addr::new(
                0x2001, 0x001f, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff
            ))
            .is_err()
        );
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 0x000f, 0, 0, 0, 0, 0, 0)).is_ok());
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 0x0020, 0, 0, 0, 0, 0, 0)).is_err());
    }

    #[test]
    fn test_validate_ipv6_rejects_protocol_assignment_30_boundaries() {
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 0x0030, 0, 0, 0, 0, 0, 0)).is_err());
        assert!(
            validate_ipv6(Ipv6Addr::new(
                0x2001, 0x003f, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff
            ))
            .is_err()
        );
        // 2001:2f::/28 is contiguous with the ORCHIDv2 range (2001:20::/28)
        // immediately below 2001:30::/28, so it is blocked too, just for a
        // different reason.
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 0x002f, 0, 0, 0, 0, 0, 0)).is_err());
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 0x0040, 0, 0, 0, 0, 0, 0)).is_ok());
    }

    #[test]
    fn test_validate_ipv6_rejects_documentation_3fff_boundaries() {
        assert!(validate_ipv6(Ipv6Addr::new(0x3fff, 0, 0, 0, 0, 0, 0, 0)).is_err());
        assert!(
            validate_ipv6(Ipv6Addr::new(
                0x3fff, 0x0fff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff
            ))
            .is_err()
        );
        assert!(validate_ipv6(Ipv6Addr::new(0x3fff, 0x1000, 0, 0, 0, 0, 0, 0)).is_ok());
        assert!(validate_ipv6(Ipv6Addr::new(0x3ffe, 0xffff, 0, 0, 0, 0, 0, 0)).is_ok());
    }

    #[test]
    fn test_validate_ipv6_rejects_segment_routing_5f00_boundaries() {
        assert!(validate_ipv6(Ipv6Addr::new(0x5f00, 0, 0, 0, 0, 0, 0, 0)).is_err());
        assert!(
            validate_ipv6(Ipv6Addr::new(
                0x5f00, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff
            ))
            .is_err()
        );
        assert!(validate_ipv6(Ipv6Addr::new(0x5eff, 0xffff, 0, 0, 0, 0, 0, 0)).is_ok());
        assert!(validate_ipv6(Ipv6Addr::new(0x5f01, 0, 0, 0, 0, 0, 0, 0)).is_ok());
    }

    #[test]
    fn test_validate_ipv6_rejects_amt_boundaries() {
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 3, 0, 0, 0, 0, 0, 0)).is_err());
        assert!(
            validate_ipv6(Ipv6Addr::new(
                0x2001, 3, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff
            ))
            .is_err()
        );
        // 2001:2::/48 (segments[2] == 0) is now the RFC 5180 benchmarking
        // range (see #471); segments[2] != 0 stays outside it and AMT.
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 2, 1, 0, 0, 0, 0, 0)).is_ok());
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 4, 0, 0, 0, 0, 0, 0)).is_ok());
    }

    #[test]
    fn test_validate_ipv6_rejects_as112_v6_boundaries() {
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 4, 0x0112, 0, 0, 0, 0, 0)).is_err());
        assert!(
            validate_ipv6(Ipv6Addr::new(
                0x2001, 4, 0x0112, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff
            ))
            .is_err()
        );
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 4, 0x0111, 0, 0, 0, 0, 0)).is_ok());
        assert!(validate_ipv6(Ipv6Addr::new(0x2001, 4, 0x0113, 0, 0, 0, 0, 0)).is_ok());
    }

    #[test]
    fn test_validate_ipv6_rejects_direct_delegation_as112_boundaries() {
        assert!(validate_ipv6(Ipv6Addr::new(0x2620, 0x004f, 0x8000, 0, 0, 0, 0, 0)).is_err());
        assert!(
            validate_ipv6(Ipv6Addr::new(
                0x2620, 0x004f, 0x8000, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff
            ))
            .is_err()
        );
        assert!(validate_ipv6(Ipv6Addr::new(0x2620, 0x004f, 0x7fff, 0, 0, 0, 0, 0)).is_ok());
        assert!(validate_ipv6(Ipv6Addr::new(0x2620, 0x004f, 0x8001, 0, 0, 0, 0, 0)).is_ok());
    }
}
