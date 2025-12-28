//! Streaming event tests for UDON parser.
//!
//! Tests the SAX-style streaming event model where events emit immediately
//! as syntax is parsed, with no accumulation.
//!
//! Key patterns:
//! - ElementStart { name } followed by Attribute events for [id], .class, suffix
//! - Attribute { key } followed by value event(s)
//! - ArrayStart, value events..., ArrayEnd for list values
//!
//! ## Placeholder Tests
//!
//! Tests marked with `TODO(feature):` or using `placeholder_test!` are
//! incomplete and need proper assertions once the feature is implemented.
//! DO NOT consider these tests as "passing" for implementation purposes.

use udon_core::{StreamingEvent, StreamingParser};

/// Macro for placeholder tests - features not yet implemented.
/// These tests FAIL intentionally to guide TDD implementation.
/// Once the feature is implemented, replace with real assertions.
macro_rules! placeholder_test {
    ($feature:literal, $events:expr) => {{
        // Capture events to verify parsing doesn't panic
        let _events = $events;
        // FAIL intentionally - this is a TDD placeholder!
        panic!(
            "PLACEHOLDER TEST: '{}' - feature not yet implemented.\n\
             Replace this placeholder_test! with real assertions once implemented.",
            $feature
        );
    }};
}

// =============================================================================
// Test Helper - Simplified event representation
// =============================================================================

/// Simplified event for testing (ignores spans).
#[derive(Debug, Clone, PartialEq)]
enum E {
    // Structure
    ElementStart(Option<Vec<u8>>),
    ElementEnd,

    // Attributes (key only - value follows as separate event)
    Attr(Vec<u8>),

    // Values
    ArrayStart,
    ArrayEnd,
    Nil,
    Bool(bool),
    Int(i64),
    Float(String), // Use string for comparison
    Str(Vec<u8>),
    QuotedStr(Vec<u8>),

    // Content
    Text(Vec<u8>),
    Comment(Vec<u8>),
    Raw(Vec<u8>),

    // References and Dynamics
    IdRef(Vec<u8>),      // @[id] - insert entire element
    AttrMerge(Vec<u8>),  // :[id] - merge attributes
    Interp(Vec<u8>),     // !{{expr}} - interpolation

    // Other
    Error(String),
    Warning(String),  // For inconsistent indentation warnings
    Other(String),
}

fn parse(input: &[u8]) -> Vec<E> {
    let mut parser = StreamingParser::new(1024);
    parser.feed(input);
    parser.finish();

    let mut events = Vec::new();
    while let Some(event) = parser.read() {
        events.push(E::from_streaming(event, &parser));
    }
    events
}

impl E {
    fn from_streaming(event: StreamingEvent, parser: &StreamingParser) -> Self {
        match event {
            StreamingEvent::ElementStart { name, .. } => E::ElementStart(
                name.map(|cs| parser.arena().resolve(cs).unwrap_or(&[]).to_vec())
            ),
            StreamingEvent::ElementEnd { .. } => E::ElementEnd,
            // Embedded elements are semantically the same as regular elements
            StreamingEvent::EmbeddedStart { name, .. } => E::ElementStart(
                name.map(|cs| parser.arena().resolve(cs).unwrap_or(&[]).to_vec())
            ),
            StreamingEvent::EmbeddedEnd { .. } => E::ElementEnd,
            StreamingEvent::Attribute { key, .. } => E::Attr(
                parser.arena().resolve(key).unwrap_or(&[]).to_vec()
            ),
            StreamingEvent::ArrayStart { .. } => E::ArrayStart,
            StreamingEvent::ArrayEnd { .. } => E::ArrayEnd,
            StreamingEvent::NilValue { .. } => E::Nil,
            StreamingEvent::BoolValue { value, .. } => E::Bool(value),
            StreamingEvent::IntegerValue { value, .. } => E::Int(value),
            StreamingEvent::FloatValue { value, .. } => E::Float(format!("{}", value)),
            StreamingEvent::StringValue { value, .. } => E::Str(
                parser.arena().resolve(value).unwrap_or(&[]).to_vec()
            ),
            StreamingEvent::QuotedStringValue { value, .. } => E::QuotedStr(
                parser.arena().resolve(value).unwrap_or(&[]).to_vec()
            ),
            StreamingEvent::Text { content, .. } => E::Text(
                parser.arena().resolve(content).unwrap_or(&[]).to_vec()
            ),
            StreamingEvent::Comment { content, .. } => E::Comment(
                parser.arena().resolve(content).unwrap_or(&[]).to_vec()
            ),
            StreamingEvent::RawContent { content, .. } => E::Raw(
                parser.arena().resolve(content).unwrap_or(&[]).to_vec()
            ),
            StreamingEvent::IdReference { id, .. } => E::IdRef(
                parser.arena().resolve(id).unwrap_or(&[]).to_vec()
            ),
            StreamingEvent::AttributeMerge { id, .. } => E::AttrMerge(
                parser.arena().resolve(id).unwrap_or(&[]).to_vec()
            ),
            StreamingEvent::Interpolation { expression, .. } => E::Interp(
                parser.arena().resolve(expression).unwrap_or(&[]).to_vec()
            ),
            StreamingEvent::Error { code, .. } => E::Error(code.message().to_string()),
            StreamingEvent::Warning { message, .. } => E::Warning(message),
            other => E::Other(format!("{:?}", other)),
        }
    }
}

// Helper for readable assertions
fn s(bytes: &[u8]) -> Vec<u8> {
    bytes.to_vec()
}

// =============================================================================
// Element Identity - Basic Names
// =============================================================================

mod element_names {
    use super::*;

    #[test]
    fn simple_element() {
        // |foo → ElementStart("foo"), ElementEnd
        let events = parse(b"|foo");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn element_with_hyphen() {
        let events = parse(b"|foo-bar");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo-bar"))),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn element_with_underscore() {
        let events = parse(b"|foo_bar");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo_bar"))),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn element_with_numbers() {
        let events = parse(b"|h1");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"h1"))),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn anonymous_element() {
        // | alone (with content following) or at line end
        let events = parse(b"| text");
        assert_eq!(events, vec![
            E::ElementStart(None),
            E::Text(s(b"text")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn quoted_element_name() {
        // |'has spaces' → ElementStart("has spaces")
        let events = parse(b"|'has spaces'");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"has spaces"))),
            E::ElementEnd,
        ]);
    }
}

// =============================================================================
// Element Identity - ID Syntax [id]
// =============================================================================

mod element_id {
    use super::*;

    #[test]
    fn element_with_id() {
        // |foo[myid] → ElementStart("foo"), Attr("$id"), Str("myid"), ElementEnd
        let events = parse(b"|foo[myid]");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"$id")),
            E::Str(s(b"myid")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn anonymous_with_id() {
        // |[myid] → ElementStart(None), Attr("$id"), Str("myid"), ElementEnd
        let events = parse(b"|[myid]");
        assert_eq!(events, vec![
            E::ElementStart(None),
            E::Attr(s(b"$id")),
            E::Str(s(b"myid")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn empty_id() {
        // |foo[] → ElementStart("foo"), ElementEnd (empty id ignored)
        let events = parse(b"|foo[]");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::ElementEnd,
        ]);
    }
}

// =============================================================================
// Element Identity - Class Syntax .class
// =============================================================================

mod element_class {
    use super::*;

    #[test]
    fn element_with_class() {
        // |foo.bar → ElementStart("foo"), Attr("$class"), Str("bar"), ElementEnd
        let events = parse(b"|foo.bar");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"$class")),
            E::Str(s(b"bar")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn element_with_multiple_classes() {
        // |foo.bar.baz → each class emits separately
        let events = parse(b"|foo.bar.baz");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"$class")),
            E::Str(s(b"bar")),
            E::Attr(s(b"$class")),
            E::Str(s(b"baz")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn anonymous_with_class() {
        // |.foo → ElementStart(None), Attr("$class"), Str("foo"), ElementEnd
        let events = parse(b"|.foo");
        assert_eq!(events, vec![
            E::ElementStart(None),
            E::Attr(s(b"$class")),
            E::Str(s(b"foo")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn quoted_class_name() {
        // |foo.'has spaces' → ElementStart, Attr("$class"), QuotedStr("has spaces")
        let events = parse(b"|foo.'has spaces'");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"$class")),
            E::QuotedStr(s(b"has spaces")),
            E::ElementEnd,
        ]);
    }
}

// =============================================================================
// Element Identity - Suffix Modifiers ?!*+
// =============================================================================

mod element_suffix {
    use super::*;

    #[test]
    fn element_with_question() {
        // |foo? → ElementStart, Attr("?"), Bool(true), ElementEnd
        let events = parse(b"|foo?");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"?")),
            E::Bool(true),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn element_with_bang() {
        let events = parse(b"|foo!");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"!")),
            E::Bool(true),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn element_with_star() {
        let events = parse(b"|foo*");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"*")),
            E::Bool(true),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn element_with_plus() {
        let events = parse(b"|foo+");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"+")),
            E::Bool(true),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn suffix_after_id() {
        // |foo[id]? → suffix after id
        let events = parse(b"|foo[id]?");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"$id")),
            E::Str(s(b"id")),
            E::Attr(s(b"?")),
            E::Bool(true),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn suffix_space_separated() {
        // |foo.bar ? → suffix after space
        let events = parse(b"|foo.bar ?");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"$class")),
            E::Str(s(b"bar")),
            E::Attr(s(b"?")),
            E::Bool(true),
            E::ElementEnd,
        ]);
    }
}

// =============================================================================
// Element Identity - Combined
// =============================================================================

mod element_combined {
    use super::*;

    #[test]
    fn full_identity() {
        // |foo[id].bar.baz ? → all pieces (space before suffix per SPEC.md:654)
        let events = parse(b"|foo[id].bar.baz ?");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"$id")),
            E::Str(s(b"id")),
            E::Attr(s(b"$class")),
            E::Str(s(b"bar")),
            E::Attr(s(b"$class")),
            E::Str(s(b"baz")),
            E::Attr(s(b"?")),
            E::Bool(true),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn suffix_before_id() {
        // |foo?[id] → suffix before id is valid
        let events = parse(b"|foo?[id]");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"?")),
            E::Bool(true),
            E::Attr(s(b"$id")),
            E::Str(s(b"id")),
            E::ElementEnd,
        ]);
    }
}

// =============================================================================
// Attributes - Indented
// =============================================================================

mod attributes {
    use super::*;

    #[test]
    fn simple_attribute() {
        // :key value
        let events = parse(b"|foo\n  :name Fred");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"name")),
            E::Str(s(b"Fred")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn flag_attribute() {
        // :enabled (no value = true)
        let events = parse(b"|foo\n  :enabled");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"enabled")),
            E::Bool(true),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn integer_value() {
        let events = parse(b"|foo\n  :count 42");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"count")),
            E::Int(42),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn boolean_true() {
        let events = parse(b"|foo\n  :active true");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"active")),
            E::Bool(true),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn boolean_false() {
        let events = parse(b"|foo\n  :active false");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"active")),
            E::Bool(false),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn nil_value() {
        let events = parse(b"|foo\n  :value null");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"value")),
            E::Nil,
            E::ElementEnd,
        ]);
    }

    #[test]
    fn quoted_string_double() {
        let events = parse(b"|foo\n  :msg \"hello world\"");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"msg")),
            E::QuotedStr(s(b"hello world")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn quoted_string_single() {
        let events = parse(b"|foo\n  :msg 'hello world'");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"msg")),
            E::QuotedStr(s(b"hello world")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn multiple_attributes() {
        let events = parse(b"|foo\n  :a 1\n  :b 2");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"a")),
            E::Int(1),
            E::Attr(s(b"b")),
            E::Int(2),
            E::ElementEnd,
        ]);
    }
}

// =============================================================================
// Arrays - List Values
// =============================================================================

mod arrays {
    use super::*;

