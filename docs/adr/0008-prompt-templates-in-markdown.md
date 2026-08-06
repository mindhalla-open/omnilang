# ADR-0008: Prompt templates live in `.md` files

- **Status.** Accepted
- **Date.** 2026-05-26
- **Deciders.** Project owner

## Context

Prompts embedded as inline string literals produced escaping bugs, unreadable
diffs, and no way to review a prompt change as a prompt change.

## Decision

Keep prompt templates in `.md` files loaded at runtime, not in string literals.

## Consequences

Prompt changes are reviewable as text and diffable. Templates become an
input to reproducibility: the same spec plus a different prompt is a different
build, so template changes belong in the changelog.

## Reversibility

Cheap to reverse, and no reason to.
