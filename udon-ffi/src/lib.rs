//! C FFI bindings for the UDON parser.
//!
//! This crate provides a C-compatible API for parsing UDON documents.
//! Events are returned as C structs with pointers into the original input.
//!
//! # Memory Model
//!
//! The parser copies the input internally, so the caller can free their copy
//! after calling `udon_parser_new`. All event pointers remain valid until
//! `udon_parser_free` is called.
//!
//! # Example (C)
//!
//! ```c
//! UdonParser* p = udon_parser_new(input, strlen(input));
//! const UdonEvent* e;
//! while ((e = udon_parser_next(p)) != NULL) {
//!     switch (e->type) {
//!         case UDON_EVENT_ELEMENT_START:
//!             // e->data.element_start.name_ptr, name_len
//!             break;
//!         // ...
//!     }
//! }
//! udon_parser_free(p);
//! ```

use std::ffi::{c_char, CString};
use std::ptr;
use serde::Serialize;
use udon_core::{Event, Parser, Span, Value};

/// Event types matching the Rust Event enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UdonEventType {
    ElementStart = 1,
    ElementEnd = 2,
    Attribute = 3,
    EmbeddedStart = 4,
    EmbeddedEnd = 5,
    DirectiveStart = 6,
    DirectiveEnd = 7,
    InlineDirective = 8,
    Interpolation = 9,
    Text = 10,
    RawContent = 11,
    Comment = 12,
    IdReference = 13,
    AttributeMerge = 14,
    FreeformStart = 15,
    FreeformEnd = 16,
    Error = 17,
}

/// A byte slice for FFI (pointer + length).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UdonSlice {
    pub ptr: *const u8,
    pub len: usize,
}

impl UdonSlice {
    fn from_bytes(bytes: &[u8]) -> Self {
        UdonSlice {
            ptr: bytes.as_ptr(),
            len: bytes.len(),
        }
    }

    fn null() -> Self {
        UdonSlice {
            ptr: ptr::null(),
            len: 0,
        }
    }
}

/// Source span (byte offsets).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UdonSpan {
    pub start: u32,
    pub end: u32,
}

impl From<Span> for UdonSpan {
    fn from(s: Span) -> Self {
        UdonSpan {
            start: s.start as u32,
            end: s.end as u32,
        }
    }
}

/// Value types for attributes.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UdonValueType {
    None = 0,
    Nil = 1,
    Bool = 2,
    Integer = 3,
    Float = 4,
    Rational = 5,
    Complex = 6,
    String = 7,
    QuotedString = 8,
    List = 9,
}

/// Attribute value for FFI.
///
/// For simple types (nil, bool, int, float), the value is stored directly.
/// For strings, `data` contains a pointer to the bytes.
/// For complex types, the raw bytes are stored in `data` for host parsing.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UdonValue {
    pub value_type: UdonValueType,
    pub data: UdonSlice,
    /// For Bool: 0=false, 1=true
    /// For Integer: the value (if it fits in i64)
    pub int_value: i64,
    /// For Float: the value
    pub float_value: f64,
}

impl UdonValue {
    fn from_value(v: Option<&Value<'_>>) -> Self {
        match v {
            None => UdonValue {
                value_type: UdonValueType::None,
                data: UdonSlice::null(),
                int_value: 0,
                float_value: 0.0,
            },
            Some(Value::Nil) => UdonValue {
                value_type: UdonValueType::Nil,
                data: UdonSlice::null(),
                int_value: 0,
                float_value: 0.0,
            },
            Some(Value::Bool(b)) => UdonValue {
                value_type: UdonValueType::Bool,
                data: UdonSlice::null(),
                int_value: if *b { 1 } else { 0 },
                float_value: 0.0,
            },
            Some(Value::Integer(i)) => UdonValue {
                value_type: UdonValueType::Integer,
                data: UdonSlice::null(),
                int_value: *i,
                float_value: 0.0,
            },
            Some(Value::Float(f)) => UdonValue {
                value_type: UdonValueType::Float,
                data: UdonSlice::null(),
                int_value: 0,
                float_value: *f,
            },
            Some(Value::Rational { numerator, denominator }) => UdonValue {
                value_type: UdonValueType::Rational,
                data: UdonSlice::null(),
                int_value: *numerator,    // Store numerator in int_value
                float_value: *denominator as f64, // Store denominator in float_value
            },
            Some(Value::Complex { real, imag: _ }) => UdonValue {
                value_type: UdonValueType::Complex,
                data: UdonSlice::null(),
                int_value: 0,
                float_value: *real, // Real part; imag in int_value as bits
            },
            Some(Value::String(s)) => UdonValue {
                value_type: UdonValueType::String,
                data: UdonSlice::from_bytes(s),
                int_value: 0,
                float_value: 0.0,
            },
            Some(Value::QuotedString(s)) => UdonValue {
                value_type: UdonValueType::QuotedString,
                data: UdonSlice::from_bytes(s),
                int_value: 0,
                float_value: 0.0,
            },
            Some(Value::List(_)) => UdonValue {
                // Lists are complex - for now just mark as list, host can reparse
                value_type: UdonValueType::List,
                data: UdonSlice::null(),
                int_value: 0,
                float_value: 0.0,
            },
        }
    }
}

