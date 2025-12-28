//! Example: Parse feed from URL with HTTP fetching
//!
//! Demonstrates:
//! - Fetching feeds from HTTP/HTTPS URLs
//! - Conditional GET with ETag/Last-Modified caching
//! - HTTP metadata handling
//! - Error handling for network issues
//!
//! Requires the 'http' feature (enabled by default)
//!
//! Run with:
//! ```bash
//! cargo run --example parse_url
//! ```

use feedparser_rs::parse_url;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Feed Parser Example: HTTP Fetching ===\n");

    // Example 1: Simple URL fetch
    simple_fetch_example()?;

    println!("\n{}\n", "=".repeat(60));

    // Example 2: Conditional GET with caching
    conditional_get_example()?;

    Ok(())
}

fn simple_fetch_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Simple URL Fetch");
    println!("{}", "-".repeat(40));

    // Use a real public RSS feed
    // BBC News RSS feed is reliable and publicly available
    let url = "https://feeds.bbci.co.uk/news/rss.xml";

    println!("Fetching feed from: {url}");
    println!("Please wait...\n");

    match parse_url(url, None, None, Some("feedparser-rs-example/1.0")) {
        Ok(feed) => {
            println!("Success!\n");

            // HTTP metadata
            if let Some(status) = feed.status {
                println!("HTTP Status: {status}");
            }

            if let Some(href) = &feed.href {
                println!("Final URL: {href}");
            }

            if let Some(etag) = &feed.etag {
                println!("ETag: {etag}");
            }

            if let Some(modified) = &feed.modified {
                println!("Last-Modified: {modified}");
            }

            println!("\nFeed Metadata:");
            println!("  Version: {}", feed.version);
            println!("  Encoding: {}", feed.encoding);

            if let Some(title) = &feed.feed.title {
                println!("  Title: {title}");
            }

            if let Some(link) = &feed.feed.link {
                println!("  Link: {link}");
            }

            if let Some(subtitle) = &feed.feed.subtitle {
                let preview = if subtitle.len() > 100 {
                    format!("{}...", &subtitle[..100])
                } else {
                    subtitle.clone()
                };
                println!("  Subtitle: {preview}");
            }

            println!("\nLatest Entries (first 5):");
            for (i, entry) in feed.entries.iter().enumerate().take(5) {
                println!(
                    "\n  {}. {}",
                    i + 1,
                    entry.title.as_deref().unwrap_or("[No title]")
                );

                if let Some(link) = &entry.link {
                    println!("     {link}");
                }

                if let Some(published) = &entry.published {
                    println!("     Published: {published}");
                }
            }

            println!("\nTotal entries: {}", feed.entries.len());
        }
        Err(e) => {
            eprintln!("Error fetching feed: {e}");
            eprintln!("\nNote: This example requires internet connectivity.");
            eprintln!("If you're offline, try the parse_file example instead.");
            return Err(e.into());
        }
    }

    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn conditional_get_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 2: Conditional GET with Caching");
    println!("{}", "-".repeat(40));

    // Use NPR News RSS feed (another reliable public feed)
    let url = "https://feeds.npr.org/1001/rss.xml";

    println!("First fetch (no cache):");
    println!("Fetching from: {url}");

    let first_fetch = match parse_url(url, None, None, Some("feedparser-rs-example/1.0")) {
        Ok(feed) => feed,
        Err(e) => {
            eprintln!("Error: {e}");
            eprintln!("Skipping conditional GET example (requires internet)");
            return Ok(());
        }
    };

    println!("Success!");
    if let Some(title) = &first_fetch.feed.title {
        println!("  Title: {title}");
    }

    // Save caching headers
    let etag = first_fetch.etag.clone();
    let modified = first_fetch.modified;

    println!("\nCaching headers received:");
    if let Some(ref e) = etag {
        println!("  ETag: {e}");
    }
    if let Some(ref m) = modified {
        println!("  Last-Modified: {m}");
    }

    // Second fetch with caching headers
    println!("\nSecond fetch (with cache validation):");
    println!("Sending If-None-Match and If-Modified-Since headers...");

    match parse_url(
        url,
        etag.as_deref(),
        modified.as_deref(),
        Some("feedparser-rs-example/1.0"),
    ) {
        Ok(second_fetch) => {
            if second_fetch.status == Some(304) {
                println!("\nHTTP 304 Not Modified");
                println!("Feed hasn't changed since last fetch.");
                println!("Use cached version to save bandwidth!");
            } else if second_fetch.status == Some(200) {
                println!("\nHTTP 200 OK");
                println!("Feed was modified, received new content.");
                println!("Entries: {}", second_fetch.entries.len());
            }
        }
        Err(e) => {
            eprintln!("Error on second fetch: {e}");
        }
    }

    println!("\nBandwidth Savings:");
    println!("- First fetch: Full download");
    println!("- Second fetch: Headers only (if 304)");
    println!("- Typical savings: 90%+ for unchanged feeds");

    Ok(())
}
