# Implementation Phase 3: genmachine-v4 Rewrite

This phase involves a complete, principled rewrite of the parser generator
system based on lessons learned and proof-of-concept benchmarks.

## Background

### What We Learned

1. **The original C-era genmachine was elegant** (104 lines, 6 functions, 14 states)
   - Functions returned typed values
   - Explicit MARK/TERM for accumulation
   - Recursion matched grammar structure

2. **The Rust port lost that elegance** (1,901 lines, 231 states)
   - Flattened recursion into giant state machine
   - Invented `emit()` abstraction that hid operations
   - Lost function return values
   - Backpressure handling was broken (silently dropped events)

3. **Callback-based parsing is 2-7x faster than alternatives**
   - True recursive descent compiles to efficient code
   - No buffering overhead
   - No iterator protocol dispatch
   - See `generator/genmachine-v4.md` and `gen-parser-poc/` for details

## Goals

1. **Restore elegance** - DSL describes what, generator figures out how
2. **True recursive descent** - call stack IS the element stack
3. **Callback-based output** - 2+ GiB/s throughput
4. **Type-driven emit** - return types determine events, not explicit emit()
5. **Inferred optimizations** - EOF handling, SCAN from state structure
6. **Stable Rust** - no nightly features required

## Architecture

### DSL (udon-v4.rmachine)

See `generator/genmachine-v4.md` for full specification.

Key features:
- Type declarations: `|type[Element] BRACKET`
- Inferred EOF via `EXPECTS(x)` annotation
- Single-line `|if[cond]` guards (no endif)
- Inline literals: `TypeName(literal)`, `TypeName(USE_MARK)`
- Automatic SCAN inference from state structure

### Generator (genmachine-v4)

Input: `.rmachine` file (which is valid UDON!)
Output: Rust parser with callback-based API

The generator will:
1. Parse the .rmachine file (can use UDON parser - bootstrap potential!)
2. Build internal representation of functions, states, types
3. Generate Rust code with true recursive descent
4. Emit callback invocations based on return types

### Generated Parser

```rust
pub struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
    mark_pos: usize,
    // ... minimal state
}

impl<'a> Parser<'a> {
    pub fn parse<F>(mut self, on_event: F)
    where
        F: FnMut(Event<'a>)
    {
        self.parse_document(&on_event);
    }

    fn parse_element<F>(&mut self, elem_col: i32, on_event: &mut F) {
        on_event(Event::ElementStart { .. });
        // ... true recursive calls for children ...
        on_event(Event::ElementEnd { .. });
    }
}
```

## Implementation Plan

### Step 1: DSL Parser

Write a parser for the .rmachine DSL. Options:
- Use existing UDON parser (bootstrap!)
- Write a simple hand-coded parser
- Use nom/pest for quick iteration

Output: Internal representation of machine definition.

### Step 2: Code Generator

Write the Rust code generator:
- Emit struct definition
- Emit helper methods (peek, advance, mark, term, etc.)
- Emit parse functions based on DSL functions
- Handle type-based event emission
- Infer SCAN optimizations

### Step 3: Template vs Direct Generation

Decision point:
- **Template (Liquid)**: Easier to read/modify, current approach
- **Direct codegen**: More control, no template dependency

Recommendation: Start with direct codegen for v4, keep it simple.

### Step 4: Event Types

Define the Event enum based on type declarations:
- BRACKET types → Start/End variants
- CONTENT types → single variant with content
- Error type for parse errors

### Step 5: SIMD Integration

For SCAN optimization:
- Detect self-looping states with specific char matches
- Generate memchr calls for bulk scanning
- Keep `->[\n]` syntax for explicit advance-to

### Step 6: Testing

- Port existing streaming tests to callback API
- Verify event sequence matches current parser
- Benchmark against current implementation

### Step 7: Migration

- Keep current parser as fallback during transition
- Update FFI bindings for new callback API
- Update language bindings (Ruby, Python)

## File Structure

```
generator/
├── genmachine-v4.md      # DSL specification (done)
├── genmachine-v4/        # New generator implementation
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs       # CLI entry point
│   │   ├── parser.rs     # DSL parser
│   │   ├── ir.rs         # Internal representation
│   │   ├── codegen.rs    # Rust code generation
│   │   └── scan.rs       # SCAN optimization inference
│   └── templates/        # Optional templates if needed
├── udon-v4.rmachine      # New UDON machine definition
└── _archive/
    ├── genmachine-rs     # Current generator (Ruby)
    └── udon.machine      # Current machine definition
```

## Success Criteria

1. **Correctness**: All existing tests pass with new parser
2. **Performance**: ≥2 GiB/s throughput (current is ~500 MiB/s)
3. **Code size**: Generated parser < 1000 lines (current is ~3000)
4. **DSL size**: Machine definition < 500 lines (current is ~1900)
5. **Elegance**: True recursive descent, clear event flow

## Risks and Mitigations

### Risk: Bootstrap complexity
If using UDON to parse .rmachine, need working parser first.
**Mitigation**: Start with hand-coded DSL parser, bootstrap later.

### Risk: FFI compatibility
Callback-based API different from ring-buffer API.
**Mitigation**: Provide adapter that buffers events for FFI consumers.

### Risk: Deep recursion stack overflow
Very deep nesting could overflow stack.
**Mitigation**: UDON documents rarely exceed 50-100 levels; monitor in practice.

## Timeline

This is a complete rewrite, not incremental improvement. Estimated phases:

1. DSL parser + IR: Foundation work
2. Basic codegen: Get something generating
3. Full codegen + SCAN: Complete feature set
4. Testing + benchmarking: Verify correctness and performance
5. Migration: Update bindings and documentation

## Notes

- The .rmachine DSL is valid UDON - beautiful self-describing property
- Callback-based design enables future async support without redesign
- True recursion means the compiler does optimization work for us
- This is consciousness infrastructure - build it worthy of study
