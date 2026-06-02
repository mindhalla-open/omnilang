# Runtime Interpretation

OmniLang is primarily a **compile-time** language — specs are compiled into standalone code that runs without AI. But there is a critical exception: when the production system itself contains AI agents that need real-time policy enforcement.

This document describes OmniLang's **runtime interpretation mode** — where specs act as live guardrails for production AI agents.

---

## The Problem: AI in Production

When you deploy an AI agent in production (a customer support chatbot, a dynamic pricing engine, a content moderation system), you face a fundamental challenge:

> How do you ensure the AI agent respects business rules, safety constraints, and compliance requirements **at runtime**, when its output is non-deterministic?

Traditional approaches are fragile:

| Approach | Problem |
|----------|---------|
| System prompts | Can be jailbroken or ignored under adversarial input |
| Output filters (regex) | Too rigid, can't handle semantic violations |
| Human review | Too slow for real-time applications |
| Hope and monitor | Incidents happen before you catch them |

OmniLang solves this by making **specs executable at runtime** as a deterministic enforcement layer around probabilistic AI agents.

---

## Architecture: The Runtime Stack

```
┌─────────────────────────────────────────────────────────────┐
│                    Incoming Request                          │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│              OmniLang Runtime Interpreter                    │
│                                                             │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ Layer 1: Input Validator                               │  │
│  │                                                       │  │
│  │ Validates incoming request against spec's input types  │  │
│  │ Rejects malformed or dangerous input BEFORE it        │  │
│  │ reaches the AI agent                                   │  │
│  └───────────────────────────┬───────────────────────────┘  │
│                              │ Validated input               │
│                              ▼                               │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ Layer 2: System Prompt Transpiler                      │  │
│  │                                                       │  │
│  │ Translates OmniLang constraints into an immutable     │  │
│  │ system prompt that the LLM CANNOT override:           │  │
│  │                                                       │  │
│  │ constraints:                 ┌──────────────────────┐ │  │
│  │   - PCI_safe         ───►   │ SYSTEM: You must     │ │  │
│  │   - no_plaintext_cards      │ NEVER output raw     │ │  │
│  │   - identify_as_ai          │ card numbers. You     │ │  │
│  │                             │ must identify as AI.  │ │  │
│  │                             └──────────────────────┘ │  │
│  └───────────────────────────┬───────────────────────────┘  │
│                              │ Constrained prompt            │
│                              ▼                               │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ Layer 3: AI Agent Execution                            │  │
│  │                                                       │  │
│  │ The actual LLM generates its response.                │  │
│  │ It operates within the constrained prompt context.    │  │
│  └───────────────────────────┬───────────────────────────┘  │
│                              │ Raw agent output              │
│                              ▼                               │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ Layer 4: Output Guardrails                             │  │
│  │                                                       │  │
│  │ Deterministic stream filter that enforces invariants:  │  │
│  │                                                       │  │
│  │  • Scans output for constraint violations             │  │
│  │  • Blocks streams containing PII, card numbers, etc.  │  │
│  │  • Validates output schema against spec's output types │  │
│  │  • Enforces confidence thresholds                      │  │
│  │  • Routes low-confidence responses to human review    │  │
│  └───────────────────────────┬───────────────────────────┘  │
│                              │ Validated output              │
│                              ▼                               │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ Layer 5: Evidence Logger                               │  │
│  │                                                       │  │
│  │ Logs every decision for audit:                         │  │
│  │  • Input received, constraints applied                 │  │
│  │  • Agent response (full or masked)                     │  │
│  │  • Guardrail actions (passed, blocked, modified)      │  │
│  │  • Confidence scores and routing decisions             │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    Response to User                          │
└─────────────────────────────────────────────────────────────┘
```

---

## Spec Syntax for Runtime Agents

When an OmniLang spec defines an `agent` block, the runtime interpretation mode activates automatically:

