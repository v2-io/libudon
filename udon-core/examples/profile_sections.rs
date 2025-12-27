//! Profile different sections of parsing.
//! Measures time spent in feed, finish, and event reading.

use std::time::Instant;
use udon_core::StreamingParser;

fn main() {
    let input = include_bytes!("../../examples/comprehensive.udon");
    let capacity = (input.len() / 50).max(16);
    let iterations = 100_000;

    let mut parser = StreamingParser::new(capacity);

    // Warm up
    for _ in 0..1000 {
        parser.reset();
        parser.feed(input);
        parser.finish();
        while parser.read().is_some() {}
    }

    // Measure total time
    let start_total = Instant::now();

    let mut feed_time = std::time::Duration::ZERO;
    let mut finish_time = std::time::Duration::ZERO;
    let mut read_time = std::time::Duration::ZERO;
    let mut reset_time = std::time::Duration::ZERO;
    let mut event_count = 0u64;

    for _ in 0..iterations {
        let t0 = Instant::now();
        parser.reset();
        reset_time += t0.elapsed();

        let t1 = Instant::now();
        parser.feed(input);
        feed_time += t1.elapsed();

        let t2 = Instant::now();
        parser.finish();
        finish_time += t2.elapsed();

        let t3 = Instant::now();
        while parser.read().is_some() {
            event_count += 1;
        }
        read_time += t3.elapsed();
    }

    let total_time = start_total.elapsed();

    println!("Iterations: {}", iterations);
    println!("Input size: {} bytes", input.len());
    println!("Events per parse: {}", event_count / iterations as u64);
    println!();
    println!("Total time:  {:>10.2} ms", total_time.as_secs_f64() * 1000.0);
    println!("  reset():   {:>10.2} ms ({:.1}%)",
             reset_time.as_secs_f64() * 1000.0,
             reset_time.as_secs_f64() / total_time.as_secs_f64() * 100.0);
    println!("  feed():    {:>10.2} ms ({:.1}%)",
             feed_time.as_secs_f64() * 1000.0,
             feed_time.as_secs_f64() / total_time.as_secs_f64() * 100.0);
    println!("  finish():  {:>10.2} ms ({:.1}%)",
             finish_time.as_secs_f64() * 1000.0,
             finish_time.as_secs_f64() / total_time.as_secs_f64() * 100.0);
    println!("  read():    {:>10.2} ms ({:.1}%)",
             read_time.as_secs_f64() * 1000.0,
             read_time.as_secs_f64() / total_time.as_secs_f64() * 100.0);
    println!();
    println!("Per iteration:");
    println!("  Total:     {:>10.2} µs", total_time.as_secs_f64() * 1_000_000.0 / iterations as f64);
    println!("  feed():    {:>10.2} µs", feed_time.as_secs_f64() * 1_000_000.0 / iterations as f64);
    println!("Throughput:  {:>10.2} MiB/s",
             (input.len() as f64 * iterations as f64) / (total_time.as_secs_f64() * 1024.0 * 1024.0));
}
