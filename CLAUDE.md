# CLAUDE.md — working agreements for agents and humans

Conventions in this repo are executable wherever they can be. This file is the
index; `scripts/gates/` is the enforcement. If a rule here has a gate, the gate
is authoritative — this text can rot, the gate cannot.

```bash
./scripts/gates/run-all.sh        # every convention gate; run before pushing
./scripts/gates/run-all.sh examples syntax-canon   # a subset
```

**A red gate means the work is not done.** Fix the cause. Never weaken a gate to
make a build green — if a gate is genuinely wrong, change it in its own commit
that says why.

## What this repo is

OmniLang: a specification language whose specs are compiled by AI agents into
verified code, plus a runtime that interprets `agent` blocks as live guardrails.

```
.omni spec → omni-parser (AST) → omni-analyzer (types, contracts, Spec IR)
           → runtime/ (orchestration, verification, self-correction)
           → generated code for: typescript | rust | python | go
```

| Path | Role |
|------|------|
| `crates/omni-parser` | Lexer + parser → AST. Normative for the grammar. |
| `crates/omni-analyzer` | Type check, constraints, Z3, diagnostics, Spec IR. |
| `crates/omni-cli` | The `omni` binary: `check`, `plan`, `build`, `fmt`, `verify`, `init`. |
| `crates/omni-lsp` | Editor support. |
| `runtime/` | TypeScript orchestrator, verifiers, guardrail interpreter. |
| `examples/` | Normative corpus — every file is checked and formatted by CI. |
| `docs/` | Public documentation. `docs/adr/` holds decisions with a long horizon. |

## Rules with gates behind them

| Rule | Gate |
|------|------|
| Canonical syntax is brace-free (`syntax_version 0.2`). The parser still accepts `{}` for compatibility; new code and docs must not use it. | `syntax-canon` |
| OmniLang snippets in docs use the ` ```omnilang ` fence, nothing else. | `syntax-canon` |
| Every example parses, analyzes and is canonically formatted. | `examples` |
| Every diagnostic code emitted by the analyzer is in `docs/18-error-codes.md`, and vice versa. | `error-codes` |
| Every example has an entry in `scripts/gates/budgets.json`; build cost does not grow silently. | `budgets` |
| Changes under `crates/` or `runtime/src/` update `CHANGELOG.md` (or say `[no-changelog]` in the commit). | `changelog` |
| No credentials in tracked files; no `.env` committed. | `secrets` |
| Workflows parse, declare `permissions:`, and never use `secrets.*` inside `if:`. | `workflows` |
| Rust: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`. | CI |
| Runtime coverage stays above the floor in `runtime/jest.config.js`. | CI |
| `Cargo.lock` and `runtime/package-lock.json` are committed; builds use `--locked` / `npm ci`. | CI |

## Rules a machine cannot check yet — hold these yourself

- **Spec IR changes.** `cargo test` regenerates `runtime/src/types.ts` and
  `runtime/src/ir.schema.json` from the Rust types; commit the regenerated files
  in the same change. CI fails on drift. A change that alters IR *meaning* also
  needs a version decision per `docs/19-versioning-policy.md`.
- **Diagnostic codes are a public contract.** Users grep and filter on them.
  Allocate by phase prefix (`E00xx` pipeline, `E01xx` names, `E02xx` types,
  `E03xx` constraints, `E04xx` modules, `E05xx` policy, `E06xx` workflow,
  `E07xx` schema, `E08xx` formal, `E09xx` intent, `E10xx` deps). Never reuse or
  renumber a code — that breaks every user filter silently.
- **Do not invent a second way to do a cross-cutting thing.** Error handling,
  caching, prompt templates (`.md` files, not inline strings), target lists,
  report formats — find the existing one and extend it. Autonomous work drifts
  here fastest and no test catches it.
- **The four targets move together.** `typescript`, `rust`, `python`, `go`. A
  feature that only works on one is not done; the conformance matrix builds
  `examples/hybrid_billing.omni` on all four.

## Irreversible changes — require an ADR before the code

Cheap to get wrong, expensive to undo, because specs and generated code already
exist outside this repo:

- syntax or grammar changes; anything touching `package.syntax_version`
- Spec IR schema changes; `omni.lock` or `.omni-cache/` format changes
- removing or renumbering a diagnostic code; removing a CLI flag or subcommand
- anything published: crates.io, npm, the Homebrew formula, the GitHub Action

Write `docs/adr/NNNN-title.md` first (template: `docs/adr/0000-template.md`),
then implement. Everything else is reversible — ship it behind a flag and iterate.

## Working style

- Branch `feat/*`, conventional commits (`feat:`, `fix:`, `docs:`, `test:`, `chore:`).
- One epic = one intent = one owner = one vertical slice, spec → analyzer →
  runtime → all targets → docs → example. Not "the parser part".
- Prefer deleting and regenerating a slice over patching a bad one. The code is
  cheap; the specification is the asset.
- Report status honestly: if tests fail, say they fail. A green summary over a
  red gate is the one unrecoverable error in this process.

Rationale and the full process: `docs/21-engineering-process.md`.