/// Maximum number of classes we can return (stack allocated).
const MAX_CLASSES: usize = 16;

/// Element start data.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UdonElementStart {
    pub name: UdonSlice,
    pub id: UdonValue,
    pub classes: [UdonSlice; MAX_CLASSES],
    pub num_classes: u8,
    pub suffix: c_char, // 0 if none, otherwise '?', '!', '*', '+'
    pub span: UdonSpan,
}

/// Attribute data.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UdonAttribute {
    pub key: UdonSlice,
    pub value: UdonValue,
    pub span: UdonSpan,
}

/// Directive data.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UdonDirective {
    pub name: UdonSlice,
    pub namespace: UdonSlice,
    pub is_raw: bool,
    pub span: UdonSpan,
}

/// Inline directive data.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UdonInlineDirective {
    pub name: UdonSlice,
    pub namespace: UdonSlice,
    pub is_raw: bool,
    pub content: UdonSlice,
    pub span: UdonSpan,
}

/// Text/content data.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UdonContent {
    pub content: UdonSlice,
    pub span: UdonSpan,
}

/// Error data.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UdonError {
    pub message: *const c_char,
    pub span: UdonSpan,
}

/// Union of all event data types.
#[repr(C)]
pub union UdonEventData {
    pub element_start: UdonElementStart,
    pub attribute: UdonAttribute,
    pub directive: UdonDirective,
    pub inline_directive: UdonInlineDirective,
    pub content: UdonContent,
    pub error: UdonError,
    pub span: UdonSpan, // For simple events like ElementEnd
}

/// FFI event structure.
#[repr(C)]
pub struct UdonEvent {
    pub event_type: UdonEventType,
    pub data: UdonEventData,
}

/// Opaque parser handle.
///
/// Uses a single-slot design: only one FFI event struct exists,
/// and it gets overwritten on each `next()` call. This avoids
/// the overhead of converting all events upfront.
pub struct UdonParser {
    /// Owned copy of input (events reference this).
    /// SAFETY: Must stay alive as long as `events` exists.
    #[allow(dead_code)]
    input: Vec<u8>,

    /// Parsed Rust events (reference input via 'static lie).
    /// SAFETY: These contain references to `input`. The 'static lifetime
    /// is a lie, but is sound because:
    /// 1. input is owned by this struct
    /// 2. events are never exposed outside this struct
    /// 3. input is only freed when the struct is freed (after events)
    events: Vec<Event<'static>>,

    /// Single FFI event slot - reused on each next() call.
    /// The returned pointer is only valid until the next next() call.
    current: UdonEvent,

    /// Current position in events
    pos: usize,
}

