//! Streaming parser infrastructure with ring buffer and chunk arena.
//!
//! This module provides true streaming parsing capability:
//! - Input arrives in chunks, events emitted as they're parsed
//! - Ring buffer prevents unbounded memory growth
//! - Chunk arena allows zero-copy references with proper cleanup
//! - Backpressure when consumer is slow
//!
//! # Architecture
//!
//! ```text
//! Input Chunks        Parser              Ring Buffer          Consumer
//!     │                  │                     │                   │
//!     │──feed(chunk)────▶│                     │                   │
//!     │                  │──emit(event)───────▶│                   │
//!     │                  │                     │◀──read()──────────│
//!     │                  │                     │──event────────────▶│
//!     │                  │◀──backpressure──────│                   │
//! ```
//!
//! # Memory Management
//!
//! Events contain `ChunkSlice` which references data in the chunk arena.
//! When events are consumed, chunks can be freed if no events reference them.

use crate::span::Span;

/// Error codes for parse errors.
///
/// Using an enum instead of String eliminates the 24-byte String overhead
/// and removes heap allocation for error messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ParseErrorCode {
    /// Unexpected end of input (generic)
    Unclosed = 0,
    /// Unclosed string literal
    UnclosedString,
    /// Unclosed quoted content
    UnclosedQuote,
    /// Unclosed array
    UnclosedArray,
    /// Unclosed bracket (e.g., in identity)
    UnclosedBracket,
    /// Unclosed block comment
    UnclosedComment,
    /// Unclosed directive
    UnclosedDirective,
    /// Unclosed freeform block
    UnclosedFreeform,
    /// Incomplete directive (missing required parts)
    IncompleteDirective,
    /// Expected attribute key after ':'
    ExpectedAttrKey,
    /// Expected class name after '.'
    ExpectedClassName,
    /// Unexpected content after value
    UnexpectedAfterValue,
    /// Tab characters not allowed (use spaces)
    NoTabs,
}

impl ParseErrorCode {
    /// Get a human-readable message for this error code.
    pub fn message(self) -> &'static str {
        match self {
            Self::Unclosed => "unclosed",
            Self::UnclosedString => "unclosed string",
            Self::UnclosedQuote => "unclosed quote",
            Self::UnclosedArray => "unclosed array",
            Self::UnclosedBracket => "unclosed bracket",
            Self::UnclosedComment => "unclosed comment",
            Self::UnclosedDirective => "unclosed directive",
            Self::UnclosedFreeform => "unclosed freeform",
            Self::IncompleteDirective => "incomplete directive",
            Self::ExpectedAttrKey => "expected attr key",
            Self::ExpectedClassName => "expected class name",
            Self::UnexpectedAfterValue => "unexpected after value",
            Self::NoTabs => "no tabs",
        }
    }
}

/// Data for an inline directive event.
///
/// Boxed in StreamingEvent to reduce enum size from 48 to 32 bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct InlineDirectiveData {
    pub name: ChunkSlice,
    pub namespace: Option<ChunkSlice>,
    pub content: ChunkSlice,
    pub span: Span,
}

/// A reference to a slice of bytes within a chunk.
///
/// This is 12 bytes: chunk_idx (u32) + start (u32) + end (u32).
/// Much smaller than a fat pointer (16 bytes) and allows cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkSlice {
    /// Index into the chunk arena
    pub chunk_idx: u32,
    /// Start offset within the chunk
    pub start: u32,
    /// End offset within the chunk (exclusive)
    pub end: u32,
}

impl ChunkSlice {
    /// Create a new chunk slice.
    #[inline]
    pub fn new(chunk_idx: u32, start: u32, end: u32) -> Self {
        Self { chunk_idx, start, end }
    }

    /// Length of the slice in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        (self.end - self.start) as usize
    }

    /// Check if the slice is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// A chunk of input data stored in the arena.
#[derive(Debug)]
pub struct Chunk {
    /// The actual bytes (owned)
    data: Box<[u8]>,
    /// Offset of this chunk in the overall input stream (for span calculation)
    stream_offset: u64,
}

impl Chunk {
    /// Create a new chunk from bytes.
    pub fn new(data: Vec<u8>, stream_offset: u64) -> Self {
        Self {
            data: data.into_boxed_slice(),
            stream_offset,
        }
    }

    /// Get a slice of the chunk's data.
    #[inline]
    pub fn slice(&self, start: u32, end: u32) -> &[u8] {
        &self.data[start as usize..end as usize]
    }

    /// Get the full data.
    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get the stream offset.
    #[inline]
    pub fn stream_offset(&self) -> u64 {
        self.stream_offset
    }
}

