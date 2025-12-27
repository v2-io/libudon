//! Count event types to understand the workload.

use udon_core::{StreamingParser, StreamingEvent};
use std::collections::HashMap;

fn main() {
    let input = include_bytes!("../../examples/comprehensive.udon");
    let capacity = (input.len() / 50).max(16);

    let mut parser = StreamingParser::new(capacity);
    parser.feed(input);
    parser.finish();

    let mut counts: HashMap<&'static str, usize> = HashMap::new();

    while let Some(event) = parser.read() {
        let name = match event {
            StreamingEvent::ElementStart { .. } => "ElementStart",
            StreamingEvent::ElementEnd { .. } => "ElementEnd",
            StreamingEvent::Attribute { .. } => "Attribute",
            StreamingEvent::Text { .. } => "Text",
            StreamingEvent::Comment { .. } => "Comment",
            StreamingEvent::StringValue { .. } => "StringValue",
            StreamingEvent::QuotedStringValue { .. } => "QuotedStringValue",
            StreamingEvent::IntegerValue { .. } => "IntegerValue",
            StreamingEvent::FloatValue { .. } => "FloatValue",
            StreamingEvent::BoolValue { .. } => "BoolValue",
            StreamingEvent::NilValue { .. } => "NilValue",
            StreamingEvent::ArrayStart { .. } => "ArrayStart",
            StreamingEvent::ArrayEnd { .. } => "ArrayEnd",
            StreamingEvent::RawContent { .. } => "RawContent",
            StreamingEvent::DirectiveStart { .. } => "DirectiveStart",
            StreamingEvent::DirectiveEnd { .. } => "DirectiveEnd",
            StreamingEvent::Interpolation { .. } => "Interpolation",
            StreamingEvent::Error { .. } => "Error",
            _ => "Other",
        };
        *counts.entry(name).or_insert(0) += 1;
    }

    println!("Event counts for comprehensive.udon ({} bytes):\n", input.len());

    let mut sorted: Vec<_> = counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));

    let total: usize = counts.values().sum();
    for (name, count) in sorted {
        println!("  {:20} {:5} ({:5.1}%)", name, count, *count as f64 / total as f64 * 100.0);
    }
    println!("\n  Total: {}", total);
}
