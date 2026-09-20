# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `SECURITY.md`: supported versions, private vulnerability reporting, and the
  threat model of the alpha — `omni build` runs model-written code and its
  dependencies on the host without a sandbox.
- Status blocks in `docs/README.md` and `docs/11-roadmap.md` saying what is
  implemented versus what is still design.
- The runtime integration suite pre-flights every toolchain it needs (cargo,
  node, go, python3 + pytest) and names the missing one, and prints the child
  process output when a build fails instead of a bare exit code.
- **Gate-based development process** (ADR-0011): `scripts/gates/run-all.sh` runs
  seven convention gates locally and in CI — workflow validity, example
  check/format coverage, brace-free syntax canon, diagnostic-code documentation,
  credential scanning, build-cost budgets, changelog. Registry and known gaps in
  `docs/gates.md`; process in `docs/21-engineering-process.md`; working
  agreements in `CLAUDE.md`.
- `docs/adr/` with 11 decision records, including the nine backfilled from the
  gitignored `tasks/README.md` decisions log.
- Coverage floor for the runtime suite (`runtime/jest.config.js`) — `--coverage`
  previously produced a number nothing enforced.
- Dependency and advisory audits in CI (`npm audit`, `rustsec/audit-check`) and
  weekly Dependabot updates for Cargo, npm and GitHub Actions.
- `scripts/metrics/report.sh` — lead time, first-pass gate rate, incident proxy.
- Epic contract issue template; PR template rewritten around risk zones.

- **Z3 formal verification hardening**:
  - SMT sorts are now taken from the operation's declared `inputs`/`outputs` types (name-based heuristics are only a fallback; fixed `valid`-style names being typed `String` because they contain the substring `id`).
  - Counterexamples are extracted from the Z3 model and reported as concrete `var = value` bindings (e.g. `amount = 1.0, balance = 0.0`).
  - Z3 invocations run with a hard 10s timeout so a difficult obligation can no longer hang `omni check`/`omni build`.
  - New diagnostics: `E0805` (Z3 not on PATH — verification skipped, honestly reported instead of the previous "Simulating formal verification (Verified successfully)") and `E0806` (formal verification declared but nothing machine-checkable was proven).
  - `formal_verification` and `proven` are now registered constraint names (no more `E0212: undefined constraint` when opting into proofs).
  - New example `examples/formal_transfer.omni` with two Z3-proven obligations and a deferred natural-language condition.

### Removed

- The TypeScript verification step no longer validates and emits a hard-coded
  `CheckoutButton` React/Vue/Svelte component on every build regardless of the
  spec (`runtime/src/testing/components.ts` deleted with it), and no longer
  prints "Bypassing deprecated mock simulation tests".
- The security step no longer prints a fake fuzzing harness that "tested" two
  fixed payloads and always reported them rejected. What remains is a small
  pattern scan (`eval(`, an unsafe-SQL marker, one known-bad lodash version)
  that writes SARIF and is report-only, and it is now described as exactly that.
- Generated demo artifacts `docs/index.html`, `docs/openapi.json` and
  `docs/runbook.md` (`omni docs` output for `acme.phase1`) removed from the docs tree.

### Changed

- `omni plan` human output prints the same token-based estimate as
  `--format json` and the budgets gate, plus a tier recommendation labelled as
  the fixed complexity heuristic it is. The previous output was invented: a
  second, unrelated cost formula (about 4× the JSON number), an "ML-based"
  router, an "A/B test group", an "87.5% pre-warmed cache hit rate" and a
  build-time forecast, none of it measured. `estimated_cached_usd` in the JSON
  is now `0.0` — a warm content-addressed cache makes no LLM call.
- `omni dashboard` labels its compliance scores, regulatory reports and trend
  chart as illustrative sample data; the "ML model selection router" card with
  invented A/B results is gone.
- CI installs Go and Python (with pytest) before the runtime test suite instead
  of after it; the GitLab template gives the runtime job the same toolchains.
- README no longer tells users to `cargo install omni-cli`: that name on
  crates.io belongs to an unrelated project.
- CI: explicit least-privilege `permissions:`, the hardcoded six-example
  `omni check` list replaced by a glob over all eleven, and the non-functional
  `cost-report` job (its `ls src/**/*.omni` guard never matched) replaced by an
  enforcing budget gate.
- `.github/issue_template/` renamed to `.github/ISSUE_TEMPLATE/`, the path
  GitHub documents.

### Fixed