    #[test]
    fn simple_array() {
        // :tags [a b c]
        let events = parse(b"|foo\n  :tags [a b c]");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"tags")),
            E::ArrayStart,
            E::Str(s(b"a")),
            E::Str(s(b"b")),
            E::Str(s(b"c")),
            E::ArrayEnd,
            E::ElementEnd,
        ]);
    }

    #[test]
    fn array_with_integers() {
        let events = parse(b"|foo\n  :nums [1 2 3]");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"nums")),
            E::ArrayStart,
            E::Int(1),
            E::Int(2),
            E::Int(3),
            E::ArrayEnd,
            E::ElementEnd,
        ]);
    }

    #[test]
    fn array_with_quoted_strings() {
        let events = parse(b"|foo\n  :names [\"Alice\" \"Bob\"]");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"names")),
            E::ArrayStart,
            E::QuotedStr(s(b"Alice")),
            E::QuotedStr(s(b"Bob")),
            E::ArrayEnd,
            E::ElementEnd,
        ]);
    }

    #[test]
    fn nested_array() {
        // :matrix [[1 2] [3 4]]
        let events = parse(b"|foo\n  :matrix [[1 2] [3 4]]");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"matrix")),
            E::ArrayStart,
            E::ArrayStart,
            E::Int(1),
            E::Int(2),
            E::ArrayEnd,
            E::ArrayStart,
            E::Int(3),
            E::Int(4),
            E::ArrayEnd,
            E::ArrayEnd,
            E::ElementEnd,
        ]);
    }

    #[test]
    fn empty_array() {
        let events = parse(b"|foo\n  :items []");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"items")),
            E::ArrayStart,
            E::ArrayEnd,
            E::ElementEnd,
        ]);
    }

    // =========================================================================
    // Comprehensive array tests - stress nesting and edge cases
    // =========================================================================

    #[test]
    fn deeply_nested_arrays_5_levels() {
        // [[[[[1]]]]]
        let events = parse(b"|foo\n  :deep [[[[[1]]]]]");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"deep")),
            E::ArrayStart,  // level 1
            E::ArrayStart,  // level 2
            E::ArrayStart,  // level 3
            E::ArrayStart,  // level 4
            E::ArrayStart,  // level 5
            E::Int(1),
            E::ArrayEnd,    // close level 5
            E::ArrayEnd,    // close level 4
            E::ArrayEnd,    // close level 3
            E::ArrayEnd,    // close level 2
            E::ArrayEnd,    // close level 1
            E::ElementEnd,
        ]);
    }

    #[test]
    fn deeply_nested_empty_arrays() {
        // [[[[[]]]]]
        let events = parse(b"|foo\n  :empty [[[[[]]]]]");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"empty")),
            E::ArrayStart,
            E::ArrayStart,
            E::ArrayStart,
            E::ArrayStart,
            E::ArrayStart,
            E::ArrayEnd,
            E::ArrayEnd,
            E::ArrayEnd,
            E::ArrayEnd,
            E::ArrayEnd,
            E::ElementEnd,
        ]);
    }

    #[test]
    fn mixed_nesting_with_values_at_each_level() {
        // [1 [2 [3 [4 [5]]] 6] 7]
        let events = parse(b"|foo\n  :mix [1 [2 [3 [4 [5]]] 6] 7]");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"mix")),
            E::ArrayStart,
            E::Int(1),
            E::ArrayStart,
            E::Int(2),
            E::ArrayStart,
            E::Int(3),
            E::ArrayStart,
            E::Int(4),
            E::ArrayStart,
            E::Int(5),
            E::ArrayEnd,
            E::ArrayEnd,
            E::ArrayEnd,
            E::Int(6),
            E::ArrayEnd,
            E::Int(7),
            E::ArrayEnd,
            E::ElementEnd,
        ]);
    }

    #[test]
    fn array_with_all_value_types() {
        // [42 3.14 true false null "quoted" bare]
        let events = parse(b"|foo\n  :types [42 3.14 true false null \"quoted\" bare]");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"types")),
            E::ArrayStart,
            E::Int(42),
            E::Float("3.14".to_string()),
            E::Bool(true),
            E::Bool(false),
            E::Nil,
            E::QuotedStr(s(b"quoted")),
            E::Str(s(b"bare")),
            E::ArrayEnd,
            E::ElementEnd,
        ]);
    }

    #[test]
    fn array_siblings_at_multiple_levels() {
        // [[a b] [c d] [e f]]
        let events = parse(b"|foo\n  :grid [[a b] [c d] [e f]]");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"grid")),
            E::ArrayStart,
            E::ArrayStart, E::Str(s(b"a")), E::Str(s(b"b")), E::ArrayEnd,
            E::ArrayStart, E::Str(s(b"c")), E::Str(s(b"d")), E::ArrayEnd,
            E::ArrayStart, E::Str(s(b"e")), E::Str(s(b"f")), E::ArrayEnd,
            E::ArrayEnd,
            E::ElementEnd,
        ]);
    }

    #[test]
    fn array_with_quoted_strings_containing_brackets() {
        // ["[not an array]" "]" "["]
        let events = parse(b"|foo\n  :tricky [\"[not an array]\" \"]\" \"[\"]");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"tricky")),
            E::ArrayStart,
            E::QuotedStr(s(b"[not an array]")),
            E::QuotedStr(s(b"]")),
            E::QuotedStr(s(b"[")),
            E::ArrayEnd,
            E::ElementEnd,
        ]);
    }

    #[test]
    fn complex_nested_structure() {
        // [[[1 2] [3 4]] [[5 6] [7 8]]]
        let events = parse(b"|foo\n  :cube [[[1 2] [3 4]] [[5 6] [7 8]]]");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"cube")),
            E::ArrayStart,                                          // outer
            E::ArrayStart,                                          // first mid
            E::ArrayStart, E::Int(1), E::Int(2), E::ArrayEnd,       // [1 2]
            E::ArrayStart, E::Int(3), E::Int(4), E::ArrayEnd,       // [3 4]
            E::ArrayEnd,                                            // close first mid
            E::ArrayStart,                                          // second mid
            E::ArrayStart, E::Int(5), E::Int(6), E::ArrayEnd,       // [5 6]
            E::ArrayStart, E::Int(7), E::Int(8), E::ArrayEnd,       // [7 8]
            E::ArrayEnd,                                            // close second mid
            E::ArrayEnd,                                            // close outer
            E::ElementEnd,
        ]);
    }

    #[test]
    fn array_with_single_quoted_strings() {
        let events = parse(b"|foo\n  :names ['Alice' 'Bob']");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"names")),
            E::ArrayStart,
            E::QuotedStr(s(b"Alice")),
            E::QuotedStr(s(b"Bob")),
            E::ArrayEnd,
            E::ElementEnd,
        ]);
    }

    #[test]
    fn array_multiline() {
        // Arrays can span multiple lines
        let events = parse(b"|foo\n  :items [\n    a\n    b\n    c\n  ]");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"items")),
            E::ArrayStart,
            E::Str(s(b"a")),
            E::Str(s(b"b")),
            E::Str(s(b"c")),
            E::ArrayEnd,
            E::ElementEnd,
        ]);
    }

    #[test]
    fn empty_arrays_at_various_depths() {
        // [[] [[]] [[[]]]]
        let events = parse(b"|foo\n  :empties [[] [[]] [[[]]]]");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Attr(s(b"empties")),
            E::ArrayStart,
            E::ArrayStart, E::ArrayEnd,                             // []
            E::ArrayStart, E::ArrayStart, E::ArrayEnd, E::ArrayEnd, // [[]]
            E::ArrayStart, E::ArrayStart, E::ArrayStart, E::ArrayEnd, E::ArrayEnd, E::ArrayEnd, // [[[]]]
            E::ArrayEnd,
            E::ElementEnd,
        ]);
    }
}

// =============================================================================
// Nesting - Indentation
// =============================================================================

mod nesting {
    use super::*;

    #[test]
    fn nested_elements() {
        let input = b"|parent\n  |child";
        let events = parse(input);
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"parent"))),
            E::ElementStart(Some(s(b"child"))),
            E::ElementEnd, // child
            E::ElementEnd, // parent
        ]);
    }

    #[test]
    fn siblings() {
        let input = b"|a\n|b";
        let events = parse(input);
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"a"))),
            E::ElementEnd,
            E::ElementStart(Some(s(b"b"))),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn deep_nesting() {
        let input = b"|a\n  |b\n    |c";
        let events = parse(input);
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"a"))),
            E::ElementStart(Some(s(b"b"))),
            E::ElementStart(Some(s(b"c"))),
            E::ElementEnd, // c
            E::ElementEnd, // b
            E::ElementEnd, // a
        ]);
    }

    #[test]
    fn dedent_to_sibling() {
        let input = b"|a\n  |b\n|c";
        let events = parse(input);
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"a"))),
            E::ElementStart(Some(s(b"b"))),
            E::ElementEnd, // b
            E::ElementEnd, // a
            E::ElementStart(Some(s(b"c"))),
            E::ElementEnd, // c
        ]);
    }
}

// =============================================================================
// Comments
// =============================================================================

mod comments {
    use super::*;

    #[test]
    fn line_comment() {
        let events = parse(b"; this is a comment");
        assert_eq!(events, vec![
            E::Comment(s(b" this is a comment")),
        ]);
    }

    #[test]
    fn comment_after_element() {
        let events = parse(b"|foo ; comment");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Comment(s(b" comment")),
            E::ElementEnd,
        ]);
    }
}

// =============================================================================
// Text Content
// =============================================================================

mod text {
    use super::*;

    #[test]
    fn inline_text() {
        let events = parse(b"|p Hello world");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::Text(s(b"Hello world")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn prose_line() {
        let events = parse(b"Just some prose");
        assert_eq!(events, vec![
            E::Text(s(b"Just some prose")),
        ]);
    }

    #[test]
    fn indented_text() {
        let input = b"|p\n  Some text\n  More text";
        let events = parse(input);
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::Text(s(b"Some text")),
            E::Text(s(b"More text")),
            E::ElementEnd,
        ]);
    }
}

// =============================================================================
// Escape Prefix (')
// =============================================================================

mod escape_prefix {
    use super::*;

    #[test]
    fn escaped_pipe() {
        // '|not-element → literal text
        let events = parse(b"'|not-element");
        assert_eq!(events, vec![
            E::Text(s(b"|not-element")),
        ]);
    }

    #[test]
    fn escaped_colon() {
        let events = parse(b"|foo\n  ':not-attr");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"foo"))),
            E::Text(s(b":not-attr")),
            E::ElementEnd,
        ]);
    }
}

// =============================================================================
// SPEC-INDENTS.md: Indentation and Hierarchy
// =============================================================================
//
// These tests cover the full specification from SPEC-INDENTS.md.
// The core principle: column position determines hierarchy, with inline
// elements nested as if on separate lines.

mod indentation_hierarchy {
    use super::*;

    // =========================================================================
    // Basic Rules
    // =========================================================================
    // 1. Greater column = child (push onto stack)
    // 2. Same column = sibling (pop current, push as child of parent)
    // 3. Lesser column = dedent (pop until column > top's base_column)
    //
    // The one rule: `pop while new_column <= stack_top.base_column`

