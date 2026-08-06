# Syntax Reference

This document provides the complete syntax specification for OmniLang. The syntax is designed to be simultaneously human-readable and machine-parseable, with minimal ambiguity.

> **Canonical style: brace-free (indentation-based).** Blocks are delimited by
> indentation (`service Foo` followed by an indented body), not by braces. The
> legacy brace form (`service Foo { ... }`) is still accepted by the parser for
> backward compatibility but is no longer the documented style.
>
> **Normative source of truth.** Where this document and the implementation
> disagree, the **`omni-parser` crate is authoritative**. The accepted grammar is
> pinned by conformance and snapshot tests over `examples/*.omni`
> (`crates/omni-parser/tests/conformance.rs`), which run on every `cargo test`.
> The `tree-sitter-omnilang` grammar is a secondary, editor-only artifact and may
> lag the parser; its resync to the brace-free syntax is tracked under the
> tooling/DX milestone.

---

## File Structure

An OmniLang file (`.omni`) has the following top-level structure:

```omnilang
// Module declaration (required, first statement)
module <namespace>.<name>

// Imports
use std.auth.*
use std.http.{Request, Response}
use ./shared/types.{Order, Customer}

// Version declaration (optional)
version: "1.0.0"

// Target language hint (optional)
target: rust | typescript | go | python | auto

// Top-level declarations
type ...
schema ...
contract ...
constraint ...
policy ...
service ...
component ...
pipeline ...
workflow ...
agent ...
```

---

## Comments

```omnilang
// Single-line comment

/* 
   Multi-line 
   comment 
*/

/// Documentation comment (attached to next declaration)
/// Supports **markdown** formatting
```

---

## Module Declaration

```omnilang
// Every file must start with a module declaration
module acme.payments.checkout
```

---

## Imports

```omnilang
// Import all exports from a module
use std.auth.*

// Import specific items
use std.http.{Request, Response, StatusCode}

// Import with alias
use std.money.Money as Currency

// Import from relative path
use ./shared/types.*
use ../common/policies.{SecurityPolicy}

// Import from registry
use registry://acme/shared-types@2.0.*
```

---

### Type Declarations

### Enum Types

```omnilang
type OrderStatus = enum
  Draft
  Pending
  Confirmed
  Shipped(tracking_id String)
  Delivered(delivered_at DateTime)
  Cancelled(reason CancelReason)
  Refunded(refund RefundDetails)
```

### Struct Types

```omnilang
type Customer = struct
  id CustomerId
  email Email
  name struct
    first String(min_length: 1, max_length: 100)
    last String(min_length: 1, max_length: 100)
  addresses List<Address>(min: 1)
  created_at DateTime
  tier CustomerTier = Standard  // default value
```

### Refined Types

```omnilang
type OrderId = String
  format regex("^ORD-[A-Z0-9]{12}$")
  example "ORD-A1B2C3D4E5F6"

type Price = Float64
  range [0.00, 999_999.99]
  precision 2
```

### Generic Types

```omnilang
type ApiResponse<T> = struct
  data Option<T>
  error Option<ApiError>
  metadata ResponseMetadata
  request_id UUID

type PaginatedList<T> = struct
  items List<T>
  pagination struct
    page Int(min: 1)
    per_page Int(range: [1, 100])
    total_items Int(min: 0)
    total_pages Int(min: 0)
```

### Type Aliases

```omnilang
type UserId = UUID
type TransactionId = ULID
type Cents = Int(min: 0)
```

---

## Service Blocks

The primary specification unit for backend services.

```omnilang
service <Name>
  // Required
  goal "<description>"

  // Optional sections (any order)
  constraints: [...]
  metrics: [...]
  inputs:
    field Type
  outputs:
    field Type
  preconditions: [...]
  postconditions: [...]
  invariants: [...]
  errors:
    ErrorName(field Type)
  budget:
    max_generation_cost $0.10
  tests: [...]
  evidence: [...]
  depends_on: [...]

  // Nested operations
  operation <MethodName>
    inputs:
      field Type
    outputs:
      field Type
```

### Full Service Example

