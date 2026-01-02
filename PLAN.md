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
- [x] Element suffixes (`?`, `!`, `*`, `+`)
- [x] Sameline attributes (`:key value`)
- [x] Block (indented) attributes
- [x] Multiple sameline attributes (`:a 1 :b 2 :c 3`)
- [x] Typed values via context-aware parsing:
  - Integer (decimal, hex 0x, octal 0o, binary 0b)
  - Float (with decimal or exponent)
  - Rational (`1/3r`, `22/7r`)
  - Complex (`3+4i`, `5i`, `3.5-2.1i`)
  - BoolTrue, BoolFalse (`true`, `false`)
  - Nil (`null`, `nil`)
  - BareValue (unquoted strings)
  - StringValue (quoted `"double"` and `'single'`)
- [x] Keywords via PHF perfect hash (`|keywords` directive)
- [x] Context-aware terminators (block/sameline/embedded/array)
- [x] Proper EOF handling via `|eof` directive
- [x] Text/prose content
- [x] Basic indentation hierarchy
- [x] Nested elements
- [x] Embedded elements (`|{name attrs content}`)
- [x] Arrays (`[a b c]`)
- [x] Comments (`;` line comments and `;{brace comments}`)
  - Comment is now BRACKET type (CommentStart/Text/CommentEnd)
  - Uses same "children loop" pattern as Element for continuation
  - Continuation lines preserve their indentation in Text content