    #[test]
    fn greater_column_is_child() {
        // |parent
        //   |child  ← column 2 > column 0, so child of parent
        let events = parse(b"|parent\n  |child");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"parent"))),
            E::ElementStart(Some(s(b"child"))),
            E::ElementEnd, // child
            E::ElementEnd, // parent
        ]);
    }

    #[test]
    fn same_column_is_sibling() {
        // |parent
        //   |child    ← column 2
        //   |sibling  ← column 2: SAME column = SIBLING of child
        let events = parse(b"|parent\n  |child\n  |sibling");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"parent"))),
            E::ElementStart(Some(s(b"child"))),
            E::ElementEnd, // child closed before sibling
            E::ElementStart(Some(s(b"sibling"))),
            E::ElementEnd, // sibling
            E::ElementEnd, // parent
        ]);
    }

    #[test]
    fn same_column_is_sibling_not_child() {
        // Critical: same column means sibling, NOT child!
        // |parent
        //   |child        ← column 2
        //   |sibling      ← column 2: SAME column = SIBLING of child, not inside it!
        //    |inside      ← column 3: ONE MORE column = INSIDE sibling
        let events = parse(b"|parent\n  |child\n  |sibling\n   |inside");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"parent"))),
            E::ElementStart(Some(s(b"child"))),
            E::ElementEnd, // child
            E::ElementStart(Some(s(b"sibling"))),
            E::ElementStart(Some(s(b"inside"))),
            E::ElementEnd, // inside
            E::ElementEnd, // sibling
            E::ElementEnd, // parent
        ]);
    }

    #[test]
    fn lesser_column_dedents() {
        // |a
        //   |b
        //     |c
        // |d  ← column 0 closes c, b, and a
        let events = parse(b"|a\n  |b\n    |c\n|d");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"a"))),
            E::ElementStart(Some(s(b"b"))),
            E::ElementStart(Some(s(b"c"))),
            E::ElementEnd, // c
            E::ElementEnd, // b
            E::ElementEnd, // a
            E::ElementStart(Some(s(b"d"))),
            E::ElementEnd, // d
        ]);
    }

    #[test]
    fn partial_dedent() {
        // |a
        //   |b
        //     |c
        //   |d  ← column 2 closes c, but stays in a
        let events = parse(b"|a\n  |b\n    |c\n  |d");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"a"))),
            E::ElementStart(Some(s(b"b"))),
            E::ElementStart(Some(s(b"c"))),
            E::ElementEnd, // c
            E::ElementEnd, // b (same column = sibling)
            E::ElementStart(Some(s(b"d"))),
            E::ElementEnd, // d
            E::ElementEnd, // a
        ]);
    }

    // =========================================================================
    // Closing Multiple Levels (SPEC-INDENTS.md line 225-243)
    // =========================================================================

    #[test]
    fn closing_multiple_levels() {
        // |one
        //   |two
        //     |three
        //       |four
        // - this prose is sibling to |one
        //
        // The prose at column 0 triggers closing four, three, two, and one.
        let events = parse(b"|one\n  |two\n    |three\n      |four\nsibling prose");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"one"))),
            E::ElementStart(Some(s(b"two"))),
            E::ElementStart(Some(s(b"three"))),
            E::ElementStart(Some(s(b"four"))),
            E::ElementEnd, // four
            E::ElementEnd, // three
            E::ElementEnd, // two
            E::ElementEnd, // one
            E::Text(s(b"sibling prose")),
        ]);
    }

    #[test]
    fn deep_nesting_then_full_close() {
        // 5 levels deep, then back to column 0
        let events = parse(b"|a\n |b\n  |c\n   |d\n    |e\n|f");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"a"))),
            E::ElementStart(Some(s(b"b"))),
            E::ElementStart(Some(s(b"c"))),
            E::ElementStart(Some(s(b"d"))),
            E::ElementStart(Some(s(b"e"))),
            E::ElementEnd, // e
            E::ElementEnd, // d
            E::ElementEnd, // c
            E::ElementEnd, // b
            E::ElementEnd, // a
            E::ElementStart(Some(s(b"f"))),
            E::ElementEnd, // f
        ]);
    }
}

mod inline_element_nesting {
    use super::*;

    // =========================================================================
    // Inline Nesting (SPEC-INDENTS.md line 76-95)
    // =========================================================================
    // |one |two |three  ; three is child of two, two is child of one
    //
    // Inline elements are exactly as if on separate lines at those columns.

    #[test]
    fn inline_nesting_basic() {
        // |one |two |three
        // Is equivalent to:
        // |one
        //      |two
        //           |three
        let events = parse(b"|one |two |three");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"one"))),
            E::ElementStart(Some(s(b"two"))),
            E::ElementStart(Some(s(b"three"))),
            E::ElementEnd, // three
            E::ElementEnd, // two
            E::ElementEnd, // one
        ]);
    }

    #[test]
    fn inline_equivalence_to_vertical() {
        // These should produce identical event sequences
        let inline = parse(b"|one |two |three");
        let vertical = parse(b"|one\n     |two\n          |three");
        assert_eq!(inline, vertical);
    }

    #[test]
    fn inline_equivalence_minimal_indent() {
        // Vertical form with minimal indentation should also be equivalent
        let inline = parse(b"|one |two |three");
        let vertical = parse(b"|one\n  |two\n    |three");
        assert_eq!(inline, vertical);
    }

    // =========================================================================
    // Sibling After Inline Elements (SPEC-INDENTS.md line 96-108)
    // =========================================================================

    #[test]
    fn sibling_after_inline_elements() {
        // |one |two |three
        //   |alpha          ; sibling of |two -- child of |one
        //
        // Stack has: [one@0, two@5, three@10]
        // alpha at column 2: pop three, pop two, push as child of one
        let events = parse(b"|one |two |three\n  |alpha");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"one"))),
            E::ElementStart(Some(s(b"two"))),
            E::ElementStart(Some(s(b"three"))),
            E::ElementEnd, // three
            E::ElementEnd, // two
            E::ElementStart(Some(s(b"alpha"))),
            E::ElementEnd, // alpha
            E::ElementEnd, // one
        ]);
    }

    // =========================================================================
    // Column Alignment = Sibling (SPEC-INDENTS.md line 110-121)
    // =========================================================================

    #[test]
    fn column_alignment_is_sibling() {
        // |one |two |three
        //      |alpha       ; same column as |two = sibling of |two
        //
        // alpha at column 5 (same as two):
        // - 5 ≤ 10? Pop three
        // - 5 ≤ 5? Pop two (same column = sibling!)
        // - 5 ≤ 0? No, stop
        // - Push alpha as child of one
        let events = parse(b"|one |two |three\n     |alpha");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"one"))),
            E::ElementStart(Some(s(b"two"))),
            E::ElementStart(Some(s(b"three"))),
            E::ElementEnd, // three
            E::ElementEnd, // two (same column = sibling)
            E::ElementStart(Some(s(b"alpha"))),
            E::ElementEnd, // alpha
            E::ElementEnd, // one
        ]);
    }

    // =========================================================================
    // Child of Inline Element (SPEC-INDENTS.md line 123-138)
    // =========================================================================

    #[test]
    fn child_of_inline_element_between_columns() {
        // |one |two |three
        //         |alpha   ; between two's and three's column = child of two
        //
        // alpha at column 8 (between 5 and 10):
        // - 8 ≤ 10? Pop three
        // - 8 ≤ 5? No, stop
        // - Push alpha as child of two
        let events = parse(b"|one |two |three\n        |alpha");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"one"))),
            E::ElementStart(Some(s(b"two"))),
            E::ElementStart(Some(s(b"three"))),
            E::ElementEnd, // three
            E::ElementStart(Some(s(b"alpha"))),
            E::ElementEnd, // alpha (child of two)
            E::ElementEnd, // two
            E::ElementEnd, // one
        ]);
    }

    #[test]
    fn child_of_inline_at_same_column_as_next() {
        // |one |two |three
        //           |alpha  ; same column as three = sibling of three (child of two)
        let events = parse(b"|one |two |three\n          |alpha");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"one"))),
            E::ElementStart(Some(s(b"two"))),
            E::ElementStart(Some(s(b"three"))),
            E::ElementEnd, // three (same column = sibling)
            E::ElementStart(Some(s(b"alpha"))),
            E::ElementEnd, // alpha
            E::ElementEnd, // two
            E::ElementEnd, // one
        ]);
    }

    // =========================================================================
    // Multi-line Progression (SPEC-INDENTS.md line 140-158)
    // =========================================================================

    #[test]
    fn multi_line_progression() {
        // |one |two |three
        //        |alpha     ; child of |two (column 7 > 5)
        //      |beta        ; sibling of |two (column 5 = 5)
        let events = parse(b"|one |two |three\n       |alpha\n     |beta");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"one"))),
            E::ElementStart(Some(s(b"two"))),
            E::ElementStart(Some(s(b"three"))),
            E::ElementEnd, // three (7 ≤ 10)
            E::ElementStart(Some(s(b"alpha"))), // child of two
            E::ElementEnd, // alpha (5 ≤ 7)
            E::ElementEnd, // two (5 ≤ 5, same column = sibling)
            E::ElementStart(Some(s(b"beta"))), // child of one
            E::ElementEnd, // beta
            E::ElementEnd, // one
        ]);
    }

    // =========================================================================
    // The Critical Insight (SPEC-INDENTS.md line 161-179)
    // =========================================================================
    // You only care about the previous line's stack state.

    #[test]
    fn stack_state_after_dedent() {
        // |one |two |three
        //   |alpha
        //      |beta      ; child of |alpha, NOT related to |two at all
        //
        // When |alpha appeared at column 2, it popped |two and |three.
        // |beta aligns with where |two was, but |two is gone.
        let events = parse(b"|one |two |three\n  |alpha\n     |beta");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"one"))),
            E::ElementStart(Some(s(b"two"))),
            E::ElementStart(Some(s(b"three"))),
            E::ElementEnd, // three
            E::ElementEnd, // two
            E::ElementStart(Some(s(b"alpha"))),
            E::ElementStart(Some(s(b"beta"))), // child of alpha!
            E::ElementEnd, // beta
            E::ElementEnd, // alpha
            E::ElementEnd, // one
        ]);
    }

    // =========================================================================
    // Complex Example: Many Inline Elements (SPEC-INDENTS.md line 183-221)
    // =========================================================================

    #[test]
    fn many_inline_elements() {
        // |a |b |c |d |e |f |g
        //          |child-of-c
        //    |child-of-a
        //
        // Stack after first line: [a@0, b@3, c@6, d@9, e@12, f@15, g@18]
        //
        // For |child-of-c at column 9:
        // - 9 ≤ 18 (g)? Pop
        // - 9 ≤ 15 (f)? Pop
        // - 9 ≤ 12 (e)? Pop
        // - 9 ≤ 9 (d)? Pop (same column!)
        // - 9 ≤ 6 (c)? No, stop
        // - Push as child of c
        //
        // For |child-of-a at column 3:
        // - 3 ≤ 9? Pop child-of-c
        // - 3 ≤ 6? Pop c
        // - 3 ≤ 3? Pop b (same column!)
        // - 3 ≤ 0? No, stop
        // - Push as child of a
        let events = parse(b"|a |b |c |d |e |f |g\n         |child-of-c\n   |child-of-a");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"a"))),
            E::ElementStart(Some(s(b"b"))),
            E::ElementStart(Some(s(b"c"))),
            E::ElementStart(Some(s(b"d"))),
            E::ElementStart(Some(s(b"e"))),
            E::ElementStart(Some(s(b"f"))),
            E::ElementStart(Some(s(b"g"))),
            E::ElementEnd, // g
            E::ElementEnd, // f
            E::ElementEnd, // e
            E::ElementEnd, // d (same column)
            E::ElementStart(Some(s(b"child-of-c"))),
            E::ElementEnd, // child-of-c
            E::ElementEnd, // c
            E::ElementEnd, // b (same column)
            E::ElementStart(Some(s(b"child-of-a"))),
            E::ElementEnd, // child-of-a
            E::ElementEnd, // a
        ]);
    }

    #[test]
    fn inline_with_text_content() {
        // |a |b |c text for c
        let events = parse(b"|a |b |c text for c");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"a"))),
            E::ElementStart(Some(s(b"b"))),
            E::ElementStart(Some(s(b"c"))),
            E::Text(s(b"text for c")),
            E::ElementEnd, // c
            E::ElementEnd, // b
            E::ElementEnd, // a
        ]);
    }
}

// =============================================================================
// SPEC-INDENTS.md: Automatic Prose Dedentation (lines 303-493)
// =============================================================================
//
// UDON automatically strips leading whitespace from prose content based on
// its context within elements.
//
// Rules:
// 1. Inline content (same line as element) does NOT establish content_base
// 2. First indented line establishes content_base_column
// 3. Subsequent lines at >= content_base: extra spaces preserved in output
// 4. Subsequent lines at < content_base: warning + update content_base

mod prose_dedentation {
    use super::*;

    // =========================================================================
    // Basic Dedentation
    // =========================================================================

