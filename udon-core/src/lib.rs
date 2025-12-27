//! UDON Core Parser
//!
//! Streaming, event-based parser for UDON (Universal Document & Object Notation).
//! Emits structural events without building an AST.
//!
//! # Architecture
//!
//! - **streaming.rs** - Ring buffer, chunk arena, StreamingEvent enum
//! - **parser.rs** - Generated streaming state machine from .machine DSL
//! - **span.rs** - Span/Location types
//! - **value.rs** - Scalar value types

pub mod parser;
pub mod span;
pub mod streaming;
pub mod value;

pub use parser::{StreamingParser, ParserState, FunctionId};
pub use span::{Location, Span};
pub use streaming::{ChunkArena, ChunkSlice, EventRing, FeedResult, InlineDirectiveData, ParseErrorCode, StreamingEvent};
pub use value::Value;
