# Roadmap

OmniLang is an ambitious project. This roadmap outlines the evolutionary path from concept to production-grade language, broken into pragmatic milestones.

---

## Status (2026-09-20)

The checkboxes below are the original plan, not a progress report. Where things stand:

| Phase | State |
|-------|-------|
| 0 — Foundation | Implemented: parser, analyzer, `check` / `plan` / `build`, TypeScript target, scenario tests as generated Jest suites. |
| 1 — Core Language | Implemented in prototype form: contracts, constraints, four targets, a contract-coverage gate. Property tests are generated into the output, not run by a dedicated runner. |
| 2 — Production Readiness | Partial: module system, budgets, content-addressed cache, `omni.lock`, CI templates, an LSP binary. Missing: a binary that builds outside a repository checkout, and a reproducible-build story that survives a fresh clone. |
| 3 — 5 | Prototypes and experimental commands only: `dashboard`, `publish` / `install` / `search` against a local mock registry, `--federated`, `agents benchmark`. None is production-grade. |

The project is **alpha**. [`19-versioning-policy.md`](./19-versioning-policy.md) says what
`1.0` will freeze; the [CHANGELOG](../CHANGELOG.md) says what has shipped.

---

## Guiding Principle

> Build the minimal useful thing first, prove it works on real problems, then expand.

We will NOT try to build the full vision from day one. Instead, we iterate through increasingly capable versions, each delivering real value.

---

## Phase 0: Foundation (Months 1-3)

**Goal:** Prove the core concept works end-to-end on a narrow use case.

### Deliverables

- [ ] **Grammar & Parser**
  - Formal EBNF grammar for OmniLang core syntax
  - Tree-sitter grammar for editor support
  - Parser implementation (Rust or TypeScript)

- [ ] **Analyzer (v0)**
  - Type checking for basic types (primitives, structs, enums)
  - Constraint validation (detect conflicts)
  - Dependency resolution

- [ ] **Single-Agent Pipeline**
  - Integration with one LLM provider (e.g., Anthropic Claude)
  - Basic generate → verify → emit loop
  - TypeScript as the only target language

- [ ] **Basic Test Runner**
  - Scenario test execution
  - Pass/fail reporting

- [ ] **CLI (v0)**
  - `omni check` — validate specs
  - `omni build` — generate + verify
  - `omni plan` — dry run with cost estimate

### Success Criteria

A developer can write a spec for a simple REST API (3-5 endpoints), run `omni build`, and get a working TypeScript service with passing tests.

---

## Phase 1: Core Language (Months 4-6)

**Goal:** Support real-world service specifications with meaningful verification.

### Deliverables

- [ ] **Extended Type System**
  - Refined types with validation rules
  - Generic types
  - Union and intersection types
  - Option and Result types

- [ ] **Full Constraint System**
  - Built-in constraint library (idempotent, cacheable, rate_limited, etc.)
  - Custom constraint definitions
  - Constraint verification hooks

- [ ] **Contract System**
  - Pre/postconditions
  - Invariants
  - Error types
  - `old()` function for delta assertions

- [ ] **Property-Based Testing**
  - Arbitrary value generation
  - Shrinking on failure
  - Property test execution

- [ ] **Multi-Target Support**
  - Add Rust and Python as target languages
  - Target selection per service

- [ ] **Evidence System (v1)**
  - Test results as evidence
  - Coverage reporting
  - Basic evidence chain

### Success Criteria

A team can specify a microservice system (5-10 services) with contracts and property tests, and get verified implementations in TypeScript, Rust, or Python.

---

## Phase 2: Production Readiness (Months 7-9)

**Goal:** Make OmniLang viable for production use in teams.

### Deliverables

- [ ] **Module System**
  - Imports and exports
  - Module manifests (`omni.toml`)
  - Dependency resolution with lock files
  - Mixins and generic specs

- [ ] **Multi-Agent Support**
  - Agent registration protocol
  - Role-based task assignment
  - Parallel agent execution
  - Agent retry with feedback loop

- [ ] **OMWF Wire Format**
  - Token-optimized serialization for inter-agent communication
  - JSON ↔ OMWF bidirectional converter
  - Constrained decoding schema generator
  - Benchmark: token savings measurement suite

- [ ] **Budget System**
  - Token tracking
  - Cost budgets with alerts
  - Model selection strategies (cheap/balanced/expensive)

- [ ] **CI/CD Integration**
  - GitHub Actions integration
  - GitLab CI integration
  - Cost reporting in PRs

- [ ] **LSP (v1)**
  - Syntax highlighting
  - Autocomplete
  - Go to definition
  - Inline diagnostics