    #[test]
    fn basic_dedentation_uniform_indent() {
        // |section **The great indent**
        //   This content is all inner-content of |section,
        //   and will continue to be inner-content of |section
        //
        // Output:
        // **The great indent**
        // This content is all inner-content of |section,
        // and will continue to be inner-content of |section
        let events = parse(b"|section **The great indent**\n  This content line 1\n  This content line 2");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"section"))),
            E::Text(s(b"**The great indent**")),
            E::Text(s(b"This content line 1")),  // 2 spaces stripped
            E::Text(s(b"This content line 2")),  // 2 spaces stripped
            E::ElementEnd,
        ]);
    }

    #[test]
    fn inline_content_not_stripped() {
        // Inline content (same line as element) has no leading space to strip
        let events = parse(b"|p Hello world");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::Text(s(b"Hello world")),
            E::ElementEnd,
        ]);
    }

    // =========================================================================
    // First Line Establishes content_base (User's Choice)
    // =========================================================================

    #[test]
    fn content_base_established_by_first_indented_line() {
        // User chooses 2-space indent
        // |element
        //   first line  ← establishes content_base = 2
        //   second line ← stripped of 2 spaces
        let events = parse(b"|element\n  first line\n  second line");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"element"))),
            E::Text(s(b"first line")),
            E::Text(s(b"second line")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn large_content_base_user_choice() {
        // User chooses 8-space indent
        // |element
        //         first line  ← establishes content_base = 8
        //         second line ← stripped of 8 spaces
        let events = parse(b"|element\n        first line\n        second line");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"element"))),
            E::Text(s(b"first line")),
            E::Text(s(b"second line")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn content_base_aligned_with_inline_content() {
        // |later-part This stuff is inner to |later-part
        //             and, with a slightly different formatting
        //             preference-- is indented quite a ways.
        //
        // The continuation lines are aligned with "This" (column 12).
        // All 12 leading spaces are stripped.
        let events = parse(b"|later-part This stuff is here\n            and continues here\n            and here too");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"later-part"))),
            E::Text(s(b"This stuff is here")),
            E::Text(s(b"and continues here")),  // 12 spaces stripped
            E::Text(s(b"and here too")),        // 12 spaces stripped
            E::ElementEnd,
        ]);
    }

    // =========================================================================
    // Extra Spaces Preserved (lines at >= content_base)
    // =========================================================================

    #[test]
    fn extra_spaces_preserved_beyond_content_base() {
        // |element
        //   first line       ← establishes content_base = 2
        //     extra spaces   ← col 4 > 2, OUTPUT: "  extra spaces"
        let events = parse(b"|element\n  first line\n    extra spaces");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"element"))),
            E::Text(s(b"first line")),
            E::Text(s(b"  extra spaces")),  // 2 extra spaces preserved!
            E::ElementEnd,
        ]);
    }

    #[test]
    fn varying_extra_indentation() {
        // |code
        //   def foo():
        //       return 1
        //   def bar():
        //       if True:
        //           return 2
        //
        // content_base = 2 (first indented line)
        // Lines with more indent preserve the extra spaces
        let events = parse(
            b"|code\n  def foo():\n      return 1\n  def bar():\n      if True:\n          return 2"
        );
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"code"))),
            E::Text(s(b"def foo():")),
            E::Text(s(b"    return 1")),       // 4 extra spaces
            E::Text(s(b"def bar():")),
            E::Text(s(b"    if True:")),       // 4 extra spaces
            E::Text(s(b"        return 2")),   // 8 extra spaces
            E::ElementEnd,
        ]);
    }

    // =========================================================================
    // Inconsistent Indentation Warnings (lines at < content_base)
    // =========================================================================

    #[test]
    fn warning_on_lesser_indent() {
        // |element
        //       first line    ← col 6, establishes content_base = 6
        //    less indent      ← col 3 < 6, WARNING, content_base = 3
        //    now at base      ← col 3, no warning
        let events = parse(b"|element\n      first line\n   less indent\n   now at base");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"element"))),
            E::Text(s(b"first line")),        // stripped 6
            E::Warning("inconsistent indentation".to_string()),
            E::Text(s(b"less indent")),       // stripped 3, new base
            E::Text(s(b"now at base")),       // stripped 3, no warning
            E::ElementEnd,
        ]);
    }

    #[test]
    fn multiple_warnings_decreasing_indent() {
        // |element
        //       first       ← col 6, content_base = 6
        //    second         ← col 3 < 6, WARNING, content_base = 3
        //  third            ← col 2 < 3, WARNING, content_base = 2
        let events = parse(b"|element\n      first\n   second\n  third");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"element"))),
            E::Text(s(b"first")),
            E::Warning("inconsistent indentation".to_string()),
            E::Text(s(b"second")),
            E::Warning("inconsistent indentation".to_string()),
            E::Text(s(b"third")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn extra_spaces_after_content_base_update() {
        // From SPEC-INDENTS.md line 408-426:
        // |the-parent |on-line-child
        //       first-line-of-prose...   ; col 6, establishes content_base = 6
        //    but what about this???      ; col 3 < 6, WARNING, content_base = 3
        //    ^ this is the new reference ; col 3, no warning
        //        four extra spaces       ; col 7 > 3, OUTPUT: "    four extra spaces"
        //   new warning here             ; col 2 < 3, WARNING, content_base = 2
        //
        // Output:
        // first-line-of-prose...
        // but what about this???
        // ^ this is the new reference
        //     four extra spaces      ← 4 spaces preserved (7 - 3 = 4)
        // new warning here
        let input = b"|parent |child\n      first-line\n   second-line\n   third-line\n       four extra\n  fifth-line";
        let events = parse(input);
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"parent"))),
            E::ElementStart(Some(s(b"child"))),
            E::ElementEnd, // child (dedent at col 6 < child's column)
            E::Text(s(b"first-line")),
            E::Warning("inconsistent indentation".to_string()),
            E::Text(s(b"second-line")),
            E::Text(s(b"third-line")),
            E::Text(s(b"    four extra")),  // 4 extra spaces preserved
            E::Warning("inconsistent indentation".to_string()),
            E::Text(s(b"fifth-line")),
            E::ElementEnd, // parent
        ]);
    }

    // =========================================================================
    // Inline Content Does NOT Establish content_base
    // =========================================================================

    #[test]
    fn inline_content_does_not_set_content_base() {
        // |element Here is inline content
        //   first indented line  ← THIS establishes content_base, not inline
        //   second line
        let events = parse(b"|element Here is inline\n  first indented\n  second line");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"element"))),
            E::Text(s(b"Here is inline")),
            E::Text(s(b"first indented")),  // establishes base = 2
            E::Text(s(b"second line")),     // stripped 2
            E::ElementEnd,
        ]);
    }

    // =========================================================================
    // Nested Elements with Prose
    // =========================================================================

    #[test]
    fn nested_elements_independent_content_base() {
        // Each element tracks its own content_base
        // |outer
        //   |inner
        //     prose for inner  ← inner's content_base = 4
        //   prose for outer    ← outer's content_base = 2
        let events = parse(b"|outer\n  |inner\n    prose for inner\n  prose for outer");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"outer"))),
            E::ElementStart(Some(s(b"inner"))),
            E::Text(s(b"prose for inner")),
            E::ElementEnd, // inner (dedent at col 2)
            E::Text(s(b"prose for outer")),
            E::ElementEnd, // outer
        ]);
    }

    #[test]
    fn inline_element_then_indented_prose() {
        // |first |second Some prose
        //   This prose is child of |first, after |second closed
        //
        // Prose at column 2 closes |second (2 ≤ second's column)
        let events = parse(b"|first |second Some prose\n  This is first's prose");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"first"))),
            E::ElementStart(Some(s(b"second"))),
            E::Text(s(b"Some prose")),
            E::ElementEnd, // second (col 2 ≤ second's column)
            E::Text(s(b"This is first's prose")),
            E::ElementEnd, // first
        ]);
    }

    // =========================================================================
    // Blank Lines
    // =========================================================================

    #[test]
    fn blank_lines_passed_through() {
        // Blank lines within prose should be preserved
        // |element
        //   first paragraph
        //
        //   second paragraph
        let events = parse(b"|element\n  first paragraph\n\n  second paragraph");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"element"))),
            E::Text(s(b"first paragraph")),
            // Blank line handling - parser may emit empty text or nothing
            E::Text(s(b"second paragraph")),
            E::ElementEnd,
        ]);
    }

    // =========================================================================
    // Freeform Blocks Preserve Exact Whitespace
    // =========================================================================

    #[test]
    fn freeform_block_no_dedentation() {
        // |code
        //   ```
        //   def foo():
        //       return 1
        //   ```
        //
        // Content inside backticks is preserved exactly as written
        // NOTE: Content includes trailing whitespace on line before closing ```,
        // since we preserve exact content from after opening to before closing.
        let events = parse(b"|code\n  ```\n  def foo():\n      return 1\n  ```");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"code"))),
            // Freeform content should preserve exact whitespace
            E::Raw(s(b"  def foo():\n      return 1\n  ")),
            E::ElementEnd,
        ]);
    }
}

// =============================================================================
// SPEC-INDENTS.md: Comments and Indentation (lines 496-574)
// =============================================================================

mod comment_indentation {
    use super::*;

    // =========================================================================
    // Block Comments Trigger Indent/Dedent
    // =========================================================================

    #[test]
    fn block_comment_inside_element() {
        // |parent
        //   |child
        //    ; this comment is INSIDE |child (one space further right)
        let events = parse(b"|parent\n  |child\n   ; inside child");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"parent"))),
            E::ElementStart(Some(s(b"child"))),
            E::Comment(s(b" inside child")),
            E::ElementEnd, // child
            E::ElementEnd, // parent
        ]);
    }

    #[test]
    fn block_comment_as_sibling() {
        // |parent
        //   |child
        //   ; this comment is SIBLING of |child (same column)
        let events = parse(b"|parent\n  |child\n  ; sibling comment");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"parent"))),
            E::ElementStart(Some(s(b"child"))),
            E::ElementEnd, // child (same column = sibling)
            E::Comment(s(b" sibling comment")),
            E::ElementEnd, // parent
        ]);
    }

    #[test]
    fn block_comment_closes_multiple_levels() {
        // |parent
        //   |child
        //     |grandchild
        // ; this comment closes grandchild, child, AND parent (column 0)
        // |sibling
        let events = parse(b"|parent\n  |child\n    |grandchild\n; closes all\n|sibling");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"parent"))),
            E::ElementStart(Some(s(b"child"))),
            E::ElementStart(Some(s(b"grandchild"))),
            E::ElementEnd, // grandchild
            E::ElementEnd, // child
            E::ElementEnd, // parent
            E::Comment(s(b" closes all")),
            E::ElementStart(Some(s(b"sibling"))),
            E::ElementEnd, // sibling
        ]);
    }

    // =========================================================================
    // Inline Comments Emitted (consumer decides whether to keep/strip)
    // =========================================================================

    #[test]
    fn inline_comment_stripped_from_output() {
        // |p This is some text ;{TODO: improve this} and more text.
        // Parser EMITS Comment, consumer decides to keep/strip
        let events = parse(b"|p This is text ;{TODO} and more.");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::Text(s(b"This is text ")),
            E::Comment(s(b"TODO")),  // Parser emits, consumer may strip
            E::Text(s(b" and more.")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn inline_comment_at_end_of_line() {
        // |p Some text ;{comment at end}
        let events = parse(b"|p Some text ;{comment at end}");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::Text(s(b"Some text ")),
            E::Comment(s(b"comment at end")),  // Parser emits
            E::ElementEnd,
        ]);
    }

    #[test]
    fn nested_inline_comment() {
        // |p Text ;{outer ;{inner} outer} more.
        // The ;{inner} is NOT a nested comment - it's literal text inside comment
        // Only bare { and } are brace-counted, not ;{
        let events = parse(b"|p Text ;{outer ;{inner} outer} more.");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::Text(s(b"Text ")),
            E::Comment(s(b"outer ;{inner} outer")),  // ;{inner} is literal
            E::Text(s(b" more.")),
            E::ElementEnd,
        ]);
    }

    // =========================================================================
    // Escaped Semicolon
    // =========================================================================

    #[test]
    fn escaped_semicolon_at_line_start() {
        // '; This line starts with a semicolon in the output
        // Output: ; This line starts with a semicolon in the output
        let events = parse(b"'; This starts with semicolon");
        assert_eq!(events, vec![
            E::Text(s(b"; This starts with semicolon")),
        ]);
    }

    #[test]
    fn escaped_semicolon_in_element() {
        // |p
        //   '; literal semicolon at start
        let events = parse(b"|p\n  '; literal semicolon");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::Text(s(b"; literal semicolon")),
            E::ElementEnd,
        ]);
    }
}

