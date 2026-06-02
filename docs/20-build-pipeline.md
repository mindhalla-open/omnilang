# Build Pipeline & Artifacts (Reference)

This documents the build pipeline **as implemented** — the commands, flags, and
artifacts you can rely on today. It is kept in sync with the analyzer/runtime
behavior and the conformance tests.

## Commands

| Command | Purpose |
|---------|---------|
| `omni check [path]` | Parse + analyze; prints diagnostics with stable codes (see `docs/18-error-codes.md`). Exit 1 on errors. |
| `omni plan [path] [--format json]` | Dry-run: execution plan + token-based cost estimate. `--format json` for CI. |
| `omni build [path] --target <t>` | Analyze → generate (LLM) → verify → emit. `<t>` ∈ `typescript\|rust\|python\|go`. |
| `omni fmt [path] [--check]` | Idempotent brace-free formatting. |
| `omni deploy-policy --service --spec --reason` | Version + audit a runtime agent policy. |
| `omni agents benchmark` | Self-correction analytics from real traces. |
| `omni dashboard [out]` | Compliance dashboard + real `build_metrics.json`. |

### `omni build` flags

- `--target <lang>` — build-wide target language.
- `--frozen` — reproducible mode: a cache miss is a hard error (no LLM call). Use in CI.
- `--budget <usd>` — hard cost ceiling; the build stops if exceeded.
- `--full-stack` — emit API + UI + data-pipeline scaffolding.

## Determinism & caching

- Generated artifacts are content-addressed-cached under `.omni-cache/cas/`
  keyed by `(systemPrompt + userPrompt(IR) + target + generation params)`.
  An unchanged service is reused with **no LLM call**.
- `[generation]` in `omni.toml` pins provider/model/temperature/seed; these fold
  into the cache key, so changing the model invalidates affected artifacts.
- `omni.lock` records per-service cache keys + pinned parameters for `--frozen`.

## `build-report.json`

Written to `<output>/build-report.json` on every build (success or failure):

| Field | Meaning |
|-------|---------|
| `success`, `target`, `duration_ms` | Outcome and timing. |
| `provenance` | `{ ir_version, ir_hash, generation }` — ties the build to its exact IR + params. |
| `llm_calls`, `cache_hits` | Reproducibility metrics. |
| `cost_usd`, `total_tokens`, `budget_limit_usd`, `cache_savings_usd` | Budget metrics. |
| `contracts_covered`, `contracts_uncovered`, `contracts[]` | Contract coverage (see below). |
| `security_issues` | Count from the SAST scan (`.evidence/security_report.sarif`). |
| `services[]` | Per-service `{ name, success, attempts, cached }`. |

## Verification gates

A build only emits artifacts that pass:

1. **Compile + tests** — per target: `tsc`+`jest`, `cargo build`+`cargo test`,
   `mypy`+`pytest`, `go test`.
2. **Contract coverage** — every declared pre/postcondition and invariant must
   carry an enforcing `// @omni:contract <id>` marker in the generated code, or
   the build fails (`OMNI_ENFORCE_CONTRACTS=false` to disable).
3. **Self-correction** — on verification failure, the runtime feeds structured
   diagnostics back and retries (bounded), cleaning prior-attempt files between
   tries.

Analysis-time gates (before any LLM call):

- **Contradictory preconditions** — Z3 proves unsatisfiable preconditions
  (`error[E0803]`); the operation can never run.
- **Critical invariants** — safety/critical-named NL invariants warn
  (`warning[E0804]`) that they cannot be formally proven.
- **Confidence** — a service reaches `Proven` only when Z3 discharges its formal
  postconditions; declared-but-unproven formal contracts are `High`.

## Per-service target

A service may override the build target: `service Foo` with `target rust`.
Invalid targets are rejected (`error[E0242]`). A true mixed-language build
(different targets in one run) currently requires a separate `omni build
--target <t>` per target; the override is surfaced as a warning otherwise.
