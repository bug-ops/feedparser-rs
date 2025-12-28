//! Example: Error handling and the bozo pattern
//!
//! Demonstrates:
//! - The bozo flag for malformed feeds
//! - Graceful error recovery
//! - Extracting data from broken feeds
//! - Different types of parsing errors
//! - Resource limits protection
//!
//! The "bozo" pattern (from Python feedparser) means:
//! - Never panic on malformed input
//! - Set bozo=true flag when issues occur
//! - Continue parsing and extract whatever data is available
//!
//! Run with:
//! ```bash
//! cargo run --example error_handling
//! ```

use feedparser_rs::{ParserLimits, parse, parse_with_limits};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Feed Parser Example: Error Handling ===\n");

    // Example 1: Parse malformed feed (bozo pattern)
    malformed_feed_example()?;

    println!("\n{}\n", "=".repeat(60));

    // Example 2: Resource limits
    resource_limits_example();

    println!("\n{}\n", "=".repeat(60));

    // Example 3: Invalid XML recovery
    invalid_xml_example();

    println!("\n{}\n", "=".repeat(60));

    // Example 4: Network error handling
    network_error_example();

    Ok(())
}

fn malformed_feed_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Malformed Feed (Bozo Pattern)");
    println!("{}", "-".repeat(40));

    let feed_path = "examples/feeds/malformed_feed.xml";
    let feed_data = fs::read(feed_path)?;

    println!("Parsing feed with known issues...\n");

    // The parser will NOT panic, even with malformed XML
    let feed = parse(&feed_data)?;

    // Check the bozo flag
    println!("Bozo flag: {}", feed.bozo);

    if feed.bozo {
        println!("Feed has parsing issues!");

        if let Some(exception) = &feed.bozo_exception {
            println!("Exception details: {exception}");
        }

        println!("\nDespite errors, we can still extract data:");
    }

    // Even with errors, we can extract available data
    if let Some(title) = &feed.feed.title {
        println!("  Feed title: {title}");
    }

    if let Some(link) = &feed.feed.link {
        println!("  Feed link: {link}");
    }

    println!("\nEntries found: {}", feed.entries.len());
    for (i, entry) in feed.entries.iter().enumerate() {
        println!("\n  Entry {}:", i + 1);
        if let Some(title) = &entry.title {
            println!("    Title: {title}");
        }
        if let Some(link) = &entry.link {
            println!("    Link: {link}");
        }
        if let Some(summary) = &entry.summary {
            println!("    Summary: {summary}");
        }

        // Some entries may have unparseable dates
        if let Some(published) = &entry.published {
            println!("    Published: {published}");
        } else {
            println!("    Published: (unable to parse date)");
        }
    }

    println!("\nKey takeaway: The parser extracts as much data as possible,");
    println!("even when the feed has errors. Always check the bozo flag!");

    Ok(())
}

fn resource_limits_example() {
    println!("Example 2: Resource Limits Protection");
    println!("{}", "-".repeat(40));

    // Create a feed that exceeds limits
    let huge_feed = format!(
        r#"<?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>{}</title>
            <link>https://example.com</link>
          </channel>
        </rss>"#,
        "A".repeat(200_000)
    );

    println!("Testing with strict limits:");
    let strict_limits = ParserLimits::strict();
    println!("  Max text length: {}", strict_limits.max_text_length);
    println!("  Max entries: {}", strict_limits.max_entries);

    match parse_with_limits(huge_feed.as_bytes(), strict_limits) {
        Ok(feed) => {
            println!("\nParsed with limits:");
            if let Some(title) = &feed.feed.title {
                println!("  Title length: {} chars (may be truncated)", title.len());
            }
        }
        Err(e) => {
            println!("\nLimits exceeded: {e}");
            println!("This protects against DoS attacks and resource exhaustion.");
        }
    }

    // Now try with default (more permissive) limits
    println!("\n\nTesting with default limits:");
    let default_limits = ParserLimits::default();
    println!("  Max text length: {}", default_limits.max_text_length);

    match parse_with_limits(huge_feed.as_bytes(), default_limits) {
        Ok(feed) => {
            println!("\nParsed successfully:");
            if let Some(title) = &feed.feed.title {
                println!("  Title length: {} chars", title.len());
            }
        }
        Err(e) => {
            println!("Error: {e}");
        }
    }

    println!("\nUse strict limits for untrusted input!");
    println!("Use default limits for known/trusted feeds.");
}

fn invalid_xml_example() {
    println!("Example 3: Invalid XML Recovery");
    println!("{}", "-".repeat(40));

    // Various types of invalid XML
    let test_cases = vec![
        (
            "Unclosed tag",
            b"<rss version='2.0'><channel><title>Test</channel></rss>".as_slice(),
        ),
        (
            "Invalid entity",
            b"<rss version='2.0'><channel><title>Test &#xFFFF;</title></channel></rss>".as_slice(),
        ),
        (
            "Missing required elements",
            b"<rss version='2.0'><channel></channel></rss>".as_slice(),
        ),
    ];

    for (name, xml) in test_cases {
        println!("\nTest case: {name}");
        print!("  ");

        match parse(xml) {
            Ok(feed) => {
                if feed.bozo {
                    println!("Parsed with bozo flag set");
                    if let Some(ex) = &feed.bozo_exception {
                        println!("  Exception: {ex}");
                    }
                } else {
                    println!("Parsed successfully");
                }

                // Show what we recovered
                if feed.feed.title.is_some() {
                    println!("  Recovered title: {:?}", feed.feed.title);
                }
            }
            Err(e) => {
                // Some errors are unrecoverable
                println!("Unrecoverable error: {e}");
            }
        }
    }

    println!("\n\nThe parser attempts to recover from common XML errors");
    println!("and extract as much information as possible.");
}

fn network_error_example() {
    println!("Example 4: Network Error Handling");
    println!("{}", "-".repeat(40));

    #[cfg(feature = "http")]
    {
        use feedparser_rs::parse_url;

        println!("Testing various network scenarios:\n");

        // Test case 1: Invalid URL
        println!("1. Invalid URL:");
        match parse_url("not-a-valid-url", None, None, None) {
            Ok(_) => println!("   Unexpected success"),
            Err(e) => println!("   Error (expected): {e}"),
        }

        // Test case 2: Non-existent domain
        println!("\n2. Non-existent domain:");
        match parse_url(
            "https://this-domain-definitely-does-not-exist-12345.com/feed.xml",
            None,
            None,
            None,
        ) {
            Ok(_) => println!("   Unexpected success"),
            Err(e) => println!("   Error (expected): {e}"),
        }

        // Test case 3: 404 Not Found
        println!("\n3. HTTP 404:");
        match parse_url("https://httpbin.org/status/404", None, None, None) {
            Ok(_) => println!("   Unexpected success"),
            Err(e) => println!("   Error (expected): {e}"),
        }

        println!("\n\nProper error handling:");
        println!("- Use Result type for all fallible operations");
        println!("- Match on specific error types for better UX");
        println!("- Provide helpful error messages to users");
        println!("- Implement retry logic for transient failures");
        println!("- Use timeouts to prevent hanging");
    }

    #[cfg(not(feature = "http"))]
    {
        println!("HTTP feature not enabled.");
        println!("Enable with: cargo run --example error_handling --features http");
    }
}
