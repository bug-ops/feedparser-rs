//! Example: Parse podcast feed with iTunes and Podcast 2.0 metadata
//!
//! Demonstrates:
//! - iTunes podcast namespace (itunes:*)
//! - Podcast 2.0 namespace (podcast:*)
//! - Episode metadata and chapters
//! - Podcast categories and artwork
//! - Duration parsing and explicit flags
//!
//! Run with:
//! ```bash
//! cargo run --example podcast_feed
//! ```

use feedparser_rs::parse;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Feed Parser Example: Podcast Feed ===\n");

    let feed_path = "examples/feeds/sample_podcast.xml";
    let feed_data = fs::read(feed_path)?;

    let feed = parse(&feed_data)?;

    println!("Feed Version: {}", feed.version);
    println!("{}", "=".repeat(60));

    // Display podcast feed-level metadata
    display_podcast_metadata(&feed);

    println!("\n{}\n", "=".repeat(60));

    // Display episode details
    display_episodes(&feed);

    Ok(())
}

fn display_podcast_metadata(feed: &feedparser_rs::ParsedFeed) {
    println!("Podcast Metadata:");
    println!("{}", "-".repeat(40));

    if let Some(title) = &feed.feed.title {
        println!("\nTitle: {title}");
    }

    if let Some(subtitle) = &feed.feed.subtitle {
        println!("Subtitle: {subtitle}");
    }

    // iTunes-specific metadata
    if let Some(itunes) = &feed.feed.itunes {
        println!("\niTunes Metadata:");

        if let Some(author) = &itunes.author {
            println!("  Author: {author}");
        }

        // Owner information
        if let Some(owner) = &itunes.owner {
            println!("  Owner:");
            if let Some(name) = &owner.name {
                println!("    Name: {name}");
            }
            if let Some(email) = &owner.email {
                println!("    Email: {email}");
            }
        }

        // Explicit content flag
        if let Some(explicit) = itunes.explicit {
            println!("  Explicit: {}", if explicit { "YES" } else { "NO" });
        }

        // Artwork
        if let Some(image) = &itunes.image {
            println!("  Artwork: {image}");
        }

        // Categories (iTunes podcasts can have nested categories)
        if !itunes.categories.is_empty() {
            println!("  Categories:");
            for cat in &itunes.categories {
                print!("    - {}", cat.text);
                if let Some(subcategory) = &cat.subcategory {
                    print!(" > {subcategory}");
                }
                println!();
            }
        }

        if let Some(podcast_type) = &itunes.podcast_type {
            println!("  Type: {podcast_type}");
        }

        if itunes.complete == Some(true) {
            println!("  Status: Complete (no more episodes will be released)");
        }
    }

    // Podcast 2.0 metadata
    if let Some(podcast) = &feed.feed.podcast {
        println!("\nPodcast 2.0 Metadata:");

        // Funding/support information
        if !podcast.funding.is_empty() {
            println!("  Funding:");
            for funding in &podcast.funding {
                print!("    - {}", funding.url);
                if let Some(message) = &funding.message {
                    print!(": {message}");
                }
                println!();
            }
        }

        // People involved (hosts, guests, etc.)
        if !podcast.persons.is_empty() {
            println!("  People:");
            for person in &podcast.persons {
                print!("    - {}", person.name);
                if let Some(role) = &person.role {
                    print!(" [{role}]");
                }
                if let Some(img) = &person.img {
                    print!(" (photo: {img})");
                }
                println!();
            }
        }

        // Value for value (cryptocurrency support)
        if let Some(value) = &podcast.value {
            println!("  Value4Value:");
            println!("    Type: {}", value.type_);
            println!("    Method: {}", value.method);
            if !value.recipients.is_empty() {
                println!("    Recipients:");
                for recipient in &value.recipients {
                    if let Some(name) = &recipient.name {
                        print!("      - {name}");
                        print!(" ({}%)", recipient.split);
                        println!();
                    }
                }
            }
        }
    }
}

fn display_episodes(feed: &feedparser_rs::ParsedFeed) {
    println!("Episodes ({} total):", feed.entries.len());
    println!("{}", "-".repeat(40));

    for (i, entry) in feed.entries.iter().enumerate() {
        println!("\nEpisode {}:", i + 1);

        if let Some(title) = &entry.title {
            println!("  Title: {title}");
        }

        if let Some(link) = &entry.link {
            println!("  Link: {link}");
        }

        if let Some(published) = &entry.published {
            println!("  Published: {published}");
        }

        // Media enclosure (audio file)
        if !entry.enclosures.is_empty() {
            println!("  Audio:");
            for enc in &entry.enclosures {
                println!("    URL: {}", enc.url);
                if let Some(enclosure_type) = &enc.enclosure_type {
                    println!("    Type: {enclosure_type}");
                }
                if let Some(length) = enc.length {
                    #[allow(clippy::cast_precision_loss)]
                    let mb = length as f64 / 1_048_576.0;
                    println!("    Size: {mb:.2} MB ({length} bytes)");
                }
            }
        }

        // iTunes episode metadata
        if let Some(itunes) = &entry.itunes {
            println!("  iTunes:");

            if let Some(duration) = itunes.duration {
                println!("    Duration: {duration} seconds");

                // Convert to human-readable format
                let hours = duration / 3600;
                let minutes = (duration % 3600) / 60;
                let seconds = duration % 60;
                if hours > 0 {
                    println!("    ({hours:02}:{minutes:02}:{seconds:02})");
                } else {
                    println!("    ({minutes}:{seconds:02})");
                }
            }

            if let Some(episode_num) = itunes.episode {
                println!("    Episode Number: {episode_num}");
            }

            if let Some(season) = itunes.season {
                println!("    Season: {season}");
            }

            if let Some(episode_type) = &itunes.episode_type {
                println!("    Episode Type: {episode_type}");
            }

            if let Some(explicit) = itunes.explicit {
                println!("    Explicit: {}", if explicit { "YES" } else { "NO" });
            }
        }

        // Podcast 2.0 episode features
        if let Some(podcast) = &entry.podcast {
            println!("  Podcast 2.0:");

            // Transcripts
            if !podcast.transcript.is_empty() {
                println!("    Transcripts:");
                for transcript in &podcast.transcript {
                    print!("      - {}", transcript.url);
                    if let Some(transcript_type) = &transcript.transcript_type {
                        print!(" ({transcript_type})");
                    }
                    if let Some(language) = &transcript.language {
                        print!(" [{language}]");
                    }
                    println!();
                }
            }

            // Chapters
            if let Some(chapters) = &podcast.chapters {
                println!("    Chapters: {} ({})", chapters.url, chapters.type_);
            }

            // Soundbites (highlight clips)
            if !podcast.soundbite.is_empty() {
                println!("    Soundbites:");
                for soundbite in &podcast.soundbite {
                    let start_time = soundbite.start_time;
                    let duration = soundbite.duration;
                    print!("      - {:.1}s - {:.1}s", start_time, start_time + duration);
                    if let Some(title) = &soundbite.title {
                        print!(": {title}");
                    }
                    println!();
                }
            }

            // Guest information
            if !podcast.person.is_empty() {
                println!("    People:");
                for person in &podcast.person {
                    print!("      - {}", person.name);
                    if let Some(role) = &person.role {
                        print!(" ({role})");
                    }
                    println!();
                }
            }
        }
    }
}
