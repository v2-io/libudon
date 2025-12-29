# genmachine DSL Analysis

A retrospective analysis of the original C-era genmachine design, what made it
elegant, and where the Rust port diverged.

## Original Design Philosophy

The original genmachine DSL (see `_archive/udon-c-era.machine` and `genmachine`)
was designed around **recursive descent parsing** with these principles:

1. **Functions return typed values** - not void, not events, actual data
2. **Explicit data flow** - MARK/TERM for accumulation, assignment for structure
3. **Recursion matches grammar** - nested UDON elements = nested function calls
4. **Minimal state per function** - small states, complexity via composition

### The Numbers Tell the Story

| Metric           | Original C-era | Current Rust Port |
|------------------|----------------|-------------------|
| Lines            | 104            | 1,901 (18x)       |
| Functions        | 6              | 4                 |
| States           | 14             | 231 (16x)         |
| States/Function  | ~2.3           | ~58               |

The original had **more functions with fewer states each**. The port has **fewer
functions with massive state tables**. This is the opposite of recursive descent.


## DSL Syntax Reference

### Document Structure

```
|parser udon                          ; Parser name (required first)

|enum[EnumName]                       ; Enum type definition
  |VARIANT_A |VARIANT_B               ;   Variants (become PARSER_VARIANT_A, etc.)

|struct[StructName]                   ; Struct type definition
  |field_name     FieldType           ;   field and type (see Type System)
  |other_field    STRING

|struct[Child < Parent]               ; Struct inheritance
  |extra_field    Z+

|entry-point /function:state          ; Where parsing begins
```

### Type System

| Type      | Meaning                           | C Output              |
|-----------|-----------------------------------|-----------------------|
| `STRING`  | Accumulated byte slice            | `ParserString *`      |
| `LIST`    | Linked list of nodes              | `ParserList *`        |
| `DICT`    | Key-value dictionary              | `ParserDict *`        |
| `Z+`      | Positive integer                  | `uint64_t`            |
| `TypeName`| User-defined struct/enum          | `ParserTypeName *`    |

### Functions

```
|function[name:ReturnType]  | local=init | MARK
           │      │              │          │
           │      │              │          └─ Init commands (run at entry)
           │      │              └─ Local variable declarations
           │      └─ Return type (allocates S of this type)
           └─ Function name (becomes _parser_name)
```

**Special variables available in functions:**

| Variable | Meaning                              |
|----------|--------------------------------------|
| `S`      | Self result - the value being built  |
| `C`      | Shorthand for `S.children`           |
| `COL`    | Current column (1-indexed)           |
| `LINE`   | Current line (1-indexed)             |
| `CURR`   | Current character `*(p->curr)`       |

### States and Cases

```
|function[example:Node]
  |eof                              | TERM        |return S    ; Function-level EOF

  |state[:main]                                                ; State definition
    |eof                            | ...         |return S    ; State-level EOF
    |c[\n]       |.newline          | ->          |>> :next    ; Match newline
    |c[ \t]      |.whitespace       | ->          |>>          ; Match space/tab, self-loop
    |c[<P>]      |.pipe             | ...                      ; Match literal |
    |c[<L>]      |.lbracket         | ...                      ; Match literal [
    |c[<R>]      |.rbracket         | ...                      ; Match literal ]
    |c[abc]      |.letters          | ...                      ; Match a, b, or c
    |default     |.other            | ...         |>> :error   ; Fallback case

  |state[:next]
    ...
```

**Character escape sequences:**

| Syntax | Character |
|--------|-----------|
| `<P>`  | `\|` pipe |
| `<L>`  | `[` left bracket |
| `<R>`  | `]` right bracket |
| `\n`   | newline |
| `\t`   | tab |

**Substates (`.label`):**

The `.label` after a case creates a named entry point within the state:
- Generates a labeled jump target: `s_main__newline:`
- Can be jumped to directly: `|>> :main.newline`
- Useful for re-entering a state at a specific point

### Accumulation: MARK and TERM

For STRING-returning functions, MARK and TERM define the slice:

```
|function[label:STRING]   | MARK              ; Start accumulating at function entry
  |eof                    | TERM   |return S  ; End accumulation, return string
  |state[:main]
    |c[ \t]   |.done      | TERM   |return S  ; Space/tab ends the label
    |c[\n]    |.done      | TERM   |return S  ; Newline ends the label
    |default  |.collect   | ->     |>>        ; Collect character, continue
```

**MARK** - Records current position as start of slice
**TERM** - Finalizes slice from MARK to current position

With explicit variable:
```
| MARK(varname)           ; Start accumulating into specific variable
| TERM(varname)           ; End accumulation of specific variable
```

### Control Flow

**Advance patterns:**
```
| ->                      ; Advance one character
| ->[chars]               ; Advance TO first occurrence of any char (SCAN)
| [chars]->               ; Advance PAST these characters
| [\n]->                  ; Common: advance to newline
```

**State transitions:**
```
|>> :state                ; Go to state (with EOF check if needed)
|>> :state.substate       ; Go to specific substate within state
|>>                       ; Self-loop (stay in current state)
|return S                 ; Return from function with value S
|return                   ; Return void
```

**Conditionals:**
```
|if[COL <= ipar]          | return S
|elsif[COL == ibase]      | ...
|else                     | ...
|endif
```

**Errors:**
```
|err Message text here    ; Emit error and abort
```

### Building Structures

**Field assignment:**
```
S.name = /label           ; Call /label function, assign result to S.name
S.node_type = NORMAL      ; Assign enum constant
S.column = COL            ; Assign current column
```

**Function calls as values:**
```
S.name = /label           ; Assign return value
S.id = /id:delim          ; Call /id starting at :delim state
```

