# Examples

Real-world OmniLang specifications demonstrating how the language applies to common software development scenarios.

---

## Example 1: E-Commerce Checkout System

A complete checkout flow with payment processing, inventory management, and order confirmation.

```omnilang
module acme.ecommerce.checkout

use std.auth.{AuthContext, Session}
use std.http.{StatusCode}
use std.money.{Money, CurrencyCode}
use ./types.{Order, CartItem, PaymentMethod, ShippingAddress}
use ./inventory.{InventoryService}
use ./payments.{PaymentProcessor}

version: "2.1.0"
target: typescript


// ─── Types ──────────────────────────────────────────────

type CheckoutSession = struct
  id UUID
  cart_items List<CartItem>(min: 1, max: 100)
  shipping ShippingAddress
  payment PaymentMethod
  subtotal Money
  tax Money
  shipping_cost Money
  total Money
  currency CurrencyCode
  created_at DateTime
  expires_at DateTime

type CheckoutResult = enum
  Success(order Order, confirmation_number String)
  PaymentDeclined(reason String, retry_allowed Bool)
  OutOfStock(unavailable_items List<CartItem>)
  SessionExpired


// ─── Main Service ───────────────────────────────────────

service CheckoutService
  goal "Process customer checkout with payment, inventory reservation, and order creation"

  depends_on:
    - InventoryService
    - PaymentProcessor

  constraints:
    - idempotent
    - PCI_DSS_v4
    - no_plaintext_cards
    - GDPR_compliant
    - latency(p95: <500ms, p99: <1000ms)
    - availability(target: 99.95%)
    - audit_logging(level: "detailed")
    - circuit_breaker(threshold: 5, timeout: 30s)

  metrics:
    - counter payment_attempts_total
        description "Total payment attempts"
        labels [payment_method, status]
    - histogram checkout_value_usd
        description "Distribution of transaction amounts"
        buckets [10, 50, 100, 500, 1000]

  budget:
    cost:
      max_total $0.30
      per_retry $0.05
    tokens:
      model_strategy:
        generation SmartExpensive  // payment code must be high quality
        validation CheapFast

  // ─── Create Checkout Session ──────────────────────────

  operation CreateSession
    inputs:
      auth AuthContext
      cart_items List<CartItem>(min: 1)
      shipping ShippingAddress

    outputs:
      session CheckoutSession

    preconditions:
      - auth.is_authenticated
      - cart_items.all(item => item.quantity > 0)
      - shipping.is_valid_address

    postconditions:
      - session.expires_at == session.created_at + 30min
      - session.total == session.subtotal + session.tax + session.shipping_cost
      - session.subtotal == sum(cart_items.map(i => i.price * i.quantity))

    tests:
      - scenario: "Create session with valid cart"
        given:
          authenticated_user()
          cart_items: [
            Factory<CartItem>.create(price: Money(29.99, USD), quantity: 2),
            Factory<CartItem>.create(price: Money(49.99, USD), quantity: 1)
          ]
          shipping: Factory<ShippingAddress>.create(country: "US", state: "CA")
        expect:
          session.subtotal == Money(109.97, USD)
          session.tax > Money(0, USD)  // CA has sales tax
          session.expires_at > now()

      - scenario: "Empty cart rejected"
        given:
          authenticated_user()
          cart_items: []
        expect_error: status == 400

      - scenario: "Unauthenticated user rejected"
        given:
          no_auth()
          cart_items: [Factory<CartItem>.create()]
        expect_error: status == 401

  // ─── Process Checkout ─────────────────────────────────

  operation ProcessCheckout
    inputs:
      session_id UUID
      payment_token String  // tokenized, never raw card
      idempotency_key UUID

    outputs:
      result CheckoutResult

    preconditions:
      - session exists and not expired
      - payment_token is valid token format

    postconditions:
      - if result is Success:
          inventory_reserved(session.cart_items)
          payment_charged(session.total)
          order_created(result.order)
          confirmation_email_queued(session.customer_email)
      - if result is PaymentDeclined:
          inventory_not_reserved
          no_charge_made
      - if result is OutOfStock:
          no_charge_made
          unavailable_items accurately reflects current stock

    invariants:
      - no_duplicate_charges(idempotency_key)
      - sum_of_all_payments == sum_of_all_orders  // accounting conservation

    errors:
      - SessionNotFound(session_id UUID)
      - SessionExpired(expired_at DateTime)
      - PaymentFailed(provider_error String)
      - InventoryConflict(items List<CartItem>)

    tests:
      - scenario: "Successful checkout"
        given:
          valid_session(total: Money(109.97, USD))
          valid_payment_token()
          all_items_in_stock()
        expect:
          result is CheckoutResult.Success
          result.order.total == Money(109.97, USD)
          result.confirmation_number matches regex("^ORD-[A-Z0-9]{8}$")

      - scenario: "Idempotent retry"
        given:
          valid_session(total: Money(50.00, USD))
          valid_payment_token()
          idempotency_key: "key-123"
        when:
          process_checkout(key: "key-123")  // first call
          process_checkout(key: "key-123")  // duplicate call
        expect:
          both calls return same result
          payment charged exactly once

      - scenario: "Payment declined"
        given:
          valid_session()
          declined_payment_token()
        expect:
          result is CheckoutResult.PaymentDeclined
          result.retry_allowed == true
          no inventory reserved
          no email sent

      - scenario: "Partial out of stock"
        given:
          session with items [A(qty: 5), B(qty: 2)]
          item_a_stock: 3  // not enough
          item_b_stock: 10 // enough
        expect:
          result is CheckoutResult.OutOfStock
          result.unavailable_items contains A
          no payment attempted

      - scenario: "Expired session"
        given:
          session created 31 minutes ago
        expect:
          result is CheckoutResult.SessionExpired

      - property: "Payment amount matches order total"
        forall:
          session <- arbitrary<CheckoutSession>
          token <- valid_payment_token()
        given: all_items_in_stock()
        when: process_checkout(session, token)
        assert:
          if result is Success:
            payment_amount == session.total

      - benchmark: "Checkout under load"
        load: 500 concurrent_users
        duration: 120s
        expect:
          p95_latency < 500ms
          error_rate < 0.5%
          no_duplicate_charges

      - chaos: "Payment provider timeout"
        inject: payment_provider_timeout(duration: 10s)
        expect:
          circuit_breaker activates after 5 failures
          subsequent requests get immediate failure (not timeout)
          service recovers within 30s after provider recovers

      - security: "Payment token cannot be logged"
        when: process_checkout(payment_token: "tok_live_abc123")
        expect:
          application_logs do not contain "tok_live_abc123"
          audit_log contains masked version "tok_***123"

  evidence:
    required:
      - test_results(format: junit_xml, expect: all_pass)
      - coverage(format: lcov, expect: line >= 90%)
      - security_scan(format: sarif, expect: no_critical)
      - benchmark(format: json, expect: matches_constraints)
      - pci_compliance_checklist(expect: all_items_checked)
```