- **CI on `main` had still never been green.** The entry below fixed the
  0-second workflow failure on 2026-08-06, but the very next run failed too, and
  every run since: the runtime integration suite builds the Python target and
  needs pytest, and the step installing pytest came after the suite. Locally the
  suite passed because pytest was installed. Toolchain setup now precedes the
  suite, and the suite fails fast with the tool's name when one is missing.
- **CI had not run since 2026-05-26.** `.github/workflows/ci.yml` used
  `if: ${{ secrets.ANTHROPIC_API_KEY != '' }}` on a step; the `secrets` context is
  not available in `if:`, so GitHub rejected the workflow and failed every run in
  0s with no jobs and no logs. PRs #5–#9 merged with no gate running. The
  conditional now goes through `env:` and announces the skip, and
  `scripts/gates/workflows.sh` fails the build if the pattern comes back.
- `Cargo.lock` and `runtime/package-lock.json` are now tracked (ADR-0010). They
  were gitignored, which meant `npm ci` could not have worked on a clean
  checkout, `actions/setup-node`'s cache path pointed at a missing file, and the
  Rust cache key `hashFiles('**/Cargo.lock')` hashed nothing. Builds now use
  `--locked` / `npm ci`.
- Resolved 4 high-severity advisories in the runtime dependency tree
  (`npm audit fix`, no breaking upgrades; all 128 runtime tests still pass).
- Documented `E0242` (unsupported service target) in `docs/18-error-codes.md` — it
  was emitted but undocumented.
- Converted the last brace-style block in `docs/04-syntax-reference.md` to the
  canonical brace-free form (ADR-0009).

- **Formal verification soundness**: natural-language conditions were asserted verbatim into SMT scripts (invalid SMT → spurious failures), operations with no formal postconditions ran `check-sat` on preconditions alone and reported the inevitable `sat` as a "counterexample", and out-of-subset operators (`in`, `..`) were silently translated as `=` — which could produce a false proof. All conditions are now filtered to the machine-checkable subset, untranslatable constructs are hard errors, and skipped natural-language conditions are reported as deferred to generated tests.

- **Rust builds no longer break on cache hits**: when a service was restored from the `.omni-cache` build cache, the generated crate's `src/services/mod.rs` was left empty (module registration only ran on fresh LLM generations), so `cargo test` on the output failed with `E0432: unresolved import`. The cache-restore path now replays the module registration. Added a warm-cache regression test.
- Synced `runtime/package.json` (0.1.0) and `scripts/npm/package.json` (0.6.0) versions to the workspace version (0.11.0).

## [0.11.0] - 2026-05-29

### Added

- **Ollama / Llama.cpp Provider Support** (Step 6):
  - Added `LlamaCppProvider` supporting OpenAI-compatible chat completions and native `/completion` fallback endpoints.
  - Registered `llamacpp`, `llama.cpp`, `ollamacpp`, and `ollama.cpp` LLM provider options.

- **Multi-Target Code Generation** (Rust, Python & Go):
  - Enabled code generation, directory initialization, and testing for Rust, Python, and Go targets.
  - Implemented automatic packages config generation (`Cargo.toml` for Rust, `__init__.py` modules for Python, `go.mod` for Go).
  - Prompts formatting for Rust structs/enums, Python dataclasses, and Go structs with JSON tag mapping.

### Fixed

- **Stabilized Mock Generator Target Detection**:
  - Robustified language detection logic in mock LLM provider to match exact role header strings instead of fragile substring matching, preventing compilation errors and false-matches caused by leftover error logs in `.omni-cache/traces/retries.json`.

## [0.10.0] - 2026-05-25

### Added

- **Formal Verification Integration** (Step 5.1):
  - Added SMT obligation extractor and translator to translate invariants, pre/postconditions into SMT-LIB v2 scripts.
  - SMT proof certificate generation saved to `.omni-cache/proofs/`.
  - Local Z3 solver integration checking obligations and promoting verified services to `Proven` status.

- **Multi-Repository Federated Builds** (Step 5.2):
  - Supported `--federated <config_toml>` in build subcommand.
  - Implemented cross-repository contract compatibility verification check.

- **Audit & Compliance Dashboard** (Step 5.3):
  - Added `omni dashboard` subcommand generating real-time audit files and compliance reports: PCI DSS v4.0, SOC 2 Type II, and HIPAA compliance report.
  - Interactive compliance report site under `compliance/index.html` featuring org trust scores, cost trends, and Evidence Chain Browser.

