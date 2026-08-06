# ADR-0003: Monorepo layout

- **Status.** Accepted
- **Date.** 2026-05-25
- **Deciders.** Project owner

## Context

The language, the analyzer, the CLI, the LSP, the TypeScript runtime, the
tree-sitter grammar and the docs all move together. Splitting them across repos
would mean coordinating a version bump for every change that crosses the
Rust↔TS boundary — which is most of them.

## Decision

Keep everything in one repository: `crates/*`, `runtime/`, `tree-sitter-omnilang/`,
`docs/`, `examples/`.

## Consequences

One PR can change the IR, the generated TypeScript types and the docs together,
so the drift gate can be a simple `git diff --exit-code`. The cost is a CI run
that builds everything, and a release process that must version several
artifacts from one tree.

## Reversibility

Cheap to reverse for a leaf component, expensive for `crates/` ↔ `runtime/`.
