# ADR-0007: Hybrid approach — infra config split out, natural-language invariants

- **Status.** Accepted
- **Date.** 2026-05-26
- **Deciders.** Project owner

## Context

Early specs tried to express infrastructure configuration and every invariant in
formal syntax. Both fought the language: infra belongs in existing tools, and
many real invariants are not machine-checkable at all.

## Decision

Separate infrastructure configuration out of the spec, and allow invariants to
be written in natural language, marked as such.

## Consequences

The analyzer distinguishes formally verifiable constraints from natural-language
ones and says so (`E0303`, `E0305`, `E0306` warn that a constraint cannot be
statically verified). Verification is honest about its own boundary instead of
pretending everything is proven. `examples/hybrid_billing.omni` is the reference.

## Reversibility

Changing this changes spec semantics: irreversible for existing specs.
