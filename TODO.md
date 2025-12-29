# libudon TODO

Issues identified from Codex code review (Dec 2025).

---

## Critical Bugs (Silent Data Loss / Spec Violations)

### 1. Backpressure Drops Events
**File:** `udon-core/src/parser.rs:452`
**Severity:** HIGH

`emit()` ignores `try_push()` failures, so events are silently lost when the ring buffer is full. `feed()` reports `buffer_full` but parsing doesn't pause - it continues and drops events.

**Fix:** Either:
- Make `emit()` return a result and have state machine handle backpressure
- Or pause parsing when buffer is full and resume on next `feed()` call

### 2. Inline Escape Only Works at Line Start
**File:** `generator/udon.machine` (child_escaped states)
**Severity:** HIGH

The escape prefix `'` is only handled at line start via `child_escaped` state. You cannot escape `|`, `;`, or `!` mid-line in prose content.

**SPEC says:** Escape should apply wherever a prefix would be parsed.

**Example that fails:**
```
|p Some text with '|literal pipe mid-line
```

**Root cause:** This is a case of **coding to tests rather than SPEC**. The tests only covered line-start escaping, so that's all that was implemented.

**Fix:** Add escape handling in prose scanning states (`child_prose`, `inline_text`, etc.) AND add comprehensive tests for mid-line escape scenarios.

### 3. Pipe-as-Text Enforcement in Prose
**File:** `udon-core/src/parser.rs:880, 1291`
**Severity:** MEDIUM (may already be fixed)

Codex claims prose always treats `|` as element start, breaking Markdown table pipes.

**Assessment:** We have `inline_check_pipe` state that should handle this. Need to verify:
- ` | ` (space-pipe-space) is treated as text
- `|` followed by invalid element starter becomes text

**SPEC reference:** Lines 645-651 define when pipe is text vs element.

---

## Architecture Issues (Incomplete Features)

### 4. Streaming Resume Not Implemented
**Files:** `udon-core/src/parser.rs:769`
**Severity:** HIGH for streaming use case

`parse_continue()` always calls `parse_document()`. The following are scaffolded but not wired:
- `ParserState` enum
- `call_stack`
- `in_partial` flag
- Cross-chunk token handling

**Impact:** Multi-chunk feeds are not safe. Parser only works correctly when entire input is fed at once.

**Fix options:**
1. Implement proper state machine suspension/resume
2. Or document single-feed limitation and remove unused scaffolds

### 5. finish() Doesn't Close Open Elements
**Files:** `udon-core/src/parser.rs:211, 112`
**Severity:** MEDIUM

`finish()` only closes elements if `element_stack` is used, but nothing pushes to it. Closure depends entirely on state machine EOF paths.

**Assessment:** The state machine does handle EOF via return chains, emitting `ElementEnd` events. But this should be verified with tests for deeply nested elements at EOF.

### 6. Brace Counting Depth Limited to 3
**Files:** `udon-core/src/parser.rs:982, 2265`
**Severity:** HIGH

Inline comments, embedded content, and inline raw/interpolation/directive bodies use explicit `Nested`, `Nested2`, `Nested3` states. Deeper braces will misparse.

**Root cause:** The DSL supports both recursive functions AND variable increment/decrement. Using hardcoded `Nested1/2/3` states instead of either approach is **incorrect usage of the DSL**.

**Fix options:**

1. **Counter-based (simpler for pure bracket matching):**
```
|var[depth] = 0
|state[:scan]
  |c[{]  | depth += 1 | -> |>>
  |c[}]  | depth -= 1 | -> |>> :check_done
  |default | -> |>>
|state[:check_done]
  |if[depth == 0] |return
  |default |>> :scan
```

2. **Recursive function (if nested content needs different handling):**
```
|function[brace_content]
  |state[:scan]
    |c[{]  | -> |>> /brace_content :after_nested  ; Recurse
    |c[}]  |    |return                            ; Pop
    |default | -> |>>
```

Counter is likely better here since we just need balanced braces, not semantic nesting.

### 7. Span Accuracy for Suffix/ID/Class Attributes
**File:** `udon-core/src/parser.rs:467`
**Severity:** LOW

`emit_special_attribute()` uses `span_from_mark()` but `mark()` may not be set to the suffix/class position. Spans might point to element name instead.

**Fix:** Ensure `MARK` is called at the right position before emitting these attributes.

---

## Cleanup / Dead Code

### 8. Unused Event Variants
**File:** `udon-core/src/streaming.rs`

These exist in `StreamingEvent` but are never emitted:
- `InlineDirective`
- `FreeformStart`
- `FreeformEnd`

Freeform blocks use `RawContent` instead.

**Action:** Remove unused variants or implement them for API clarity.

### 9. ChunkArena::advance_consumed() Unused
**File:** `udon-core/src/streaming.rs:220`
**Severity:** MEDIUM for long streams

Chunks are never freed during streaming, so memory grows unbounded on long streams.

**Fix:** Either:
- Implement chunk cleanup when events are consumed
- Or document that parser is designed for bounded input size

### 10. Unused Scaffolds
**File:** `udon-core/src/parser.rs`

Multiple unused fields/types indicating incomplete streaming design:
- `ParserState` enum
- `call_stack: Vec<...>`
- `element_stack: Vec<...>`
- `partial: Vec<u8>`
- `in_partial: bool`

**Action:** Either implement streaming resume or remove these to reduce confusion.

---

## Documentation Out of Sync

### 11. CLAUDE.md References Removed Code
- References `udon-ffi/` which was removed
- References `event.rs` which was removed
- Feature status table may be outdated

### 12. README.md Overclaims
- Claims streaming is "complete" but several spec items are deferred
- Should sync to actual implementation status

---

## Systemic Testing Deficiency

**The test suite has a fundamental quality problem: tests were written to validate existing behavior rather than to enforce SPEC compliance.**

Evidence:
- Escape tests only cover line-start (column 0) scenarios
- Brace depth tests don't exist (hardcoded limit went unnoticed)
- Tests may have been written AFTER implementation, validating what was built rather than what should be built

**Required approach going forward:**
1. Tests MUST be derived from SPEC.md, not from implementation
2. Tests should cover edge cases: mid-line, mid-content, deeply nested, boundary conditions
3. Use property-based testing where applicable
4. Review existing tests for "column 0 bias" and other implicit assumptions

## Missing Tests

Add targeted tests for:

1. **Pipe-as-text in prose** - ` | ` should be text, not element
2. **Inline escape mid-line** - `'|` mid-prose should escape the pipe
3. **Brace depth > 3** - must work with arbitrary depth after fix
4. **Backpressure/resume** - what happens when ring buffer fills
5. **Raw block content semantics** - verify `!:lang:` content handling
6. **EOF with nested elements** - verify all ElementEnd events emit
7. **All features at non-zero columns** - elements, attributes, directives, escapes
8. **All features mid-line in prose** - not just at line start

---

## Suggested Priority Order

1. **Backpressure bug** - silent data loss is critical
2. **Fix brace counting** - use recursive function, not hardcoded states
3. **Mid-line escape** - SPEC compliance, reveals coding-to-test problem
4. **Verify pipe-as-text** - may already work, needs tests
5. **Audit tests for column-0 bias** - fix systemic testing issue
6. **Clean up unused code** - reduce confusion
7. **Sync documentation** - accuracy
8. **Decide on streaming** - implement or remove scaffolds

---

## Previous TODOs (Generator Refactoring)

- Refactor genmachine-rs and machine DSL:
  - Make emit action mapping dynamic based on what's in the machine file
  - Move special cases to liquid templates instead of hardcoding in generator
