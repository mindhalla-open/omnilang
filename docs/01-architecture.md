# Language Architecture

## Overview

OmniLang is structured as a layered architecture with clear separation between specification, orchestration, and execution concerns. Unlike a classical compiler that is fully deterministic, OmniLang's compilation engine is **dual-phase**: it combines a deterministic static analysis frontend with a probabilistic AI-driven generation backend, connected by a closed-loop verification cycle.

```
┌─────────────────────────────────────────────────────┐
│                  Human Interface                     │
│          (IDE, CLI, Editor, Review UI)               │
├─────────────────────────────────────────────────────┤
│               Specification Layer                    │
│     (OmniLang source files: .omni, .omnitest)       │
├─────────────────────────────────────────────────────┤
│         Semantic Analysis Layer [DETERMINISTIC]      │
│    (Parser → AST → Type Checker → Constraint        │
│     Resolver → Dependency Graph)                    │
├─────────────────────────────────────────────────────┤
│         Orchestration Layer [DETERMINISTIC]          │
│    (Agent Router → Task Planner → Budget Manager    │
│     → Retry Controller)                             │
├─────────────────────────────────────────────────────┤
│        Agent Execution Layer [PROBABILISTIC]         │
│    (Code Generator → Test Generator → Self-Refine)  │
├─────────────────────────────────────────────────────┤
│        Verification Layer [DETERMINISTIC]            │
│    (Test Runner → Constraint Verifier → Evidence    │
│     Collector → Benchmarker → Security Scanner)     │
├──────────────────────┬──────────────────────────────┤
│    ▲ FAIL (retry)    │    ✓ PASS                    │
│    └─────────────────┘    │                         │
├─────────────────────────────────────────────────────┤
│               Output Artifact Layer                  │
│    (Source Code → Tests → Infra Manifests →         │
│     Documentation → Traces → Reports)               │
├─────────────────────────────────────────────────────┤
│        Production Runtime Layer [OPTIONAL]           │
│    (Live Policy Interpreter → Guardrails Engine →   │
│     Intent-to-SystemPrompt Transpiler)              │
└─────────────────────────────────────────────────────┘
```

---

## Layer Details

### 1. Specification Layer

This is where humans work. OmniLang source files (`.omni`) contain:

- **Intent blocks** — what the system should do
- **Constraint blocks** — non-functional requirements and policies
- **Contract blocks** — typed interfaces with pre/postconditions
- **Test blocks** — scenarios, properties, visual expectations
- **Evidence references** — links to logs, traces, screenshots, golden files
- **Budget blocks** — token limits, cost caps, model preferences

The specification layer is **purely declarative**. There is no control flow, no loops, no function bodies. It describes the *shape* of a solution, not the solution itself.

#### File Types

| Extension | Purpose |
|-----------|---------|
| `.omni` | Primary specification files |
| `.omnitest` | Standalone test specification files |
| `.omnipolicy` | Organization-wide constraint policies |
| `.omnimod` | Module definition and export declarations |

### 2. Semantic Analysis Layer

The OmniLang **analyzer** (analogous to a traditional compiler frontend) performs:

```
Source (.omni) → Lexer → Parser → AST
                                    ↓
                            Type Checker
                                    ↓
                          Constraint Resolver
                                    ↓
                          Dependency Graph Builder
                                    ↓
                        Validated Specification IR
```

**Key operations:**

- **Parsing** — transforms `.omni` text into an Abstract Syntax Tree
- **Type checking** — validates that all types are defined, inputs match outputs across services, confidence types are properly handled
- **Constraint resolution** — checks for conflicting constraints, validates that budget allocations are feasible, verifies that policies are satisfiable
- **Dependency graphing** — determines build order, detects circular dependencies, identifies parallelizable generation tasks

The output is a **Validated Specification IR** (Intermediate Representation) — a structured, verified representation of the entire system specification.

### 3. Orchestration Layer

