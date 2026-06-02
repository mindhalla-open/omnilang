import Anthropic from "@anthropic-ai/sdk";
import pc from "picocolors";
import { LLMProvider } from "./base";

export class AnthropicProvider implements LLMProvider {
  private client?: Anthropic;
  private isMock: boolean;

  constructor() {
    this.isMock = process.env.OMNI_MOCK_LLM === "true";
    if (!this.isMock) {
      const apiKey = process.env.ANTHROPIC_API_KEY || process.env.OMNI_API_KEY;
      if (!apiKey) {
        throw new Error(
          "Missing Anthropic API Key. Please set ANTHROPIC_API_KEY or OMNI_API_KEY in your environment, or run with OMNI_MOCK_LLM=true for mock mode."
        );
      }
      this.client = new Anthropic({ apiKey });
    }
  }

  public async generateCode(systemPrompt: string, userPrompt: string, model?: string): Promise<string> {
    if (this.isMock) {
      console.log(pc.blue("   [MOCK LLM] Simulating Claude 3.5 Sonnet response..."));
      return this.getMockResponse(systemPrompt, userPrompt);
    }

    if (!this.client) {
      throw new Error("Anthropic client is not initialized.");
    }

    const selectedModel = model || process.env.ANTHROPIC_MODEL || process.env.OMNI_MODEL || "claude-3-5-sonnet-20241022";
    const response = await this.client.messages.create({
      model: selectedModel,
      max_tokens: 4000,
      system: systemPrompt,
      messages: [{ role: "user", content: userPrompt }],
    });

    const inputTokens = response.usage?.input_tokens || 0;
    const outputTokens = response.usage?.output_tokens || 0;
    const cost = (inputTokens / 1_000_000) * 3.0 + (outputTokens / 1_000_000) * 15.0;
    console.log(pc.green(`   [LLM Telemetry] Model: ${selectedModel}`));
    console.log(pc.green(`   [LLM Telemetry] Tokens: Input: ${inputTokens}, Output: ${outputTokens}`));
    console.log(pc.green(`   [LLM Telemetry] Estimated Cost: $${cost.toFixed(5)}`));

    const content = response.content[0];
    if (content.type !== "text") {
      throw new Error("Unexpected non-text response from Claude API.");
    }

    return content.text;
  }

  public getMockResponse(systemPrompt: string, userPrompt: string): string {
    const promptLower = userPrompt.toLowerCase();
    const systemPromptLower = systemPrompt.toLowerCase();

    // Detect target from the system prompt
    let raw: string;
    let comment = "//";
    if (systemPromptLower.includes("senior rust and test engineer")) {
      raw = this.getMockRustResponse(promptLower, userPrompt);
    } else if (systemPromptLower.includes("senior python and test engineer")) {
      raw = this.getMockPythonResponse(promptLower, userPrompt);
      comment = "#";
    } else if (systemPromptLower.includes("senior go and test engineer")) {
      raw = this.getMockGoResponse(promptLower, userPrompt);
    } else {
      raw = this.getMockTypeScriptResponse(promptLower, userPrompt, systemPrompt);
    }
    // Parity: ensure every target's generated code carries the contract markers
    // the coverage gate expects. The TypeScript mock embeds them inline; the
    // generic Rust/Python/Go mocks get them injected from the prompt's contract
    // list so all four targets behave identically.
    return this.injectContractMarkers(raw, userPrompt, comment);
  }

  /// Appends `@omni:contract <id>` markers (extracted from the prompt's contract
  /// section) to the first generated file, unless the response already marks
  /// contracts. Keeps mock parity across targets without hard-coding ids.
  private injectContractMarkers(responseJson: string, prompt: string, comment: string): string {
    const ids = Array.from(prompt.matchAll(/\[([A-Za-z0-9_.]+)\]/g))
      .map((m) => m[1])
      .filter((id) => id.includes("."));
    if (ids.length === 0) return responseJson;
    try {
      const parsed = JSON.parse(responseJson);
      if (!Array.isArray(parsed.files) || parsed.files.length === 0) return responseJson;
      if (parsed.files.some((f: any) => String(f.content).includes("@omni:contract"))) {
        return responseJson;
      }
      const markers = ids.map((id) => `${comment} @omni:contract ${id}`).join("\n");
      parsed.files[0].content = `${parsed.files[0].content}\n${markers}\n`;
      return JSON.stringify(parsed);
    } catch {
      return responseJson;
    }
  }

