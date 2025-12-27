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

### Initial Implementation

| Test | Old Parser | Streaming | Overhead |
|------|------------|-----------|----------|
| comprehensive.udon (15KB) | 32.8 µs / 444 MiB/s | 40.0 µs / 365 MiB/s | +22% |
| minimal.udon (52 bytes) | 112.6 ns / 457 MiB/s | 219.7 ns / 234 MiB/s | +95% |

### Hot Path Fix: Cached Chunk Pointer

**Problem identified:** Every `peek()` and `eof()` call went through arena lookup:
```rust
// SLOW: 3 levels of indirection per character
fn peek(&self) -> Option<u8> {
    self.current_chunk_data().get(self.pos).copied()
}
fn current_chunk_data(&self) -> &[u8] {
    self.chunks.get(self.current_chunk)  // Vec lookup
        .map(|c| c.data())                // method call
        .unwrap_or(&[])                   // Option unwrap
}
```

**Fix:** Cache raw pointer to current chunk data:
```rust
struct StreamingParser {
    current_ptr: *const u8,  // Cached on feed()
    current_len: usize,
    // ...
}

#[inline(always)]
fn peek(&self) -> Option<u8> {
    if self.pos < self.current_len {
        Some(unsafe { *self.current_ptr.add(self.pos) })
    } else {
        None
    }
}
```

**Results after fix:**

| Test | Before Fix | After Fix | Improvement |
|------|------------|-----------|-------------|
| comprehensive.udon | 40.0 µs / 365 MiB/s | 39.2 µs / 372 MiB/s | +2% |
| minimal.udon | 219.7 ns / 234 MiB/s | 203.5 ns / 253 MiB/s | +8% |

Still ~19% slower than old parser on large files, ~80% on small files.

### Ring Buffer Optimization: Power-of-2 Bitmask

**Problem identified:** Ring buffer `try_push()` had expensive modulo operation:
```rust
// SLOW: modulo is expensive, and capacity() computed mask + 1
self.write_pos = (self.write_pos + 1) % self.capacity();
```

**Fix:** Power-of-2 sizing with bitmask and stored capacity:
```rust
struct EventRing {
    events: Vec<Option<StreamingEvent>>,
    read_pos: usize,
    write_pos: usize,
    count: usize,
    capacity: usize,  // Stored directly for fast full check
    mask: usize,      // capacity - 1, for fast wrap
}

#[inline]
pub fn try_push(&mut self, event: StreamingEvent) -> Result<(), StreamingEvent> {
    if self.count == self.capacity {  // Direct field access
        return Err(event);
    }
    unsafe {
        *self.events.get_unchecked_mut(self.write_pos) = Some(event);
    }
    self.write_pos = (self.write_pos + 1) & self.mask;  // Bitmask, not modulo
    self.count += 1;
    Ok(())
}
```

**Results after fix:**

| Test | Before Bitmask | After Bitmask | Improvement | vs Old Parser |
|------|----------------|---------------|-------------|---------------|
| comprehensive.udon | 39.2 µs / 372 MiB/s | 35.9 µs / 407 MiB/s | +9% | **+9% overhead** |
| minimal.udon | 203.5 ns / 253 MiB/s | 202 ns / 254 MiB/s | ~same | +79% (fixed costs) |

The comprehensive file overhead dropped from 22% to just 9%. Small files still dominated by fixed allocation costs.

### Remaining Overhead Sources

1. **`feed()` copies input** - `chunk.to_vec()` copies all bytes into arena
2. **Fixed allocation costs** - arena, ring buffer, element stack (dominates small files)
3. **StreamingEvent size** - 48 bytes vs Event 64 bytes (actually smaller, not an issue)

### Next: Zero-Copy Feed

For single-chunk parsing (common case), we can avoid the copy by storing
a borrowed reference directly. The arena is only needed when:
- Multiple chunks are fed (true streaming)
- Events need to outlive the input

## Optimization Opportunities

1. **Zero-copy feed** - Borrow input directly for single-chunk case
2. **Parser reuse** - Amortize allocation costs across multiple parses
3. **Pre-allocated ring buffer** - Avoid per-parse allocation
4. **SIMD scanning** - Fast search for special characters (memchr)

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