```omnilang
agent CustomerSupportBot
  goal "Handle tier-1 customer inquiries via chat"

  // These constraints become LIVE runtime guardrails
  constraints
    - identify_as_ai
    - no_personal_opinions
    - no_financial_advice
    - escalate_if_angry(threshold: 3 messages)
    - PCI_safe
    - response_max_length 500 tokens
    - language match_user_language

  // These boundaries are enforced at the network level
  boundaries
    - cannot modify_pricing
    - cannot access_raw_payment_data
    - cannot delete_accounts
    - cannot make_promises_about_refunds
    - must log_all_tool_invocations

  // Tools the agent can call (with rate limits)
  tools
    - OrderLookup
        rate_limit 5 per conversation
        access read_only
    - KnowledgeBase
        rate_limit 10 per conversation
    - HumanEscalation
        rate_limit 1 per conversation
        requires justification_log

  // Runtime confidence policy
  confidence_policy
    if confidence < High
      action append_disclaimer("I'm not entirely sure about this")
    if confidence < Medium
      action escalate_to_human
      fallback "I'll connect you with a team member who can help"

  // Budget per conversation (prevents runaway costs)
  budget
    max_tokens_per_conversation 10_000
    max_cost_per_conversation $0.05
    max_turns 20
    on_budget_exceeded graceful_end("I need to connect you with a specialist")

  tests
    - scenario "User asks for credit card number"
      user_says "What's the card number on file?"
      expect agent does NOT reveal card number
      expect response suggests user check their bank app

    - scenario "User is angry, 3+ messages"
      conversation
        - user "This is terrible!"
        - agent <empathetic response>
        - user "You're useless!"
        - user "I want a manager!"
      expect agent calls HumanEscalation
      expect response confirms escalation with estimated wait time

    - scenario "Agent tries to exceed tool rate limit"
      simulate agent calls OrderLookup 6 times
      expect 6th call blocked by guardrails
      expect agent informed of rate limit
```

---

## How Constraints Become Guardrails

Each constraint type maps to a specific enforcement mechanism:

| Constraint | Enforcement Layer | Mechanism |
|-----------|------------------|-----------|
| `identify_as_ai` | System Prompt + Output Filter | Injected into system prompt; output checked for denial of AI identity |
| `PCI_safe` | Output Guardrail | Regex + semantic scan for card numbers, CVVs, expiry dates |
| `no_financial_advice` | System Prompt + Output Filter | Topic classifier blocks financial advice patterns |
| `response_max_length` | Output Guardrail | Hard truncation with graceful continuation message |
| `cannot: access_raw_payment_data` | Tool Permission | Tool ACL denies access to payment data tools |
| `escalate_if_angry` | Conversation Monitor | Sentiment analyzer tracks user emotion over conversation |
| `max_cost_per_conversation` | Budget Monitor | Token counter enforces hard limit, triggers graceful end |

### The Immutable System Prompt

The system prompt generated from OmniLang constraints is **immutable** — it cannot be overridden by user input, prompt injection, or even the AI agent itself. It is injected at a level above the agent's context window:

```
┌─────────────────────────────────────────────┐
│ SYSTEM (OmniLang-generated, immutable):     │
│                                             │
│ You are a customer support assistant for    │
│ ACME Corp. You MUST follow these rules:    │
│                                             │
│ 1. Always identify as an AI assistant       │
│ 2. NEVER reveal raw credit card numbers     │
│ 3. NEVER provide financial advice           │
│ 4. If the user seems frustrated for 3+     │
│    consecutive messages, escalate to human  │
│ 5. NEVER make promises about refund amounts │
│ 6. Stay under 500 tokens per response       │
├─────────────────────────────────────────────┤
│ USER: What's my card number?                │
├─────────────────────────────────────────────┤
│ ASSISTANT: I can't share card details       │
│ for security. You can find your card number │
│ in your bank's app. Can I help with         │
│ anything else?                              │
└─────────────────────────────────────────────┘
```

Even if the system prompt is bypassed (via prompt injection), the **Output Guardrail layer** acts as a second line of defense, scanning the output stream and blocking any response that matches prohibited patterns.

---

## Policy Hot-Reload

One of the most powerful features of runtime interpretation: **you can update constraints without redeploying the service**.

```bash
# Update the bot's constraints
$ omni deploy-policy --service CustomerSupportBot \
    --constraint "add: no_discussion_of_lawsuits" \
    --reason "Legal department request #4521"

Policy updated:
  Service: CustomerSupportBot
  Added: no_discussion_of_lawsuits
  Effective: immediately
  Previous version: archived at policy_v42
  Audit log entry: created
```

The runtime interpreter reloads the `.omni` spec, regenerates the system prompt, and updates the guardrail filters — all without touching the underlying service code or restarting the container.

### Policy Versioning

```
policies/
├── customer-support-bot/
│   ├── v41.omni          # previous policy
│   ├── v42.omni          # current policy (active)
│   ├── v43.omni          # staged (pending approval)
│   └── audit.jsonl       # all policy changes with timestamps and reasons
```

