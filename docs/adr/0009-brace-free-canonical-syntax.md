# ADR-0009: Brace-free canonical syntax (`syntax_version 0.2`)

- **Status.** Accepted
- **Date.** 2026-05-29
- **Deciders.** Project owner

## Context

The grammar supported both `{}` blocks and indentation-based blocks. Two ways to
write the same spec split the examples, the docs and the formatter, and gave
agents an ambiguous target to imitate.

## Decision

Indentation-based (brace-free) layout is canonical. `package.syntax_version` is
pinned to `0.2`. The parser continues to accept `{}` blocks for backward
compatibility; the formatter emits only the canonical form.

## Consequences

Because the parser still accepts braces, nothing in the compiler stops brace
syntax from creeping back into docs and examples — and it did: two occurrences
survived the 2026-06 conversion and one was still in
`docs/04-syntax-reference.md` two months later. Enforcement lives in the
`syntax-canon` gate, not in the parser.

## Reversibility

The canon is a convention and could change; the compatibility promise for
existing brace specs cannot be withdrawn without breaking users.
