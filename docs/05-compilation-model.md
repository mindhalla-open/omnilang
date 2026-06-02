# Compilation Model

In OmniLang, "compilation" means something fundamentally different from traditional languages. Instead of transforming source code into machine code, OmniLang transforms **specifications into verified, working systems**.

---

## The New Build Pipeline

### Traditional Compilation

```
Source Code → Lexer → Parser → AST → IR → Optimizer → Machine Code
```

### OmniLang Compilation

```
Spec (.omni) → Analyzer → Validated IR → Orchestrator → AI Agents → Generated Code
                                                                         ↓
                                                                    Verifier
                                                                    ↓      ↓
                                                                  PASS    FAIL
                                                                    ↓      ↓
                                                              Emit Artifacts  Retry/Escalate
```

The key difference: **the build process includes generation AND verification as a single atomic operation**. You don't get output until it's proven correct.

---

## Compilation Phases

### Phase 1: Static Analysis

**Input:** `.omni` source files  
**Output:** Validated Specification IR  
**Time:** Milliseconds (deterministic, no AI involved)

This phase is traditional compiler work:

```
1. Lexical Analysis     → Token stream
2. Parsing              → Abstract Syntax Tree (AST)
3. Name Resolution      → Resolved symbol table
4. Type Checking        → Type-checked AST
5. Constraint Validation → Verified constraints (no conflicts, no impossibilities)
6. Dependency Analysis  → Ordered dependency graph
7. IR Emission          → Validated Specification IR
```

**Errors caught in this phase:**
- Syntax errors
- Undefined types or services
- Type mismatches between connected services
- Conflicting constraints (e.g., `latency < 10ms` + `must_call_external_api`)
- Circular dependencies
- Missing required fields

### Phase 2: Planning

**Input:** Validated Specification IR  
**Output:** Execution Plan  
**Time:** Seconds

The orchestrator creates a plan for AI agents:

```omnilang
// Internal representation (not user-facing)
execution_plan {
  tasks: [
    {
      id: "T1"
      type: generate_types
      spec: [PaymentStatus, Token, Order]
      agent: CodeGenAgent
      model: CheapFast
      budget: $0.02
      dependencies: []
    },
    {
      id: "T2"
      type: generate_service
      spec: CheckoutService
      agent: CodeGenAgent
      model: Balanced
      budget: $0.10
      dependencies: [T1]
    },
    {
      id: "T3"
      type: generate_tests
      spec: CheckoutService.tests
      agent: TestGenAgent
      model: Balanced
      budget: $0.05
      dependencies: [T1, T2]
    },
    {
      id: "T4"
      type: verify
      checks: [run_tests, benchmark, security_scan]
      dependencies: [T2, T3]
    }
  ]

  parallelism: T1 → (T2, T3 parallel where possible) → T4
  total_budget: $0.25
  max_retries: 3
}
```

#### Spec IR Minification

After constructing the execution plan, the orchestrator serializes the Spec IR into **OMWF (OmniLang Minimized Wire Format)** — a compact, token-optimized text format that produces 40-50% fewer LLM tokens than equivalent JSON. This minification step ensures maximum context efficiency when the spec data is injected into agent prompts.

