# Core Concepts

OmniLang is built around six fundamental concepts that together form a complete specification framework for AI-native development.

---

## 1. Intent Blocks

An **intent block** declares *what* the system should do without specifying *how*. It is the primary unit of specification in OmniLang.

### Syntax

```omnilang
service <Name>
  goal "<natural language description of purpose>"

  // ... constraints, contracts, tests, evidence, budget
```

### Design Philosophy

The `goal` field accepts natural language — but it is not a freeform prompt. It exists within the structured scaffolding of the intent block, which gives the AI agent precise boundaries:

- The **name** provides semantic anchoring
- The **constraints** limit the solution space
- The **contracts** define exact inputs/outputs
- The **tests** define acceptance criteria

The natural language in `goal` fills the gap between these structural elements — it provides the *why* and the *spirit* of the component.

### Intent Block Types

| Block Type | Keyword | Purpose |
|-----------|---------|---------|
| Service | `service` | A backend service, API endpoint, or microservice |
| Component | `component` | A UI component or reusable frontend element |
| Pipeline | `pipeline` | A data transformation or ETL pipeline |
| Workflow | `workflow` | A multi-step business process with state transitions |
| Agent | `agent` | An AI agent definition with capabilities and boundaries |
| Schema | `schema` | A data model or database schema |
| Policy | `policy` | A system-wide rule or governance constraint |

### Example: Service Intent

```omnilang
service UserAuthentication
  goal "Authenticate users via email/password or OAuth2 providers"

  constraints:
    - bcrypt_password_hashing(rounds: 12)
    - session_ttl: 24h
    - max_login_attempts: 5 per 15min
    - OWASP_compliance

  inputs:
    credentials EmailPassword | OAuthToken

  outputs:
    session AuthSession
    user UserProfile

  tests:
    - scenario: "Valid email/password login"
      given: registered_user(email: "test@example.com")
      when: login(email: "test@example.com", password: "correct")
      expect: session.is_valid == true

    - scenario: "Brute force protection"
      given: registered_user(email: "victim@example.com")
      when: login_attempts(email: "victim@example.com", count: 6, window: 10min)
      expect: status == RateLimited
```

### Example: Component Intent

```omnilang
component ShoppingCart
  goal "Display cart items with real-time total, quantity editing, and checkout CTA"

  constraints:
    - accessible(WCAG: "AA")
    - responsive(breakpoints: [mobile, tablet, desktop])
    - max_render_time: 16ms

  props:
    items List<CartItem>
    currency CurrencyCode
    on_checkout Callback

  state:
    quantities Map<ItemId, Quantity>
    promo_code Option<String>

  visual_spec:
    - @assets/cart_desktop_golden.png  --viewport 1440x900
    - @assets/cart_mobile_golden.png   --viewport 375x812

  tests:
    - scenario: "Empty cart"
      given: items == []
      expect_visual: shows "Your cart is empty" message with CTA to shop

    - scenario: "Quantity update"
      given: items == [mock_item(qty: 2)]
      when: set_quantity(item_id: mock_item.id, qty: 3)
      expect: total == mock_item.price * 3
```

---

## 2. Constraints

**Constraints** define the boundaries and non-functional requirements that the generated implementation must satisfy. They are machine-verifiable rules, not suggestions.

### Constraint Categories

```omnilang
constraints:
  // Performance
  - latency(p95: <200ms, p99: <500ms)
  - throughput(min: 10_000 rps)
  - memory(max: 512MB)

  // Security
  - PCI_DSS_compliant
  - no_plaintext_secrets
  - input_sanitization(strategy: "strict")
  - OWASP_top_10

  // Reliability
  - idempotent
  - retry_safe
  - circuit_breaker(threshold: 5, timeout: 30s)

  // Compliance
  - GDPR_compliant
  - data_residency(regions: [EU])
  - audit_logging

  // Style & Quality
  - code_style: "google"
  - test_coverage(min: 90%)
  - no_deprecated_apis

  // Economic
  - max_generation_cost: $0.25
  - prefer_oss_dependencies
```

### Built-in Constraint Library

OmniLang ships with a standard library of well-known constraints:

