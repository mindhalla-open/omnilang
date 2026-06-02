# Agent Protocol

The Agent Protocol defines how AI agents interact with OmniLang specifications. It is the contract between the OmniLang runtime and any agent that generates, verifies, or reviews code.

---

## Overview

OmniLang is agent-agnostic. Any AI model or system that implements the Agent Protocol can participate in the build process. This decoupling ensures:

- **No vendor lock-in** — switch between OpenAI, Anthropic, Google, open-source models
- **Specialization** — different agents for different tasks (code gen, security review, UI)
- **Competition** — agents can be benchmarked against each other on the same specs
- **Evolution** — as models improve, the specs remain stable

---

## Agent Roles

The orchestrator assigns agents to specific roles based on the task:

| Role | Responsibility | Typical Model Profile |
|------|---------------|----------------------|
| **Architect** | Decompose specs into implementation plans | Smart, expensive |
| **CodeGen** | Generate implementation source code | Balanced |
| **TestGen** | Generate test implementations | Balanced |
| **SecurityAuditor** | Review generated code for vulnerabilities | Specialized |
| **PerformanceAnalyst** | Optimize for performance constraints | Specialized |
| **InfraGen** | Generate infrastructure manifests | Balanced |
| **UIGen** | Generate UI components and styles | Specialized (multimodal) |
| **Reviewer** | Review and critique other agents' output | Smart, expensive |
| **Fixer** | Fix failing tests or constraint violations | Balanced |

---

## Protocol Specification

### Message Format

All agent communication uses structured messages:

```json
{
  "protocol_version": "1.0",
  "message_type": "generation_request",
  "request_id": "req_abc123",
  "timestamp": "2025-01-15T10:30:00Z",

  "role": "codegen",
  "task": {
    "type": "generate_service",
    "target": "Checkout",
    "target_language": "typescript",
    "spec": { /* Validated Spec IR for Checkout service */ },
    "context": {
      "dependent_types": [ /* types this service uses */ ],
      "connected_services": [ /* services this depends on */ ],
      "project_policies": [ /* applicable policies */ ]
    }
  },

  "constraints": {
    "budget": {
      "max_tokens": 50000,
      "max_cost": "$0.10"
    },
    "requirements": [
      "idempotent",
      "PCI_safe",
      "latency_p95_under_200ms"
    ]
  },

  "expected_output": {
    "format": "structured",
    "artifacts": [
      { "type": "source_code", "language": "typescript", "path": "src/services/checkout.ts" },
      { "type": "source_code", "language": "typescript", "path": "src/services/checkout.types.ts" }
    ]
  }
}
```

### Response Format

```json
{
  "protocol_version": "1.0",
  "message_type": "generation_response",
  "request_id": "req_abc123",
  "timestamp": "2025-01-15T10:30:15Z",
  
  "status": "success",
  "confidence": "High",
  
  "artifacts": [
    {
      "type": "source_code",
      "path": "src/services/checkout.ts",
      "content": "... generated TypeScript code ...",
      "language": "typescript"
    },
    {
      "type": "source_code",
      "path": "src/services/checkout.types.ts",
      "content": "... generated type definitions ...",
      "language": "typescript"
    }
  ],
  
  "metadata": {
    "model": "claude-sonnet-4-20250514",
    "tokens_used": { "input": 12000, "output": 8500 },
    "cost": "$0.06",
    "generation_time_ms": 15000,
    "reasoning": "Implemented checkout as a stateless service with idempotency key pattern..."
  },
  
  "self_assessment": {
    "confidence": "High",
    "concerns": [],
    "assumptions": [
      "Using Stripe as payment provider based on PCI constraint",
      "Using Redis for idempotency key storage"
    ]
  }
}
```

---

## Token-Optimized Wire Format (OMWF)

### The Problem: JSON Is a Tax on Intelligence

The JSON examples above are readable for humans — but catastrophically wasteful for LLM tokenizers. Consider a simple agent response fragment:

