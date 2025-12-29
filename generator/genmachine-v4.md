# genmachine v4 Specification

A declarative DSL for generating recursive descent parsers with streaming (SAX-style)
event output. Designed for elegance, performance, and clarity.

## Design Philosophy

### Core Principles

1. **Type system declares behavior** - Return types determine what events are emitted
2. **EOF handling is inferred** - No explicit `|eof` cases needed
3. **SCAN optimization is automatic** - Generator infers memchr usage from state structure
4. **Call stack is the element stack** - Recursive descent naturally handles nesting
5. **DSL describes grammar, generator infers mechanics**

### Historical Context

The original C-era genmachine (104 lines, 6 functions, 14 states) used:
- Functions that returned typed values
- Explicit MARK/TERM for accumulation
- `return S` to return the accumulated value
- Caller assignment: `S.name = /label`

The Rust port lost this elegance (1,901 lines, 231 states) by:
- Flattening recursion into a giant state machine
- Inventing `emit()` abstraction that hid operations
- Losing function return values

This v4 spec restores the original elegance while producing SAX-style events:
- Functions have return types that declare event behavior
- MARK/TERM remain explicit
- `return` triggers events based on type
- No explicit `emit()` needed


## Document Structure

```
|parser udon                              ; Parser name (required first)

|type[Element]       BRACKET              ; Type declarations
|type[Name]          CONTENT
|type[INT]           INTERNAL

|entry-point /document                    ; Where parsing begins

|function[document]                       ; Function definitions
  |state[:main]
    ...
```


## Type System

Types declare what happens when a function returns:

| Category   | Behavior                                        | Example Types                    |
|------------|------------------------------------------------|----------------------------------|
| `BRACKET`  | Emit Start on entry, End on return             | Element, Embedded, Directive     |
| `CONTENT`  | Emit event with accumulated content on return  | Name, Text, Comment, Attr        |
| `INTERNAL` | No emission - internal use only                | INT, BOOL                        |

```
|type[Element]       BRACKET    ; ElementStart on entry, ElementEnd on exit
|type[Embedded]      BRACKET    ; EmbeddedStart/End
|type[Directive]     BRACKET    ; DirectiveStart/End
|type[Array]         BRACKET    ; ArrayStart/End

|type[Name]          CONTENT    ; emits Name event on return
|type[Text]          CONTENT    ; emits Text event
|type[Comment]       CONTENT    ; emits Comment event
|type[Attr]          CONTENT    ; emits Attr event
|type[StringValue]   CONTENT    ; emits StringValue event

|type[INT]           INTERNAL   ; no emit - internal integer
|type[BOOL]          INTERNAL   ; no emit - internal boolean
```


## Functions

### Basic Syntax

```
|function[name]                           ; Void function (no return type)
|function[name:ReturnType]                ; Returns/emits ReturnType
|function[name:Type]  :param1 :param2     ; With parameters
|function[name:Type]  | MARK | x = 0      ; With init actions
|function[name:Type]  | EXPECTS(})        ; With unclosed error annotation
```

### Parameters

Parameters are passed by the caller and available throughout the function:

```
|function[element:Element]  :elem_col :parent_col
  |state[:children]
    |if[COL <= elem_col]                  |return  ; Use param in condition
```

### Init Actions

Actions after the parameter list run on function entry:

```
|function[name:Name]  | MARK              ; Start accumulating immediately
|function[count:INT]  | result = 0        ; Initialize variable
|function[brace:Comment]  | depth = 1 | MARK | EXPECTS(})
```

### Special Variables

| Variable | Meaning                                |
|----------|----------------------------------------|
| `COL`    | Current column (1-indexed)             |
| `LINE`   | Current line (1-indexed)               |


## States

### Basic Syntax

```
|state[:name]                             ; State definition
|state[:name]  | depth = 1                ; With local init
|state[:name]  | EXPECTS(])               ; With unclosed delimiter annotation
```

### EXPECTS Annotation

Declares that this state/function expects a closing delimiter. On EOF without
seeing it, the generator emits an unclosed error:

```
|function[dquote_string:StringValue]  | MARK | EXPECTS(")
  |state[:main]
    |c["]        | TERM | ->              |return
    |c[\\]       | -> | ->                |>>
    |default     | ->                     |>>
    ; No |eof needed! Generator infers: TERM, emit unclosed_dquote error, return
```


## Cases (Character Matching)

Each case has three columns: match | actions | transition

