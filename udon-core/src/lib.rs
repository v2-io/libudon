//! UDON Core Parser
//!
//! Streaming, event-based parser for UDON (Universal Document & Object Notation).
//! Emits structural events without building an AST.
//!
//! # Architecture
//!
//! - **event.rs** - Event enum for batch parsing (borrows from input)
//! - **streaming.rs** - Streaming infrastructure (ring buffer, chunk arena)
//! - **span.rs** - Span/Location types
//! - **value.rs** - Attribute value types
//! - **parser.rs** - Generated from .machine DSL

pub mod event;
pub mod parser;
pub mod span;
pub mod streaming;
pub mod value;

pub use event::Event;
pub use parser::{Parser, StreamingParser, ParserState, FunctionId};
pub use span::{Location, Span};
pub use streaming::{ChunkArena, ChunkSlice, EventRing, FeedResult, StreamingEvent};
pub use value::Value;