| Constraint | Category | Verification Method |
|-----------|----------|-------------------|
| `idempotent` | Reliability | Property-based test: f(f(x)) == f(x) |
| `PCI_DSS_compliant` | Security | Static analysis + checklist |
| `WCAG_AA` | Accessibility | Automated accessibility scan |
| `GDPR_compliant` | Compliance | Data flow analysis + checklist |
| `no_plaintext_secrets` | Security | Secret scanning |
| `latency(p95: <Xms)` | Performance | Benchmark harness |

### Custom Constraints

```omnilang
define constraint no_external_calls
  description "Implementation must not make any outbound HTTP calls"
  verify static_analysis(rule: "no_http_client_imports")
  severity critical
```

---

## 3. Contracts

**Contracts** define the formal interface of a component — inputs, outputs, preconditions, postconditions, and invariants. They are inspired by Design by Contract (DbC) and serve as machine-checkable specifications.

### Syntax

```omnilang
contract TransferMoney
  inputs:
    from_account AccountId
    to_account AccountId
    amount Money(positive: true)
    currency CurrencyCode

  outputs:
    transaction_id TransactionId
    new_balance_from Money
    new_balance_to Money

  preconditions:
    - from_account != to_account
    - balance(from_account) >= amount
    - account_status(from_account) == Active
    - account_status(to_account) == Active

  postconditions:
    - balance(from_account) == old(balance(from_account)) - amount
    - balance(to_account) == old(balance(to_account)) + amount
    - sum_all_balances() == old(sum_all_balances())  // conservation

  invariants:
    - balance(any_account) >= 0  // no negative balances
    - transaction_log.is_append_only

  errors:
    - InsufficientFunds(available Money, requested Money)
    - AccountFrozen(account AccountId, reason String)
    - DailyLimitExceeded(limit Money, attempted Money)
```

### Pre/Postconditions

Preconditions and postconditions are **executable assertions** that the verification engine checks:

- **Preconditions** — must be true before execution; define valid input states
- **Postconditions** — must be true after execution; define expected output states
- **Invariants** — must be true at all times; define system-wide consistency rules

The `old()` function references a value's state before the operation, enabling delta-based assertions.

---

## 4. Tests as First-Class Citizens

In OmniLang, tests are not an afterthought — they are part of the specification. Every intent block should contain tests. Tests serve as BDD-style or property-based checks that prove correctness.

### Test Types

```omnilang
tests:
  // Scenario-based (BDD-style)
  - scenario: "Happy path checkout"
    given: cart_with_items(count: 3)
    when: checkout(payment: valid_card)
    expect: order_status == Confirmed
    expect_side_effect: email_sent_to(user.email)

  // Property-based (QuickCheck-style)
  - property: "Serialization roundtrip"
    forall: order <- arbitrary<Order>
    assert: deserialize(serialize(order)) == order

  // Snapshot / Visual
  - visual: "Cart renders correctly on mobile"
    viewport: 375x812
    state: cart_with_items(count: 2)
    expect_visual: @golden/cart_mobile.png --tolerance 1.5%

  // Performance
  - benchmark: "Checkout latency"
    load: 1000 concurrent_users
    duration: 60s
    expect: p95_latency < 200ms
    expect: error_rate < 0.1%

  // Chaos / Edge Case
  - chaos: "Database failover"
    inject: database_disconnect(duration: 5s)
    expect: service_degrades_gracefully
    expect: no_data_loss

  // Security
  - security: "SQL injection resistance"
    input: "'; DROP TABLE users; --"
    expect: input_rejected
    expect: no_query_executed
```

### Test Data Factories

```omnilang
factory CartItem
  defaults:
    id uuid()
    name fake.product_name()
    price Money(random(1.00, 999.99), USD)
    quantity random_int(1, 10)

  variants:
    expensive:
      price Money(5000.00, USD)
    free:
      price Money(0.00, USD)
    digital:
      requires_shipping false
```

---

## 5. Evidence

**Evidence** is a first-class concept in OmniLang that prevents the AI agent from "trusting itself." Every claim the agent makes about the generated implementation must be backed by verifiable evidence.

### Evidence Types

