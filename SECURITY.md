# Security Policy

## Supported versions

OmniLang is alpha. Only the latest commit on `main` and the most recent tagged
release receive fixes. There are no long-term support branches yet.

## Reporting a vulnerability

Use GitHub's private vulnerability reporting: **Security → Report a
vulnerability** on <https://github.com/mindhalla-open/omnilang>. Please do not
open a public issue for anything exploitable.

Include the spec or input that triggers the problem, the command you ran, the
version (`omni --version`) and the effect you observed. You should hear back
within seven days; a fix or a documented mitigation follows in the next release.

## What to know before running OmniLang

These are design properties of the current alpha, not bugs. Treat them as the
threat model until the items in the roadmap change them.

- **`omni build` executes code that a language model wrote, on your machine.**
  Verification runs the generated project's own toolchain — `npm install` and
  Jest, `cargo build` and `cargo test`, `pytest`, `go test` — in the output
  directory, without a sandbox. Dependencies come from the generated
  `package.json` / `Cargo.toml` / `go.mod`. Run builds in a container or a
  throwaway environment, and review the output before using it.
- **`omni check`, `omni plan` and `omni fmt` never call a model and never
  execute anything.** They are safe to run on untrusted specs; the parser is
  fuzzed against malformed input in the test suite.
- **API keys are read from the environment** (`ANTHROPIC_API_KEY`,
  `OMNI_API_KEY`) or a local `.env`, which is gitignored. Nothing writes them to
  disk, the cache or the build report. The `secrets` gate rejects commits that
  contain a key of a known shape.
- **The playground server (`npm run playground`) is a development tool.** It
  accepts a spec over HTTP with permissive CORS, builds it and runs the result.
  Never expose it to a network you do not control.
- **The build cache and lockfile are not signed.** `.omni-cache/` contents are
  trusted verbatim on a cache hit; `omni.lock` records keys, not content hashes
  of the restored files. Do not share a cache directory with untrusted parties.
- **The runtime guardrail interpreter is a prototype.** Its output filters
  (card-number redaction, length, topic, tool ACL) are pattern-based. They are a
  defence layer, not a guarantee, and have not been independently reviewed.

## Dependency and advisory monitoring

CI runs `npm audit --audit-level=high` on the runtime and `rustsec/audit-check`
on the Cargo workspace on every push, and Dependabot opens weekly update PRs for
Cargo, npm and GitHub Actions.