```omnilang
service InventoryManager
  goal "Manage product inventory with real-time stock tracking"

  constraints:
    - eventual_consistency(max_lag: 5s)
    - idempotent
    - audit_logging
    - max_batch_size: 1000

  depends_on:
    - ProductCatalog
    - WarehouseService

  operation CheckStock
    inputs:
      product_id ProductId
      warehouse Option<WarehouseId>

    outputs:
      available Int(min: 0)
      reserved Int(min: 0)
      incoming List<IncomingShipment>

    constraints:
      - latency(p99: <50ms)
      - cacheable(ttl: 10s)

    tests:
      - scenario: "Product in stock"
        given: product_exists(id: "P001", stock: 42)
        expect: available == 42

      - scenario: "Unknown product"
        given: product_not_exists(id: "P999")
        expect_error: ProductNotFound

  operation ReserveStock
    inputs:
      product_id ProductId
      quantity Int(min: 1)
      reservation_ttl Duration = 15min

    outputs:
      reservation_id ReservationId
      expires_at DateTime

    preconditions:
      - available_stock(product_id) >= quantity

    postconditions:
      - available_stock(product_id) == old(available_stock(product_id)) - quantity
      - reserved_stock(product_id) == old(reserved_stock(product_id)) + quantity

    errors:
      - InsufficientStock(available: Int, requested: Int)
      - ProductNotFound(product_id ProductId)

    tests:
      - scenario: "Successful reservation"
        given: product_exists(id: "P001", stock: 10)
        when: reserve(product_id: "P001", quantity: 3)
        expect: reservation_id != null
        expect: available_stock("P001") == 7

      - property: "Stock conservation"
        forall: p <- arbitrary<ProductId>, q <- Int(1, 100)
        given: available_stock(p) >= q
        when: reserve(product_id: p, quantity: q)
        assert: total_stock(p) == old(total_stock(p))
```

---

## Component Blocks

For UI components and frontend specifications.

```omnilang
component <Name>
  goal "<description>"

  // Component interface
  props ...
  state ...
  events ...
  slots ...

  // Constraints and styling
  constraints [...]
  style_guide <reference>

  // Visual specifications
  visual_spec [...]

  // Tests
  tests [...]
```

### Component Example

```omnilang
component ProductCard
  goal "Display a product with image, name, price, and add-to-cart action"

  props:
    product: Product
    currency: CurrencyCode = USD
    on_add_to_cart: Callback<(ProductId, Quantity) -> void>
    variant enum
      compact
      detailed
      featured
    = detailed

  state:
    quantity: Int(range: [1, 99]) = 1
    is_loading: Bool = false
    image_loaded: Bool = false

  events:
    - add_to_cart(product_id: ProductId, quantity: Quantity)
    - quantity_changed(new_quantity: Quantity)

  constraints:
    - accessible(WCAG: "AA")
    - responsive(breakpoints: [320, 768, 1024, 1440])
    - max_bundle_size: 15KB gzipped
    - no_layout_shift(CLS: < 0.1)
    - interactive_within: 100ms

  visual_spec:
    - @golden/product_card_desktop.png    --viewport 1440x900  --variant detailed
    - @golden/product_card_mobile.png     --viewport 375x812   --variant compact
    - @golden/product_card_featured.png   --viewport 1440x900  --variant featured

  tests
    - scenario "Renders product info"
      given product == mock_product(name: "Wireless Mouse", price: 29.99)
      expect_visual shows product.name, formatted_price("$29.99"), product.image

    - scenario "Add to cart interaction"
      given product == mock_product()
      when click(button: "Add to Cart")
      expect event add_to_cart emitted with (product.id, quantity)
      expect is_loading == true briefly

    - scenario "Quantity bounds"
      when set_quantity(100)
      expect quantity == 99  // clamped to max

    - scenario "Image lazy loading"
      given product == mock_product()
      expect image has loading="lazy"
      expect placeholder shown until image_loaded == true
```

---

## Pipeline Blocks

For data transformations and ETL processes.

```omnilang
pipeline <Name>
  goal "<description>"

  source ...
  stages [...]
  sink ...

  constraints [...]
  schedule <cron expression>
  tests [...]
```

### Pipeline Example

```omnilang
pipeline DailyRevenueReport
  goal "Aggregate daily revenue by product category and region"

  source
    type PostgreSQL
    table orders
    filter "created_at >= today() - 1day"

  stages
    - name "Filter completed orders"
      filter status == OrderStatus.Completed

    - name "Enrich with product data"
      join products on orders.product_id == products.id

    - name "Aggregate revenue"
      group_by [products.category, orders.region]
      aggregate
        total_revenue sum(orders.total)
        order_count count(*)
        avg_order_value avg(orders.total)

    - name "Format report"
      transform RevenueReportRow

  sink
    type S3
    path "s3://reports/revenue/{date}/report.parquet"
    format parquet

  constraints
    - data_freshness < 2h
    - idempotent
    - exactly_once_processing

  schedule "0 6 * * *"  // daily at 6 AM

  tests
    - scenario "Normal day"
      given orders(count: 1000, date: "2025-01-15")
      expect output_rows > 0
      expect sum(output.total_revenue) == sum(source.total where status == Completed)

    - scenario "No orders"
      given orders(count: 0, date: "2025-01-16")
      expect output_rows == 0
      expect no_error
```

