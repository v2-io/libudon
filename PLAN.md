# libudon Development Plan

Parser implementation using descent (~/src/descent/).

## Architecture

descent **replaces** the old parser infrastructure entirely. The old ring-buffer,
ChunkSlice, ChunkArena, genmachine architecture is gone. descent generates clean
callback-based recursive descent parsers from `.desc` specifications.

**Key insight from implementation-phase-2.md:** The streaming event model is the
foundation, not a feature. The parser emits events as it parses—no accumulation.
The tree builder (when implemented) will be just another event consumer.

## Current Status

**Branch:** `phase-3-genmachine-rewrite`

### What Works

- [x] Elements with names (`|element`)
- [x] Element identity (`|element[id].class1.class2`)
- [x] Sameline attributes (`:key value`)
- [x] Block (indented) attributes
- [x] Multiple sameline attributes (`:a 1 :b 2 :c 3`)
- [x] Typed values via context-aware parsing:
  - Integer (decimal, hex 0x, octal 0o, binary 0b)
  - Float (with decimal or exponent)
  - BoolTrue, BoolFalse (`true`, `false`)
  - Nil (`null`, `nil`)
  - BareValue (unquoted strings)
- [x] Keywords via PHF perfect hash (`|keywords` directive)
- [x] Context-aware terminators (block/sameline/embedded/array)
- [x] Proper EOF handling via `|eof` directive
- [x] Text/prose content
- [x] Basic indentation hierarchy
- [x] Nested elements

### What's Incomplete

- [ ] Element suffixes (`?`, `!`, `*`, `+`)
- [ ] Embedded elements (`|{name attrs content}`)
- [ ] Directives (`!if`, `!for`, `!include`, etc.)
- [ ] Raw blocks (`!:lang:`)
- [ ] Interpolation (`!{{expr}}`)
- [ ] Escape sequences (`'|` escapes pipe in prose)
- [ ] Comments (`;` handling in various contexts)
- [ ] Quoted strings in values
- [ ] Arrays (`[a b c]`)

## Phase 3: Build Forward (IN PROGRESS)

### 3.1 Test Infrastructure (NEXT)

Tests come first. Every grammar feature gets tests before implementation.

- [ ] Delete old tests (reference deleted infrastructure)
- [ ] Build test harness for descent event model
- [ ] Extract test cases from SPEC.md examples
- [ ] Property-based / permutation testing framework
- [ ] Tests for what already works (baseline)

### 3.2 Core Grammar Completion

Test-driven. Each feature: write tests → implement → verify.

1. **Quoted strings** - `"double"` and `'single'` quoted values
2. **Arrays** - `[item1 item2 item3]` inline lists
3. **Element suffixes** - `?`, `!`, `*`, `+` expand to attributes
4. **Embedded elements** - `|{name attrs content}` inline in prose
5. **Comments** - `;` at line start and inline (context-aware)
6. **Escape sequences** - `'|`, `'\`, etc.

### 3.3 Directive System

The parser's only directive-level knowledge is body mode (per parser-strategy.md):

| Syntax | Body | Parser Behavior |
|--------|------|-----------------|
| `!foo` | UDON | Parse body recursively as UDON |
| `!:foo:` | Raw | Capture body verbatim, tag with "foo" |

1. **Block directives** - `!if`, `!elif`, `!else`, `!for`, `!let`
2. **Inline directives** - `!name{content}` with balanced braces
3. **Raw directives** - `!:lang:` block and `!{:lang: content}` inline
4. **Interpolation** - `!{{expr}}`, `!{{expr | filter}}`

### 3.4 Cleanup

- [ ] Remove `udon-core/src/values_parser.rs` (obsolete)
- [ ] Evaluate `udon-core/src/value.rs` (post-hoc classification may be redundant)

## Grammar DRY Refactoring

Issues identified in `generator/udon.desc` and `generator/values.desc`:

### Completed

- [x] **`array_block`, `array_sameline`, `array_embedded`** → unified to single `array` function
  - Were 27 lines, now 9 lines
  - Only differed in recursive call target

### Ready to Unify

- [x] **`prose`, `prose_pipe`, `prose_backtick`** → unified to `prose(line_col, parent_col, prepend)`
  - Using `<>` for no prepend, `'|'` for pipe, `'`'` for backtick
  - `prose_backticks` kept separate (calls `text_backticks` for ``)

- [x] **`value_block`, `value_sameline`, `value_embedded`** → unified to `value(space_term, bracket)`
  - `space_term`: 0 (block) or 1 (sameline)
  - `bracket`: `'\0'` (none), `'}'` (embedded), `']'` (array)

- [ ] **`double_quoted` vs `single_quoted`** (lines 368-378)
  - Differ only in quote character matched
  - Comment says "parameterized version had scan optimization issues with :quote"
  - May require descent enhancement to unify

### Values.desc Issues

- [ ] **21 nearly-identical number parsing states** in `values.desc`
  - `dec_start`, `dec_digits`, `hex_start`, `hex_digits`, `oct_start`, etc.
  - Each number format (dec, hex, oct, bin) has 4-5 states with identical structure
  - Only differences: valid digit characters and type emitted
  - Could potentially use descent's character class feature more effectively

### Naming/Vocabulary Issues

- [x] **Magic numbers**: 124='|', 33='!', 96='`' used as prepend values
  - Now using character literals: `'|'`, `'!'`, `'`'`

