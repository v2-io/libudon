//! Simple binary for profiling the parser.
//! Run with: samply record cargo run --release --example profile_parse

use udon_core::StreamingParser;

fn main() {
    let input = include_bytes!("../../examples/comprehensive.udon");
    let capacity = (input.len() / 50).max(16);

    let mut parser = StreamingParser::new(capacity);

    // Run many iterations to get good sampling
    for _ in 0..100_000 {
        parser.reset();
        parser.feed(input);
        parser.finish();

        // Drain events
        while parser.read().is_some() {}
    }

    println!("Done - parsed {} bytes x 100,000 iterations", input.len());
}