- **Cost Optimization Engine** (Step 5.4):
  - Updated `omni plan` output to compute and display ML model routing recommendations (Cheap, Balanced, Premium), predictive build cost estimation, and cache statistics.

- **Self-Improving Agents** (Step 5.5):
  - TypeScript runtime integration for self-improving agents tracking execution logs.
  - Save retry failures to `.omni-cache/traces/retries.json` and successful traces to `.omni-cache/traces/`.
  - Automatic prompt optimizer analyzing errors (compilation, test, schema errors) to append adaptive prompt suggestions.
  - A/B testing strategy router evaluating multiple model tiers.

## [0.9.0] - 2026-05-25

### Added

- **Package Registry** (Step 4.1):
  - Mock registry and package manager subcommands: `publish`, `install`, `search`.
  - Organization-scoped packages support (`@acme/...`, `@community/...`).
  - Publish verification hook checking packages before registry upload.
  - Local package caching registry under `.omni-cache/registry/`.

- **Policy Enforcement** (Step 4.2):
  - Organization-wide policy files (`.omnipolicy`) parser and compliance checker.
  - Policy inheritance (org -> project -> service) and override mechanism with annotations.

- **Workflow Blocks** (Step 4.3):
  - Multi-step business workflow blocks parsing and state machine transition validation.
  - Reachability check and dead state detection.
  - Mermaid state diagram generator and TypeScript state machine pattern code generator.

- **Agent Marketplace** (Step 4.4):
  - Agent performance evaluation and benchmarking suite.
  - Multi-agent leaderboard console view: `omni agents benchmark`.

- **IDE Rich Features** (Step 4.5):
  - LSP code actions ("Extract mixin", "Add test scenario") and rename symbol refactoring support.
  - Intent Entropy Analyzer & Logical Gap Detector.

- **Schema Blocks** (Step 4.6):
  - Database schema block parser and multi-DB SQL migration generators (Postgres, MySQL, MongoDB, DynamoDB).
  - Schema evolution breaking change detector comparing schema version updates.

- **Documentation Generation** (Step 4.7):
  - CLI subcommand: `omni docs generate`.
  - OpenAPI/Swagger specification builder.
  - Incident response runbook generator from service constraints.
  - Gorgeous, responsive, interactive HTML documentation site.

## [0.8.0] - 2026-05-25

### Added

- **Confidence Types** (Step 3.1):
  - `TrustLevel` enum in Rust: `Speculative`, `Low`, `Medium`, `High`, `Proven`
  - Integrated `Confident<T>` wrapper type as a built-in symbol
  - Service local confidence calculation based on verification constraints & tests
  - Dependency-aware confidence propagation through DAG/fixed-point iteration
  - Trust policies verification (e.g. `production requires High`)
  - Included confidence levels and evidence chains in SpecIR Service Definitions

- **Visual Testing** (Step 3.2):
  - Screenshot comparison engine in the TypeScript runtime
  - Golden files management under `.golden/`
  - Tolerance configuration (pixel threshold) & ignore regions
  - Side-by-side textual diff viewer inside the CLI/Console output for visual regressions

- **Performance Testing** (Step 3.3):
  - Built-in benchmark harness simulator
  - SLO verification for p50/p95/p99 latency and throughput
  - Support for load profiles (ramp-up, spike, soak)
  - Flamegraph visualization mock generation and structured performance evidence logging

- **Security & Chaos Testing** (Step 3.4 & 3.5):
  - SAST Semgrep rules scanner and dependency vulnerability checker in TypeScript
  - Input fuzzing simulation for Injection vulnerabilities (SQLi, XSS)
  - Chaos fault injection framework supporting network partition, service crashes, CPU exhaustion, and clock skew
  - Resilience verification and recovery time constraint validation
  - SARIF-compliant security report output format

- **Multimodal Types & UI Components** (Step 3.6 & 3.7):
  - Registered `Image`, `Screenshot`, `Trace`, `Log`, `Diagram` built-in types
  - UI Component block slots, props, state, and events validation
  - Multi-framework code generators (React, Vue, Svelte)
  - Responsive breakpoints, WCAG accessibility rules, and bundle size constraints validation

## [0.7.0] - 2026-05-25

### Added

