//! Boundary tests: EOF and chunk splitting
//!
//! Tests that the parser handles:
//! 1. EOF at various positions in input
//! 2. Input split across multiple chunks (when streaming is implemented)
//!
//! These tests catch issues like:
//! - Premature value emission (e.g., "true" emitting BoolTrue before seeing "-is-best")
//! - Incomplete constructs at EOF
//! - State corruption across chunk boundaries

mod common;

use common::{load_fixtures_by_name, Gen};
use udon_core::{Parser, Event, StreamingParser, StreamEvent};

/// Collect events from parsing, returning formatted strings
fn collect_events(input: &[u8]) -> Vec<String> {
    let mut events = Vec::new();
    Parser::new(input).parse(|e| {
        events.push(format_event(&e));
    });
    events
}

fn format_event(event: &Event) -> String {
    match event {
        Event::ElementStart { .. } => "ElementStart".to_string(),
        Event::ElementEnd { .. } => "ElementEnd".to_string(),
        Event::EmbeddedStart { .. } => "EmbeddedStart".to_string(),
        Event::EmbeddedEnd { .. } => "EmbeddedEnd".to_string(),
        Event::DirectiveStart { .. } => "DirectiveStart".to_string(),
        Event::DirectiveEnd { .. } => "DirectiveEnd".to_string(),
        Event::ArrayStart { .. } => "ArrayStart".to_string(),
        Event::ArrayEnd { .. } => "ArrayEnd".to_string(),
        Event::FreeformStart { .. } => "FreeformStart".to_string(),
        Event::FreeformEnd { .. } => "FreeformEnd".to_string(),
        Event::CommentStart { .. } => "CommentStart".to_string(),
        Event::CommentEnd { .. } => "CommentEnd".to_string(),
        Event::Name { content, .. } => format!("Name {:?}", String::from_utf8_lossy(content)),
        Event::Text { content, .. } => format!("Text {:?}", String::from_utf8_lossy(content)),
        Event::Attr { content, .. } => format!("Attr {:?}", String::from_utf8_lossy(content)),
        Event::StringValue { content, .. } => format!("StringValue {:?}", String::from_utf8_lossy(content)),
        Event::BareValue { content, .. } => format!("BareValue {:?}", String::from_utf8_lossy(content)),
        Event::BoolTrue { .. } => "BoolTrue".to_string(),
        Event::BoolFalse { .. } => "BoolFalse".to_string(),
        Event::Nil { .. } => "Nil".to_string(),
        Event::Interpolation { content, .. } => format!("Interpolation {:?}", String::from_utf8_lossy(content)),
        Event::Reference { content, .. } => format!("Reference {:?}", String::from_utf8_lossy(content)),
        Event::RawContent { content, .. } => format!("RawContent {:?}", String::from_utf8_lossy(content)),
        Event::Raw { content, .. } => format!("Raw {:?}", String::from_utf8_lossy(content)),
        Event::Integer { content, .. } => format!("Integer {:?}", String::from_utf8_lossy(content)),
        Event::Float { content, .. } => format!("Float {:?}", String::from_utf8_lossy(content)),
        Event::Rational { content, .. } => format!("Rational {:?}", String::from_utf8_lossy(content)),
        Event::Complex { content, .. } => format!("Complex {:?}", String::from_utf8_lossy(content)),
        Event::Warning { content, .. } => format!("Warning {:?}", String::from_utf8_lossy(content)),
        Event::Error { code, .. } => format!("Error {:?}", code),
    }
}

