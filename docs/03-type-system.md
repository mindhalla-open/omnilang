# Type System

OmniLang features a rich type system that goes far beyond traditional data types. It includes **data types**, **confidence types**, **multimodal types**, **budget types**, and **semantic types** — all designed to minimize ambiguity between human intent and AI-generated output.

---

## Design Principles

1. **Types encode meaning, not just shape** — `Email` is not just `String`, it carries validation rules and semantic intent
2. **Types flow across boundaries** — a contract's output type must be compatible with its consumer's input type
3. **Types express uncertainty** — `Confident<T>` wraps any type with a trust level
4. **Types include non-data concepts** — cost budgets, latency SLOs, and visual expectations are typed

---

## Primitive Types

```omnilang
// Scalar types
bool
int                    // arbitrary precision
int32, int64           // fixed width
float32, float64
string
char

// Temporal types
datetime               // ISO 8601
duration               // e.g., 5s, 200ms, 24h
timestamp              // Unix epoch milliseconds
date
time

// Identity types
uuid
ulid
cuid

// Monetary
money(amount: float64, currency: CurrencyCode)

// Network
url
email
ipAddress(v4 | v6)
port
```

---

## Collection Types

```omnilang
list<T>                // ordered collection
set<T>                 // unique unordered collection
map<K, V>              // key-value mapping
queue<T>               // FIFO queue
stack<T>               // LIFO stack
tree<T>                // hierarchical structure

// Bounded collections
list<T>(min: 1, max: 100)
string(min_length: 1, max_length: 255)
```

---

## Algebraic Types

```omnilang
// Sum types (tagged unions)
type paymentStatus = enum
  Pending
  Processing
  Completed(transaction_id transactionId)
  Failed(error paymentError)
  Refunded(refund_id refundId, amount money)

// Product types (records/structs)
type address = struct
  street string(max_length: 200)
  city string
  state option<string>
  postal_code postalCode
  country countryCode(ISO_3166_1)

// Option type (nullable)
option<T> = Some(T) | None

// Result type (fallible operations)
result<T, E> = Ok(T) | Err(E)
```

---

## Semantic Types (Refined Types)

Semantic types add domain-specific validation rules to base types. They are not just aliases — they carry **verification constraints** that the agent must ensure in the generated code.

```omnilang
// Define refined types with validation rules
type email = string
  format RFC_5322
  max_length 254
  normalize lowercase

type password = string
  min_length 12
  must_contain [uppercase, lowercase, digit, special]
  not_in common_passwords_list
  storage bcrypt(rounds: 12)  // semantic: how to persist

type postalCode = string
  format regex("^[0-9]{5}(-[0-9]{4})?$")  // US ZIP

type latitude = float64
  range [-90.0, 90.0]

type longitude = float64
  range [-180.0, 180.0]

type geoPoint = struct
  lat latitude
  lng longitude

type age = int
  range [0, 150]

type percentage = float64
  range [0.0, 100.0]
  precision 2
```

---

## Confidence Types

Confidence types wrap any value with a trust level, enabling the system to track and propagate uncertainty.

```omnilang
// Generic confidence wrapper
type confident<T> = struct
  value T
  confidence confidenceLevel
  evidence list<evidenceRef>
  generated_by agentId
  generated_at timestamp

// Confidence levels
type confidenceLevel = enum
  Proven         // Formally verified
  High           // Thoroughly tested
  Medium         // Functionally tested
  Low            // Minimally tested
  Speculative    // Unverified

// Usage in contracts
contract analyzeSentiment
  inputs:
    text string

  outputs:
    sentiment confident<sentimentScore>
    // The output explicitly carries its confidence level

  postconditions:
    - sentiment.confidence >= Medium
    // Agent must achieve at least Medium confidence
```

### Confidence Arithmetic

```omnilang
// Confidence degrades through composition
Confident<A> + Confident<B> → Confident<C>
  where C.confidence = min(A.confidence, B.confidence)

// Confidence can be upgraded via additional evidence
upgrade(Confident<T>, evidence: Evidence) → Confident<T>
  where new.confidence = compute_confidence(old.evidence + evidence)
```

---

## Multimodal Types

OmniLang treats non-textual data as first-class types, enabling specs that reference images, audio, video, and structured traces.

