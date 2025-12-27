# libudon

High-performance streaming UDON parser in Rust.

## What is UDON?

UDON (Universal Document & Object Notation) is a unified notation for documents, data, and configuration. See the [specification](https://github.com/josephwecker/udon) for details.

## Performance

- **813 MiB/s** throughput on comprehensive test files
- **1.83x faster** than previous batch parser implementation
- SIMD-accelerated scanning via memchr
- Zero-copy chunk arena with ring buffer backpressure

## Status

Streaming parser complete with SAX-style event model. See the [implementation roadmap](https://github.com/josephwecker/udon/blob/main/implementation-phase-2.md) for remaining features.

Working:
- Elements with id/classes (`|element[id].class`)
- Inline and indented attributes (`:key value`)
- All value types (strings, integers, floats, booleans, arrays, etc.)
- Comments (`;`)
- Text/prose content
- Basic indentation hierarchy
- Streaming with ring buffer and backpressure

## Structure

```
libudon/
├── udon-core/           # Core parser library
│   └── src/
│       ├── streaming.rs # Ring buffer, chunk arena, StreamingEvent
│       ├── parser.rs    # Generated streaming state machine
│       ├── value.rs     # Scalar value parsing
│       └── span.rs      # Source locations
├── generator/           # Parser generator
│   ├── udon.machine     # State machine DSL
│   ├── genmachine-rs    # Generator script (Ruby)
│   └── templates/       # Liquid templates
└── examples/            # Test fixtures and profiling tools
```

## Building

```bash
cargo build --release
```

## Testing

```bash
cargo test
```

## Benchmarking

```bash
cargo bench --bench parse
```

## Regenerating the Parser

The parser is generated from a declarative `.machine` DSL:

```bash
./generate-parser.sh
```

## Usage (Rust)

```rust
use udon_core::{StreamingParser, StreamingEvent};

let input = b"|article[intro]\n  :author Joseph\n  Hello, world!\n";

// Create parser with event buffer capacity
let mut parser = StreamingParser::new(256);

// Feed input (can be called multiple times for true streaming)
parser.feed(input);
parser.finish();

// Read events
while let Some(event) = parser.read() {
    match &event {
        StreamingEvent::ElementStart { name, span } => {
            if let Some(name_slice) = name {
                let name_bytes = parser.arena().resolve(*name_slice);
                println!("Element: {:?}", name_bytes.map(|b| std::str::from_utf8(b)));
            }
        }
        StreamingEvent::Text { content, span } => {
            let text = parser.arena().resolve(*content);
            println!("Text: {:?}", text.map(|b| std::str::from_utf8(b)));
        }
        StreamingEvent::Attribute { key, span } => {
            let key_bytes = parser.arena().resolve(*key);
            println!("Attribute: {:?}", key_bytes.map(|b| std::str::from_utf8(b)));
        }
        _ => {}
    }
}

// Reuse parser for next document (keeps allocated capacity)
parser.reset();
```

## Related Repositories

- [udon](https://github.com/josephwecker/udon) — Specification and examples
- [udon-ruby](https://github.com/josephwecker/udon-ruby) — Ruby gem with native extension

## License

MIT