  private extractServiceName(prompt: string): string {
    // Match the quoted service name from the prompt (e.g. service: "paymentService").
    // Names may be camelCase (lower first letter), so do not require uppercase —
    // otherwise the snake_cased file path and the mod declaration diverge (E0583).
    const quotedMatch = prompt.match(/service:\s*"([A-Za-z][A-Za-z0-9]*)"/);
    if (quotedMatch) return quotedMatch[1];
    // Fallback: find a word ending in "Service".
    const svcMatch = prompt.match(/([A-Za-z][A-Za-z0-9]*[Ss]ervice)/);
    if (svcMatch) return svcMatch[1];
    return "DefaultService";
  }

  private toSnakeCase(name: string): string {
    return name.replace(/([A-Z])/g, "_$1").toLowerCase().replace(/^_/, "");
  }

  private getMockTypeScriptResponse(promptLower: string, rawPrompt: string, systemPrompt: string): string {
    // Case 0: SelfCorrectingService for validating the self-correction loop
    if (promptLower.includes("selfcorrectingservice")) {
      const isRetry = systemPrompt.includes("past generation retries") || systemPrompt.includes("LLM Prompt Adaptations");
      if (!isRetry) {
        // Return code with a TS type error on the first attempt
        return JSON.stringify({
          files: [
            {
              path: "src/services/SelfCorrectingService.ts",
              content: `export class SelfCorrectingService {
  public async process(): Promise<string> {
    const value: number = "this is a string, which triggers a TS compile error";
    return "ok";
  }
}
`
            },
            {
              path: "tests/SelfCorrectingService.test.ts",
              content: `import { SelfCorrectingService } from "../src/services/SelfCorrectingService";
describe("SelfCorrectingService", () => {
  it("should process", async () => {
    const service = new SelfCorrectingService();
    const res = await service.process();
    expect(res).toBe("ok");
  });
});
`
            }
          ]
        });
      } else {
        // Return corrected code on the retry attempt
        return JSON.stringify({
          files: [
            {
              path: "src/services/SelfCorrectingService.ts",
              content: `export class SelfCorrectingService {
  public async process(): Promise<string> {
    const value: number = 42;
    return "ok";
  }
}
`
            },
            {
              path: "tests/SelfCorrectingService.test.ts",
              content: `import { SelfCorrectingService } from "../src/services/SelfCorrectingService";
describe("SelfCorrectingService", () => {
  it("should process", async () => {
    const service = new SelfCorrectingService();
    const res = await service.process();
    expect(res).toBe("ok");
  });
});
`
            }
          ]
        });
      }
    }

    // Case 0.5: BankTransferService for validating self-correction loop in demos
    if (promptLower.includes("banktransferservice")) {
      const isRetry = systemPrompt.includes("past generation retries") || systemPrompt.includes("LLM Prompt Adaptations");
      if (!isRetry) {
        return JSON.stringify({
          files: [
            {
              path: "src/services/BankTransferService.ts",
              content: `import { AccountId, Account } from '../types';
export { AccountId, Account };
export class BankTransferService {
  public accounts = new Map<AccountId, Account>();
  public transfer(fromId: AccountId, toId: AccountId, amount: number): boolean {
    const value: number = "this triggers a TS compile error on the first attempt";
    return true;
  }
}
`
            },
            {
              path: "tests/BankTransferService.test.ts",
              content: `import { BankTransferService } from "../src/services/BankTransferService";
describe("BankTransferService", () => {
  it("should define service", () => {
    const service = new BankTransferService();
    expect(service).toBeDefined();
  });
});
`
            }
          ]
        });
      } else {
        return JSON.stringify({
          files: [
            {
              path: "src/services/BankTransferService.ts",
              content: `import { AccountId, Account } from '../types';
export { AccountId, Account };
export class BankTransferService {
  public accounts = new Map<AccountId, Account>();
  public transfer(fromId: AccountId, toId: AccountId, amount: number): boolean {
    if (amount <= 0) {
      throw new Error('Transfer amount must be strictly greater than zero');
    }
    const fromAcc = this.accounts.get(fromId);
    const toAcc = this.accounts.get(toId);
    if (!fromAcc || !toAcc || fromAcc.status !== 'Active' || toAcc.status !== 'Active') {
      throw new Error('Both accounts must exist and be in Active status');
    }
    if (fromAcc.balance < amount) {
      throw new Error('Sender account must have sufficient funds');
    }
    fromAcc.balance -= amount;
    toAcc.balance += amount;
    return true;
  }
  public addAccount(account: Account): void {
    this.accounts.set(account.id, { ...account });
  }
}
`
            },
            {
              path: "tests/BankTransferService.test.ts",
              content: `import { BankTransferService } from "../src/services/BankTransferService";
describe("BankTransferService", () => {
  let service: BankTransferService;
  beforeEach(() => {
    service = new BankTransferService();
    service.addAccount({ id: 'acc-1', balance: 100, status: 'Active' });
    service.addAccount({ id: 'acc-2', balance: 50, status: 'Active' });
  });
  it("should transfer funds successfully", () => {
    const res = service.transfer('acc-1', 'acc-2', 30);
    expect(res).toBe(true);
    expect(service.accounts.get('acc-1')?.balance).toBe(70);
    expect(service.accounts.get('acc-2')?.balance).toBe(80);
  });
  it("should reject negative transfers", () => {
    expect(() => service.transfer('acc-1', 'acc-2', -10)).toThrow('Transfer amount must be strictly greater than zero');
  });
});
`
            }
          ]
        });
      }
    }

    // Case 0.6: PaymentService for billing & invariants
    if (promptLower.includes("paymentservice")) {
      return JSON.stringify({
        files: [
          {
            path: "src/services/PaymentService.ts",
            content: `import { AccountId, Account } from '../types';
export { AccountId, Account };

export class PaymentService {
  public accounts = new Map<AccountId, Account>();
  public auditTrail: string[] = [];

  public deposit(accountId: AccountId, amount: number): number {
    // @omni:contract paymentService.Deposit.pre.1
    if (amount <= 0) {
      throw new Error("Deposit amount must be strictly greater than zero");
    }
    const account = this.accounts.get(accountId);
    if (!account) {
      throw new Error("Account not found");
    }
    const before = account.balance;
    account.balance += amount;
    // @omni:contract paymentService.Deposit.post.1
    if (account.balance !== before + amount) {
      throw new Error("New balance must increase exactly by the deposit amount");
    }
    // @omni:contract paymentService.inv.audit_logging
    this.auditTrail.push(\`deposit:\${accountId}:\${amount}\`);
    // @omni:contract paymentService.inv.balance_safety
    if (account.balance < 0) {
      throw new Error("balance_safety: account.balance >= 0");
    }
    return account.balance;
  }

  public charge(accountId: AccountId, amount: number): boolean {
    const account = this.accounts.get(accountId);
    if (!account) {
      throw new Error("Account not found");
    }
    // @omni:contract paymentService.Charge.pre.2
    if (account.status !== "Active") {
      throw new Error("Account must be in Active status");
    }
    // @omni:contract paymentService.Charge.pre.1
    if (account.balance < amount) {
      throw new Error("Account balance must be greater than or equal to the charge amount");
    }
    account.balance -= amount;
    // @omni:contract paymentService.Charge.post.1
    // On success, the balance is decreased by the charge amount.
    // @omni:contract paymentService.inv.balance_safety
    if (account.balance < 0) {
      throw new Error("balance_safety: account.balance >= 0");
    }
    // @omni:contract paymentService.inv.audit_logging
    this.auditTrail.push(\`charge:\${accountId}:\${amount}\`);
    return true;
  }

  public addAccount(account: Account): void {
    this.accounts.set(account.id, { ...account });
  }
}
`
          },
          {
            path: "tests/PaymentService.test.ts",
            content: `import { PaymentService } from '../src/services/PaymentService';
import fc from 'fast-check';

describe('PaymentService', () => {
  let service: PaymentService;

  beforeEach(() => {
    service = new PaymentService();
    service.addAccount({ id: 'acc-1', balance: 100, status: 'Active' });
    service.addAccount({ id: 'acc-2', balance: 0, status: 'Active' });
    service.addAccount({ id: 'acc-inactive', balance: 100, status: 'Inactive' });
  });

  test('deposit increases balance', () => {
    const newBalance = service.deposit('acc-1', 50);
    expect(newBalance).toBe(150);
    expect(service.accounts.get('acc-1')?.balance).toBe(150);
  });

  test('deposit rejects non-positive amount', () => {
    expect(() => service.deposit('acc-1', 0)).toThrow('Deposit amount must be strictly greater than zero');
    expect(() => service.deposit('acc-1', -10)).toThrow('Deposit amount must be strictly greater than zero');
  });

  test('charge decreases balance', () => {
    const success = service.charge('acc-1', 40);
    expect(success).toBe(true);
    expect(service.accounts.get('acc-1')?.balance).toBe(60);
  });

  test('charge rejects if balance is insufficient', () => {
    expect(() => service.charge('acc-2', 10)).toThrow('Account balance must be greater than or equal to the charge amount');
  });

  test('charge rejects if account is inactive', () => {
    expect(() => service.charge('acc-inactive', 50)).toThrow('Account must be in Active status');
  });

  test('property-based testing: balance remains non-negative', () => {
    fc.assert(fc.property(fc.integer({ min: 1, max: 100 }), (amount) => {
      const localService = new PaymentService();
      localService.addAccount({ id: 'acc-prop', balance: 100, status: 'Active' });
      try {
        localService.charge('acc-prop', amount);
        expect(localService.accounts.get('acc-prop')?.balance).toBeGreaterThanOrEqual(0);
      } catch (e: any) {
        expect(e.message).toMatch(/(balance_safety|greater than or equal to)/);
      }
    }));
  });
});
`
          }
        ]
      });
    }

    // Case 0.7: OrderService for full-stack ordering specification
    if (promptLower.includes("orderservice")) {
      return JSON.stringify({
        files: [
          {
            path: "src/services/OrderService.ts",
            content: `import { UserId, ProductId, OrderId, OrderItemId, OrderItem, Order } from '../types';
export { UserId, ProductId, OrderId, OrderItemId, OrderItem, Order };

export class OrderService {
  public orders = new Map<OrderId, Order>();
  public orderItems = new Map<OrderId, OrderItem[]>();
  public users = new Map<UserId, { id: UserId; name: string; status: string }>();

  public createOrder(userId: UserId, items: OrderItem[]): Order {
    const user = this.users.get(userId);
    if (!user || user.status !== 'Active') {
      throw new Error('User must exist and have an Active status');
    }
    if (items.length === 0) {
      throw new Error('At least one item must be present in the order');
    }
    for (const item of items) {
      if (item.qty <= 0) {
        throw new Error('All product quantities must be strictly positive');
      }
    }

    let total = 0;
    for (const item of items) {
      total += item.price * item.qty;
    }

    const orderId = \`order-\${Date.now()}\`;
    const order: Order = {
      id: orderId,
      userId,
      total,
      status: 'Created'
    };

    this.orders.set(orderId, order);
    this.orderItems.set(orderId, [...items]);
    return order;
  }

  public applyPromoCode(orderId: OrderId, code: string): Order {
    const order = this.orders.get(orderId);
    if (!order || order.status !== 'Created') {
      throw new Error('Order must exist and be in Created status');
    }
    if (code !== 'VALID_CODE') {
      throw new Error('Promo code must be valid');
    }

    const user = this.users.get(order.userId);
    if (order.total > 100 && user && user.status === 'VIP') {
      order.total = order.total * 0.9;
    }
    return order;
  }

  public processRefund(orderId: OrderId): boolean {
    const order = this.orders.get(orderId);
    if (!order || order.status !== 'Paid') {
      throw new Error('Order must be in Paid status to refund');
    }
    order.status = 'Refunded';
    return true;
  }

  public addUser(id: UserId, name: string, status: string): void {
    this.users.set(id, { id, name, status });
  }
}
`
          },
          {
            path: "tests/OrderService.test.ts",
            content: `import { OrderService } from '../src/services/OrderService';
import { OrderItem } from '../src/types';
import fc from 'fast-check';

describe('OrderService', () => {
  let service: OrderService;

  beforeEach(() => {
    service = new OrderService();
    service.addUser('user-1', 'Alice', 'Active');
    service.addUser('vip-1', 'Bob', 'Active');
  });

  test('createOrder calculates total', () => {
    const items: OrderItem[] = [
      { id: 'item-1', orderId: '', productId: 'prod-1', qty: 2, price: 10 },
      { id: 'item-2', orderId: '', productId: 'prod-2', qty: 1, price: 25 }
    ];
    const order = service.createOrder('user-1', items);
    expect(order.total).toBe(45);
    expect(order.status).toBe('Created');
  });

  test('applyPromoCode gives 10% discount to VIP on orders > 100', () => {
    const items: OrderItem[] = [
      { id: 'item-1', orderId: '', productId: 'prod-1', qty: 1, price: 150 }
    ];
    
    // Test for VIP user
    const orderVip = service.createOrder('vip-1', items);
    service.users.set('vip-1', { id: 'vip-1', name: 'Bob', status: 'VIP' });
    const discountedOrder = service.applyPromoCode(orderVip.id, 'VALID_CODE');
    expect(discountedOrder.total).toBe(135); // 150 - 10%
    
    // Test for regular user
    const orderRegular = service.createOrder('user-1', items);
    const nonDiscountedOrder = service.applyPromoCode(orderRegular.id, 'VALID_CODE');
    expect(nonDiscountedOrder.total).toBe(150);
  });

  test('property-based testing: total non-negative', () => {
    fc.assert(fc.property(fc.integer({ min: 1, max: 10 }), fc.integer({ min: 1, max: 100 }), (qty, price) => {
      const items: OrderItem[] = [
        { id: 'item-1', orderId: '', productId: 'prod-1', qty, price }
      ];
      const localService = new OrderService();
      localService.addUser('user-1', 'Alice', 'Active');
      const order = localService.createOrder('user-1', items);
      expect(order.total).toBeGreaterThanOrEqual(0);
    }));
  });
});
`
          }
        ]
      });
    }

    // Case 1: Checkout service
    if (promptLower.includes("checkoutservice") || promptLower.includes("\"checkout\"")) {
      return JSON.stringify({
        files: [
          {
            path: "src/services/Checkout.ts",
            content: `export interface ICheckoutService {
  placeOrder(customerId: string, items: any[]): Promise<string>;
}

export class CheckoutService implements ICheckoutService {
  public metrics = {
    payment_attempts_total: {
      counters: new Map<string, number>(),
      inc(labels: { payment_method: string; status: string }) {
        const key = JSON.stringify(labels);
        const val = this.counters.get(key) || 0;
        this.counters.set(key, val + 1);
      }
    }
  };

  public async placeOrder(customerId: string, items: any[]): Promise<string> {
    if (!customerId || customerId.trim() === "") {
      throw new Error("Customer ID is required");
    }
    if (!items || items.length === 0) {
      throw new Error("Cart is empty");
    }
    
    // Record metric attempt
    this.metrics.payment_attempts_total.inc({ payment_method: "credit_card", status: "success" });

    // Simulate order placement
    return "order-12345";
  }
}
`
          },
          {
            path: "tests/Checkout.test.ts",
            content: `import { CheckoutService } from "../src/services/Checkout";

describe("CheckoutService", () => {
  let service: CheckoutService;

  beforeEach(() => {
    service = new CheckoutService();
  });

  it("should place an order successfully", async () => {
    const orderId = await service.placeOrder("cust_999", [{ productId: "prod_1", quantity: 2 }]);
    expect(orderId).toBe("order-12345");
  });

  it("should throw error if customerId is missing", async () => {
    await expect(service.placeOrder("", [])).rejects.toThrow("Customer ID is required");
  });

  it("should record payment attempts total metric", async () => {
    await service.placeOrder("cust_999", [{ productId: "prod_1", quantity: 2 }]);
    const counters = service.metrics.payment_attempts_total.counters;
    const labelKey = JSON.stringify({ payment_method: "credit_card", status: "success" });
    expect(counters.get(labelKey)).toBe(1);
  });
});
`
          }
        ]
      });
    }

    // Case 1.5: UserService / stabilization test spec
    if (promptLower.includes("userservice") || promptLower.includes("registeruser")) {
      return JSON.stringify({
        files: [
          {
            path: "src/services/UserService.ts",
            content: `/**
 * UserService provides methods to register and manage user accounts
 */
export class UserService {
  private users = new Map<string, string>();

  /**
   * Register a new user with a given name
   */
  public async registerUser(name: string): Promise<string> {
    if (!name || name.trim() === "") {
      throw new Error("Name cannot be empty");
    }
    const id = "user-" + Math.random().toString(36).substring(2, 9);
    this.users.set(id, name);
    return id;
  }

  /**
   * Retrieve user name by their ID
   */
  public async getUserName(id: string): Promise<string> {
    const name = this.users.get(id);
    if (!name) {
      throw new Error("User not found");
    }
    return name;
  }

  /**
   * Update user profile details
   */
  public async updateUser(id: string, name: string): Promise<void> {
    if (!name || name.trim() === "") {
      throw new Error("Name cannot be empty");
    }
    if (!this.users.has(id)) {
      throw new Error("User not found");
    }
    this.users.set(id, name);
  }
}
`
          },
          {
            path: "tests/UserService.test.ts",
            content: `import { UserService } from "../src/services/UserService";

describe("UserService", () => {
  let service: UserService;

  beforeEach(() => {
    service = new UserService();
  });

  it("should register, retrieve, and update user successfully", async () => {
    const id = await service.registerUser("Alice");
    expect(id).toBeDefined();

    const name = await service.getUserName(id);
    expect(name).toBe("Alice");

    await service.updateUser(id, "Bob");
    const updatedName = await service.getUserName(id);
    expect(updatedName).toBe("Bob");
  });

  it("should fail to register user with empty name", async () => {
    await expect(service.registerUser("")).rejects.toThrow("Name cannot be empty");
  });

  it("should fail to update user with empty name", async () => {
    await expect(service.updateUser("user-123", "")).rejects.toThrow("Name cannot be empty");
  });
});
`
          }
        ]
      });
    }

    // Case 2: GreetingService / hello world spec
    if (promptLower.includes("greeting") || promptLower.includes("sayhello")) {
      return JSON.stringify({
        files: [
          {
            path: "src/services/GreetingService.ts",
            content: `export interface IGreetingService {
  sayHello(name: string): Promise<{ id: string; message: string; created_at: Date }>;
}

export class GreetingService implements IGreetingService {
  public async sayHello(name: string): Promise<{ id: string; message: string; created_at: Date }> {
    if (!name || name.trim() === "") {
      throw new Error("Name cannot be empty");
    }
    return {
      id: "greet-123",
      message: \`Hello, \${name}!\`,
      created_at: new Date()
    };
  }
}
`
          },
          {
            path: "tests/GreetingService.test.ts",
            content: `import { GreetingService } from "../src/services/GreetingService";

describe("GreetingService", () => {
  let service: GreetingService;

  beforeEach(() => {
    service = new GreetingService();
  });

  it("should return greeting message", async () => {
    const result = await service.sayHello("World");
    expect(result.message).toBe("Hello, World!");
    expect(result.id).toBe("greet-123");
  });

  it("should throw error on empty name", async () => {
    await expect(service.sayHello("")).rejects.toThrow("Name cannot be empty");
  });
});
`
          }
        ]
      });
    }

    // Default basic fallback mock — dynamic name
    const svcName = this.extractServiceName(rawPrompt);
    return JSON.stringify({
      files: [
        {
          path: `src/services/${svcName}.ts`,
          content: `export class ${svcName} {
  public async execute(input: any): Promise<any> {
    return { success: true };
  }
}
`
        },
        {
          path: `tests/${svcName}.test.ts`,
          content: `import { ${svcName} } from "../src/services/${svcName}";

describe("${svcName}", () => {
  it("should execute successfully", async () => {
    const service = new ${svcName}();
    const result = await service.execute({});
    expect(result.success).toBe(true);
  });
});
`
        }
      ]
    });
  }