---

## Workflow Blocks

For multi-step business processes with state machines.

```omnilang
orchestrator <Name>
  goal "<description>"

  states: [...]
  transitions:
    - FromState -> ToState [on: TriggerName]
```

### Workflow Example

```omnilang
orchestrator OrderFulfillment
  goal "Manage order lifecycle from placement to delivery"

  states: [Placed, PaymentPending, Paid, Picking, Packed, Shipped, Delivered, Cancelled, Refunded]

  transitions:
    - Placed -> PaymentPending
        trigger: order_placed
        action: initiate_payment

    - PaymentPending -> Paid
        trigger: payment_confirmed
        timeout: 30min -> Cancelled
        action: notify_warehouse

    - PaymentPending -> Cancelled
        trigger: payment_failed
        action: release_inventory, notify_customer

    - Paid -> Picking
        trigger: warehouse_acknowledged
        action: create_pick_list

    - Picking -> Packed
        trigger: items_picked
        action: generate_shipping_label

    - Packed -> Shipped
        trigger: carrier_pickup
        action: send_tracking_info

    - Shipped -> Delivered
        trigger: delivery_confirmed
        action: request_review, complete_order

    - * -> Cancelled
        trigger: customer_cancellation
        guard state not in [Shipped, Delivered]
        action: refund_if_paid, release_inventory

  constraints:
    - max_duration: 14days (Placed -> Delivered)
    - audit_all_transitions
    - no_backward_transitions except via Cancelled/Refunded

  tests:
    - scenario: "Happy path"
      trace: Placed -> PaymentPending -> Paid -> Picking -> Packed -> Shipped -> Delivered
      expect: total_duration < 7days

    - scenario: "Payment timeout"
      given: state == PaymentPending
      when: elapsed(30min) without payment_confirmed
      expect: state == Cancelled
      expect: inventory_released
```

---

## Agent Blocks

Define AI agent capabilities and boundaries.

```omnilang
agent <Name>
  goal "<description>"

  capabilities [...]
  boundaries [...]
  tools [...]
  
  model ...
  budget ...

  tests [...]
```

### Agent Example

```omnilang
agent CustomerSupportAgent
  goal "Handle tier-1 customer support inquiries via chat"

  capabilities
    - answer_faq
    - lookup_order_status
    - initiate_return
    - escalate_to_human

  boundaries
    - cannot modify_pricing
    - cannot access_payment_details
    - cannot delete_accounts
    - must identify_as_ai
    - must escalate_if_sentiment(negative, duration: > 3 messages)

  tools
    - OrderLookup(input: OrderId, output: OrderSummary)
    - KnowledgeBase(input: Query, output: List<Article>)
    - ReturnInitiation(input: ReturnRequest, output: ReturnConfirmation)
    - HumanEscalation(input: ConversationContext, output: TicketId)

  model
    preference Balanced
    temperature 0.3
    max_response_tokens 500

  budget
    max_cost_per_conversation $0.05
    max_turns 20

  tests
    - scenario "Order status inquiry"
      user_says "Where is my order ORD-ABC123?"
      expect agent calls OrderLookup(ORD-ABC123)
      expect response contains order status and tracking info

    - scenario "Out of scope request"
      user_says "Give me a 50% discount"
      expect agent does NOT call any pricing tool
      expect response politely declines and offers alternatives

    - scenario "Angry customer escalation"
      conversation
        - user "This is terrible service!"
        - agent <empathetic response>
        - user "I want to speak to a manager NOW!"
        - user "You're useless!"
      expect agent calls HumanEscalation
      expect response confirms escalation
```

---

## Schema Blocks

Define data models and database schemas.

```omnilang
schema <Name>
  goal "<description>"
  target postgresql | mysql | mongodb | dynamodb | auto

  entities:
    EntityName
      field Type
```

### Schema Example

