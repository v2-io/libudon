# CLAUDE.md - Agent Guidelines for libudon

This is the core UDON parser library in Rust. It produces a C-compatible shared
library that language bindings (Ruby, Python, etc.) link against.

## Implementation Plan

**See `~/src/udon/implementation-phase-2.md` for the comprehensive roadmap.**
**See `~/src/udon/SPEC.md` for the authoritative UDON specification.**

Current state (streaming-parser branch):
- Streaming parser with ring buffer architecture
- 1.83x faster than old batch parser (17.9 µs vs 32.8 µs for comprehensive.udon)
- 238 tests in udon-core (142 passing, 96 TDD placeholders for unimplemented features)
- FFI code needs updating to use new StreamingEvent API

## Unified Inline Syntax (NEW - Dec 2025)

All prefixes support bracket-delimited inline forms:

| Syntax | Description |
|--------|-------------|
| `\|{element ...}` | Embedded element |
| `!{{expr}}` | Interpolation (double-brace) |
| `!{directive ...}` | Inline directive |
| `;{comment}` | Inline comment |

Key rules:
- **Bracket mode stays bracket mode**: Inside `|{...}`, use `|{nested}` not `|nested`
- **Brace-counting**: All inline forms use brace-counting for balanced `{}`
- **Parser emits comments**: Comments are events, consumer decides to keep/strip

## Critical: Use SPEC.md as Authority

The UDON specification lives at `~/src/udon/SPEC.md`. This is the **only**
authoritative source for syntax rules.

Do NOT use:
- `generator/_archive/` files (old C-era, outdated)
- Guesses about "obvious" behavior
- Other UDON-like formats as reference

When SPEC.md is ambiguous, ask Joseph for clarification.

## Architecture

```
libudon/
├── udon-core/           # Core parser library
│   └── src/
│       ├── lib.rs       # Public API exports
│       ├── streaming.rs # Ring buffer, ChunkArena, StreamingEvent
│       ├── parser.rs    # Generated streaming state machine
│       ├── event.rs     # LEGACY Event enum (to be removed)
│       ├── value.rs     # Scalar value parsing
│       └── span.rs      # Source locations
├── udon-ffi/            # C ABI bindings (NEEDS UPDATE)
│   └── src/lib.rs       # Currently broken, uses old Event API
├── generator/           # Parser generator
│   ├── udon.machine     # State machine definition (edit this)
│   ├── genmachine-rs    # Generator script (Ruby)
│   ├── templates/       # Liquid templates for code generation
│   │   └── parser.rs.liquid  # Parser template
│   └── _archive/        # Old C-era files (not authoritative)
├── examples/            # Test fixtures
└── generate-parser.sh   # Regenerates parser.rs from udon.machine
```

## Streaming Parser Architecture

The parser uses a streaming/SAX-style architecture:

```
Input chunks → StreamingParser → EventRing ← Consumer reads
                    ↓
              ChunkArena
           (manages chunks)
```

### Key Components

**ChunkArena** - Stores input chunks with reference tracking
- Chunks pushed via `parser.feed(chunk)`
- Synthetic chunks created for cross-chunk tokens
- Chunks can be freed when no events reference them

**EventRing** - Fixed-size ring buffer for events
- Power-of-2 sizing for fast modulo via bitmask
- Backpressure when full (feed returns `buffer_full: true`)
- Consumer pops events with `parser.read()`

**ChunkSlice** (12 bytes) - Reference to data in arena
```rust
struct ChunkSlice {
    chunk_idx: u32,  // Index into arena
    start: u32,      // Byte offset
    end: u32,        // Byte offset (exclusive)
}
```

**StreamingEvent** (40 bytes) - SAX-style event enum
- Uses ChunkSlice for string data (not borrowed references)
- Error variant uses ParseErrorCode enum (not String)
- InlineDirective is boxed to reduce enum size

### API Usage

```rust
let mut parser = StreamingParser::new(1024);  // event buffer capacity

// Feed chunks (can be called multiple times for true streaming)
parser.feed(b"|div Hello World\n");
parser.finish();

// Read events
while let Some(event) = parser.read() {
    // Resolve ChunkSlice to bytes
    if let StreamingEvent::Text { content, .. } = &event {
        let bytes = parser.arena().resolve(*content);
    }
}

// Reuse parser (keeps allocated capacity)
parser.reset();
```

## StreamingEvent vs Event (LEGACY)

**StreamingEvent** (streaming.rs) - CURRENT, used by parser
- Uses ChunkSlice for string references
- ParseErrorCode enum for errors
- SAX-style: id/classes/suffixes emit as separate Attribute events
- Attribute values emit as separate value events