```json
{
  "order_id": "12345",
  "status": "success",
  "confidence": "High"
}
```

Every quoted key, every colon, every brace consumes separate tokens (or forces the tokenizer into suboptimal chunking). In real-world agent communication — where specs, context, and artifacts can span tens of thousands of tokens — **30-40% of all tokens are spent on syntactic sugar** that carries zero semantic value.

This creates two cascading problems:

1. **Direct cost waste** — you're paying for empty symbols on every API call during `omni build`
2. **Context window pollution** — the model's finite context fills with noise, causing it to "forget" earlier parts of the spec or conversation faster

### The Solution: Dual-Format Architecture

OmniLang uses a **dual-format architecture** for agent communication:

```
┌─────────────────────────────────────────────────────────────────────┐
│                    HUMAN LAYER (always JSON)                        │
│                                                                     │
│  • CLI output (`omni build --verbose`)                              │
│  • IDE debug panel                                                  │
│  • Log files and traces                                             │
│  • `omni debug agent` command                                       │
│                                                                     │
│  Developers ALWAYS see full, pretty-printed JSON.                   │
│  No human ever needs to read or write OMWF.                         │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                    ┌───────────┴───────────┐
                    │  Automatic Conversion  │
                    │  JSON ↔ OMWF           │
                    │  (by the Orchestrator) │
                    └───────────┬───────────┘
                                │
┌───────────────────────────────┴─────────────────────────────────────┐
│                    AGENT LAYER (always OMWF)                        │
│                                                                     │
│  • Orchestrator → Agent task dispatch                               │
│  • Agent → Orchestrator responses                                   │
│  • Agent → Agent collaboration (multi-agent pipelines)              │
│  • Spec IR serialization for context injection                      │
│                                                                     │
│  All inter-agent traffic is OMWF. Agents never see raw JSON.        │
└─────────────────────────────────────────────────────────────────────┘
```

### OMWF Syntax

OMWF (OmniLang Minimized Wire Format) is a compact, line-oriented text format designed to produce the minimum number of tokens when processed by BPE-based tokenizers (GPT, Claude, Gemini, LLaMA, etc.).

**Design principles:**
- No quotes around keys (keys are bare identifiers)
- Minimal delimiters (whitespace and newlines are structure)
- No redundant braces for flat structures
- Compact nested notation for hierarchies
- Single-line streaming mode for low-latency channels

#### Example: Generation Request

The JSON message format shown above (52 tokens) becomes this OMWF equivalent (28 tokens, **−46%**):

```
@req req_abc123 v1.0
type generation_request
role codegen
ts 2025-01-15T10:30:00Z

task
  type generate_service
  target Checkout
  lang typescript
  context
    dependent_types [PaymentStatus Token Order]
    connected_services [PaymentGateway Inventory]
    policies [PCI_v4 GDPR]

budget
  max_tokens 50000
  max_cost 0.10

expect
  source_code typescript src/services/checkout.ts
  source_code typescript src/services/checkout.types.ts
```

#### Example: Generation Response

The JSON response (61 tokens) becomes (31 tokens, **−49%**):

```
@res req_abc123 v1.0
status success
confidence High
ts 2025-01-15T10:30:15Z

artifacts
  source_code src/services/checkout.ts typescript
    |... generated TypeScript code ...
  source_code src/services/checkout.types.ts typescript
    |... generated type definitions ...

meta
  model claude-sonnet-4-20250514
  tokens_in 12000
  tokens_out 8500
  cost 0.06
  time_ms 15000
  reasoning Implemented checkout as a stateless service with idempotency key pattern

self_assessment
  confidence High
  concerns []
  assumptions
    - Using Stripe as payment provider based on PCI constraint
    - Using Redis for idempotency key storage
```

#### Streaming Mode (Single-Line)

For low-latency inter-agent channels (e.g., agent-to-agent streaming during collaboration), OMWF supports a single-line pipe-delimited format:

```
@res req_abc123|status=success|confidence=High|tokens_in=12000|tokens_out=8500|cost=0.06
```

This is used when the orchestrator streams partial results between agents in a pipeline (e.g., Architect → CodeGen handoff).

#### Batch Mode (Arrays)

For transmitting collections (e.g., list of dependent types or test results):

```
@batch test_results
  pass checkout_idempotency 12ms
  pass checkout_validation 8ms
  fail checkout_timeout expected=200ms actual=342ms
  pass checkout_rollback 45ms
```

### Token Savings by Tier

OMWF provides different levels of impact depending on the execution tier:

| Tier | Context Window | JSON Overhead | OMWF Savings | Practical Impact |
|------|---------------|--------------|-------------|-----------------|
| **Tier 1: Local SLM** | 8K–16K tokens | 30-40% wasted | 40-50% recovered | 2× more spec data fits in context; local models can handle larger services |
| **Tier 2: Cloud** | 128K–200K tokens | 30-40% wasted | 40-50% recovered | Direct cost reduction on every `omni build`; fewer retries due to better context |
| **Streaming** | N/A | Extra tokens = latency | Fewer tokens generated | Faster Time-to-First-Token; agents act sooner in pipeline |

**Cost impact example:**

A typical `omni build` of 10 services at $0.42 total:
- With JSON wire format: ~120K tokens of inter-agent traffic
- With OMWF wire format: ~65K tokens of inter-agent traffic (−46%)
- **Savings: ~$0.08 per build** (19% cost reduction on agent communication overhead)

### Constrained Decoding Integration

The main barrier to custom AI formats is that LLMs are trained on terrabytes of JSON and may produce syntax errors when asked to output unfamiliar formats. OMWF solves this via **constrained decoding** (also called grammar-constrained generation).

When the orchestrator dispatches a task to an agent, it also provides the OMWF grammar as a **decoding constraint**:

```
Orchestrator sends to inference engine:
  1. System prompt + spec context (in OMWF)
  2. OMWF grammar schema (formal grammar for the expected response)
     │
     ▼
Inference engine (vLLM, llama.cpp, TGI, etc.)
  • Applies grammar at each token generation step
  • Model can ONLY produce tokens that are valid OMWF
  • Zero syntax errors in output — guaranteed by construction
     │
     ▼
Agent response: always valid OMWF
```

This means:
- **No fine-tuning required** — the model doesn't need to "learn" OMWF
- **Zero parsing errors** — the grammar constraint makes invalid output impossible
- **Works with any model** — any LLM + constrained decoding engine produces valid OMWF

### Configuration

OMWF is configured in `omni.toml`:

```toml
[protocol]
wire_format = "omwf"        # "json" | "omwf" | "auto"
                             # "auto" = OMWF for agents, JSON for humans (default)
debug_format = "json"        # format for debug logs and CLI output (always human-readable)
stream_mode = "single_line"  # "single_line" | "block" — for inter-agent streaming

[protocol.omwf]
version = "1.0"
# Enable token counting comparison in build reports
report_savings = true        # show "JSON: X tokens vs OMWF: Y tokens" in build output
```

When `report_savings` is enabled, every `omni build` report includes:

```
Token efficiency report:
  Wire format: OMWF v1.0
  Total agent traffic:
    JSON equivalent: 118,400 tokens
    OMWF actual:      64,230 tokens
    Savings:           54,170 tokens (45.7%)
    Cost saved:        $0.08
```

---

## Agent Lifecycle

### 1. Registration

Agents register with the orchestrator, declaring their capabilities:

```json
{
  "agent_id": "codegen-typescript-v3",
  "capabilities": ["generate_service", "generate_types", "fix_code"],
  "languages": ["typescript", "javascript"],
  "model": "claude-sonnet-4-20250514",
  "cost_per_1k_tokens": { "input": 0.003, "output": 0.015 },
  "max_context_window": 200000,
  "specializations": ["web_services", "rest_apis"],
  "benchmarks": {
    "code_quality_score": 0.92,
    "test_pass_rate": 0.97,
    "avg_generation_time_ms": 12000
  }
}
```