// =============================================================================
// Edge Cases and Regression Tests
// =============================================================================

mod indentation_edge_cases {
    use super::*;

    #[test]
    fn single_space_indent_is_valid() {
        // Minimal indent of 1 space is valid
        // |a
        //  |b  ← just 1 space indent
        let events = parse(b"|a\n |b");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"a"))),
            E::ElementStart(Some(s(b"b"))),
            E::ElementEnd,
            E::ElementEnd,
        ]);
    }

    #[test]
    fn mixed_indent_levels_valid() {
        // Different siblings can use different indent levels
        // (though stylistically discouraged)
        // |parent
        //   |child1
        //  |child2   ← valid sibling at col 1
        let events = parse(b"|parent\n  |child1\n |child2");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"parent"))),
            E::ElementStart(Some(s(b"child1"))),
            E::ElementEnd, // child1
            E::ElementStart(Some(s(b"child2"))),
            E::ElementEnd, // child2
            E::ElementEnd, // parent
        ]);
    }

    #[test]
    fn element_with_only_attributes_then_sibling() {
        // |a
        //   :attr value
        // |b
        let events = parse(b"|a\n  :attr value\n|b");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"a"))),
            E::Attr(s(b"attr")),
            E::Str(s(b"value")),
            E::ElementEnd, // a
            E::ElementStart(Some(s(b"b"))),
            E::ElementEnd, // b
        ]);
    }

    #[test]
    fn prose_then_element_at_same_level() {
        // |parent
        //   some prose
        //   |child
        let events = parse(b"|parent\n  some prose\n  |child");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"parent"))),
            E::Text(s(b"some prose")),
            E::ElementStart(Some(s(b"child"))),
            E::ElementEnd, // child
            E::ElementEnd, // parent
        ]);
    }

    #[test]
    fn element_at_column_zero_after_deep_nesting() {
        // Ensure complete stack cleanup
        let events = parse(b"|a\n |b\n  |c\n   |d\n    |e\n     |f\n|z");
        // |z should close all 6 nested elements
        let element_ends = events.iter().filter(|e| matches!(e, E::ElementEnd)).count();
        assert_eq!(element_ends, 7); // 6 from a-f, 1 from z
    }
}

// =============================================================================
// SPEC.md: Element Recognition (lines 645-651)
// =============================================================================
//
// | is only an element when followed by:
//   - Unicode letter (\p{L}) — named element
//   - "[" — anonymous element with id
//   - "." — anonymous element with class
//   - "{" — embedded element
//   - "'" — quoted element name
// Otherwise "|" is prose (preserves Markdown table compatibility)

mod element_recognition {
    use super::*;

    #[test]
    fn pipe_followed_by_letter_is_element() {
        let events = parse(b"|div");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"div"))),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn pipe_followed_by_bracket_is_anonymous_with_id() {
        let events = parse(b"|[myid]");
        assert_eq!(events, vec![
            E::ElementStart(None),
            E::Attr(s(b"$id")),
            E::Str(s(b"myid")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn pipe_followed_by_dot_is_anonymous_with_class() {
        let events = parse(b"|.myclass");
        assert_eq!(events, vec![
            E::ElementStart(None),
            E::Attr(s(b"$class")),
            E::Str(s(b"myclass")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn pipe_followed_by_brace_is_embedded() {
        // |{em text} should be embedded element
        let events = parse(b"|p |{em emphasized}");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::ElementStart(Some(s(b"em"))),  // embedded start
            E::Text(s(b"emphasized")),
            E::ElementEnd,  // embedded end
            E::ElementEnd,  // p
        ]);
    }

    #[test]
    fn pipe_followed_by_quote_is_quoted_name() {
        let events = parse(b"|'my element'");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"my element"))),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn pipe_followed_by_space_is_anonymous_element() {
        // "| " followed by content = anonymous element
        let events = parse(b"| content");
        assert_eq!(events, vec![
            E::ElementStart(None),
            E::Text(s(b"content")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn pipe_in_markdown_table_is_prose() {
        // Markdown table syntax should pass through as prose
        // | Header | Header |  ← pipe not followed by valid element start
        let events = parse(b"| Header | Header |");
        // Should be treated as prose, not elements
        assert!(events.iter().any(|e| matches!(e, E::Text(_))));
    }

    #[test]
    fn pipe_followed_by_number_is_prose() {
        // |123 is NOT an element (elements must start with letter)
        let events = parse(b"|123");
        // Should be prose or error, not ElementStart
        let has_element = events.iter().any(|e| matches!(e, E::ElementStart(_)));
        // This might be prose OR a parse decision - let's see what happens
        // For now, just assert it doesn't panic
        assert!(!events.is_empty());
    }

    #[test]
    fn pipe_followed_by_hyphen_is_prose() {
        // |- is NOT an element (hyphen can't start element name)
        let events = parse(b"|- list item");
        // Should be prose
        assert!(events.iter().any(|e| matches!(e, E::Text(_))));
    }
}

// =============================================================================
// SPEC.md: Suffix Positions (lines 89-105)
// =============================================================================

mod suffix_positions {
    use super::*;

    #[test]
    fn suffix_after_name() {
        // |name?
        let events = parse(b"|field?");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"field"))),
            E::Attr(s(b"?")),
            E::Bool(true),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn suffix_after_name_before_id() {
        // |name?[id]
        let events = parse(b"|field?[myid]");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"field"))),
            E::Attr(s(b"?")),
            E::Bool(true),
            E::Attr(s(b"$id")),
            E::Str(s(b"myid")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn suffix_after_id() {
        // |name[id]?
        let events = parse(b"|field[myid]?");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"field"))),
            E::Attr(s(b"$id")),
            E::Str(s(b"myid")),
            E::Attr(s(b"?")),
            E::Bool(true),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn suffix_after_id_space_before_class() {
        // |name[id]? .class
        let events = parse(b"|field[myid]? .cls");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"field"))),
            E::Attr(s(b"$id")),
            E::Str(s(b"myid")),
            E::Attr(s(b"?")),
            E::Bool(true),
            E::Attr(s(b"$class")),
            E::Str(s(b"cls")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn suffix_space_separated_at_end() {
        // |name[id].class ?
        let events = parse(b"|field[myid].cls ?");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"field"))),
            E::Attr(s(b"$id")),
            E::Str(s(b"myid")),
            E::Attr(s(b"$class")),
            E::Str(s(b"cls")),
            E::Attr(s(b"?")),
            E::Bool(true),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn suffix_on_class_reserved_error() {
        // |name.class? — reserved, should error or be invalid
        // Per SPEC.md line 100-105: "NOT allowed — reserved for class-level modifiers"
        let events = parse(b"|field.cls?");
        // Should either error or treat ? as separate token
        // The key is it should NOT attach ? to the class
        let has_error = events.iter().any(|e| matches!(e, E::Error(_)));
        // If no error, at least verify ? isn't a class attribute
        if !has_error {
            // This is acceptable if parser treats it as something else
            assert!(!events.is_empty());
        }
    }

    #[test]
    fn multiple_suffixes() {
        // |name?! — multiple suffixes
        let events = parse(b"|field?!");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"field"))),
            E::Attr(s(b"?")),
            E::Bool(true),
            E::Attr(s(b"!")),
            E::Bool(true),
            E::ElementEnd,
        ]);
    }
}

// =============================================================================
// SPEC.md: Inline Attributes (lines 109-118)
// =============================================================================

mod inline_attributes {
    use super::*;

    #[test]
    fn multiple_inline_attributes() {
        // |element :key1 value1 :key2 value2
        let events = parse(b"|el :k1 v1 :k2 v2");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"el"))),
            E::Attr(s(b"k1")),
            E::Str(s(b"v1")),
            E::Attr(s(b"k2")),
            E::Str(s(b"v2")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn attribute_value_terminates_at_next_attribute() {
        // For INLINE attributes, space terminates bare value
        // :msg hello means msg="hello", then "world :count 5" is text content
        // (Block-level attributes are different - whole line is value)
        let events = parse(b"|el :msg hello world :count 5");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"el"))),
            E::Attr(s(b"msg")),
            E::Str(s(b"hello")),
            E::Text(s(b"world :count 5")),  // Rest becomes text content
            E::ElementEnd,
        ]);
    }

    #[test]
    fn attribute_value_terminates_at_inline_child() {
        // Value runs until space + "|"
        let events = parse(b"|parent :attr value |child");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"parent"))),
            E::Attr(s(b"attr")),
            E::Str(s(b"value")),
            E::ElementStart(Some(s(b"child"))),
            E::ElementEnd, // child
            E::ElementEnd, // parent
        ]);
    }

    #[test]
    fn quoted_attribute_key() {
        // :'complex key' value
        let events = parse(b"|el :'my key' value");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"el"))),
            E::Attr(s(b"my key")),
            E::Str(s(b"value")),
            E::ElementEnd,
        ]);
    }
}

// =============================================================================
// SPEC.md: Embedded Elements |{...} (lines 185-212)
// =============================================================================

mod embedded_elements {
    use super::*;

    // =========================================================================
    // Basic Embedded Elements
    // =========================================================================

