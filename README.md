<h1 align="center">⚡ OmniLang</h1>

<p align="center">
  <strong>The specification language for AI-native development.</strong><br>
  Write specs, not code. Let AI agents generate verified implementations.
</p>

<p align="center">
  <a href="https://github.com/denelvis/omnilang/actions"><img src="https://img.shields.io/github/actions/workflow/status/denelvis/omnilang/ci.yml?branch=main&label=CI&style=flat-square" alt="CI"></a>
  <a href="LICENSE-APACHE"><img src="https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue?style=flat-square" alt="License"></a>
  <img src="https://img.shields.io/badge/status-alpha-orange?style=flat-square" alt="Status">
</p>

---

## What is OmniLang?

OmniLang is a **hybrid, declarative specification language** designed for AI-native development. You describe *what* your system should do — the interfaces, schemas, and invariants — and let the compiler and AI agents generate the verified implementation.

By using a hybrid approach, developers can write strict signatures and type contracts to keep the AI on rails (preventing hallucinations) while expressing business rules in either natural language or formal math:

```omnilang
module acme.billing

type accountId = string

type account = struct
  id accountId
  balance money
  status string

service paymentService
  goal "Handle billing account deposits and charges safely"

  /// Deposits money into the account
  operation Deposit
    inputs:
      accountId accountId
      amount money
    outputs:
      result money
    preconditions:
      - "Deposit amount must be strictly greater than zero"
    postconditions:
      - "New balance must increase exactly by the deposit amount"

  /// Charges the account for a purchase
  operation Charge
    inputs:
      accountId accountId
      amount money
    outputs:
      success bool
    preconditions:
      - "Account balance must be greater than or equal to the charge amount"
      - "Account must be in Active status"
    postconditions:
      - "If the charge is successful, the balance is decreased by the charge amount"

  invariants:
    - balance_safety: "account.balance >= 0"
```

Then:

```bash
$ omni build
[1/5] Analyzing specs... ✓ (3 services, 12 types)
[2/5] Planning execution... ✓ (8 tasks, estimated cost: $0.15)
[3/5] Generating artifacts... ✓
[4/5] Verifying... ✓ (24 tests passed, all constraints met)
[5/5] Emitting artifacts... ✓

Build succeeded. Confidence: High. Cost: $0.12. Duration: 1m 42s.
```

**Output:** production-ready TypeScript (or Rust, Python, Go) with tests, types, and infrastructure manifests. The AI is only needed during the build — the output is pure, deterministic code.

## Key Ideas

| Concept | Description |
|---------|-------------|
| **Hybrid Intent** | Combine strict type contracts (to prevent AI hallucinations) with natural language or formal business rules. |
| **Separation of Concerns** | Keep your specification clean: business logic goes in `.omni`, compiler/LLM configurations go in `omni.toml`. |
| **Verification is Compilation** | AI generates code → unit/property tests run → constraints check → output is proven correct. |
| **Progressive Formalization** | Start with natural language constraints and add formal mathematical logic as the project matures. |
| **Budget-aware** | Token limits, cost caps, and model tiering — configured globally in `omni.toml`. |

## Installation

> ⚠️ OmniLang is in early development. The CLI supports `omni check`, `omni plan`, `omni build` (TypeScript, Rust, Python, Go targets), `omni init`, `omni fmt`, and `omni verify`. Registry commands (`publish`, `install`, `search`) currently operate against a local mock registry.

```bash
# From source (requires Rust 1.85+). Not on crates.io yet — the `omni-cli` crate
# name there belongs to an unrelated project, so do not `cargo install` it.
git clone https://github.com/mindhalla-open/omnilang.git
cd omnilang
cargo install --path crates/omni-cli --locked

# `omni build` additionally needs Node.js 18+ and, today, a checkout of this
# repository: it runs the generator from ./runtime. Pre-built binaries and
# package-manager installs are planned but not published yet.
```

## Quick Start

```bash
# Initialize a new project
omni init my-api

# Validate your specs
omni check

# See what would be generated (dry run)
omni plan

# Generate verified code
omni build
```

## Documentation

Full specification docs are in [`docs/`](./docs/):

| Document | Description |
|----------|-------------|
| [Vision & Philosophy](./docs/00-vision.md) | Why OmniLang exists |
| [Architecture](./docs/01-architecture.md) | Layered architecture, Thinking/Acting split |
| [Core Concepts](./docs/02-core-concepts.md) | Intent blocks, constraints, contracts, evidence |
| [Type System](./docs/03-type-system.md) | Refined types, generics, confidence types |
| [Syntax Reference](./docs/04-syntax-reference.md) | Complete grammar and syntax |
| [Compilation Model](./docs/05-compilation-model.md) | 5-phase build pipeline |
| [Testing & Verification](./docs/06-testing-verification.md) | Scenario, property, visual, chaos tests |
| [Module System](./docs/07-module-system.md) | Imports, exports, packages |
| [Agent Protocol](./docs/08-agent-protocol.md) | How AI agents interact with specs |
| [Tooling & IDE](./docs/09-tooling-ide.md) | LSP, VS Code, entropy analysis |
| [Examples](./docs/10-examples.md) | Real-world spec examples |
| [Roadmap](./docs/11-roadmap.md) | Development phases |
| [Glossary](./docs/12-glossary.md) | Key terms and abbreviations |

## Project Structure

```
crates/
├── omni-parser/     # Lexer + Parser → AST
├── omni-analyzer/   # Type checker, constraint resolver → Spec IR
└── omni-cli/        # CLI binary (`omni` command)
```

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for build instructions and contribution guidelines.

Most of the code here is written by AI agents, so the conventions are executable
rather than written down and hoped for: `./scripts/gates/run-all.sh` runs every
one of them locally, and CI runs the same command.

| Document | Description |
|----------|-------------|
| [Engineering Process](./docs/21-engineering-process.md) | Epic contracts, gates instead of rituals, what is measured |
| [Gates](./docs/gates.md) | Every machine check, what it replaces, and the known gaps |
| [Decision Records](./docs/adr/README.md) | Decisions with a long horizon |
| [CLAUDE.md](./CLAUDE.md) | Working agreements for agents and humans |

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
