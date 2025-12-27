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

### Right-Sized Buffer Capacity

**Problem:** Fixed 1024-slot ring buffer for all inputs wasted memory and time initializing.

**Fix:** Size buffer based on input length:
```rust
fn estimate_capacity(input_len: usize) -> usize {
    (input_len / 50).max(16)  // EventRing rounds up to power of 2
}
```

### Parser Reuse with reset()

**Problem:** Allocating parser on every parse dominated small file times.

**Fix:** Add `reset()` method that clears state but keeps allocated capacity:
```rust
pub fn reset(&mut self) {
    self.state = ParserState::Document;
    self.chunks.clear();    // Keeps capacity
    self.events.clear();    // Keeps capacity
    // ... reset other fields
}
```

**Results with parser reuse:**

| Test | Before | After | vs Old Parser |
|------|--------|-------|---------------|
| comprehensive.udon | 32.9 µs / 443 MiB/s | 32.7 µs / 446 MiB/s | ~same |
| minimal.udon | 180 ns / 285 MiB/s | **113 ns / 454 MiB/s** | ~same |

Parser reuse eliminated all allocation overhead.

### SIMD Scanning with memchr

**Problem:** Character-by-character scanning in prose states is slow.

**Solution:** Use memchr for bulk SIMD-accelerated scanning:
```rust
fn scan_prose(&mut self) -> Option<u8> {
    let remaining = unsafe {
        std::slice::from_raw_parts(
            self.current_ptr.add(self.pos),
            self.current_len - self.pos
        )
    };
    match memchr::memchr3(b'\n', b';', b'|', remaining) {
        Some(offset) => {
            self.pos += offset;
            self.column += offset as u32;
            self.global_offset += offset as u64;
            Some(remaining[offset])
        }
        None => { /* EOF handling */ }
    }
}
```

Applied to:
- `SProse`, `SChildProse`, `SInlineText` - scan until `\n`, `;`, `|`
- `SLineComment`, `SBlockComment`, `SEscapedText`, `SChildEscapedText` - scan until `\n`

**Note:** Manual state optimization is not sustainable. Need to automate SIMD emission via generator.

### SCAN-First DSL Command (2024-12-27)

**Problem:** Manual SIMD edits get overwritten when regenerating parser.

**Solution:** Added generic `SCAN(chars)` DSL command and SCAN-first state optimization:

```
; DSL syntax - SCAN on state line triggers SCAN-first mode
|state[:prose] SCAN(\n;<P>)
  |eof         | emit(Text) |return
  |c[\n]       | emit(Text) |>> :start
  |c[;]        | emit(Text) | -> |>> :check_inline_comment
  |c[<P>]      | emit(Text) | -> |>> /element :start
```

**Generator emits:**
```rust
State::SProse => {
    // SCAN-first: bulk scan and match result
    match self.scan_to3(b'\n', b';', b'|') {
        Some(b'\n') => { emit(Text); state = State::SStart; }
        Some(b';') => { emit(Text); advance(); state = State::SCheckInlineComment; }
        Some(b'|') => { emit(Text); advance(); parse_element(); state = State::SStart; }
        None => { emit(Text); return; }
        _ => {}
    }
}
```

**Template provides `scan_to1/2/3` methods:**
```rust
fn scan_to3(&mut self, b1: u8, b2: u8, b3: u8) -> Option<u8> {
    let remaining = unsafe {
        std::slice::from_raw_parts(self.current_ptr.add(self.pos), self.current_len - self.pos)
    };
    match memchr::memchr3(b1, b2, b3, remaining) {
        Some(offset) => {
            self.pos += offset;
            self.column += offset as u32;
            self.global_offset += offset as u64;
            Some(remaining[offset])
        }
        None => { /* advance to EOF */ None }
    }
}
```

**States using SCAN-first:**
- Document: `prose`, `escaped_text`, `line_comment`, `block_comment`
- Element: `inline_text`, `elem_line_comment`
- Children: `child_prose`, `child_escaped_text`, `child_line_comment`, `child_block_comment`
- Attributes: `attr_dquote_content`, `attr_squote_content`, `attr_bare`
- Inline attrs: `inline_attr_dquote_content`, `inline_attr_squote_content`
- Identity: `id_bracket_value`, `id_quoted_name_content`, `id_class_quoted_content`

**Final Results (2024-12-27):**

| Test | Old Parser | Streaming + SCAN-first | Speedup |
|------|------------|------------------------|---------|
| comprehensive.udon | 32.8 µs / 444 MiB/s | **18.7 µs / 778 MiB/s** | **1.75x** |
| minimal.udon | 112.6 ns / 457 MiB/s | **74.8 ns / 688 MiB/s** | **1.51x** |

## Remaining Optimization Opportunities

1. **Event emission optimization** - Ring buffer push may have overhead
2. **State machine dispatch** - Match on enum has some cost
3. **Zero-copy feed** - Borrow input directly for single-chunk case (attempted, had issues)

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
