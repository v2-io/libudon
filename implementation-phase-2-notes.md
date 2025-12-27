# Streaming Parser Implementation Notes

These notes capture architectural decisions made during the streaming parser implementation on the `streaming-parser` branch.

## Chunk Boundary Handling

### The Problem

Tokens can span chunk boundaries:
```
Chunk 1: "|elem"
Chunk 2: "ent-name :attr..."
```

Long content can span many chunks:
```
Chunk 1: "Very long prose..."     (64KB)
Chunk 2: "...continues..."        (64KB)
Chunk 3: "...until newline\n"     (64KB)
```

### The Solution: Hybrid Approach

**Short tokens (identifiers, keys, values): Copy-on-span**
- Most tokens fit within a single chunk
- Only tokens crossing boundaries pay the copy cost
- Concatenated bytes go into arena as synthetic chunk
- ChunkSlice stays simple (single chunk reference, 12 bytes)

**Long content (prose, raw blocks): SAX-style incremental emission**
- Emit Text events at chunk boundaries, don't accumulate
- Consumer concatenates consecutive Text events if needed
- This is how XML SAX parsers handle large text nodes
- Prevents unbounded memory accumulation

```rust
// Long prose spanning 3 chunks emits 3 events:
Text { content: slice_chunk1, span: ... }  // ends at chunk boundary
Text { content: slice_chunk2, span: ... }  // ends at chunk boundary
Text { content: slice_chunk3, span: ... }  // ends at newline
```

### Partial Token Buffer

For short tokens that span chunks:

```rust
struct PartialToken {
    /// Bytes from end of previous chunk
    bytes: Vec<u8>,
    /// Parser state context (what we were parsing)
    context: PartialContext,
}
```

When token completes:
1. Concatenate partial.bytes + new chunk bytes
2. Push as synthetic chunk to arena
3. Emit event with ChunkSlice pointing to synthetic chunk

### Why Not Multi-Chunk ChunkSlice?

Considered: `ChunkSlice` referencing multiple chunks `[(chunk0, range), (chunk1, range)]`

Rejected because:
- Complicates slice resolution
- Complicates arena cleanup (can't free chunk0 until all multi-chunk slices consumed)
- Most tokens don't span chunks anyway
- Copy cost is acceptable for the rare spanning case

## Benchmark Results (2024-12-26)

First implementation comparison:

| Test | Old Parser | Streaming | Overhead |
|------|------------|-----------|----------|
| comprehensive.udon (15KB) | 32.8 µs / 444 MiB/s | 40.0 µs / 365 MiB/s | +22% |
| minimal.udon (52 bytes) | 112.6 ns / 457 MiB/s | 219.7 ns / 234 MiB/s | +95% |

Overhead sources:
- ChunkArena allocation
- Copying input bytes into arena (could be ref-counted later)
- Ring buffer setup

Small input overhead is fixed costs; amortizes with parser reuse.

## Optimization Opportunities

1. **Parser reuse** - Amortize allocation costs
2. **Reference-counted chunks** - Avoid copying in feed()
3. **Pre-allocated ring buffer** - Avoid per-parse allocation
4. **SIMD scanning** - Fast search for special characters

## Architecture Decisions

### ChunkSlice (12 bytes)
```rust
struct ChunkSlice {
    chunk_idx: u32,  // Index into arena
    start: u32,      // Byte offset
    end: u32,        // Byte offset (exclusive)
}
```

Smaller than fat pointer (16 bytes), enables chunk cleanup.

### StreamingEvent vs Event

- `Event<'a>` - Borrows from input, zero-copy, single-parse lifetime
- `StreamingEvent` - Uses ChunkSlice, can outlive input chunks (with arena)

### Ring Buffer Backpressure

When ring buffer is full, `feed()` returns with `buffer_full: true`.
Consumer must `read()` events before more parsing can proceed.
This prevents unbounded memory growth.
