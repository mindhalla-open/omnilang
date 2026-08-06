# ADR-0011: Gate-based development process

- **Status.** Accepted
- **Date.** 2026-08-06
- **Deciders.** Project owner

## Context

Most code here is written by AI agents. The classic controls — a review that
reads the whole diff, a checklist a contributor promises to have followed, an
agreement reached in conversation — do not survive that. An agent never heard
the agreement, and a reviewer's attention does not scale with generation speed.

The failure mode is not theoretical. From 2026-05-26 to 2026-08-06 the entire CI
pipeline failed at 0s on every run: a step used `if: ${{ secrets.X != '' }}`,
which is not a valid context in `if:` and invalidates the whole workflow. Five
PRs merged during the outage with no gate running at all. The PR checklist still
asked contributors to confirm that `cargo clippy` passed.

## Decision

Adopt the process in `docs/21-engineering-process.md`:

- Conventions are executable, in `scripts/gates/`, runnable by one command
  locally and in CI. A rule that exists only in prose is not a rule.
- A ritual may only be removed in exchange for a gate that catches what the
  ritual was for. Checklist items covered by CI are deleted from the PR template.
- Human review targets risk zones — boundaries, invariants, irreversibility,
  fidelity to intent — not the whole diff.
- Every incident becomes a gate. `scripts/gates/workflows.sh` is the first one,
  and it exists because of the outage above.
- Decisions with a long horizon become ADRs; irreversible changes need one
  before the code.

## Consequences

Adding a convention now costs a script, not a paragraph. The gate suite is
itself a maintained artifact and can rot — `run-all.sh` fails when a gate has
nothing to check, rather than passing vacuously.

## Reversibility

Fully reversible; the gates are ordinary shell scripts.
