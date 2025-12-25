# CLAUDE.md - Agent Guidelines for libudon

This is the core UDON parser library in Rust. It produces a C-compatible shared
library that language bindings (Ruby, Python, etc.) link against.

## Implementation Plan

**See `~/src/udon/implementation-phase-2.md` for the comprehensive roadmap.**

Current state: ~30% of SPEC.md implemented. Phase 2 focuses on:
- Complete parser (all SPEC.md features)
- True streaming with ring buffer
- Arena-allocated tree structure
- World-class error messages

## Critical: Use SPEC.md as Authority

The UDON specification lives at `~/src/udon/SPEC.md`. This is the **only**
authoritative source for syntax rules.

Do NOT use:
- `generator/_archive/` files (old C-era, outdated)
- Guesses about "obvious" behavior
- Other UDON-like formats as reference

When SPEC.md is ambiguous, ask Joseph for clarification.

## Architecture

```
libudon/
├── udon-core/           # Core parser library
│   └── src/
│       ├── lib.rs       # Public API
│       ├── event.rs     # Event enum (hand-written, stable)
│       ├── parser.rs    # Generated state machine
│       ├── value.rs     # Attribute value parsing
│       └── span.rs      # Source locations
├── udon-ffi/            # C ABI bindings (cdylib)
│   └── src/lib.rs
├── generator/           # Parser generator
│   ├── udon.machine     # Current state machine (authoritative)
│   ├── genmachine-rs    # Generator script (Ruby)
│   ├── templates/       # Liquid templates for code generation
│   └── _archive/        # Old C-era files (not authoritative)
└── examples/            # Test fixtures
```

## Workflow: Modifying the Parser

1. **Edit `generator/udon.machine`** — The declarative state machine DSL
2. **Regenerate**: `./generate-parser.sh`
3. **Build**: `cargo build`
4. **Test**: `cargo test`

Do NOT edit `udon-core/src/parser.rs` directly — it's generated.

## The .machine DSL

The state machine DSL uses a pipe-delimited format:

```
|function[name]
  |state[:state_name]
    |c[chars]   |.label  | actions               |>> :next_state
    |default    |.label  | actions               |>> :next_state
```

Key patterns:
- `|c[x]` — Match character 'x'
- `|c[\n]` — Match newline
- `|c[ \t]` — Match space or tab
- `|default` — Fallback case
- `|eof` — End of input
- `|>> :state` — Transition to state
- `|return` — Return from function
- `| emit(EventType)` — Emit an event
- `| MARK` — Mark current position
- `| TERM` — Terminate slice (MARK to current)

## Key Files

| File | Purpose | Edit? |
|------|---------|-------|
| `generator/udon.machine` | State machine definition | YES |
| `udon-core/src/parser.rs` | Generated parser | NO (regenerate) |
| `udon-core/src/event.rs` | Event enum | YES (stable API) |
| `udon-core/src/value.rs` | Value type parsing | YES |
| `generator/templates/parser.rs.liquid` | Code gen template | YES (carefully) |

## Design Decisions

1. **Zero-copy**: Events contain `&'a [u8]` slices into input buffer
2. **SIMD scanning**: Uses `memchr` crate for fast character search
3. **Event-based**: Parser emits events, doesn't build AST (yet)
4. **Generated**: Parser code generated from DSL for maintainability

## Known Issues / TODOs

Current parser bugs (tests exist in udon-ruby):
- Indented attributes parsed as text (should be `:attribute` events)
- Integer values returned as strings (should be `Value::Integer`)

Missing features (see SPEC.md):
- Element suffixes (`?`, `!`, `*`, `+`)
- Embedded elements (`|{...}`)
- Directives (`!name`, `!raw:lang`)
- Column-aligned siblings
- Freeform blocks (triple backtick)

## Testing

```bash
cargo test                    # All tests
cargo test --lib              # Unit tests only
cargo test parsing            # Tests matching "parsing"
```

## Performance Profiling

```bash
cargo build --release
cargo bench                   # Criterion benchmarks (if configured)
```

## Related Repositories

- `~/src/udon` — Specification, examples, implementation plan
- `~/src/udon-ruby` — Ruby gem using this library
