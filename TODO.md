# libudon TODO

See `implementation-phase-3.md` for the current roadmap.

## Historical Note

The previous TODO.md contained issues from a Codex code review (Dec 2025) of the old
ring-buffer/state-machine architecture. That architecture has been completely replaced
by descent-generated recursive descent parsing. Most issues are now obsolete:

- **Backpressure (#1)**: Eliminated by callback-based design (no ring buffer)
- **Streaming resume (#4)**: Different approach in descent (multi-chunk support)
- **Brace depth limit (#6)**: Still relevant - implement properly in udon.desc
- **Unused scaffolds (#8-10)**: Deleted with old architecture
- **Testing deficiency**: Still relevant - tests must derive from SPEC, not implementation

The relevant items have been integrated into implementation-phase-3.md.
