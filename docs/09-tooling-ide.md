# Tooling & IDE Support

OmniLang is designed with rich tooling as a first-class concern. The developer experience must be excellent — not just for writing specs, but for understanding, reviewing, and debugging the AI-generated output.

---

## CLI Tool (`omni`)

The `omni` command-line tool is the primary interface for working with OmniLang projects.

### Core Commands

```bash
# Project management
omni init                    # Initialize a new OmniLang project
omni init --template api     # Initialize from a template
omni check                   # Validate specs (no AI, no cost)
omni plan                    # Show execution plan and cost estimate
omni build                   # Full build: generate + verify
omni build --dry-run         # Show what would be generated

# Targeted operations
omni regen service Checkout  # Regenerate a specific service
omni verify                  # Re-verify existing artifacts
omni test                    # Run only the test suite

# Dependency management
omni deps resolve            # Resolve and lock dependencies
omni deps update             # Update dependencies
omni deps audit              # Security audit of dependencies
omni deps tree               # Show dependency graph

# Code inspection
omni coverage                # Show spec coverage report
omni diff                    # Show diff between spec and generated code
omni trace <service>         # Show generation trace for a service
omni cost                    # Show cost breakdown

# Agent management
omni agents list             # List registered agents
omni agents benchmark        # Show agent performance stats
omni agents test <agent>     # Test an agent against sample specs

# Publishing
omni publish                 # Publish spec package to registry
omni login                   # Authenticate with registry

# Development
omni watch                   # Watch specs and rebuild on change
omni serve                   # Start local dev server with hot reload
omni playground              # Interactive REPL for experimenting
```

### Command Examples

```bash
# Initialize a project
$ omni init --template microservices
Creating OmniLang project...
  ├── omni.toml ✓
  ├── specs/ ✓
  ├── policies/ ✓
  ├── golden/ ✓
  └── .omni-cache/ ✓

Project initialized. Run 'omni check' to validate.

# Quick validation (free, instant)
$ omni check
✓ 12 services, 47 types, 89 tests — no errors

# Cost estimation before building
$ omni plan
Execution Plan:
  23 tasks across 12 services
  Estimated cost: $0.42 (±$0.15)
  Estimated time: 2-4 minutes
  Proceed? [Y/n]

# Watch mode for development
$ omni watch
Watching specs/**/*.omni for changes...
[14:23:15] Changed: checkout.omni
[14:23:15] Rebuilding service Checkout... ✓ ($0.08, 12s)
[14:23:27] All tests pass.
```

---

## Language Server Protocol (LSP)

OmniLang provides a full LSP implementation for editor integration.

### Features

| Feature | Description |
|---------|-------------|
| **Syntax Highlighting** | Full grammar-based highlighting for `.omni` files |
| **Autocomplete** | Types, constraints, keywords, imported names |
| **Go to Definition** | Jump to type definitions, service declarations |
| **Find References** | Find all usages of a type, service, or constraint |
| **Hover Information** | Show type info, constraint docs, linked tests |
| **Diagnostics** | Real-time error and warning detection |
| **Code Actions** | Quick fixes, refactoring suggestions |
| **Rename Symbol** | Rename types, services across all files |
| **Document Symbols** | Outline view of all declarations |
| **Folding** | Collapse blocks, test sections, constraint lists |
| **Formatting** | Auto-format `.omni` files |
| **Signature Help** | Show parameter info for built-in functions |

### Smart Features (AI-Enhanced)

| Feature | Description |
|---------|-------------|
| **Spec Completion** | AI suggests constraints, tests based on service goal |
| **Missing Test Detection** | Highlights untested paths |
| **Constraint Conflict Warning** | Detects impossible constraint combinations |
| **Cross-Service Validation** | Validates types across service boundaries |
| **Cost Estimation** | Shows estimated generation cost inline |
| **Confidence Preview** | Predicts likely confidence level before building |

### Editor Support

