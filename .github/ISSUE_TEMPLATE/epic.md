---
name: Epic contract
about: The entry point for work. One intent, one owner, delivered as a vertical slice.
title: '[Epic] '
labels: epic
---

<!--
An epic without all five sections is a wish, not an epic — do not start it.
The rule for acceptance criteria: if a criterion cannot be expressed as a check,
either rewrite it, or mark it "human" and say why a machine cannot judge it.
-->

**Intent.** What changes for the user of OmniLang, and why now. One or two sentences.

**Owner.** One person, from here to a released artifact.

## Boundaries

- In scope:
- Out of scope (explicitly not touched):

## Invariants

- **Syntax / grammar compatibility:** does existing `.omni` source still parse and mean the same thing?
- **Spec IR:** does the IR change? If yes — regenerated `runtime/src/types.ts` and `ir.schema.json`, plus a version decision per `docs/19-versioning-policy.md`.
- **Diagnostic codes:** new codes allocated in the right phase range and documented in `docs/18-error-codes.md`. No renumbering, no reuse.
- **Target parity:** which of `typescript | rust | python | go` are affected? All four must stay green.
- **Cache / lock formats:** `.omni-cache/`, `omni.lock` — readable by the previous version?
- **Conventions that must not be reinvented:** see `CLAUDE.md`.

## Acceptance criteria (the oracle)

<!-- Name the actual job or command that proves each one. -->

1. Machine-checked: <test / gate / command>
2. Machine-checked: <test / gate / command>
3. Human-checked (and why it cannot be automated): …

## Risk and rollback

- **Blast radius:** who breaks if this is wrong — spec authors, generated code already in users' repos, one target, the runtime?
- **Irreversible steps:** anything from the list in `CLAUDE.md` → needs an ADR in `docs/adr/` **before** implementation. Link it here.
- **Rollback plan:** revert / feature flag / version pin. How long does undoing it take?
- **Pre-mortem:** what breaks first, and who notices — CI, a user's build, or nobody until the next release?