**Event** (event.rs) - LEGACY, to be removed
- Uses `&'a [u8]` borrowed references
- Only exists for API compatibility
- Parser no longer produces these events
- FFI code incorrectly tries to use this

## Workflow: Modifying the Parser

1. **Edit `generator/udon.machine`** — The declarative state machine DSL
2. **Regenerate**: `./generate-parser.sh`
3. **Build**: `cargo build`
4. **Test**: `cargo test`

Do NOT edit `udon-core/src/parser.rs` directly — it's generated.

## The .machine DSL

The state machine DSL uses a pipe-delimited format:

```
|function[name]
  |state[:state_name] SCAN(\n;<P>)    ; Optional SCAN-first optimization
    |c[chars]   |.label  | actions               |>> :next_state
    |default    |.label  | actions               |>> :next_state
    |eof        |        | emit(Error:unclosed)  |return
```

Key patterns:
- `|c[x]` — Match character 'x'
- `|c[\n]` — Match newline
- `|c[ \t]` — Match space or tab
- `|default` — Fallback case
- `|eof` — End of input (REQUIRED for every state to avoid infinite loops)
- `|>> :state` — Transition to state
- `|return` — Return from function
- `| emit(EventType)` — Emit an event
- `| emit(Error:error_name)` — Emit error with ParseErrorCode
- `| MARK` — Mark current position
- `| TERM` — Terminate slice (MARK to current)

### SCAN-first Optimization

For content-scanning states, add SCAN to the state line:
```
|state[:prose] SCAN(\n;<P>)
```

This generates SIMD-accelerated memchr scanning. Characters:
- `\n` — newline
- `<P>` — pipe (|)
- `<BS>` — backslash
- `<L>` — left bracket ([)
- `<R>` — right bracket (])

## Key Files

| File | Purpose | Edit? |
|------|---------|-------|
| `generator/udon.machine` | State machine definition | YES |
| `generator/templates/parser.rs.liquid` | Code gen template | YES (carefully) |
| `udon-core/src/parser.rs` | Generated parser | NO (regenerate) |
| `udon-core/src/streaming.rs` | Ring buffer, events | YES |
| `udon-core/src/event.rs` | LEGACY events | REMOVE |
| `udon-core/src/value.rs` | Value type parsing | YES |

## Performance Characteristics

Current benchmarks (streaming-parser branch):

| Test | Time | Throughput | vs Old Parser |
|------|------|------------|---------------|
| comprehensive.udon (15KB) | 17.9 µs | 813 MiB/s | 1.83x faster |
| minimal.udon (52 bytes) | 73.5 ns | 700 MiB/s | 1.53x faster |

Key optimizations:
- SCAN-first with memchr for SIMD scanning
- Cached chunk pointer (avoids arena lookup in hot path)
- Power-of-2 ring buffer with bitmask modulo
- ParseErrorCode enum instead of String
- Boxed InlineDirective to reduce event size (48 → 40 bytes)

## What Needs Implementation (TDD)

Tests exist for all these features; implement to make tests pass.

### Parser Features (in udon.machine)
| Feature | Tests | Priority |
|---------|-------|----------|
| Embedded elements `\|{...}` | 20 tests | HIGH |
| Indentation edge cases | 15 tests | HIGH |
| Interpolation `!{{...}}` | 13 tests | MEDIUM |
| Block directives (`!if`, `!for`) | 16 tests | MEDIUM |
| Inline comments `;{...}` | 7 tests | MEDIUM |
| Raw block `!raw:lang` | 6 tests | MEDIUM |
| Raw inline `!{raw:kind ...}` | 5 tests | LOW |
| Freeform blocks ``` | 3 tests | LOW |
| References `@[id]`, `:[id]` | 2 tests | LOW |

### FFI (udon-ffi/src/lib.rs) - BROKEN
The FFI code uses the old Event enum and deprecated Parser API:
- Uses `Event` fields that don't exist (id, classes, suffix, value)
- Calls deprecated `Parser::new()` and `parser.parse()` (which panic)
- Needs complete rewrite to use StreamingParser + StreamingEvent

### Legacy Code to Remove
- `Parser<'a>` struct in parser.rs (lines ~2951-2969) - deprecated, panics
- `Event<'a>` enum in event.rs - no longer used by parser
- README.md examples - show old API

## Testing

```bash
cargo test                    # All tests
cargo test --lib              # Unit tests only
cargo test parsing            # Tests matching "parsing"
cargo bench --bench parse     # Benchmarks
```

Test files:
- `udon-core/tests/streaming.rs` - Streaming parser tests
- `udon-core/tests/parsing.rs` - Integration tests

## Related Repositories

- `~/src/udon` — Specification, examples, implementation plan
- `~/src/udon-ruby` — Ruby gem using this library (needs FFI update)