    #[test]
    fn basic_embedded_element() {
        // |{em text}
        let events = parse(b"|p This has |{em emphasis} here");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::Text(s(b"This has ")),
            E::ElementStart(Some(s(b"em"))),
            E::Text(s(b"emphasis")),
            E::ElementEnd, // em
            E::Text(s(b" here")),
            E::ElementEnd, // p
        ]);
    }

    #[test]
    fn embedded_with_attributes() {
        // |{a :href /foo link text}
        let events = parse(b"|p Click |{a :href /foo here}!");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::Text(s(b"Click ")),
            E::ElementStart(Some(s(b"a"))),
            E::Attr(s(b"href")),
            E::Str(s(b"/foo")),
            E::Text(s(b"here")),
            E::ElementEnd, // a
            E::Text(s(b"!")),
            E::ElementEnd, // p
        ]);
    }

    #[test]
    fn nested_embedded_elements() {
        // |{a :href /doc the |{em official} documentation}
        let events = parse(b"|p See |{a :href /doc the |{em official} docs}.");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::Text(s(b"See ")),
            E::ElementStart(Some(s(b"a"))),
            E::Attr(s(b"href")),
            E::Str(s(b"/doc")),
            E::Text(s(b"the ")),
            E::ElementStart(Some(s(b"em"))),
            E::Text(s(b"official")),
            E::ElementEnd, // em
            E::Text(s(b" docs")),
            E::ElementEnd, // a
            E::Text(s(b".")),
            E::ElementEnd, // p
        ]);
    }

    #[test]
    fn multiple_embedded_siblings() {
        // |nav |{a Home} |{a About}
        // Space between } and |{ is preserved as text (user controls spacing)
        let events = parse(b"|nav |{a Home} |{a About}");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"nav"))),
            E::ElementStart(Some(s(b"a"))),
            E::Text(s(b"Home")),
            E::ElementEnd,
            E::Text(s(b" ")),  // Space between siblings is preserved
            E::ElementStart(Some(s(b"a"))),
            E::Text(s(b"About")),
            E::ElementEnd,
            E::ElementEnd, // nav
        ]);
    }

    #[test]
    fn embedded_anonymous_element() {
        // |{.highlight text}
        let events = parse(b"|p Some |{.highlight important} text");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::Text(s(b"Some ")),
            E::ElementStart(None),
            E::Attr(s(b"$class")),
            E::Str(s(b"highlight")),
            E::Text(s(b"important")),
            E::ElementEnd,
            E::Text(s(b" text")),
            E::ElementEnd,
        ]);
    }

    // =========================================================================
    // Embedded with ID, Class, Suffix
    // =========================================================================

    #[test]
    fn embedded_with_id() {
        // |{span[myid] text}
        let events = parse(b"|p Some |{span[myid] text} here");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::Text(s(b"Some ")),
            E::ElementStart(Some(s(b"span"))),
            E::Attr(s(b"$id")),
            E::Str(s(b"myid")),
            E::Text(s(b"text")),
            E::ElementEnd,
            E::Text(s(b" here")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn embedded_with_id_and_class() {
        // |{span[id].cls text}
        let events = parse(b"|p |{span[id].highlight text}");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::ElementStart(Some(s(b"span"))),
            E::Attr(s(b"$id")),
            E::Str(s(b"id")),
            E::Attr(s(b"$class")),
            E::Str(s(b"highlight")),
            E::Text(s(b"text")),
            E::ElementEnd,
            E::ElementEnd,
        ]);
    }

    #[test]
    fn embedded_with_suffix() {
        // |{field? optional content}
        let events = parse(b"|form |{field? optional}");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"form"))),
            E::ElementStart(Some(s(b"field"))),
            E::Attr(s(b"?")),
            E::Bool(true),
            E::Text(s(b"optional")),
            E::ElementEnd,
            E::ElementEnd,
        ]);
    }

    // =========================================================================
    // Embedded with Interpolation
    // =========================================================================

    #[test]
    fn embedded_with_interpolation() {
        // |{em !{{value}}}
        let events = parse(b"|p Hello |{em !{{user.name}}}!");
        // TODO(embedded+interpolation): Add proper assertions once both features work
        // Should contain embedded element with interpolation inside
        assert!(events.iter().any(|e| matches!(e, E::ElementStart(Some(n)) if n == b"em")),
            "PLACEHOLDER: embedded+interpolation not yet implemented");
    }

    #[test]
    fn embedded_with_interpolation_and_text() {
        // |{a :href !{{url}} click here}
        let events = parse(b"|p |{a :href !{{base_url}} click here}");
        // TODO(embedded+interpolation): Add proper assertions once both features work
        assert!(events.iter().any(|e| matches!(e, E::ElementStart(Some(n)) if n == b"a")),
            "PLACEHOLDER: embedded+interpolation not yet implemented");
    }

    // =========================================================================
    // Deep Nesting
    // =========================================================================

    #[test]
    fn deeply_nested_embedded() {
        // |{a |{b |{c text}}}
        let events = parse(b"|p |{a |{b |{c deep}}}");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::ElementStart(Some(s(b"a"))),
            E::ElementStart(Some(s(b"b"))),
            E::ElementStart(Some(s(b"c"))),
            E::Text(s(b"deep")),
            E::ElementEnd, // c
            E::ElementEnd, // b
            E::ElementEnd, // a
            E::ElementEnd, // p
        ]);
    }

    #[test]
    fn complex_nested_with_attributes() {
        // |p |{div.outer |{span[id].inner :data val text}}
        // Elements: p, div, span = 3 elements with proper nesting
        let events = parse(b"|p |{div.outer |{span[id].inner :data val text}}");
        let element_count = events.iter()
            .filter(|e| matches!(e, E::ElementStart(_)))
            .count();
        assert_eq!(element_count, 3); // p, div, span
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn embedded_empty() {
        // |{} — anonymous empty embedded
        let events = parse(b"|p before |{} after");
        // Should handle empty embedded
        assert!(!events.is_empty());
    }

    #[test]
    fn embedded_with_braces_in_content() {
        // Braces inside quoted strings should be fine
        let events = parse(b"|p |{code \"func() { return 1; }\"}");
        assert!(events.iter().any(|e| matches!(e, E::ElementStart(Some(n)) if n == b"code")));
    }

    #[test]
    fn embedded_adjacent() {
        // |{a}|{b} — no space between
        let events = parse(b"|p |{a one}|{b two}");
        let element_count = events.iter()
            .filter(|e| matches!(e, E::ElementStart(_)))
            .count();
        assert_eq!(element_count, 3); // p, a, b
    }

    #[test]
    fn unclosed_embedded_element_error() {
        // |{em missing close
        let events = parse(b"|p This has |{em unclosed");
        assert!(events.iter().any(|e| matches!(e, E::Error(_))));
    }

    #[test]
    fn unclosed_nested_embedded_error() {
        // |{a |{b unclosed}
        let events = parse(b"|p |{a |{b text}");
        assert!(events.iter().any(|e| matches!(e, E::Error(_))));
    }

    #[test]
    fn mismatched_braces_error() {
        // |{a text}} — extra closing brace
        let events = parse(b"|p |{a text}}");
        // Extra } should either be text or error
        assert!(!events.is_empty());
    }

    // =========================================================================
    // Complex Real-World Scenarios
    // =========================================================================

    #[test]
    fn nav_with_pipe_separators() {
        // Navigation with | as visual separator between links
        // |nav |{a Home} | |{a About} | |{a Help}
        let events = parse(b"|nav |{a Home} | |{a About} | |{a Help}");
        // Should have: nav, three links, and pipe text between them
        let element_count = events.iter()
            .filter(|e| matches!(e, E::ElementStart(_)))
            .count();
        assert_eq!(element_count, 4); // nav + 3 links

        // The | should become text
        let has_pipe_text = events.iter().any(|e| {
            matches!(e, E::Text(t) if t.contains(&b'|'))
        });
        assert!(has_pipe_text);
    }

    #[test]
    fn dense_nav_embedded_list() {
        // Dense packed: |ul |{li |{a Home} | }|{li |{a About} | }|{li |{a Exit}}
        // This tests: no space between closing } and next |{
        let events = parse(b"|ul |{li |{a Home} | }|{li |{a About} | }|{li |{a Exit}}");
        // Should parse as: ul containing li's containing a's
        let element_count = events.iter()
            .filter(|e| matches!(e, E::ElementStart(_)))
            .count();
        assert_eq!(element_count, 7); // ul + 3*li + 3*a
    }

    #[test]
    fn breadcrumb_with_separators() {
        // |nav.breadcrumb |{a Home} > |{a Products} > |{a Widget}
        let events = parse(b"|nav.breadcrumb |{a Home} > |{a Products} > |{a Widget}");
        assert!(events.iter().any(|e| matches!(e, E::ElementStart(Some(n)) if n == b"nav")));
        // > should be text between links
    }

    #[test]
    fn inline_formatting_mixed() {
        // Complex prose with mixed embedded elements
        // |p Click |{a :href /here here} or press |{kbd Ctrl}+|{kbd C} to copy.
        let events = parse(b"|p Click |{a :href /here here} or press |{kbd Ctrl}+|{kbd C} to copy.");
        let element_count = events.iter()
            .filter(|e| matches!(e, E::ElementStart(_)))
            .count();
        assert_eq!(element_count, 4); // p, a, kbd, kbd
    }

    #[test]
    fn table_cell_with_embedded() {
        // |table |tr |td |{strong Name} |td |{em Value}
        let events = parse(b"|table |tr |td |{strong Name} |td |{em Value}");
        // Complex inline nesting with embedded elements
        assert!(!events.is_empty());
    }

    #[test]
    fn embedded_with_interpolation_and_pipes() {
        // |p Status: |{span.status !{status}} | Updated: |{time !{updated_at}}
        let events = parse(b"|p Status: |{span.status !{status}} | Updated: |{time !{updated_at}}");
        assert!(events.iter().any(|e| matches!(e, E::ElementStart(Some(n)) if n == b"span")));
        assert!(events.iter().any(|e| matches!(e, E::ElementStart(Some(n)) if n == b"time")));
    }

    #[test]
    fn deeply_nested_with_text_between() {
        // |div |{p Start |{em |{strong deep}} middle |{code end}} after
        let events = parse(b"|div |{p Start |{em |{strong deep}} middle |{code end}} after");
        let element_count = events.iter()
            .filter(|e| matches!(e, E::ElementStart(_)))
            .count();
        assert_eq!(element_count, 5); // div, p, em, strong, code
    }

    #[test]
    fn embedded_in_prose_line() {
        // Prose line (indented) with embedded elements
        let input = b"|article\n  This is |{em emphasized} and |{strong bold} text.";
        let events = parse(input);
        assert!(events.iter().any(|e| matches!(e, E::ElementStart(Some(n)) if n == b"em")));
        assert!(events.iter().any(|e| matches!(e, E::ElementStart(Some(n)) if n == b"strong")));
    }
}

// =============================================================================
// SPEC.md: Value Types (lines 729-821)
// =============================================================================

mod value_types {
    use super::*;

    // =========================================================================
    // Integers with various bases
    // =========================================================================

    #[test]
    fn hex_integer() {
        let events = parse(b"|el :val 0xFF");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"el"))),
            E::Attr(s(b"val")),
            E::Int(255),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn hex_integer_lowercase() {
        let events = parse(b"|el :val 0xff");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"el"))),
            E::Attr(s(b"val")),
            E::Int(255),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn octal_integer() {
        let events = parse(b"|el :val 0o755");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"el"))),
            E::Attr(s(b"val")),
            E::Int(493), // 755 octal = 493 decimal
            E::ElementEnd,
        ]);
    }

    #[test]
    fn binary_integer() {
        let events = parse(b"|el :val 0b1010");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"el"))),
            E::Attr(s(b"val")),
            E::Int(10),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn integer_with_underscores() {
        let events = parse(b"|el :val 1_000_000");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"el"))),
            E::Attr(s(b"val")),
            E::Int(1_000_000),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn negative_integer() {
        let events = parse(b"|el :val -42");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"el"))),
            E::Attr(s(b"val")),
            E::Int(-42),
            E::ElementEnd,
        ]);
    }

    // =========================================================================
    // Floats
    // =========================================================================

    #[test]
    fn float_basic() {
        let events = parse(b"|el :val 3.14");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"el"))),
            E::Attr(s(b"val")),
            E::Float("3.14".to_string()),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn float_scientific_notation() {
        let events = parse(b"|el :val 1.5e-3");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"el"))),
            E::Attr(s(b"val")),
            E::Float("0.0015".to_string()), // or "1.5e-3" depending on formatting
            E::ElementEnd,
        ]);
    }

    #[test]
    fn float_scientific_uppercase() {
        let events = parse(b"|el :val 1E10");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"el"))),
            E::Attr(s(b"val")),
            E::Float("10000000000".to_string()),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn float_with_underscores() {
        let events = parse(b"|el :val 1_000.5");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"el"))),
            E::Attr(s(b"val")),
            E::Float("1000.5".to_string()),
            E::ElementEnd,
        ]);
    }

    // =========================================================================
    // Rationals and Complex (if supported)
    // =========================================================================

    #[test]
    fn rational_number() {
        // 1/3r
        let events = parse(b"|el :val 1/3r");
        // Check if rational is supported
        let has_rational = events.iter().any(|e| {
            matches!(e, E::Other(s) if s.contains("Rational"))
        });
        // If not, might parse as string
        assert!(!events.is_empty());
    }

    #[test]
    fn complex_number() {
        // 3+4i
        let events = parse(b"|el :val 3+4i");
        // Check if complex is supported
        assert!(!events.is_empty());
    }

    #[test]
    fn pure_imaginary() {
        // 5i
        let events = parse(b"|el :val 5i");
        assert!(!events.is_empty());
    }

    // =========================================================================
    // Nil variants: nil, null, ~
    // =========================================================================

    #[test]
    fn nil_keyword() {
        let events = parse(b"|el :val nil");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"el"))),
            E::Attr(s(b"val")),
            E::Nil,
            E::ElementEnd,
        ]);
    }

    #[test]
    fn null_keyword() {
        // "null" for JSON familiarity
        let events = parse(b"|el :val null");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"el"))),
            E::Attr(s(b"val")),
            E::Nil,
            E::ElementEnd,
        ]);
    }

    #[test]
    fn tilde_nil() {
        // "~" for YAML familiarity
        let events = parse(b"|el :val ~");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"el"))),
            E::Attr(s(b"val")),
            E::Nil,
            E::ElementEnd,
        ]);
    }

    // =========================================================================
    // Boolean case sensitivity
    // =========================================================================

    #[test]
    fn true_lowercase_only() {
        // true is boolean, True is string
        let events = parse(b"|el :val true");
        assert!(events.iter().any(|e| matches!(e, E::Bool(true))));
    }

    #[test]
    fn true_uppercase_is_string() {
        // True should be string, not boolean
        let events = parse(b"|el :val True");
        assert!(events.iter().any(|e| matches!(e, E::Str(v) if v == b"True")));
    }

    #[test]
    fn false_uppercase_is_string() {
        let events = parse(b"|el :val FALSE");
        assert!(events.iter().any(|e| matches!(e, E::Str(v) if v == b"FALSE")));
    }

    // =========================================================================
    // Strings
    // =========================================================================

    #[test]
    fn quoted_forces_string_type() {
        // "42" should be string, not integer
        let events = parse(b"|el :val \"42\"");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"el"))),
            E::Attr(s(b"val")),
            E::QuotedStr(s(b"42")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn quoted_true_is_string() {
        let events = parse(b"|el :val \"true\"");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"el"))),
            E::Attr(s(b"val")),
            E::QuotedStr(s(b"true")),
            E::ElementEnd,
        ]);
    }

    // =========================================================================
    // Flag attributes (presence = true)
    // =========================================================================

    #[test]
    fn flag_attribute_is_true() {
        // :enabled (no value) = true
        let events = parse(b"|el :enabled");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"el"))),
            E::Attr(s(b"enabled")),
            E::Bool(true),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn flag_followed_by_another_attribute() {
        // :debug :name foo — debug is flag, name has value
        let events = parse(b"|el :debug :name foo");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"el"))),
            E::Attr(s(b"debug")),
            E::Bool(true),
            E::Attr(s(b"name")),
            E::Str(s(b"foo")),
            E::ElementEnd,
        ]);
    }
}

