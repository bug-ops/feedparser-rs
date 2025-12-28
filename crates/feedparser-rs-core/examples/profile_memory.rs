//! DHAT memory profiler for feedparser-rs
//!
//! Usage:
//!   `cargo run --release --example profile_memory`
//!
//! View results:
//!   Open dhat-heap.json at <https://nnethercote.github.io/dh_view/dh_view.html>
//!
//! Metrics tracked:
//! - Total allocations per parse
//! - Total bytes allocated
//! - Peak memory usage
//! - Allocation hot spots (functions causing most allocations)

use feedparser_rs::parse;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    let _profiler = dhat::Profiler::new_heap();

    println!("=== feedparser-rs Memory Profiling ===\n");

    // Profile small feed (2.7 KB)
    println!("Profiling SMALL feed (2.7 KB) - 1000 iterations...");
    let small = include_bytes!("../../../benchmarks/fixtures/small.xml");
    for _ in 0..1000 {
        let _ = parse(small);
    }
    println!("  Completed: 1000 parses\n");

    // Profile medium feed (24 KB)
    println!("Profiling MEDIUM feed (24 KB) - 100 iterations...");
    let medium = include_bytes!("../../../benchmarks/fixtures/medium.xml");
    for _ in 0..100 {
        let _ = parse(medium);
    }
    println!("  Completed: 100 parses\n");

    // Profile large feed (237 KB)
    println!("Profiling LARGE feed (237 KB) - 10 iterations...");
    let large = include_bytes!("../../../benchmarks/fixtures/large.xml");
    for _ in 0..10 {
        let _ = parse(large);
    }
    println!("  Completed: 10 parses\n");

    println!("=== Profiling Complete ===");
    println!("\nResults saved to: dhat-heap.json");
    println!("View at: https://nnethercote.github.io/dh_view/dh_view.html");
    println!("\nKey metrics to analyze:");
    println!("  - Total allocations per parse (target: <200 for small)");
    println!("  - Total bytes allocated");
    println!("  - Peak memory usage");
    println!("  - Short-lived allocations (optimization candidates)");
    println!("  - Top allocation hot spots");
}
