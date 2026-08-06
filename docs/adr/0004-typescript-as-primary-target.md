# ADR-0004: TypeScript as the primary codegen target

- **Status.** Accepted, extended
- **Date.** 2026-05-25 (extended 2026-05-29)
- **Deciders.** Project owner

## Context

Generated code needs a target where LLMs are strongest and where verification
tooling is cheapest to run in CI.

## Decision

Make TypeScript the primary target and the default for `omni build`.

## Extension (2026-05-29)

Rust, Python and Go were added as first-class targets. Parity is enforced: the
conformance matrix builds `examples/hybrid_billing.omni` on all four, and
`E0242` rejects any other target in a spec.

## Consequences

"Multi-target" is now a promise the project has to keep on every change — a
feature that only lands on TypeScript is unfinished.

## Reversibility

Adding a target is cheap. Removing one breaks specs in the wild: irreversible.