  private getMockRustResponse(promptLower: string, rawPrompt: string): string {
    // Checkout service in Rust
    if (promptLower.includes("checkoutservice") || promptLower.includes("\"checkout\"")) {
      return JSON.stringify({
        files: [
          {
            path: "src/services/checkout.rs",
            content: `use thiserror::Error;

#[derive(Error, Debug)]
pub enum CheckoutError {
    #[error("Customer ID is required")]
    MissingCustomerId,
    #[error("Cart is empty")]
    EmptyCart,
}

pub struct CheckoutService;

impl CheckoutService {
    pub fn new() -> Self {
        Self
    }

    /// Place an order for a customer with the given items.
    pub fn place_order(&self, customer_id: &str, items: &[&str]) -> Result<String, CheckoutError> {
        // Precondition: customer_id must not be empty
        if customer_id.trim().is_empty() {
            return Err(CheckoutError::MissingCustomerId);
        }
        // Precondition: items must not be empty
        if items.is_empty() {
            return Err(CheckoutError::EmptyCart);
        }

        // Simulate order placement
        Ok("order-12345".to_string())
    }
}
`
          },
          {
            path: "tests/checkout_test.rs",
            content: `use omni_build::services::checkout::*;

#[test]
fn test_place_order_success() {
    let service = CheckoutService::new();
    let result = service.place_order("cust_999", &["prod_1"]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "order-12345");
}

#[test]
fn test_place_order_missing_customer_id() {
    let service = CheckoutService::new();
    let result = service.place_order("", &["prod_1"]);
    assert!(result.is_err());
}

#[test]
fn test_place_order_empty_cart() {
    let service = CheckoutService::new();
    let result = service.place_order("cust_999", &[]);
    assert!(result.is_err());
}
`
          }
        ]
      });
    }

    // Default fallback for Rust — dynamic name
    const svcName = this.extractServiceName(rawPrompt);
    const snakeName = this.toSnakeCase(svcName);
    return JSON.stringify({
      files: [
        {
          path: `src/services/${snakeName}.rs`,
          content: `pub struct ${svcName};\n\nimpl ${svcName} {\n    pub fn new() -> Self { Self }\n\n    pub fn execute(&self) -> Result<String, String> {\n        Ok("success".to_string())\n    }\n}\n`
        },
        {
          path: `tests/${snakeName}_test.rs`,
          content: `use omni_build::services::${snakeName}::*;\n\n#[test]\nfn test_${snakeName}_execute() {\n    let service = ${svcName}::new();\n    let result = service.execute();\n    assert!(result.is_ok());\n}\n`
        }
      ]
    });
  }

