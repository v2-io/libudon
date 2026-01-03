use std::time::Instant;
use udon_core::{Parser, tree::Document};

fn main() {
    // Generate a decent-sized document
    let mut input = String::new();
    for i in 0..1000 {
        input.push_str(&format!("|item[id-{}].class{} :count {} :name \"Item {}\"\n", i, i % 5, i, i));
        input.push_str(&format!("  Some text content for item {}.\n", i));
        if i % 3 == 0 {
            input.push_str("  |nested :foo bar\n");
        }
    }
    let bytes = input.as_bytes();
    println!("Input size: {} bytes ({:.1} KB)", bytes.len(), bytes.len() as f64 / 1024.0);

    // Warm up
    for _ in 0..10 {
        let _ = Document::parse(bytes);
    }

    // Benchmark tree parsing
    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let doc = Document::parse(bytes).unwrap();
        std::hint::black_box(&doc);
    }
    let tree_time = start.elapsed();
    let tree_per_iter = tree_time / iterations;
    let tree_throughput = (bytes.len() as f64 * iterations as f64) / tree_time.as_secs_f64() / 1_000_000.0;

    // Benchmark streaming (SAX-like)
    let start = Instant::now();
    for _ in 0..iterations {
        let mut count = 0usize;
        Parser::new(bytes).parse(|_event| count += 1);
        std::hint::black_box(count);
    }
    let sax_time = start.elapsed();
    let sax_per_iter = sax_time / iterations;
    let sax_throughput = (bytes.len() as f64 * iterations as f64) / sax_time.as_secs_f64() / 1_000_000.0;

    println!("\nStreaming (SAX-like):");
    println!("  {:?} per parse", sax_per_iter);
    println!("  {:.1} MB/s", sax_throughput);

    println!("\nTree (DOM-like):");
    println!("  {:?} per parse", tree_per_iter);
    println!("  {:.1} MB/s", tree_throughput);

    println!("\nOverhead: {:.1}x slower", tree_per_iter.as_nanos() as f64 / sax_per_iter.as_nanos() as f64);
}