/// Create a new parser and parse the input.
///
/// The input is copied internally, so the caller can free their copy.
/// Returns NULL on allocation failure.
#[no_mangle]
pub extern "C" fn udon_parser_new(input: *const u8, len: usize) -> *mut UdonParser {
    if input.is_null() && len > 0 {
        return ptr::null_mut();
    }

    // Copy input into owned buffer
    let input_slice = if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(input, len) }
    };
    let owned_input = input_slice.to_vec();

    // Parse - events will reference owned_input
    let mut parser = Parser::new(&owned_input);
    let rust_events = parser.parse();

    // SAFETY: Transmute the lifetime from 'a (tied to owned_input) to 'static.
    // This is sound because:
    // 1. owned_input is moved into the struct and lives as long as the struct
    // 2. events are never exposed outside the struct (we convert on demand)
    // 3. The struct is only freed via udon_parser_free, which drops in correct order
    let events: Vec<Event<'static>> = unsafe { std::mem::transmute(rust_events) };

    // Create zeroed FFI event for the single slot
    let current = UdonEvent {
        event_type: UdonEventType::Text,
        data: UdonEventData {
            span: UdonSpan { start: 0, end: 0 },
        },
    };

    let parser = Box::new(UdonParser {
        input: owned_input,
        events,
        current,
        pos: 0,
    });

    Box::into_raw(parser)
}

/// Get the next event, or NULL if no more events.
///
/// The returned pointer is valid until the next call to `udon_parser_next`
/// or until `udon_parser_free` is called. Do not free the returned pointer.
#[no_mangle]
pub extern "C" fn udon_parser_next(parser: *mut UdonParser) -> *const UdonEvent {
    if parser.is_null() {
        return ptr::null();
    }

    let parser = unsafe { &mut *parser };

    if parser.pos >= parser.events.len() {
        return ptr::null();
    }

    // Convert just this one event into the single slot
    let rust_event = &parser.events[parser.pos];
    parser.current = convert_event(rust_event);
    parser.pos += 1;

    &parser.current as *const UdonEvent
}

/// Reset the parser to the beginning.
#[no_mangle]
pub extern "C" fn udon_parser_reset(parser: *mut UdonParser) {
    if !parser.is_null() {
        let parser = unsafe { &mut *parser };
        parser.pos = 0;
    }
}

/// Get the number of events.
#[no_mangle]
pub extern "C" fn udon_parser_event_count(parser: *const UdonParser) -> usize {
    if parser.is_null() {
        return 0;
    }
    let parser = unsafe { &*parser };
    parser.events.len()
}

/// Free the parser.
#[no_mangle]
pub extern "C" fn udon_parser_free(parser: *mut UdonParser) {
    if !parser.is_null() {
        unsafe {
            drop(Box::from_raw(parser));
        }
    }
}

/// Get the library version.
#[no_mangle]
pub extern "C" fn udon_version() -> *const c_char {
    // Null-terminated version string
    b"0.9.0\0".as_ptr() as *const c_char
}

// --- Internal conversion ---

