# CLAUDE.md - Agent Guidelines for libudon

This is the core UDON parser library in Rust. It produces a C-compatible shared
library that language bindings (Ruby, Python, etc.) link against.

## Implementation Plan

**See `~/src/udon/implementation-phase-2.md` for the phase-2 roadmap (current parser).**
**See `implementation-phase-3.md` for the parser generator rewrite (in progress on `phase-3-genmachine-rewrite` branch).**
**See `~/src/udon/SPEC.md` for the authoritative UDON specification.**

### Phase 3: parser-gen Rewrite

The `phase-3-genmachine-rewrite` branch contains work on a complete rewrite of the
parser generator system. Key findings from benchmarks:

- **Callback-based parsing is 2-7x faster** than ring-buffer or generator approaches
- True recursive descent (call stack = element stack) is both faster and cleaner
- New DSL spec: `generator/parser-gen.md`
- Target parser definition: `generator/udon.pspec`

See `implementation-phase-3.md` for the full roadmap and rationale.

### Current State (main branch)
- Streaming parser with ring buffer architecture
- comprehensive.udon (15KB): ~30 µs @ 490 MiB/s (with all features enabled)
- 242 tests in udon-core (all passing)
- Embedded elements `|{...}` fully working
- Freeform blocks (```) fully working
- References `@[id]` and `:[id]` fully working
- Interpolation `!{{...}}` in prose/inline content working
- Raw block directives `!:label:` working
- Inline directives `!{name ...}` working
- Block directives `!if`, `!for`, etc. working
- Prose dedentation with content_base tracking
- Pipe-as-text in inline content (` | ` is text, not element)
- FFI code needs updating to use new StreamingEvent API

Deferred features (syntax passes through as literal):
- Interpolation in attribute values: `:href !{{url}}`
- Interpolation in element IDs: `|div[!{{id}}]`

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

## Directive Parsing (Dec 2025 Clarifications)

The parser uses a `raw` flag to distinguish raw vs non-raw directives:

**Block directives:**
- `!:lang:` → DirectiveStart{name: "lang", raw: true}, content as prose until dedent
- `!if condition` → DirectiveStart{name: "if", raw: false}, rest of line is statement,
  then normal UDON children until dedent

**Inline directives:**
- `!{:json: {"key": "val"}}` → raw=true, content is brace-counted opaque bytes
- `!{include |{em content}}` → raw=false, content is parsed as UDON

**Interpolation in typed contexts (attribute values, element IDs):**
- Wholly interpolated: `|div[!{{id}}]` → Interpolation event, type is unparsed
- Concatenated: `|div[prefix_!{{id}}]` → ArrayStart, StringValue, Interpolation, ArrayEnd

See SPEC.md for full details.

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

### Implicit Stack via Function Calls

**Important:** The element/directive stack is implicit via function calls in the DSL,
not an explicit data structure. When you call `/element(COL)` or `/directive(COL)`,
you push onto the call stack. When you `return`, you pop. The column parameter
carries the indentation context.

This means:
- Nesting is handled by recursive function calls
- Dedent detection uses the `elem_col` parameter passed to the function
- `|if[ACTUAL_COL <= elem_col]` checks if we've dedented past the current container
- On dedent, emit the appropriate End event and `return` to pop the stack

Don't create explicit stack data structures for tracking element/directive nesting.

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
- `<LB>` — left brace ({)
- `<RB>` — right brace (})

### Emit Actions

The generator (`genmachine-rs`) maps emit actions in `.machine` to Rust code.
Available emit actions:

| Action | Generated Code |
|--------|----------------|
| `emit(Text)` | `Text { content: self.term(), span }` |
| `emit(Comment)` | `Comment { content: self.term(), span }` |
| `emit(RawContent)` | `RawContent { content: self.term(), span }` |
| `emit(ElementStart)` | `ElementStart { name: Some(self.term()), span }` |
| `emit(ElementStartAnon)` | `ElementStart { name: None, span }` |
| `emit(ElementEnd)` | `ElementEnd { span }` |
| `emit(EmbeddedStart)` | `EmbeddedStart { name: Some(self.term()), span }` |
| `emit(EmbeddedStartAnon)` | `EmbeddedStart { name: None, span }` |
| `emit(EmbeddedEnd)` | `EmbeddedEnd { span }` |
| `emit(Attribute)` | `Attribute { key: self.term(), span }` |
| `emit(Attribute:$id)` | `Attribute { key: "$id", span }` |
| `emit(Attribute:?)` | `Attribute { key: "?", span }` |
| `emit(BoolValue:true)` | `BoolValue { value: true, span }` |
| `emit(StringValue)` | `StringValue { value: self.term(), span }` |
| `emit(TypedValue)` | Calls `emit_typed_value()` for int/float/bool/nil |
| `emit(Interpolation)` | `Interpolation { expression: self.term(), span }` |
| `emit(DirectiveStart)` | `DirectiveStart { name: self.term(), namespace: None, span }` |
| `emit(DirectiveEnd)` | `DirectiveEnd { span }` |
| `emit(Error:name)` | `Error { code: ParseErrorCode::Name, span }` |

**To add a new emit action:**
1. Add the event variant to `StreamingEvent` in `streaming.rs`
2. Add the handler in `genmachine-rs` (around line 975, in `emit_rust` method)
3. Use in `.machine` file
4. Regenerate with `./generate-parser.sh`

### Helper Methods (CALL:method)

For complex or reusable logic, add helper methods to `parser.rs.liquid` and invoke
them via `CALL:method_name` in the `.machine` file.

**Existing helpers:**
- `emit_special_attribute(key)` - Emit attribute with static key (e.g., "$id", "?")
- `emit_pipe_text()` - Emit literal "|" as Text (for prose pipes that aren't elements)
- `emit_typed_value()` - Parse accumulated value and emit Int/Float/Bool/Nil/String

**To add a new helper method:**
1. Add the method to `generator/templates/parser.rs.liquid`
2. Use `CALL:method_name` in `.machine` file (note: no parentheses in DSL)
3. The generator maps `CALL:foo` to `self.foo();`
4. Regenerate with `./generate-parser.sh`

**Example helper (emit_pipe_text in parser.rs.liquid):**
```rust
/// Emit a literal "|" as text. Used when pipe in inline content is not followed
/// by a valid element starter (per SPEC.md:645-651).
fn emit_pipe_text(&mut self) {
    let pipe_bytes = b"|".to_vec();
    let chunk_idx = self.chunks.push(pipe_bytes);
    let pipe_slice = ChunkSlice::new(chunk_idx, 0, 1);
    let span = Span::new(self.global_offset as usize - 1, self.global_offset as usize);
    self.emit(StreamingEvent::Text { content: pipe_slice, span });
}
```

**Important DSL notes:**
- `-> |` advances to next character (REQUIRED before `|return` when matching `}`)
- Without `-> |`, position doesn't advance after matching
- `/element(ACTUAL_COL)` calls the element function with a continuation state
- After a function returns, execution continues at the specified state
- `CALL:method` invokes helper without arguments; for arguments use emit patterns

### Debugging with Trace Mode

The generator supports a `--trace` flag that inserts `eprintln!` statements throughout
the generated parser, showing state transitions, positions, and mark values:

```bash
# Generate parser with trace statements
ruby generator/genmachine-rs --trace generator/udon.machine > udon-core/src/parser.rs

