# ADR-0006: OMWF token-optimized wire format

- **Status.** **Superseded in practice — unresolved.** Needs a decision.
- **Date.** 2026-05-25 (contradicted 2026-05-30)
- **Deciders.** Project owner

## Context

Inter-agent messages carry the Spec IR into prompts. OMWF (OmniLang Minimized
Wire Format) was specified as a compact text encoding claimed to cut 40–50% of
tokens versus JSON.

## Decision (original)

Introduce OMWF as the wire format between the orchestrator and agents, in Phase 2.

## What actually happened

- `runtime/src/omwf/{grammar,serializer,index}.ts` exists but is **imported
  nowhere** outside itself — it is dead code.
- The CLI's `--wire-format` flag is marked `[DEPRECATED] … Ignored — always uses
  minified JSON` (`crates/omni-cli/src/main.rs`).
- `docs/01-architecture.md` and `docs/05-compilation-model.md` still describe
  OMWF as the active mechanism, so the documentation currently describes
  behaviour the code does not have.

## Consequences

Either OMWF gets wired in and the flag revived, or it gets deleted and the docs
corrected. Leaving it half-present is the worst of the three: it misleads
readers and it is exactly the kind of drift no test catches.

## Reversibility

Deleting it is cheap today (nothing depends on it) and gets more expensive the
longer the docs keep promising it.
