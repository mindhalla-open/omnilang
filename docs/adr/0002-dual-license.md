# ADR-0002: Dual license Apache-2.0 OR MIT

- **Status.** Accepted
- **Date.** 2026-05-25
- **Deciders.** Project owner

## Context

The project targets the Rust and JavaScript ecosystems, where permissive dual
licensing is the norm and a prerequisite for corporate adoption.

## Decision

License the project as Apache-2.0 OR MIT, matching the Rust project's own
convention. Contributions are accepted under both.

## Consequences

Maximum downstream compatibility; patent grant available via Apache-2.0 for
users who need it. Both `LICENSE-APACHE` and `LICENSE-MIT` must ship in every
distribution.

## Reversibility

Irreversible in practice — relicensing needs the agreement of every contributor.
