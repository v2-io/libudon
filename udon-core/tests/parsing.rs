//! Integration tests for UDON parsing.
//!
//! Organized by grammar construct, from simplest to most complex.
//! Each test specifies expected events explicitly.

use udon_core::{Event, Parser};

// =============================================================================
// Test Helpers
// =============================================================================

/// Parse input and return events, filtering out spans for easier comparison.
fn parse(input: &[u8]) -> Vec<EventKind> {
    let mut parser = Parser::new(input);
    parser.parse().into_iter().map(EventKind::from).collect()
}

/// Simplified event representation for testing (ignores spans).
///
/// NOTE: This now matches the streaming event model where:
/// - ElementStart just has name (id/classes/suffix emit as separate Attribute events)
/// - Attribute just has key (value comes as a separate event afterward)
#[derive(Debug, PartialEq)]
enum EventKind {
    // Structure events
    ElementStart { name: Option<Vec<u8>> },
    ElementEnd,

    // Attribute event (value follows as separate event)
    Attribute { key: Vec<u8> },

    // Value events
    ArrayStart,
    ArrayEnd,
    NilValue,
    BoolValue(bool),
    IntegerValue(i64),
    FloatValue(f64),
    StringValue(Vec<u8>),
    QuotedStringValue(Vec<u8>),

    // Content events
    Text(Vec<u8>),
    Comment(Vec<u8>),
    RawContent(Vec<u8>),

    // Directive events
    DirectiveStart { name: Vec<u8>, namespace: Option<Vec<u8>> },
    DirectiveEnd,
    Interpolation(Vec<u8>),

    // Error
    Error(String),
}

impl From<Event<'_>> for EventKind {
    fn from(event: Event<'_>) -> Self {
        match event {
            // Structure events
            Event::ElementStart { name, .. } => EventKind::ElementStart {
                name: name.map(|n| n.to_vec()),
            },
            Event::ElementEnd { .. } => EventKind::ElementEnd,

            // Attribute event (key only, value follows separately)
            Event::Attribute { key, .. } => EventKind::Attribute {
                key: key.to_vec(),
            },

            // Value events
            Event::ArrayStart { .. } => EventKind::ArrayStart,
            Event::ArrayEnd { .. } => EventKind::ArrayEnd,
            Event::NilValue { .. } => EventKind::NilValue,
            Event::BoolValue { value, .. } => EventKind::BoolValue(value),
            Event::IntegerValue { value, .. } => EventKind::IntegerValue(value),
            Event::FloatValue { value, .. } => EventKind::FloatValue(value),
            Event::StringValue { value, .. } => EventKind::StringValue(value.to_vec()),
            Event::QuotedStringValue { value, .. } => EventKind::QuotedStringValue(value.to_vec()),

            // Content events
            Event::Text { content, .. } => EventKind::Text(content.to_vec()),
            Event::Comment { content, .. } => EventKind::Comment(content.to_vec()),
            Event::RawContent { content, .. } => EventKind::RawContent(content.to_vec()),

            // Directive events
            Event::DirectiveStart { name, namespace, .. } => EventKind::DirectiveStart {
                name: name.to_vec(),
                namespace: namespace.map(|n| n.to_vec()),
            },
            Event::DirectiveEnd { .. } => EventKind::DirectiveEnd,
            Event::Interpolation { expression, .. } => EventKind::Interpolation(expression.to_vec()),

            // Map other events as needed
            _ => EventKind::Error("Unexpected event type".to_string()),
        }
    }
}

// =============================================================================
// Phase 1: Comments and Text (Currently Implemented)
// =============================================================================

mod comments_and_text {
    use super::*;

    #[test]
    fn empty_input() {
        let events = parse(b"");
        assert_eq!(events, vec![]);
    }

    #[test]
    fn single_comment() {
        let events = parse(b"; this is a comment\n");
        assert_eq!(events, vec![EventKind::Comment(b" this is a comment".to_vec())]);
    }

    #[test]
    fn multiple_comments() {
        let events = parse(b"; first\n; second\n; third\n");
        assert_eq!(
            events,
            vec![
                EventKind::Comment(b" first".to_vec()),
                EventKind::Comment(b" second".to_vec()),
                EventKind::Comment(b" third".to_vec()),
            ]
        );
    }

    #[test]
    fn simple_text() {
        let events = parse(b"Hello world\n");
        assert_eq!(events, vec![EventKind::Text(b"Hello world".to_vec())]);
    }