---

## Example 2: Real-Time Chat Application

```omnilang
module acme.messaging.chat

use std.auth.{AuthContext, UserId}
use std.time.{DateTime, Duration}

version: "1.0.0"
target: typescript


type Message = struct
  id UUID
  conversation_id ConversationId
  sender UserId
  content MessageContent
  sent_at DateTime
  edited_at Option<DateTime>
  reactions Map<Emoji, Set<UserId>>
  thread_id Option<UUID>
  read_by Set<UserId>

type MessageContent = enum
  Text(body String(max_length: 4000))
  Image(url URL, caption Option<String>)
  File(url URL, name String, size_bytes Int)
  System(event SystemEvent)

type Conversation = struct
  id ConversationId
  type enum
    DirectMessage
    Group
    Channel
  participants Set<UserId>(min: 2)
  name Option<String>
  created_at DateTime
  last_message_at DateTime

type TypingIndicator = struct
  user UserId
  conversation ConversationId
  started_at DateTime


service ChatService
  goal "Real-time messaging with typing indicators, read receipts, and reactions"

  constraints:
    - websocket_support
    - message_delivery_guarantee: at_least_once
    - message_ordering: causal within conversation
    - latency(p95: <100ms for send, <50ms for typing indicator)
    - max_message_size: 4KB text, 50MB file
    - encrypt_in_transit(TLS_1_3)
    - encrypt_at_rest(AES_256)
    - rate_limit(messages: 30 per minute per user)
    - profanity_filter(configurable: true)
    - max_participants_per_group: 500

  operation SendMessage
    inputs:
      auth AuthContext
      conversation_id ConversationId
      content MessageContent
      thread_id Option<UUID>

    outputs:
      message Message

    preconditions:
      - auth.user_id in conversation.participants
      - content.length <= max_message_size

    postconditions:
      - message delivered to all online participants in real-time
      - message persisted to database
      - offline participants receive on reconnect
      - push notification sent to offline participants

    tests:
      - scenario: "Send text message"
        given:
          user_a in conversation with user_b
          user_b is online (websocket connected)
        when: user_a sends "Hello!"
        expect:
          user_b receives message within 100ms
          message.content == Text("Hello!")
          message.sender == user_a.id

      - scenario: "Offline message delivery"
        given:
          user_a in conversation with user_b
          user_b is offline
        when: user_a sends "Are you there?"
        then: user_b comes online
        expect:
          user_b receives message on reconnect
          push notification was sent to user_b

      - scenario: "Rate limiting"
        given: user_a in conversation
        when: user_a sends 31 messages in 1 minute
        expect: 31st message rejected with RateLimited error

  operation StartTyping
    inputs:
      auth AuthContext
      conversation_id ConversationId

    postconditions:
      - all other online participants see typing indicator
      - typing indicator auto-expires after 5s without renewal
      - typing indicator clears when message sent

    tests:
      - scenario: "Typing indicator visible"
        given:
          user_a and user_b in conversation, both online
        when: user_a starts typing
        expect: user_b sees typing indicator for user_a within 50ms

      - scenario: "Typing indicator expires"
        given: user_a starts typing
        when: 5s pass without renewal
        expect: typing indicator disappears for all participants

  operation ReactToMessage
    inputs:
      auth AuthContext
      message_id UUID
      emoji Emoji

    postconditions:
      - reaction added to message
      - reaction visible to all participants in real-time
      - same user + same emoji = toggle (remove if exists)

    tests:
      - scenario: "Add reaction"
        when: user_a reacts with 👍 to message
        expect: message.reactions[👍] contains user_a.id

      - scenario: "Toggle reaction"
        given: user_a already reacted with 👍
        when: user_a reacts with 👍 again
        expect: message.reactions[👍] does not contain user_a.id
```