- **Module System** (Step 2.1):
  - `export` and `private` visibility modifiers for declarations
  - `mixin` blocks with `constraints`, `postconditions`, and `tests` sections
  - `apply: MixinName` in service blocks for mixin composition
  - Visibility checking: detects exporting private declarations
  - `omni.toml` manifest parsing with `[package]` and `[dependencies]` sections
  - `omni.lock` lockfile generation for deterministic dependency pinning
  - Semantic versioning parsing and conflict detection
  - Import resolution with registry, relative, and standard imports

- **Multi-Agent Support** (Step 2.2):
  - Agent registry with capability declaration and matching
  - Budget-aware agent routing (`CheapFast` / `Balanced` / `SmartExpensive` tiers)
  - Dependency-aware parallel execution engine with rate limiting
  - Retry with model escalation and progress tracking callbacks

- **OMWF Wire Format** (Step 2.3):
  - OMWF serializer: Spec IR → compact token-efficient text format
  - OMWF deserializer: OMWF text → structured data
  - Bidirectional JSON ↔ OMWF converter
  - Constrained decoding grammar generator (BNF, regex, JSON Schema)
  - Token savings calculator (~40-60% fewer tokens vs JSON)

- **LSP v1** (Step 2.4):
  - New `crates/omni-lsp/` crate — tower-lsp based language server
  - Real-time diagnostics from parser and analyzer
  - Keyword and type autocomplete with trigger characters
  - Hover information for OmniLang keywords
  - Go to definition for types, services, and mixins
  - Document symbol provider (outline view)

- **Budget System** (Step 2.5):
  - Per-task token counting with input/output breakdown
  - Per-service cost accumulation and total build cost tracking
  - Budget enforcement (`max_total`) and alert thresholds (`alert_at`)
  - Cost estimation from spec IR before build
  - Formatted cost report generation

- **CI/CD Integration** (Step 2.6):
  - `.github/actions/setup-omni/` composite action with Rust toolchain + caching
  - CI workflow: format check, clippy, tests on PR; release build on merge
  - PR cost reporting via `actions/github-script`

- **Three-Tier Storage & Incremental Builds** (Step 2.7):
  - Content-addressed cache (CAS) with SHA-256 hashing under `.omni/cache/`
  - Change detection: skip LLM calls for unchanged specs
  - Git auto-commit with structured messages after verified builds
  - Agentic memory: reasoning trace logger for full chain-of-thought transparency
  - Trace reports with prompt/response/verification/decision history

### Changed

- Parser now recognizes `constraints`, `export`, `private`, and `mixin` as keywords
- All `constraints:` sections in parsers now handle both `KwConstraints` and `Ident` tokens
- Analyzer pipeline includes module system checks (visibility + mixin expansion)
- Wire format replaced: OMWF → strict minified JSON (`runtime/src/wire.ts`)
- Workspace version bumped to 0.7.0

### Deprecated

- **OMWF wire format** — frozen as experiment, all exports marked `@deprecated`. Lossy serialization, non-standard format, premature optimization. Replaced with minified JSON.
- `--wire-format` CLI flag — hidden from help, emits warning if used

---

## [0.6.0] - 2026-05-25

### Added

- **Type Narrowing**: Control-flow-aware type analysis emits info diagnostics when optional fields are compared without null-check guards in preconditions.
- **Option Propagation**: Postcondition validation warns when accessing fields on optional-typed outputs without null-check guards.
- **Refined Generator Config**: Spec IR now includes `GeneratorConfig` for refined types, extracting range bounds, string length constraints, format patterns, and precision for constraint-aware arbitrary value generation.
- **Deterministic Type Mapping**: New `type_mapping` module with OmniLang → target type mapping tables for TypeScript, Rust, and Python (e.g. `UUID → Uuid`, `Money → Decimal`). Includes target-specific constraint compatibility validation.
- **Evidence System — Full Chain**:
  - `evidence/` directory: `omni verify` now copies JUnit XML and LCOV files to a dedicated evidence directory.
  - `evidence/chain.json`: Structured mapping linking constraints ("all tests pass", "code coverage") to their evidence artifacts.
  - CI-friendly JSON output: `omni verify --format json` emits machine-readable verification results.
- **Distribution & Packaging**:
  - Shell installer script (`scripts/install.sh`) for `curl | sh` installation.
  - Homebrew formula template (`scripts/homebrew/omni.rb`).
  - npm wrapper package (`scripts/npm/`) for `npx @omnilang/cli` support.
  - `cargo install omni-cli` support (existing Cargo.toml metadata).

### Changed