    #[test]
    fn text_with_comment() {
        let events = parse(b"Some text ; with comment\n");
        assert_eq!(
            events,
            vec![
                EventKind::Text(b"Some text ".to_vec()),
                EventKind::Comment(b" with comment".to_vec()),
            ]
        );
    }

    #[test]
    fn blank_lines() {
        let events = parse(b"\n\n\n");
        assert_eq!(events, vec![]);
    }

    #[test]
    fn mixed_content() {
        let events = parse(b"; Comment\nText line\n; Another comment\n");
        assert_eq!(
            events,
            vec![
                EventKind::Comment(b" Comment".to_vec()),
                EventKind::Text(b"Text line".to_vec()),
                EventKind::Comment(b" Another comment".to_vec()),
            ]
        );
    }
}

// =============================================================================
// Phase 2: Elements (To Be Implemented)
// =============================================================================

mod elements {
    use super::*;

    #[test]
    fn simple_element() {
        let events = parse(b"|div\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart { name: Some(b"div".to_vec()) },
                EventKind::ElementEnd,
            ]
        );
    }

    #[test]
    fn element_with_id() {
        // In streaming model: ElementStart, Attribute($id), StringValue, ElementEnd
        let events = parse(b"|div[main]\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart { name: Some(b"div".to_vec()) },
                EventKind::Attribute { key: b"$id".to_vec() },
                EventKind::StringValue(b"main".to_vec()),
                EventKind::ElementEnd,
            ]
        );
    }

    #[test]
    fn element_with_classes() {
        // In streaming model: each .class emits Attribute($class), StringValue
        let events = parse(b"|div.container.wide\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart { name: Some(b"div".to_vec()) },
                EventKind::Attribute { key: b"$class".to_vec() },
                EventKind::StringValue(b"container".to_vec()),
                EventKind::Attribute { key: b"$class".to_vec() },
                EventKind::StringValue(b"wide".to_vec()),
                EventKind::ElementEnd,
            ]
        );
    }

    #[test]
    fn element_with_id_and_classes() {
        let events = parse(b"|div[main].container.wide\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart { name: Some(b"div".to_vec()) },
                EventKind::Attribute { key: b"$id".to_vec() },
                EventKind::StringValue(b"main".to_vec()),
                EventKind::Attribute { key: b"$class".to_vec() },
                EventKind::StringValue(b"container".to_vec()),
                EventKind::Attribute { key: b"$class".to_vec() },
                EventKind::StringValue(b"wide".to_vec()),
                EventKind::ElementEnd,
            ]
        );
    }

    #[test]
    fn anonymous_element_with_id() {
        let events = parse(b"|[only-id]\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart { name: None },
                EventKind::Attribute { key: b"$id".to_vec() },
                EventKind::StringValue(b"only-id".to_vec()),
                EventKind::ElementEnd,
            ]
        );
    }

    #[test]
    fn class_only_element() {
        let events = parse(b"|.mixin.another\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart { name: None },
                EventKind::Attribute { key: b"$class".to_vec() },
                EventKind::StringValue(b"mixin".to_vec()),
                EventKind::Attribute { key: b"$class".to_vec() },
                EventKind::StringValue(b"another".to_vec()),
                EventKind::ElementEnd,
            ]
        );
    }

    #[test]
    fn element_with_inline_content() {
        let events = parse(b"|div Hello world\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart { name: Some(b"div".to_vec()) },
                EventKind::Text(b"Hello world".to_vec()),
                EventKind::ElementEnd,
            ]
        );
    }

    #[test]
    fn nested_elements_rightward() {
        // |a |b |c means a > b > c
        let events = parse(b"|a |b |c\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart { name: Some(b"a".to_vec()) },
                EventKind::ElementStart { name: Some(b"b".to_vec()) },
                EventKind::ElementStart { name: Some(b"c".to_vec()) },
                EventKind::ElementEnd, // c
                EventKind::ElementEnd, // b
                EventKind::ElementEnd, // a
            ]
        );
    }

    #[test]
    fn element_with_suffix_after_name() {
        // Suffix emits as Attribute("?") + BoolValue(true)
        let events = parse(b"|field?\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart { name: Some(b"field".to_vec()) },
                EventKind::Attribute { key: b"?".to_vec() },
                EventKind::BoolValue(true),
                EventKind::ElementEnd,
            ]
        );
    }

    #[test]
    fn element_with_suffix_after_id() {
        let events = parse(b"|field[name]!\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart { name: Some(b"field".to_vec()) },
                EventKind::Attribute { key: b"$id".to_vec() },
                EventKind::StringValue(b"name".to_vec()),
                EventKind::Attribute { key: b"!".to_vec() },
                EventKind::BoolValue(true),
                EventKind::ElementEnd,
            ]
        );
    }

    #[test]
    fn element_with_all_suffixes() {
        // Test each suffix type - each emits Attribute(suffix) + BoolValue(true)
        for (input, expected_suffix) in [
            (b"|x?\n".as_slice(), b"?".as_slice()),
            (b"|x!\n".as_slice(), b"!".as_slice()),
            (b"|x*\n".as_slice(), b"*".as_slice()),
            (b"|x+\n".as_slice(), b"+".as_slice()),
        ] {
            let events = parse(input);
            assert_eq!(
                events,
                vec![
                    EventKind::ElementStart {
                        name: Some(b"x".to_vec()),
                    },
                    EventKind::Attribute { key: expected_suffix.to_vec() },
                    EventKind::BoolValue(true),
                    EventKind::ElementEnd,
                ],
                "Failed for suffix {:?}", expected_suffix
            );
        }
    }

    #[test]
    fn element_with_numeric_id() {
        // Per SPEC: |step[1] should parse 1 as id
        // Note: Currently emits as StringValue, may change to IntegerValue in future
        let events = parse(b"|step[1]\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart {
                    name: Some(b"step".to_vec()),
                },
                EventKind::Attribute { key: b"$id".to_vec() },
                EventKind::StringValue(b"1".to_vec()), // Currently emits as string
                EventKind::ElementEnd,
            ]
        );
    }
}