### 2. Task Assignment

The orchestrator selects agents based on:

```
Selection criteria:
  1. Capability match (must support the task type)
  2. Language support (must support the target language)
  3. Budget fit (cost must be within budget)
  4. Quality history (prefer agents with better benchmarks)
  5. Specialization (prefer specialists for domain-specific tasks)
```

### 3. Generation

The agent receives the task, generates artifacts, and returns them with metadata.

### 4. Verification

The orchestrator runs the verification engine against the agent's output.

### 5. Feedback Loop

If verification fails, the agent receives a **fix request**:

```json
{
  "message_type": "fix_request",
  "request_id": "req_abc123_retry1",
  "original_request_id": "req_abc123",
  "retry_number": 1,
  "max_retries": 3,
  
  "failures": [
    {
      "type": "test_failure",
      "test": "Duplicate request returns same result",
      "expected": "PaymentStatus.Completed",
      "actual": "PaymentStatus.Processing",
      "stack_trace": "...",
      "relevant_code": "checkout.ts:45-60"
    }
  ],
  
  "previous_artifacts": [ /* the failing code */ ],
  "hints": [
    "The idempotency check is not consulting the idempotency store before processing",
    "Consider checking for existing transaction before creating new one"
  ]
}
```

---

## Multi-Agent Collaboration

For complex services, multiple agents may collaborate:

```
                    ┌──────────────┐
                    │ Orchestrator │
                    └──────┬───────┘
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
        ┌──────────┐ ┌──────────┐ ┌──────────┐
        │ Architect│ │ CodeGen  │ │ TestGen  │
        │ Agent    │ │ Agent    │ │ Agent    │
        └────┬─────┘ └────┬─────┘ └────┬─────┘
             │             │             │
             │  plan       │  code       │  tests
             ▼             ▼             ▼
        ┌──────────────────────────────────────┐
        │        Reviewer Agent                 │
        │  (reviews all outputs for quality)   │
        └──────────────────┬───────────────────┘
                           │
                           ▼
                    ┌──────────────┐
                    │  Verifier    │
                    └──────────────┘
```

### Collaboration Patterns

**Sequential Pipeline:**
```
Architect → CodeGen → TestGen → SecurityAudit → Reviewer
```

**Parallel with Merge:**
```
Architect →  ┌─ CodeGen (service A) ─┐
             ├─ CodeGen (service B) ─┼─→ Merge → Verify
             └─ CodeGen (service C) ─┘
```

**Adversarial Review:**
```
CodeGen Agent 1 → generates code
CodeGen Agent 2 → generates alternative code
Reviewer Agent  → selects better implementation
```

**Iterative Refinement:**
```
CodeGen → Verify → Fail → Fixer → Verify → Pass
```

---

## MCP Integration (Model Context Protocol)

OmniLang agents interact with external tools and the verification sandbox via **MCP (Model Context Protocol)** — an open standard for agent-to-tooling communication. MCP is to AI agents what USB-C is to hardware: a universal connector that turns any database, browser, or API into a capability the agent can use.

### MCP Architecture: Host / Client / Server

The MCP integration has three distinct roles:

