# Versioning & Compatibility Policy

OmniLang carries three independent version numbers. Keeping them separate lets the
language, its intermediate representation, and the toolchain evolve at their own
pace without conflating unrelated breaking changes.

| Version | Where | Meaning |
|---------|-------|---------|
| `package.syntax_version` | `omni.toml` | The OmniLang grammar a project targets (currently `0.2`, brace-free). |
| `ir_version` | Spec IR root (`CURRENT_IR_VERSION`) | Schema version of the analyzer's Spec IR (currently `1`). The runtime rejects IR with an unknown version. |
| CLI / crate version | `Cargo.toml` (`0.11.0`) | The `omni` toolchain release. |

## Semantic-versioning rules

Once a component reaches `1.0`, it follows semver:

- **MAJOR** — a breaking change: source valid under the old `syntax_version` no
  longer parses/analyzes the same way; IR consumers must change; a CLI flag is
  removed or changes meaning.
- **MINOR** — a backward-compatible addition: new syntax that older specs don't
  use, a new optional IR field, a new CLI flag with a default.
- **PATCH** — a fix with no interface change.

### IR compatibility

- New IR fields are added as **optional/additive** within a major version; the
  runtime's `assertIrVersion` only hard-fails on a different `ir_version`, and the
  JSON Schema (`runtime/src/ir.schema.json`) gates structural drift in CI.
- A breaking IR change bumps `ir_version` and the runtime's `SUPPORTED_IR_VERSION`
  together.

### Syntax compatibility

- The **`omni-parser` crate is the normative source of truth** for syntax (see
  `docs/04-syntax-reference.md`); the conformance corpus pins accepted grammar.
- The legacy brace syntax remains accepted for backward compatibility but is not
  the documented style.

## Deprecation policy

A feature is deprecated for at least one MINOR release before removal:

1. The analyzer emits a warning (with a stable `E…` code, see
   `docs/18-error-codes.md`) when the deprecated form is used.
2. The migration is documented here and in the CHANGELOG.
3. Removal happens only on the next MAJOR bump.

## Status

The project is **alpha**. `syntax_version` and `ir_version` are pre-1.0 and may
still change. The path to a stable `1.0` (which freezes these guarantees) is the
remaining production-hardening work tracked in the roadmap (security scanning,
observability dashboard, large-scale stress testing).
