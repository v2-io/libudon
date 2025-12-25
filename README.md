# libudon

High-performance UDON parser in Rust with C ABI bindings.

## Overview

This library provides:

- **udon-core**: Zero-copy streaming parser emitting events
- **udon-ffi**: C-compatible shared library for FFI bindings

## Building

```bash
cargo build --release
```

The shared library will be at `target/release/libudon.{so,dylib,dll}`.

## Performance

- Pure Rust parsing: ~1.3 GiB/s
- Uses SIMD-accelerated scanning via `memchr`
- Zero-copy event emission

## C API

```c
#include "udon.h"

// Create parser
UdonParser* parser = udon_parser_new(input, length);

// Iterate events
while (udon_parser_next(parser)) {
    UdonEvent* event = udon_parser_event(parser);
    // Process event...
}

// Or batch parse to JSON
char* json = udon_parse_json(input, length);
// Use json...
udon_free_json(json);

// Cleanup
udon_parser_free(parser);
```

## License

MIT
