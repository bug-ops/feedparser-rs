//! Base URL resolution for xml:base support
//!
//! This module provides URL resolution following RFC 3986, supporting
//! the `xml:base` attribute used in Atom and some RSS feeds.

use std::net::IpAddr;
use url::Url;

/// Validates that a URL is safe for external use (no SSRF risks)
///
/// This function checks for common SSRF attack vectors including:
/// - Non-HTTP(S) schemes (file://, data://, etc.)
/// - Localhost addresses (127.0.0.1, `::1`, localhost)
/// - Private IP ranges (192.168.x.x, 10.x.x.x, 172.16-31.x.x)
/// - Cloud metadata endpoints (169.254.169.254)
///
/// # Arguments
///
/// * `url` - The URL to validate
///
/// # Returns
///
/// `true` if the URL is safe to use, `false` if it poses SSRF risks
///
/// # Examples
///
/// ```
/// use feedparser_rs::util::base_url::is_safe_url;
///
/// // Safe URLs
/// assert!(is_safe_url("http://example.com/"));
/// assert!(is_safe_url("https://github.com/"));
///
/// // Unsafe URLs
/// assert!(!is_safe_url("file:///etc/passwd"));
/// assert!(!is_safe_url("http://localhost/"));
/// assert!(!is_safe_url("http://192.168.1.1/"));
/// assert!(!is_safe_url("http://169.254.169.254/"));
/// ```
#[must_use]
pub fn is_safe_url(url: &str) -> bool {
    let Ok(parsed) = Url::parse(url) else {
        return false;
    };

    // Only allow http and https schemes
    match parsed.scheme() {
        "http" | "https" => {}
        _ => return false,
    }

    // Check the host using url::Host enum which properly handles IP addresses
    if let Some(host) = parsed.host() {
        match host {
            url::Host::Domain(domain) => {
                // Reject localhost domain
                if domain == "localhost" {
                    return false;
                }

                // Reject cloud metadata endpoints
                if domain == "metadata.google.internal" {
                    return false;
                }
            }
            url::Host::Ipv4(ipv4) => {
                let ip = IpAddr::V4(ipv4);
                // Reject localhost and private IPs
                if ip.is_loopback() || is_private_ip(&ip) {
                    return false;
                }

                // Reject cloud metadata IP
                let octets = ipv4.octets();
                if octets == [169, 254, 169, 254] {
                    return false;
                }
            }
            url::Host::Ipv6(ipv6) => {
                let ip = IpAddr::V6(ipv6);
                // Reject localhost and private IPs
                if ip.is_loopback() || is_private_ip(&ip) {
                    return false;
                }
            }
        }
    }

    true
}

/// Checks if an IP address is in a private range
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            octets[0] == 10
                || (octets[0] == 172 && (16..=31).contains(&octets[1]))
                || (octets[0] == 192 && octets[1] == 168)
                || octets[0] == 127
        }
        IpAddr::V6(ipv6) => {
            ipv6.is_loopback() || ipv6.is_unspecified() || (ipv6.segments()[0] & 0xfe00) == 0xfc00
        }
    }
}

/// Resolves a potentially relative URL against a base URL
///
/// If `href` is already absolute, returns it unchanged.
/// If `base` is None or invalid, returns `href` unchanged.
/// Otherwise, resolves `href` relative to `base`.
///
/// # Arguments
///
/// * `href` - The URL to resolve (may be relative or absolute)
/// * `base` - The base URL to resolve against (may be None)
///
/// # Returns
///
/// The resolved URL as a string
///
/// # Examples
///
/// ```
/// use feedparser_rs::util::base_url::resolve_url;
///
/// // Absolute URLs are returned unchanged
/// assert_eq!(
///     resolve_url("http://example.com/page", Some("http://other.com/")),
///     "http://example.com/page"
/// );
///
/// // Relative URLs are resolved against the base
/// assert_eq!(
///     resolve_url("page.html", Some("http://example.com/dir/")),
///     "http://example.com/dir/page.html"
/// );
///
/// // Without a base, relative URLs are returned unchanged
/// assert_eq!(resolve_url("page.html", None), "page.html");
/// ```
#[must_use]
pub fn resolve_url(href: &str, base: Option<&str>) -> String {
    // If href is already absolute, return it
    if href.starts_with("http://")
        || href.starts_with("https://")
        || href.starts_with("mailto:")
        || href.starts_with("tel:")
    {
        return href.to_string();
    }

    // If no base URL, return href unchanged
    let Some(base_str) = base else {
        return href.to_string();
    };

    // Try to parse base URL
    let Ok(base_url) = Url::parse(base_str) else {
        return href.to_string();
    };

    // Resolve href against base
    base_url
        .join(href)
        .map_or_else(|_| href.to_string(), |resolved| resolved.to_string())
}