See [Agent Protocol — Token-Optimized Wire Format](./08-agent-protocol.md#token-optimized-wire-format-omwf) for the full OMWF specification.

### Phase 3: Generation (Probabilistic)

**Input:** Execution Plan + Spec IR  
**Output:** Raw generated artifacts  
**Time:** Seconds to minutes (AI-dependent)  
**Nature:** ⚡ Probabilistic — this is the only phase where AI is involved

Each task in the plan is dispatched to an AI agent. The agent selection follows the [Three-Tier Runtime](./01-architecture.md#three-tier-ai-runtime) model:

- **Simple tasks** (type generation, boilerplate CRUD) → Tier 1 Local SLM (free, instant)
- **Complex tasks** (business logic, security-critical code) → Tier 2 Cloud Reasoning
- **Specialized tasks** (UI, ML pipelines) → Tier 2 with domain-specific agents

Each agent receives:

1. The specific spec block to implement (from the Validated Spec IR)
2. Target language preference
3. Relevant context (dependent types, connected services, project policies)
4. Constraints to respect (as structured metadata, not just text)
5. Examples and evidence references (including human-provided diagrams, traces)
6. All data serialized in [OMWF](./08-agent-protocol.md#token-optimized-wire-format-omwf) (token-optimized wire format) for maximum context efficiency

The agent produces:

1. Implementation source code
2. Test implementations
3. Configuration files
4. Infrastructure manifests

### Phase 4: Verification (Deterministic, Sandboxed)

**Input:** Generated artifacts + Spec constraints + Test definitions  
**Output:** Verification report  
**Time:** Seconds to minutes  
**Nature:** 🔒 Fully deterministic — no AI involved

Verification runs inside the **Agentic Sandbox** — an isolated micro-container environment. See [Sandbox Environment](#sandbox-environment) below for details.

The verification engine runs:

```
Verification Suite
├── Compile Check       → does the generated code even compile/type-check?
├── Unit Tests          → run generated test code
├── Property Tests      → run property-based tests with shrinking
├── Contract Checks     → verify pre/postconditions hold
├── Constraint Checks
│   ├── Performance     → run benchmarks in sandbox profiler, check SLOs
│   ├── Security        → run SAST/DAST scanners
│   ├── Size            → check bundle size, binary size
│   └── Style           → run linters, formatters
├── Visual Tests        → compare screenshots against golden files
├── Type Safety         → verify generated code type-checks in target language
└── Integration Tests   → verify service-to-service contracts via mock services
```

### Phase 5: Self-Refinement Loop or Emit

This is where the **closed-loop** nature of OmniLang compilation becomes apparent.

**If verification PASSES:**
- Emit all artifacts to the output directory
- Generate verification report with full evidence chain
- Compute final confidence level
- Report cost breakdown
- The output is clean, optimized code covered by tests — **AI is no longer needed at runtime**

**If verification FAILS — Self-Refinement kicks in:**

The system does not simply "retry with the same prompt." It performs **structured error feedback injection**:

```
┌──────────────────────────────────────────────────────────┐
│                 Self-Refinement Loop                      │
│                                                          │
│  1. Collect failure evidence:                            │
│     • Which test failed? (name, assertion, line)         │
│     • What was expected vs actual?                       │
│     • Stack trace of the failure                         │
│     • Relevant generated code snippet                    │
│     • Performance profiler output (if SLO violation)     │
│     • Security scanner report (if vuln found)            │
│                                                          │
│  2. Construct enhanced context for the agent:            │
│     • Original spec (unchanged)                          │
│     • Previous generated code (the failing version)      │
│     • Structured error report                            │
│     • Specific hints: "line 45: idempotency check        │
│       not consulting the store before processing"        │
│                                                          │
│  3. Re-dispatch to agent with enhanced prompt            │
│     • Budget tracking: deduct retry cost                 │
│     • May escalate to a more powerful model              │
│       (e.g., Balanced → SmartExpensive)                  │
│                                                          │
│  4. Re-verify the new output                             │
│     • If PASS → emit artifacts                           │
│     • If FAIL → repeat (up to max_retries)               │
│     • If max_retries exceeded → escalate to human        │
│       with full evidence log and all attempted solutions │
└──────────────────────────────────────────────────────────┘
```

The error message the agent receives is not a vague "try again" — it is a structured compilation error:

```
OmniLang Compilation Error [E4012]: Constraint Violation

  Service: Checkout
  Constraint: latency(p95: <200ms)
  Measured: p95 = 342ms

  Benchmark Details:
    Tool: k6 v0.49
    Duration: 60s, 1000 VUs
    p50: 89ms, p95: 342ms, p99: 891ms

  Likely Cause:
    checkout.ts:67 — synchronous database call inside hot path
    Suggestion: consider async batch or connection pooling

  Previous Attempts: 1/3
  Remaining Budget: $0.15
```

---

## Build Modes

### `omni build` (Full Build)

Runs the complete pipeline: analyze → plan → generate → verify → emit.

```bash
$ omni build
[1/5] Analyzing specs... ✓ (12 services, 47 types, 89 tests)
[2/5] Planning execution... ✓ (23 tasks, estimated cost: $0.42)
[3/5] Generating artifacts...
  ├── Types (14/14) ✓
  ├── Services (12/12) ✓
  └── Tests (89/89) ✓
[4/5] Verifying...
  ├── Unit tests: 156 passed, 0 failed ✓
  ├── Property tests: 23 passed ✓
  ├── Benchmarks: all within SLO ✓
  ├── Security scan: no issues ✓
  └── Visual diff: 0 regressions ✓
[5/5] Emitting artifacts... ✓

Build succeeded.
  Confidence: High
  Total cost: $0.38
  Duration: 2m 14s
  Output: ./build/
```

### `omni check` (Analysis Only)

Runs only Phase 1. No AI, no cost. Validates specs are internally consistent.

```bash
$ omni check
Analyzing specs... ✓
12 services, 47 types, 89 tests, 0 errors, 2 warnings.

Warnings:
  - checkout.omni:45 — service Checkout has no performance constraints
  - auth.omni:12 — type Password missing storage constraint
```

### `omni plan` (Dry Run)

Runs Phases 1-2. Shows what would be generated and estimated cost.

```bash
$ omni plan
Execution Plan:
  23 generation tasks
  12 services to implement in TypeScript
  89 tests to generate
  Estimated cost: $0.42 (±$0.15)
  Estimated time: 2-4 minutes
  Model allocation:
    - CheapFast: 14 tasks ($0.08)
    - Balanced: 7 tasks ($0.21)
    - SmartExpensive: 2 tasks ($0.13)
```

### `omni verify` (Re-verify)

Runs only Phase 4 against existing generated artifacts. Useful after manual edits.

```bash
$ omni verify
Running verification suite against ./build/...
  ├── Unit tests: 155 passed, 1 failed ✗
  │   └── FAIL: checkout_test.ts:45 — "Duplicate request returns same result"
  └── Remaining checks skipped (test failure)

Verification failed. 1 test failure.
```

### `omni regen <target>` (Partial Regeneration)

Regenerates a specific service or component without rebuilding everything.

```bash
$ omni regen service Checkout
[1/3] Analyzing Checkout spec... ✓
[2/3] Regenerating Checkout service...
  ├── Implementation ✓
  └── Tests ✓
[3/3] Verifying Checkout... ✓

Regeneration succeeded. Confidence: High. Cost: $0.08.
```

---

## Incremental Compilation

OmniLang supports incremental builds. When a spec file changes, only affected components are regenerated:

```
Change in types.omni (modified: Order type)
  └── Affected services: Checkout, OrderManager, Shipping
      └── Affected tests: checkout_tests, order_tests, shipping_tests
          └── Regenerate only these 6 targets (not the entire project)
```

The dependency graph (built in Phase 1) determines the minimal rebuild set.

---

## Sandbox Environment

The **Agentic Sandbox** is the execution environment where verification happens. Unlike simple Docker containers (which share the host kernel and are vulnerable to kernel exploits), OmniLang uses **MicroVMs** — lightweight virtual machines with their own Linux kernel that boot in milliseconds.

> **Core principle:** *"Never trust code generated by an AI agent with your local machine or production cluster."*

### Three Security Layers

The sandbox implements defense-in-depth with three distinct security layers:

```
┌─────────────────────────────────────────────────────────────┐
│                    LAYER 1: MicroVM Isolation                │
│                                                             │
│  Technology: Firecracker (AWS) or Kata Containers           │
│  Boot time: 80–100ms                                        │
│  Isolation: Own Linux kernel (NOT shared with host)         │
│                                                             │
│  Unlike Docker containers that share the host kernel,       │
│  MicroVMs provide hardware-level isolation. Even if the     │
│  AI generates a kernel exploit, it cannot escape the VM.    │
├─────────────────────────────────────────────────────────────┤
│                  LAYER 2: Zero-Trust Network                 │
│                                                             │
│  Default: NO internet access (egress filtering)             │
│                                                             │
│  The agent cannot:                                          │
│  • Download unknown scripts or packages                     │
│  • Exfiltrate API keys or source code                       │
│  • Call external APIs without explicit whitelist             │
│                                                             │
│  Whitelist (configurable per project):                      │
│  • registry.npmjs.org (for npm install)                     │
│  • crates.io (for Rust dependencies)                        │
│  • pypi.org (for Python packages)                           │
│  • All other egress BLOCKED                                 │
├─────────────────────────────────────────────────────────────┤
│               LAYER 3: Resource Limits                       │
│                                                             │
│  Protection against fork bombs, infinite loops, and         │
│  resource exhaustion:                                        │
│                                                             │
│  • CPU: hard limit (e.g., 2 cores)                          │
│  • Memory: hard limit (e.g., 4GB)                           │
│  • Disk: ephemeral, capped (e.g., 10GB)                    │
│  • Process count: capped (e.g., max 256 processes)          │
│  • Session timeout: forced kill (e.g., 5 minutes)           │
│  • Open files: limited (e.g., 1024 descriptors)             │
└─────────────────────────────────────────────────────────────┘
```

### What the Sandbox Provides

```
│  • Reproducible (deterministic seed)                │
│  • Resource-limited (CPU/memory caps match spec)    │
└─────────────────────────────────────────────────────┘
```

### Sandbox Configuration

```toml
# omni.toml
[sandbox]
runtime = "docker"           # or "firecracker", "nsjail"
cpu_limit = "2 cores"
memory_limit = "4GB"
network = "none"             # hermetic by default
timeout = "5min"

[sandbox.databases]
postgresql = { version = "16", auto_provision = true }
redis = { version = "7", auto_provision = true }

[sandbox.profilers]
cpu = true
memory = true
flamegraph = true
```

### Why Hermetic?

The sandbox has **no external network access** by design. This ensures:

1. **Reproducibility** — verification results don't depend on external service availability
2. **Security** — generated code cannot exfiltrate data during verification
3. **Cost control** — no accidental calls to paid APIs during testing
4. **Speed** — no network latency; all dependencies are mocked locally

---

## Caching

Generated artifacts are content-addressed and cached:

```
Cache key = hash(spec_block + constraints + target_language + model_version)
```

If the spec hasn't changed and the same model version is used, the cached result is returned without calling the AI agent — **zero cost, instant build**.

Cache storage:
- **Local:** `.omni-cache/` directory (gitignored)
- **Remote:** Shared team cache (optional, for CI/CD)

---

## Target Languages

OmniLang can target multiple implementation languages:

| Target | Status | Best For |
|--------|--------|----------|
| TypeScript | Primary | Web services, APIs, frontend |
| Rust | Primary | Performance-critical, systems |
| Python | Primary | Data pipelines, ML services |
| Go | Primary | Cloud-native, microservices |
| Java/Kotlin | Secondary | Enterprise, Android |
| Swift | Secondary | iOS, macOS |
| C# | Secondary | .NET ecosystem |

### Target Selection

```omnilang
// Per-file target
target: typescript

// Per-service target override
service PaymentProcessor
  target rust  // performance-critical → Rust
  // ...

// Auto-select based on constraints
service MLInference
  target auto
  constraints:
    - gpu_acceleration
    - numpy_compatible
  // auto → Python (inferred from constraints)
```

---

## Output Structure

A successful `omni build` produces:

```
build/
├── src/                          # Generated implementation code
│   ├── types/                    # Shared types
│   ├── services/                 # Service implementations
│   ├── components/               # UI components
│   └── pipelines/                # Data pipelines
├── tests/                        # Generated tests
│   ├── unit/
│   ├── integration/
│   ├── property/
│   └── visual/
├── infra/                        # Infrastructure manifests
│   ├── Dockerfile
│   ├── docker-compose.yml
│   └── k8s/
├── docs/                         # Generated API documentation
├── reports/                      # Verification reports
│   ├── verification.json         # Full verification results
│   ├── coverage.html             # Test coverage
│   ├── benchmark.json            # Performance results
│   └── security.sarif            # Security scan results
├── evidence/                     # Evidence chain
│   ├── trace.jsonl               # Full generation trace
│   └── cost_report.json          # Token/cost breakdown
└── omni.lock                     # Lockfile (spec hashes + model versions)
```

---

## The Lock File

`omni.lock` ensures reproducible builds:

```json
{
  "version": "1.0.0",
  "generated_at": "2025-01-15T10:30:00Z",
  "spec_hash": "sha256:abc123...",
  "targets": {
    "service:Checkout": {
      "spec_hash": "sha256:def456...",
      "model": "claude-sonnet-4-20250514",
      "model_version": "2025-01-10",
      "generation_cost": "$0.08",
      "confidence": "High",
      "artifact_hash": "sha256:ghi789..."
    }
  }
}
```
