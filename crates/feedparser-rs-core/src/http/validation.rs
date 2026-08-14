use crate::error::Result;
use crate::util::ssrf;
use url::Url;

/// Validates a URL to prevent Server-Side Request Forgery (SSRF) attacks
///
/// This function ensures that URLs only point to public, safe destinations.
/// It is the single validation entry point used both before sending the
/// initial request and to re-validate every redirect hop
/// (see [`crate::http::FeedHttpClient`]), and shares its rule set with
/// `xml:base` resolution via [`crate::util::base_url::is_safe_url`].
///
/// # Security Checks
///
/// 1. Only HTTP and HTTPS schemes are allowed
/// 2. Private IP ranges are blocked (RFC 1918, RFC 4193)
/// 3. Localhost and loopback addresses are blocked
/// 4. Link-local addresses are blocked (169.254.0.0/16)
/// 5. Cloud metadata endpoints are blocked
/// 6. Internal domain names are blocked (.local, .internal)
///
/// # Errors
///
/// Returns `FeedError::Http` if:
/// - The URL is malformed or invalid
/// - The URL scheme is not HTTP or HTTPS
/// - The URL points to a private IP address, localhost, or internal domain
/// - The URL points to a cloud metadata endpoint
///
/// # Examples
///
/// ```
/// use feedparser_rs::http::validation::validate_url;
///
/// // These are allowed
/// assert!(validate_url("https://example.com/feed.xml").is_ok());
/// assert!(validate_url("http://blog.example.org/rss").is_ok());
///
/// // These are blocked
/// assert!(validate_url("http://localhost/").is_err());
/// assert!(validate_url("http://192.168.1.1/").is_err());
/// assert!(validate_url("http://169.254.169.254/").is_err());
/// assert!(validate_url("file:///etc/passwd").is_err());
/// ```
pub fn validate_url(url_str: &str) -> Result<Url> {
    ssrf::validate_url(url_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Positive tests - these should pass
    #[test]
    fn test_valid_http_url() {
        assert!(validate_url("http://example.com/feed.xml").is_ok());
    }

    #[test]
    fn test_valid_https_url() {
        assert!(validate_url("https://blog.example.org/rss").is_ok());
    }

    #[test]
    fn test_valid_with_port() {
        assert!(validate_url("https://example.com:8443/feed").is_ok());
    }

    #[test]
    fn test_valid_with_path() {
        assert!(validate_url("https://example.com/path/to/feed.xml").is_ok());
    }

    // Negative tests - scheme validation
    #[test]
    fn test_reject_file_scheme() {
        assert!(validate_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn test_reject_ftp_scheme() {
        assert!(validate_url("ftp://example.com/file").is_err());
    }

    #[test]
    fn test_reject_javascript_scheme() {
        assert!(validate_url("javascript:alert(1)").is_err());
    }

    #[test]
    fn test_reject_data_scheme() {
        assert!(validate_url("data:text/html,<script>alert(1)</script>").is_err());
    }

    // Negative tests - IPv4 private ranges
    #[test]
    fn test_reject_ipv4_private_10() {
        assert!(validate_url("http://10.0.0.1/").is_err());
        assert!(validate_url("http://10.255.255.255/").is_err());
    }

    #[test]
    fn test_reject_ipv4_private_172() {
        assert!(validate_url("http://172.16.0.1/").is_err());
        assert!(validate_url("http://172.31.255.255/").is_err());
    }

    #[test]
    fn test_reject_ipv4_private_192() {
        assert!(validate_url("http://192.168.0.1/").is_err());
        assert!(validate_url("http://192.168.255.255/").is_err());
    }

    #[test]
    fn test_reject_ipv4_localhost() {
        assert!(validate_url("http://127.0.0.1/").is_err());
        assert!(validate_url("http://127.0.0.2/").is_err());
    }

    #[test]
    fn test_reject_ipv4_link_local() {
        assert!(validate_url("http://169.254.169.254/").is_err());
        assert!(validate_url("http://169.254.0.1/").is_err());
    }

    #[test]
    fn test_reject_ipv4_zero() {
        assert!(validate_url("http://0.0.0.0/").is_err());
    }

    #[test]
    fn test_reject_ipv4_broadcast() {
        assert!(validate_url("http://255.255.255.255/").is_err());
    }

    // Negative tests - IPv6
    #[test]
    fn test_reject_ipv6_loopback() {
        assert!(validate_url("http://[::1]/").is_err());
    }

    #[test]
    fn test_reject_ipv6_link_local() {
        assert!(validate_url("http://[fe80::1]/").is_err());
    }

    #[test]
    fn test_reject_ipv6_unique_local() {
        assert!(validate_url("http://[fc00::1]/").is_err());
        assert!(validate_url("http://[fd00::1]/").is_err());
    }

    // Negative tests - domain names
    #[test]
    fn test_reject_localhost_domain() {
        assert!(validate_url("http://localhost/").is_err());
    }

    #[test]
    fn test_reject_local_tld() {
        assert!(validate_url("http://myserver.local/").is_err());
    }

    #[test]
    fn test_reject_internal_tld() {
        assert!(validate_url("http://server.internal/").is_err());
    }

    #[test]
    fn test_reject_cloud_metadata() {
        assert!(validate_url("http://metadata.google.internal/").is_err());
        assert!(validate_url("http://metadata.azure.com/").is_err());
    }

    // Edge cases
    #[test]
    fn test_reject_no_host() {
        assert!(validate_url("http://").is_err());
    }

    #[test]
    fn test_reject_invalid_url() {
        assert!(validate_url("not a url").is_err());
    }

    #[test]
    fn test_public_ip_allowed() {
        // Public IPs should be allowed
        assert!(validate_url("http://8.8.8.8/").is_ok());
        assert!(validate_url("http://1.1.1.1/").is_ok());
    }

    #[test]
    fn test_carrier_grade_nat_blocked() {
        assert!(validate_url("http://100.64.0.1/").is_err());
        assert!(validate_url("http://100.127.255.255/").is_err());
    }

    #[test]
    fn test_ipv6_multicast_blocked() {
        assert!(validate_url("http://[ff00::1]/").is_err());
        assert!(validate_url("http://[ff02::1]/").is_err());
    }
}
