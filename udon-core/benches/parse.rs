//! Benchmarks for UDON parsing.
//!
//! Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use udon_core::StreamingParser;

/// Estimate appropriate ring buffer capacity for input size.
/// Uses input_len / 50 (rough events-per-byte estimate), min 16.
/// EventRing will round up to next power of 2.
#[inline]
fn estimate_capacity(input_len: usize) -> usize {
    (input_len / 50).max(16)
}

/// Benchmark streaming parser with comprehensive example.
fn bench_streaming_comprehensive(c: &mut Criterion) {
    let input = include_bytes!("../../examples/comprehensive.udon");
    let capacity = estimate_capacity(input.len());

    let mut group = c.benchmark_group("streaming");
    group.throughput(Throughput::Bytes(input.len() as u64));

    // Reuse parser across iterations (amortizes allocation cost)
    let mut parser = StreamingParser::new(capacity);

    group.bench_function("comprehensive.udon", |b| {
        b.iter(|| {
            parser.reset();
            parser.feed(black_box(input));
            parser.finish();
            // Drain events to simulate real usage
            let mut count = 0;
            while parser.read().is_some() {
                count += 1;
            }
            count
        })
    });

    group.finish();
}

/// Benchmark streaming parser with minimal example.
fn bench_streaming_minimal(c: &mut Criterion) {
    let input = include_bytes!("../../examples/minimal.udon");
    let capacity = estimate_capacity(input.len());

    let mut group = c.benchmark_group("streaming");
    group.throughput(Throughput::Bytes(input.len() as u64));

    // Reuse parser across iterations (amortizes allocation cost)
    let mut parser = StreamingParser::new(capacity);

    group.bench_function("minimal.udon", |b| {
        b.iter(|| {
            parser.reset();
            parser.feed(black_box(input));
            parser.finish();
            let mut count = 0;
            while parser.read().is_some() {
                count += 1;
            }
            count
        })
    });

    group.finish();
}

/// Benchmark simple cases for baseline measurements.
fn bench_streaming_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_simple");

    // Empty input
    group.bench_function("empty", |b| {
        b.iter(|| {
            let mut parser = StreamingParser::new(16);
            parser.feed(black_box(b""));
            parser.finish();
            let mut count = 0;
            while parser.read().is_some() {
                count += 1;
            }
            count
        })
    });

    // Just comments
    let comments = b"; comment 1\n; comment 2\n; comment 3\n";
    group.throughput(Throughput::Bytes(comments.len() as u64));
    group.bench_function("comments_only", |b| {
        b.iter(|| {
            let mut parser = StreamingParser::new(16);
            parser.feed(black_box(comments));
            parser.finish();
            let mut count = 0;
            while parser.read().is_some() {
                count += 1;
            }
            count
        })
    });

    // Just text
    let text = b"Hello world\nThis is prose\nMore text here\n";
    group.throughput(Throughput::Bytes(text.len() as u64));
    group.bench_function("text_only", |b| {
        b.iter(|| {
            let mut parser = StreamingParser::new(16);
            parser.feed(black_box(text));
            parser.finish();
            let mut count = 0;
            while parser.read().is_some() {
                count += 1;
            }
            count
        })
    });

    group.finish();
}

criterion_group!(benches, bench_streaming_comprehensive, bench_streaming_minimal, bench_streaming_simple);
criterion_main!(benches);