| Editor | Status | Installation |
|--------|--------|-------------|
| VS Code | Primary | `ext install omnilang.omnilang` |
| JetBrains IDEs | Primary | Plugin marketplace |
| Neovim | Community | `nvim-omnilang` plugin |
| Zed | Community | Built-in (pending) |
| Sublime Text | Community | Package Control |
| Helix | Community | Tree-sitter grammar |

---

## Intent Observability Layer

Since OmniLang operates at the intent level (not the code level), the IDE needs fundamentally different debugging and inspection tools. Instead of breakpoints and variable watches, OmniLang IDE provides an **Intent Observability Layer** — tools for understanding how well your specification communicates intent to AI agents.

### Intent Entropy Analysis

The IDE analyzes each `goal` field and flags when the natural language is **too ambiguous** for reliable generation:

```
┌──────────────────────────────────────────────────────────┐
│ service DataProcessor                                     │
│   goal "process data"                                     │
│          ▲                                                │
│          │                                                │
│   ┌──────┴────────────────────────────────────────────┐   │
│   │ ⚠️  HIGH ENTROPY (score: 8.7/10)                  │   │
│   │                                                    │   │
│   │ This goal has 47 possible interpretations.         │   │
│   │ An agent may generate anything from a CSV parser   │   │
│   │ to a Kafka stream processor.                       │   │
│   │                                                    │   │
│   │ Suggestions to reduce ambiguity:                   │   │
│   │  • Specify data format (JSON, CSV, Protobuf?)      │   │
│   │  • Specify processing type (transform, filter,     │   │
│   │    aggregate, validate?)                           │   │
│   │  • Add at least 2 concrete test scenarios          │   │
│   │  • Define input/output schemas                     │   │
│   │                                                    │   │
│   │ [Suggest Improvements] [Ignore]                    │   │
│   └────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────┘
```

Entropy is computed by a local SLM that estimates how many distinct implementations could satisfy the current spec. More constraints, tests, and typed I/O reduce entropy.

### Reasoning Trace Viewer

After `omni build`, developers can click on any spec block and see the full **reasoning graph** — the chain of decisions the AI agent made during generation:

```
┌──────────────────────────────────────────────────────────┐
│ 🧠 Reasoning Trace: service Checkout                      │
│                                                          │
│ Step 1: Architect Agent                                  │
│ ├── Decision: Use event-driven architecture               │
│ ├── Reason: "idempotent" constraint + high throughput     │
│ └── Confidence: High                                      │
│                                                          │
│ Step 2: CodeGen Agent                                     │
│ ├── Decision: Chose Redis for idempotency key store       │
│ ├── Alternatives considered: PostgreSQL, DynamoDB         │
│ ├── Reason: "p95 < 200ms" constraint favors in-memory    │
│ └── Confidence: High                                      │
│                                                          │
│ Step 3: CodeGen Agent                                     │
│ ├── Decision: Stripe SDK for payment processing           │
│ ├── Reason: "PCI_DSS_v4" constraint → prefer tokenized   │
│ └── Confidence: Medium (assumption about provider)        │
│                                                          │
│ Step 4: QA Agent                                          │
│ ├── Generated 12 additional edge case tests               │
│ ├── Found: race condition in concurrent reservation       │
│ └── Status: Fixed by CodeGen on retry #1                  │
│                                                          │
│ Step 5: Verification                                      │
│ ├── All 23 tests passed                                   │
│ ├── p95 latency: 142ms ✓ (< 200ms)                       │
│ ├── Security scan: 0 issues                               │
│ └── Final confidence: High                                │
│                                                          │
│ [View Generated Code] [View Diffs] [Re-run with changes] │
└──────────────────────────────────────────────────────────┘
```

### Logical Gap Detector

The IDE continuously analyzes the specification for **logical gaps** — areas where the spec is internally consistent but underspecified:

```
┌──────────────────────────────────────────────────────────┐
│ 🔍 Logical Gap Analysis                                   │
│                                                          │
│ service OrderManager                                      │
│                                                          │
│ ⚠ Gap 1: Error path coverage                              │
│   Declared errors: 5                                      │
│   Tested errors: 3                                        │
│   Untested: InsufficientInventory, PaymentTimeout         │
│   [Generate Test Scenarios]                               │
│                                                          │
│ ⚠ Gap 2: Missing state transition                          │
│   Workflow has path: Placed → Paid → Shipped              │
│   But no path for: Paid → Refunded (before shipping)     │
│   [Add Transition] [Mark as Intentional]                  │
│                                                          │
│ ⚠ Gap 3: Cross-service type mismatch                       │
│   OrderService outputs: { items: List<OrderItem> }       │
│   ShippingService expects: { items: List<ShippableItem> } │
│   ShippableItem requires field 'weight' not in OrderItem  │
│   [View Types] [Add Missing Field]                        │
│                                                          │
│ ℹ Info: Spec coverage = 78%. Target: 85%.                 │
│ [View Full Coverage Report]                               │
└──────────────────────────────────────────────────────────┘
```

---

## IDE Features

### 1. Inline Evidence Viewer

View referenced evidence directly in the editor:

```
┌─────────────────────────────────────────────────┐
│ service Checkout                                │
│   evidence                                       │
│     - @docs/payment_flow.png  ← [Preview] 🖼️    │
│                                                  │
│       ┌─────────────────────────┐               │
│       │ [Inline image preview   │               │
│       │  of payment_flow.png]   │               │
│       └─────────────────────────┘               │
│                                                  │
│     - @traces/checkout.json  ← [Trace viewer] 📊│
└─────────────────────────────────────────────────┘
```

### 2. Test Result Overlay

After `omni build`, test results appear inline:

```
┌─────────────────────────────────────────────────┐
│ tests:                                           │
│   - scenario: "Duplicate request"  ✅ PASS (12ms)│
│     expect: same result, no duplicate charge     │
│                                                  │
│   - scenario: "Expired token"      ❌ FAIL       │
│     expect: status == Declined                   │
│     ┌─ Error: status was "Processing"            │
│     │  Expected: Declined                        │
│     │  Generated code: checkout.ts:45            │
│     │  [View Code] [Fix] [Retry]                 │
│     └────────────────────────────────────────    │
└─────────────────────────────────────────────────┘
```

### 3. Visual Diff Tool

For visual tests, a side-by-side diff viewer:

```
┌──────────────────────┬──────────────────────┐
│ Golden (Expected)     │ Actual (Generated)    │
│ ┌──────────────────┐ │ ┌──────────────────┐ │
│ │                  │ │ │                  │ │
│ │  [Screenshot]    │ │ │  [Screenshot]    │ │
│ │                  │ │ │                  │ │
│ └──────────────────┘ │ └──────────────────┘ │
│                      │                      │
│ Diff: 1.2% pixel     │ Tolerance: 2%        │
│ difference           │ Result: ✅ PASS       │
└──────────────────────┴──────────────────────┘
```

### 4. Cost Dashboard

Real-time cost tracking in the IDE:

```
┌─────────────────────────────────────────────────┐
│ 💰 Build Cost Dashboard                          │
│                                                  │
│ Current build: $0.38 / $1.00 budget              │
│ ████████████████████░░░░░░░░░░░░ 38%             │
│                                                  │
│ By service:                                      │
│   Checkout        $0.12 ███████░░░░░             │
│   Auth            $0.08 ████░░░░░░░░             │
│   Products        $0.06 ███░░░░░░░░░             │
│   Orders          $0.05 ██░░░░░░░░░░             │
│   Other (8)       $0.07 ███░░░░░░░░░             │
│                                                  │
│ By phase:                                        │
│   Generation      $0.32                          │
│   Verification    $0.06                          │
│                                                  │
│ Monthly total: $12.45                            │
└─────────────────────────────────────────────────┘
```