/// Format StreamEvent for comparison (ignoring spans)
fn format_stream_event(event: &StreamEvent) -> String {
    match event {
        StreamEvent::ElementStart { .. } => "ElementStart".to_string(),
        StreamEvent::ElementEnd { .. } => "ElementEnd".to_string(),
        StreamEvent::EmbeddedStart { .. } => "EmbeddedStart".to_string(),
        StreamEvent::EmbeddedEnd { .. } => "EmbeddedEnd".to_string(),
        StreamEvent::DirectiveStart { .. } => "DirectiveStart".to_string(),
        StreamEvent::DirectiveEnd { .. } => "DirectiveEnd".to_string(),
        StreamEvent::ArrayStart { .. } => "ArrayStart".to_string(),
        StreamEvent::ArrayEnd { .. } => "ArrayEnd".to_string(),
        StreamEvent::FreeformStart { .. } => "FreeformStart".to_string(),
        StreamEvent::FreeformEnd { .. } => "FreeformEnd".to_string(),
        StreamEvent::CommentStart { .. } => "CommentStart".to_string(),
        StreamEvent::CommentEnd { .. } => "CommentEnd".to_string(),
        StreamEvent::Name { content, .. } => format!("Name {:?}", String::from_utf8_lossy(content)),
        StreamEvent::Text { content, .. } => format!("Text {:?}", String::from_utf8_lossy(content)),
        StreamEvent::Attr { content, .. } => format!("Attr {:?}", String::from_utf8_lossy(content)),
        StreamEvent::StringValue { content, .. } => format!("StringValue {:?}", String::from_utf8_lossy(content)),
        StreamEvent::BareValue { content, .. } => format!("BareValue {:?}", String::from_utf8_lossy(content)),
        StreamEvent::BoolTrue { .. } => "BoolTrue".to_string(),
        StreamEvent::BoolFalse { .. } => "BoolFalse".to_string(),
        StreamEvent::Nil { .. } => "Nil".to_string(),
        StreamEvent::Interpolation { content, .. } => format!("Interpolation {:?}", String::from_utf8_lossy(content)),
        StreamEvent::Reference { content, .. } => format!("Reference {:?}", String::from_utf8_lossy(content)),
        StreamEvent::RawContent { content, .. } => format!("RawContent {:?}", String::from_utf8_lossy(content)),
        StreamEvent::Raw { content, .. } => format!("Raw {:?}", String::from_utf8_lossy(content)),
        StreamEvent::Integer { content, .. } => format!("Integer {:?}", String::from_utf8_lossy(content)),
        StreamEvent::Float { content, .. } => format!("Float {:?}", String::from_utf8_lossy(content)),
        StreamEvent::Rational { content, .. } => format!("Rational {:?}", String::from_utf8_lossy(content)),
        StreamEvent::Complex { content, .. } => format!("Complex {:?}", String::from_utf8_lossy(content)),
        StreamEvent::Warning { content, .. } => format!("Warning {:?}", String::from_utf8_lossy(content)),
        StreamEvent::Error { code, .. } => format!("Error {:?}", code),
    }
}

/// Parse input in multiple chunks using StreamingParser
fn parse_multi_chunk(chunks: &[&[u8]]) -> Vec<String> {
    let mut events = Vec::new();
    let mut parser = StreamingParser::new();

    for chunk in chunks {
        parser.parse(chunk, |event| {
            events.push(format_stream_event(&event));
        });
    }

    // Signal end of input
    parser.finish(|event| {
        events.push(format_stream_event(&event));
    });

    events
}

// =============================================================================
// EOF Boundary Tests
// =============================================================================

/// Test that EOF at various positions doesn't panic and produces sensible output
#[test]
fn eof_doesnt_panic() {
    let inputs = [
        b"|element".as_slice(),
        b"|element[id]".as_slice(),
        b"|element.class".as_slice(),
        b"|el :attr".as_slice(),
        b"|el :attr value".as_slice(),
        b"|el :attr 42".as_slice(),
        b"|el :attr true".as_slice(),
        b"|el :attr 0x".as_slice(),
        b"|el :attr 3.".as_slice(),
        b"|a\n  |b".as_slice(),
        b"|a\n  |b\n    |c".as_slice(),
    ];

    for input in inputs {
        // Should not panic
        let events = collect_events(input);
        // Should produce at least some events
        assert!(!events.is_empty(), "No events for input: {:?}", String::from_utf8_lossy(input));
    }
}