---

## Example 3: ML Model Serving Pipeline

```omnilang
module acme.ml.serving

use std.observe.{Metric, Alert}
use std.infra.{ContainerSpec, ResourceLimit}

version: "1.0.0"
target: python


type ModelVersion = struct
  model_id String
  version String(format: semver)
  artifact_path URL
  input_schema JSONSchema
  output_schema JSONSchema
  metadata struct
    trained_at DateTime
    training_metrics Map<String, Float64>
    framework enum
      PyTorch
      TensorFlow
      ONNX
      JAX
    size_bytes Int

type PredictionRequest = struct
  model_id String
  version Option<String>  // None = latest
  input JSON
  request_id UUID
  priority enum
    Low
    Normal
    High
    Critical

type PredictionResponse = struct
  prediction JSON
  model_version String
  latency_ms Float64
  confidence Float64
  request_id UUID


service ModelServingPlatform
  goal "Serve ML models with low latency, auto-scaling, A/B testing, and monitoring"

  constraints:
    - latency(p50: <20ms, p95: <50ms, p99: <200ms) for inference
    - availability: 99.99%
    - auto_scale(min: 2, max: 50, metric: "requests_per_second", target: 1000)
    - gpu_support(types: [T4, A100])
    - model_isolation  // models don't share memory
    - graceful_degradation  // return cached/fallback on overload
    - canary_deployment(traffic_percentage: 5, duration: 30min)

  operation Predict
    inputs:
      request PredictionRequest

    outputs:
      response PredictionResponse

    preconditions:
      - model exists and is deployed
      - input validates against model.input_schema

    postconditions:
      - response.prediction validates against model.output_schema
      - response.latency_ms == actual_processing_time
      - prediction logged for monitoring

    tests:
      - scenario: "Standard prediction"
        given:
          model "sentiment-v2" deployed with input schema { text: String }
        when:
          predict(model: "sentiment-v2", input: { text: "Great product!" })
        expect:
          response.prediction.label in ["positive", "negative", "neutral"]
          response.confidence in [0.0, 1.0]
          response.latency_ms < 50

      - benchmark: "Throughput under load"
        model: "sentiment-v2"
        load: 5000 rps sustained
        duration: 5min
        expect:
          p95_latency < 50ms
          error_rate < 0.01%
          auto_scaler increases replicas

      - chaos: "Model server crash"
        inject: kill_random_replica
        expect:
          remaining replicas absorb traffic
          new replica starts within 30s
          no failed predictions during failover

  operation DeployModel
    inputs:
      model ModelVersion
      strategy enum
        RollingUpdate
        BlueGreen
        Canary

    outputs:
      deployment_id UUID
      status DeploymentStatus

    postconditions:
      - if strategy == Canary:
          5% of traffic routes to new version for 30min
          if error_rate(new) > error_rate(old) * 1.5: auto_rollback
          if healthy after 30min: promote to 100%

    tests:
      - scenario: "Canary deployment with regression"
        given: model-v1 serving at 99.9% success rate
        when: deploy model-v2 with Canary strategy
        and: model-v2 has 5% error rate (regression)
        expect:
          auto_rollback within 5min
          alert fired: "Canary deployment rolled back"
          model-v1 still serving 100% traffic

  evidence:
    required:
      - test_results(format: junit_xml, expect: all_pass)
      - load_test_results(format: json, expect: meets_slo)
      - container_scan(expect: no_critical_vulnerabilities)
```

