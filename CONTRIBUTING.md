# Contributing to OmniLang

Thank you for your interest in contributing to OmniLang! This document provides guidelines and instructions for contributing.

## Prerequisites

- [Rust](https://rustup.rs/) (1.85+)
- Git
- For the runtime and its test suite: Node.js 20+, plus the target toolchains the
  integration suite builds against — Go 1.22+, Python 3 with `pytest`, and Z3 on
  `PATH` for the formal-verification tests. A missing tool fails the suite with a
  message naming it; CI installs the same set in `.github/workflows/ci.yml`.

## Getting Started

```bash
# Clone the repository
git clone https://github.com/denelvis/omnilang.git
cd omnilang

# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Run the CLI
cargo run -- --help
```

## Code Style

We enforce consistent code style via CI:

```bash
# Format code
cargo fmt --all

# Lint
cargo clippy -- -D warnings
```

All PRs must pass `cargo fmt --check` and `cargo clippy` before merge.

## Project Structure

```
crates/
├── omni-parser/     # Lexer + Parser → AST
├── omni-analyzer/   # Type checker + Constraint resolver → Spec IR
└── omni-cli/        # CLI binary (`omni` command)
```

## Making Changes

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/my-feature`
3. Make your changes, as a vertical slice — spec → analyzer → runtime → all four
   targets → docs → example. A change that only lands on one target is unfinished.
4. Add tests for new functionality
5. Commit with [conventional commits](https://www.conventionalcommits.org/):
   - `feat: add enum type parsing`
   - `fix: handle unterminated strings in lexer`
   - `docs: update syntax reference`
   - `test: add snapshot tests for service blocks`
6. Open a Pull Request

## Before you push

Two commands. Everything CI enforces is reproducible locally — if these are
green, the pipeline should be too.

```bash
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
./scripts/gates/run-all.sh
```

`run-all.sh` checks the conventions no compiler knows about: canonical brace-free
syntax, every example analyzable and formatted, diagnostic codes documented,
build-cost budgets, changelog, credentials, workflow validity. Each gate and the
reason it exists are listed in [`docs/gates.md`](./docs/gates.md).

**A red gate means the work is not done.** Fix the cause; never weaken a gate to
get a green build.

## Pull Requests

The PR template does not ask you to confirm formatting, lints, tests, coverage
or the changelog — those are gates, and a checkbox that duplicates a gate only
teaches people to tick without reading. It asks for the things a machine cannot
judge: whether the change stayed inside its boundaries, which invariants it
touches, and what is irreversible about it.

**Irreversible changes need an ADR first.** Syntax and grammar, the Spec IR,
cache and lock formats, removing or renumbering a diagnostic code, removing a
CLI flag, anything published — write `docs/adr/NNNN-title.md` before the
implementation. See [`docs/adr/`](./docs/adr/README.md).

The full process, and where it breaks, is in
[`docs/21-engineering-process.md`](./docs/21-engineering-process.md). Working
agreements for both humans and AI agents are in [`CLAUDE.md`](./CLAUDE.md).

## Reporting Issues

Use [GitHub Issues](https://github.com/denelvis/omnilang/issues) with the appropriate template:

- **Bug Report** — something doesn't work as expected
- **Feature Request** — a new capability you'd like
- **RFS (Request for Spec)** — a proposal to change the OmniLang specification

## License

By contributing, you agree that your contributions will be licensed under Apache-2.0 OR MIT (dual license).