- [ ] **Incremental Builds**
  - Dependency-aware rebuild
  - Content-addressed caching

### Success Criteria

A 5-person team can adopt OmniLang for a real project, use it in their CI/CD pipeline, and track generation costs.

---

## Phase 3: Advanced Features (Months 10-14)

**Goal:** Differentiate OmniLang with unique AI-native capabilities.

### Deliverables

- [ ] **Confidence Types**
  - Trust level computation
  - Confidence propagation through dependency graph
  - Trust policies (production readiness gates)

- [ ] **Visual Testing**
  - Screenshot comparison engine
  - Golden file management
  - Tolerance and ignore regions

- [ ] **Performance Testing**
  - Built-in benchmark harness
  - SLO verification
  - Load test integration

- [ ] **Security Testing**
  - SAST integration
  - Input fuzzing
  - Dependency vulnerability scanning

- [ ] **Chaos Testing**
  - Fault injection framework
  - Resilience verification
  - Recovery time validation

- [ ] **Multimodal Types**
  - Image, trace, and log types
  - Evidence attachment system
  - Visual spec support for UI components

- [ ] **UI Component Specs**
  - `component` blocks
  - Visual golden management
  - Accessibility constraint verification

### Success Criteria

A developer can specify a full-stack application (API + UI + data pipeline) with visual tests, performance SLOs, and security constraints, and get a verified system.

---

## Phase 4: Ecosystem (Months 15-20)

**Goal:** Build the ecosystem that makes OmniLang self-sustaining.

### Deliverables

- [ ] **Package Registry**
  - Publish and consume spec packages
  - Versioning and compatibility checks
  - Organization-scoped packages

- [ ] **Policy Enforcement**
  - Organization-wide policies
  - Policy compliance reporting
  - Policy inheritance and overrides

- [ ] **Workflow Blocks**
  - State machine specifications
  - Transition validation
  - Workflow visualization

- [ ] **Agent Marketplace**
  - Custom agent registration
  - Agent benchmarking
  - Agent leaderboard

- [ ] **IDE Rich Features**
  - Inline evidence viewer
  - Visual diff tool
  - Cost dashboard
  - Dependency graph viewer
  - AI-assisted spec writing

- [ ] **Schema Blocks**
  - Database schema specifications
  - Migration generation
  - Schema evolution validation

- [ ] **Documentation Generation**
  - API docs from contracts
  - Architecture diagrams from topology
  - Runbooks from specs

### Success Criteria

An open-source community is forming around OmniLang. Multiple organizations use it in production. The package registry has 50+ published spec packages.

---

## Phase 5: Enterprise & Scale (Months 21+)

**Goal:** Enterprise-grade features for large organizations.

### Planned Features

- [ ] **Formal Verification Integration**
  - Connect to SMT solvers for critical invariants
  - Proof certificates for high-confidence code

- [ ] **Multi-Repository Support**
  - Cross-repo spec dependencies
  - Federated build system

- [ ] **Audit & Compliance Dashboard**
  - Organization-wide compliance view
  - Evidence chain browser
  - Regulatory report generation

- [ ] **Cost Optimization Engine**
  - ML-based model selection
  - Predictive cost estimation
  - Cache hit optimization

- [ ] **Self-Improving Agents**
  - Agent fine-tuning on project-specific patterns
  - Learning from retry history
  - Automated prompt optimization

---

## Versioning Strategy

| Phase | Language Version | Stability |
|-------|-----------------|-----------|
| Phase 0 | 0.1.x | Experimental — syntax may change |
| Phase 1 | 0.2.x | Alpha — core syntax stabilizing |
| Phase 2 | 0.5.x | Beta — production-usable with caveats |
| Phase 3 | 0.8.x | Release Candidate |
| Phase 4 | 1.0.x | Stable — backward compatibility guaranteed |
| Phase 5 | 1.x.x+ | LTS — long-term support |

---

## Non-Goals (Explicitly Out of Scope)

1. **OmniLang is NOT a general-purpose programming language.** It will never have loops, functions with bodies, or manual memory management.
2. **OmniLang does NOT replace testing frameworks.** It defines tests; execution uses existing tools (Jest, pytest, cargo test).
3. **OmniLang does NOT lock you into one AI provider.** The agent protocol is provider-agnostic.
4. **OmniLang does NOT generate "perfect" code.** It generates *verified* code. Humans can and should review and modify generated code.
5. **OmniLang does NOT eliminate the need for engineers.** It shifts their work from writing code to writing specifications, reviewing output, and designing systems.