| Type | Format | Purpose |
|------|--------|---------|
| Test results | JUnit XML / TAP | Proof that tests pass |
| Coverage report | LCOV / Cobertura | Proof of test coverage |
| Benchmark results | JSON / CSV | Proof of performance characteristics |
| Security scan | SARIF | Proof of security analysis |
| Visual diff | PNG + diff overlay | Proof of visual correctness |
| Trace log | OpenTelemetry | Proof of runtime behavior |
| Cost report | JSON | Proof of resource consumption |
| Dependency audit | JSON | Proof of dependency safety |

### Attaching Evidence to Specs

```omnilang
service PaymentProcessor
  goal "Process credit card payments"

  evidence:
    // Reference evidence: human-provided artifacts that inform generation
    - reference: @docs/payment_flow_diagram.png
        type architecture_diagram
        description "Approved payment flow from tech design review"

    // Expected evidence: artifacts the agent MUST produce during generation
    - required: test_results
        format junit_xml
        expect all_pass

    - required: security_scan
        format sarif
        expect no_critical, no_high

    - required: benchmark
        format json
        expect matches(constraints.latency)

    - required: coverage
        format lcov
        expect line_coverage >= 90%
```

### Evidence Chain

The verification engine maintains an **evidence chain** — a complete, auditable record of how each constraint was verified:

```
Constraint: "latency(p95: <200ms)"
  └── Evidence: benchmark_results.json
       ├── Generated at: 2025-01-15T10:30:00Z
       ├── Agent: codegen-v3
       ├── Method: k6 load test, 1000 VUs, 60s
       ├── Result: p95 = 142ms ✓
       └── Reproducible: true (seed: 0xDEADBEEF)
```

---

## 6. Trust Types

**Trust types** model the inherent uncertainty of AI-generated artifacts. They are OmniLang's answer to the question: "How confident is the agent in this output?"

### Confidence Levels

```omnilang
type Confidence = enum
  Proven       // Formally verified or exhaustively tested
  High         // All tests pass, all constraints met, benchmarked
  Medium       // Tests pass but edge cases may be underexplored
  Low          // Basic functionality works, needs human review
  Speculative  // Best-effort generation, not verified
```

### Trust Policies

```omnilang
policy ProductionReadiness
  description "Rules for what can be deployed to production"

  rules:
    - if confidence < High:
        action block_deployment
        notify tech_lead

    - if confidence == Medium:
        action allow_staging_only
        require human_review(within: 24h)

    - if confidence == Low:
        action sandbox_only
        require pair_review(with: senior_engineer)
        flag "AI-generated, not production-ready"

    - if confidence == Speculative:
        action reject
        message "Insufficient confidence for any environment"
```

### Confidence Propagation

Confidence propagates through the dependency graph. If service A depends on service B, the confidence of A cannot exceed the confidence of B:

```
confidence(A) <= min(confidence(A_own), confidence(B), confidence(C))
```

This ensures that a system's overall trust level accurately reflects its weakest link.

---

## How Concepts Interact

```
┌─────────────────────────────────────────────┐
│              Intent Block                    │
│  ┌─────────┐  ┌────────────┐  ┌──────────┐ │
│  │  Goal    │  │ Constraints│  │ Contracts│ │
│  │ (what)   │  │ (limits)   │  │ (shape)  │ │
│  └────┬─────┘  └─────┬──────┘  └────┬─────┘ │
│       │              │              │        │
│       ▼              ▼              ▼        │
│  ┌─────────────────────────────────────────┐ │
│  │              Tests                       │ │
│  │  (prove the goal is met within limits    │ │
│  │   and the shape is correct)              │ │
│  └──────────────────┬──────────────────────┘ │
│                     │                        │
│                     ▼                        │
│  ┌─────────────────────────────────────────┐ │
│  │            Evidence                      │ │
│  │  (proof that tests passed and           │ │
│  │   constraints were verified)            │ │
│  └──────────────────┬──────────────────────┘ │
│                     │                        │
│                     ▼                        │
│  ┌─────────────────────────────────────────┐ │
│  │          Trust Level                     │ │
│  │  (computed from evidence strength)      │ │
│  └─────────────────────────────────────────┘ │
└─────────────────────────────────────────────┘
```

Each concept reinforces the others:
- **Goals** explain what success looks like
- **Constraints** narrow the solution space
- **Contracts** formalize the interface
- **Tests** prove correctness
- **Evidence** documents proof
- **Trust** quantifies confidence in the whole
