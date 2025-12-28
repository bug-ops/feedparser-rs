//! Example: Parse feed from local file
//!
//! Demonstrates:
//! - Reading feed from filesystem
//! - Basic feed metadata access
//! - Using type-safe Url, Email wrappers
//! - Iterating over entries
//!
//! Run with:
//! ```bash
//! cargo run --example parse_file
//! ```

use feedparser_rs::parse;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Feed Parser Example: Local File ===\n");

    // Example 1: Parse RSS 2.0 feed
    parse_rss_example()?;

    println!("\n{}\n", "=".repeat(60));

    // Example 2: Parse Atom feed
    parse_atom_example()?;

    Ok(())
}

fn parse_rss_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: RSS 2.0 Feed");
    println!("{}", "-".repeat(40));

    // Read feed from file
    let feed_path = "examples/feeds/sample_rss.xml";
    let feed_data = fs::read(feed_path)?;

    // Parse the feed
    let feed = parse(&feed_data)?;

    // Check for parsing issues (bozo pattern)
    if feed.bozo {
        println!("Warning: Feed had parsing issues");
        if let Some(exception) = &feed.bozo_exception {
            println!("Exception: {exception}");
        }
    }

    // Display feed metadata
    println!("\nFeed Metadata:");
    println!("  Version: {}", feed.version);
    println!("  Encoding: {}", feed.encoding);

    if let Some(title) = &feed.feed.title {
        println!("  Title: {title}");
    }

    // Demonstrate type-safe Url access
    if let Some(link) = &feed.feed.link {
        println!("  Link: {}", link.as_str());
        // Url derefs to str, so string methods work directly
        if link.starts_with("https://") {
            println!("  (secure URL)");
        }
    }

    if let Some(subtitle) = &feed.feed.subtitle {
        println!("  Subtitle: {subtitle}");
    }

    if let Some(language) = &feed.feed.language {
        println!("  Language: {language}");
    }

    // Display entries
    println!("\nEntries ({} total):", feed.entries.len());
    for (i, entry) in feed.entries.iter().enumerate().take(3) {
        println!("\n  Entry {}:", i + 1);

        if let Some(title) = &entry.title {
            println!("    Title: {title}");
        }

        if let Some(link) = &entry.link {
            println!("    Link: {link}");
        }

        // Demonstrate Email type access
        if let Some(author) = &entry.author {
            println!("    Author: {author}");
        }

        if let Some(published) = &entry.published {
            println!("    Published: {published}");
        }

        // Show categories/tags
        if !entry.tags.is_empty() {
            let categories: Vec<&str> = entry.tags.iter().map(|t| t.term.as_str()).collect();
            println!("    Categories: {}", categories.join(", "));
        }

        // Show enclosures (media attachments)
        if !entry.enclosures.is_empty() {
            println!("    Enclosures:");
            for enc in &entry.enclosures {
                println!("      - {}", enc.url);
                if let Some(enclosure_type) = &enc.enclosure_type {
                    println!("        Type: {enclosure_type}");
                }
                if let Some(length) = enc.length {
                    println!("        Size: {length} bytes");
                }
            }
        }
    }

    Ok(())
}

fn parse_atom_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 2: Atom Feed");
    println!("{}", "-".repeat(40));

    let feed_path = "examples/feeds/sample_atom.xml";
    let feed_data = fs::read(feed_path)?;

    let feed = parse(&feed_data)?;

    println!("\nFeed Metadata:");
    println!("  Version: {}", feed.version);

    if let Some(title) = &feed.feed.title {
        println!("  Title: {title}");
    }

    if let Some(subtitle) = &feed.feed.subtitle {
        println!("  Subtitle: {subtitle}");
    }

    // Atom feeds often have multiple authors
    if !feed.feed.authors.is_empty() {
        println!("\n  Authors:");
        for author in &feed.feed.authors {
            if let Some(name) = &author.name {
                print!("    - {name}");
            }
            if let Some(email) = &author.email {
                print!(" <{email}>");
            }
            if let Some(uri) = &author.uri {
                print!(" ({uri})");
            }
            println!();
        }
    }

    // Atom supports multiple links with different rel values
    if !feed.feed.links.is_empty() {
        println!("\n  Links:");
        for link in &feed.feed.links {
            print!("    - {}", link.href);
            if let Some(rel) = &link.rel {
                print!(" [rel={rel}]");
            }
            if let Some(link_type) = &link.link_type {
                print!(" ({link_type})");
            }
            println!();
        }
    }

    println!("\nEntries ({} total):", feed.entries.len());
    for (i, entry) in feed.entries.iter().enumerate() {
        println!("\n  Entry {}:", i + 1);

        if let Some(title) = &entry.title {
            println!("    Title: {title}");
        }

        if let Some(id) = &entry.id {
            println!("    ID: {id}");
        }

        if let Some(summary) = &entry.summary {
            println!("    Summary: {summary}");
        }

        // Atom content can have different types
        if !entry.content.is_empty() {
            let content = &entry.content[0];
            if let Some(content_type) = &content.content_type {
                println!("    Content type: {content_type}");
            }
            let value = &content.value;
            let preview = if value.len() > 100 {
                format!("{}...", &value[..100])
            } else {
                value.clone()
            };
            println!("    Content: {preview}");
        }

        if !entry.tags.is_empty() {
            let categories: Vec<&str> = entry.tags.iter().map(|t| t.term.as_str()).collect();
            println!("    Categories: {}", categories.join(", "));
        }
    }

    Ok(())
}