/// Arena for input chunks with reference tracking.
///
/// Chunks are stored contiguously and can be freed when no events
/// reference them (tracked by min_referenced).
#[derive(Debug)]
pub struct ChunkArena {
    chunks: Vec<Chunk>,
    /// Total bytes received so far (for stream offset calculation)
    total_bytes: u64,
    /// Minimum chunk index still potentially referenced by unconsumed events
    min_referenced: usize,
}

impl ChunkArena {
    /// Create a new empty arena.
    pub fn new() -> Self {
        Self {
            chunks: Vec::with_capacity(16),
            total_bytes: 0,
            min_referenced: 0,
        }
    }

    /// Add a new chunk to the arena, returning its index.
    pub fn push(&mut self, data: Vec<u8>) -> u32 {
        let idx = self.chunks.len() as u32;
        let offset = self.total_bytes;
        self.total_bytes += data.len() as u64;
        self.chunks.push(Chunk::new(data, offset));
        idx
    }

    /// Get a chunk by index.
    #[inline]
    pub fn get(&self, idx: u32) -> Option<&Chunk> {
        self.chunks.get(idx as usize)
    }

    /// Resolve a ChunkSlice to actual bytes.
    #[inline]
    pub fn resolve(&self, slice: ChunkSlice) -> Option<&[u8]> {
        self.chunks
            .get(slice.chunk_idx as usize)
            .map(|chunk| chunk.slice(slice.start, slice.end))
    }

    /// Mark events up to a certain chunk as consumed.
    /// Chunks before min_referenced can be freed.
    pub fn advance_consumed(&mut self, min_chunk: usize) {
        self.min_referenced = self.min_referenced.max(min_chunk);
        // Note: actual freeing is deferred - we could shrink the vec
        // but that would invalidate indices. Instead, we track what's
        // "logically freed" and can compact later if needed.
    }

    /// Number of chunks currently held.
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Check if arena is empty.
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Total bytes received through this arena.
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    /// Clear all chunks, resetting for reuse.
    /// Keeps allocated capacity.
    pub fn clear(&mut self) {
        self.chunks.clear();
        self.total_bytes = 0;
        self.min_referenced = 0;
    }
}

impl Default for ChunkArena {
    fn default() -> Self {
        Self::new()
    }
}

/// Streaming event with chunk-referenced data.
///
/// Unlike `Event<'a>` which borrows from input, `StreamingEvent` uses
/// `ChunkSlice` to reference data in the arena. This allows the input
/// chunks to be managed independently of event lifetimes.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamingEvent {
    // ========== Structure Events ==========

    /// Element start: `|name`
    ElementStart {
        name: Option<ChunkSlice>,
        span: Span,
    },

    /// Element end (dedent or document end)
    ElementEnd { span: Span },

    /// Embedded element start: `|{`
    EmbeddedStart {
        name: Option<ChunkSlice>,
        span: Span,
    },

    /// Embedded element end: `}`
    EmbeddedEnd { span: Span },

    // ========== Attribute Events ==========

    /// Attribute key: `:key`
    Attribute { key: ChunkSlice, span: Span },

    // ========== Value Events ==========

    ArrayStart { span: Span },
    ArrayEnd { span: Span },

    NilValue { span: Span },
    BoolValue { value: bool, span: Span },
    IntegerValue { value: i64, span: Span },
    FloatValue { value: f64, span: Span },
    RationalValue { numerator: i64, denominator: i64, span: Span },
    ComplexValue { real: f64, imag: f64, span: Span },
    StringValue { value: ChunkSlice, span: Span },
    QuotedStringValue { value: ChunkSlice, span: Span },

    // ========== Content Events ==========

    Text { content: ChunkSlice, span: Span },
    Comment { content: ChunkSlice, span: Span },
    RawContent { content: ChunkSlice, span: Span },

    // ========== Directive Events ==========

    DirectiveStart {
        name: ChunkSlice,
        namespace: Option<ChunkSlice>,
        span: Span,
    },
    DirectiveEnd { span: Span },

    /// Boxed to reduce enum size (this variant is ~48 bytes unboxed)
    InlineDirective(Box<InlineDirectiveData>),

    Interpolation { expression: ChunkSlice, span: Span },

    // ========== Reference Events ==========

    IdReference { id: ChunkSlice, span: Span },
    AttributeMerge { id: ChunkSlice, span: Span },

    // ========== Block Events ==========

    FreeformStart { span: Span },
    FreeformEnd { span: Span },

    // ========== Error ==========

    Error { code: ParseErrorCode, span: Span },
}