// =============================================================================
// Phase 3: Attributes (To Be Implemented)
// =============================================================================

mod attributes {
    use super::*;

    #[test]
    fn simple_attribute() {
        let events = parse(b"|div :class container\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart {
                    name: Some(b"div".to_vec()),
                },
                EventKind::Attribute {
                    key: b"class".to_vec(),
                },
                EventKind::StringValue(b"container".to_vec()),
                EventKind::ElementEnd,
            ]
        );
    }

    #[test]
    fn flag_attribute() {
        let events = parse(b"|button :disabled\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart {
                    name: Some(b"button".to_vec()),
                },
                EventKind::Attribute {
                    key: b"disabled".to_vec(),
                },
                EventKind::BoolValue(true),
                EventKind::ElementEnd,
            ]
        );
    }

    #[test]
    fn quoted_string_value() {
        let events = parse(b"|div :title \"Hello World\"\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart {
                    name: Some(b"div".to_vec()),
                },
                EventKind::Attribute {
                    key: b"title".to_vec(),
                },
                EventKind::QuotedStringValue(b"Hello World".to_vec()),
                EventKind::ElementEnd,
            ]
        );
    }

    #[test]
    fn indented_attributes() {
        // Attributes can appear on indented lines after an element
        let events = parse(b"|div\n  :title Hello\n  :author Joseph\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart {
                    name: Some(b"div".to_vec()),
                },
                EventKind::Attribute {
                    key: b"title".to_vec(),
                },
                EventKind::StringValue(b"Hello".to_vec()),
                EventKind::Attribute {
                    key: b"author".to_vec(),
                },
                EventKind::StringValue(b"Joseph".to_vec()),
                EventKind::ElementEnd,
            ]
        );
    }

    #[test]
    fn indented_flag_attribute() {
        let events = parse(b"|button\n  :disabled\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart {
                    name: Some(b"button".to_vec()),
                },
                EventKind::Attribute {
                    key: b"disabled".to_vec(),
                },
                EventKind::BoolValue(true),
                EventKind::ElementEnd,
            ]
        );
    }

    #[test]
    fn list_attribute_inline() {
        // Test list parsing in inline attributes with streaming events
        let events = parse(b"|server :ports [8080 8443 9000]\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart {
                    name: Some(b"server".to_vec()),
                },
                EventKind::Attribute {
                    key: b"ports".to_vec(),
                },
                EventKind::ArrayStart,
                EventKind::IntegerValue(8080),
                EventKind::IntegerValue(8443),
                EventKind::IntegerValue(9000),
                EventKind::ArrayEnd,
                EventKind::ElementEnd,
            ]
        );
    }

    #[test]
    fn list_attribute_indented() {
        // Test list parsing in indented attributes with streaming events
        let events = parse(b"|config\n  :tags [api public internal]\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart {
                    name: Some(b"config".to_vec()),
                },
                EventKind::Attribute {
                    key: b"tags".to_vec(),
                },
                EventKind::ArrayStart,
                EventKind::StringValue(b"api".to_vec()),
                EventKind::StringValue(b"public".to_vec()),
                EventKind::StringValue(b"internal".to_vec()),
                EventKind::ArrayEnd,
                EventKind::ElementEnd,
            ]
        );
    }

    #[test]
    fn list_with_quoted_strings() {
        // Test array with quoted strings in streaming model
        let events = parse(b"|app :env [\"production\" \"staging\" \"dev\"]\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart {
                    name: Some(b"app".to_vec()),
                },
                EventKind::Attribute {
                    key: b"env".to_vec(),
                },
                EventKind::ArrayStart,
                EventKind::QuotedStringValue(b"production".to_vec()),
                EventKind::QuotedStringValue(b"staging".to_vec()),
                EventKind::QuotedStringValue(b"dev".to_vec()),
                EventKind::ArrayEnd,
                EventKind::ElementEnd,
            ]
        );
    }

    #[test]
    fn list_with_mixed_types() {
        // Test array with mixed types in streaming model
        let events = parse(b"|data :values [42 true hello 3.14]\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart {
                    name: Some(b"data".to_vec()),
                },
                EventKind::Attribute {
                    key: b"values".to_vec(),
                },
                EventKind::ArrayStart,
                EventKind::IntegerValue(42),
                EventKind::BoolValue(true),
                EventKind::StringValue(b"hello".to_vec()),
                EventKind::FloatValue(3.14),
                EventKind::ArrayEnd,
                EventKind::ElementEnd,
            ]
        );
    }
}

