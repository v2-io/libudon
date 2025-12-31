# libudon TODO

See `PLAN.md` for the phase-3 roadmap.

## Immediate: Nomenclature & Grammar Fixes

Per `~/src/udon/SPEC-UPDATE.md`, the parser needs terminology and grammar fixes:

### Terminology Refactor

Rename functions in `udon.desc` to use consistent positional terminology:

| Old Name | New Name | Purpose |
|----------|----------|---------|
| `inline_attr` | `sameline_attr` | Attr on element line |
| `bare_value_inline` | `bare_value_sameline` | Sameline attr values |
| `inline_text` | `sameline_text` | Text on element line |
| `inline_text_pipe` | `sameline_text_pipe` | After `\|` on element line |
| `inline_text_bang` | `sameline_text_bang` | After `!` on element line |

### Value Terminator Fixes

Current terminators are wrong - `}` and `]` terminate in wrong contexts:

| Context | Current | Correct |
|---------|---------|---------|
| Block attr | `\n ]}` | `\n` or ` ;` |
| Sameline attr | `\n :\|]}` | `\n ␣` |
| Embedded attr | (same) | `\n ␣ }` |
| Array item | (same) | `\n ␣ ]` |

Need new/renamed functions:
- `bare_value_block` - terminates `\n` or ` ;`
- `bare_value_sameline` - terminates `\n ␣`
- `bare_value_embedded` - terminates `\n ␣ }`
- `bare_value_array` - terminates `\n ␣ ]`

### Comment Handling

- Block prose: `;` is literal (no change needed)
- Sameline prose: `;` starts comment (verify this works)
- Block attr: ` ;` (space-semicolon) starts comment
- Escape: `\;` in sameline, quote in block attr

## Testing

- [ ] New test infrastructure for descent event model
- [ ] Tests derived from SPEC.md, not implementation
- [ ] Cover edge cases from SPEC-UPDATE.md terminology

## From Previous Review

- **Brace depth**: Ensure arbitrary depth works (counter or recursion)
- **Span accuracy**: MARK positions for suffix/ID/class attributes

## Cleanup

- [ ] Remove `udon-core/src/values_parser.rs` (obsolete - values.desc now concatenated)
- [ ] Update CLAUDE.md with new terminology