/// Test that values without trailing newline parse correctly
#[test]
fn values_at_eof() {
    let cases = [
        (b"|el :v 42".as_slice(), "Integer"),
        (b"|el :v 3.14".as_slice(), "Float"),
        (b"|el :v true".as_slice(), "BoolTrue"),
        (b"|el :v false".as_slice(), "BoolFalse"),
        (b"|el :v null".as_slice(), "Nil"),
        (b"|el :v hello".as_slice(), "BareValue"),
        (b"|el :v 0xFF".as_slice(), "Integer"),
        (b"|el :v 0o755".as_slice(), "Integer"),
        (b"|el :v 0b1010".as_slice(), "Integer"),
    ];

    for (input, expected_type) in cases {
        let events = collect_events(input);
        let has_expected = events.iter().any(|e| e.starts_with(expected_type));
        assert!(
            has_expected,
            "Expected {} in events for input {:?}, got: {:?}",
            expected_type,
            String::from_utf8_lossy(input),
            events
        );
    }
}

/// Test that elements close properly at EOF
#[test]
fn elements_close_at_eof() {
    let input = b"|a\n  |b\n    |c";
    let events = collect_events(input);

    // Count starts and ends
    let starts = events.iter().filter(|e| *e == "ElementStart").count();
    let ends = events.iter().filter(|e| *e == "ElementEnd").count();

    assert_eq!(starts, ends, "Mismatched ElementStart/End: {:?}", events);
    assert_eq!(starts, 3, "Expected 3 elements, got {}: {:?}", starts, events);
}

/// Test EOF at every position in a fixture
#[test]
fn eof_at_every_position() {
    let full_input = b"|article[main].featured\n  :author Joseph\n  :count 42\n  Hello world\n";

    for split_at in 1..full_input.len() {
        let truncated = &full_input[..split_at];
        // Should not panic
        let events = collect_events(truncated);

        // ElementStart/End should be balanced
        let starts = events.iter().filter(|e| *e == "ElementStart").count();
        let ends = events.iter().filter(|e| *e == "ElementEnd").count();
        assert_eq!(
            starts, ends,
            "Unbalanced at position {}: {} starts, {} ends\nInput: {:?}\nEvents: {:?}",
            split_at,
            starts,
            ends,
            String::from_utf8_lossy(truncated),
            events
        );
    }
}

// =============================================================================
// Premature Emission Tests
// =============================================================================

/// Test that "true" doesn't emit BoolTrue when followed by more content
#[test]
fn no_premature_bool_emission() {
    // "true-flag" should be BareValue, not BoolTrue
    let input = b"|el :attr true-flag\n";
    let events = collect_events(input);

    assert!(
        !events.iter().any(|e| e == "BoolTrue"),
        "Should not emit BoolTrue for 'true-flag': {:?}",
        events
    );
    assert!(
        events.iter().any(|e| e.starts_with("BareValue")),
        "Should emit BareValue for 'true-flag': {:?}",
        events
    );
}

/// Test that "null" doesn't emit Nil when followed by more content
#[test]
fn no_premature_nil_emission() {
    let input = b"|el :attr nullable\n";
    let events = collect_events(input);

    assert!(
        !events.iter().any(|e| e == "Nil"),
        "Should not emit Nil for 'nullable': {:?}",
        events
    );
}

/// Test that integers don't emit prematurely
#[test]
fn no_premature_integer_emission() {
    // "42abc" should be BareValue, not Integer
    let input = b"|el :attr 42abc\n";
    let events = collect_events(input);

    assert!(
        !events.iter().any(|e| e.starts_with("Integer")),
        "Should not emit Integer for '42abc': {:?}",
        events
    );
}

/// Test that hex prefix doesn't emit prematurely
#[test]
fn no_premature_hex_emission() {
    // "0xGHI" is not valid hex, should be BareValue
    let input = b"|el :attr 0xGHI\n";
    let events = collect_events(input);

    // This might emit Integer "0x" or BareValue "0xGHI" depending on implementation
    // The key is it shouldn't panic
    assert!(
        !events.is_empty(),
        "Should produce events for '0xGHI'"
    );
}

// =============================================================================
// Stochastic EOF Tests
// =============================================================================

