# ADR-0010: Commit `Cargo.lock` and `runtime/package-lock.json`

- **Status.** Accepted
- **Date.** 2026-08-06
- **Deciders.** Project owner

## Context

Both lockfiles were in `.gitignore`. Three things followed, none of them
intended:

1. `npm ci --prefix runtime` in CI cannot run without a committed
   `package-lock.json` — it refuses by design.
2. `actions/setup-node`'s `cache-dependency-path: runtime/package-lock.json`
   pointed at a file that was never checked out.
3. The Rust cache key in `.github/actions/setup-omni/action.yml` is
   `hashFiles('**/Cargo.lock')`, which hashed nothing.

None of this was visible, because the workflow itself had been failing at 0s
since 2026-05-26 (see ADR-0011).

A workspace that ships binaries and promises reproducible builds (roadmap task
07, `omni build --frozen`, `omni.lock`) cannot leave its own dependency graph
floating.

## Decision

Track `Cargo.lock` and `runtime/package-lock.json`. CI builds with `--locked`
and `npm ci`, so a stale lockfile fails the build instead of silently resolving
to something new.

## Consequences

Dependency bumps become explicit commits — Dependabot is configured to raise
them weekly. `--locked` will fail loudly when someone changes `Cargo.toml`
without refreshing the lock; that is the point.

## Reversibility

Trivially reversible, but reversing it re-breaks `npm ci`.