impl StreamingEvent {
    /// Get the span for this event.
    pub fn span(&self) -> Span {
        match self {
            Self::ElementStart { span, .. } => *span,
            Self::ElementEnd { span } => *span,
            Self::EmbeddedStart { span, .. } => *span,
            Self::EmbeddedEnd { span } => *span,
            Self::Attribute { span, .. } => *span,
            Self::ArrayStart { span } => *span,
            Self::ArrayEnd { span } => *span,
            Self::NilValue { span } => *span,
            Self::BoolValue { span, .. } => *span,
            Self::IntegerValue { span, .. } => *span,
            Self::FloatValue { span, .. } => *span,
            Self::RationalValue { span, .. } => *span,
            Self::ComplexValue { span, .. } => *span,
            Self::StringValue { span, .. } => *span,
            Self::QuotedStringValue { span, .. } => *span,
            Self::Text { span, .. } => *span,
            Self::Comment { span, .. } => *span,
            Self::RawContent { span, .. } => *span,
            Self::DirectiveStart { span, .. } => *span,
            Self::DirectiveEnd { span } => *span,
            Self::InlineDirective(data) => data.span,
            Self::Interpolation { span, .. } => *span,
            Self::IdReference { span, .. } => *span,
            Self::AttributeMerge { span, .. } => *span,
            Self::FreeformStart { span } => *span,
            Self::FreeformEnd { span } => *span,
            Self::Error { span, .. } => *span,
        }
    }

    /// Get the minimum chunk index referenced by this event.
    /// Used for determining when chunks can be freed.
    pub fn min_chunk_idx(&self) -> Option<u32> {
        match self {
            Self::ElementStart { name: Some(s), .. } => Some(s.chunk_idx),
            Self::EmbeddedStart { name: Some(s), .. } => Some(s.chunk_idx),
            Self::Attribute { key, .. } => Some(key.chunk_idx),
            Self::StringValue { value, .. } => Some(value.chunk_idx),
            Self::QuotedStringValue { value, .. } => Some(value.chunk_idx),
            Self::Text { content, .. } => Some(content.chunk_idx),
            Self::Comment { content, .. } => Some(content.chunk_idx),
            Self::RawContent { content, .. } => Some(content.chunk_idx),
            Self::DirectiveStart { name, .. } => Some(name.chunk_idx),
            Self::InlineDirective(data) => Some(data.name.chunk_idx),
            Self::Interpolation { expression, .. } => Some(expression.chunk_idx),
            Self::IdReference { id, .. } => Some(id.chunk_idx),
            Self::AttributeMerge { id, .. } => Some(id.chunk_idx),
            _ => None,
        }
    }
}

/// Fixed-size ring buffer for events.
///
/// Uses power-of-2 sizing for fast modulo via bitmask.
/// Provides backpressure when full - producer must wait for consumer to read.
#[derive(Debug)]
pub struct EventRing {
    /// The actual event storage (power-of-2 sized)
    events: Vec<Option<StreamingEvent>>,
    /// Read position (consumer)
    read_pos: usize,
    /// Write position (producer)
    write_pos: usize,
    /// Number of events currently in buffer
    count: usize,
    /// Capacity (power of 2) - stored directly for fast full check
    capacity: usize,
    /// Bitmask for fast modulo (capacity - 1)
    mask: usize,
}

impl EventRing {
    /// Create a new ring buffer with at least the given capacity.
    /// Actual capacity will be rounded up to the next power of 2.
    pub fn new(min_capacity: usize) -> Self {
        // Round up to power of 2 for fast modulo via bitmask
        let capacity = min_capacity.max(2).next_power_of_two();
        let mut events = Vec::with_capacity(capacity);
        events.resize_with(capacity, || None);
        Self {
            events,
            read_pos: 0,
            write_pos: 0,
            count: 0,
            capacity,
            mask: capacity - 1,
        }
    }

    /// Create with default capacity (1024 events).
    pub fn with_default_capacity() -> Self {
        Self::new(1024)
    }