// =============================================================================
// Phase 4: Directives (To Be Implemented)
// =============================================================================

mod directives {
    use super::*;

    #[test]
    #[ignore = "directives not yet implemented"]
    fn block_directive() {
        let events = parse(b"!if user\n  |div Welcome\n");
        assert_eq!(
            events,
            vec![
                EventKind::DirectiveStart {
                    name: b"if".to_vec(),
                    namespace: None,
                },
                EventKind::ElementStart {
                    name: Some(b"div".to_vec()),
                },
                EventKind::Text(b"Welcome".to_vec()),
                EventKind::ElementEnd,
                EventKind::DirectiveEnd,
            ]
        );
    }

    #[test]
    #[ignore = "directives not yet implemented"]
    fn raw_directive() {
        let events = parse(b"!raw:sql\n  SELECT * FROM users\n");
        assert_eq!(
            events,
            vec![
                EventKind::DirectiveStart {
                    name: b"sql".to_vec(),
                    namespace: Some(b"raw".to_vec()),
                },
                EventKind::RawContent(b"SELECT * FROM users\n".to_vec()),
                EventKind::DirectiveEnd,
            ]
        );
    }

    #[test]
    #[ignore = "directives not yet implemented"]
    fn interpolation() {
        let events = parse(b"Hello !{user.name}!\n");
        assert_eq!(
            events,
            vec![
                EventKind::Text(b"Hello ".to_vec()),
                EventKind::Interpolation(b"user.name".to_vec()),
                EventKind::Text(b"!".to_vec()),
            ]
        );
    }
}

// =============================================================================
// Phase 5: Escape Sequences (To Be Implemented)
// =============================================================================

mod escapes {
    use super::*;

    #[test]
    fn escaped_pipe() {
        // '| should be literal pipe, not element
        let events = parse(b"'|not-an-element\n");
        assert_eq!(events, vec![EventKind::Text(b"|not-an-element".to_vec())]);
    }

    #[test]
    fn escaped_colon() {
        let events = parse(b"':not-an-attribute\n");
        assert_eq!(events, vec![EventKind::Text(b":not-an-attribute".to_vec())]);
    }

    #[test]
    fn escaped_semicolon() {
        let events = parse(b"';not-a-comment\n");
        assert_eq!(events, vec![EventKind::Text(b";not-a-comment".to_vec())]);
    }

    #[test]
    fn escaped_apostrophe() {
        let events = parse(b"''literal apostrophe\n");
        assert_eq!(events, vec![EventKind::Text(b"'literal apostrophe".to_vec())]);
    }
}

