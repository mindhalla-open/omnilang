# Gates

Every gate here replaces something a human used to promise, remember or notice.
A gate may only be added when it catches something real, and a ritual may only
be dropped once the gate that replaces it is green.

Run them all before pushing:

```bash
./scripts/gates/run-all.sh
```

## Convention gates — `scripts/gates/`

| Gate | Replaces | Why it exists |
|------|----------|---------------|
| `workflows` | "check the YAML before pushing" | **Incident 2026-05-26 → 2026-08-06.** `if: ${{ secrets.X != '' }}` is not a valid context in `if:`; GitHub failed every run in 0s with no jobs and no logs, and 5 PRs merged with no gates at all. Also enforces `permissions:` on every workflow. |
| `examples` | reviewer running `omni check` by hand; a hardcoded list of 6 files in CI YAML | The list in CI had drifted to cover 6 of 11 examples. Now globbed, plus `omni fmt --check`. |
| `syntax-canon` | "we write brace-free now" | ADR-0009 made brace-free canonical but the parser still accepts `{}`, so nothing stopped drift. Two occurrences survived the 2026-06 conversion; one was still in `docs/04-syntax-reference.md` on 2026-08-06. |
| `error-codes` | "the table is generated from the source" (it was not) | `E0242` shipped emitted-but-undocumented. Codes are a public contract users filter on. Checks both directions. |
| `secrets` | security review by eye | Dependency-free floor: known key shapes in tracked files, and no committed `.env`. |
| `budgets` | the `cost-report` CI job, which posted a comment nobody blocked on — and whose `ls src/**/*.omni` guard never matched anything in this repo | Caps per-spec and total `omni plan` cost in `scripts/gates/budgets.json`. A new example must be budgeted explicitly. |
| `changelog` | the PR checkbox "I have updated the CHANGELOG" | Required when `crates/` or `runtime/src/` changed. Waiver: `[no-changelog]` in the commit message — deliberate and visible in history. |

## Pipeline gates — CI

| Gate | Replaces |
|------|----------|
| `cargo fmt --all -- --check` | style review |
| `cargo clippy --workspace --all-targets --locked -D warnings` | review of small correctness mistakes |
| `cargo test --workspace --locked` + jest suite | manual regression |
| `git diff --exit-code runtime/src/types.ts runtime/src/ir.schema.json` | coordinating a Rust↔TypeScript change by hand |
| Coverage floor in `runtime/jest.config.js` | "coverage looks fine" — `--coverage` reported a number nothing enforced |
| Multi-target conformance matrix (4 targets) | "it works on TypeScript, the rest probably follow" |
| `npm audit --audit-level=high`, `rustsec/audit-check` | dependency review by eye. Fixed 4 high-severity advisories on 2026-08-06. |
| `--locked` / `npm ci` on committed lockfiles | "works on my machine" (ADR-0010) |
| Toolchain preflight in `runtime/tests/integration.test.ts` | reading a bare `Expected: 0, Received: 1`. **Incident 2026-08-06 → 2026-09-20:** the suite needed pytest before the CI step that installed it; main stayed red for six weeks while the changelog said CI was fixed. |

## Rules for gates

1. **A red gate means the work is not done.** Not "look into it later".
2. **A gate that cannot fail is not a gate.** `run-all.sh` fails when a gate
   finds nothing to check — an empty example set or an empty code list is a bug
   in the gate, not a pass.
3. **Never weaken a gate to go green.** If a gate is genuinely wrong, change it
   in its own commit that explains why.
4. **Every incident becomes a gate**, in the same PR as the fix where possible.
   Add a row here saying which incident it came from.

## Known gaps

Recorded rather than quietly skipped:

- **Actions are pinned to tags, not commit SHAs.** A tag can be moved. Dependabot
  raises weekly updates; SHA pinning is the stricter option and is not done yet.
- **`gh`-based metrics are advisory** (`scripts/metrics/report.sh`), not enforced.
- **No performance budget** on compile time or binary size.
- **Docs prose is unverified** beyond OmniLang snippets — e.g. `docs/01-architecture.md`
  and `docs/05-compilation-model.md` describe OMWF as active while the module is
  unreferenced (ADR-0006).
- **Nothing checks that a CI fix actually worked.** The 2026-08-06 workflow fix
  was declared done from a local run; the first CI run after it was red for a
  different reason and stayed red until 2026-09-20. Until branch protection
  requires the checks, a red `main` needs a human to open
  `gh run list --workflow ci.yml --branch main` after every push.
- **No second reviewer.** The manifesto keeps four-eyes for irreversible changes;
  this project is one person, so the substitute is an ADR before the code, not a
  second human. That is a weaker control and should be named as one.