```
┌──────────────────────────────────────────────────────────────┐
│                    HOST (IDE / CI/CD)                         │
│                                                              │
│  The environment where the developer works.                  │
│  Manages the lifecycle of clients and servers.               │
│  Examples: Cursor, VS Code, GitHub Actions, `omni` CLI      │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                    CLIENT                               │  │
│  │                                                        │  │
│  │  Built into the host. Translates abstract OmniLang     │  │
│  │  compiler commands into structured JSON-RPC requests.  │  │
│  │  Manages sessions, routing, and authentication.        │  │
│  └────────────────────────┬───────────────────────────────┘  │
└───────────────────────────┼──────────────────────────────────┘
                            │ JSON-RPC (MCP Protocol)
                            ▼
┌──────────────────────────────────────────────────────────────┐
│                   MCP SERVERS (Tools)                         │
│                                                              │
│  Independent microservices, each giving agents a capability: │
│                                                              │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐         │
│  │ Postgres-MCP │ │ Puppeteer-MCP│ │ GitHub-MCP   │         │
│  │              │ │              │ │              │         │
│  │ Read/write   │ │ Browser for  │ │ Git commit,  │         │
│  │ DB schemas,  │ │ visual tests,│ │ PR creation, │         │
│  │ run queries  │ │ screenshots  │ │ CI triggers  │         │
│  └──────────────┘ └──────────────┘ └──────────────┘         │
│                                                              │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐         │
│  │ Benchmark-MCP│ │ Security-MCP │ │ Custom-MCP   │         │
│  │              │ │              │ │              │         │
│  │ k6, wrk,     │ │ Semgrep,     │ │ Community or │         │
│  │ vegeta,      │ │ Trivy,       │ │ org-specific │         │
│  │ returns p95  │ │ returns SARIF │ │ tooling      │         │
│  └──────────────┘ └──────────────┘ └──────────────┘         │
└──────────────────────────────────────────────────────────────┘
```

### Community MCP Servers

The community can publish MCP servers that extend OmniLang's verification capabilities:

```toml
# omni.toml
[mcp_servers]
benchmark = { package = "@omnilang/mcp-benchmark", version = "^1.0" }
security = { package = "@omnilang/mcp-security", version = "^2.0" }
visual = { package = "@omnilang/mcp-visual-diff", version = "^1.0" }
pci = { package = "@community/mcp-pci-dss", version = "^3.0" }
```

When OmniLang encounters a constraint like `expect_performance: p95 < 200ms`, it dispatches to the benchmark MCP server, which:

1. Spins up a micro-container with the benchmark tool
2. Runs the load test against the generated service
3. Returns structured results (JSON with p50/p95/p99/throughput)
4. The verification engine checks if results satisfy the constraint

### Writing a Custom MCP Server

```typescript
// Example: custom compliance checker MCP server
import { McpServer } from "@omnilang/mcp-sdk";

const server = new McpServer("hipaa-compliance");

server.addTool("check_hipaa", {
  description: "Verify HIPAA compliance of generated code",
  inputSchema: {
    source_files: "List<FilePath>",
    data_types: "List<TypeDefinition>"
  },
  handler: async (input) => {
    // 1. Scan for PII handling patterns
    // 2. Verify encryption at rest
    // 3. Check audit logging
    // 4. Validate access controls
    return {
      compliant: true,
      findings: [],
      evidence: { /* structured compliance report */ }
    };
  }
});
```

---

## Model Tiering Strategy

OmniLang maps task complexity to model tiers automatically based on the spec:

| Task Profile | Model Tier | Examples |
|-------------|-----------|----------|
| Boilerplate, type generation | **Tier 1: Local SLM** (7-13B, quantized) | CRUD services, DTO types, simple validation |
| Standard business logic | **Tier 2: Balanced** (cloud, mid-tier) | REST APIs, data transformations, standard auth |
| Security-critical, complex algorithms | **Tier 2: Smart-Expensive** (cloud, frontier) | Payment processing, cryptography, ML pipelines |
| Code review and architecture | **Tier 2: Smart-Expensive** (cloud, frontier) | Multi-service design, security audit, perf optimization |

### Tier Override in Specs

```omnilang
service PasswordHasher
  goal "Hash and verify passwords"

  budget
    model_tier SmartExpensive  // force frontier model for security
    // This overrides the automatic tier selection
```

---

## Agent Configuration

Users can configure agent behavior in `omni.toml`:

