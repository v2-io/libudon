//! Benchmark comparison of parser approaches
//!
//! Compares:
//! 1. Callback-based (true recursion)
//! 2. Generator-based (genawaiter, explicit stack)
//! 3. Ring-buffer based (explicit stack, mimics current libudon)

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use gen_parser_poc::callback::SimpleCallbackParser;
use gen_parser_poc::genawaiter_attempt::Parser as GenawaiterParser;
use gen_parser_poc::ringbuffer::RingBufferParser;

/// Small input - typical single element
const SMALL_INPUT: &[u8] = b"|div[id] Hello World\n";

/// Medium input - nested elements
const MEDIUM_INPUT: &[u8] = b"\
|html
  |head
    |title Page Title
  |body
    |div[container]
      |h1 Welcome
      |p This is a paragraph with some text content.
      |ul
        |li Item one
        |li Item two
        |li Item three
      |p Another paragraph here.
    |footer
      |p Copyright 2025
";

/// Generate a large input with many nested elements
fn generate_large_input() -> Vec<u8> {
    let mut input = Vec::new();
    for i in 0..100 {
        input.extend_from_slice(format!("|section[sec{}]\n", i).as_bytes());
        for j in 0..10 {
            input.extend_from_slice(format!("  |div[div{}_{}]\n", i, j).as_bytes());
            input.extend_from_slice(b"    |p Some text content here for benchmarking.\n");
            input.extend_from_slice(b"    |span More inline content.\n");
        }
    }
    input
}

/// Generate deeply nested input (stress test for recursion)
fn generate_deep_input() -> Vec<u8> {
    let mut input = Vec::new();
    let depth = 50;
    for i in 0..depth {
        let indent = "  ".repeat(i);
        input.extend_from_slice(format!("{}|level{}\n", indent, i).as_bytes());
    }
    // Add content at the deepest level
    let indent = "  ".repeat(depth);
    input.extend_from_slice(format!("{}Deep content here\n", indent).as_bytes());
    input
}

fn bench_callback(c: &mut Criterion) {
    let mut group = c.benchmark_group("callback");

    // Small input
    group.throughput(Throughput::Bytes(SMALL_INPUT.len() as u64));
    group.bench_function("small", |b| {
        b.iter(|| {
            let mut count = 0;
            SimpleCallbackParser::new(black_box(SMALL_INPUT)).parse(|_e| {
                count += 1;
            });
            count
        });
    });

    // Medium input
    group.throughput(Throughput::Bytes(MEDIUM_INPUT.len() as u64));
    group.bench_function("medium", |b| {
        b.iter(|| {
            let mut count = 0;
            SimpleCallbackParser::new(black_box(MEDIUM_INPUT)).parse(|_e| {
                count += 1;
            });
            count
        });
    });

    // Large input
    let large_input = generate_large_input();
    group.throughput(Throughput::Bytes(large_input.len() as u64));
    group.bench_function("large", |b| {
        b.iter(|| {
            let mut count = 0;
            SimpleCallbackParser::new(black_box(&large_input)).parse(|_e| {
                count += 1;
            });
            count
        });
    });

    // Deep nesting
    let deep_input = generate_deep_input();
    group.throughput(Throughput::Bytes(deep_input.len() as u64));
    group.bench_function("deep", |b| {
        b.iter(|| {
            let mut count = 0;
            SimpleCallbackParser::new(black_box(&deep_input)).parse(|_e| {
                count += 1;
            });
            count
        });
    });

    group.finish();
}

fn bench_genawaiter(c: &mut Criterion) {
    let mut group = c.benchmark_group("genawaiter");

    // Small input
    group.throughput(Throughput::Bytes(SMALL_INPUT.len() as u64));
    group.bench_function("small", |b| {
        b.iter(|| {
            GenawaiterParser::new(black_box(SMALL_INPUT))
                .parse_flat()
                .count()
        });
    });

    // Medium input
    group.throughput(Throughput::Bytes(MEDIUM_INPUT.len() as u64));
    group.bench_function("medium", |b| {
        b.iter(|| {
            GenawaiterParser::new(black_box(MEDIUM_INPUT))
                .parse_flat()
                .count()
        });
    });

    // Large input
    let large_input = generate_large_input();
    group.throughput(Throughput::Bytes(large_input.len() as u64));
    group.bench_function("large", |b| {
        b.iter(|| {
            GenawaiterParser::new(black_box(&large_input))
                .parse_flat()
                .count()
        });
    });

    // Deep nesting
    let deep_input = generate_deep_input();
    group.throughput(Throughput::Bytes(deep_input.len() as u64));
    group.bench_function("deep", |b| {
        b.iter(|| {
            GenawaiterParser::new(black_box(&deep_input))
                .parse_flat()
                .count()
        });
    });

    group.finish();
}

fn bench_ringbuffer(c: &mut Criterion) {
    let mut group = c.benchmark_group("ringbuffer");

    // Small input
    group.throughput(Throughput::Bytes(SMALL_INPUT.len() as u64));
    group.bench_function("small", |b| {
        b.iter(|| {
            let mut parser = RingBufferParser::new(black_box(SMALL_INPUT), 64);
            parser.parse();
            parser.drain().len()
        });
    });

    // Medium input
    group.throughput(Throughput::Bytes(MEDIUM_INPUT.len() as u64));
    group.bench_function("medium", |b| {
        b.iter(|| {
            let mut parser = RingBufferParser::new(black_box(MEDIUM_INPUT), 256);
            parser.parse();
            parser.drain().len()
        });
    });

    // Large input
    let large_input = generate_large_input();
    group.throughput(Throughput::Bytes(large_input.len() as u64));
    group.bench_function("large", |b| {
        b.iter(|| {
            let mut parser = RingBufferParser::new(black_box(&large_input), 8192);
            parser.parse();
            parser.drain().len()
        });
    });

    // Deep nesting
    let deep_input = generate_deep_input();
    group.throughput(Throughput::Bytes(deep_input.len() as u64));
    group.bench_function("deep", |b| {
        b.iter(|| {
            let mut parser = RingBufferParser::new(black_box(&deep_input), 256);
            parser.parse();
            parser.drain().len()
        });
    });

    group.finish();
}

/// Direct comparison - all three on same input
fn bench_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("comparison");
    group.throughput(Throughput::Bytes(MEDIUM_INPUT.len() as u64));

    group.bench_function("callback", |b| {
        b.iter(|| {
            let mut count = 0;
            SimpleCallbackParser::new(black_box(MEDIUM_INPUT)).parse(|_e| {
                count += 1;
            });
            count
        });
    });

    group.bench_function("genawaiter", |b| {
        b.iter(|| {
            GenawaiterParser::new(black_box(MEDIUM_INPUT))
                .parse_flat()
                .count()
        });
    });

    group.bench_function("ringbuffer", |b| {
        b.iter(|| {
            let mut parser = RingBufferParser::new(black_box(MEDIUM_INPUT), 256);
            parser.parse();
            parser.drain().len()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_callback,
    bench_genawaiter,
    bench_ringbuffer,
    bench_comparison,
);
criterion_main!(benches);
