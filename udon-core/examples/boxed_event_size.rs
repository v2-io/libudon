//! Test if boxing large variants reduces enum size.
//! These types are only used for size measurement via size_of.

#![allow(dead_code)]

use std::mem::size_of;

// Simulated types
#[derive(Clone, Copy)]
struct ChunkSlice { _data: [u8; 12] }
#[derive(Clone, Copy)]
struct Span { _data: [u8; 8] }

// Current layout (unboxed)
enum EventUnboxed {
    // Small variants
    ElementEnd { span: Span },
    Text { content: ChunkSlice, span: Span },
    Attribute { key: ChunkSlice, span: Span },

    // Large variants
    InlineDirective {
        name: ChunkSlice,
        namespace: Option<ChunkSlice>,
        content: ChunkSlice,
        span: Span,
    },
    Error { message: String, span: Span },
}

// Boxed layout
struct InlineDirectiveData {
    name: ChunkSlice,
    namespace: Option<ChunkSlice>,
    content: ChunkSlice,
    span: Span,
}

enum EventBoxed {
    // Small variants (same)
    ElementEnd { span: Span },
    Text { content: ChunkSlice, span: Span },
    Attribute { key: ChunkSlice, span: Span },

    // Large variants boxed
    InlineDirective(Box<InlineDirectiveData>),
    Error { message: Box<str>, span: Span },
}

fn main() {
    println!("=== Unboxed Event ===");
    println!("EventUnboxed: {} bytes", size_of::<EventUnboxed>());
    println!("Option<EventUnboxed>: {} bytes", size_of::<Option<EventUnboxed>>());

    println!();
    println!("=== Boxed Event ===");
    println!("EventBoxed: {} bytes", size_of::<EventBoxed>());
    println!("Option<EventBoxed>: {} bytes", size_of::<Option<EventBoxed>>());

    println!();
    println!("=== Savings ===");
    let unboxed = size_of::<EventUnboxed>();
    let boxed = size_of::<EventBoxed>();
    println!("Per event: {} bytes saved ({}% reduction)",
             unboxed - boxed,
             (unboxed - boxed) * 100 / unboxed);

    // With 512 events per parse:
    let events = 512;
    println!("Per parse ({} events): {} bytes saved",
             events, events * (unboxed - boxed));
}