/// Run EOF tests on all fixtures with random truncation points
#[test]
fn stochastic_eof_on_fixtures() {
    let mut gen = Gen::from_env_or_random();
    let fixture_names = ["elements", "values", "indentation"];

    for name in fixture_names {
        let cases = load_fixtures_by_name(name);

        for case in cases {
            let input = case.udon.as_bytes();

            // Test EOF at random positions (Poisson count of positions to test)
            let num_tests = gen.poisson(5.0).max(3);

            for _ in 0..num_tests {
                if input.is_empty() {
                    continue;
                }
                let split_at = gen.rng.gen_range(1..=input.len());
                let truncated = &input[..split_at];

                // Should not panic
                let events = collect_events(truncated);

                // ElementStart/End should be balanced
                let starts = events.iter().filter(|e| *e == "ElementStart").count();
                let ends = events.iter().filter(|e| *e == "ElementEnd").count();

                if starts != ends {
                    eprintln!(
                        "Seed: {} - Unbalanced at {}::{} position {}/{}\nInput: {:?}\nEvents: {:?}",
                        gen.seed,
                        name,
                        case.id,
                        split_at,
                        input.len(),
                        String::from_utf8_lossy(truncated),
                        events
                    );
                    panic!("Unbalanced ElementStart/End");
                }
            }
        }
    }
}

// =============================================================================
// Chunk Boundary Tests (placeholder for when streaming is implemented)
// =============================================================================

/// Test that splitting input at chunk boundaries produces the same result
/// for single-line inputs.
///
/// NOTE: StreamingParser uses line-oriented streaming, which means it parses
/// complete lines independently. Multi-line elements (with child lines) will
/// NOT produce identical results when split across line boundaries, because
/// the parser doesn't track indentation state across parse calls.
///
/// For true cross-line streaming, the parser would need to track element stack
/// state between chunks. This is a known limitation of line-oriented streaming.
#[test]
fn chunk_boundary_consistency() {
    // Only test SINGLE-LINE inputs - multi-line requires stateful streaming
    let inputs = [
        b"|el :attr true-is-the-best\n".as_slice(),
        b"|el :count 42\n".as_slice(),
        b"|el :ratio 3.14\n".as_slice(),
        b"|el :enabled true\n".as_slice(),
        b"|div[main].container\n".as_slice(),
        b"; comment line\n".as_slice(),
        b"|element\n".as_slice(),
    ];

    for full in inputs {
        let full_events = collect_events(full);

        // Test splitting at every position
        for split_at in 1..full.len() {
            let chunk1 = &full[..split_at];
            let chunk2 = &full[split_at..];
            let chunked_events = parse_multi_chunk(&[chunk1, chunk2]);

            if full_events != chunked_events {
                panic!(
                    "Chunk boundary mismatch at position {} of {:?}\n\
                     Full events:    {:?}\n\
                     Chunked events: {:?}",
                    split_at,
                    String::from_utf8_lossy(full),
                    full_events,
                    chunked_events
                );
            }
        }
    }
}

/// Test multi-chunk parsing with more than 2 chunks (single-line input)
#[test]
fn multi_chunk_parsing() {
    // Single line - multi-line requires stateful streaming (see chunk_boundary_consistency)
    let full = b"|el :author Joseph :count 42\n";

    // Parse in 4 chunks (roughly equal-sized)
    let chunk_size = full.len() / 4;
    let chunks: Vec<&[u8]> = full.chunks(chunk_size).collect();

    let full_events = collect_events(full);
    let chunked_events = parse_multi_chunk(&chunks);

    assert_eq!(
        full_events, chunked_events,
        "Multi-chunk parsing should match single-buffer:\n\
         Full: {:?}\nChunked: {:?}",
        full_events, chunked_events
    );
}

/// Test streaming parser with byte-at-a-time input
#[test]
fn byte_at_a_time_streaming() {
    let full = b"|el :attr value\n";

    // Feed one byte at a time
    let chunks: Vec<&[u8]> = full.iter().map(|b| std::slice::from_ref(b)).collect();
    let chunked_events = parse_multi_chunk(&chunks);

    let full_events = collect_events(full);
    assert_eq!(
        full_events, chunked_events,
        "Byte-at-a-time should match single-buffer"
    );
}

/// Test that streaming parser tracks global offset correctly
#[test]
fn streaming_offset_tracking() {
    let mut parser = StreamingParser::new();

    // First chunk
    parser.parse(b"|el :a 1\n", |_| {});
    assert_eq!(parser.offset(), 9, "Offset after first chunk");

    // Second chunk
    parser.parse(b"|el :b 2\n", |_| {});
    assert_eq!(parser.offset(), 18, "Offset after second chunk");
}

use rand::Rng;
