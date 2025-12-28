//! Benchmarks for newtype wrappers: Arc<str> vs String performance
//!
//! Tests the claim that Arc<str> cloning is faster than String cloning
//! for `MimeType`, `Url`, and `Email` types.
//!
//! Key questions:
//! 1. Is `Arc::clone()` actually faster for typical MIME types?
//! 2. What's the creation overhead of Arc vs String?
//! 3. What's the break-even point (clones needed to justify Arc)?

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use feedparser_rs::{Email, MimeType, Url};
use std::hint::black_box;
use std::sync::Arc;

/// Benchmark `MimeType` (`Arc<str>`) cloning vs String cloning
fn bench_mimetype_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("MimeType_clone");

    // Test common MIME types found in RSS/Atom feeds
    let mime_types = [
        "text/html",            // Short (9 bytes)
        "text/plain",           // Short (10 bytes)
        "application/xml",      // Medium (15 bytes)
        "application/rss+xml",  // Medium (20 bytes)
        "application/atom+xml", // Medium (21 bytes)
        "application/json",     // Medium (16 bytes)
        "audio/mpeg",           // Short (10 bytes)
        "video/mp4",            // Short (9 bytes)
        "image/jpeg",           // Short (10 bytes)
        "application/pdf",      // Medium (15 bytes)
    ];

    for mime_str in &mime_types {
        let mime = MimeType::new(*mime_str);

        group.bench_with_input(BenchmarkId::new("Arc_str", mime_str), &mime, |b, m| {
            b.iter(|| black_box(m.clone()));
        });

        // Compare to direct String clone
        let string = mime_str.to_string();
        group.bench_with_input(BenchmarkId::new("String", mime_str), &string, |b, s| {
            b.iter(|| black_box(s.clone()));
        });

        // Compare to Arc<String> (alternative design)
        let arc_string = Arc::new(mime_str.to_string());
        group.bench_with_input(
            BenchmarkId::new("Arc_String", mime_str),
            &arc_string,
            |b, a| b.iter(|| black_box(Arc::clone(a))),
        );
    }

    group.finish();
}

/// Benchmark `MimeType` creation overhead: `Arc::from(str)` vs `String::from(str)`
fn bench_mimetype_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("MimeType_creation");

    let mime_types = ["text/html", "application/rss+xml", "application/atom+xml"];

    for mime_str in &mime_types {
        group.bench_with_input(
            BenchmarkId::new("Arc_from_str", mime_str),
            mime_str,
            |b, s| b.iter(|| MimeType::new(black_box(*s))),
        );

        group.bench_with_input(
            BenchmarkId::new("String_from_str", mime_str),
            mime_str,
            |b, s| b.iter(|| black_box(*s).to_string()),
        );

        group.bench_with_input(
            BenchmarkId::new("Arc_str_from_str", mime_str),
            mime_str,
            |b, s| b.iter(|| Arc::<str>::from(black_box(*s))),
        );
    }

    group.finish();
}

/// Benchmark `Url` (String) cloning
fn bench_url_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("Url_clone");

    let urls = [
        "https://example.com",                            // Short URL (19 bytes)
        "https://example.com/feed.xml",                   // Medium URL (27 bytes)
        "https://example.com/feed.xml?param=value",       // Long URL (43 bytes)
        "https://example.com/blog/2024/12/article-title", // Long URL (47 bytes)
    ];

    for url_str in &urls {
        let url = Url::new(*url_str);

        group.bench_with_input(BenchmarkId::new("String", url_str), &url, |b, u| {
            b.iter(|| black_box(u.clone()));
        });

        // Compare to Arc<str> (if we switched)
        let arc_url = Arc::<str>::from(*url_str);
        group.bench_with_input(BenchmarkId::new("Arc_str", url_str), &arc_url, |b, a| {
            b.iter(|| black_box(Arc::clone(a)));
        });
    }

    group.finish();
}

/// Benchmark `Email` (String) cloning
fn bench_email_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("Email_clone");

    let emails = [
        "user@example.com",                      // Short email (17 bytes)
        "john.doe@example.com",                  // Medium email (21 bytes)
        "very.long.email@subdomain.example.com", // Long email (38 bytes)
    ];

    for email_str in &emails {
        let email = Email::new(*email_str);

        group.bench_with_input(BenchmarkId::new("String", email_str), &email, |b, e| {
            b.iter(|| black_box(e.clone()));
        });

        // Compare to Arc<str> (if we switched)
        let arc_email = Arc::<str>::from(*email_str);
        group.bench_with_input(
            BenchmarkId::new("Arc_str", email_str),
            &arc_email,
            |b, a| b.iter(|| black_box(Arc::clone(a))),
        );
    }

    group.finish();
}

/// Benchmark break-even analysis: creation + N clones
///
/// Tests: At how many clones does Arc become faster than String?
fn bench_breakeven_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("breakeven_analysis");

    let mime_str = "application/rss+xml";
    let clone_counts = [1, 2, 5, 10, 20, 50, 100];

    for &n_clones in &clone_counts {
        // Arc<str> approach: 1 creation + N clones
        group.bench_with_input(BenchmarkId::new("Arc_str", n_clones), &n_clones, |b, &n| {
            b.iter(|| {
                let mime = MimeType::new(black_box(mime_str));
                for _ in 0..n {
                    let _ = black_box(mime.clone());
                }
            });
        });

        // String approach: 1 creation + N clones
        group.bench_with_input(BenchmarkId::new("String", n_clones), &n_clones, |b, &n| {
            b.iter(|| {
                let s = black_box(mime_str).to_string();
                for _ in 0..n {
                    let _ = black_box(s.clone());
                }
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_mimetype_clone,
    bench_mimetype_creation,
    bench_url_clone,
    bench_email_clone,
    bench_breakeven_analysis,
);
criterion_main!(benches);