    /// Actual capacity (power of 2).
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Check if the buffer is full.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.count == self.capacity
    }

    /// Check if the buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Number of events available to read.
    #[inline]
    pub fn available(&self) -> usize {
        self.count
    }

    /// Space available for writing.
    #[inline]
    pub fn space(&self) -> usize {
        self.capacity - self.count
    }

    /// Try to push an event. Returns Err if buffer is full.
    #[inline]
    pub fn try_push(&mut self, event: StreamingEvent) -> Result<(), StreamingEvent> {
        if self.count == self.capacity {
            return Err(event);
        }
        // SAFETY: write_pos is always < capacity due to masking
        unsafe {
            *self.events.get_unchecked_mut(self.write_pos) = Some(event);
        }
        self.write_pos = (self.write_pos + 1) & self.mask;  // Fast modulo
        self.count += 1;
        Ok(())
    }

    /// Push an event, panicking if full.
    pub fn push(&mut self, event: StreamingEvent) {
        self.try_push(event).expect("EventRing is full");
    }

    /// Pop an event from the front. Returns None if empty.
    #[inline]
    pub fn pop(&mut self) -> Option<StreamingEvent> {
        if self.count == 0 {
            return None;
        }
        // SAFETY: read_pos is always < capacity due to masking
        let event = unsafe { self.events.get_unchecked_mut(self.read_pos).take() };
        self.read_pos = (self.read_pos + 1) & self.mask;  // Fast modulo
        self.count -= 1;
        event
    }

    /// Peek at the front event without removing it.
    #[inline]
    pub fn peek(&self) -> Option<&StreamingEvent> {
        if self.count == 0 {
            return None;
        }
        // SAFETY: read_pos is always < capacity due to masking
        unsafe { self.events.get_unchecked(self.read_pos).as_ref() }
    }

    /// Iterate over available events without consuming them.
    pub fn iter(&self) -> impl Iterator<Item = &StreamingEvent> {
        let mask = self.mask;
        let read_pos = self.read_pos;
        let count = self.count;
        let events = &self.events;
        (0..count).filter_map(move |i| {
            let idx = (read_pos + i) & mask;  // Fast modulo
            events[idx].as_ref()
        })
    }

    /// Clear all events from the buffer.
    /// Fast reset - just resets pointers, doesn't drop individual events.
    pub fn clear(&mut self) {
        // Clear any events that might have data
        for i in 0..self.count {
            let idx = (self.read_pos + i) & self.mask;
            self.events[idx] = None;
        }
        self.read_pos = 0;
        self.write_pos = 0;
        self.count = 0;
    }
}

/// Result of a feed operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedResult {
    /// Number of bytes consumed from input
    pub bytes_consumed: usize,
    /// Number of events written to ring buffer
    pub events_written: usize,
    /// Whether the ring buffer is full (backpressure)
    pub buffer_full: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_slice() {
        let slice = ChunkSlice::new(0, 10, 20);
        assert_eq!(slice.len(), 10);
        assert!(!slice.is_empty());

        let empty = ChunkSlice::new(0, 5, 5);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_chunk_arena() {
        let mut arena = ChunkArena::new();
        assert!(arena.is_empty());

        let idx = arena.push(b"hello world".to_vec());
        assert_eq!(idx, 0);
        assert_eq!(arena.len(), 1);

        let slice = ChunkSlice::new(0, 0, 5);
        assert_eq!(arena.resolve(slice), Some(b"hello".as_slice()));

        let idx2 = arena.push(b"goodbye".to_vec());
        assert_eq!(idx2, 1);
        assert_eq!(arena.len(), 2);
    }

    #[test]
    fn test_event_ring_basic() {
        let mut ring = EventRing::new(4);
        assert!(ring.is_empty());
        assert!(!ring.is_full());
        assert_eq!(ring.available(), 0);
        assert_eq!(ring.space(), 4);

        ring.push(StreamingEvent::ElementEnd { span: Span::new(0, 0) });
        assert_eq!(ring.available(), 1);
        assert_eq!(ring.space(), 3);

        let event = ring.pop();
        assert!(matches!(event, Some(StreamingEvent::ElementEnd { .. })));
        assert!(ring.is_empty());
    }

    #[test]
    fn test_event_ring_wrap() {
        let mut ring = EventRing::new(4);

        // Fill the buffer
        for i in 0..4 {
            ring.push(StreamingEvent::IntegerValue { value: i, span: Span::new(0, 0) });
        }
        assert!(ring.is_full());
        assert_eq!(ring.try_push(StreamingEvent::NilValue { span: Span::new(0, 0) }),
                   Err(StreamingEvent::NilValue { span: Span::new(0, 0) }));

        // Read two
        assert!(matches!(ring.pop(), Some(StreamingEvent::IntegerValue { value: 0, .. })));
        assert!(matches!(ring.pop(), Some(StreamingEvent::IntegerValue { value: 1, .. })));

        // Write two more (wrapping)
        ring.push(StreamingEvent::IntegerValue { value: 100, span: Span::new(0, 0) });
        ring.push(StreamingEvent::IntegerValue { value: 101, span: Span::new(0, 0) });

        // Read all
        assert!(matches!(ring.pop(), Some(StreamingEvent::IntegerValue { value: 2, .. })));
        assert!(matches!(ring.pop(), Some(StreamingEvent::IntegerValue { value: 3, .. })));
        assert!(matches!(ring.pop(), Some(StreamingEvent::IntegerValue { value: 100, .. })));
        assert!(matches!(ring.pop(), Some(StreamingEvent::IntegerValue { value: 101, .. })));
        assert!(ring.is_empty());
    }
}

#[test]
fn test_event_sizes() {
    use std::mem::size_of;
    println!("StreamingEvent: {} bytes", size_of::<StreamingEvent>());
    println!("ChunkSlice: {} bytes", size_of::<ChunkSlice>());
    println!("Span: {} bytes", size_of::<crate::Span>());
    println!("String: {} bytes", size_of::<String>());
}