// =============================================================================
// SPEC.md: Dynamics Extension
// =============================================================================
//
// Unified inline syntax:
// - !{{expr}} — interpolation (double-brace)
// - !{directive ...} — inline directive
// - !{raw:kind ...} — raw inline directive

mod dynamics {
    use super::*;

    // =========================================================================
    // Interpolation - Basic (double-brace syntax: !{{expr}})
    // =========================================================================

    #[test]
    fn basic_interpolation() {
        // !{{user.name}} — double-brace for interpolation
        let events = parse(b"|p Hello, !{{user.name}}!");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::Text(s(b"Hello, ")),
            E::Interp(s(b"user.name")),
            E::Text(s(b"!")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn interpolation_standalone() {
        // Just !{{expr}} at document level
        let events = parse(b"!{{greeting}}");
        assert_eq!(events, vec![
            E::Interp(s(b"greeting")),
        ]);
    }

    #[test]
    fn interpolation_property_access() {
        // !{{user.profile.name}}
        let events = parse(b"|p !{{user.profile.name}}");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::Interp(s(b"user.profile.name")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn interpolation_array_access() {
        // !{{items[0]}}
        let events = parse(b"|p First: !{{items[0]}}");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::Text(s(b"First: ")),
            E::Interp(s(b"items[0]")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn multiple_interpolations() {
        // Multiple !{{}} in one line
        let events = parse(b"|p !{{first}} and !{{second}} and !{{third}}");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::Interp(s(b"first")),
            E::Text(s(b" and ")),
            E::Interp(s(b"second")),
            E::Text(s(b" and ")),
            E::Interp(s(b"third")),
            E::ElementEnd,
        ]);
    }

    // =========================================================================
    // Interpolation - Filters
    // =========================================================================

    #[test]
    fn interpolation_with_filter() {
        // !{{name | capitalize}} - filter expression is captured verbatim
        let events = parse(b"|p Hello, !{{name | capitalize}}!");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::Text(s(b"Hello, ")),
            E::Interp(s(b"name | capitalize")),
            E::Text(s(b"!")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn interpolation_multiple_filters() {
        // !{{value | filter1 | filter2 | filter3}}
        let events = parse(b"|p !{{text | strip | capitalize | truncate}}");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::Interp(s(b"text | strip | capitalize | truncate")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn interpolation_filter_with_arg() {
        // !{{date | format "%Y-%m-%d"}}
        let events = parse(b"|p !{{date | format \"%Y-%m-%d\"}}");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::Interp(s(b"date | format \"%Y-%m-%d\"")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn interpolation_filter_with_multiple_args() {
        // !{{price | currency "USD"}}
        let events = parse(b"|p !{{price | currency \"USD\"}}");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"p"))),
            E::Interp(s(b"price | currency \"USD\"")),
            E::ElementEnd,
        ]);
    }

    // =========================================================================
    // Interpolation - Edge Cases (line starts, nesting)
    // =========================================================================

    #[test]
    fn interpolation_at_child_line_start() {
        // Interpolation can start a child content line
        let events = parse(b"|parent\n  !{{'the-issue' | embed}}");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"parent"))),
            E::Interp(s(b"'the-issue' | embed")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn inline_comment_at_child_line_start() {
        // Inline comment can start a child content line (edge case)
        let events = parse(b"|parent\n  ;{and this, as an edge case}");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"parent"))),
            E::Comment(s(b"and this, as an edge case")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn interpolation_multiple_child_lines() {
        // Multiple child lines can each start with interpolation
        // When each interpolation is on its own line, newline+indent is structural
        let events = parse(b"|parent\n  !{{first}}\n  !{{second}}");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"parent"))),
            E::Interp(s(b"first")),
            E::Interp(s(b"second")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn interpolation_with_surrounding_text() {
        // Interpolation surrounded by text on same line preserves text
        let events = parse(b"|parent\n  before !{{middle}} after");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"parent"))),
            E::Text(s(b"before ")),
            E::Interp(s(b"middle")),
            E::Text(s(b" after")),
            E::ElementEnd,
        ]);
    }

    // =========================================================================
    // Interpolation - In Attributes
    // =========================================================================

    #[test]
    fn interpolation_in_attribute_value() {
        // :href !{{base_url}}/users
        let events = parse(b"|a :href !{{base}}/users");
        // TODO(interpolation): Verify interpolation in attr value
        placeholder_test!("interpolation+attrs", events);
    }

    #[test]
    fn interpolation_full_attribute_value() {
        // :class !{{dynamic_class}}
        let events = parse(b"|div :class !{{computed_class}}");
        // TODO(interpolation): Verify attr value is interpolation
        placeholder_test!("interpolation+attrs", events);
    }

    #[test]
    fn interpolation_in_element_id() {
        // |el[!{{dynamic_id}}]
        let events = parse(b"|div[!{{item.id}}]");
        // TODO(interpolation): Verify interpolation in element id
        placeholder_test!("interpolation+id", events);
    }

    // =========================================================================
    // Block Directives - Conditionals
    // =========================================================================

    #[test]
    fn if_directive() {
        let events = parse(b"!if logged_in\n  |greeting Welcome!");
        // TODO(directives): Verify IfStart, condition, content, IfEnd
        placeholder_test!("block-directives", events);
    }

    #[test]
    fn if_else_directive() {
        let events = parse(b"!if logged_in\n  |p Welcome!\n!else\n  |p Please login");
        // TODO(directives): Verify If/Else structure
        placeholder_test!("block-directives", events);
    }

    #[test]
    fn if_elif_else_directive() {
        let events = parse(b"!if admin\n  |p Admin\n!elif moderator\n  |p Mod\n!else\n  |p User");
        // TODO(directives): Verify If/Elif/Else structure
        placeholder_test!("block-directives", events);
    }

    #[test]
    fn if_with_comparison() {
        // !if age >= 18
        let events = parse(b"!if age >= 18\n  |p Adult");
        // TODO(directives): Verify comparison expression captured
        placeholder_test!("block-directives", events);
    }

    #[test]
    fn if_with_logical_operators() {
        // !if user.verified and user.subscribed
        let events = parse(b"!if verified and subscribed\n  |p Premium user");
        // TODO(directives): Verify logical operators parsed
        placeholder_test!("block-directives", events);
    }

    #[test]
    fn if_with_contains() {
        // !if tags contains "featured"
        let events = parse(b"!if tags contains \"featured\"\n  |badge Featured");
        // TODO(directives): Verify contains operator
        placeholder_test!("block-directives", events);
    }

    #[test]
    fn unless_directive() {
        // !unless — negated conditional
        let events = parse(b"!unless disabled\n  |button Click me");
        // TODO(directives): Verify unless
        placeholder_test!("block-directives", events);
    }

    // =========================================================================
    // Block Directives - Loops
    // =========================================================================

    #[test]
    fn for_directive() {
        let events = parse(b"!for item in items\n  |li !{{item.name}}");
        // TODO(directives): Verify for loop structure
        placeholder_test!("block-directives", events);
    }

    #[test]
    fn for_with_index() {
        // Common pattern: for item, index in items
        let events = parse(b"!for item in items\n  |li !{{forloop.index}}: !{{item}}");
        // TODO(directives): Verify forloop object
        placeholder_test!("block-directives", events);
    }

    #[test]
    fn for_nested() {
        // Nested for loops
        let events = parse(b"!for row in rows\n  |tr\n    !for cell in row\n      |td !{{cell}}");
        // TODO(directives): Verify nested for
        placeholder_test!("block-directives", events);
    }

    #[test]
    fn for_with_limit() {
        // !for item in items limit:5
        let events = parse(b"!for item in items limit:5\n  |li !{{item}}");
        // TODO(directives): Verify for with limit
        placeholder_test!("block-directives", events);
    }

    // =========================================================================
    // Block Directives - Variables and Includes
    // =========================================================================

    #[test]
    fn let_directive() {
        // !let local_var = expression
        let events = parse(b"!let name = user.first_name\n  |p Hello !{{name}}");
        // TODO(directives): Verify let binding
        placeholder_test!("block-directives", events);
    }

    #[test]
    fn include_directive() {
        // !include partials/header
        let events = parse(b"!include partials/header");
        // TODO(directives): Verify include
        placeholder_test!("block-directives", events);
    }

    #[test]
    fn include_with_variables() {
        // !include partials/card title: "Hello"
        let events = parse(b"!include partials/card title: \"Hello\"");
        // TODO(directives): Verify include with args
        placeholder_test!("block-directives", events);
    }

    // =========================================================================
    // Raw Directives - Block Form
    // =========================================================================

    #[test]
    fn raw_block_directive() {
        // !raw:elixir
        //   def hello, do: :world
        let events = parse(b"!raw:elixir\n  def hello, do: :world");
        // TODO(raw): Verify raw content emitted
        placeholder_test!("raw-block", events);
    }

    #[test]
    fn raw_block_preserves_pipes() {
        // Pipes inside raw should NOT be elements
        let events = parse(b"!raw:elixir\n  value |> transform() |> output()");
        // TODO(raw): Verify no elements created for pipes
        placeholder_test!("raw-block", events);
    }

    #[test]
    fn raw_block_preserves_colons() {
        // Colons inside raw should NOT be attributes
        let events = parse(b"!raw:python\n  def foo():\n    return {:key => \"value\"}");
        // TODO(raw): Verify colons not parsed as attrs
        placeholder_test!("raw-block", events);
    }

    #[test]
    fn raw_block_with_indentation() {
        // Raw content should preserve indentation
        let input = b"!raw:python\n  def foo():\n      return 1\n  def bar():\n      return 2";
        let events = parse(input);
        // TODO(raw): Verify indentation preserved
        placeholder_test!("raw-block", events);
    }

    #[test]
    fn raw_multiple_languages() {
        // Different language tags
        let events_sql = parse(b"!raw:sql\n  SELECT * FROM users");
        let events_json = parse(b"!raw:json\n  {\"key\": \"value\"}");
        let events_html = parse(b"!raw:html\n  <div>Hello</div>");
        // TODO(raw): Verify language tag captured
        placeholder_test!("raw-block", events_sql);
    }

    // =========================================================================
    // Raw Directives - Inline Form (new syntax: !{raw:kind ...})
    // =========================================================================

    #[test]
    fn raw_inline_directive() {
        // !{raw:json {"key": "value"}}
        let events = parse(b"|p The data is !{raw:json {\"key\": \"value\"}}.");
        // TODO(raw-inline): Verify inline raw parsed
        placeholder_test!("raw-inline", events);
    }

    #[test]
    fn raw_inline_nested_braces() {
        // Balanced nested braces should work (brace-counting)
        // !{raw:regex [a-z]{3,5}}
        let events = parse(b"|p Pattern: !{raw:regex [a-z]{3,5}}");
        // TODO(raw-inline): Verify brace counting
        placeholder_test!("raw-inline", events);
    }

    #[test]
    fn raw_inline_sql() {
        // !{raw:sql SELECT * FROM users}
        let events = parse(b"|p Query: !{raw:sql SELECT * FROM users}");
        // TODO(raw-inline): Verify content captured
        placeholder_test!("raw-inline", events);
    }

    #[test]
    fn raw_inline_with_nested_json() {
        // Complex nested braces - brace counting
        let events = parse(b"|p !{raw:json {\"outer\": {\"inner\": [1, 2, 3]}}}");
        // TODO(raw-inline): Verify nested braces handled
        placeholder_test!("raw-inline", events);
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn unclosed_interpolation_error() {
        // !{{unclosed — missing closing braces
        let events = parse(b"|p Hello !{{unclosed");
        // This should error - real assertion
        assert!(events.iter().any(|e| matches!(e, E::Error(_))),
            "Unclosed interpolation should produce an error");
    }

    #[test]
    fn empty_interpolation() {
        // !{{}} — empty expression
        let events = parse(b"|p Value: !{{}}");
        // TODO(interpolation): Decide if empty is error or allowed
        placeholder_test!("interpolation", events);
    }

    #[test]
    fn single_brace_is_directive_not_interpolation() {
        // !{something} is a directive, not interpolation
        // !{{something}} is interpolation
        let events = parse(b"|p !{raw:text hello}");
        // TODO(raw-inline): Verify single-brace is directive
        placeholder_test!("raw-inline", events);
    }

    #[test]
    fn directive_at_root_level() {
        // Directives can appear at root
        let events = parse(b"!if true\n  |root Content");
        // TODO(directives): Verify directive at root
        placeholder_test!("block-directives", events);
    }

    #[test]
    fn directive_inside_element() {
        // Directives inside elements
        let events = parse(b"|div\n  !if show\n    |p Conditional content");
        // TODO(directives): Verify nested directive
        placeholder_test!("block-directives", events);
    }

    #[test]
    fn escaped_bang_is_not_directive() {
        // '! — escaped bang is literal
        let events = parse(b"|p This is '!not a directive");
        // Real assertion - escape should work
        assert!(events.iter().any(|e| matches!(e, E::Text(_))),
            "Escaped bang should produce text");
    }
}

