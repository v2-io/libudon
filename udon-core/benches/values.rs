//! Benchmarks comparing value parsing approaches.
//!
//! Compares:
//! - value.rs: Post-hoc type classifier (current implementation) - classifies AND converts
//! - values_parser.rs: descent-generated streaming parser - type classification only
//! - lexical-core: Optimized numeric parsing - conversion only (you provide the type)
//!
//! Run with: cargo bench --bench values

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use udon_core::value::Value;
use udon_core::values_parser::Parser as ValuesParser;

/// Test values covering all types
static TEST_VALUES: &[&[u8]] = &[
    // Nil
    b"null",
    b"nil",
    b"~",
    // Bool
    b"true",
    b"false",
    // Integers - decimal
    b"0",
    b"42",
    b"1000000",
    b"1_000_000",
    b"-42",
    // Integers - hex
    b"0xFF",
    b"0xDEADBEEF",
    b"0xdead_beef",
    // Integers - octal
    b"0o755",
    b"0o777",
    // Integers - binary
    b"0b1010",
    b"0b1111_0000",
    // Floats
    b"3.14",
    b"0.001",
    b"1.5e-3",
    b"1.5E+10",
    b"-2.5",
    b"1_000.5",
    // Rationals
    b"1/3r",
    b"22/7r",
    b"-1/2r",
    // Complex
    b"5i",
    b"3+4i",
    b"3-4i",
    b"1.5+2.5i",
    // Strings (bare)
    b"hello",
    b"hello-world",
    b"not-a-number",
    // Strings (quoted)
    b"\"hello world\"",
    b"'single quotes'",
];

/// Benchmark value.rs (post-hoc classifier)
fn bench_value_rs(c: &mut Criterion) {
    let mut group = c.benchmark_group("value_rs");

    // Individual value types
    for &input in TEST_VALUES {
        let name = std::str::from_utf8(input).unwrap_or("<binary>");
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("parse", name), &input, |b, &input| {
            b.iter(|| Value::parse(black_box(input)))
        });
    }

    // Batch: all values
    let total_bytes: usize = TEST_VALUES.iter().map(|v| v.len()).sum();
    group.throughput(Throughput::Bytes(total_bytes as u64));
    group.bench_function("all_values", |b| {
        b.iter(|| {
            for &input in TEST_VALUES {
                black_box(Value::parse(black_box(input)));
            }
        })
    });

    group.finish();
}

/// Benchmark values_parser.rs (descent-generated)
fn bench_values_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("values_parser");

    // Individual value types
    for &input in TEST_VALUES {
        let name = std::str::from_utf8(input).unwrap_or("<binary>");
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("parse", name), &input, |b, &input| {
            b.iter(|| {
                let mut event_count = 0;
                ValuesParser::new(black_box(input)).parse(|_event| {
                    event_count += 1;
                });
                event_count
            })
        });
    }

    // Batch: all values
    let total_bytes: usize = TEST_VALUES.iter().map(|v| v.len()).sum();
    group.throughput(Throughput::Bytes(total_bytes as u64));
    group.bench_function("all_values", |b| {
        b.iter(|| {
            let mut total_events = 0;
            for &input in TEST_VALUES {
                ValuesParser::new(black_box(input)).parse(|_event| {
                    total_events += 1;
                });
            }
            total_events
        })
    });

    group.finish();
}

/// Focused benchmark on numeric parsing (where descent should excel)
///
/// This compares type detection speed. Note the different responsibilities:
/// - value.rs: detects type AND converts to native value
/// - values_parser: detects type only (emits typed events with byte slices)
/// - lexical-core: converts only (you must already know the type)
fn bench_numeric_parsing(c: &mut Criterion) {
    let numeric_values: &[&[u8]] = &[
        b"42",
        b"1_000_000",
        b"0xFF",
        b"0xDEADBEEF",
        b"0o755",
        b"0b1010",
        b"3.14159265358979",
        b"1.5e-10",
        b"22/7r",
        b"3+4i",
    ];

    let total_bytes: usize = numeric_values.iter().map(|v| v.len()).sum();

    let mut group = c.benchmark_group("numeric_parsing");
    group.throughput(Throughput::Bytes(total_bytes as u64));

    group.bench_function("value_rs", |b| {
        b.iter(|| {
            for &input in numeric_values {
                black_box(Value::parse(black_box(input)));
            }
        })
    });

    group.bench_function("values_parser", |b| {
        b.iter(|| {
            let mut total_events = 0;
            for &input in numeric_values {
                ValuesParser::new(black_box(input)).parse(|_event| {
                    total_events += 1;
                });
            }
            total_events
        })
    });

    group.finish();
}

/// Benchmark lexical-core with realistic prefix detection.
///
/// lexical-core doesn't auto-detect 0x/0o/0b prefixes, so we add manual
/// prefix detection to make a fair comparison.
fn bench_lexical_core(c: &mut Criterion) {
    // Mixed numeric values including prefixed formats
    let mixed_numerics: &[&[u8]] = &[
        b"0",
        b"42",
        b"3.14",
        b"1000000",
        b"0xFF",
        b"0o755",
        b"0b1010",
        b"0.001",
        b"-42",
        b"1.5e-3",
        b"0xDEADBEEF",
        b"1.5E+10",
        b"-2.5",
        b"3.14159265358979",
    ];

    let total_bytes: usize = mixed_numerics.iter().map(|v| v.len()).sum();

    let mut group = c.benchmark_group("lexical_core");
    group.throughput(Throughput::Bytes(total_bytes as u64));

    // lexical-core with prefix detection + type detection
    group.bench_function("with_prefix_detection", |b| {
        b.iter(|| {
            for &input in mixed_numerics {
                let input = black_box(input);
                // Check for prefixes
                if input.len() >= 2 && input[0] == b'0' {
                    match input[1] {
                        b'x' | b'X' => {
                            // Hex - parse without prefix
                            if let Ok(v) = i64::from_str_radix(
                                std::str::from_utf8(&input[2..]).unwrap_or(""),
                                16
                            ) {
                                black_box(v);
                                continue;
                            }
                        }
                        b'o' | b'O' => {
                            // Octal
                            if let Ok(v) = i64::from_str_radix(
                                std::str::from_utf8(&input[2..]).unwrap_or(""),
                                8
                            ) {
                                black_box(v);
                                continue;
                            }
                        }
                        b'b' | b'B' => {
                            // Binary
                            if let Ok(v) = i64::from_str_radix(
                                std::str::from_utf8(&input[2..]).unwrap_or(""),
                                2
                            ) {
                                black_box(v);
                                continue;
                            }
                        }
                        _ => {}
                    }
                }
                // Standard decimal: try int then float
                if let Ok(v) = lexical_core::parse::<i64>(input) {
                    black_box(v);
                } else if let Ok(v) = lexical_core::parse::<f64>(input) {
                    black_box(v);
                }
            }
        })
    });

    // value.rs on same mixed values
    group.bench_function("value_rs", |b| {
        b.iter(|| {
            for &input in mixed_numerics {
                black_box(Value::parse(black_box(input)));
            }
        })
    });

    // values_parser on same mixed values
    group.bench_function("values_parser", |b| {
        b.iter(|| {
            let mut total_events = 0;
            for &input in mixed_numerics {
                ValuesParser::new(black_box(input)).parse(|_event| {
                    total_events += 1;
                });
            }
            total_events
        })
    });

    group.finish();
}

criterion_group!(benches, bench_value_rs, bench_values_parser, bench_numeric_parsing, bench_lexical_core);
criterion_main!(benches);