  private getMockPythonResponse(promptLower: string, rawPrompt: string): string {
    // Checkout service in Python
    if (promptLower.includes("checkoutservice") || promptLower.includes("\"checkout\"")) {
      return JSON.stringify({
        files: [
          {
            path: "app/services/checkout.py",
            content: `from dataclasses import dataclass
from typing import List


class CheckoutError(Exception):
    """Base error for checkout operations."""
    pass


class MissingCustomerIdError(CheckoutError):
    """Raised when customer ID is missing."""
    pass


class EmptyCartError(CheckoutError):
    """Raised when cart is empty."""
    pass


@dataclass
class OrderResult:
    order_id: str


class CheckoutService:
    def place_order(self, customer_id: str, items: List[str]) -> OrderResult:
        """Place an order for a customer with the given items."""
        # Precondition: customer_id must not be empty
        if not customer_id or not customer_id.strip():
            raise MissingCustomerIdError("Customer ID is required")
        # Precondition: items must not be empty
        if not items:
            raise EmptyCartError("Cart is empty")

        # Simulate order placement
        return OrderResult(order_id="order-12345")
`
          },
          {
            path: "tests/test_checkout.py",
            content: `import pytest
from app.services.checkout import CheckoutService, MissingCustomerIdError, EmptyCartError


class TestCheckoutService:
    def setup_method(self):
        self.service = CheckoutService()

    def test_place_order_success(self):
        result = self.service.place_order("cust_999", ["prod_1"])
        assert result.order_id == "order-12345"

    def test_place_order_missing_customer_id(self):
        with pytest.raises(MissingCustomerIdError):
            self.service.place_order("", ["prod_1"])

    def test_place_order_empty_cart(self):
        with pytest.raises(EmptyCartError):
            self.service.place_order("cust_999", [])
`
          }
        ]
      });
    }

    // Default fallback for Python — dynamic name
    const svcName = this.extractServiceName(rawPrompt);
    const snakeName = this.toSnakeCase(svcName);
    return JSON.stringify({
      files: [
        {
          path: `app/services/${snakeName}.py`,
          content: `class ${svcName}:\n    def execute(self, **kwargs):\n        return {"success": True}\n`
        },
        {
          path: `tests/test_${snakeName}.py`,
          content: `from app.services.${snakeName} import ${svcName}\n\n\ndef test_${snakeName}_execute():\n    service = ${svcName}()\n    result = service.execute()\n    assert result["success"] is True\n`
        }
      ]
    });
  }

