//! Streaming event tests for UDON parser.
//!
//! Tests the SAX-style streaming event model where events emit immediately
//! as syntax is parsed, with no accumulation.
//!
//! Key patterns:
//! - ElementStart { name } followed by Attribute events for [id], .class, suffix
//! - Attribute { key } followed by value event(s)
//! - ArrayStart, value events..., ArrayEnd for list values

use udon_core::{Event, Parser};

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

    // Other
    Error(String),
    Other(String),
}

fn parse(input: &[u8]) -> Vec<E> {
    let mut parser = Parser::new(input);
    parser.parse().into_iter().map(E::from).collect()
}

impl From<Event<'_>> for E {
    fn from(event: Event<'_>) -> Self {
        match event {
            Event::ElementStart { name, .. } => E::ElementStart(name.map(|n| n.to_vec())),
            Event::ElementEnd { .. } => E::ElementEnd,
            Event::Attribute { key, .. } => E::Attr(key.to_vec()),
            Event::ArrayStart { .. } => E::ArrayStart,
            Event::ArrayEnd { .. } => E::ArrayEnd,
            Event::NilValue { .. } => E::Nil,
            Event::BoolValue { value, .. } => E::Bool(value),
            Event::IntegerValue { value, .. } => E::Int(value),
            Event::FloatValue { value, .. } => E::Float(format!("{}", value)),
            Event::StringValue { value, .. } => E::Str(value.to_vec()),
            Event::QuotedStringValue { value, .. } => E::QuotedStr(value.to_vec()),
            Event::Text { content, .. } => E::Text(content.to_vec()),
            Event::Comment { content, .. } => E::Comment(content.to_vec()),
            Event::RawContent { content, .. } => E::Raw(content.to_vec()),
            Event::Error { message, .. } => E::Error(message.to_string()),
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