- [x] **`inline_*` → `sameline_*`**: Per FULL-SPEC vocabulary
  - `inline_directive` → `sameline_directive`
  - `inline_raw` → `sameline_raw`
  - `inline_dir_body` → `sameline_dir_body`

- [x] **Bracket aliases**: Now using character literals directly
  - `<L>` → `'['`, `<R>` → `']'`, `<RB>` → `'}'`
  - Kept `<SQ>`, `<DQ>`, `<BS>` for readability in escape contexts

### Known Test Failures (Variations)

Canonical tests pass, but variation tests (with random indentation) reveal edge cases:

- [ ] **Freeform blocks with indentation** - `basic_freeform_block`, `freeform_preserves_pipes`
  - Freeform parser includes leading whitespace in RawContent
  - Doesn't find closing ``` when input is indented
- [ ] **Error cases with indentation** - `tab_character_error`, `unclosed_*` variants
  - Error handling doesn't account for indented input variations
- [ ] **Embedded elements** - `unclosed_embedded_element_error` variations

These need investigation - the variation test framework is surfacing real edge cases.

### Compliance Issues

- [x] **Unicode identifier support** (CRITICAL for SPEC compliance)
  - Element names, attribute keys, class names now support Unicode
  - Replaced `LETTER` → `XLBL_START` and `LABEL_CONT` → `XLBL_CONT`
  - Uses `unicode-xid` crate for XID_Start/XID_Continue checks

## Phase 4: Multi-Chunk Streaming & Performance

From descent TODO #10 - resumable state machine for true streaming:

```rust
loop {
    match parser.parse(chunk, on_event) {
        ParseResult::Complete => break,
        ParseResult::NeedMoreData => {
            chunk = get_next_chunk();  // Caller controls flow
        }
    }
}
```

**Design:**
- Zero-copy for 99% of input (tokens within chunks)
- Small internal buffer (~256 bytes) for cross-boundary tokens only
- Backpressure via blocking callbacks (caller controls chunk feed rate)
- No ring buffer needed - callbacks are 2-7x faster

**Tasks:**
- [ ] Multi-chunk streaming in descent (ParseResult enum)
- [ ] Cross-boundary token handling
- [ ] Benchmark suite (criterion)
- [ ] Memory profiling on large files

## Phase 5: Tree Builder

Build arena-allocated tree from events (event consumer pattern):

- [ ] `Document` and `Node` structs with arena allocation
- [ ] Tree builder that consumes parser events
- [ ] Navigation (parent, children, siblings)
- [ ] Simple selectors
- [ ] String interning for element/attribute names

## Phase 6: Language Bindings

### Ruby (udon-ruby)
- [ ] FFI layer for streaming API
- [ ] Lazy tree projection (Ruby objects created on access)
- [ ] Update to use new callback-based parser

### Other Targets
- [ ] WASM build (`wasm32-unknown-unknown`)
- [ ] Python via PyO3
- [ ] C ABI shared library

## Key Files

| File | Purpose |
|------|---------|
| `generator/udon.desc` | Main parser grammar |
| `generator/values.desc` | Value type parsing (concatenated) |
| `udon-core/src/parser.rs` | GENERATED by descent - do not edit |
| `regenerate-parser` | Script to regenerate parser |

## descent Workflow

```bash
# Regenerate parser (concatenates .desc files, runs descent, builds)
./regenerate-parser

# Generate only (no build)
./regenerate-parser --no-build

# Debug parsing stages
descent debug generator/udon.desc
descent debug generator/udon.desc --tokens
```

## Reference

- `~/src/descent/CLAUDE.md` - descent usage guide
- `~/src/udon/SPEC.md` - Authoritative UDON specification
- `~/src/udon/implementation-phase-2.md` - Ideal streaming architecture
- `~/src/udon/parser-strategy.md` - Multi-language strategy