  private getMockGoResponse(promptLower: string, rawPrompt: string): string {
    // Checkout service in Go
    if (promptLower.includes("checkoutservice") || promptLower.includes("\"checkout\"")) {
      return JSON.stringify({
        files: [
          {
            path: "services/checkout.go",
            content: `package services

import (
	"errors"
	"strings"
)

var (
	ErrMissingCustomerID = errors.New("customer ID is required")
	ErrEmptyCart         = errors.New("cart is empty")
)

type CheckoutService struct{}

func NewCheckoutService() *CheckoutService {
	return &CheckoutService{}
}

func (s *CheckoutService) PlaceOrder(customerID string, items []string) (string, error) {
	if strings.TrimSpace(customerID) == "" {
		return "", ErrMissingCustomerID
	}
	if len(items) == 0 {
		return "", ErrEmptyCart
	}
	return "order-12345", nil
}
`
          },
          {
            path: "services/checkout_test.go",
            content: `package services

import (
	"testing"
)

func TestPlaceOrderSuccess(t *testing.T) {
	s := NewCheckoutService()
	id, err := s.PlaceOrder("cust_999", []string{"prod_1"})
	if err != nil {
		t.Fatalf("expected no error, got %v", err)
	}
	if id != "order-12345" {
		t.Errorf("expected order-12345, got %s", id)
	}
}

func TestPlaceOrderMissingCustomerID(t *testing.T) {
	s := NewCheckoutService()
	_, err := s.PlaceOrder("", []string{"prod_1"})
	if err == nil {
		t.Fatal("expected error, got nil")
	}
	if err != ErrMissingCustomerID {
		t.Errorf("expected ErrMissingCustomerID, got %v", err)
	}
}

func TestPlaceOrderEmptyCart(t *testing.T) {
	s := NewCheckoutService()
	_, err := s.PlaceOrder("cust_999", []string{})
	if err == nil {
		t.Fatal("expected error, got nil")
	}
	if err != ErrEmptyCart {
		t.Errorf("expected ErrEmptyCart, got %v", err)
	}
}
`
          }
        ]
      });
    }

    // Default fallback for Go — dynamic name. Go requires exported identifiers
    // (types, funcs, Test functions) to be PascalCase, so the identifier name is
    // PascalCased independently of the snake_cased file path.
    const svcName = this.extractServiceName(rawPrompt);
    const snakeName = this.toSnakeCase(svcName);
    const goName = svcName.charAt(0).toUpperCase() + svcName.slice(1);
    return JSON.stringify({
      files: [
        {
          path: `services/${snakeName}.go`,
          content: `package services\n\ntype ${goName} struct{}\n\nfunc New${goName}() *${goName} {\n    return &${goName}{}\n}\n\nfunc (s *${goName}) Execute() (string, error) {\n    return "success", nil\n}\n`
        },
        {
          path: `services/${snakeName}_test.go`,
          content: `package services\n\nimport "testing"\n\nfunc Test${goName}Execute(t *testing.T) {\n    s := New${goName}()\n    res, err := s.Execute()\n    if err != nil {\n        t.Fatalf("unexpected error: %v", err)\n    }\n    if res != "success" {\n        t.Errorf("expected success, got %s", res)\n    }\n}\n`
        }
      ]
    });
  }
}