---

## Example 4: Organization-Wide Security Policy

```omnilang
module acme.policies.security

version: "3.0.0"


policy GlobalSecurityPolicy
  description "Mandatory security requirements for all ACME services"
  scope global
  enforced true  // cannot be overridden by individual services

  rules:
    // ─── Authentication ───────────────────────────────
    - all external-facing services must:
        require authentication
        supported_methods [JWT, OAuth2, mTLS]
        session_ttl max 24h
        refresh_token_rotation true

    // ─── Authorization ────────────────────────────────
    - all endpoints must:
        implement RBAC or ABAC
        principle least_privilege
        log all_access_decisions

    // ─── Data Protection ──────────────────────────────
    - all PII fields must:
        encrypt_at_rest AES_256_GCM
        mask_in_logs true
        comply_with GDPR
        retention_max 2years unless legal_hold

    // ─── Network Security ─────────────────────────────
    - all service-to-service communication must:
        use mTLS
        certificate_rotation every 90days

    - all external communication must:
        use TLS_1_3
        HSTS max_age 31536000, includeSubDomains

    // ─── Input Validation ─────────────────────────────
    - all inputs must:
        validate schema + semantic
        sanitize XSS, SQL_injection, command_injection
        max_size 10MB unless explicitly configured

    // ─── Secrets Management ───────────────────────────
    - all secrets must:
        source vault or environment (never hardcoded)
        rotate every 90days
        audit all_access_logged

    // ─── AI Agent Constraints ─────────────────────────
    - all AI agents must:
        operate_with least_privilege
        log all_tool_invocations
        respect budget_constraints
        cannot access production_data during generation
        must use tokenized test data
```

---

## Example 5: Multi-Service System (Module Composition)

```omnilang
// File: specs/acme/platform/system.omni
module acme.platform.system

use ./auth.{AuthService}
use ./catalog.{CatalogService}
use ./checkout.{CheckoutService}
use ./shipping.{ShippingService}
use ./notifications.{NotificationService}
use ./analytics.{AnalyticsService}

version: "1.0.0"


system AcmePlatform
  goal "Complete e-commerce platform with auth, catalog, checkout, shipping, notifications"

  services:
    - AuthService
    - CatalogService
    - CheckoutService
    - ShippingService
    - NotificationService
    - AnalyticsService

  topology:
    gateway -> [AuthService, CatalogService, CheckoutService]
    CheckoutService -> [InventoryService, PaymentProcessor, NotificationService]
    ShippingService -> [InventoryService, NotificationService]
    AnalyticsService <- [all services]  // event consumer

  constraints:
    - all services: containerized(runtime: kubernetes)
    - all services: health_check(path: "/health", interval: 10s)
    - all services: structured_logging(format: JSON)
    - all services: distributed_tracing(OpenTelemetry)
    - inter_service: circuit_breaker(threshold: 5, timeout: 30s)
    - inter_service: retry(max: 3, backoff: exponential)

  tests:
    - integration: "End-to-end checkout flow"
      steps:
        1. AuthService.login(user: test_user) -> session
        2. CatalogService.search(query: "headphones") -> products
        3. CheckoutService.createSession(items: [products[0]]) -> checkout
        4. CheckoutService.processCheckout(session: checkout.id) -> order
        5. ShippingService.getTracking(order: order.id) -> tracking
      expect:
        order.status == Confirmed
        tracking is assigned within 24h
        notification_sent(type: "order_confirmation", to: test_user.email)

    - chaos: "Cascading failure resilience"
      inject: PaymentProcessor offline for 60s
      expect:
        CheckoutService returns graceful error
        CatalogService and ShippingService unaffected
        system recovers within 30s after PaymentProcessor returns
        no data inconsistency

  evidence:
    required:
      - architecture_diagram(type: generated, format: mermaid)
      - api_documentation(format: openapi_3)
      - runbook(for_each: service, covers: [deploy, rollback, incident_response])
```
