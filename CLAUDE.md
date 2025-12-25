# CLAUDE.md - Agent Guidelines for libudon

This is the core UDON parser library in Rust. It produces a C-compatible shared
library that language bindings (Ruby, Python, etc.) link against.

## Implementation Plan

**See `~/src/udon/implementation-phase-2.md` for the comprehensive roadmap.**

Current state: ~30% of SPEC.md implemented. Phase 2 focuses on:
- Complete parser (all SPEC.md features)
- True streaming with ring buffer
- Arena-allocated tree structure
- World-class error messages

## Architecture

```
libudon/
├── udon-core/        # Core parser, no_std compatible
│   └── src/
│       ├── lib.rs
│       ├── event.rs  # Event enum (hand-written, stable)
│       ├── span.rs   # Location types
│       ├── value.rs  # Attribute value parsing
│       └── parser.rs # Generated from .machine DSL
├── udon-ffi/         # C ABI exports (cdylib)
│   └── src/lib.rs
└── generator/        # Code generator
    ├── genmachine-rs # Ruby script that generates parser.rs
    ├── udon.machine  # Current parser state machine (authoritative)
    ├── _archive/     # Old C-era machine files (not authoritative)
    └── templates/
        └── parser.rs.liquid
```

## Generator Pipeline

The parser is generated from a declarative `.machine` DSL:

```bash
cd generator && ruby genmachine-rs udon.machine > ../udon-core/src/parser.rs
```

This enables grammar tuning without hand-editing parser code.

## Key Design Decisions

1. **Zero-copy**: Events contain `&'a [u8]` slices into input buffer
2. **SIMD scanning**: Uses `memchr` crate for fast character search
3. **Event streaming**: Parser emits events without building AST
4. **C ABI**: `udon-ffi` exports stable C interface for FFI

## Performance Targets

- Parsing: >1 GiB/s for well-formed input
- Memory: O(depth) stack, no heap allocation per event
- FFI: Minimal overhead, batch JSON for scripting languages

## Building

```bash
cargo build --release
```

## Testing

```bash
cargo test
```

## Specification

The UDON specification lives in a separate repo. See:
https://github.com/josephwecker/udon

When in doubt about syntax rules, consult SPEC.md in that repo.
