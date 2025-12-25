# libudon

High-performance UDON parser in Rust with C ABI bindings.

## What is UDON?

UDON (Universal Document & Object Notation) is a unified notation for documents, data, and configuration. See the [specification](https://github.com/josephwecker/udon) for details.

## Status

**Phase 2 in progress.** Current implementation covers ~30% of SPEC.md. See the [implementation roadmap](https://github.com/josephwecker/udon/blob/main/implementation-phase-2.md) for the complete plan.

Working:
- Elements with id/classes (`|element[id].class`)
- Inline attributes (`:key value`)
- Comments (`;`)
- Text/prose content
- Basic indentation hierarchy

In progress:
- Indented attributes
- Element suffixes (`?`, `!`, `*`, `+`)
- Embedded elements (`|{...}`)
- Directives (`!name`, `!raw:lang`)
- Streaming with ring buffer
- Tree API

## Structure

```
libudon/
├── udon-core/           # Core parser library
│   └── src/
│       ├── event.rs     # Event enum (hand-written)
│       ├── parser.rs    # Generated state machine
│       ├── value.rs     # Attribute value parsing
│       └── span.rs      # Source locations
├── udon-ffi/            # C ABI bindings
├── generator/           # Parser generator
│   ├── udon.machine     # State machine DSL
│   ├── genmachine-rs    # Generator script (Ruby)
│   └── templates/       # Liquid templates
└── examples/            # Test fixtures
```

## Building

```bash
cargo build --release
```

The shared library will be at `target/release/libudon.{so,dylib,dll}`.

## Testing

```bash
cargo test
```

## Regenerating the Parser

The parser is generated from a declarative `.machine` DSL:

```bash
./generate-parser.sh
```

## Usage (Rust)

```rust
use udon_core::{Parser, Event};

let input = b"|article[intro]\n  :author Joseph\n  Hello, world!\n";
let mut parser = Parser::new(input);
let events = parser.parse();

for event in &events {
    match event {
        Event::ElementStart { name, id, .. } => {
            println!("Element: {:?} id={:?}", name, id);
        }
        Event::Text { content, .. } => {
            println!("Text: {:?}", std::str::from_utf8(content));
        }
        _ => {}
    }
}
```

## Usage (C FFI)

```c
#include "udon.h"

// Create parser
UdonParser* parser = udon_parser_new(input, length);

// Iterate events
while (udon_parser_next(parser)) {
    UdonEvent* event = udon_parser_event(parser);
    switch (event->type) {
        case UDON_ELEMENT_START:
            // event->data.element_start.name, etc.
            break;
        case UDON_TEXT:
            // event->data.content.content, etc.
            break;
    }
}

// Cleanup
udon_parser_free(parser);
```

## Related Repositories

- [udon](https://github.com/josephwecker/udon) — Specification and examples
- [udon-ruby](https://github.com/josephwecker/udon-ruby) — Ruby gem with native extension

## License

MIT
