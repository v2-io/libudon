//! Property-based tests for the UDON parser
//!
//! These tests verify structural invariants that must hold for ANY input,
//! not just carefully crafted examples. proptest will generate thousands
//! of random inputs and shrink failures to minimal cases.

use proptest::prelude::*;
use udon_core::event::Event;
use udon_core::Parser;

// Limit test cases for debugging - increase once stable
fn config() -> ProptestConfig {
    ProptestConfig {
        cases: 100,  // Reduced from default 256
        max_shrink_iters: 100,
        timeout: 1000,  // 1 second timeout per case
        ..ProptestConfig::default()
    }
}

// =============================================================================
// Test Helpers
// =============================================================================

fn parse(input: &[u8]) -> Vec<Event<'_>> {
    let mut parser = Parser::new(input);
    parser.parse()
}

/// Count event types in a parse result
fn count_events(events: &[Event<'_>]) -> EventCounts {
    let mut counts = EventCounts::default();
    for event in events {
        match event {
            Event::ElementStart { .. } => counts.element_start += 1,
            Event::ElementEnd { .. } => counts.element_end += 1,
            Event::ArrayStart { .. } => counts.array_start += 1,
            Event::ArrayEnd { .. } => counts.array_end += 1,
            Event::Attribute { .. } => counts.attribute += 1,
            Event::Error { .. } => counts.error += 1,
            _ => counts.other += 1,
        }
    }
    counts
}

#[derive(Default, Debug)]
struct EventCounts {
    element_start: usize,
    element_end: usize,
    array_start: usize,
    array_end: usize,
    attribute: usize,
    error: usize,
    other: usize,
}

// =============================================================================
// Property: Parser Never Panics
// =============================================================================

proptest! {
    #![proptest_config(config())]

    /// The parser must never panic on any input, valid or invalid.
    /// This is the most fundamental property.
    #[test]
    fn parser_never_panics(input in prop::collection::vec(any::<u8>(), 0..1000)) {
        // Just parse - if it panics, the test fails
        let _ = parse(&input);
    }

    /// Parser never panics on ASCII-heavy input (more likely to be valid UDON)
    #[test]
    fn parser_never_panics_ascii(input in "[a-zA-Z0-9|:.\\[\\]\\n \\t'\"?!*+;{}@#$%^&()-=_/\\\\]{0,500}") {
        let _ = parse(input.as_bytes());
    }
}

// =============================================================================
// Property: Structural Balance
// =============================================================================

proptest! {
    #![proptest_config(config())]
    /// ElementStart and ElementEnd must be balanced.
    /// Every ElementStart must eventually have a matching ElementEnd.
    #[test]
    fn elements_are_balanced(input in prop::collection::vec(any::<u8>(), 0..500)) {
        let events = parse(&input);
        let counts = count_events(&events);

        prop_assert_eq!(
            counts.element_start,
            counts.element_end,
            "ElementStart ({}) != ElementEnd ({})",
            counts.element_start,
            counts.element_end
        );
    }

    /// ArrayStart and ArrayEnd must be balanced.
    #[test]
    fn arrays_are_balanced(input in prop::collection::vec(any::<u8>(), 0..500)) {
        let events = parse(&input);
        let counts = count_events(&events);

        prop_assert_eq!(
            counts.array_start,
            counts.array_end,
            "ArrayStart ({}) != ArrayEnd ({})",
            counts.array_start,
            counts.array_end
        );
    }

    /// During parsing, we should never have more ends than starts.
    /// This checks the nesting property incrementally.
    #[test]
    fn nesting_never_goes_negative(input in prop::collection::vec(any::<u8>(), 0..500)) {
        let events = parse(&input);

        let mut element_depth: i32 = 0;
        let mut array_depth: i32 = 0;

        for (i, event) in events.iter().enumerate() {
            match event {
                Event::ElementStart { .. } => element_depth += 1,
                Event::ElementEnd { .. } => element_depth -= 1,
                Event::ArrayStart { .. } => array_depth += 1,
                Event::ArrayEnd { .. } => array_depth -= 1,
                _ => {}
            }

            prop_assert!(
                element_depth >= 0,
                "Element depth went negative at event {}: {:?}",
                i, event
            );
            prop_assert!(
                array_depth >= 0,
                "Array depth went negative at event {}: {:?}",
                i, event
            );
        }
    }
}

// =============================================================================
// Property: Valid UDON Structures
// =============================================================================

proptest! {
    #![proptest_config(config())]
    /// A simple element |name should always produce ElementStart + ElementEnd
    #[test]
    fn simple_element_produces_two_events(name in "[a-zA-Z][a-zA-Z0-9_-]{0,20}") {
        let input = format!("|{}", name);
        let events = parse(input.as_bytes());

        prop_assert!(events.len() >= 2, "Expected at least 2 events, got {}", events.len());
        prop_assert!(
            matches!(events[0], Event::ElementStart { .. }),
            "First event should be ElementStart, got {:?}", events[0]
        );
        prop_assert!(
            matches!(events.last(), Some(Event::ElementEnd { .. })),
            "Last event should be ElementEnd, got {:?}", events.last()
        );
    }

    /// An empty array [] should produce ArrayStart + ArrayEnd
    #[test]
    fn empty_array_produces_balanced_events(attr_name in "[a-zA-Z][a-zA-Z0-9_-]{0,10}") {
        let input = format!("|foo\n  :{} []", attr_name);
        let events = parse(input.as_bytes());
        let counts = count_events(&events);

        prop_assert_eq!(counts.array_start, 1);
        prop_assert_eq!(counts.array_end, 1);
    }

    /// Nested arrays should produce balanced ArrayStart/ArrayEnd pairs
    #[test]
    fn nested_arrays_balanced(depth in 1usize..10) {
        let opens: String = (0..depth).map(|_| '[').collect();
        let closes: String = (0..depth).map(|_| ']').collect();
        let input = format!("|foo\n  :arr {}{}", opens, closes);
        let events = parse(input.as_bytes());
        let counts = count_events(&events);

        prop_assert_eq!(
            counts.array_start, depth,
            "Expected {} ArrayStart, got {}", depth, counts.array_start
        );
        prop_assert_eq!(
            counts.array_end, depth,
            "Expected {} ArrayEnd, got {}", depth, counts.array_end
        );
    }

    /// Nested elements via indentation should be balanced
    #[test]
    fn nested_elements_via_indent_balanced(depth in 1usize..20) {
        let mut input = String::new();
        for i in 0..depth {
            let indent = "  ".repeat(i);
            input.push_str(&format!("{}|level{}\n", indent, i));
        }
        let events = parse(input.as_bytes());
        let counts = count_events(&events);

        prop_assert_eq!(
            counts.element_start, depth,
            "Expected {} ElementStart, got {}", depth, counts.element_start
        );
        prop_assert_eq!(
            counts.element_end, depth,
            "Expected {} ElementEnd, got {}", depth, counts.element_end
        );
    }
}

// =============================================================================
// Property: Attributes
// =============================================================================

proptest! {
    #![proptest_config(config())]
    /// Every Attribute event should be followed by a value event or another
    /// structural event (Attribute, ElementEnd, ArrayStart, etc.)
    #[test]
    fn attribute_followed_by_value_or_structure(input in prop::collection::vec(any::<u8>(), 0..500)) {
        let events = parse(&input);

        for (i, event) in events.iter().enumerate() {
            if matches!(event, Event::Attribute { .. }) {
                // There should be a next event
                if i + 1 < events.len() {
                    let next = &events[i + 1];
                    // Next should be a value, another attribute, array, or end
                    let valid_next = matches!(
                        next,
                        Event::NilValue { .. }
                        | Event::BoolValue { .. }
                        | Event::IntegerValue { .. }
                        | Event::FloatValue { .. }
                        | Event::RationalValue { .. }
                        | Event::ComplexValue { .. }
                        | Event::StringValue { .. }
                        | Event::QuotedStringValue { .. }
                        | Event::Attribute { .. }
                        | Event::ArrayStart { .. }
                        | Event::ElementEnd { .. }
                        | Event::ElementStart { .. }
                        | Event::Error { .. }
                    );
                    prop_assert!(
                        valid_next,
                        "Attribute at {} followed by unexpected event: {:?}",
                        i, next
                    );
                }
            }
        }
    }
}

// =============================================================================
// Property: Comments and Text
// =============================================================================

proptest! {
    #![proptest_config(config())]
    /// Lines starting with ; should produce Comment events
    #[test]
    fn semicolon_lines_are_comments(comment_text in "[^\n]{0,50}") {
        let input = format!("; {}", comment_text);
        let events = parse(input.as_bytes());

        prop_assert!(
            events.iter().any(|e| matches!(e, Event::Comment { .. })),
            "Expected a Comment event for input starting with ;"
        );
    }

    /// Plain text lines (not starting with special chars) produce Text events
    #[test]
    fn plain_text_produces_text_event(text in "[a-zA-Z][a-zA-Z0-9 ]{0,50}") {
        // Ensure text doesn't start with special chars
        if !text.starts_with('|') && !text.starts_with(';') && !text.starts_with(':') {
            let events = parse(text.as_bytes());

            prop_assert!(
                events.iter().any(|e| matches!(e, Event::Text { .. })),
                "Expected a Text event for plain text input"
            );
        }
    }
}

// =============================================================================
// Property: Idempotence of re-parsing
// =============================================================================

proptest! {
    #![proptest_config(config())]
    /// Parsing the same input twice should produce identical results.
    /// (Tests determinism)
    #[test]
    fn parsing_is_deterministic(input in prop::collection::vec(any::<u8>(), 0..500)) {
        let events1 = parse(&input);
        let events2 = parse(&input);

        prop_assert_eq!(events1.len(), events2.len(), "Different number of events");

        // Compare event types (not spans, as those should be identical anyway)
        for (i, (e1, e2)) in events1.iter().zip(events2.iter()).enumerate() {
            let same_type = std::mem::discriminant(e1) == std::mem::discriminant(e2);
            prop_assert!(same_type, "Event {} differs: {:?} vs {:?}", i, e1, e2);
        }
    }
}
