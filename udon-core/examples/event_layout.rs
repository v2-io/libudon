//! Analyze StreamingEvent layout to find largest variants.

use udon_core::StreamingEvent;
use udon_core::streaming::ChunkSlice;
use udon_core::Span;

fn main() {
    use std::mem::size_of;

    println!("=== Component Sizes ===");
    println!("ChunkSlice: {} bytes", size_of::<ChunkSlice>());
    println!("Option<ChunkSlice>: {} bytes", size_of::<Option<ChunkSlice>>());
    println!("Span: {} bytes", size_of::<Span>());
    println!("String: {} bytes", size_of::<String>());
    println!("i64: {} bytes", size_of::<i64>());
    println!("f64: {} bytes", size_of::<f64>());
    println!("bool: {} bytes", size_of::<bool>());
    println!();

    println!("=== Estimated Variant Sizes ===");
    // These are rough estimates based on components
    println!("ElementStart: Option<ChunkSlice>(16) + Span(8) = ~24 bytes payload");
    println!("ElementEnd: Span(8) = ~8 bytes payload");
    println!("Attribute: ChunkSlice(12) + Span(8) = ~20 bytes payload");
    println!("Text/Comment: ChunkSlice(12) + Span(8) = ~20 bytes payload");
    println!("IntegerValue: i64(8) + Span(8) = ~16 bytes payload");
    println!("FloatValue: f64(8) + Span(8) = ~16 bytes payload");
    println!("ComplexValue: f64(8) + f64(8) + Span(8) = ~24 bytes payload");
    println!("DirectiveStart: ChunkSlice(12) + Option<ChunkSlice>(16) + Span(8) = ~36 bytes payload");
    println!("InlineDirective: ChunkSlice(12)*2 + Option<ChunkSlice>(16) + Span(8) = ~48 bytes payload");
    println!("Error: String(24) + Span(8) = ~32 bytes payload");
    println!();

    println!("=== Actual Event Size ===");
    println!("StreamingEvent: {} bytes", size_of::<StreamingEvent>());
    println!("The largest variant (InlineDirective ~48 bytes) sets the size");
}
