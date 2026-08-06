<!--
Style, formatting, tests, changelog, coverage, examples, error-code docs and
budgets are all machine-checked — see docs/gates.md. There is nothing to promise
here about them: either the gates are green or the work is not done.

What is left is what a machine cannot judge. Please answer only that.
-->

## Intent

What changes for the user, and why now. Link the epic or ADR.

Closes #

## Boundaries

Did this stay inside the epic? Anything touched that was declared out of scope,
and why.

## Invariants

Tick what applies, and say what you did about it.

- [ ] **Syntax / grammar** — existing `.omni` source still parses and means the same
- [ ] **Spec IR** — regenerated `types.ts` + `ir.schema.json`, version decided per `docs/19-versioning-policy.md`
- [ ] **Diagnostic codes** — new codes in the right phase range; none renumbered or reused
- [ ] **Target parity** — `typescript | rust | python | go` all still build the reference spec
- [ ] **Cache / lock formats** — `.omni-cache/`, `omni.lock` still readable by the previous version
- [ ] Not applicable — this change cannot touch any of the above, because: …

## Irreversibility

- [ ] Everything here is reversible by a revert.
- [ ] This contains an irreversible change (syntax, IR, removed code/flag, published
      artifact). ADR: `docs/adr/…`

**Rollback plan:**

## What a reviewer should actually look at

Point at the two or three places where judgment is needed. If the honest answer
is "nothing — the gates cover this", say that; it is a good answer.