/// Combines two base URLs, with child overriding parent
///
/// This handles nested `xml:base` attributes where a child element's
/// base URL may be relative to its parent's base.
///
/// # Arguments
///
/// * `parent_base` - The parent element's base URL (may be None)
/// * `child_base` - The child element's xml:base value (may be None)
///
/// # Returns
///
/// The effective base URL for the child element, or None if no base is set
///
/// # Examples
///
/// ```
/// use feedparser_rs::util::base_url::combine_bases;
///
/// // Child absolute base overrides parent
/// assert_eq!(
///     combine_bases(Some("http://parent.com/"), Some("http://child.com/")),
///     Some("http://child.com/".to_string())
/// );
///
/// // Child relative base is resolved against parent
/// assert_eq!(
///     combine_bases(Some("http://example.com/feed/"), Some("items/")),
///     Some("http://example.com/feed/items/".to_string())
/// );
///
/// // No child base, parent is used
/// assert_eq!(
///     combine_bases(Some("http://example.com/"), None),
///     Some("http://example.com/".to_string())
/// );
///
/// // No bases at all
/// assert_eq!(combine_bases(None, None), None);
/// ```
#[must_use]
pub fn combine_bases(parent_base: Option<&str>, child_base: Option<&str>) -> Option<String> {
    match (parent_base, child_base) {
        (_, Some(child)) => {
            // Child has a base - resolve it against parent if parent exists
            Some(resolve_url(child, parent_base))
        }
        (Some(parent), None) => Some(parent.to_string()),
        (None, None) => None,
    }
}

/// Context for tracking base URLs during parsing
///
/// This struct maintains the current base URL context and provides
/// methods for URL resolution within a parsing context.
#[derive(Debug, Clone, Default)]
pub struct BaseUrlContext {
    /// The current effective base URL
    base: Option<String>,
}

impl BaseUrlContext {
    /// Creates a new context with no base URL
    #[must_use]
    pub const fn new() -> Self {
        Self { base: None }
    }

    /// Creates a new context with an initial base URL
    #[must_use]
    pub fn with_base(base: impl Into<String>) -> Self {
        Self {
            base: Some(base.into()),
        }
    }

    /// Gets the current base URL
    #[must_use]
    pub fn base(&self) -> Option<&str> {
        self.base.as_deref()
    }

    /// Updates the base URL with a new xml:base value
    ///
    /// The new base is resolved against the current base if it's relative.
    pub fn update_base(&mut self, xml_base: &str) {
        let new_base = resolve_url(xml_base, self.base.as_deref());
        self.base = Some(new_base);
    }

    /// Resolves a URL against the current base
    #[must_use]
    pub fn resolve(&self, href: &str) -> String {
        resolve_url(href, self.base.as_deref())
    }

    /// Resolves a URL against the current base with SSRF protection
    ///
    /// This method performs URL resolution and validates the result to prevent
    /// Server-Side Request Forgery (SSRF) attacks via malicious xml:base attributes.
    ///
    /// # Security
    ///
    /// If the resolved URL fails SSRF safety checks (localhost, private IPs,
    /// dangerous schemes), the original `href` is returned unchanged instead
    /// of the resolved URL.
    ///
    /// # Arguments
    ///
    /// * `href` - The URL to resolve (may be relative or absolute)
    ///
    /// # Returns
    ///
    /// The resolved URL if safe, otherwise the original `href`
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::util::base_url::BaseUrlContext;
    ///
    /// // Safe URL resolution
    /// let ctx = BaseUrlContext::with_base("http://example.com/");
    /// assert_eq!(ctx.resolve_safe("page.html"), "http://example.com/page.html");
    ///
    /// // SSRF blocked - returns original href
    /// let dangerous_ctx = BaseUrlContext::with_base("http://localhost/");
    /// assert_eq!(dangerous_ctx.resolve_safe("admin"), "admin");
    /// ```
    #[must_use]
    pub fn resolve_safe(&self, href: &str) -> String {
        let resolved = self.resolve(href);

        // Use lowercase for case-insensitive scheme comparison (RFC 3986)
        let resolved_lower = resolved.to_lowercase();

        // Block dangerous schemes (file://, data://, javascript://, etc.)
        // Case-insensitive to prevent bypass via FILE://, JAVASCRIPT:, etc.
        if resolved_lower.starts_with("file://")
            || resolved_lower.starts_with("data:")
            || resolved_lower.starts_with("javascript:")
            || resolved_lower.starts_with("ftp://")
            || resolved_lower.starts_with("gopher://")
        {
            // Dangerous scheme - return original href
            return href.to_string();
        }

        // Validate HTTP(S) URLs for SSRF
        if resolved_lower.starts_with("http://") || resolved_lower.starts_with("https://") {
            if is_safe_url(&resolved) {
                resolved
            } else {
                // SSRF blocked - check if href itself is an unsafe absolute URL
                // If href is an absolute URL pointing to dangerous target, return empty
                // Otherwise return original relative href (safe since it requires base to resolve)
                let href_is_unsafe_absolute = Url::parse(href).is_ok_and(|parsed_href| {
                    let is_http_scheme = matches!(parsed_href.scheme(), "http" | "https");
                    is_http_scheme && !is_safe_url(href)
                });

                if href_is_unsafe_absolute {
                    String::new()
                } else {
                    href.to_string()
                }
            }
        } else {
            // Other schemes (mailto:, tel:) or relative URLs pass through
            resolved
        }
    }