```toml
[agents]
# Default agent settings
default_model = "balanced"
max_retries = 3
timeout = "5min"

# Local model for Tier 1 tasks
[agents.local]
model = "llama-3.1-8b-code"    # or any GGUF-compatible model
path = "~/.omni/models/"
quantization = "Q4_K_M"
gpu_layers = 32                 # use GPU if available

[agents.codegen]
model = "claude-sonnet-4-20250514"
temperature = 0.2
max_tokens = 100000

[agents.testgen]
model = "gpt-4o"
temperature = 0.3
max_tokens = 50000

[agents.security]
model = "claude-opus-4-20250514"  # use best model for security
temperature = 0.1
max_tokens = 80000

[agents.reviewer]
model = "claude-opus-4-20250514"
temperature = 0.1
# Reviewer always runs, even if tests pass
always_review = true

[agents.qa]
# Adversarial QA agent — tries to break the coder's output
model = "claude-sonnet-4-20250514"
temperature = 0.4     # slightly higher temp for creative edge cases
role = "adversarial"  # generates tricky test inputs
```

---

## Custom Agents

Teams can register custom agents for domain-specific tasks:

```omnilang
agent FinancialComplianceChecker
  goal "Verify financial calculations comply with regulations"

  capabilities
    - verify_financial_accuracy
    - check_rounding_rules
    - validate_tax_calculations

  tools
    - FinancialCalculator
    - TaxRuleEngine
    - AuditTrailVerifier

  activation
    when service has constraint "financial_accuracy"
    or service handles type Money

  checks
    - all monetary calculations use Decimal, not Float
    - rounding follows Banker's Rounding (IEEE 754)
    - tax calculations match jurisdiction rules
    - audit trail captures all monetary state changes
```

---

## Agent Observability

All agent interactions are logged for debugging and audit:

```json
{
  "trace_id": "trace_xyz",
  "spans": [
    {
      "agent": "codegen-typescript-v3",
      "task": "generate_service:Checkout",
      "start": "2025-01-15T10:30:00Z",
      "end": "2025-01-15T10:30:15Z",
      "tokens": { "in": 12000, "out": 8500 },
      "cost": "$0.06",
      "result": "success",
      "confidence": "High"
    },
    {
      "agent": "testgen-v2",
      "task": "generate_tests:Checkout",
      "start": "2025-01-15T10:30:15Z",
      "end": "2025-01-15T10:30:25Z",
      "tokens": { "in": 15000, "out": 6000 },
      "cost": "$0.04",
      "result": "success",
      "confidence": "High"
    },
    {
      "agent": "verifier",
      "task": "verify:Checkout",
      "start": "2025-01-15T10:30:25Z",
      "end": "2025-01-15T10:30:35Z",
      "result": "pass",
      "tests_run": 23,
      "tests_passed": 23
    }
  ],
  "total_cost": "$0.10",
  "total_duration_ms": 35000
}
```

### Agent Benchmarking

Track agent performance over time:

```bash
$ omni agents benchmark
Agent Performance Report (last 30 days):

| Agent                    | Tasks | Pass Rate | Avg Cost | Avg Time |
|--------------------------|-------|-----------|----------|----------|
| codegen-typescript-v3    | 245   | 94.3%     | $0.07    | 14s      |
| codegen-rust-v2          | 89    | 91.0%     | $0.09    | 18s      |
| testgen-v2               | 312   | 97.1%     | $0.04    | 10s      |
| security-auditor-v1      | 178   | 99.4%     | $0.12    | 25s      |

Top failure reasons:
  1. Performance constraint violation (34%)
  2. Edge case test failure (28%)
  3. Type mismatch in generated code (19%)
  4. Missing error handling (12%)
  5. Other (7%)
```

---

## Security Model

### Agent Sandboxing (MicroVM Isolation)

Agents operate inside **MicroVMs** (Firecracker or Kata Containers), not simple Docker containers. This provides hardware-level isolation with a separate Linux kernel per sandbox:

