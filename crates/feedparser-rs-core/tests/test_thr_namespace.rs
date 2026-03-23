#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Integration tests for RFC 4685 Atom Threading Extensions (thr: namespace)

use feedparser_rs::parse;

#[test]
fn test_thr_count_and_updated_from_fixture() {
    let xml = include_bytes!("../../../tests/fixtures/atom/thr_count_updated.xml");
    let feed = parse(xml).unwrap();

    assert!(!feed.bozo, "valid feed must not set bozo");
    assert_eq!(feed.entries.len(), 5);

    // Entry 1: both thr:count and thr:updated
    let replies1 = feed.entries[0]
        .links
        .iter()
        .find(|l| l.rel.as_deref() == Some("replies"))
        .expect("entry 1 replies link");
    assert_eq!(replies1.thr_count, Some(10));
    assert!(replies1.thr_updated.is_some());

    // Entry 2: only thr:count
    let replies2 = feed.entries[1]
        .links
        .iter()
        .find(|l| l.rel.as_deref() == Some("replies"))
        .expect("entry 2 replies link");
    assert_eq!(replies2.thr_count, Some(5));
    assert!(replies2.thr_updated.is_none());

    // Entry 3: only thr:updated
    let replies3 = feed.entries[2]
        .links
        .iter()
        .find(|l| l.rel.as_deref() == Some("replies"))
        .expect("entry 3 replies link");
    assert_eq!(replies3.thr_count, None);
    assert!(replies3.thr_updated.is_some());

    // Entry 4: thr:count="0" must be Some(0)
    let replies4 = feed.entries[3]
        .links
        .iter()
        .find(|l| l.rel.as_deref() == Some("replies"))
        .expect("entry 4 replies link");
    assert_eq!(replies4.thr_count, Some(0));

    // Entry 5: malformed values — both must be None, no bozo
    let replies5 = feed.entries[4]
        .links
        .iter()
        .find(|l| l.rel.as_deref() == Some("replies"))
        .expect("entry 5 replies link");
    assert_eq!(replies5.thr_count, None);
    assert!(replies5.thr_updated.is_none());
    assert!(!feed.bozo);
}