```omnilang
// Visual types
// Visual types
type screenshot = image
  format PNG | JPEG | WebP
  metadata struct
    viewport viewport
    device_pixel_ratio float64
    captured_at timestamp

type visualGolden = struct
  reference screenshot
  tolerance percentage        // pixel diff tolerance
  ignore_regions list<rect>   // areas to skip (e.g., timestamps)

type viewport = struct
  width int(range: [320, 3840])
  height int(range: [240, 2160])

// Trace types
type trace = struct
  spans list<span>
  format OpenTelemetry | Jaeger | Zipkin

type span = struct
  operation string
  duration duration
  status spanStatus
  attributes map<string, Any>
  children list<span>

// Log types
type logStream = struct
  entries list<logEntry>
  format JSON | Plaintext | Structured

type logEntry = struct
  timestamp timestamp
  level logLevel
  message string
  context map<string, Any>

// Schema types
type databaseSchema = struct
  tables list<tableDef>
  format SQL_DDL | Prisma | TypeORM

type apiSchema = struct
  endpoints list<endpointDef>
  format OpenAPI_3 | GraphQL_SDL | Protobuf

// Document types
type diagram = image
  source Mermaid | PlantUML | D2
  rendered screenshot

type document = struct
  content Markdown | RST | AsciiDoc
  embedded_media list<image | diagram>
```

---

## Budget Types

Budget types make resource constraints explicit and trackable.

```omnilang
type tokenBudget = struct
  max_input_tokens int
  max_output_tokens int
  max_total_tokens int
  model_preference modelPreference

type costBudget = struct
  max_total money
  per_component option<money>
  per_retry option<money>
  alert_threshold percentage  // alert when N% consumed

type modelPreference = enum
  CheapFast         // e.g., GPT-4o-mini, Claude Haiku
  Balanced          // e.g., GPT-4o, Claude Sonnet
  SmartExpensive    // e.g., o3, Claude Opus
  Custom(model_id String)

type timeBudget = struct
  max_generation_time duration
  max_verification_time duration
  timeout_action timeoutAction

type timeoutAction = enum
  ReturnBestEffort(confidence speculative)
  Fail(message String)
  EscalateToHuman

// Usage
budget
  cost:
    max_total $0.50
    per_component $0.10
    alert_threshold 80%

  tokens:
    max_total_tokens 100_000
    model_preference Balanced

  time:
    max_generation_time 5min
    timeout_action EscalateToHuman
```

---

## Type Composition

Types compose through standard algebraic operations:

```omnilang
// Union types
type paymentMethod = creditCard | bankTransfer | cryptoWallet | payPal

// Intersection types (must satisfy all)
type secureEndpoint = endpoint & authenticated & rateLimited & logged

// Generic types
type paginated<T> = struct
  items list<T>
  total int
  page int
  per_page int(range: [1, 100])
  has_next bool

// Mapped types (transform all fields)
type nullable<T: struct> = {
  [field in T]: option<T[field]>
}

// Partial types (all fields optional)
type updateRequest<T: struct> = partial<T> & { id: T.id }
```

---

## Type Compatibility and Subtyping

OmniLang uses **structural subtyping** — types are compatible if their shapes match, regardless of name.

```omnilang
type dog = struct
  name string
  age int

type pet = struct
  name string

// dog is a subtype of pet (has all required fields)
// So a function expecting pet can accept dog
```

### Contract Type Checking

The analyzer verifies type compatibility across service boundaries:

```omnilang
service orderService
  outputs:
    order order  // includes field: items: list<orderItem>

service shippingService
  inputs:
    items list<shippableItem>  // shippableItem ⊂ orderItem must hold

// The analyzer checks: orderItem is structurally compatible with shippableItem
// If not → compile-time error: "orderItem missing field 'weight' required by shippableItem"
```

---

## Standard Library Types

OmniLang includes a standard library of commonly used domain types:

| Module | Types |
|--------|-------|
| `std.auth` | `UserId`, `SessionToken`, `Permission`, `Role`, `AuthContext` |
| `std.http` | `HttpMethod`, `StatusCode`, `Header`, `Request`, `Response` |
| `std.money` | `Money`, `CurrencyCode`, `ExchangeRate` |
| `std.geo` | `GeoPoint`, `BoundingBox`, `Distance` |
| `std.time` | `TimeZone`, `DateRange`, `Cron`, `Recurrence` |
| `std.media` | `Image`, `Video`, `Audio`, `MimeType` |
| `std.infra` | `ContainerSpec`, `ResourceLimit`, `HealthCheck` |
| `std.observe` | `Metric`, `Trace`, `LogLevel`, `Alert` |
