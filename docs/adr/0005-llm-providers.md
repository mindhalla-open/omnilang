# ADR-0005: Anthropic as the primary provider, local providers supported

- **Status.** Accepted, extended
- **Date.** 2026-05-25 (extended 2026-05-29)
- **Deciders.** Project owner

## Context

Build-time generation needs a strong hosted model; development, CI and
offline/air-gapped use need something that costs nothing and leaks nothing.

## Decision

Anthropic Claude is the primary provider. Ollama is supported for local and
offline runs. `OMNI_MOCK_LLM=true` gives CI a deterministic, zero-cost path.

## Extension (2026-05-29, commit 7698f56)

`runtime/src/providers/llamacpp.ts` added llama.cpp as a third provider.

## Consequences

The provider interface is a real abstraction and must stay one. CI's default
path never spends money; the real-LLM E2E is opt-in on a secret being present
and announces itself when skipped.

## Reversibility

Reversible per provider — the interface isolates them.