    /// Creates a child context inheriting this context's base
    #[must_use]
    pub fn child(&self) -> Self {
        Self {
            base: self.base.clone(),
        }
    }

    /// Creates a child context with an additional xml:base
    #[must_use]
    pub fn child_with_base(&self, xml_base: &str) -> Self {
        let new_base = combine_bases(self.base.as_deref(), Some(xml_base));
        Self { base: new_base }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_absolute_url() {
        assert_eq!(
            resolve_url("http://example.com/page", Some("http://other.com/")),
            "http://example.com/page"
        );
        assert_eq!(
            resolve_url("https://example.com/page", Some("http://other.com/")),
            "https://example.com/page"
        );
    }

    #[test]
    fn test_resolve_relative_url() {
        assert_eq!(
            resolve_url("page.html", Some("http://example.com/dir/")),
            "http://example.com/dir/page.html"
        );
        assert_eq!(
            resolve_url("/absolute/path", Some("http://example.com/dir/")),
            "http://example.com/absolute/path"
        );
        assert_eq!(
            resolve_url("../sibling/page", Some("http://example.com/dir/sub/")),
            "http://example.com/dir/sibling/page"
        );
    }

    #[test]
    fn test_resolve_without_base() {
        assert_eq!(resolve_url("page.html", None), "page.html");
        assert_eq!(
            resolve_url("http://example.com", None),
            "http://example.com"
        );
    }

    #[test]
    fn test_resolve_invalid_base() {
        assert_eq!(
            resolve_url("page.html", Some("not a valid url")),
            "page.html"
        );
    }

    #[test]
    fn test_resolve_special_schemes() {
        assert_eq!(
            resolve_url("mailto:test@example.com", Some("http://example.com/")),
            "mailto:test@example.com"
        );
        assert_eq!(
            resolve_url("tel:+1234567890", Some("http://example.com/")),
            "tel:+1234567890"
        );
    }

    #[test]
    fn test_combine_bases_child_absolute() {
        assert_eq!(
            combine_bases(Some("http://parent.com/"), Some("http://child.com/")),
            Some("http://child.com/".to_string())
        );
    }

    #[test]
    fn test_combine_bases_child_relative() {
        assert_eq!(
            combine_bases(Some("http://example.com/feed/"), Some("items/")),
            Some("http://example.com/feed/items/".to_string())
        );
    }

    #[test]
    fn test_combine_bases_no_child() {
        assert_eq!(
            combine_bases(Some("http://example.com/"), None),
            Some("http://example.com/".to_string())
        );
    }

    #[test]
    fn test_combine_bases_no_parent() {
        assert_eq!(
            combine_bases(None, Some("http://example.com/")),
            Some("http://example.com/".to_string())
        );
    }

    #[test]
    fn test_combine_bases_none() {
        assert_eq!(combine_bases(None, None), None);
    }

    #[test]
    fn test_context_new() {
        let ctx = BaseUrlContext::new();
        assert!(ctx.base().is_none());
    }

    #[test]
    fn test_context_with_base() {
        let ctx = BaseUrlContext::with_base("http://example.com/");
        assert_eq!(ctx.base(), Some("http://example.com/"));
    }

    #[test]
    fn test_context_update_base() {
        let mut ctx = BaseUrlContext::with_base("http://example.com/feed/");
        ctx.update_base("items/");
        assert_eq!(ctx.base(), Some("http://example.com/feed/items/"));
    }

    #[test]
    fn test_context_resolve() {
        let ctx = BaseUrlContext::with_base("http://example.com/feed/");
        assert_eq!(
            ctx.resolve("item.html"),
            "http://example.com/feed/item.html"
        );
        assert_eq!(ctx.resolve("http://other.com/"), "http://other.com/");
    }

    #[test]
    fn test_context_child() {
        let parent = BaseUrlContext::with_base("http://example.com/");
        let child = parent.child();
        assert_eq!(child.base(), Some("http://example.com/"));
    }

    #[test]
    fn test_context_child_with_base() {
        let parent = BaseUrlContext::with_base("http://example.com/feed/");
        let child = parent.child_with_base("items/");
        assert_eq!(child.base(), Some("http://example.com/feed/items/"));
    }

    #[test]
    fn test_fragment_preservation() {
        assert_eq!(
            resolve_url("#section", Some("http://example.com/page.html")),
            "http://example.com/page.html#section"
        );
    }

    #[test]
    fn test_query_string_preservation() {
        assert_eq!(
            resolve_url("?query=value", Some("http://example.com/page.html")),
            "http://example.com/page.html?query=value"
        );
    }

    #[test]
    fn test_empty_href() {
        // Empty href should resolve to base URL itself
        assert_eq!(
            resolve_url("", Some("http://example.com/page.html")),
            "http://example.com/page.html"
        );
    }

    // SSRF Protection Tests
    #[test]
    fn test_is_safe_url_file_scheme() {
        assert!(!is_safe_url("file:///etc/passwd"));
        assert!(!is_safe_url("file:///C:/Windows/System32/config/sam"));
    }

    #[test]
    fn test_is_safe_url_localhost() {
        assert!(!is_safe_url("http://localhost/"));
        assert!(!is_safe_url("http://127.0.0.1/"));
        assert!(!is_safe_url("http://[::1]/"));
        assert!(!is_safe_url("https://localhost:8080/api"));
    }

    #[test]
    fn test_is_safe_url_private_ip() {
        // 192.168.x.x range
        assert!(!is_safe_url("http://192.168.1.1/"));
        assert!(!is_safe_url("http://192.168.0.1/"));
        assert!(!is_safe_url("http://192.168.255.255/"));

        // 10.x.x.x range
        assert!(!is_safe_url("http://10.0.0.1/"));
        assert!(!is_safe_url("http://10.255.255.255/"));

        // 172.16-31.x.x range
        assert!(!is_safe_url("http://172.16.0.1/"));
        assert!(!is_safe_url("http://172.31.255.255/"));
        assert!(!is_safe_url("http://172.20.10.5/"));

        // 127.x.x.x range
        assert!(!is_safe_url("http://127.0.0.2/"));
        assert!(!is_safe_url("http://127.255.255.255/"));
    }

    #[test]
    fn test_is_safe_url_cloud_metadata() {
        assert!(!is_safe_url("http://169.254.169.254/"));
        assert!(!is_safe_url("http://169.254.169.254/latest/meta-data/"));
        assert!(!is_safe_url("http://metadata.google.internal/"));
    }

    #[test]
    fn test_is_safe_url_valid_urls() {
        assert!(is_safe_url("http://example.com/"));
        assert!(is_safe_url("https://github.com/"));
        assert!(is_safe_url("http://1.1.1.1/"));
        assert!(is_safe_url("https://8.8.8.8/"));
        assert!(is_safe_url("http://example.com:8080/path"));
    }

    #[test]
    fn test_is_safe_url_other_schemes() {
        assert!(!is_safe_url("ftp://example.com/"));
        assert!(!is_safe_url("data:text/html,<script>alert('xss')</script>"));
        assert!(!is_safe_url("javascript:alert('xss')"));
        assert!(!is_safe_url("gopher://example.com/"));
    }

    #[test]
    fn test_is_safe_url_ipv6() {
        // Loopback
        assert!(!is_safe_url("http://[::1]/"));
        assert!(!is_safe_url("http://[0:0:0:0:0:0:0:1]/"));

        // Private ULA (fc00::/7)
        assert!(!is_safe_url("http://[fc00::1]/"));
        assert!(!is_safe_url("http://[fd00::1]/"));

        // Public IPv6 should be allowed
        assert!(is_safe_url("http://[2001:4860:4860::8888]/"));
    }

    #[test]
    fn test_is_safe_url_invalid_urls() {
        assert!(!is_safe_url("not a url"));
        assert!(!is_safe_url(""));
        assert!(!is_safe_url("://invalid"));
    }
}