### 5. Dependency Graph Viewer

Interactive visualization of the service dependency graph:

```
┌─────────────────────────────────────────────────┐
│ 🔗 Service Dependency Graph                      │
│                                                  │
│    ┌─────────┐                                   │
│    │ Gateway │                                   │
│    └────┬────┘                                   │
│    ┌────┼────────┐                               │
│    ▼    ▼        ▼                               │
│ ┌─────┐ ┌──────┐ ┌────────┐                     │
│ │Auth │ │Catalog│ │Checkout│                     │
│ └─────┘ └──┬───┘ └───┬────┘                     │
│            ▼         ▼                           │
│         ┌──────┐ ┌────────┐                      │
│         │Search│ │Payment │                      │
│         └──────┘ └────────┘                      │
│                                                  │
│ [Hover for details] [Click to navigate]          │
└─────────────────────────────────────────────────┘
```

---

## CI/CD Integration

### GitHub Actions

```yaml
# .github/workflows/omni.yml
name: OmniLang Build

on:
  pull_request:
    paths: ['specs/**/*.omni']

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: omnilang/setup@v1
      - run: omni check
        # Free, fast — catches spec errors

  build:
    needs: check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: omnilang/setup@v1
      
      - name: Build from spec
        run: omni build
        env:
          OMNI_API_KEY: ${{ secrets.OMNI_API_KEY }}
          OMNI_MAX_COST: "$5.00"
      
      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: generated-code
          path: build/

      - name: Post cost report
        uses: omnilang/cost-report@v1
        with:
          report: build/reports/cost_report.json
```

### GitLab CI

```yaml
# .gitlab-ci.yml
stages:
  - check
  - build
  - verify

omni-check:
  stage: check
  script:
    - omni check
  rules:
    - changes: ["specs/**/*.omni"]

omni-build:
  stage: build
  script:
    - omni build
  artifacts:
    paths:
      - build/
    reports:
      junit: build/reports/test_results.xml
```

---

## Observability Integration

### OpenTelemetry

OmniLang emits traces compatible with OpenTelemetry:

```
Trace: omni-build-20250115
├── Span: analyze (45ms)
├── Span: plan (120ms)
├── Span: generate
│   ├── Span: agent:codegen:Checkout (15s)
│   ├── Span: agent:codegen:Auth (12s)
│   └── Span: agent:testgen:all (18s)
├── Span: verify
│   ├── Span: unit_tests (3.2s)
│   ├── Span: property_tests (8.5s)
│   ├── Span: security_scan (12s)
│   └── Span: benchmarks (65s)
└── Span: emit (200ms)
```

These traces can be sent to Jaeger, Grafana Tempo, Datadog, or any OpenTelemetry-compatible backend.

### Metrics

Standard metrics exported by the build process:

| Metric | Type | Description |
|--------|------|-------------|
| `omni_build_duration_seconds` | Histogram | Total build time |
| `omni_build_cost_dollars` | Counter | Cumulative build cost |
| `omni_build_success_total` | Counter | Successful builds |
| `omni_build_failure_total` | Counter | Failed builds |
| `omni_agent_tokens_used` | Counter | Tokens consumed per agent |
| `omni_test_pass_rate` | Gauge | Current test pass rate |
| `omni_spec_coverage` | Gauge | Spec coverage percentage |
| `omni_agent_retry_count` | Counter | Agent retry frequency |

---

## Configuration Files

### `omni.toml` (Project Config)

See [Module System](./07-module-system.md) for the full reference.

### `.omniignore` (File Exclusion)

```gitignore
# Ignore generated output
build/

# Ignore cache
.omni-cache/

# Ignore local overrides
*.local.omni

# Ignore draft specs
drafts/
```

### `.omnifmt` (Formatter Config)

```toml
[format]
indent = 2
max_line_length = 100
trailing_comma = true
sort_imports = true
sort_constraints = true
blank_lines_between_blocks = 1
```
