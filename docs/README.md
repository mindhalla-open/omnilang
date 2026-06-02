# OmniLang Documentation

> **OmniLang** — a spec-driven, AI-native language for describing intentions, constraints, and verifiable artifacts.

OmniLang is not another programming language. It is a **verification-first specification language** designed for the era of AI agents. Humans write *what* they want and *how to verify it*; agents compile that into working implementations, tests, and operational artifacts.

---

## Documentation Map

| Document | Description |
|----------|-------------|
| [Vision & Philosophy](./00-vision.md) | Why OmniLang exists and the paradigm shift it represents |
| [Language Architecture](./01-architecture.md) | Core layers, dual-phase compilation, three-tier AI runtime |
| [Core Concepts](./02-core-concepts.md) | Intent blocks, constraints, contracts, evidence, and trust types |
| [Type System](./03-type-system.md) | Data types, confidence types, multimodal types, and budget types |
| [Syntax Reference](./04-syntax-reference.md) | Complete syntax specification with examples |
| [Compilation Model](./05-compilation-model.md) | How specs become running systems via AI agents |
| [Testing & Verification](./06-testing-verification.md) | Tests as first-class citizens, property-based testing, visual diff |
| [Module System](./07-module-system.md) | Composability, imports, namespaces, and registry |
| [Agent Protocol](./08-agent-protocol.md) | How AI agents interact with OmniLang specs, MCP integration |
| [Tooling & IDE](./09-tooling-ide.md) | Editor support, LSP, intent observability, CI integration |
| [Examples](./10-examples.md) | Real-world OmniLang specifications |
| [Roadmap](./11-roadmap.md) | Evolution path and milestones |
| [Glossary](./12-glossary.md) | Key terms and definitions |
| [Ecosystem](./13-ecosystem.md) | Verification packages, MCP servers, training data, Spec Engineer role |
| [Runtime Interpretation](./14-runtime-interpretation.md) | Live policy enforcement for production AI agents |
| [Target Languages](./15-target-languages.md) | Why agents write Rust/Go/TS, not binary code |

---

## Quick Example

```omnilang
service checkout
  goal "Process payment in <200ms at p95"

  constraints:
    - idempotent
    - PCI_safe
    - no_plaintext_cards

  inputs:
    order_id uuid
    payment_token token

  outputs:
    status paymentStatus

  budget:
    max_generation_cost: $0.10
    llm_strategy: "cheap-fast" for validation, "smart-expensive" for generation

  tests:
    - scenario: "Duplicate request"
      expect: "Same result, no duplicate charge"

    - scenario: "Expired token"
      expect: status == paymentStatus.Declined
      expect_log: contains("token_expired")

  evidence:
    - @traces/checkout_production_sample.json
    - @screenshots/checkout_flow_golden.png
```

---

## Design Principles

1. **Intent over Implementation** — describe *what*, not *how*
2. **Verification as Compilation** — specs compile into proven, working systems
3. **Deterministic Sandwich** — probabilistic AI is always wrapped by deterministic validation and verification
4. **Uncertainty as a First-Class Concept** — model confidence, fallbacks, and human-in-the-loop
5. **Local-First** — spec authoring and validation work offline; cloud is only for generation
6. **Multimodal by Default** — screenshots, logs, traces, and schemas are native types
7. **Economics-Aware** — token budgets, model tiering, and cost constraints are part of the language
8. **Composable** — modules snap together like typed building blocks
9. **Human-Readable, Machine-Precise** — natural language inside structured scaffolding
10. **Runtime Enforceable** — specs double as live guardrails for production AI agents

---

## Status

🚧 **OmniLang is in the design phase.** This documentation describes the target architecture and serves as the specification for building the language toolchain.
