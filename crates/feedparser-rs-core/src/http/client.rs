use super::resolver::SsrfSafeResolver;
use super::response::FeedHttpResponse;
use super::validation::validate_url;
use crate::error::{FeedError, Result};
use reqwest::blocking::{Client, Response};
use reqwest::header::{
    ACCEPT, ACCEPT_ENCODING, HeaderMap, HeaderName, HeaderValue, IF_MODIFIED_SINCE, IF_NONE_MATCH,
    USER_AGENT,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Maximum number of redirects to follow before aborting, matching the hop
/// count `reqwest::redirect::Policy::limited` used to enforce. A custom
/// policy does not get this for free, so [`FeedHttpClient::redirect_policy`]
/// enforces it explicitly alongside SSRF re-validation.
const MAX_REDIRECTS: usize = 10;

/// HTTP client for fetching feeds
pub struct FeedHttpClient {
    client: Client,
    user_agent: String,
    timeout: Duration,
}

impl FeedHttpClient {
    /// Creates a new HTTP client with default settings
    ///
    /// Default settings:
    /// - 30 second timeout
    /// - Gzip, deflate, and brotli compression enabled
    /// - Maximum 10 redirects, each re-validated against the SSRF checks
    /// - DNS resolution re-validated against the SSRF checks to close
    ///   DNS-rebinding gaps between validation and connect time
    /// - No system/environment proxy (`HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`)
    ///   is honored: a proxy connects through a hostname that is resolved by
    ///   the proxy itself, not by the DNS-rebinding-safe resolver above,
    ///   which would silently defeat that protection
    /// - Custom User-Agent
    ///
    /// # Errors
    ///
    /// Returns `FeedError::Http` if the underlying HTTP client cannot be created.
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .gzip(true)
            .deflate(true)
            .brotli(true)
            .redirect(Self::redirect_policy())
            .dns_resolver(Arc::new(SsrfSafeResolver))
            .no_proxy()
            .build()
            .map_err(|e| FeedError::Http {
                message: format!("Failed to create HTTP client: {e}"),
            })?;

        Ok(Self {
            client,
            user_agent: format!(
                "feedparser-rs/{} (+https://github.com/bug-ops/feedparser-rs)",
                env!("CARGO_PKG_VERSION")
            ),
            timeout: Duration::from_secs(30),
        })
    }

    /// Decides whether a redirect hop should be followed.
    ///
    /// Extracted from [`Self::redirect_policy`] so the decision can be unit
    /// tested directly: `reqwest::redirect::Attempt` has no public
    /// constructor, so the policy closure itself cannot be exercised without
    /// a live HTTP round trip.
    ///
    /// # Errors
    ///
    /// Returns `FeedError::Http` if the hop count exceeds `MAX_REDIRECTS`
    /// or `next_url` fails SSRF validation.
    fn should_follow_redirect(next_url: &str, previous_hops: usize) -> Result<()> {
        if previous_hops > MAX_REDIRECTS {
            return Err(FeedError::Http {
                message: format!("Too many redirects (max {MAX_REDIRECTS})"),
            });
        }

        validate_url(next_url)?;
        Ok(())
    }

    /// Formats a `reqwest::Error` including its full `source()` chain.
    ///
    /// `reqwest::Error`'s `Display` only prints the outer error kind and URL —
    /// it never walks the source chain. Without this, the SSRF rejection
    /// reason attached via `redirect::Attempt::error` (see
    /// [`Self::redirect_policy`]) or a resolver failure (see
    /// `resolver::SsrfSafeResolver`) is silently dropped, and callers only
    /// see a generic "error following redirect" / "error sending request".
    fn describe_request_error(error: &reqwest::Error) -> String {
        use std::fmt::Write as _;

        let mut message = error.to_string();
        let mut source = std::error::Error::source(error);
        while let Some(err) = source {
            let _ = write!(message, ": {err}");
            source = err.source();
        }
        message
    }

    /// Builds a redirect policy that re-validates every hop against the SSRF
    /// checks in [`validate_url`], not just the initial request URL.
    ///
    /// `reqwest`'s built-in `Policy::limited` only counts hops — it never
    /// re-runs SSRF validation on the `Location` header, so a malicious
    /// server could pass the initial check and then redirect to an internal
    /// address (e.g. cloud metadata endpoints) and have it followed
    /// silently. A custom policy also does not enforce a hop limit on its
    /// own, so [`Self::should_follow_redirect`] replicates `Policy::limited`'s
    /// bound explicitly.
    fn redirect_policy() -> reqwest::redirect::Policy {
        reqwest::redirect::Policy::custom(|attempt| {
            match Self::should_follow_redirect(attempt.url().as_str(), attempt.previous().len()) {
                Ok(()) => attempt.follow(),
                Err(e) => attempt.error(e),
            }
        })
    }

    /// Sets a custom User-Agent header
    ///
    /// # Security
    ///
    /// User-Agent is truncated to 512 bytes to prevent header injection attacks.
    #[must_use]
    pub fn with_user_agent(mut self, agent: String) -> Self {
        // Truncate to 512 bytes to prevent header injection
        const MAX_USER_AGENT_LEN: usize = 512;
        self.user_agent = if agent.len() > MAX_USER_AGENT_LEN {
            agent.chars().take(MAX_USER_AGENT_LEN).collect()
        } else {
            agent
        };
        self
    }

    /// Sets request timeout
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Insert header with consistent error handling
    ///
    /// Helper method to reduce boilerplate in header insertion.
    #[inline]
    fn insert_header(
        headers: &mut HeaderMap,
        name: HeaderName,
        value: &str,
        field_name: &str,
    ) -> Result<()> {
        headers.insert(
            name,
            HeaderValue::from_str(value).map_err(|e| FeedError::Http {
                message: format!("Invalid {field_name}: {e}"),
            })?,
        );
        Ok(())
    }

    /// Fetches a feed from the given URL
    ///
    /// Supports conditional GET with `ETag` and `Last-Modified` headers.
    ///
    /// # Arguments
    ///
    /// * `url` - HTTP/HTTPS URL to fetch
    /// * `etag` - Optional `ETag` from previous fetch
    /// * `modified` - Optional `Last-Modified` from previous fetch
    /// * `extra_headers` - Additional custom headers
    ///
    /// # Errors
    ///
    /// Returns `FeedError::Http` if the request fails or headers are invalid.
    pub fn get(
        &self,
        url: &str,
        etag: Option<&str>,
        modified: Option<&str>,
        extra_headers: Option<&HeaderMap>,
    ) -> Result<FeedHttpResponse> {
        // Validate URL to prevent SSRF attacks
        let validated_url = validate_url(url)?;
        let url_str = validated_url.as_str();

        let mut headers = HeaderMap::new();

        // Standard headers
        Self::insert_header(&mut headers, USER_AGENT, &self.user_agent, "User-Agent")?;

        headers.insert(
            ACCEPT,
            HeaderValue::from_static(
                "application/rss+xml, application/atom+xml, application/xml, text/xml, */*",
            ),
        );

        headers.insert(
            ACCEPT_ENCODING,
            HeaderValue::from_static("gzip, deflate, br"),
        );

        // Conditional GET headers with length validation
        if let Some(etag_val) = etag {
            // Truncate ETag to 1KB to prevent oversized headers
            const MAX_ETAG_LEN: usize = 1024;
            let sanitized_etag = if etag_val.len() > MAX_ETAG_LEN {
                &etag_val[..MAX_ETAG_LEN]
            } else {
                etag_val
            };
            Self::insert_header(&mut headers, IF_NONE_MATCH, sanitized_etag, "ETag")?;
        }

        if let Some(modified_val) = modified {
            // Truncate Last-Modified to 64 bytes (RFC 822 dates are ~30 bytes)
            const MAX_MODIFIED_LEN: usize = 64;
            let sanitized_modified = if modified_val.len() > MAX_MODIFIED_LEN {
                &modified_val[..MAX_MODIFIED_LEN]
            } else {
                modified_val
            };
            Self::insert_header(
                &mut headers,
                IF_MODIFIED_SINCE,
                sanitized_modified,
                "Last-Modified",
            )?;
        }

        // Merge extra headers
        if let Some(extra) = extra_headers {
            headers.extend(extra.clone());
        }

        let response = self
            .client
            .get(url_str)
            .headers(headers)
            .send()
            .map_err(|e| FeedError::Http {
                message: format!("HTTP request failed: {}", Self::describe_request_error(&e)),
            })?;

        Self::build_response(response, url_str)
    }

    /// Converts `reqwest` Response to `FeedHttpResponse`
    fn build_response(response: Response, _original_url: &str) -> Result<FeedHttpResponse> {
        let status = response.status().as_u16();
        let url = response.url().to_string();

        // Convert headers to HashMap with pre-allocated capacity
        let mut headers_map = HashMap::with_capacity(response.headers().len());
        for (name, value) in response.headers() {
            if let Ok(val_str) = value.to_str() {
                headers_map.insert(name.to_string(), val_str.to_string());
            }
        }

        // Extract caching headers
        let etag = headers_map.get("etag").cloned();
        let last_modified = headers_map.get("last-modified").cloned();
        let content_type = headers_map.get("content-type").cloned();

        // Extract encoding from Content-Type
        let encoding = content_type
            .as_ref()
            .and_then(|ct| FeedHttpResponse::extract_charset_from_content_type(ct));

        // Read body (handles gzip/deflate automatically)
        let body = if status == 304 {
            // Not Modified - no body
            Vec::new()
        } else {
            response
                .bytes()
                .map_err(|e| FeedError::Http {
                    message: format!("Failed to read response body: {e}"),
                })?
                .to_vec()
        };

        Ok(FeedHttpResponse {
            status,
            url,
            headers: headers_map,
            body,
            etag,
            last_modified,
            content_type,
            encoding,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = FeedHttpClient::new();
        assert!(client.is_ok());
    }

    // === Redirect re-validation tests ===

    #[test]
    fn test_redirect_rejects_metadata_endpoint() {
        let result =
            FeedHttpClient::should_follow_redirect("http://169.254.169.254/latest/meta-data/", 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_redirect_rejects_private_ip() {
        let result = FeedHttpClient::should_follow_redirect("http://10.0.0.5/admin", 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_redirect_allows_public_ip_within_hop_limit() {
        let result = FeedHttpClient::should_follow_redirect("http://8.8.8.8/", 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_redirect_rejects_over_hop_limit_even_for_safe_url() {
        let result = FeedHttpClient::should_follow_redirect("http://8.8.8.8/", MAX_REDIRECTS + 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_redirect_allows_at_hop_limit() {
        let result = FeedHttpClient::should_follow_redirect("http://8.8.8.8/", MAX_REDIRECTS);
        assert!(result.is_ok());
    }

    #[test]
    #[allow(clippy::significant_drop_tightening)]
    fn test_redirect_to_metadata_endpoint_rejected_end_to_end() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/redirect")
            .with_status(302)
            .with_header("location", "http://169.254.169.254/latest/meta-data/")
            .create();

        // Bypasses the front-door `validate_url` (which would reject the
        // mockito server's loopback address) to exercise only the redirect
        // policy against a real HTTP redirect chain.
        let client = Client::builder()
            .redirect(FeedHttpClient::redirect_policy())
            .build()
            .unwrap();

        let url = format!("{}/redirect", server.url());
        let result = client.get(&url).send();

        let err = result.expect_err("redirect to a metadata IP must be rejected");
        // A broken policy would also error here (169.254.169.254 is
        // unroutable from the test host), so assert on the actual SSRF
        // rejection reason surviving through `describe_request_error`, not
        // just on any error — proving the *redirect policy itself* is what
        // rejected it, not a downstream connection failure.
        let description = FeedHttpClient::describe_request_error(&err);
        assert!(
            description.contains("Link-local address not allowed"),
            "expected the SSRF rejection reason in the error chain, got: {description}"
        );
        mock.assert();
    }

    #[test]
    #[allow(clippy::significant_drop_tightening)]
    fn test_redirect_follows_legitimate_multi_hop_chain() {
        let mut server = mockito::Server::new();
        let addr = server.socket_address();

        let mock1 = server
            .mock("GET", "/hop1")
            .with_status(302)
            .with_header("location", "http://public.test/hop2")
            .create();
        let mock2 = server
            .mock("GET", "/hop2")
            .with_status(302)
            .with_header("location", "http://public.test/hop3")
            .create();
        let mock3 = server
            .mock("GET", "/hop3")
            .with_status(200)
            .with_body("ok")
            .create();

        // `.resolve()` points the SSRF-passing domain "public.test" at the
        // mock server's real (loopback) address, so the full policy +
        // resolver stack can be driven end-to-end offline without the
        // front door or `SsrfSafeResolver` rejecting the mock's actual
        // loopback IP — proving legitimate multi-hop chains still work.
        let client = Client::builder()
            .redirect(FeedHttpClient::redirect_policy())
            .dns_resolver(Arc::new(SsrfSafeResolver))
            .resolve("public.test", addr)
            .build()
            .unwrap();

        let url = format!("http://public.test:{}/hop1", addr.port());
        let response = client.get(&url).send().unwrap();

        assert_eq!(response.status().as_u16(), 200);
        mock1.assert();
        mock2.assert();
        mock3.assert();
    }

    #[test]
    #[allow(clippy::significant_drop_tightening)]
    fn test_redirect_chain_rejects_metadata_after_legitimate_hops() {
        let mut server = mockito::Server::new();
        let addr = server.socket_address();

        let mock1 = server
            .mock("GET", "/hop1")
            .with_status(302)
            .with_header("location", "http://public.test/hop2")
            .create();
        let mock2 = server
            .mock("GET", "/hop2")
            .with_status(302)
            .with_header("location", "http://169.254.169.254/latest/meta-data/")
            .create();

        let client = Client::builder()
            .redirect(FeedHttpClient::redirect_policy())
            .dns_resolver(Arc::new(SsrfSafeResolver))
            .resolve("public.test", addr)
            .build()
            .unwrap();

        // This is issue #436's actual attack shape: an initial URL and its
        // first redirect both look legitimate, only the final hop targets
        // a cloud metadata address.
        let url = format!("http://public.test:{}/hop1", addr.port());
        let err = client.get(&url).send().expect_err(
            "a chain ending at a metadata IP must be rejected, even after legitimate hops",
        );

        let description = FeedHttpClient::describe_request_error(&err);
        assert!(
            description.contains("Link-local address not allowed"),
            "expected the SSRF rejection reason to survive a multi-hop chain, got: {description}"
        );
        mock1.assert();
        mock2.assert();
    }

    #[test]
    fn test_dns_resolver_wired_into_client_rejects_loopback() {
        let client = Client::builder()
            .dns_resolver(Arc::new(SsrfSafeResolver))
            .build()
            .unwrap();

        // Exercises the resolver as actually attached to a `Client` (not
        // just `SsrfSafeResolver::resolve` in isolation): "localhost"
        // resolves via the OS hosts file to 127.0.0.1/::1, both filtered
        // out, so the connection must fail before any bytes are sent.
        let result = client.get("http://localhost/").send();
        assert!(result.is_err());
    }

    #[test]
    fn test_custom_user_agent() {
        let client = FeedHttpClient::new()
            .unwrap()
            .with_user_agent("CustomBot/1.0".to_string());
        assert_eq!(client.user_agent, "CustomBot/1.0");
    }

    #[test]
    fn test_custom_timeout() {
        let timeout = Duration::from_secs(60);
        let client = FeedHttpClient::new().unwrap().with_timeout(timeout);
        assert_eq!(client.timeout, timeout);
    }

    // SSRF protection tests
    #[test]
    fn test_reject_localhost_url() {
        let client = FeedHttpClient::new().unwrap();
        let result = client.get("http://localhost/feed.xml", None, None, None);
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("Localhost domain not allowed"));
    }

    #[test]
    fn test_reject_private_ip() {
        let client = FeedHttpClient::new().unwrap();
        let result = client.get("http://192.168.1.1/feed.xml", None, None, None);
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("Private IP address not allowed"));
    }

    #[test]
    fn test_reject_metadata_endpoint() {
        let client = FeedHttpClient::new().unwrap();
        let result = client.get("http://169.254.169.254/latest/meta-data/", None, None, None);
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        // Should be rejected as AWS metadata endpoint or link-local
        assert!(err_msg.contains("metadata") || err_msg.contains("Link-local"));
    }

    #[test]
    fn test_reject_file_scheme() {
        let client = FeedHttpClient::new().unwrap();
        let result = client.get("file:///etc/passwd", None, None, None);
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("Unsupported URL scheme"));
    }

    #[test]
    fn test_reject_internal_domain() {
        let client = FeedHttpClient::new().unwrap();
        let result = client.get("http://server.local/feed.xml", None, None, None);
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("Internal domain TLD not allowed"));
    }

    #[test]
    fn test_insert_header_valid() {
        let mut headers = HeaderMap::new();
        let result =
            FeedHttpClient::insert_header(&mut headers, USER_AGENT, "TestBot/1.0", "User-Agent");
        assert!(result.is_ok());
        assert_eq!(headers.get(USER_AGENT).unwrap(), "TestBot/1.0");
    }

    #[test]
    fn test_insert_header_invalid_value() {
        let mut headers = HeaderMap::new();
        // Invalid header value with control characters
        let result = FeedHttpClient::insert_header(
            &mut headers,
            USER_AGENT,
            "Invalid\nHeader",
            "User-Agent",
        );
        assert!(result.is_err());
        match result {
            Err(FeedError::Http { message }) => {
                assert!(message.contains("Invalid User-Agent"));
            }
            _ => panic!("Expected Http error"),
        }
    }

    #[test]
    fn test_insert_header_multiple_headers() {
        let mut headers = HeaderMap::new();

        FeedHttpClient::insert_header(&mut headers, USER_AGENT, "TestBot/1.0", "User-Agent")
            .unwrap();

        FeedHttpClient::insert_header(&mut headers, ACCEPT, "application/xml", "Accept").unwrap();

        assert_eq!(headers.len(), 2);
        assert_eq!(headers.get(USER_AGENT).unwrap(), "TestBot/1.0");
        assert_eq!(headers.get(ACCEPT).unwrap(), "application/xml");
    }
}