The orchestration layer is the "brain" that translates a validated spec into a plan for AI agents. It is responsible for:

#### Agent Router

Selects the right agent (or model) for each task based on:
- Task complexity (simple CRUD vs. complex algorithm)
- Budget constraints (cheap model for boilerplate, expensive for critical logic)
- Domain expertise (security-specialized agent for auth, UI agent for frontend)

#### Task Planner

Decomposes the specification into ordered generation tasks:

```
Spec: service Checkout
  ├── Task 1: Generate data types (PaymentStatus, Token)
  ├── Task 2: Generate service implementation
  ├── Task 3: Generate unit tests
  ├── Task 4: Generate integration tests
  ├── Task 5: Generate infrastructure manifest
  └── Task 6: Run verification suite
```

After planning, the Task Planner serializes all task data into **OMWF (OmniLang Minimized Wire Format)** — a compact text format optimized for LLM tokenizers that reduces inter-agent token overhead by 40-50% compared to JSON. See [Agent Protocol — OMWF](./08-agent-protocol.md#token-optimized-wire-format-omwf) for details.

#### Budget Manager

Tracks token consumption, API costs, and time budgets. Can:
- Pause generation if cost exceeds the cap
- Switch to cheaper models mid-generation
- Report cost breakdown per service/module

#### Retry Controller

Manages the generate-verify-retry loop:

```
Generate → Verify → PASS → Emit artifacts
              ↓
            FAIL → Analyze failure → Adjust prompt/constraints → Retry
              ↓
        MAX_RETRIES → Escalate to Human (with full evidence log)
```

### 4. Agent Execution Layer

Where AI agents do the actual work. This layer is **probabilistic** — its output is non-deterministic and must always be verified by the deterministic verification layer below.

- **Code Generator** — produces implementation code in the target language
- **Test Generator** — creates executable test implementations from test specs
- **Infrastructure Generator** — produces deployment manifests
- **Self-Refinement Engine** — when verification fails, automatically constructs enhanced prompts with error context, logs, and traces and re-submits to the agent

The agent execution layer runs on the [Three-Tier AI Runtime](#three-tier-ai-runtime) described below.

### 5. Verification Layer

This layer is **fully deterministic** — no AI involved. It executes inside an isolated sandbox (container) and runs:

- **Test Runner** — executes generated tests against generated code
- **Constraint Verifier** — checks non-functional requirements (performance benchmarks, security scans, size limits)
- **Benchmarker** — runs load tests, measures p95/p99 latency, throughput
- **Security Scanner** — SAST/DAST analysis, dependency audit, secret scanning
- **Evidence Collector** — gathers logs, diffs, metrics, and screenshots as proof of correctness

If verification fails, the system loops back to the Agent Execution Layer with structured error feedback. This closed loop is what makes the compiler self-healing.

### 6. Output Artifact Layer

The final output of "compiling" an OmniLang spec. **Critically, the output is pure, deterministic code that does NOT require AI to run in production.** The AI was only needed during the build.

| Artifact | Description |
|----------|-------------|
| Source code | Implementation in target language (Rust, TypeScript, Go, Python, etc.) |
| Test suites | Unit, integration, property-based, and acceptance tests |
| Infrastructure manifests | Dockerfiles, Kubernetes YAMLs, Terraform configs |
| API documentation | Auto-generated from contracts |
| Verification report | Evidence that all constraints were met |
| Cost report | Token/API usage breakdown |
| Trace log | Full generation trace for audit and reproducibility |

### 7. Production Runtime Layer (Optional)

For systems that include AI agents operating in production (e.g., customer support bots, dynamic decision engines), OmniLang provides a **runtime interpretation mode**. See [Runtime Interpretation](./14-runtime-interpretation.md) for full details.

In this mode, OmniLang specs are not compiled away — they run as **live policy interpreters** that enforce constraints on production AI agents in real time:

- **Intent-to-SystemPrompt Transpiler** — translates OmniLang constraints into immutable system prompts that the LLM cannot override
- **Guardrails Engine** — monitors agent output streams and blocks responses that violate spec invariants (e.g., plaintext card numbers)
- **Policy Hot-Reload** — update constraints without redeploying the underlying service

---

## Architecture Diagram

```
                          ┌──────────────┐
                          │  Developer   │
                          │  writes .omni│
                          └──────┬───────┘
                                 │
                                 ▼
                    ┌────────────────────────┐
                    │   OmniLang Analyzer     │
                    │  (parse, type-check,    │
                    │   resolve constraints)  │
                    └────────────┬────────────┘
                                 │
                          Validated Spec IR
                                 │
                                 ▼
                    ┌────────────────────────┐
                    │   Orchestrator          │
                    │  (plan tasks, route     │
                    │   agents, manage budget)│
                    └────────────┬────────────┘
                                 │
                    ┌────────────┼────────────┐
                    ▼            ▼            ▼
              ┌──────────┐ ┌──────────┐ ┌──────────┐
              │ Agent:    │ │ Agent:    │ │ Agent:    │
              │ CodeGen   │ │ TestGen   │ │ InfraGen  │
              └─────┬─────┘ └─────┬─────┘ └─────┬─────┘
                    │             │             │
                    ▼             ▼             ▼
              ┌──────────────────────────────────────┐
              │        Verification Engine            │
              │  (run tests, check constraints,       │
              │   collect evidence, benchmark)         │
              └──────────────────┬───────────────────┘
                                 │
                           ┌─────┴─────┐
                           │           │
                        PASS         FAIL
                           │           │
                           ▼           ▼
                    ┌────────┐   ┌──────────┐
                    │ Emit   │   │ Retry /  │
                    │Artifacts│  │ Escalate │
                    └────────┘   └──────────┘
```

---

## Three-Tier AI Runtime

The AI agents powering OmniLang compilation are not a monolithic "call to an API." They operate across three tiers, split between two fundamentally different execution environments — **Thinking** (reasoning on GPUs) and **Acting** (executing on CPUs).

### The Thinking / Acting Split

This is the architectural divide between "brains" and "hands":

```
┌─────────────────────────────────────────────────────────────┐
│           THINKING ENVIRONMENT (GPU Clusters)                │
│                                                             │
│  Where LLMs reason about code, architecture, and solutions  │
│  Runs on: GPU accelerators (local or cloud)                 │
│                                                             │
│  ┌──────────────────────┐  ┌──────────────────────┐        │
│  │ TIER 1: LOCAL SLM    │  │ TIER 2: CLOUD        │        │
│  │                      │  │ REASONING            │        │
│  │ • 7B-13B params      │  │                      │        │
│  │ • Quantized (GGUF)   │  │ • Multi-Agent System │        │
│  │ • Offline-capable    │  │ • Frontier models    │        │
│  │ • <1s latency        │  │ • 5s – 5min latency  │        │
│  │ • Cost: FREE         │  │ • Cost: $0.01-$1.00+ │        │
│  │                      │  │                      │        │
│  │ Tasks:               │  │ Agents:              │        │
│  │ • Autocomplete       │  │ • Architect           │        │
│  │ • Syntax validation  │  │ • Coder               │        │
│  │ • Simple code gen    │  │ • QA (adversarial)    │        │
│  │                      │  │ • Security Auditor    │        │
│  └──────────────────────┘  └──────────────────────┘        │
└─────────────────────────────┬───────────────────────────────┘
                              │
                    [ Commands via MCP ]
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│            ACTING ENVIRONMENT (CPU MicroVMs)                 │
│                                                             │
│  Where generated code is built, tested, and verified        │
│  Runs on: Standard CPU servers (isolated from production)   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ TIER 3: AGENTIC SANDBOX (MicroVM)                   │   │
│  │                                                     │   │
│  │  Isolation: Firecracker MicroVM / Kata Containers   │   │
│  │  Boot time: 80-100ms                                │   │
│  │  Lifecycle: Ephemeral (destroyed after each task)   │   │
│  │                                                     │   │
│  │  Provides:                                          │   │
│  │  ├── Language runtimes (Node.js, Rust, Python, Go)  │   │
│  │  ├── Database emulators (Postgres, Redis, DynamoDB) │   │
│  │  ├── Network mocks (HTTP, gRPC service stubs)       │   │
│  │  ├── CPU/Memory profilers (SLO constraint checking) │   │
│  │  ├── Security scanners (SAST, secret detection)     │   │
│  │  └── Benchmark harnesses (k6, wrk, custom load gen) │   │
│  │                                                     │   │
│  │  Security layers:                                   │   │
│  │  ├── Own Linux kernel (not shared with host)        │   │
│  │  ├── Zero-Trust network (egress whitelist only)     │   │
│  │  └── Resource limits (CPU, memory, process caps)    │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

**Why separate hardware?**

| Concern | Thinking (GPU) | Acting (CPU) |
|---------|---------------|-------------|
| **Hardware** | GPU accelerators (A100, H100, Apple Silicon) | Standard x86/ARM servers |
| **Cost** | Expensive (LLM inference) | Cheap (compile, test, bench) |
| **Nature** | Probabilistic (AI reasoning) | Deterministic (code execution) |
| **Security risk** | Low (read-only access to specs) | High (executes untrusted code) |
| **Network** | May access model APIs | Hermetically isolated |
| **State** | Stateless between tasks | Ephemeral per-task VMs |

### How the Tiers Interact

```
Developer types .omni spec
        │
        ▼
   ┌─────────────┐    Fast, free,    ┌────────────────────┐
   │ Tier 1: SLM │ ──────────────► │ Autocomplete,       │
   │ (Local)      │    offline       │ inline validation,  │
   └──────┬──────┘                  │ simple codegen      │
          │                          └────────────────────┘
          │ omni build
          ▼
   ┌──────────────┐   Complex tasks  ┌────────────────────┐
   │ Tier 2: Cloud│ ──────────────► │ Multi-agent code    │
   │ (Reasoning)  │   delegated      │ generation, review, │
   └──────┬───────┘                  │ architecture design │
          │                          └────────────────────┘
          │ Generated artifacts
          ▼
   ┌──────────────┐   Deterministic  ┌────────────────────┐
   │ Tier 3:      │ ──────────────► │ Run tests, bench,   │
   │ Sandbox      │   verification   │ security scan,      │
   └──────┬───────┘                  │ collect evidence    │
          │                          └────────────────────┘
          │
    ┌─────┴─────┐
    │           │
  PASS        FAIL → feedback to Tier 2 for retry
    │
    ▼
  Emit Artifacts
```

### Local-First Philosophy

OmniLang follows a **local-first** approach:

1. `omni check` runs entirely in Tier 1 — free, instant, offline
2. `omni plan` runs in Tier 1 — estimates cost before touching cloud
3. `omni build` uses Tier 2 only when necessary — simple services can be generated locally by Tier 1 SLMs
4. Tier 3 sandbox can run locally (Docker) or in cloud — configurable per project

This means a developer can work offline for spec authoring and validation, only connecting to cloud when they need to generate complex implementations.

---

## Dual-Phase Compilation: Deterministic + Probabilistic

Unlike a classical compiler that is fully deterministic (same input → same output), OmniLang's compilation engine explicitly separates two fundamentally different phases:

| Phase | Nature | Speed | Cost | AI Involved? |
|-------|--------|-------|------|-------------|
| **Static Analysis** | Deterministic | Milliseconds | Free | No |
| **Planning** | Deterministic | Seconds | Free | No |
| **Generation** | Probabilistic | Seconds–Minutes | $$$ | Yes |
| **Verification** | Deterministic | Seconds–Minutes | Free* | No |
| **Emit** | Deterministic | Milliseconds | Free | No |

\* Verification itself is free, but running benchmarks/load tests may consume compute resources.

The key insight: **the probabilistic phase (AI generation) is always sandwiched between deterministic phases (analysis and verification)**. This means:

- The AI can never produce output that violates the type system (caught by analysis)
- The AI can never produce output that fails tests or constraints (caught by verification)
- The only thing the AI controls is *how* to implement the spec — and that implementation is always proven correct before it leaves the build system

```
   DETERMINISTIC              PROBABILISTIC              DETERMINISTIC
  ┌──────────────┐          ┌──────────────┐          ┌──────────────┐
  │ Static       │          │ AI Agent     │          │ Verification │
  │ Analysis     │ ──────►  │ Generation   │ ──────►  │ Engine       │
  │              │          │              │          │              │
  │ • Parse AST  │          │ • Code synth │          │ • Run tests  │
  │ • Type check │          │ • Test synth │          │ • Benchmark  │
  │ • Validate   │          │ • Infra gen  │          │ • Sec scan   │
  │   constraints│          │              │          │ • Evidence   │
  └──────────────┘          └──────────────┘          └──────┬───────┘
                                    ▲                        │
                                    │     FAIL               │
                                    └────────────────────────┘
                                    (structured error feedback)
```

---

## Code Storage Architecture: Three-Tier Storage Model

The question of where and how to store generated code brings us to a fundamental realization: **AI-generated code is no longer just static text in a file** as it was when written by humans. Instead, it is a dynamic, version-controlled, and highly contextual artifact.

To manage this, OmniLang organizes the storage of specifications and generated code across three distinct tiers based on its lifecycle and purpose:

```
┌─────────────────────────────────────────────────────────────┐
│                    Three-Tier Storage Model                 │
├─────────────────────────────────────────────────────────────┤
│ 1. Git Repository (Single Source of Truth)                 │
│    - Human-readable specifications (.omni)                  │
│    - Production-ready generated source code (Rust/Go/TS)    │
│    - Full audit trail, diffs, and version tracking          │
├─────────────────────────────────────────────────────────────┤
│ 2. Runtime Artifact Registry (Bazel-style Cache)            │
│    - Content-addressable cache (.omni/cache/)               │
│    - Fast lookup via BLAKE3 block hashes                    │
│    - Prevents redundant LLM calls and speeds up compilation  │
├─────────────────────────────────────────────────────────────┤
│ 3. Agentic Memory (Vector DB & Knowledge Graph)             │
│    - Local agentic memory (.omni/memory/)                   │
│    - Reasoning traces (failures, fixes, design decisions)   │
│    - Prevents hallucinations by reusing developer experience  │
└─────────────────────────────────────────────────────────────┘
```

### 1. Permanent Storage (Single Source of Truth): Traditional Git

Even though the code is written by AI agents, in production it lives exactly where human-written code does: in standard Git repositories (GitHub, GitLab, or local Git servers).

* **Transparency for the Team**: Git serves as the universal interface shared by humans and AI agents. When an agent finishes compiling `.omni` specs into code (e.g., Rust, Go, or TypeScript), it automatically creates a `git commit` and performs a `git push`.
* **Automatic Versioning**: Each change in the OmniLang specification corresponds to a strict Git commit containing the matching generated code. If something goes wrong in production, developers or the orchestrator agent can immediately run `git checkout` to return the system to a previous stable state.

### 2. Runtime Cache (The Artifact Registry): Build Acceleration

Generating code from scratch, running it in isolated microVMs, and iteratively fixing errors is a heavy, probabilistic process that takes time (seconds to minutes) and incurs API costs (tokens). Re-running this flow on every build when specifications haven't changed would be highly inefficient.

To prevent this, `OmniRuntime` implements a **Content-Addressable Storage (CAS)** system similar to modern build tools like Bazel or Turborepo:

1. Every semantic block in an OmniLang spec (e.g., a specific RPC method in `service Checkout`) is assigned a unique BLAKE3 hash. This hash depends on the intent text, typings, contracts, and the specific version of the AI model.
2. Compiled artifacts (source code, tests, binary builds) are stored in a local or cloud cache indexed by this hash.
3. During a build, the compiler computes the hash of each block. If the hash matches an entry in the cache, the runtime immediately retrieves the pre-verified code without making any calls to the LLM.

### 3. Agentic Knowledge Base (Vector DB & Graph Memory)

This is a fundamentally new storage layer designed specifically for AI-driven development. For agents to improve as the project evolves, they need to "remember" how they wrote the code, what errors they encountered, and why they chose a particular architecture.

An agentic knowledge base (powered by vector databases or knowledge graphs) runs side-by-side with the Git repository:

* **Reasoning Traces**: Stores the logs of the agent's decision-making process. For example: *"In version 1.2, I tried using library X, but the `PCI_safe` constraint check failed due to header logging. I refactored the code using library Y."*
* **Contextual Search**: When a developer adds new requirements or modifies a spec in the future, the agent first queries its local memory to see how it resolved similar problems in the past. This enables the agent to reuse project-specific experience and drastically reduces LLM hallucinations.

---

### Physical Project Directory Structure

A typical OmniLang project folder is laid out as follows:

```text
my-awesome-app/
├── schema.omni            # Human-authored specifications, goals, and contracts
├── .omni/                 # Hidden runtime directory (analogous to .git or .next)
│   ├── cache/             # Content-addressable AST trees and generated caches
│   └── memory/            # Local vector/graph database of the agent's experience
└── generated/             # Target production-ready files generated by the agents
    ├── src/               # Clean, verified source code (Rust, Go, TypeScript, etc.)
    ├── tests/             # Generated unit, integration, and property-based tests
    └── Dockerfile         # Infrastructure manifests for automated deployment
```

---

## Design Decisions

### Why Not Embed in an Existing Language?

We considered embedding OmniLang as a DSL within TypeScript, Python, or Rust. We chose a standalone language because:

1. **Neutral target** — OmniLang should generate *any* target language, not be biased toward one
2. **Simpler tooling** — a focused syntax enables better IDE support, validation, and error messages
3. **No escape hatches** — embedding in a general-purpose language tempts developers to write implementation code, defeating the purpose
4. **Clean separation** — the spec should be clearly distinct from the implementation

### Why Not Just Use Structured Markdown?

Markdown is ambiguous. Two developers reading the same Markdown spec will interpret it differently. OmniLang provides:

1. **Formal grammar** — unambiguous parsing
2. **Type checking** — catches mismatches at spec time
3. **Constraint validation** — detects impossible or conflicting requirements
4. **Tooling foundation** — enables autocomplete, refactoring, and go-to-definition

### Why File-Based, Not GUI-Based?

Files are:
- Version-controllable (Git)
- Diffable (code review)
- Scriptable (CI/CD)
- Composable (imports)
- Universal (any editor)

OmniLang is files-first, but rich IDE support (visual previews, inline evidence rendering) is a core part of the tooling layer.

### Why Separate Deterministic and Probabilistic Phases?

This is a fundamental design principle. By clearly isolating where AI "lives" in the pipeline:

1. **Debuggability** — when something goes wrong, you know instantly whether it's a spec error (deterministic) or an agent error (probabilistic)
2. **Security** — the deterministic layers enforce hard guarantees that the AI cannot bypass
3. **Cost control** — deterministic phases are free; you only pay for the probabilistic phase
4. **Reproducibility** — with the same model version and seed, probabilistic generation can be made reproducible via the lock file
5. **Trust** — stakeholders can trust the output because they know it passed deterministic verification, regardless of which AI model generated it