```
|c[x]           | actions          |>> :state   ; Match single char
|c[\n]          | actions          |>>          ; Match newline (self-loop)
|c[ \t]         | actions          |return      ; Match space or tab
|c[abc]         | actions          |>> :next    ; Match a, b, or c
|default        | actions          |>> :other   ; Fallback
```

### Character Escapes

| Syntax | Character      |
|--------|----------------|
| `\n`   | Newline        |
| `\t`   | Tab            |
| `\\`   | Backslash      |
| `<P>`  | Pipe `\|`      |
| `<L>`  | Left bracket `[`  |
| `<R>`  | Right bracket `]` |
| `<LB>` | Left brace `{`    |
| `<RB>` | Right brace `}`   |

### Built-in Character Classes

| Class        | Characters                           |
|--------------|--------------------------------------|
| `LETTER`     | Unicode letters `\p{L}`              |
| `LABEL_CONT` | `LETTER` + digits + `_` + `-`        |

```
|c[LETTER]               | MARK           |>> :name_cont
|c[LABEL_CONT]           | ->             |>>
```

### Combined Character Classes

Multiple character specifications in one match:

```
|c[LETTER'[.?!*+]        ; Match: letter OR ' OR [ OR . OR ? OR ! OR * OR +
```


## Actions (Middle Column)

Actions are pipe-separated and execute left-to-right:

### Advance

```
| ->                     ; Advance one character
| ->[\n]                 ; Advance TO newline (SIMD memchr)
| ->[;|\n]               ; Advance TO any of these chars
```

### Accumulation

```
| MARK                   ; Mark current position as start of slice
| TERM                   ; Terminate slice (MARK to current position)
| TERM(-1)               ; Terminate slice ending 1 char before current
| PREPEND(|)             ; Prepend literal to accumulation
```

### Function Calls

```
| /function              ; Call function (void)
| /function(args)        ; Call with arguments
| var = /function        ; Capture return value
```

### Variables

```
| result = 0             ; Assignment
| depth += 1             ; Increment
| depth -= 1             ; Decrement
| col = COL              ; Capture current column
```

### Inline Literal Events

For emitting events with literal or pre-accumulated content:

```
| TypeName               ; Emit event with no payload (BoolTrue, Nil)
| TypeName(literal)      ; Emit with literal value (Attr($id), Attr(?))
| TypeName(USE_MARK)     ; Emit using current MARK/TERM content
```

Examples:
```
|c[?]        | Attr(?) | BoolTrue | ->    |return    ; Emit Attr "?", BoolTrue
|c[[]        | Attr($id) | /value         |>> :close ; Emit Attr "$id", parse value
|default     | TERM | BareValue(USE_MARK) |return    ; Emit BareValue with accumulated
```


## Transitions (Right Column)

```
|>>                      ; Self-loop (stay in current state)
|>> :state               ; Go to named state
|return                  ; Return from function
|return value            ; Return with specific value (for INTERNAL types)
```


## Conditionals

Single-line guards only - no block structure:

```
|if[condition]                            |return
|if[condition] | actions                  |>> :state
|                                         |>> :fallthrough  ; Next line is else
```

The condition is checked; if true, execute actions and transition. If false,
fall through to the next line.

Examples:
```
|state[:check_dedent]
  |if[COL <= elem_col]                    |return  ; Dedent detected
  |                                       |>> :child_dispatch

|state[:check_depth]
  |if[depth == 0] | TERM(-1)              |return
  |                                       |>> :main
```

### Condition Syntax

```
|if[COL <= elem_col]                      ; Comparison
|if[depth == 0]                           ; Equality
|if[parent_col >= 0 && line_col <= parent_col]  ; Compound
```


## Dedent Handling

The call stack naturally handles element nesting. Each element function:
1. Knows its own column (passed as parameter)
2. Loops looking for children
3. When it sees a line at column <= its column, returns WITHOUT consuming

```
|function[element:Element]  :elem_col :parent_col

  ; ... parse identity ...

  |state[:children]
    |c[\n]       | ->                     |>>
    |c[ ]        | col = /count_indent    |>> :check_child
    |default     | col = 0                |>> :check_child

  |state[:check_child]
    |if[col <= elem_col]                  |return  ; Dedent - don't consume!
    |                                     |>> :child_dispatch
```

When returning:
- BRACKET types auto-emit their End event
- Parent function sees the same line, does the same check
- Call stack unwinds naturally, emitting ElementEnd at each level


## Inferred EOF Handling

EOF behavior is **always inferred** - no explicit `|eof` cases needed. The
generator follows these rules in order:

1. **If MARK is active** → TERM (finalize any accumulation)
2. **If EXPECTS(x) declared** → emit unclosed error for x
3. **Based on return type:**
   - BRACKET types → emit End event
   - CONTENT types → emit the content event
   - INTERNAL types → no emission
4. **Return** from function

### Examples

**Simple content accumulation:**
```
|function[name:Name]  | MARK
  |state[:main]
    |c[LABEL_CONT]   | ->                 |>>
    |default         | TERM               |return
    ; EOF inferred: TERM (MARK active), emit Name, return
```

**Nested in bracket type:**
```
|function[element:Element]  :elem_col
  |state[:children]
    |c[\n]           | ->                 |>>
    |c[ ]            | col = /count       |>> :check
    |default         | col = 0            |>> :check
    ; EOF inferred: emit ElementEnd (BRACKET type), return
```

**With unclosed delimiter:**
```
|function[dquote:StringValue]  | MARK | EXPECTS(")
  |state[:main]
    |c["]            | TERM | ->          |return
    |c[\\]           | -> | ->            |>>
    |default         | ->                 |>>
    ; EOF inferred: TERM, emit unclosed_dquote error, emit StringValue, return
```

**Void function (no return type):**
```
|function[skip_whitespace]
  |state[:main]
    |c[ \t]          | ->                 |>>
    |default         |                    |return
    ; EOF inferred: just return (no type, no emission)
```

The generator walks the call stack on EOF, unwinding each function with its
appropriate End events, ensuring the event stream is always well-formed.


## Automatic SCAN Optimization

The generator automatically infers SIMD-accelerated scanning from state structure.

**Rule:** If a state has a self-looping default (`|default | -> |>>`), the
explicit character cases become SCAN targets.

```
|state[:prose]
  |c[\n]      | TERM                      |return
  |c[;]       | TERM | ->                 |>> :check_semi
  |c[|]       | TERM | ->                 |>> :check_pipe
  |c[!]       | TERM | ->                 |>> :check_bang
  |default    | ->                        |>>  ← triggers auto-SCAN
```

Generator infers: scan for `[\n;|!]` using memchr, then dispatch.

Generated code (conceptual):
```rust
match self.scan_to4(b'\n', b';', b'|', b'!') {
    Some(b'\n') => { /* newline case */ }
    Some(b';')  => { /* semicolon case */ }
    Some(b'|')  => { /* pipe case */ }
    Some(b'!')  => { /* bang case */ }
    None        => { /* EOF handling */ }
}
```

**Limitation:** States with character classes (LETTER, LABEL_CONT) cannot use
memchr SCAN since memchr works on specific bytes. These fall back to byte-by-byte.

### Explicit Advance-To

For explicit "skip to delimiter" operations, use `->[]`:

```
|c[\t]       | /error(no_tabs) | ->[\n]   |>> :line  ; Skip to newline
```

This also uses memchr but is an explicit action, not automatic state optimization.


## Built-in Functions

### Error Emission

Error is a built-in - no explicit function definition needed:

```
| /error(no_tabs)        ; Emit Error event with code "no_tabs"
| /error(unclosed_bracket)
```


## Complete Example

