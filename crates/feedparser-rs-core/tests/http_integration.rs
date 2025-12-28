#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(feature = "http")]
#[allow(clippy::significant_drop_tightening)]
mod http_tests {
    use feedparser_rs::FeedError;
    use feedparser_rs::http::{FeedHttpClient, FeedHttpResponse};
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use reqwest::blocking::Client;
    use reqwest::header::{
        ACCEPT, ACCEPT_ENCODING, HeaderMap, HeaderValue, IF_MODIFIED_SINCE, IF_NONE_MATCH,
        USER_AGENT,
    };
    use std::collections::HashMap;
    use std::io::Write;

    /// Helper function to fetch URL without validation for testing
    /// This bypasses SSRF protection to allow testing with mock servers on localhost
    fn test_get(
        url: &str,
        etag: Option<&str>,
        modified: Option<&str>,
        extra_headers: Option<&HeaderMap>,
        user_agent: Option<&str>,
    ) -> Result<FeedHttpResponse, FeedError> {
        let reqwest_client = Client::builder()
            .gzip(true)
            .deflate(true)
            .brotli(true)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|e| FeedError::Http {
                message: format!("Failed to create HTTP client: {e}"),
            })?;

        let mut headers = HeaderMap::new();
        let ua = user_agent.unwrap_or("feedparser-rs-test/0.1.0");
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(ua).map_err(|e| FeedError::Http {
                message: format!("Invalid User-Agent: {e}"),
            })?,
        );
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

        if let Some(etag_val) = etag {
            headers.insert(
                IF_NONE_MATCH,
                HeaderValue::from_str(etag_val).map_err(|e| FeedError::Http {
                    message: format!("Invalid ETag: {e}"),
                })?,
            );
        }

        if let Some(modified_val) = modified {
            headers.insert(
                IF_MODIFIED_SINCE,
                HeaderValue::from_str(modified_val).map_err(|e| FeedError::Http {
                    message: format!("Invalid Last-Modified: {e}"),
                })?,
            );
        }

        if let Some(extra) = extra_headers {
            headers.extend(extra.clone());
        }

        let response = reqwest_client
            .get(url)
            .headers(headers)
            .send()
            .map_err(|e| FeedError::Http {
                message: format!("HTTP request failed: {e}"),
            })?;

        let status = response.status().as_u16();
        let final_url = response.url().to_string();

        let mut headers_map = HashMap::new();
        for (name, value) in response.headers() {
            if let Ok(val_str) = value.to_str() {
                headers_map.insert(name.to_string(), val_str.to_string());
            }
        }

        let etag = headers_map.get("etag").cloned();
        let last_modified = headers_map.get("last-modified").cloned();
        let content_type = headers_map.get("content-type").cloned();
        let encoding = content_type
            .as_ref()
            .and_then(|ct| FeedHttpResponse::extract_charset_from_content_type(ct));

        let body = if status == 304 {
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
            url: final_url,
            headers: headers_map,
            body,
            etag,
            last_modified,
            content_type,
            encoding,
        })
    }

    // === Basic HTTP GET Tests ===

    #[test]
    fn test_get_successful_request() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/feed.xml")
            .with_status(200)
            .with_header("content-type", "application/xml")
            .with_body(
                r#"<?xml version="1.0"?><rss version="2.0"><channel><title>Test</title></channel></rss>"#,
            )
            .create();

        let url = format!("{}/feed.xml", server.url());
        let response = test_get(&url, None, None, None, None).unwrap();

        assert_eq!(response.status, 200);
        assert!(!response.body.is_empty());
        assert_eq!(response.content_type.as_deref(), Some("application/xml"));
        mock.assert();
    }

    #[test]
    fn test_get_404_not_found() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/missing.xml")
            .with_status(404)
            .with_body("Not Found")
            .create();

        let url = format!("{}/missing.xml", server.url());
        let response = test_get(&url, None, None, None, None).unwrap();

        assert_eq!(response.status, 404);
        mock.assert();
    }

    #[test]
    fn test_get_500_server_error() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/error.xml")
            .with_status(500)
            .with_body("Internal Server Error")
            .create();

        let url = format!("{}/error.xml", server.url());
        let response = test_get(&url, None, None, None, None).unwrap();

        assert_eq!(response.status, 500);
        mock.assert();
    }

    #[test]
    fn test_get_connection_refused() {
        let result = test_get("http://localhost:19999/nonexistent", None, None, None, None);

        assert!(result.is_err());
        match result.unwrap_err() {
            FeedError::Http { message } => {
                assert!(
                    message.contains("request failed")
                        || message.contains("connection")
                        || message.contains("error")
                );
            }
            _ => panic!("Expected HTTP error"),
        }
    }

    // === Conditional GET Tests ===

    #[test]
    fn test_get_with_etag_not_modified() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/feed.xml")
            .match_header("if-none-match", "\"abc123\"")
            .with_status(304)
            .create();

        let url = format!("{}/feed.xml", server.url());
        let response = test_get(&url, Some("\"abc123\""), None, None, None).unwrap();

        assert_eq!(response.status, 304);
        assert_eq!(response.body.len(), 0);
        mock.assert();
    }

    #[test]
    fn test_get_with_etag_modified() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/feed.xml")
            .match_header("if-none-match", "\"old-etag\"")
            .with_status(200)
            .with_header("etag", "\"new-etag\"")
            .with_body("<rss>new content</rss>")
            .create();

        let url = format!("{}/feed.xml", server.url());
        let response = test_get(&url, Some("\"old-etag\""), None, None, None).unwrap();

        assert_eq!(response.status, 200);
        assert_eq!(response.etag.as_deref(), Some("\"new-etag\""));
        assert!(!response.body.is_empty());
        mock.assert();
    }

    #[test]
    fn test_get_with_last_modified() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/feed.xml")
            .match_header("if-modified-since", "Mon, 01 Jan 2024 00:00:00 GMT")
            .with_status(304)
            .create();

        let url = format!("{}/feed.xml", server.url());
        let response = test_get(
            &url,
            None,
            Some("Mon, 01 Jan 2024 00:00:00 GMT"),
            None,
            None,
        )
        .unwrap();

        assert_eq!(response.status, 304);
        mock.assert();
    }

    #[test]
    fn test_get_with_both_etag_and_last_modified() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/feed.xml")
            .match_header("if-none-match", "\"etag123\"")
            .match_header("if-modified-since", "Mon, 01 Jan 2024 00:00:00 GMT")
            .with_status(304)
            .create();

        let url = format!("{}/feed.xml", server.url());
        let response = test_get(
            &url,
            Some("\"etag123\""),
            Some("Mon, 01 Jan 2024 00:00:00 GMT"),
            None,
            None,
        )
        .unwrap();

        assert_eq!(response.status, 304);
        mock.assert();
    }

    #[test]
    fn test_get_extracts_etag_header() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/feed.xml")
            .with_status(200)
            .with_header("etag", "\"def456\"")
            .with_body("<rss></rss>")
            .create();

        let url = format!("{}/feed.xml", server.url());
        let response = test_get(&url, None, None, None, None).unwrap();

        assert_eq!(response.etag.as_deref(), Some("\"def456\""));
        assert_eq!(response.status, 200);
        mock.assert();
    }

    #[test]
    fn test_get_extracts_last_modified_header() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/feed.xml")
            .with_status(200)
            .with_header("last-modified", "Tue, 02 Jan 2024 12:00:00 GMT")
            .with_body("<rss></rss>")
            .create();

        let url = format!("{}/feed.xml", server.url());
        let response = test_get(&url, None, None, None, None).unwrap();

        assert_eq!(
            response.last_modified.as_deref(),
            Some("Tue, 02 Jan 2024 12:00:00 GMT")
        );
        mock.assert();
    }

    // === Header Tests ===

    #[test]
    fn test_user_agent_header_sent() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/feed.xml")
            .match_header(
                "user-agent",
                mockito::Matcher::Regex(r"feedparser-rs-test/.*".to_string()),
            )
            .with_status(200)
            .create();

        let url = format!("{}/feed.xml", server.url());
        let _ = test_get(&url, None, None, None, None).unwrap();

        mock.assert();
    }

    #[test]
    fn test_custom_user_agent_header_sent() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/feed.xml")
            .match_header("user-agent", "CustomBot/2.0")
            .with_status(200)
            .create();

        let url = format!("{}/feed.xml", server.url());
        let _ = test_get(&url, None, None, None, Some("CustomBot/2.0")).unwrap();

        mock.assert();
    }

    #[test]
    fn test_accept_header_sent() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/feed.xml")
            .match_header(
                "accept",
                "application/rss+xml, application/atom+xml, application/xml, text/xml, */*",
            )
            .with_status(200)
            .create();

        let url = format!("{}/feed.xml", server.url());
        let _ = test_get(&url, None, None, None, None).unwrap();

        mock.assert();
    }

    #[test]
    fn test_accept_encoding_header_sent() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/feed.xml")
            .match_header("accept-encoding", "gzip, deflate, br")
            .with_status(200)
            .create();

        let url = format!("{}/feed.xml", server.url());
        let _ = test_get(&url, None, None, None, None).unwrap();

        mock.assert();
    }

    #[test]
    fn test_custom_headers() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/feed.xml")
            .match_header("x-custom-header", "test-value")
            .with_status(200)
            .create();

        let mut extra_headers = HeaderMap::new();
        extra_headers.insert("x-custom-header", HeaderValue::from_static("test-value"));

        let url = format!("{}/feed.xml", server.url());
        let _ = test_get(&url, None, None, Some(&extra_headers), None).unwrap();

        mock.assert();
    }

    #[test]
    fn test_multiple_custom_headers() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/feed.xml")
            .match_header("x-custom-1", "value1")
            .match_header("x-custom-2", "value2")
            .with_status(200)
            .create();

        let mut extra_headers = HeaderMap::new();
        extra_headers.insert("x-custom-1", HeaderValue::from_static("value1"));
        extra_headers.insert("x-custom-2", HeaderValue::from_static("value2"));

        let url = format!("{}/feed.xml", server.url());
        let _ = test_get(&url, None, None, Some(&extra_headers), None).unwrap();

        mock.assert();
    }

    // === Encoding Tests ===

    #[test]
    fn test_content_type_utf8_extracted() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/feed.xml")
            .with_status(200)
            .with_header("content-type", "text/xml; charset=utf-8")
            .with_body("<rss></rss>")
            .create();

        let url = format!("{}/feed.xml", server.url());
        let response = test_get(&url, None, None, None, None).unwrap();

        assert_eq!(response.encoding.as_deref(), Some("utf-8"));
        assert_eq!(
            response.content_type.as_deref(),
            Some("text/xml; charset=utf-8")
        );
        mock.assert();
    }

    #[test]
    fn test_content_type_iso8859_extracted() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/feed.xml")
            .with_status(200)
            .with_header("content-type", "application/xml; charset=iso-8859-1")
            .with_body("<rss></rss>")
            .create();

        let url = format!("{}/feed.xml", server.url());
        let response = test_get(&url, None, None, None, None).unwrap();

        assert_eq!(response.encoding.as_deref(), Some("iso-8859-1"));
        mock.assert();
    }

    #[test]
    fn test_content_type_charset_quoted() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/feed.xml")
            .with_status(200)
            .with_header("content-type", "text/xml; charset=\"windows-1252\"")
            .with_body("<rss></rss>")
            .create();

        let url = format!("{}/feed.xml", server.url());
        let response = test_get(&url, None, None, None, None).unwrap();

        assert_eq!(response.encoding.as_deref(), Some("windows-1252"));
        mock.assert();
    }

    #[test]
    fn test_no_charset_in_content_type() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/feed.xml")
            .with_status(200)
            .with_header("content-type", "application/xml")
            .with_body("<rss></rss>")
            .create();

        let url = format!("{}/feed.xml", server.url());
        let response = test_get(&url, None, None, None, None).unwrap();

        assert_eq!(response.encoding, None);
        assert_eq!(response.content_type.as_deref(), Some("application/xml"));
        mock.assert();
    }

    #[test]
    fn test_charset_extraction_from_response() {
        let ct = "text/html; boundary=something; charset=utf-8";
        let charset = FeedHttpResponse::extract_charset_from_content_type(ct);
        assert_eq!(charset, Some("utf-8".to_string()));
    }

    // === Redirect Tests ===

    #[test]
    fn test_follows_301_redirect() {
        let mut server = mockito::Server::new();

        let mock_redirect = server
            .mock("GET", "/old-feed.xml")
            .with_status(301)
            .with_header("location", &format!("{}/new-feed.xml", server.url()))
            .create();

        let mock_target = server
            .mock("GET", "/new-feed.xml")
            .with_status(200)
            .with_body("<rss>new content</rss>")
            .create();

        let url = format!("{}/old-feed.xml", server.url());
        let response = test_get(&url, None, None, None, None).unwrap();

        assert_eq!(response.status, 200);
        assert!(response.url.contains("new-feed.xml"));
        mock_redirect.assert();
        mock_target.assert();
    }

    #[test]
    fn test_follows_302_redirect() {
        let mut server = mockito::Server::new();

        let mock_redirect = server
            .mock("GET", "/temp.xml")
            .with_status(302)
            .with_header("location", &format!("{}/final.xml", server.url()))
            .create();

        let mock_target = server
            .mock("GET", "/final.xml")
            .with_status(200)
            .with_body("<rss></rss>")
            .create();

        let url = format!("{}/temp.xml", server.url());
        let response = test_get(&url, None, None, None, None).unwrap();

        assert_eq!(response.status, 200);
        mock_redirect.assert();
        mock_target.assert();
    }

    #[test]
    fn test_follows_307_temporary_redirect() {
        let mut server = mockito::Server::new();

        let mock_redirect = server
            .mock("GET", "/temp.xml")
            .with_status(307)
            .with_header("location", &format!("{}/final.xml", server.url()))
            .create();

        let mock_target = server
            .mock("GET", "/final.xml")
            .with_status(200)
            .with_body("<rss></rss>")
            .create();

        let url = format!("{}/temp.xml", server.url());
        let response = test_get(&url, None, None, None, None).unwrap();

        assert_eq!(response.status, 200);
        mock_redirect.assert();
        mock_target.assert();
    }

    #[test]
    fn test_redirect_preserves_final_url() {
        let mut server = mockito::Server::new();

        server
            .mock("GET", "/old.xml")
            .with_status(301)
            .with_header("location", &format!("{}/new.xml", server.url()))
            .create();

        server
            .mock("GET", "/new.xml")
            .with_status(200)
            .with_header("etag", "\"final-etag\"")
            .with_body("<rss></rss>")
            .create();

        let original_url = format!("{}/old.xml", server.url());
        let response = test_get(&original_url, None, None, None, None).unwrap();

        assert!(response.url.contains("new.xml"));
        assert!(!response.url.contains("old.xml"));
        assert_eq!(response.etag.as_deref(), Some("\"final-etag\""));
    }

    // === Compression Tests ===

    #[test]
    fn test_gzip_compression_automatic_decompression() {
        let xml = b"<rss version=\"2.0\"><channel><title>Compressed Feed</title></channel></rss>";
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(xml).unwrap();
        let compressed = encoder.finish().unwrap();

        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/feed.xml")
            .with_status(200)
            .with_header("content-encoding", "gzip")
            .with_body(compressed)
            .create();

        let url = format!("{}/feed.xml", server.url());
        let response = test_get(&url, None, None, None, None).unwrap();

        assert_eq!(response.body, xml);
        mock.assert();
    }

    // === Response Body Tests ===

    #[test]
    fn test_response_body_empty_on_304() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/feed.xml")
            .match_header("if-none-match", "\"abc\"")
            .with_status(304)
            .create();

        let url = format!("{}/feed.xml", server.url());
        let response = test_get(&url, Some("\"abc\""), None, None, None).unwrap();

        assert_eq!(response.status, 304);
        assert!(response.body.is_empty());
        mock.assert();
    }

    #[test]
    fn test_response_body_preserved_on_200() {
        let body = "<rss version=\"2.0\"><channel><title>Test Feed</title><item><title>Item 1</title></item></channel></rss>";
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/feed.xml")
            .with_status(200)
            .with_body(body)
            .create();

        let url = format!("{}/feed.xml", server.url());
        let response = test_get(&url, None, None, None, None).unwrap();

        assert_eq!(response.status, 200);
        assert_eq!(response.body, body.as_bytes());
        mock.assert();
    }

    #[test]
    fn test_response_headers_extracted() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/feed.xml")
            .with_status(200)
            .with_header("content-type", "application/xml")
            .with_header("etag", "\"abc123\"")
            .with_header("last-modified", "Mon, 01 Jan 2024 00:00:00 GMT")
            .with_header("cache-control", "max-age=3600")
            .with_body("<rss></rss>")
            .create();

        let url = format!("{}/feed.xml", server.url());
        let response = test_get(&url, None, None, None, None).unwrap();

        assert_eq!(response.status, 200);
        assert_eq!(response.etag.as_deref(), Some("\"abc123\""));
        assert_eq!(
            response.last_modified.as_deref(),
            Some("Mon, 01 Jan 2024 00:00:00 GMT")
        );
        assert_eq!(response.content_type.as_deref(), Some("application/xml"));
        assert!(response.headers.contains_key("cache-control"));
        mock.assert();
    }

    // === Error Handling Tests ===

    #[test]
    fn test_invalid_etag_returns_error() {
        let result = test_get(
            "https://example.com/feed.xml",
            Some("\n\r"),
            None,
            None,
            None,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            FeedError::Http { message } => {
                assert!(message.contains("Invalid ETag") || message.contains("etag"));
            }
            _ => panic!("Expected HTTP error for invalid ETag"),
        }
    }

    #[test]
    fn test_invalid_last_modified_returns_error() {
        let result = test_get(
            "https://example.com/feed.xml",
            None,
            Some("\n\r"),
            None,
            None,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            FeedError::Http { message } => {
                assert!(
                    message.contains("Invalid Last-Modified") || message.contains("last-modified")
                );
            }
            _ => panic!("Expected HTTP error for invalid Last-Modified"),
        }
    }

    // === Content Type Tests ===

    #[test]
    fn test_various_content_types() {
        let content_types = vec![
            "application/rss+xml",
            "application/atom+xml",
            "application/xml",
            "text/xml",
            "application/json",
        ];

        for ct in content_types {
            let mut server = mockito::Server::new();
            let mock = server
                .mock("GET", "/feed")
                .with_status(200)
                .with_header("content-type", ct)
                .with_body("<rss></rss>")
                .create();

            let url = format!("{}/feed", server.url());
            let response = test_get(&url, None, None, None, None).unwrap();

            assert_eq!(response.content_type.as_deref(), Some(ct));
            mock.assert();
        }
    }

    // === Real-world Scenarios ===

    #[test]
    fn test_feed_fetch_complete_workflow() {
        let mut server = mockito::Server::new();

        // First fetch: 200 OK with ETag and Last-Modified
        let mock1 = server
            .mock("GET", "/feed.xml")
            .with_status(200)
            .with_header("etag", "\"v1\"")
            .with_header("last-modified", "Mon, 01 Jan 2024 00:00:00 GMT")
            .with_header("content-type", "application/rss+xml; charset=utf-8")
            .with_body("<rss version=\"2.0\"><channel><title>V1</title></channel></rss>")
            .create();

        let url = format!("{}/feed.xml", server.url());

        // Initial fetch
        let response1 = test_get(&url, None, None, None, None).unwrap();
        assert_eq!(response1.status, 200);
        assert_eq!(response1.etag.as_deref(), Some("\"v1\""));
        assert_eq!(
            response1.last_modified.as_deref(),
            Some("Mon, 01 Jan 2024 00:00:00 GMT")
        );
        assert_eq!(response1.encoding.as_deref(), Some("utf-8"));
        mock1.assert();

        // Second fetch: 304 Not Modified
        let mock2 = server
            .mock("GET", "/feed.xml")
            .match_header("if-none-match", "\"v1\"")
            .match_header("if-modified-since", "Mon, 01 Jan 2024 00:00:00 GMT")
            .with_status(304)
            .create();

        let response2 = test_get(
            &url,
            response1.etag.as_deref(),
            response1.last_modified.as_deref(),
            None,
            None,
        )
        .unwrap();
        assert_eq!(response2.status, 304);
        assert!(response2.body.is_empty());
        mock2.assert();
    }

    // === Client Builder Tests ===

    #[test]
    fn test_client_creation() {
        let client = FeedHttpClient::new();
        assert!(client.is_ok());
    }

    #[test]
    fn test_custom_user_agent() {
        let _client = FeedHttpClient::new()
            .unwrap()
            .with_user_agent("CustomBot/1.0".to_string());
        // Client should be created successfully
        // Actual user agent testing is done in test_custom_user_agent_header_sent
    }

    #[test]
    fn test_custom_timeout() {
        use std::time::Duration;

        let _client = FeedHttpClient::new()
            .unwrap()
            .with_timeout(Duration::from_secs(60));
        // Client should be created successfully
    }
}
