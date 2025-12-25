//! Benchmarks for UDON parsing.
//!
//! Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use udon_core::Parser;

/// Benchmark parsing the comprehensive example.
fn bench_comprehensive(c: &mut Criterion) {
    let input = include_bytes!("../../../examples/comprehensive.udon");

    let mut group = c.benchmark_group("parse");
    group.throughput(Throughput::Bytes(input.len() as u64));

    group.bench_function("comprehensive.udon", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(input));
            parser.parse()
        })
    });

    group.finish();
}

/// Benchmark parsing the minimal example.
fn bench_minimal(c: &mut Criterion) {
    let input = include_bytes!("../../../examples/minimal.udon");

    let mut group = c.benchmark_group("parse");
    group.throughput(Throughput::Bytes(input.len() as u64));

    group.bench_function("minimal.udon", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(input));
            parser.parse()
        })
    });

    group.finish();
}

/// Benchmark simple cases for baseline measurements.
fn bench_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple");

    // Empty input
    group.bench_function("empty", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(b""));
            parser.parse()
        })
    });

    // Just comments
    let comments = b"; comment 1\n; comment 2\n; comment 3\n";
    group.throughput(Throughput::Bytes(comments.len() as u64));
    group.bench_function("comments_only", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(comments));
            parser.parse()
        })
    });

    // Just text
    let text = b"Hello world\nThis is prose\nMore text here\n";
    group.throughput(Throughput::Bytes(text.len() as u64));
    group.bench_function("text_only", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(text));
            parser.parse()
        })
    });

    group.finish();
}

criterion_group!(benches, bench_comprehensive, bench_minimal, bench_simple);
criterion_main!(benches);
