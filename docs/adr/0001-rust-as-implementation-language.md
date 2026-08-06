# ADR-0001: Rust as the compiler implementation language

- **Status.** Accepted
- **Date.** 2026-05-25
- **Deciders.** Project owner

## Context

The compiler frontend (lexer, parser, analyzer, Z3 integration) must be fast,
distributable as a single binary, and stable enough to be called from CI, an
LSP server and a Node.js runtime. Backfilled from the decisions log.

## Decision

Implement the compiler in Rust as a Cargo workspace.

## Consequences

Single static binary, no runtime dependency for `omni check`; strong types for
the AST and Spec IR; `schemars`/`serde` give a generated JSON Schema for the IR
for free. The cost is a two-language project — the orchestration runtime stays
TypeScript — which makes the Rust↔TS boundary the most fragile seam in the
system. That seam is why the IR drift gate exists.

## Reversibility

Effectively irreversible. Everything downstream, including the IR export path,
assumes it.