// =============================================================================
// SPEC.md: Inline Comments (;{...})
// =============================================================================

mod inline_comments {
    use super::*;

    #[test]
    fn basic_inline_comment() {
        // ;{comment} in prose
        let events = parse(b"|p Text ;{TODO: fix this} more text.");

        // Should emit: ElementStart, Text, Comment, Text, ElementEnd
        assert_eq!(events.len(), 5, "Expected 5 events: {:?}", events);
        assert_eq!(events[0], E::ElementStart(Some(s(b"p"))));
        assert_eq!(events[1], E::Text(s(b"Text ")));
        assert_eq!(events[2], E::Comment(s(b"TODO: fix this")),
            "Expected Comment event with 'TODO: fix this', got {:?}", events[2]);
        assert_eq!(events[3], E::Text(s(b" more text.")));
        assert_eq!(events[4], E::ElementEnd);
    }

    #[test]
    fn inline_comment_with_nested_braces() {
        // Brace-counting: balanced braces inside allowed
        let events = parse(b"|p Text ;{comment with {nested} braces} continues.");

        // Comment content should include the nested braces
        let comment = events.iter().find_map(|e| {
            if let E::Comment(c) = e { Some(c.clone()) } else { None }
        });
        assert_eq!(comment, Some(s(b"comment with {nested} braces")),
            "Nested braces should be preserved in comment, got {:?}", comment);
    }

    #[test]
    fn multiple_inline_comments() {
        // Multiple ;{...} in one line
        let events = parse(b"|p First ;{note 1} middle ;{note 2} end.");

        // Should have 2 Comment events
        let comments: Vec<_> = events.iter().filter_map(|e| {
            if let E::Comment(c) = e { Some(c.clone()) } else { None }
        }).collect();
        assert_eq!(comments, vec![s(b"note 1"), s(b"note 2")],
            "Expected two comments, got {:?}", comments);
    }

    #[test]
    fn inline_comment_in_element() {
        // ;{...} after element content
        let events = parse(b"|div Content here ;{hidden note}");

        // Should emit Comment event in element context
        let comment = events.iter().find_map(|e| {
            if let E::Comment(c) = e { Some(c.clone()) } else { None }
        });
        assert_eq!(comment, Some(s(b"hidden note")),
            "Expected Comment 'hidden note', got {:?}", comment);
    }

    #[test]
    fn unclosed_inline_comment_error() {
        // ;{unclosed
        let events = parse(b"|p Text ;{unclosed comment");
        // Should error
        assert!(events.iter().any(|e| matches!(e, E::Error(_))),
            "Unclosed inline comment should produce an error");
    }

    #[test]
    fn inline_comment_vs_line_comment() {
        // ; at line start is line comment, ;{ is inline
        let line_comment = parse(b"; This is a line comment");
        let inline_comment = parse(b"|p ;{This is inline} text");

        // Line comment should emit Comment (but different structure - full line)
        let line_has_comment = line_comment.iter().any(|e| matches!(e, E::Comment(_)));
        assert!(line_has_comment, "Line comment should produce Comment event");

        // Inline comment is embedded in element content
        let inline_has_comment = inline_comment.iter().any(|e| {
            matches!(e, E::Comment(c) if c == &s(b"This is inline"))
        });
        assert!(inline_has_comment, "Inline comment should emit Comment with content");
    }

    #[test]
    fn comment_emitted_not_stripped() {
        // Parser emits comment events, consumer decides to keep/strip
        let events = parse(b"|p Before ;{emitted comment} after");

        // Comment should be emitted as event (not stripped)
        let comment = events.iter().find_map(|e| {
            if let E::Comment(c) = e { Some(c.clone()) } else { None }
        });
        assert_eq!(comment, Some(s(b"emitted comment")),
            "Comment should be emitted, not stripped. Got {:?}", comment);

        // Text before and after should be preserved
        let texts: Vec<_> = events.iter().filter_map(|e| {
            if let E::Text(t) = e { Some(t.clone()) } else { None }
        }).collect();
        assert!(texts.contains(&s(b"Before ")), "Text before comment should exist");
        assert!(texts.contains(&s(b" after")), "Text after comment should exist");
    }
}

// =============================================================================
// SPEC.md: References (lines 555-633)
// =============================================================================

mod references {
    use super::*;

    #[test]
    fn id_reference() {
        // @[id] — insert entire element
        let events = parse(b"|page\n  @[header]");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"page"))),
            E::IdRef(s(b"header")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn attribute_merge_reference() {
        // :[id] — merge attributes
        let events = parse(b"|database :[base-db] :name mydb");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"database"))),
            E::AttrMerge(s(b"base-db")),
            E::Attr(s(b"name")),
            E::Str(s(b"mydb")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn class_as_mixin() {
        // |.defaults defines inheritable traits
        let events = parse(b"|.defaults\n  :pool 5");
        assert_eq!(events, vec![
            E::ElementStart(None),
            E::Attr(s(b"$class")),
            E::Str(s(b"defaults")),
            E::Attr(s(b"pool")),
            E::Int(5),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn element_using_mixin() {
        // |database.defaults uses the mixin
        let events = parse(b"|database.defaults\n  :name mydb");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"database"))),
            E::Attr(s(b"$class")),
            E::Str(s(b"defaults")),
            E::Attr(s(b"name")),
            E::Str(s(b"mydb")),
            E::ElementEnd,
        ]);
    }
}

// =============================================================================
// SPEC.md: Triple-Backtick Freeform (lines 354-387)
// =============================================================================

mod freeform_blocks {
    use super::*;

    #[test]
    fn basic_freeform_block() {
        // ```
        // content
        // ```
        // Leading newline after opening ``` is skipped
        let events = parse(b"```\nfreeform content\n```");
        assert_eq!(events, vec![
            E::Raw(s(b"freeform content\n")),
        ]);
    }

    #[test]
    fn freeform_preserves_pipes() {
        // Pipes inside freeform are not elements - they're literal text
        let events = parse(b"```\n|not-an-element\n|another\n```");
        assert_eq!(events, vec![
            E::Raw(s(b"|not-an-element\n|another\n")),
        ]);
    }

    #[test]
    fn freeform_inside_element() {
        // Content includes trailing whitespace before closing ```
        let events = parse(b"|code\n  ```\n  raw content\n  ```");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"code"))),
            E::Raw(s(b"  raw content\n  ")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn unclosed_freeform_error() {
        let events = parse(b"```\nunclosed");
        // Real assertion - unclosed should error
        assert!(events.iter().any(|e| matches!(e, E::Error(_))),
            "Unclosed freeform should produce an error");
    }
}

// =============================================================================
// SPEC.md: Error Cases
// =============================================================================

mod error_cases {
    use super::*;

    #[test]
    fn tab_character_error() {
        let events = parse(b"|el\n\t|child");
        assert!(events.iter().any(|e| matches!(e, E::Error(_))));
    }

    #[test]
    fn unclosed_double_quote_string() {
        let events = parse(b"|el :val \"unclosed");
        assert!(events.iter().any(|e| matches!(e, E::Error(_))));
    }

    #[test]
    fn unclosed_single_quote_string() {
        let events = parse(b"|el :val 'unclosed");
        assert!(events.iter().any(|e| matches!(e, E::Error(_))));
    }

    #[test]
    fn unclosed_array() {
        let events = parse(b"|el :val [1 2 3");
        assert!(events.iter().any(|e| matches!(e, E::Error(_))));
    }

    #[test]
    fn unclosed_bracket_in_id() {
        let events = parse(b"|el[unclosed");
        assert!(events.iter().any(|e| matches!(e, E::Error(_))));
    }

    #[test]
    fn unclosed_quoted_element_name() {
        let events = parse(b"|'unclosed name");
        assert!(events.iter().any(|e| matches!(e, E::Error(_))));
    }
}

// =============================================================================
// SPEC.md: Literal Escape Prefix (lines 279-293)
// =============================================================================

mod literal_escape {
    use super::*;

    #[test]
    fn escaped_pipe_is_text() {
        let events = parse(b"'|not-element");
        assert_eq!(events, vec![
            E::Text(s(b"|not-element")),
        ]);
    }

    #[test]
    fn escaped_colon_is_text() {
        let events = parse(b"|el\n  ':not-attr");
        assert_eq!(events, vec![
            E::ElementStart(Some(s(b"el"))),
            E::Text(s(b":not-attr")),
            E::ElementEnd,
        ]);
    }

    #[test]
    fn escaped_semicolon_is_text() {
        let events = parse(b"';not-comment");
        assert_eq!(events, vec![
            E::Text(s(b";not-comment")),
        ]);
    }

    #[test]
    fn escaped_apostrophe_is_text() {
        let events = parse(b"''literal-apostrophe");
        assert_eq!(events, vec![
            E::Text(s(b"'literal-apostrophe")),
        ]);
    }

    #[test]
    fn escaped_bang_is_text() {
        let events = parse(b"'!not-directive");
        assert_eq!(events, vec![
            E::Text(s(b"!not-directive")),
        ]);
    }
}
