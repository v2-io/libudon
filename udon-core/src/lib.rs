//! UDON Core Parser
//!
//! Streaming, event-based parser for UDON (Universal Document & Object Notation).
//! Emits structural events without building an AST.
//!
//! # Architecture
//!
//! - **event.rs** - Event enum (hand-written, stable API)
//! - **span.rs** - Span/Location types
//! - **value.rs** - Attribute value types
//! - **parser.rs** - Generated from .machine DSL

pub mod event;
pub mod parser;
pub mod span;
pub mod value;

pub use event::Event;
pub use parser::Parser;
pub use span::{Location, Span};
pub use value::Value;