fn convert_event(event: &Event<'_>) -> UdonEvent {
    match event {
        Event::ElementStart {
            name,
            id,
            classes,
            suffix,
            span,
        } => {
            let mut c_classes = [UdonSlice::null(); MAX_CLASSES];
            let num_classes = classes.len().min(MAX_CLASSES);
            for (i, class) in classes.iter().take(MAX_CLASSES).enumerate() {
                c_classes[i] = UdonSlice::from_bytes(class);
            }

            UdonEvent {
                event_type: UdonEventType::ElementStart,
                data: UdonEventData {
                    element_start: UdonElementStart {
                        name: name.map(UdonSlice::from_bytes).unwrap_or(UdonSlice::null()),
                        id: UdonValue::from_value(id.as_ref()),
                        classes: c_classes,
                        num_classes: num_classes as u8,
                        suffix: suffix.map(|c| c as c_char).unwrap_or(0),
                        span: (*span).into(),
                    },
                },
            }
        }

        Event::ElementEnd { span } => UdonEvent {
            event_type: UdonEventType::ElementEnd,
            data: UdonEventData {
                span: (*span).into(),
            },
        },

        Event::Attribute { key, value, span } => UdonEvent {
            event_type: UdonEventType::Attribute,
            data: UdonEventData {
                attribute: UdonAttribute {
                    key: UdonSlice::from_bytes(key),
                    value: UdonValue::from_value(value.as_ref()),
                    span: (*span).into(),
                },
            },
        },

        Event::EmbeddedStart {
            name,
            id,
            classes,
            span,
        } => {
            let mut c_classes = [UdonSlice::null(); MAX_CLASSES];
            let num_classes = classes.len().min(MAX_CLASSES);
            for (i, class) in classes.iter().take(MAX_CLASSES).enumerate() {
                c_classes[i] = UdonSlice::from_bytes(class);
            }

            UdonEvent {
                event_type: UdonEventType::EmbeddedStart,
                data: UdonEventData {
                    element_start: UdonElementStart {
                        name: name.map(UdonSlice::from_bytes).unwrap_or(UdonSlice::null()),
                        id: UdonValue::from_value(id.as_ref()),
                        classes: c_classes,
                        num_classes: num_classes as u8,
                        suffix: 0,
                        span: (*span).into(),
                    },
                },
            }
        }

        Event::EmbeddedEnd { span } => UdonEvent {
            event_type: UdonEventType::EmbeddedEnd,
            data: UdonEventData {
                span: (*span).into(),
            },
        },

        Event::DirectiveStart {
            name,
            namespace,
            is_raw,
            span,
        } => UdonEvent {
            event_type: UdonEventType::DirectiveStart,
            data: UdonEventData {
                directive: UdonDirective {
                    name: UdonSlice::from_bytes(name),
                    namespace: namespace
                        .map(UdonSlice::from_bytes)
                        .unwrap_or(UdonSlice::null()),
                    is_raw: *is_raw,
                    span: (*span).into(),
                },
            },
        },

        Event::DirectiveEnd { span } => UdonEvent {
            event_type: UdonEventType::DirectiveEnd,
            data: UdonEventData {
                span: (*span).into(),
            },
        },

        Event::InlineDirective {
            name,
            namespace,
            is_raw,
            content,
            span,
        } => UdonEvent {
            event_type: UdonEventType::InlineDirective,
            data: UdonEventData {
                inline_directive: UdonInlineDirective {
                    name: UdonSlice::from_bytes(name),
                    namespace: namespace
                        .map(UdonSlice::from_bytes)
                        .unwrap_or(UdonSlice::null()),
                    is_raw: *is_raw,
                    content: UdonSlice::from_bytes(content),
                    span: (*span).into(),
                },
            },
        },

        Event::Interpolation { expression, span } => UdonEvent {
            event_type: UdonEventType::Interpolation,
            data: UdonEventData {
                content: UdonContent {
                    content: UdonSlice::from_bytes(expression),
                    span: (*span).into(),
                },
            },
        },

        Event::Text { content, span } => UdonEvent {
            event_type: UdonEventType::Text,
            data: UdonEventData {
                content: UdonContent {
                    content: UdonSlice::from_bytes(content),
                    span: (*span).into(),
                },
            },
        },

        Event::RawContent { content, span } => UdonEvent {
            event_type: UdonEventType::RawContent,
            data: UdonEventData {
                content: UdonContent {
                    content: UdonSlice::from_bytes(content),
                    span: (*span).into(),
                },
            },
        },

        Event::Comment { content, span } => UdonEvent {
            event_type: UdonEventType::Comment,
            data: UdonEventData {
                content: UdonContent {
                    content: UdonSlice::from_bytes(content),
                    span: (*span).into(),
                },
            },
        },

        Event::IdReference { id, span } => UdonEvent {
            event_type: UdonEventType::IdReference,
            data: UdonEventData {
                content: UdonContent {
                    content: UdonSlice::from_bytes(id),
                    span: (*span).into(),
                },
            },
        },

        Event::AttributeMerge { id, span } => UdonEvent {
            event_type: UdonEventType::AttributeMerge,
            data: UdonEventData {
                content: UdonContent {
                    content: UdonSlice::from_bytes(id),
                    span: (*span).into(),
                },
            },
        },

        Event::FreeformStart { span } => UdonEvent {
            event_type: UdonEventType::FreeformStart,
            data: UdonEventData {
                span: (*span).into(),
            },
        },

        Event::FreeformEnd { span } => UdonEvent {
            event_type: UdonEventType::FreeformEnd,
            data: UdonEventData {
                span: (*span).into(),
            },
        },

        Event::Error { message, span } => {
            // message is &'static str, so pointer is stable
            UdonEvent {
                event_type: UdonEventType::Error,
                data: UdonEventData {
                    error: UdonError {
                        message: message.as_ptr() as *const c_char,
                        span: (*span).into(),
                    },
                },
            }
        }
    }
}

