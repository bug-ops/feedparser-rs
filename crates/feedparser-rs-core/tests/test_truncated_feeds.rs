#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use feedparser_rs::parse;

#[test]
fn test_truncated_rss20_sets_bozo() {
    let data = b"<rss version=\"2.0\"><channel><title>Truncated";
    let feed = parse(data).unwrap();
    assert!(feed.bozo, "truncated RSS 2.0 must set bozo=true");
    assert!(
        feed.bozo_exception
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains("truncated"),
        "bozo_exception must mention truncation"
    );
}

#[test]
fn test_truncated_rss20_no_closing_channel_sets_bozo() {
    let data =
        b"<rss version=\"2.0\"><channel><title>No Close</title><item><title>Item</title></item>";
    let feed = parse(data).unwrap();
    assert!(feed.bozo, "RSS 2.0 without </channel> must set bozo=true");
}

#[test]
fn test_truncated_atom_sets_bozo() {
    let data =
        b"<?xml version=\"1.0\"?><feed xmlns=\"http://www.w3.org/2005/Atom\"><title>Truncated";
    let feed = parse(data).unwrap();
    assert!(feed.bozo, "truncated Atom feed must set bozo=true");
    assert!(
        feed.bozo_exception
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains("truncated"),
        "bozo_exception must mention truncation"
    );
}

#[test]
fn test_truncated_atom_no_closing_feed_sets_bozo() {
    let data = b"<?xml version=\"1.0\"?><feed xmlns=\"http://www.w3.org/2005/Atom\"><title>No Close</title><entry><title>Entry</title></entry>";
    let feed = parse(data).unwrap();
    assert!(feed.bozo, "Atom feed without </feed> must set bozo=true");
}

#[test]
fn test_valid_rss20_no_bozo() {
    let data = b"<?xml version=\"1.0\"?><rss version=\"2.0\"><channel><title>Valid</title></channel></rss>";
    let feed = parse(data).unwrap();
    assert!(!feed.bozo, "valid RSS 2.0 must NOT set bozo");
}

#[test]
fn test_valid_atom_no_bozo() {
    let data = b"<?xml version=\"1.0\"?><feed xmlns=\"http://www.w3.org/2005/Atom\"><title>Valid</title></feed>";
    let feed = parse(data).unwrap();
    assert!(!feed.bozo, "valid Atom feed must NOT set bozo");
}

#[test]
fn test_truncated_rss10_sets_bozo() {
    let data = b"<?xml version=\"1.0\"?><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" xmlns=\"http://purl.org/rss/1.0/\"><channel><title>Truncated";
    let feed = parse(data).unwrap();
    assert!(feed.bozo, "truncated RSS 1.0 feed must set bozo=true");
}

#[test]
fn test_valid_rss10_no_bozo() {
    let data = b"<?xml version=\"1.0\"?><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" xmlns=\"http://purl.org/rss/1.0/\"><channel rdf:about=\"http://example.com/\"><title>Valid</title><link>http://example.com</link><description>Test</description></channel></rdf:RDF>";
    let feed = parse(data).unwrap();
    assert!(!feed.bozo, "valid RSS 1.0 feed must NOT set bozo");
}