**List operations:**
```
C << /node                ; Append result of /node to S.children
S.classes << /label       ; Append to arbitrary list field
C <<< /value              ; Append and finalize (for string values)
```

**Dictionary operations:**
```
S.attributes{key, value}          ; Add key-value pair
S.attributes{/label, /value}      ; Key and value from function calls
S.attributes{g.name, g}           ; Key from field, value is whole struct
```


## Example: Original vs Port

### Original: Parsing a label (element name)

```
|function[label:STRING]   | lvl=0 | MARK
  |eof                    | TERM                   |return S
  |state[:init]
    |c[(]     |.delim     |                        |>> :delim.nest
    |c[ \t\n<L><P>.!]|.done| TERM                  |return S
    |default  |.collect   | ->                     |>>
  |state[:delim]
    |eof      |err Unexpected end of file - missing closing ')'
    |c[(]     |.nest      | -> | lvl+=1            |>>
    |c[)]     |.unnest    | -> | lvl-=1
      |if[lvl==0]         | TERM                   |return S
      |else                                        |>>
    |default  |.collect   | ->                     |>>
```

**What this does:**
1. Function returns STRING, starts MARK at entry
2. Normal mode: collect until whitespace/special char, TERM and return
3. Delimited mode `(...)`: track nesting depth, collect until balanced

**14 lines, 2 states, clear logic.**

### Port: Would need to flatten this into...

The port would need:
- A state for each case (no function return values)
- `emit(LabelStart)`, `emit(LabelEnd)` or similar
- Generator translates emit() to actual event code
- Inline the delimited parsing or create more states

Result: More states, less clarity, emit() abstraction.


## What the Original Got Right

### 1. Functions as Abstraction Boundaries

```
|function[node:Node]
  |state[:init]
    |default  |.name      | S.name=/label          |>> :identity
```

The `/label` call is a complete abstraction:
- Parses a label (however complex that is)
- Returns a STRING
- Caller just assigns it

No need to know *how* label parsing works. No shared state machine states.

### 2. Return Values for Data Flow

```
S.name = /label
S.id = /id
C << /node
```

Data flows via return values and assignment. You can trace exactly where each
piece of data comes from and goes to.

### 3. Recursion Matches Structure

```
|state[:child]
  |c[<P>]   |.node      | -> | C << /node        |>> :child
```

Child elements are parsed by recursively calling `/node`. The call stack
naturally tracks nesting depth. No explicit element stack needed.

### 4. Explicit Accumulation

```
| MARK                    ; "I'm starting to accumulate here"
| TERM                    ; "I'm done accumulating"
|return S                 ; "Here's what I accumulated"
```

No hidden state. No implicit operations. The DSL shows exactly when
accumulation starts and ends.


## What the Port Got Wrong

### 1. Flattened Recursion into State Machine

Instead of:
```
/element calls /label → gets name
/element calls /element → gets child (recursion!)
```

The port has:
```
231 states in a flat switch
No recursive calls
Explicit element_stack to track nesting
```

The call stack was replaced with manual stack management.

### 2. Invented emit() Abstraction

Original:
```
| TERM                    |return S
```

Port:
```
| emit(Text)              |>> :next
```

The `emit(Text)` hides:
- The TERM operation (implicit)
- Span creation (implicit)
- Event construction (in generator)
- Ring buffer push (in template)

100+ lines of generator code to translate emit() into actual operations.

### 3. Lost Substate Jump Targets

Original:
```
|>> :main.collect         ; Jump to .collect case within :main
```

Port:
- Substates parsed but not functional
- Can only jump to state starts
- Must create separate states for each entry point

### 4. No Return Values

Original functions return typed values. The port's functions return void and
push events to a buffer. This breaks the natural data flow that made the
original readable.


## Lessons for a New Approach

### 1. Embrace Recursive Descent

Rust functions CAN call each other recursively. Use that:

```rust
fn parse_element(&mut self) -> Element {
    let name = self.parse_label();
    let children = self.parse_children(); // Recursive!
    Element { name, children }
}
```

### 2. SAX Events at Function Boundaries

If streaming events are needed, emit them at function entry/exit:

```rust
fn parse_element(&mut self) {
    let name = self.parse_label();        // Returns value
    self.emit(ElementStart { name });     // Emit on entry
    self.parse_children();                // Recurses
    self.emit(ElementEnd);                // Emit on exit
}
```

The call stack guarantees matched Start/End events.

### 3. Keep MARK/TERM Explicit

Whether building structs or emitting events, string accumulation should be
visible in the DSL:

```
| MARK
| ... advance ...
| TERM -> content                         ; Explicit: TERM produces content
| emit ElementStart(name=content)         ; Explicit: what we emit with what
```

### 4. Generator Should Be Dumb

The generator should translate DSL → target language mechanically:
- No 100-line case statements
- No implicit operations
- DSL says what to do; generator just translates syntax

### 5. Consider the Call Stack for Backpressure

With recursive descent, parse state lives in the call stack. For true streaming
with pause/resume:
- Check buffer after each emit, yield if full
- On resume, call stack is intact, continue where we left off
- Or: size buffer large enough that backpressure is rare

This is actually simpler than the current approach, which drops events silently.


## Files Reference

| File | Description |
|------|-------------|
| `_archive/udon-c-era.machine` | Original UDON parser definition (C-era) |
| `_archive/udon-wip.machine` | Work-in-progress during port (shows emit() introduction) |
| `genmachine` | Original generator (Ruby, outputs C) |
| `genmachine-rs` | Current generator (Ruby, outputs Rust) |
| `udon.machine` | Current UDON parser definition (1901 lines, 231 states) |
| `templates/parser.rs.liquid` | Rust template for generated parser |
