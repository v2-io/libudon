# Agent Context Protocol — Brainstorm

*Raw thinking about what agents actually need from tooling*

---

## The Core Insight

LSP was built for humans staring at screens with mice and keyboards. Every design decision reflects that:

- Keystroke events (humans type one char at a time)
- Cursor position (hover requires knowing where the mouse is)
- Visual feedback (syntax highlighting, squiggles, colors)
- Spatial navigation ("go to" assumes you're moving through a visual space)
- Low latency streaming (humans notice 50ms delays)

Agents have *none* of these constraints and benefit from *none* of these features.

**Syntax highlighting is the perfect example.** It's the quintessential "tightening the feedback loop" feature for humans—you instantly see you forgot to close a string because the colors go wrong. For agents? Completely useless. We receive plain text.

So what's the agent equivalent? What would give us the same immediate, pre-failure feedback that highlighting gives humans?

---

## What Actually Causes Friction (From Lived Experience)

### Reading Code

**Problem: Where am I?**
I read lines 73-95 and I'm flying blind. Is this inside a function? Which class? Does it continue after line 95 or is that the end?

**What would help:**
```
SCOPE STACK:
  1   mod parser
 23     impl Parser
 67       pub fn parse_element(...)
---
(your requested lines)
---
 96     end of fn
156   end of impl
```

Just that. Tell me where I am in the structure. It's like... the agent equivalent of seeing the code *in context* in an IDE with proper folding.

**Problem: What is this thing?**
I see `token.kind` but I don't know what type `token` is. I see `self.emit(...)` but I don't know what `emit` does or where it's defined.

**What would help:**
Inline type annotations (even for typed languages where they're inferred), or a quick "this thing → that definition" mapping in the response:

```
REFERENCES OUT:
  token        : Token           (from line 68, self.peek())
  self.emit()  : → parser.rs:234, emits Event to output buffer
  Event::Start : → event.rs:12
```

**Problem: Is my search complete?**
I grep for "parse_element" and get 23 results. But did I miss anything? Were files excluded? Is there a camelCase variant I should have searched for too?

**What would help:**
```
SEARCH COVERAGE:
  Searched: 342 files
  Excluded: target/ (build), vendor/ (third-party)
  Note: parser.rs is GENERATED from udon.machine
  Variants found: parseElement (2 JS files), parse-element (1 YAML)
```

### Editing Code

**Problem: Line numbers are fragile**
I read the file, decide to edit line 47, but by the time I make the edit, something has shifted. Or I make one edit and all subsequent line references are wrong.

**What would help:**
Stable anchors that survive edits:
```
@parser.rs::Parser::parse_element::body::line3
```

That means "third line of parse_element's body"—survives edits above, survives edits to line length, only breaks if you actually modify parse_element itself.

**Problem: Text patterns can be ambiguous**
I try to replace `self.emit(` with something else, but that string appears 7 times in the file. The edit fails or worse, succeeds in the wrong place.

**What would help:**
Pre-edit ambiguity warnings:
```
EDIT HAZARD: Pattern "self.emit(" matches 7 times in file
  Unique alternatives:
    "self.emit(Event::Start"  → line 78 (unique)
    "self.emit(Event::End"    → line 94 (unique)
```

**Problem: I don't know if my edit broke something**
I make a change and... did that work? Do types still check? Did I break something in another file?

**What would help:**
Immediate post-edit verification:
```
EDIT APPLIED: parser.rs:78-82

IMMEDIATE VERIFICATION:
  Syntax: ✓
  Types:  ✓
  New warnings: 1 (unused variable 'tmp' @ line 79)

RIPPLE CHECK:
  ✗ Call site at document.rs:45 now has type mismatch
    Expected: Element, Got: Option<Element>
    Suggested fix: add .unwrap() or propagate Option
```

This is huge. Instead of "make edit → run build → wait → parse errors → figure out what I broke" it's just... instant feedback right in the edit response.

---

## The Propose/Validate/Apply Pattern

This is maybe the most important shift. Current tools are:
1. Make edit
2. Discover if it worked

Better would be:
1. Propose edit
2. See what would happen (validation, impact, side effects)
3. Adjust if needed
4. Apply when confident

```
PROPOSE: Add parameter 'opts: Options' to parse_element()

VALIDATION:
  Syntax: would pass
  Types: would pass

IMPACT:
  15 call sites need updating (list follows)
  3 tests would fail without call site updates
  Public API change (breaking if semver matters)

CASCADING FIXES AVAILABLE:
  Auto-fix all 15 call sites with Options::default()? [y/n]

APPLY? [yes / adjust / cancel]
```

---

## Intent Changes Everything

If I tell the tool *what I'm trying to do*, it can give me relevant context proactively:

**Intent: "reading" / "understanding"**
- Heavy on explanations
- Show me documentation, examples
- Link to tests that demonstrate behavior

**Intent: "editing"**
- Heavy on anchors and uniqueness
- Warn me about ambiguous patterns
- Show me style conventions I should match
- Tell me about generated files I shouldn't edit

**Intent: "debugging"**
- Show me recent changes to this code
- Show me test coverage (is this even tested?)
- Show me related error handling
- Show me git blame for context

**Intent: "refactoring"**
- Show me all usages (exhaustively)
- Show me what depends on this
- Warn me about public API boundaries
- Preview the full impact

---

## Semantic Neighbors

When I read a function, I almost always need to immediately read:
- What it calls (definitions of helpers)
- What calls it (to understand expected behavior)
- Tests that exercise it (to see examples)

What if reading a function included a preview of its neighborhood?

```
READING: Parser::parse_element

CALLS (depth 1):
  → self.peek()    : returns Token (parser.rs:234)
  → self.advance() : moves cursor (parser.rs:245)
  → Event::element : constructs event (event.rs:34)

CALLED BY:
  ← parse_document() @ line 45 (3 call sites)
  ← parse_fragment() @ line 120 (1 call site)

TESTED BY:
  test_parse_element_basic   @ tests/parser_test.rs:89
  test_parse_element_nested  @ tests/parser_test.rs:112
```

One read gives me the local graph. Huge reduction in round-trips.

---

## Types as First-Class Citizens

Even in typed languages, the actual types at a specific location aren't always obvious (generics, inference, traits). Even more so in dynamic languages.

What if every code read came with type annotations inferred from usage?

```python
def process(data):     # data: List[Event] (high confidence, 47 call sites)
    results = []       # results: List[ProcessedEvent]
    for item in data:  # item: Event
        x = transform(item)  # x: Result[ProcessedEvent, Error]
        results.append(x.unwrap())
    return results     # returns: List[ProcessedEvent]
```

Even if the source doesn't have these annotations, the *read* could include them. The agent version of type hints in an IDE.

---

## Staleness and Churn

Some code is battle-tested and stable. Some is actively being rewritten. Some is ancient and nobody remembers why it exists. This context matters!

```
PRAGMATICS:
  Last modified: 8 months ago (stable)
  Changes in last year: 2
  Authors: joseph (92%), automated (8%)

  This function: STABLE (low churn, many dependents)
  This file: ACTIVE (12 changes this month)
  This line specifically: ANCIENT (unchanged 3 years, here be dragons)
```

Or even:
```
CHURN HOTSPOTS in view:
  Lines 234-250: 🔥 12 changes last month (active development)
  Lines 400-450: 🧊 0 changes in 2 years (fossilized)
```

---

## Confidence and Epistemic Status

Not all information is equally reliable. The agent equivalent of showing things in italics vs bold vs normal.

```
{
  "function_type": "Element -> Result<Parsed>",
  "confidence": "proven",
  "basis": "type system"
}

{
  "purpose": "parses a single UDON element and its children",
  "confidence": "inferred",
  "basis": "function name, doc comments, test descriptions"
}

{
  "is_performance_critical": "possibly",
  "confidence": "heuristic",
  "basis": "called in hot loop based on call graph analysis"
}
```

This lets agents calibrate. "The type checker says this" vs "I'm guessing based on naming conventions."

---

## The Anchor System

This deserves deep thought. Line:column is the original sin of tool-code interfaces.

**What makes a good anchor?**

1. **Semantic**: Based on meaning, not bytes
2. **Hierarchical**: Can drill down (file → class → method → block → statement)
3. **Stable**: Survives unrelated edits
4. **Resolvable**: Can always ask "where is this now?"
5. **Fuzzy-matchable**: If exact anchor disappears, find closest match

**Possible syntax:**
```
@file.rs                                 # whole file
@file.rs::Parser                         # struct/class
@file.rs::Parser::parse_element          # method
@file.rs::Parser::parse_element::body    # method body (excluding signature)
@file.rs::Parser::parse_element::sig     # just the signature
@file.rs::Parser::parse_element::body:3  # line 3 of the body
@file.rs::Parser::parse_element::match#1 # first match expression
@file.rs::Parser::parse_element::match#1::arm:TokenKind::Ident  # specific arm
```

**Resolution API:**
```
RESOLVE @parser.rs::Parser::parse_element::body:3
  Currently: line 70 (was line 68 before your last edit)
  Status: VALID

RESOLVE @parser.rs::OldFunction
  Status: DELETED (removed in commit abc123, 2 days ago)
  Nearest: @parser.rs::NewFunction (70% similarity)
```

---

## Semantic Diffs

Instead of "lines 45-50 changed", what if diffs were semantic?

```
SEMANTIC DIFF (last commit):
  - Return type changed: Element → Option<Element>
  - New error handling path added for empty input
  - Function now fallible where it wasn't before

AFFECTED CONSUMERS:
  12 call sites now need to handle None case
  3 tests assume non-None return
```

This is what you actually want to know. Not "these bytes changed" but "this is what's *different* about how this code behaves."

---

## Cross-Language/Cross-Boundary Awareness

Real projects have FFI, bindings, generated code. A rename isn't complete if you change the Rust but not the Ruby binding.

```
RENAME IMPACT for Parser::parse_element → Parser::parse_node

  Rust source: 1 definition, 15 usages
  FFI binding (udon-ffi): exports as 'udon_parse_element' ⚠️
  Ruby binding: calls via FFI.udon_parse_element ⚠️
  Docs: references 'parse_element' in 3 places

WARNING: FFI boundary - C symbol name must stay stable OR be updated in:
  - udon-ffi/src/lib.rs
  - ruby/lib/udon/ffi.rb
```

---

## "What Would Help Here?"

A meta-query. I'm looking at this code and I don't even know what I don't know.

```
REQUEST: What would help me here?
LOCATION: @parser.rs::Parser::parse_element

SUGGESTIONS:
  - This function has 0 doc comments; understanding may be difficult
  - Similar function parse_text (line 45) has better documentation
  - 3 tests exercise this; reading them might clarify behavior
  - Recent bug fix (commit xyz, 3 days ago) added edge case handling
  - This function is 73 lines; consider reading in parts:
    - signature/setup: lines 67-72
    - main match: lines 74-90
    - cleanup/return: lines 91-95
```

The tool notices things I might not think to ask about.

---

## Error Prevention vs Error Recovery

LSP is largely about error recovery—you make an error, squiggly line appears, you fix it.

ACP should be about error *prevention*—you're about to make an error, warning appears, you don't make it.

**Before you edit:**
- "This pattern is ambiguous, here are unique alternatives"
- "This file is generated, your edits will be lost"
- "This is a public API, changes break consumers"

**Before you commit:**
- "This change breaks type checking in 3 files"
- "This introduces a cycle in the module graph"
- "This shadows a variable from outer scope"

**Before you even read:**
- "This file is 5000 lines; here's a map"
- "This function is known to be confusing; here's the key insight"
- "This code is deprecated; here's the replacement"

---

## Generation-Aware Assistance

I want to add a function. Instead of just "here's blank space, good luck":

```
REQUEST: Help me add a function to Parser

CONVENTIONS DETECTED:
  - Functions in this impl return Result<T, ParseError>
  - Private helpers are prefixed with try_ or do_
  - Error handling uses ? operator throughout
  - Functions appear in rough call-order (callers before callees)

SUGGESTED PLACEMENT:
  - After parse_attribute (line 130) if it's a peer parser
  - At end of impl (line 156) if it's a helper

YOU'LL LIKELY NEED:
  use crate::error::ParseError;  // already imported ✓
  use crate::event::Event;       // already imported ✓

SIMILAR FUNCTIONS TO REFERENCE:
  - parse_text (line 45) - simple, good template
  - parse_element (line 67) - more complex, handles nesting
```

---

## Test Awareness

Which tests cover this code? If I change this, what tests should I run? If tests are failing, which code paths are suspect?

```
COVERAGE for Parser::parse_element:
  Lines 67-72 (setup): covered by 4 tests
  Lines 73-85 (main path): covered by 4 tests
  Lines 86-89 (error path): ⚠️ UNTESTED
  Lines 90-95 (nested case): covered by 2 tests

RELEVANT TESTS:
  test_parse_element_basic - exercises happy path
  test_parse_element_empty - exercises error at line 88 (wait, contradiction?)
  test_parse_element_nested - exercises recursion

IF YOU MODIFY this function:
  Minimum test run: cargo test parse_element
  Full coverage: cargo test parser::
```

---

## The Response Envelope

Every response could have a standard enrichment envelope:

```
{
  "content": { ... },           // What you asked for

  "structure": { ... },         // Where you are (scope chain, boundaries)
  "semantics": { ... },         // What things mean (types, signatures)
  "references": { ... },        // Relationships (calls, usages, tests)
  "pragmatics": { ... },        // History, authorship, churn
  "hazards": [ ... ],           // Warnings relevant to your intent
  "suggestions": [ ... ],       // Proactive help

  "confidence": { ... },        // How reliable is this info
  "freshness": { ... },         // How stale might this be
  "anchors": { ... }            // Stable references for editing
}
```

Clients request which layers they want. Servers provide what they can. Graceful degradation.

---

## Implementation Sketch

What would a minimal useful implementation look like?

**Level 0: Enriched Read**
Just add structure breadcrumbs to file reads. No semantic analysis, just parsing.
```
SCOPE: module > impl > function
(your code here)
END: line 95, next sibling at 97, impl ends at 156
```

**Level 1: + Anchors**
Add an anchor system. Reads include anchors, edits can use anchors.

**Level 2: + References**
Build a basic call graph / usage index. "What calls this, what does this call."

**Level 3: + Validation**
Hook into compiler/type-checker for pre-edit validation.

**Level 4: + Intent**
Context-aware responses based on declared intent.

**Level 5: + Proposals**
Full propose/validate/apply workflow with cascading changes.

---

## Random Ideas That Might Be Good

- **Diff narration**: Not just "what changed" but "what you should know about what changed"
- **Staleness alerts**: "Your understanding of this code is based on version X; it's now version Y, here's what changed"
- **Confidence degradation**: As files change, confidence in cached analysis decreases
- **Pattern detection**: "You're doing X, which usually means you want Y too"
- **Conversation context**: Server remembers what agent was working on, anticipates needs
- **Checkpoint/restore**: Mark a known-good state, be able to roll back
- **Impact radius**: "If you change this, you need to think about those"
- **Idiom matching**: "This codebase handles X by doing Y, match that pattern"
- **Failure prediction**: "This kind of change often causes Z, watch for it"

---

## What's Different From LSP

| LSP | ACP |
|-----|-----|
| Keystroke-driven | Batch-oriented |
| Cursor/position | Semantic anchors |
| Visual feedback | Structured metadata |
| Navigate-to actions | Data returned |
| Incremental/streaming | Complete responses |
| Error recovery | Error prevention |
| Single-file focus | Cross-file awareness |
| Implicit confidence | Explicit confidence |
| Generic responses | Intent-aware responses |
| Edit then discover | Propose then apply |

---

## Open Questions

- How to handle languages with poor semantic tooling? Graceful fallback?
- How to balance depth vs latency? Agent can wait longer but not forever
- How to share/sync between multiple agents? Locks? Events?
- How much should the protocol care about specific languages vs be generic?
- Should there be a "learning" mode where server adapts to agent patterns?
- What's the right transport? JSON-RPC like LSP? Something simpler?
- How does this interact with existing tools? Wrapper? Replacement? Complement?

---

## Wild Ideas

**The code as a database query target.** Not "read this file" but "SELECT * FROM functions WHERE visibility = 'public' AND return_type LIKE 'Result%'"

**Predictive context.** "You just read function A. 80% of the time, agents reading A also read B and C. Here they are."

**Causal tracing.** "This variable got this value because of line 45 which depended on config X which came from..."

**Hypothetical execution.** "If you called this function with X, here's what would happen (traced statically)."

**Semantic bookmarks.** "Remember this location as 'the place where errors are handled'" → later: "take me to 'the place where errors are handled'"

**Confidence propagation.** "I'm 90% sure about A, and A implies B, so I'm 80% sure about B."

---

*This is brainstorming. Some of these ideas are good, some are probably impractical, some might be brilliant in ways I haven't figured out yet. The core intuition is: agents need different affordances than humans, and nobody has really built tooling that takes that seriously.*
