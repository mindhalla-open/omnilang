# Architecture Decision Records

A decision belongs here when it has a **long horizon** — when reversing it later
would mean changing specs users already wrote, code already generated, or
artifacts already published. Everything else belongs in a commit message.

Until 2026-08-06 these decisions lived in `tasks/README.md`, which is in
`.gitignore` — the entire rationale for the project existed on one laptop and in
one person's memory. ADR-0001 … ADR-0009 are backfilled from that log; dates are
the original decision dates.

## Index

| # | Decision | Status |
|---|----------|--------|
| [0001](./0001-rust-as-implementation-language.md) | Rust as the compiler implementation language | Accepted |
| [0002](./0002-dual-license.md) | Dual license: Apache-2.0 OR MIT | Accepted |
| [0003](./0003-monorepo.md) | Monorepo layout | Accepted |
| [0004](./0004-typescript-as-primary-target.md) | TypeScript as the primary codegen target | Accepted, extended |
| [0005](./0005-llm-providers.md) | Anthropic primary, local providers supported | Accepted, extended |
| [0006](./0006-omwf-wire-format.md) | OMWF token-optimized wire format | **Superseded / unresolved** |
| [0007](./0007-hybrid-approach.md) | Hybrid approach: infra config split out, natural-language invariants | Accepted |
| [0008](./0008-prompt-templates-in-markdown.md) | Prompt templates live in `.md` files | Accepted |
| [0009](./0009-brace-free-canonical-syntax.md) | Brace-free canonical syntax (`syntax_version 0.2`) | Accepted |
| [0010](./0010-commit-lockfiles.md) | Commit `Cargo.lock` and `runtime/package-lock.json` | Accepted |
| [0011](./0011-gate-based-process.md) | Gate-based development process | Accepted |

## How to add one

1. Copy [`0000-template.md`](./0000-template.md) to `NNNN-short-title.md`.
2. Write it **before** the implementation — an ADR written afterwards is a
   changelog entry wearing a costume.
3. Add a row to the index above.
4. Never edit a decision after the fact. Supersede it with a new ADR and set the
   old one's status to `Superseded by NNNN`. The wrong turns are the valuable part.
