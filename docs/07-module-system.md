# Module System

OmniLang's module system enables composability, reuse, and organization of specifications at scale. It is designed to support both small projects and large enterprise systems with hundreds of services.

---

## Module Basics

Every `.omni` file belongs to a module, declared at the top of the file:

```omnilang
module acme.payments.checkout
```

The module path follows a hierarchical namespace convention:

```
<organization>.<domain>.<component>
```

### File-to-Module Mapping

By default, file paths map to module paths:

```
specs/
├── acme/
│   ├── payments/
│   │   ├── checkout.omni     → module acme.payments.checkout
│   │   ├── refund.omni       → module acme.payments.refund
│   │   └── types.omni        → module acme.payments.types
│   ├── catalog/
│   │   ├── products.omni     → module acme.catalog.products
│   │   └── search.omni       → module acme.catalog.search
│   └── shipping/
│       ├── fulfillment.omni  → module acme.shipping.fulfillment
│       └── tracking.omni     → module acme.shipping.tracking
```

---

## Imports and Exports

### Exporting

By default, all top-level declarations in a module are **public**. Use `internal` to restrict visibility:

```omnilang
module acme.payments.checkout

// Public (importable by other modules)
type PaymentStatus = enum
  Pending
  Completed
  Failed

service Checkout
  goal "Process checkout"

// Internal (visible only within this module)
internal type InternalState = enum
  Draft
  Processing

internal constraint legacy_compat
  goal "legacy compatibility"
```

### Importing

```omnilang
// Import everything from a module
use acme.payments.types.*

// Import specific items
use acme.payments.types.{PaymentStatus, Money, Currency}

// Import with alias (resolve name conflicts)
use acme.payments.types.Order as PaymentOrder
use acme.catalog.types.Order as CatalogOrder

// Import from relative path
use ./types.*
use ../shared/common.{ErrorResponse}

// Import from standard library
use std.auth.{Session, Permission}
use std.http.*

// Import from registry
use registry://acme/shared-types@^2.0.0
```

### Re-exports

A module can re-export items from its dependencies:

```omnilang
module acme.payments

// Re-export for convenience
export use ./checkout.{Checkout, PaymentStatus}
export use ./refund.{RefundService}
export use ./types.*

// Now consumers can do:
// use acme.payments.{Checkout, PaymentStatus, RefundService}
```

---

## Module Manifests

Each project has a root manifest file (`omni.toml`) that configures the module system:

```toml
[project]
name = "acme-ecommerce"
version = "1.0.0"
organization = "acme"

[defaults]
target = "typescript"
model_preference = "balanced"

[dependencies]
std = "1.0"  # standard library (always included)
acme-shared-types = { registry = "acme/shared-types", version = "^2.0" }
payment-constraints = { git = "https://github.com/acme/payment-constraints", tag = "v1.2" }

[policies]
security = "./policies/security.omnipolicy"
performance = "./policies/performance.omnipolicy"

[build]
output_dir = "./build"
cache_dir = "./.omni-cache"
parallel_agents = 4

[budget]
max_total_cost = "$5.00"
cost_alert_threshold = 80
```

---

## Dependency Management

### Version Resolution

OmniLang uses semantic versioning with a lock file (`omni.lock`):

```
omni.toml  → Desired dependency ranges
omni.lock  → Exact resolved versions (committed to Git)
```

```bash
$ omni deps resolve   # Resolve dependencies and update lock file
$ omni deps update    # Update all dependencies to latest compatible versions
$ omni deps audit     # Check for known issues in dependencies
$ omni deps tree      # Show dependency graph
```

### Dependency Graph

```bash
$ omni deps tree
acme-ecommerce@1.0.0
├── std@1.0.3
├── acme-shared-types@2.1.0
│   └── std@1.0.3 (deduped)
└── payment-constraints@1.2.0
    └── std@1.0.3 (deduped)
```

---

## Composability Patterns

### Trait-like Mixins

Define reusable capability bundles:

```omnilang
mixin Auditable
  constraints:
    - audit_logging(level: "detailed")
    - tamper_proof_log

  postconditions:
    - audit_log.last_entry.action == self.name
    - audit_log.last_entry.timestamp == now()
    - audit_log.last_entry.actor == context.current_user

mixin rateLimited(requests int, window duration)
  constraints:
    - rate_limit(max: requests, per: window)
    - rate_limit_response 429 with Retry-After header

  tests:
    - scenario: "Rate limit enforced"
      when: send_requests(count: requests + 1, within: window)
      expect: last_response.status == 429

mixin cacheable(ttl duration)
  constraints:
    - cache(strategy: "read-through", ttl: ttl)
    - cache_invalidation: on_write

  postconditions:
    - cache_hit_rate > 80% under steady-state load

// Apply mixins to services
service ProductSearch
  includes [Auditable, RateLimited(100, 1min), Cacheable(5min)]

  goal "Search products by keyword, category, and filters"
  // ...
```

