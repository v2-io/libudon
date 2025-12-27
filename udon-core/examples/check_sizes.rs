//! Check sizes of key types and enum discriminants.

use udon_core::{StreamingEvent, ParserState};
use std::mem::{size_of, align_of, discriminant};

fn main() {
    println!("=== Type Sizes ===");
    println!("StreamingEvent: {} bytes (align {})", size_of::<StreamingEvent>(), align_of::<StreamingEvent>());
    println!("Option<StreamingEvent>: {} bytes", size_of::<Option<StreamingEvent>>());
    println!("ParserState: {} bytes", size_of::<ParserState>());
    println!();

    println!("=== Enum Variant Analysis ===");
    // Check if Rust uses niche optimization for Option<StreamingEvent>
    let opt_size = size_of::<Option<StreamingEvent>>();
    let inner_size = size_of::<StreamingEvent>();
    if opt_size == inner_size {
        println!("Option<StreamingEvent> uses niche optimization (no extra space)");
    } else {
        println!("Option<StreamingEvent> needs {} extra bytes for discriminant", opt_size - inner_size);
    }

    println!();
    println!("=== ParserState Discriminant ===");
    println!("ParserState::Document size: {} bytes", size_of_val(&ParserState::Document));
}