// =============================================================================
// Phase 6: Indentation (To Be Implemented)
// =============================================================================

mod indentation {
    use super::*;

    #[test]
    fn child_by_indent() {
        let events = parse(b"|parent\n  |child\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart {
                    name: Some(b"parent".to_vec()),
                },
                EventKind::ElementStart {
                    name: Some(b"child".to_vec()),
                },
                EventKind::ElementEnd, // child
                EventKind::ElementEnd, // parent
            ]
        );
    }

    #[test]
    fn sibling_by_same_indent() {
        let events = parse(b"|parent\n  |child1\n  |child2\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart {
                    name: Some(b"parent".to_vec()),
                },
                EventKind::ElementStart {
                    name: Some(b"child1".to_vec()),
                },
                EventKind::ElementEnd, // child1
                EventKind::ElementStart {
                    name: Some(b"child2".to_vec()),
                },
                EventKind::ElementEnd, // child2
                EventKind::ElementEnd, // parent
            ]
        );
    }

    #[test]
    fn dedent_closes_multiple() {
        let events = parse(
            b"|a\n  |b\n    |c\n|d\n", // d is sibling of a, closes both c, b, and a
        );
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart {
                    name: Some(b"a".to_vec()),
                },
                EventKind::ElementStart {
                    name: Some(b"b".to_vec()),
                },
                EventKind::ElementStart {
                    name: Some(b"c".to_vec()),
                },
                EventKind::ElementEnd, // c
                EventKind::ElementEnd, // b
                EventKind::ElementEnd, // a
                EventKind::ElementStart {
                    name: Some(b"d".to_vec()),
                },
                EventKind::ElementEnd, // d
            ]
        );
    }

    #[test]
    fn inline_then_indented_prose() {
        // |first |second Some prose
        //   This prose is child of |first, sibling of |second
        let events = parse(b"|first |second Some prose\n  This prose\n");
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart {
                    name: Some(b"first".to_vec()),
                },
                EventKind::ElementStart {
                    name: Some(b"second".to_vec()),
                },
                EventKind::Text(b"Some prose".to_vec()),
                EventKind::ElementEnd, // second (closed by dedent to col 2)
                EventKind::Text(b"This prose".to_vec()),
                EventKind::ElementEnd, // first
            ]
        );
    }

    #[test]
    fn inline_triple_with_dedent() {
        // |first |second |third  Inner text
        //                This prose is inside |second, after |third closed
        //        This prose is inside |first, sibling of |second
        // This is sibling of |first
        let input = b"|first |second |third Inner\n               After third\n       Inside first\nSibling of first\n";
        let events = parse(input);
        assert_eq!(
            events,
            vec![
                EventKind::ElementStart {
                    name: Some(b"first".to_vec()),
                },
                EventKind::ElementStart {
                    name: Some(b"second".to_vec()),
                },
                EventKind::ElementStart {
                    name: Some(b"third".to_vec()),
                },
                EventKind::Text(b"Inner".to_vec()),
                EventKind::ElementEnd, // third (col 15 <= 15)
                EventKind::Text(b"After third".to_vec()),
                EventKind::ElementEnd, // second (col 7 <= 7)
                EventKind::Text(b"Inside first".to_vec()),
                EventKind::ElementEnd, // first (col 0 <= 0)
                EventKind::Text(b"Sibling of first".to_vec()),
            ]
        );
    }

    #[test]
    fn tab_causes_error() {
        let events = parse(b"|div\n\t|child\n");
        // Should have an error event for the tab
        assert!(events.iter().any(|e| matches!(e, EventKind::Error(_))));
    }
}

// =============================================================================
// Fixture Tests: Parse real example files
// =============================================================================

mod fixtures {
    use super::*;

    #[test]
    fn comprehensive_parses_without_panic() {
        let input = include_bytes!("../../examples/comprehensive.udon");
        let mut parser = Parser::new(input);
        let events = parser.parse();
        // For now, just verify it doesn't panic and produces some events
        assert!(!events.is_empty(), "Should produce events");
    }

    #[test]
    fn minimal_parses_without_panic() {
        let input = include_bytes!("../../examples/minimal.udon");
        let mut parser = Parser::new(input);
        let events = parser.parse();
        assert!(!events.is_empty(), "Should produce events");
    }
}
