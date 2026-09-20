# Engineering Process

How OmniLang is built. Short version: **the process is a mechanism for managing
the risk of an unnoticed error, not a mechanism for distributing labour.**

Writing code stopped being the bottleneck here — most of it is generated. What
did not get cheaper is stating the intent clearly and checking the result. So
the process spends nothing on coordinating work and everything on those two.

## The rule everything follows

> **A ritual may be dropped only in exchange for a gate.** Every ceremony that
> disappears is replaced by a machine check that catches what the ceremony was
> for. Without the replacement, nothing was made faster — the risk just moved to
> the release.

There is a worked example in this repo's own history. From 2026-05-26 to
2026-08-06 the CI pipeline failed at 0s on every single run — an invalid
`secrets` context in a step-level `if:` — and five PRs merged with no gate
running at all, while the PR template asked contributors to confirm that clippy
passed. The ceremony was intact. The gate was gone. Nobody noticed for ten weeks.
That failure is now `scripts/gates/workflows.sh`.

The sequel matters just as much. The run that was supposed to confirm that fix
was red too — a test needed pytest before the step that installed it — and it
stayed red for six more weeks, because the fix had been judged from a local run
and nobody opened the CI page. A gate nobody looks at is a ritual with extra steps.

## The unit of work is an intent, not a task

One epic = one intent = one owner = one vertical slice, carried from `.omni`
syntax through the analyzer, the IR, the runtime, all four targets, the docs and
an example — to a released artifact. Not "the parser part, then the runtime
part". Splitting by component produces components that do not fit together;
splitting by risk produces changes you can undo.

Start from the template in `.github/ISSUE_TEMPLATE/epic.md`. It asks for five
things, and an epic missing any of them is not ready to start:

1. **Intent** — what changes for the user, and why now.
2. **Boundaries** — what is explicitly not touched.
3. **Invariants** — grammar compatibility, Spec IR, diagnostic codes, target
   parity, cache and lock formats.
4. **The oracle** — for each acceptance criterion, the command or job that proves
   it. If a criterion cannot be expressed as a check, rewrite it or mark it
   "human" and say why.
5. **Risk and rollback** — blast radius, irreversible steps, how long undoing takes.

That artifact is what an agent is given. A human contributor asks a clarifying
question when something is missing; an agent confidently invents the answer. This
is why the specification is the asset and the code is not.

## Decompose by risk, not by effort

The question is never "how many days and who does it". It is **"what here is
irreversible, and what breaks on release"**.

Irreversible in this project — because specs and generated code exist outside
this repo:

- syntax or grammar changes; `package.syntax_version`
- Spec IR schema; `omni.lock` and `.omni-cache/` formats
- removing or renumbering a diagnostic code; removing a CLI flag or subcommand
- anything published: crates.io, npm, Homebrew, the GitHub Action

These require an ADR in `docs/adr/` **before** the implementation. Everything
else is reversible: ship it and iterate.

## Where human attention goes

Review does not read the diff evenly. Formatting, boilerplate and trivial tests
are the machine's job — attention spent there is a process defect, not
diligence. A reviewer reads four things:

1. **Boundaries** — did this stay inside the epic?
2. **Invariants** — is a contract broken?
3. **Irreversibility** — is something here impossible to undo?
4. **Fidelity to intent** — does this do what the epic asked?

The PR template asks for exactly those. Everything the gates cover was deleted
from it; a checkbox duplicating a gate teaches people to tick without reading.

## Every incident becomes a gate

An error that reaches a release is not closed by a conversation or a promise to
be more careful. It is closed by a test, a lint or a check that will not let it
through again, added in the same change as the fix, with a row in `docs/gates.md`
naming the incident it came from. Retrospectives are a queue of things to
automate.

## What is measured

Not story points, not lines, not the number of PRs — those grow on their own and
mean nothing.

```bash
./scripts/metrics/report.sh          # last 30 days
./scripts/metrics/report.sh 90
```

- **Lead time** from intent to merge.
- **First-pass gate rate** — the share of PRs whose first CI run was green. This
  is the honest measure of whether the specs are good enough.
- **Incidents and their blast radius.**
- **Share of rules enforced by machine** rather than by prose.

If the first-pass rate sits below 50%, or incidents go up, or lead time does not
improve, the conclusion is not "the tools are bad" — it is that the process
needs stricter gates and smaller irreversible steps. That is agreed in advance,
so that it does not have to be argued about afterwards.

## Where this breaks — honestly

- **Review is now the bottleneck**, and it is human. Automate the oracle before
  removing the ceremony, never after.
- **One person, no second reviewer.** For irreversible changes the substitute is
  an ADR before the code. That is a weaker control than four eyes, and it should
  be named as one rather than quietly assumed away.
- **Cross-cutting consistency degrades invisibly.** An autonomous slice happily
  invents its own error handling, its own cache format, its own report shape.
  Only machine-checked conventions and ADRs hold this; taste does not scale at
  this speed. See the "do not invent a second way" rule in `CLAUDE.md`.
- **Cargo cult is the real risk.** Dropping rituals is easy and pleasant;
  building gates is work. Doing the first without the second shows up on the
  third incident, not the first.

## Related

- `CLAUDE.md` — the working agreements, and which gate enforces each
- `docs/gates.md` — the gate registry and its known gaps
- `docs/adr/` — decisions with a long horizon
- `CONTRIBUTING.md` — how to get set up and open a PR