```
|parser udon

; Type declarations
|type[Element]       BRACKET
|type[Name]          CONTENT
|type[Text]          CONTENT
|type[Attr]          CONTENT
|type[StringValue]   CONTENT
|type[BoolTrue]      CONTENT
|type[INT]           INTERNAL

|entry-point /document

|function[document]
  |state[:line]
    |c[\n]           | ->                 |>>
    |c[ ]            | col = /count_indent |>> :dispatch
    |c[\t]           | /error(no_tabs) | ->[\n] |>> :line
    |default         | col = 0            |>> :dispatch

  |state[:dispatch]
    |c[|]            | ->                 |>> :check_pipe
    |default         | /prose(col, -1)    |>> :line

  |state[:check_pipe]
    |c[LETTER'[.?!*+] | /element(col, -1) |>> :line
    |default         | /prose_pipe(col, -1) |>> :line


|function[element:Element]  :elem_col :parent_col

  |state[:identity]
    |c[LETTER]       | /name              |>> :post_name
    |c[[]            | ->                 |>> :bracket
    |c[?!*+]         | /suffix            |>> :post_identity

  |state[:bracket]  | EXPECTS(])
    |c[]]            | ->                 |>> :post_bracket
    |default         | Attr($id) | /value |>> :bracket_close

  |state[:bracket_close]  | EXPECTS(])
    |c[]]            | ->                 |>> :post_bracket

  |state[:post_bracket]
    |c[?!*+]         | /suffix            |>> :post_identity
    |default         |                    |>> :post_identity

  |state[:post_identity]
    |c[\n]           | ->                 |>> :children
    |c[ \t]          | ->                 |>> :pre_content
    |default         | /inline_text(elem_col) |>> :children

  |state[:children]
    |c[\n]           | ->                 |>>
    |c[ ]            | col = /count_indent |>> :check_child
    |default         | col = 0            |>> :check_child

  |state[:check_child]
    |if[col <= elem_col]                  |return  ; Dedent
    |                                     |>> :child_dispatch

  |state[:child_dispatch]
    |c[|]            | ->                 |>> :child_pipe
    |default         | /prose(col, elem_col) |>> :children

  |state[:child_pipe]
    |c[LETTER'[.?!*+] | /element(col, elem_col) |>> :children
    |default         | /prose_pipe(col, elem_col) |>> :children


|function[name:Name]  | MARK
  |state[:main]
    |c[LABEL_CONT]   | ->                 |>>
    |default         | TERM               |return


|function[suffix]
  |state[:main]
    |c[?]            | Attr(?) | BoolTrue | -> |return
    |c[!]            | Attr(!) | BoolTrue | -> |return
    |c[*]            | Attr(*) | BoolTrue | -> |return
    |c[+]            | Attr(+) | BoolTrue | -> |return
    |default         |                    |return


|function[count_indent:INT]  | result = 0
  |state[:main]
    |c[ ]            | -> | result += 1   |>>
    |default         |                    |return result
```


## Summary of Key Differences from Current genmachine

| Aspect | Current | v4 |
|--------|---------|-----|
| Event emission | Explicit `emit(Type)` | Inferred from return type |
| EOF handling | Explicit `\|eof` cases | Inferred from context |
| SCAN optimization | Manual `SCAN(chars)` annotation | Automatic from state structure |
| Element stack | Explicit stack management | Implicit via call stack |
| Helper functions | Many small functions for literals | Inline `TypeName(literal)` |
| Conditionals | `\|if` / `\|endif` blocks | Single-line guards |

The result: a cleaner, more declarative DSL that describes **what** to parse,
while the generator figures out **how** to do it efficiently.


## Runtime Architecture: Callback-Based

Based on proof-of-concept benchmarks (see `gen-parser-poc/`), the v4 generator
will produce **callback-based parsers** rather than ring-buffer state machines.

### Why Callback-Based?

Benchmark results comparing three architectures:

| Input Size | Callback | Ring Buffer | Generator (genawaiter) |
|------------|----------|-------------|------------------------|
| Small (21B) | **2.14 GiB/s** | 318 MiB/s | 298 MiB/s |
| Medium (266B) | **1.70 GiB/s** | 830 MiB/s | 689 MiB/s |
| Large (~92KB) | **2.24 GiB/s** | 1.50 GiB/s | 1.02 GiB/s |
| Deep (50 levels) | **2.66 GiB/s** | 1.82 GiB/s | 1.54 GiB/s |

**Callback is 2-7x faster** across all test cases.

### Why Callback Wins

1. **No buffering overhead** - events delivered directly to consumer
2. **No iterator protocol** - no `next()` virtual dispatch
3. **True recursion** - compiler optimizes the call graph beautifully
4. **Inlineable callback** - `FnMut` gets monomorphized

### Generated API

The v4 generator will produce parsers with this signature:

```rust
impl<'a> Parser<'a> {
    /// Parse document, calling callback for each event.
    /// Backpressure: callback can block (e.g., send to bounded channel).
    pub fn parse<F>(self, on_event: F)
    where
        F: FnMut(Event<'a>);
}
```

For consumers needing iterator semantics, a thin adapter can wrap this:

```rust
// Channel-based iterator adapter
let (tx, rx) = sync_channel(1024);
thread::spawn(move || parser.parse(|e| tx.send(e).unwrap()));
for event in rx { /* ... */ }
```

### Backpressure Strategy

With callback-based parsing:

1. **Blocking callback** (recommended) - callback sends to bounded channel,
   blocks when full. Implicit backpressure, simple code.

2. **Large buffer** - for in-memory processing, just collect events.
   No backpressure needed if document fits in memory.

3. **Async integration** - future work could add async callback support
   for non-blocking backpressure.

### Trade-off Accepted

The callback approach does NOT support explicit pause/resume mid-parse
(without blocking). This is acceptable because:

- Blocking backpressure via channels works well in practice
- True streaming (pause/resume) would require complex state serialization
- The 2-7x performance gain outweighs this limitation