# Run with trace output
cargo run --example debug_interp 2>&1 | grep TRACE
```

Trace output format:
```
TRACE L1147: element/child_prose pos=10 peek=Some(98)
TRACE L1154: child_prose.bang (SCAN) mark=10 pos=17
```

- `L####` - Line number in udon.machine
- `function/state` - Current function and state
- `pos=N` - Current parse position in input
- `peek=Some(N)` - Next byte (or None for EOF)
- `mark=N` - Current mark position (for MARK/TERM accumulation)
- `(SCAN)` - Indicates SCAN-first optimization was used

This is invaluable for debugging parser issues. When something goes wrong,
**look at the DSL lines referenced in the trace** rather than reasoning backwards
from generated Rust code.

**To disable trace mode:** Regenerate without the flag:
```bash
./generate-parser.sh  # or: ruby generator/genmachine-rs generator/udon.machine > ...
```

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

Current benchmarks (main branch, full feature set):

| Test | Time | Throughput |
|------|------|------------|
| comprehensive.udon (15KB) | ~28 µs | ~520 MiB/s |
| minimal.udon (52 bytes) | ~105 ns | ~470 MiB/s |
| mathml-to-latex.udon (112KB) | ~189 µs | ~565 MiB/s |

Key optimizations:
- Fused eof+peek pattern: single `match self.peek()` replaces separate `if eof()` + `if let peek()`
- SCAN-first with memchr for SIMD scanning (39 states optimized)
- Brace symbols `<LB>` and `<RB>` for embedded content SCAN
- Cached chunk pointer (avoids arena lookup in hot path)
- Power-of-2 ring buffer with bitmask modulo
- ParseErrorCode enum instead of String
- Boxed InlineDirective to reduce event size (48 → 40 bytes)

## What Needs Implementation (TDD)

Tests exist for all these features; implement to make tests pass.

### Parser Features (in udon.machine)
| Feature | Status | Notes |
|---------|--------|-------|
| Embedded elements `\|{...}` | DONE | 26/26 tests pass |
| Pipe-as-text ` \| ` | DONE | Pipes not followed by element starters are text |
| Freeform blocks ``` | DONE | 5/5 tests pass, proper content boundaries |
| References `@[id]`, `:[id]` | DONE | 4/4 tests pass, IdReference and AttributeMerge events |
| Prose dedentation | DONE | 14/15 tests pass (1 depends on freeform in dynamics context) |
| Inline comments `;{...}` | DONE | With brace-counting |
| Suffix handling | DONE | `?!*+` work in all positions |
| Double-brace interpolation `!{{...}}` | PARTIAL | 7/13 tests pass - works in prose/inline, attr/id pending |
| Inline directives `!{name ...}` | TODO | Tests are placeholders |
| Block directives (`!if`, `!for`) | TODO | Tests are placeholders |
| Raw block `!:lang:` | TODO | Tests are placeholders |
| Raw inline `!{:kind: ...}` | TODO | Tests are placeholders |

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