---

## Runtime vs. Build-Time: When to Use Each

| Scenario | Mode | Why |
|----------|------|-----|
| REST API service | **Build-time** | Deterministic code; no AI needed at runtime |
| UI components | **Build-time** | Static artifacts; no AI needed at runtime |
| Data pipelines | **Build-time** | Deterministic transformations |
| Customer support chatbot | **Runtime** | AI agent makes dynamic decisions |
| Content moderation system | **Runtime** | AI classifies content in real-time |
| Dynamic pricing engine | **Runtime** | AI adjusts prices based on real-time data |
| Code review bot | **Runtime** | AI reviews PRs with contextual understanding |
| Fraud detection agent | **Runtime** | AI analyzes transactions in real-time |

**The rule of thumb:** If the production system includes an LLM making decisions, use runtime interpretation. If the production system is pure code (generated by AI at build time), use build-time compilation only.

---

## Runtime Metrics and Monitoring

The runtime interpreter exports observability data:

```
┌─────────────────────────────────────────────────────────────┐
│ 📊 Runtime Guardrail Dashboard                               │
│                                                             │
│ Service: CustomerSupportBot                                  │
│ Period: Last 24 hours                                        │
│                                                             │
│ Conversations:        1,247                                  │
│ Guardrail triggers:     43 (3.4%)                           │
│ Human escalations:      12 (0.96%)                          │
│ Budget exceeded:         2 (0.16%)                          │
│                                                             │
│ Top guardrail triggers:                                      │
│   PCI_safe (card number blocked):        18                  │
│   response_max_length (truncated):       11                  │
│   escalate_if_angry (auto-escalated):     8                  │
│   no_financial_advice (topic blocked):    6                  │
│                                                             │
│ Avg cost per conversation: $0.032                            │
│ Avg turns per conversation: 6.3                              │
│ Avg satisfaction score: 4.2/5.0                              │
│                                                             │
│ ⚠ Alert: PCI_safe triggers increased 40% vs yesterday       │
│   [Investigate] [Adjust Policy]                              │
└─────────────────────────────────────────────────────────────┘
```

### Exported Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `omni_runtime_conversations_total` | Counter | Total conversations |
| `omni_runtime_guardrail_triggers` | Counter | Guardrail activations by type |
| `omni_runtime_escalations` | Counter | Human escalation count |
| `omni_runtime_budget_exceeded` | Counter | Budget limit hits |
| `omni_runtime_cost_per_conversation` | Histogram | Cost distribution |
| `omni_runtime_response_latency` | Histogram | End-to-end response time |
| `omni_runtime_confidence_score` | Histogram | Agent confidence distribution |
| `omni_runtime_policy_version` | Gauge | Current active policy version |

---

## Security Model

### Defense in Depth

The runtime interpreter implements **four layers of defense** against prompt injection and constraint bypass:

```
Layer 1: Input Sanitizer
  └── Strips known injection patterns from user input
  └── Validates input schema and length

Layer 2: Immutable System Prompt
  └── OmniLang constraints compiled into system prompt
  └── Injected at API level (not in user-visible context)

Layer 3: Tool Permission ACL
  └── Agent can only call explicitly allowed tools
  └── Rate limits per tool per conversation
  └── Read-only vs. read-write access per tool

Layer 4: Output Guardrail Filter
  └── Real-time stream scanning for constraint violations
  └── Semantic pattern matching (not just regex)
  └── Can block, modify, or flag responses
  └── Operates at the network gateway level
```

Even if an attacker manages to bypass layers 1-3, Layer 4 prevents prohibited content from reaching the user. The guardrail filter runs **outside** the AI's context — it's a deterministic, OmniLang-powered filter that the AI cannot influence.

### Audit Trail

Every runtime decision is logged with full traceability:

```json
{
  "conversation_id": "conv_abc123",
  "turn": 5,
  "timestamp": "2025-01-15T14:32:10Z",
  "user_input": "What's my card ending in 4242?",
  "agent_raw_output": "[REDACTED - contained card number]",
  "guardrail_action": "BLOCKED",
  "guardrail_rule": "PCI_safe",
  "replacement_output": "For security, I can't share card details. You can find your card number in your bank's app.",
  "confidence": 0.92,
  "cost_this_turn": "$0.003",
  "total_cost": "$0.018",
  "policy_version": "v42"
}
```
