//! Parser events - the core output of the UDON parser.
//!
//! The parser emits a stream of events rather than building an AST.
//! This enables streaming processing and lower memory usage.
//!
//! These types are stable and hand-written (not generated).

use crate::span::Span;
use crate::value::Value;

/// Core parser events.
///
/// The lifetime `'a` refers to the source buffer - all byte slices
/// are zero-copy references into the original input.
#[derive(Debug, Clone)]
pub enum Event<'a> {
    /// Element start: `|name[id].class1.class2`
    ElementStart {
        /// Element name (None for anonymous elements like `|[id]` or `|.class`)
        name: Option<&'a [u8]>,
        /// Element identity from `[id]` syntax
        id: Option<Value<'a>>,
        /// Class names from `.class` syntax
        classes: Vec<&'a [u8]>,
        /// Suffix modifier: `?`, `!`, `*`, or `+`
        suffix: Option<char>,
        /// Source span
        span: Span,
    },

    /// Element end (dedent detected)
    ElementEnd {
        span: Span,
    },

    /// Attribute: `:key value`
    Attribute {
        /// Attribute key
        key: &'a [u8],
        /// Attribute value (None for flag attributes like `:enabled`)
        value: Option<Value<'a>>,
        /// Source span
        span: Span,
    },

    /// Embedded element start: `|{`
    EmbeddedStart {
        name: Option<&'a [u8]>,
        id: Option<Value<'a>>,
        classes: Vec<&'a [u8]>,
        span: Span,
    },

    /// Embedded element end: `}`
    EmbeddedEnd {
        span: Span,
    },

    /// Block directive start: `!name` or `!raw:lang`
    DirectiveStart {
        /// Directive name (e.g., "if", "for", "sql" for `!raw:sql`)
        name: &'a [u8],
        /// Namespace (e.g., "raw" for `!raw:sql`)
        namespace: Option<&'a [u8]>,
        /// Whether this is a raw directive (body captured verbatim)
        is_raw: bool,
        /// Source span
        span: Span,
    },

    /// Block directive end
    DirectiveEnd {
        span: Span,
    },

    /// Inline directive: `!name{content}`
    InlineDirective {
        name: &'a [u8],
        namespace: Option<&'a [u8]>,
        is_raw: bool,
        /// Content inside the braces (for raw: verbatim; otherwise: parsed separately)
        content: &'a [u8],
        span: Span,
    },

    /// Interpolation: `!{expr}` or `!{expr | filter1 | filter2}`
    Interpolation {
        /// The expression (everything inside `!{...}`)
        expression: &'a [u8],
        /// Source span
        span: Span,
    },

    /// Text/prose content
    Text {
        content: &'a [u8],
        span: Span,
    },

    /// Raw content (inside `!raw:` directive)
    RawContent {
        content: &'a [u8],
        span: Span,
    },

    /// Comment: `; text`
    Comment {
        content: &'a [u8],
        span: Span,
    },

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

    /// Freeform block start: ``` ` ` ` ```
    FreeformStart {
        span: Span,
    },

    /// Freeform block end
    FreeformEnd {
        span: Span,
    },

    /// Parse error (with recovery - parsing continues)
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
            Event::Attribute { span, .. } => *span,
            Event::EmbeddedStart { span, .. } => *span,
            Event::EmbeddedEnd { span } => *span,
            Event::DirectiveStart { span, .. } => *span,
            Event::DirectiveEnd { span } => *span,
            Event::InlineDirective { span, .. } => *span,
            Event::Interpolation { span, .. } => *span,
            Event::Text { span, .. } => *span,
            Event::RawContent { span, .. } => *span,
            Event::Comment { span, .. } => *span,
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
}
