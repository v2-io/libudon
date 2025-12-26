//! Parser events - the core output of the UDON streaming parser.
//!
//! This is a SAX-style event model: events are emitted as the parser
//! encounters syntax, with no accumulation. Structure is represented
//! by start/end event pairs.
//!
//! For arrays: ArrayStart, value events..., ArrayEnd
//! For attributes: Attribute { key }, then value event(s)
//!
//! These types are stable and hand-written (not generated).

use crate::span::Span;

/// Streaming parser events.
///
/// The lifetime `'a` refers to the source buffer - all byte slices
/// are zero-copy references into the original input.
///
/// ## Event Sequences
///
/// Element identity `|foo[myid].bar?` emits:
/// ```text
/// ElementStart { name: "foo" }
/// Attribute { key: "$id" }
/// StringValue("myid")           // or other value type
/// Attribute { key: "$class" }
/// StringValue("bar")
/// Attribute { key: "?" }
/// BoolValue(true)
/// ```
///
/// Array value `:tags [a b c]` emits:
/// ```text
/// Attribute { key: "tags" }
/// ArrayStart
/// StringValue("a")
/// StringValue("b")
/// StringValue("c")
/// ArrayEnd
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Event<'a> {
    // ========== Structure Events ==========

    /// Element start: `|name`
    ///
    /// The name is None for anonymous elements (`|[id]` or `|.class`).
    /// Identity syntax ([id], .class, suffix) emits as subsequent Attribute events.
    ElementStart {
        name: Option<&'a [u8]>,
        span: Span,
    },

    /// Element end (dedent detected or document end)
    ElementEnd {
        span: Span,
    },

    /// Embedded element start: `|{`
    ///
    /// Like ElementStart, identity emits as subsequent Attribute events.
    EmbeddedStart {
        name: Option<&'a [u8]>,
        span: Span,
    },

    /// Embedded element end: `}`
    EmbeddedEnd {
        span: Span,
    },

    // ========== Attribute Events ==========

    /// Attribute key: `:key`
    ///
    /// The next event(s) are the value:
    /// - Scalar: one value event (StringValue, IntegerValue, etc.)
    /// - Array: ArrayStart, value events..., ArrayEnd
    /// - Flag (no value): BoolValue(true) is implied
    Attribute {
        key: &'a [u8],
        span: Span,
    },

    // ========== Value Events ==========

    /// Array/list start: `[`
    ArrayStart {
        span: Span,
    },

    /// Array/list end: `]`
    ArrayEnd {
        span: Span,
    },

    /// Nil value: `null`, `nil`, or `~`
    NilValue {
        span: Span,
    },

    /// Boolean value: `true` or `false`
    BoolValue {
        value: bool,
        span: Span,
    },

    /// Integer value: `42`, `0xFF`, `0o755`, `0b1010`, `-17`
    IntegerValue {
        value: i64,
        span: Span,
    },

    /// Float value: `3.14`, `1.5e-3`, `-2.5`
    FloatValue {
        value: f64,
        span: Span,
    },

    /// Rational value: `1/3r`, `22/7r`
    RationalValue {
        numerator: i64,
        denominator: i64,
        span: Span,
    },

    /// Complex value: `3+4i`, `5i`
    ComplexValue {
        real: f64,
        imag: f64,
        span: Span,
    },

    /// String value (unquoted bare string)
    StringValue {
        value: &'a [u8],
        span: Span,
    },

    /// Quoted string value (may need unescaping)
    QuotedStringValue {
        value: &'a [u8],
        span: Span,
    },

    // ========== Content Events ==========

    /// Text/prose content
    Text {
        content: &'a [u8],
        span: Span,
    },

    /// Raw content (inside freeform or raw directive)
    RawContent {
        content: &'a [u8],
        span: Span,
    },

    /// Comment: `; text`
    Comment {
        content: &'a [u8],
        span: Span,
    },

    // ========== Directive Events ==========

    /// Block directive start: `!name` or `!namespace:name`
    DirectiveStart {
        name: &'a [u8],
        namespace: Option<&'a [u8]>,
        span: Span,
    },

    /// Block directive end
    DirectiveEnd {
        span: Span,
    },

    /// Inline directive: `!name{content}`
    ///
    /// Content is the raw bytes inside braces. For `!raw:lang{...}`,
    /// content is verbatim. For other directives, content may need parsing.
    InlineDirective {
        name: &'a [u8],
        namespace: Option<&'a [u8]>,
        content: &'a [u8],
        span: Span,
    },

    /// Interpolation: `!{expr}` or `!{expr | filter}`
    Interpolation {
        expression: &'a [u8],
        span: Span,
    },

    // ========== Reference Events ==========

    /// ID reference: `@[id]`
    IdReference {
        id: &'a [u8],
        span: Span,
    },

    /// Attribute merge: `:[id]`
    AttributeMerge {
        id: &'a [u8],
        span: Span,
    },

    // ========== Block Events ==========

    /// Freeform block start: ``` ` ` ` ```
    FreeformStart {
        span: Span,
    },

    /// Freeform block end
    FreeformEnd {
        span: Span,
    },

    // ========== Error Events ==========

    /// Parse error (parser continues after emitting this)
    Error {
        message: &'static str,
        span: Span,
    },
}

impl<'a> Event<'a> {
    /// Get the span for this event.
    pub fn span(&self) -> Span {
        match self {
            Event::ElementStart { span, .. } => *span,
            Event::ElementEnd { span } => *span,
            Event::EmbeddedStart { span, .. } => *span,
            Event::EmbeddedEnd { span } => *span,
            Event::Attribute { span, .. } => *span,
            Event::ArrayStart { span } => *span,
            Event::ArrayEnd { span } => *span,
            Event::NilValue { span } => *span,
            Event::BoolValue { span, .. } => *span,
            Event::IntegerValue { span, .. } => *span,
            Event::FloatValue { span, .. } => *span,
            Event::RationalValue { span, .. } => *span,
            Event::ComplexValue { span, .. } => *span,
            Event::StringValue { span, .. } => *span,
            Event::QuotedStringValue { span, .. } => *span,
            Event::Text { span, .. } => *span,
            Event::RawContent { span, .. } => *span,
            Event::Comment { span, .. } => *span,
            Event::DirectiveStart { span, .. } => *span,
            Event::DirectiveEnd { span } => *span,
            Event::InlineDirective { span, .. } => *span,
            Event::Interpolation { span, .. } => *span,
            Event::IdReference { span, .. } => *span,
            Event::AttributeMerge { span, .. } => *span,
            Event::FreeformStart { span } => *span,
            Event::FreeformEnd { span } => *span,
            Event::Error { span, .. } => *span,
        }
    }

    /// Check if this is an error event.
    pub fn is_error(&self) -> bool {
        matches!(self, Event::Error { .. })
    }

    /// Check if this is a value event (can follow Attribute or be inside Array).
    pub fn is_value(&self) -> bool {
        matches!(
            self,
            Event::NilValue { .. }
                | Event::BoolValue { .. }
                | Event::IntegerValue { .. }
                | Event::FloatValue { .. }
                | Event::RationalValue { .. }
                | Event::ComplexValue { .. }
                | Event::StringValue { .. }
                | Event::QuotedStringValue { .. }
                | Event::ArrayStart { .. }
        )
    }

    /// Check if this is a structure start event (has matching end).
    pub fn is_structure_start(&self) -> bool {
        matches!(
            self,
            Event::ElementStart { .. }
                | Event::EmbeddedStart { .. }
                | Event::DirectiveStart { .. }
                | Event::ArrayStart { .. }
                | Event::FreeformStart { .. }
        )
    }
}