- `SpecIR` now includes `type_mappings` field with the target-specific type mapping table.
- `TypeDef` in Spec IR now includes an optional `generator` field with refined type constraints.
- `omni verify` command gains `--format` (human/json) and `--evidence-dir` flags.

## [0.5.0] - 2026-05-25

### Added

- **Contract System**: Service-level `invariants:` block with expression validation, `old()` postcondition-only enforcement (exactly 1 argument), and CamelCase symbol resolution against the symbol table.
- **Property-Based Testing**: Extended `forall:` quantifier parsing to support comma-separated entries with both `in` and `<-` syntaxes, and generic call expressions like `arbitrary<T>`.
- **Multi-Target Code Generation**: Support for `--target typescript`, `--target rust`, and `--target python` in the CLI build command and orchestrator runtime.
  - **TypeScript**: Jest tests, `fast-check` for property-based testing.
  - **Rust**: `Cargo.toml` scaffolding, `proptest` dev-dependency, service module auto-registration.
  - **Python**: `requirements.txt` with `pytest`/`hypothesis`/`mypy`/`pydantic`, `pyproject.toml` configuration.
- **Evidence System**: `omni verify` CLI subcommand that scans a build directory for JUnit XML test reports and LCOV coverage files, parses them, and prints a color-coded compliance report with `--report` for per-suite detail.
- **LLM Prompt Enhancements**: Target-specific system prompts instructing the LLM on contracts, invariants, custom error types, and property-based testing per language.
- **Mock LLM Extensions**: Mock responses for Rust (with `thiserror` error enums) and Python (with `dataclass`/`Exception` patterns) targets.
- **Tree-sitter**: `invariants_section` grammar rule, `invariants` keyword highlighting, and corpus test case for `Service with Invariants`.

## [0.4.0] - 2026-05-25

### Added

- Top-level custom constraint declarations in AST, parser, symbol table, and analyzer.
- Built-in validation rules for standard service constraints: `cacheable`, `rate_limited`, `authorized`, `latency`, and `eventual_consistency`.
- Service conflict detection (e.g. contradictory latency/SLO targets, conflicting authenticated/anonymous requirements).
- Transitive constraint propagation checks along the service dependency chain (via `depends_on`).
- Tree-sitter grammar rules, highlight queries, and tests for constraint declarations.
- Comprehensive unit tests in `parser.rs` and `type_check.rs`.

## [0.3.0] - 2026-05-25

### Added

- Extended type system parsing support for generic parameters (`<T: Bound1 + Bound2, E>`), union types (`A | B`), intersection types (`A & B`), and Option shorthand (`T?`).
- Typechecker validations for refined types constraints (`format`, `range`, `min_length`, `max_length`, `precision`, `example`).
- Refined types base type inference deducing the primitive base type from constraints (e.g. `format` -> `String`, `range` -> `Int` / `Float`, `precision` -> `Decimal`).
- Generic type argument count validation and type bounds presence checks in resolve phase.
- Extended type constructs support in tree-sitter grammar rules, syntax highlighting queries, and VS Code TextMate patterns.
- Unit testing suite for parser, typechecker, and tree-sitter.

## [0.2.0] - 2026-05-25

### Added

- Complete parser, AST types, name resolution, symbol registration, and type checking for six new blocks: `Component`, `Pipeline`, `Workflow`, `Agent`, `Schema`, and `Policy`.
- Complete Tree-sitter grammar rules, highlighting queries, and VS Code TextMate grammar updates for all new top-level blocks and their keywords.
- Support for visual spec options (e.g. command-line flags) and list-based policy conditions/clauses.
- GitHub issue templates (`bug_report.md`, `feature_request.md`, `rfs_proposal.md`), PR template, and a binary release workflow (`release.yml`).
- Verification example file `examples/phase-1-blocks.omni` containing all blocks.

## [0.1.0] - 2026-05-25

### Added

- Project scaffolding: Cargo workspace with `omni-parser`, `omni-analyzer`, `omni-cli` crates
- CLI skeleton with `check`, `plan`, `build`, `init` subcommands
- AST type definitions for OmniLang Phase 0 syntax
- Single-Agent Pipeline (TypeScript Runtime & LLM Code Generation) in `runtime/`
- Tree-sitter Grammar & Editor Support in `tree-sitter-omnilang/` and `editors/vscode/`
- Declarative `metrics` block in service specifications (`counter`, `gauge`, `histogram`) with parser, AST, and syntax highlighting support
- Auto-instrumentation of service metrics in TypeScript code generator
- Dual license: Apache-2.0 OR MIT