- [x] Block-level escape prefix (`'` before `|;:!'` at line start)
- [x] Directives (`!if`, `!elif`, `!else`, `!for`, `!let`, `!include`, `!unless`)
- [x] Raw blocks (`!:lang:`) and inline raw (`!{:lang:content}`)
- [x] Interpolation (`!{{expr}}`, `!{{expr | filter}}`)
- [x] Inline element nesting (proper sibling detection on sameline)
- [x] Freeform blocks (```) inside elements
- [x] Prose content_base tracking:
  - First prose line establishes content_base
  - Extra spaces beyond content_base preserved in output
  - Warning event on inconsistent (decreased) indentation
  - content_base updated on inconsistent indent

### What Needs Work

- [ ] **Fixture coverage** - Many fixtures have empty events, need SPEC-based expectations
- [ ] **Variation test edge cases** - Some edge cases with indentation variations
- [ ] **value.rs evaluation** - Post-hoc classification may be redundant now

## Phase 3: Build Forward (IN PROGRESS)

### 3.1 Test Infrastructure ✓

- [x] Test harness for descent event model (canonical.rs)
- [x] YAML fixture format with expected events
- [x] Variation testing (random indentation, sibling elements)
- [ ] Extract more test cases from SPEC.md examples
- [ ] Property-based testing for edge cases

### 3.2 Core Grammar - COMPLETE

All core grammar features implemented:
- [x] Quoted strings (`"double"` and `'single'`)
- [x] Arrays (`[item1 item2 item3]`)
- [x] Element suffixes (`?`, `!`, `*`, `+`)
- [x] Embedded elements (`|{name attrs content}`)
- [x] Comments (line and brace)
- [x] Block-level escapes (`'|`, `';`, etc.)

### 3.3 Directive System - COMPLETE

- [x] Block directives (`!if`, `!elif`, `!else`, `!for`, `!let`, `!unless`)
- [x] Inline directives (`!{name args}`)
- [x] Raw directives (`!:lang:` block and `!{:lang: content}` inline)
- [x] Interpolation (`!{{expr}}`, `!{{expr | filter}}`)

### 3.4 Cleanup

- [x] Removed `udon-core/src/values_parser.rs` (obsolete)
- [x] Removed `udon-core/benches/values.rs` (used values_parser.rs)
- [ ] Evaluate `udon-core/src/value.rs` (post-hoc classification may be redundant)

## Fixture Status

### CRITICAL WARNING

**DO NOT fill fixture expectations by tracing parser output.**

This anti-pattern has happened multiple times and cements bugs as expected behavior.
Work will be reverted if fixtures are filled this way.

**Correct workflow:**
1. Read FULL-SPEC.md for the feature
2. Write fixture expectations based on SPEC
3. Run tests - they WILL fail
4. Fix the PARSER to match SPEC
5. Tests pass

### Audited Against SPEC
- [x] **value_types.yaml** (23 tests) - All value types per SPEC
- [x] **prose_dedentation.yaml** (13 tests) - Audited, comments fixed
- [x] **literal_escape.yaml** (5 tests) - Filled per SPEC 104-130, ALL PASS
- [x] **indentation_hierarchy.yaml** (7 tests) - Filled per SPEC 543-820, ALL PASS
- [x] **inline_comments.yaml** (8 tests) - Audited, 1 bug exposed (space after comment)
- [x] **comments.yaml** (30+ tests) - Comprehensive audit, 6 bugs exposed
- [x] **references.yaml** (4 tests) - Filled, exposes missing grammar

### Needs SPEC Audit

These were filled by tracing parser output (WRONG). Need audit against FULL-SPEC.md:

- [ ] **embedded_elements.yaml** (20 tests) - Audit against SPEC
- [ ] **inline_element_nesting.yaml** (11 tests) - Audit against SPEC

### Not Yet Filled

| Fixture | Empty Tests | Priority |
|---------|-------------|----------|
| indentation_edge_cases.yaml | 5 | Low |
| comments_and_text.yaml | 2 | Low |
| arrays.yaml | 1 | Low |

## Known Parser Bugs

Discovered while filling fixtures - need grammar fixes:

1. **Interpolation in attr values not implemented** (SPEC 904-910 notes this)
   - `|{a :href !{{url}} text}` - `!{{url}}` is not recognized as interpolation
   - Currently treated as literal text, braces get mangled

2. ~~**Text before nested embedded elements lost**~~ - FIXED
   - Text content before nested `|{...}` now captured correctly

3. **Empty Text events after nested embedded**
   - Nested embedded elements emit `Text ""` after closing
   - Not a bug per se, but could be optimized away

4. **SPEC Update:** Removed `~` as Nil synonym (only `null`/`nil` now)

5. ~~**Block prose semicolons should be LITERAL**~~ - FIXED (SPEC line 408)
   - Parser now treats `;` in block prose as literal text
   - `;` at line start is still a block comment per SPEC 459-466

6. ~~**Comment continuation not implemented**~~ - FIXED (SPEC lines 419-428)
   - Comment refactored to BRACKET type (like Element)
   - Uses same "children loop" pattern for continuation lines
   - Each continuation line is a separate Text event inside CommentStart/End

7. ~~**Space after inline comment stripped**~~ - FIXED (SPEC line 495)
   - After inline comment, now goes to `:post_sameline_inline` instead of `:pre_content`
   - Spaces after `}` are preserved in text content

8. **Block-level references not implemented** (SPEC 1473-1488)
   - `@[id]` at block level for element insertion
   - `:[id]` in attribute position for merge

## Grammar DRY Refactoring

### Completed

- [x] `array` - unified from 3 variants, owns `[` delimiter
- [x] `prose` - unified with prepend parameter
- [x] `value` - unified with space_term and bracket parameters
- [x] Character literals instead of magic numbers
- [x] `inline_*` → `sameline_*` per SPEC vocabulary
- [x] Unicode identifier support (XID_Start/XID_Continue)

### Remaining

- [ ] `double_quoted` vs `single_quoted` - could parameterize
- [ ] values.desc number parsing - 21 nearly-identical states

## Grammar Clarity Principles

1. **Functions describe complete constructs** — `array` parses `[...]` including delimiters
2. **States have single responsibilities** — separate `:entry`, `:content`, etc.
3. **Callers dispatch, callees consume** — callers check, callees own their syntax
4. **Delegate to appropriate abstractions** — `array` calls `/value`, not quote parsing

## Phase 4: Multi-Chunk Streaming & Performance

From descent TODO - resumable state machine for true streaming:

```rust
loop {
    match parser.parse(chunk, on_event) {
        ParseResult::Complete => break,
        ParseResult::NeedMoreData => {
            chunk = get_next_chunk();
        }
    }
}
```

**Tasks:**
- [ ] Multi-chunk streaming in descent (ParseResult enum)
- [ ] Cross-boundary token handling
- [ ] Benchmark suite (criterion)
- [ ] Memory profiling on large files

## Phase 5: Tree Builder

Build arena-allocated tree from events:

- [ ] `Document` and `Node` structs with arena allocation
- [ ] Tree builder that consumes parser events
- [ ] Navigation (parent, children, siblings)
- [ ] Simple selectors
- [ ] String interning for element/attribute names

## Phase 6: Language Bindings

### Ruby (udon-ruby)
- [ ] FFI layer for streaming API
- [ ] Lazy tree projection
- [ ] Update to use callback-based parser

### Other Targets
- [ ] WASM build
- [ ] Python via PyO3
- [ ] C ABI shared library

## Key Files

| File | Purpose |
|------|---------|
| `generator/udon.desc` | Main parser grammar |
| `generator/values.desc` | Value type parsing (concatenated) |
| `udon-core/src/parser.rs` | GENERATED by descent - do not edit |
| `regenerate-parser` | Script to regenerate parser |

## Reference

- `~/src/descent/CLAUDE.md` - descent usage guide
- `~/src/udon/FULL-SPEC.md` - **Authoritative UDON specification**
- `~/src/udon/FULL-EBNF.md` - Extracted EBNF
- `~/src/udon/implementation-phase-2.md` - Ideal streaming architecture
- `~/src/udon/parser-strategy.md` - Multi-language strategy
