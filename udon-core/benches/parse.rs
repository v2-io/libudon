//! Benchmarks for UDON parsing.
//!
//! Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use udon_core::Parser;

/// Benchmark parser with comprehensive example.
fn bench_parse_comprehensive(c: &mut Criterion) {
    let input = include_bytes!("../../examples/comprehensive.udon");

    let mut group = c.benchmark_group("parse");
    group.throughput(Throughput::Bytes(input.len() as u64));

    group.bench_function("comprehensive.udon", |b| {
        b.iter(|| {
            let mut count = 0;
            Parser::new(black_box(input)).parse(|_| count += 1);
            count
        })
    });

    group.finish();
}

/// Benchmark parser with minimal example.
fn bench_parse_minimal(c: &mut Criterion) {
    let input = include_bytes!("../../examples/minimal.udon");

    let mut group = c.benchmark_group("parse");
    group.throughput(Throughput::Bytes(input.len() as u64));

    group.bench_function("minimal.udon", |b| {
        b.iter(|| {
            let mut count = 0;
            Parser::new(black_box(input)).parse(|_| count += 1);
            count
        })
    });

    group.finish();
}

/// Benchmark simple cases for baseline measurements.
fn bench_parse_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_simple");

    // Empty input
    group.bench_function("empty", |b| {
        b.iter(|| {
            let mut count = 0;
            Parser::new(black_box(b"")).parse(|_| count += 1);
            count
        })
    });

    // Just comments
    let comments = b"; comment 1\n; comment 2\n; comment 3\n";
    group.throughput(Throughput::Bytes(comments.len() as u64));
    group.bench_function("comments_only", |b| {
        b.iter(|| {
            let mut count = 0;
            Parser::new(black_box(comments)).parse(|_| count += 1);
            count
        })
    });

    // Just text
    let text = b"Hello world\nThis is prose\nMore text here\n";
    group.throughput(Throughput::Bytes(text.len() as u64));
    group.bench_function("text_only", |b| {
        b.iter(|| {
            let mut count = 0;
            Parser::new(black_box(text)).parse(|_| count += 1);
            count
        })
    });

    // Nested elements
    let nested = b"|html\n  |head\n    |title Page\n  |body\n    |h1 Hello\n    |p World\n";
    group.throughput(Throughput::Bytes(nested.len() as u64));
    group.bench_function("nested_elements", |b| {
        b.iter(|| {
            let mut count = 0;
            Parser::new(black_box(nested)).parse(|_| count += 1);
            count
        })
    });

    // Embedded elements and interpolation
    let dynamic = b"|p Hello |{em world} and !{{name}} works\n";
    group.throughput(Throughput::Bytes(dynamic.len() as u64));
    group.bench_function("dynamic_content", |b| {
        b.iter(|| {
            let mut count = 0;
            Parser::new(black_box(dynamic)).parse(|_| count += 1);
            count
        })
    });

    group.finish();
}

/// Benchmark scaling with input size.
fn bench_parse_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_scaling");

    // Generate inputs of different sizes
    for size in [100, 1000, 10000] {
        let input = generate_test_input(size);
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_function(format!("{}_lines", size), |b| {
            b.iter(|| {
                let mut count = 0;
                Parser::new(black_box(&input)).parse(|_| count += 1);
                count
            })
        });
    }

    group.finish();
}

/// Generate test input of approximately n lines.
fn generate_test_input(lines: usize) -> Vec<u8> {
    let mut input = Vec::with_capacity(lines * 30);
    for i in 0..lines {
        match i % 4 {
            0 => input.extend_from_slice(format!("|div.item-{}\n", i).as_bytes()),
            1 => input.extend_from_slice(b"  :key value\n"),
            2 => input.extend_from_slice(b"  Some text content\n"),
            3 => input.extend_from_slice(b"; A comment line\n"),
            _ => unreachable!(),
        }
    }
    input
}

criterion_group!(
    benches,
    bench_parse_comprehensive,
    bench_parse_minimal,
    bench_parse_simple,
    bench_parse_scaling
);
criterion_main!(benches);