### Extension Points

Define slots where other modules can plug in:

```omnilang
service OrderPipeline
  goal "Process orders through a configurable pipeline"

  stages:
    - validate_order   // built-in
    - @extension("pre_payment")  // extension point
    - process_payment  // built-in
    - @extension("post_payment") // extension point
    - fulfill_order    // built-in

  extension_contract "pre_payment"
    inputs:
      order ValidatedOrder
    outputs:
      order ValidatedOrder  // may modify order
    constraints:
      - must_not_modify [order.id, order.customer]
      - max_latency 100ms

// In another module:
extend OrderPipeline.pre_payment
  name "FraudCheck"
  impl service FraudDetection
  priority 1  // runs first among extensions
```

### Generic Specifications

Parameterized specs for common patterns:

```omnilang
generic CrudService<Entity, Id>
  goal "Standard CRUD operations for ${Entity.name}"

  operation Create
    inputs:
      data Partial<Entity>
    outputs:
      entity Entity
    postconditions:
      - entity.id is newly generated
      - entity matches data (for provided fields)

  operation Read
    inputs:
      id Id
    outputs:
      entity Entity
    errors:
      NotFound(id Id)

  operation Update
    inputs:
      id Id
      data Partial<Entity>
    outputs:
      entity Entity
    errors:
      NotFound(id Id)
    postconditions:
      - entity.id == id
      - entity matches data (for provided fields)
      - entity.updated_at > old(entity.updated_at)

  operation Delete
    inputs:
      id Id
    outputs:
      success bool
    errors:
      NotFound(id Id)

  operation List
    inputs:
      filters partial<Entity>
      pagination paginationParams
    outputs:
      result paginatedList<Entity>

// Instantiate for a specific entity
service productService = CrudService<product, productId>
  // Add product-specific extensions
  constraints:
    - soft_delete

  operation Search
    inputs:
      query string
      filters productFilters
    outputs:
      result paginatedList<product>
    constraints:
      - full_text_search
      - latency(p95: <100ms)
```

---

## Namespacing and Scoping

### Scope Rules

1. **Types** are visible within their module and to importers
2. **Constraints** defined at module level apply to all services in that module
3. **Policies** at project level apply to all modules
4. **Tests** are scoped to their parent block but can reference other modules' types

### Name Resolution Order

```
1. Current block scope
2. Current module scope
3. Imported names
4. Standard library (auto-imported)
```

### Conflict Resolution

```omnilang
// If two imports provide the same name:
use acme.payments.{Order}
use acme.catalog.{Order}       // ERROR: ambiguous import "Order"

// Fix with aliases:
use acme.payments.{Order as PayOrder}
use acme.catalog.{Order as CatOrder}
```

---

## Registry

OmniLang specs can be published to and consumed from a package registry.

### Publishing

```bash
$ omni publish
Publishing acme-shared-types@2.1.0 to registry...
  ├── Validating spec... ✓
  ├── Running tests... ✓ (all pass)
  ├── Generating docs... ✓
  └── Uploading... ✓

Published: registry://acme/shared-types@2.1.0
```

### Consuming

```toml
# omni.toml
[dependencies]
shared-types = { registry = "acme/shared-types", version = "^2.0" }
```

### Registry Contents

A published package includes:

```
acme-shared-types@2.1.0/
├── specs/              # .omni source files
├── golden/             # Visual golden files
├── docs/               # Generated documentation
├── omni.toml           # Package manifest
├── omni.lock           # Locked dependencies
└── CHANGELOG.md        # Version history
```

---

## Module Best Practices

1. **One domain concept per module** — don't mix payments and shipping in one file
2. **Types in a shared module** — common types used across services go in `<domain>/types.omni`
3. **Policies at project root** — cross-cutting concerns like security go in `policies/`
4. **Use mixins for cross-cutting** — logging, rate limiting, caching are mixin candidates
5. **Version your published specs** — follow semver, especially for shared types
6. **Keep modules small** — each file should be readable in one sitting (<300 lines)