// ========== Batch JSON Interface ==========
//
// For scripting languages where per-event FFI calls are expensive,
// this returns all events as a single JSON string.

/// JSON-serializable event for batch output.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum JsonEvent {
    ElementStart {
        name: Option<String>,
        id: Option<JsonValue>,
        classes: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        suffix: Option<char>,
        span: [u32; 2],
    },
    ElementEnd {
        span: [u32; 2],
    },
    Attribute {
        key: String,
        value: Option<JsonValue>,
        span: [u32; 2],
    },
    Text {
        content: String,
        span: [u32; 2],
    },
    Comment {
        content: String,
        span: [u32; 2],
    },
    Error {
        message: String,
        span: [u32; 2],
    },
    // Add others as needed
    #[serde(rename = "embedded_start")]
    EmbeddedStart {
        name: Option<String>,
        id: Option<JsonValue>,
        classes: Vec<String>,
        span: [u32; 2],
    },
    #[serde(rename = "embedded_end")]
    EmbeddedEnd { span: [u32; 2] },
    #[serde(rename = "directive_start")]
    DirectiveStart {
        name: String,
        namespace: Option<String>,
        is_raw: bool,
        span: [u32; 2],
    },
    #[serde(rename = "directive_end")]
    DirectiveEnd { span: [u32; 2] },
    #[serde(rename = "inline_directive")]
    InlineDirective {
        name: String,
        namespace: Option<String>,
        is_raw: bool,
        content: String,
        span: [u32; 2],
    },
    Interpolation {
        expression: String,
        span: [u32; 2],
    },
    #[serde(rename = "raw_content")]
    RawContent {
        content: String,
        span: [u32; 2],
    },
    #[serde(rename = "id_reference")]
    IdReference {
        id: String,
        span: [u32; 2],
    },
    #[serde(rename = "attribute_merge")]
    AttributeMerge {
        id: String,
        span: [u32; 2],
    },
    #[serde(rename = "freeform_start")]
    FreeformStart { span: [u32; 2] },
    #[serde(rename = "freeform_end")]
    FreeformEnd { span: [u32; 2] },
}

/// JSON-serializable value.
#[derive(Serialize)]
#[serde(untagged)]
enum JsonValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

fn bytes_to_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn value_to_json(v: &Value<'_>) -> JsonValue {
    match v {
        Value::Nil => JsonValue::Null,
        Value::Bool(b) => JsonValue::Bool(*b),
        Value::Integer(i) => JsonValue::Int(*i),
        Value::Float(f) => JsonValue::Float(*f),
        Value::String(s) | Value::QuotedString(s) => JsonValue::String(bytes_to_string(s)),
        Value::Rational { numerator, denominator } => {
            JsonValue::String(format!("{}/{}", numerator, denominator))
        }
        Value::Complex { real, imag } => {
            JsonValue::String(format!("{}+{}i", real, imag))
        }
        Value::List(_) => JsonValue::String("[list]".to_string()),
    }
}