```omnilang
schema ECommerceDB
  goal "E-commerce data model with products, orders, and customers"
  target postgresql

  entity Product
    id ProductId @primary
    name String(max_length: 200) @indexed
    description String(max_length: 5000)
    price Money
    category CategoryId @foreign(Category.id)
    status ProductStatus = Active
    created_at DateTime @default(now())
    updated_at DateTime @updated_at

  entity Order
    id OrderId @primary
    customer CustomerId @foreign(Customer.id)
    items List<OrderItem> @embedded
    total Money
    status OrderStatus = Placed
    placed_at DateTime @default(now())
    completed_at Option<DateTime>

  entity OrderItem
    product ProductId @foreign(Product.id)
    quantity Int(min: 1)
    unit_price Money  // snapshot at time of order
    subtotal Money

  relations:
    - Customer has_many Orders
    - Order has_many OrderItems
    - Product belongs_to Category
    - Category has_many Products (tree: nested_set)

  indexes:
    - Order(customer, placed_at) // lookup orders by customer
    - Product(category, status)  // catalog browsing
    - Order(status) where status != "Completed"  // partial index

  constraints:
    - soft_delete(field: deleted_at)
    - row_level_security(tenant_field: org_id)
    - encryption_at_rest(fields: [Customer.email, Customer.phone])
```

---

## Policy Blocks

Define organization-wide rules and governance.

```omnilang
policy <Name>
  description "<purpose>"
  scope global | module | service

  rules: [...]
```

### Policy Example

```omnilang
policy SecurityBaseline
  description "Minimum security requirements for all services"
  scope global

  rules:
    - all services must:
        - use TLS 1.3+ for external communication
        - validate all inputs
        - sanitize all outputs
        - log authentication events
        - implement rate limiting

    - all endpoints with PII must:
        - encrypt at rest (AES-256)
        - mask in logs
        - comply with GDPR data retention

    - all agents must:
        - operate with least-privilege permissions
        - log all tool invocations
        - respect budget constraints

    - if service handles payments:
        - apply PCI_DSS_v4
        - require quarterly_security_review
        - evidence penetration_test_report(max_age: 90days)
```

---

## Budget Blocks

```omnilang
budget
  cost:
    max_total $1.00
    per_service $0.25
    per_retry $0.05
    alert_at 80%

  tokens:
    max_total 200_000
    model_strategy:
      validation CheapFast
      generation Balanced
      security_review SmartExpensive

  time:
    max_generation 10min
    max_verification 5min
    on_timeout EscalateToHuman
```

---

## Evidence Blocks

```omnilang
evidence
  // Human-provided reference materials
  references:
    - @docs/architecture_diagram.png
        type diagram
    - @docs/api_contract_v2.yaml
        type api_spec
    - @traces/production_sample.json
        type trace

  // Required agent-produced evidence
  required:
    - test_results:
        format junit_xml
        expect all_pass

    - coverage:
        format lcov
        expect line >= 85%, branch >= 75%

    - security_scan:
        format sarif
        expect no_critical, no_high

    - performance:
        format json
        expect matches_constraints
```

---

## Metrics Blocks

Used inside services to specify application telemetry/metrics that should be automatically instrumented by code generators.

```omnilang
metrics:
  - counter payment_attempts_total
      description "Total payment attempts"
      labels [payment_method, status]

  - histogram checkout_value_usd
      description "Distribution of transaction amounts"
      buckets [10, 50, 100, 500, 1000]

  - gauge active_connections
      description "Current active checkout connections"
```

---

## Reserved Keywords

```
module, use, type, struct, enum, service, component, pipeline,
workflow, agent, schema, policy, contract, constraint, budget,
evidence, metrics, counter, gauge, histogram, tests, factory,
define, goal, inputs, outputs, preconditions, postconditions,
invariants, errors, depends_on, rpc, props, state, events, slots,
source, stages, sink, states, transitions, triggers, rules,
schedule, version, target, as, in, not, and, or, if, else, forall,
assert, expect, given, when, scenario, property, visual, benchmark,
chaos, security, option, result, true, false, null, none, some,
ok, err, old, self
```

---

## Operator Reference

| Operator | Meaning | Example |
|----------|---------|---------|
| `==` | Equality | `status == Active` |
| `!=` | Inequality | `from != to` |
| `<`, `>`, `<=`, `>=` | Comparison | `latency < 200ms` |
| `&&` | Logical AND | `active && verified` |
| `\|\|` | Logical OR | `admin \|\| owner` |
| `!` | Logical NOT | `!deleted` |
| `in` | Membership | `status in [Active, Pending]` |
| `not in` | Non-membership | `role not in [Guest]` |
| `..` | Range (inclusive) | `1..100` |
| `..<` | Range (exclusive end) | `0..<items.length` |
| `@` | Asset reference | `@golden/screenshot.png` |
| `->` | Transition / mapping | `Placed -> Paid` |
| `\|` | Union type | `String \| Int` |
| `&` | Intersection type | `Logged & Authenticated` |
| `?` | Optional shorthand | `String?` ≡ `Option<String>` |
