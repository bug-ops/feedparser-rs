#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use feedparser_rs::parse;

fn load_fixture(path: &str) -> Vec<u8> {
    let fixture_path = format!("../../tests/fixtures/{path}");
    std::fs::read(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to load fixture '{fixture_path}': {e}"))
}

#[test]
fn test_doctype_inline_entity_not_expanded() {
    let xml = load_fixture("malformed/doctype-entity-declaration.xml");
    let result = parse(&xml);

    assert!(
        result.is_ok(),
        "Parser must not return Err on DOCTYPE with inline entities"
    );
    let feed = result.unwrap();

    assert!(feed.bozo, "Unknown entity &xxe; must set bozo=true");

    let title = feed.feed.title.unwrap_or_default();
    assert!(
        !title.contains("INJECTED"),
        "Entity must NOT be expanded: title was '{title}'"
    );
}

#[test]
fn test_xml_bomb_billion_laughs_no_explosion() {
    let xml = load_fixture("malformed/xml-bomb-billion-laughs.xml");

    // Must not panic or OOM — quick-xml does not expand DTD entities
    let result = parse(&xml);

    assert!(
        result.is_ok(),
        "Parser must not return Err on billion-laughs XML bomb"
    );
    let feed = result.unwrap();

    // Unknown entity refs (lol4 etc.) trigger bozo
    assert!(feed.bozo, "Unexpanded entity references must set bozo=true");
}

#[test]
fn test_nul_byte_no_panic() {
    let data =
        b"<?xml version=\"1.0\"?>\n<rss version=\"2.0\">\n<channel>\n<title>Before\x00After</title>\n</channel>\n</rss>";

    // Must not panic — either Ok or Err is acceptable
    let result = std::panic::catch_unwind(|| parse(data));
    assert!(
        result.is_ok(),
        "Parser must not panic on NUL byte in feed body"
    );

    // If parsing succeeded, title should not cause memory issues
    if let Ok(Ok(feed)) = result {
        // bozo may or may not be set — both acceptable
        let _ = feed.feed.title;
    }
}

#[test]
fn test_extremely_long_attribute_no_panic() {
    let long_value = "A".repeat(1_000_000);
    let data = format!(
        r#"<?xml version="1.0"?><rss version="2.0"><channel><title>Test</title><item><title>Entry</title><link href="{long_value}" /></item></channel></rss>"#
    );

    // Must not panic or OOM
    let result = std::panic::catch_unwind(|| parse(data.as_bytes()));
    assert!(
        result.is_ok(),
        "Parser must not panic on 1MB attribute value"
    );

    if let Ok(Ok(feed)) = result {
        assert_eq!(feed.feed.title.as_deref(), Some("Test"));
    }
}

#[test]
fn test_recursive_namespace_prefixes_no_panic() {
    let xml = load_fixture("malformed/recursive-namespace-prefixes.xml");

    let result = std::panic::catch_unwind(|| parse(&xml));
    assert!(
        result.is_ok(),
        "Parser must not panic on recursive namespace prefixes"
    );

    if let Ok(Ok(feed)) = result {
        assert_eq!(
            feed.feed.title.as_deref(),
            Some("Test"),
            "Feed title should be parsed correctly"
        );
    }
}

#[test]
fn test_malformed_namespace_uri_no_panic() {
    let xml = load_fixture("malformed/malformed-namespace-uri.xml");

    let result = std::panic::catch_unwind(|| parse(&xml));
    assert!(
        result.is_ok(),
        "Parser must not panic on malformed namespace URIs"
    );

    if let Ok(Ok(feed)) = result {
        assert_eq!(feed.feed.title.as_deref(), Some("Test"));
        // dc:creator is matched by prefix — should still parse
        assert_eq!(feed.feed.author.as_deref(), Some("Author"));
    }
}