fn event_to_json(event: &Event<'_>) -> JsonEvent {
    match event {
        Event::ElementStart { name, id, classes, suffix, span } => JsonEvent::ElementStart {
            name: name.map(bytes_to_string),
            id: id.as_ref().map(value_to_json),
            classes: classes.iter().map(|c| bytes_to_string(c)).collect(),
            suffix: *suffix,
            span: [span.start as u32, span.end as u32],
        },
        Event::ElementEnd { span } => JsonEvent::ElementEnd {
            span: [span.start as u32, span.end as u32],
        },
        Event::Attribute { key, value, span } => JsonEvent::Attribute {
            key: bytes_to_string(key),
            value: value.as_ref().map(value_to_json),
            span: [span.start as u32, span.end as u32],
        },
        Event::Text { content, span } => JsonEvent::Text {
            content: bytes_to_string(content),
            span: [span.start as u32, span.end as u32],
        },
        Event::Comment { content, span } => JsonEvent::Comment {
            content: bytes_to_string(content),
            span: [span.start as u32, span.end as u32],
        },
        Event::Error { message, span } => JsonEvent::Error {
            message: message.to_string(),
            span: [span.start as u32, span.end as u32],
        },
        Event::EmbeddedStart { name, id, classes, span } => JsonEvent::EmbeddedStart {
            name: name.map(bytes_to_string),
            id: id.as_ref().map(value_to_json),
            classes: classes.iter().map(|c| bytes_to_string(c)).collect(),
            span: [span.start as u32, span.end as u32],
        },
        Event::EmbeddedEnd { span } => JsonEvent::EmbeddedEnd {
            span: [span.start as u32, span.end as u32],
        },
        Event::DirectiveStart { name, namespace, is_raw, span } => JsonEvent::DirectiveStart {
            name: bytes_to_string(name),
            namespace: namespace.map(bytes_to_string),
            is_raw: *is_raw,
            span: [span.start as u32, span.end as u32],
        },
        Event::DirectiveEnd { span } => JsonEvent::DirectiveEnd {
            span: [span.start as u32, span.end as u32],
        },
        Event::InlineDirective { name, namespace, is_raw, content, span } => JsonEvent::InlineDirective {
            name: bytes_to_string(name),
            namespace: namespace.map(bytes_to_string),
            is_raw: *is_raw,
            content: bytes_to_string(content),
            span: [span.start as u32, span.end as u32],
        },
        Event::Interpolation { expression, span } => JsonEvent::Interpolation {
            expression: bytes_to_string(expression),
            span: [span.start as u32, span.end as u32],
        },
        Event::RawContent { content, span } => JsonEvent::RawContent {
            content: bytes_to_string(content),
            span: [span.start as u32, span.end as u32],
        },
        Event::IdReference { id, span } => JsonEvent::IdReference {
            id: bytes_to_string(id),
            span: [span.start as u32, span.end as u32],
        },
        Event::AttributeMerge { id, span } => JsonEvent::AttributeMerge {
            id: bytes_to_string(id),
            span: [span.start as u32, span.end as u32],
        },
        Event::FreeformStart { span } => JsonEvent::FreeformStart {
            span: [span.start as u32, span.end as u32],
        },
        Event::FreeformEnd { span } => JsonEvent::FreeformEnd {
            span: [span.start as u32, span.end as u32],
        },
    }
}

/// Parse UDON and return all events as a JSON string.
///
/// This is much faster than iterating with `udon_parser_next` for scripting
/// languages, as it avoids per-event FFI call overhead.
///
/// Returns a null-terminated JSON string. Caller must free with `udon_free_string`.
/// Returns NULL on error.
#[no_mangle]
pub extern "C" fn udon_parse_json(input: *const u8, len: usize) -> *mut c_char {
    if input.is_null() && len > 0 {
        return ptr::null_mut();
    }

    let input_slice = if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(input, len) }
    };

    let mut parser = Parser::new(input_slice);
    let events = parser.parse();

    let json_events: Vec<JsonEvent> = events.iter().map(event_to_json).collect();

    match serde_json::to_string(&json_events) {
        Ok(json) => {
            match CString::new(json) {
                Ok(cstr) => cstr.into_raw(),
                Err(_) => ptr::null_mut(),
            }
        }
        Err(_) => ptr::null_mut(),
    }
}

/// Free a string returned by `udon_parse_json`.
#[no_mangle]
pub extern "C" fn udon_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parse() {
        let input = b"|div Hello\n";
        let parser = udon_parser_new(input.as_ptr(), input.len());
        assert!(!parser.is_null());

        let count = udon_parser_event_count(parser);
        assert!(count > 0);

        // Get first event
        let event = udon_parser_next(parser);
        assert!(!event.is_null());

        let event = unsafe { &*event };
        assert_eq!(event.event_type, UdonEventType::ElementStart);

        udon_parser_free(parser);
    }

    #[test]
    fn test_empty_input() {
        let parser = udon_parser_new(ptr::null(), 0);
        assert!(!parser.is_null());

        let count = udon_parser_event_count(parser);
        assert_eq!(count, 0);

        let event = udon_parser_next(parser);
        assert!(event.is_null());

        udon_parser_free(parser);
    }
}