- **Own kernel** — even a kernel exploit in generated code cannot escape the MicroVM
- **No network access** during generation (prevents data exfiltration)
- **No filesystem access** beyond the project directory
- **No persistent state** between tasks (stateless by design)
- **Token limits** enforced at the protocol level
- **Output validation** — generated code is scanned before being accepted
- **Resource caps** — CPU, memory, process count, and disk are hard-limited to prevent fork bombs and infinite loops
- **Forced timeout** — MicroVMs are destroyed after a configurable timeout (default: 5 minutes)

See [Compilation Model — Sandbox Environment](./05-compilation-model.md#sandbox-environment) for full configuration details.

### Secrets Management

Agents never receive actual secrets. Specs reference secrets by name:

```omnilang
service Database
  connection
    host env("DB_HOST")
    password secret("db_password")  // agent sees the reference, not the value
```

The orchestrator resolves secrets only during verification (in a secure runtime) — never in the agent prompt.

---

## Instrumental Deadlock Resolution (Missing Tools)

What happens when an agent encounters a task that requires an MCP tool that doesn't exist? For example, the spec says `goal: "sync data with CRM X"` but there is no `CRM-X-MCP` server available.

In OmniLang, this is not a fatal error. The system has three escalation paths:

### Path 1: JIT Tool Creation (Dynamic Extension)

If the agent has access to basic MCP tools (`HTTP_Client`, `Browser`, `FileSystem`), it can **write its own missing tool at runtime**:

```
Agent detects: no MCP server for CRM X
    │
    ▼
1. Agent uses Browser-MCP to find CRM X API documentation
2. Agent reads docs, understands auth flow and endpoints
3. Agent writes a TypeScript MCP adapter script:

    // auto-generated: crm-x-adapter.ts
    import { McpServer } from "@omnilang/mcp-sdk";
    
    const server = new McpServer("crm-x");
    server.addTool("sync_contacts", {
      handler: async (input) => {
        const token = await authenticate(input.credentials);
        return await fetch(`https://api.crm-x.com/contacts`, {
          headers: { Authorization: `Bearer ${token}` }
        });
      }
    });

4. Agent loads the adapter into its MCP runtime
5. Compilation continues with the new tool
```

This is called **JIT (Just-In-Time) Tool creation** — the agent extends its own capabilities on the fly.

### Path 2: Registry Fallback

If JIT creation is too complex, the agent searches the global MCP registry (like npm for MCP servers):

```
Agent searches: "MCP server for CRM X"
    │
    ▼
Registry returns: @community/mcp-crm-x@2.1.0
    │
    ▼
Agent downloads into sandbox MicroVM
    │
    ▼
Initializes the server, continues compilation
```

### Path 3: Human-in-the-Loop Escalation

If the API is private, undocumented, or requires physical credentials, the agent pauses and asks the human:

```
🛑 [Compilation Paused]

Reason: Missing integration tool for CRM X.
Human action required.

I attempted to create the integration automatically, but the
API requires OAuth authorization from your organization.

Required steps:
  1. Create an application at: https://crm-x.com/developer
  2. Obtain Client_ID and Client_Secret
  3. Add them to your project config:

     # omni.toml
     [secrets]
     crm_x_client_id = { env = "CRM_X_CLIENT_ID" }
     crm_x_client_secret = { env = "CRM_X_CLIENT_SECRET" }

Once configured, run `omni build --resume` to continue.
```

When the developer provides credentials, the agent auto-generates the MCP adapter, tests the connection in the sandbox, and resumes compilation.

### Resolution Priority

```
1. JIT Tool Creation  ──► fastest, fully autonomous
        │ (if too complex)
        ▼
2. Registry Fallback  ──► fast, uses community work
        │ (if not available)
        ▼
3. Human Escalation   ──► slowest, but always works
```

This three-tier fallback ensures that **OmniLang never hits a dead end** — there is always a path to completion, even for novel integrations.
