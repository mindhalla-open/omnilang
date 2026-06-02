import {
  BudgetTracker,
  estimateBuildCost,
  modelForTier,
  DEFAULT_BUDGET,
  ModelPricing,
} from "../src/budget";
import { SpecIR } from "../src/types";

describe("model tiering", () => {
  it("maps each tier to a model in the pricing table", () => {
    expect(modelForTier("CheapFast")).toBeTruthy();
    expect(modelForTier("Balanced")).toBeTruthy();
    expect(modelForTier("SmartExpensive")).toBe("claude-opus-4-20250514");
  });

  it("downgrades to the cheapest tier when the budget is nearly exhausted", () => {
    const tracker = new BudgetTracker({ ...DEFAULT_BUDGET, maxTotal: 1.0 });
    tracker.recordUsage("t1", "S", "claude-opus-4-20250514", {
      inputTokens: 40000,
      outputTokens: 6000,
      totalTokens: 46000,
    });
    // Heavy spend against a $1.00 budget → remaining < 0.5 → CheapFast.
    expect(tracker.selectModelTier()).toBe("CheapFast");
    expect(tracker.selectModel()).toBe(modelForTier("CheapFast"));
  });
});

describe("BudgetTracker", () => {
  test("should record token usage and calculate cost for sonnet", () => {
    const tracker = new BudgetTracker();
    const usage = {
      inputTokens: 2000,
      outputTokens: 1000,
      totalTokens: 3000,
    };

    const task = tracker.recordUsage(
      "task-1",
      "UserService",
      "claude-3-5-sonnet-20241022",
      usage
    );

    expect(task.taskId).toBe("task-1");
    expect(task.serviceName).toBe("UserService");
    expect(task.model).toBe("claude-3-5-sonnet-20241022");
    // Cost calculation: (2000 / 1000) * 0.003 + (1000 / 1000) * 0.015 = 2 * 0.003 + 1 * 0.015 = 0.006 + 0.015 = 0.021
    expect(task.cost).toBeCloseTo(0.021);
    expect(tracker.getTotalCost()).toBeCloseTo(0.021);

    const totalTokens = tracker.getTotalTokens();
    expect(totalTokens.inputTokens).toBe(2000);
    expect(totalTokens.outputTokens).toBe(1000);
    expect(totalTokens.totalTokens).toBe(3000);
  });

  test("should support zero pricing models like Qwen", () => {
    const tracker = new BudgetTracker();
    const usage = {
      inputTokens: 5000,
      outputTokens: 5000,
      totalTokens: 10000,
    };

    const task = tracker.recordUsage(
      "task-2",
      "BillingService",
      "qwen2.5-coder:7b",
      usage
    );

    expect(task.cost).toBe(0.0);
    expect(tracker.getTotalCost()).toBe(0.0);
  });

  test("should aggregate costs per service", () => {
    const tracker = new BudgetTracker();
    tracker.recordUsage("task-1", "UserService", "claude-3-5-sonnet-20241022", {
      inputTokens: 1000,
      outputTokens: 500,
      totalTokens: 1500,
    }); // (1 * 0.003) + (0.5 * 0.015) = 0.003 + 0.0075 = 0.0105

    tracker.recordUsage("task-2", "BillingService", "claude-3-5-sonnet-20241022", {
      inputTokens: 2000,
      outputTokens: 2000,
      totalTokens: 4000,
    }); // (2 * 0.003) + (2 * 0.015) = 0.006 + 0.03 = 0.036

    tracker.recordUsage("task-3", "UserService", "qwen2.5-coder:7b", {
      inputTokens: 10000,
      outputTokens: 10000,
      totalTokens: 20000,
    }); // 0.0

    const breakdown = tracker.getServiceBreakdown();
    expect(breakdown).toHaveLength(2);

    const userSvc = breakdown.find((s) => s.serviceName === "UserService");
    const billingSvc = breakdown.find((s) => s.serviceName === "BillingService");

    expect(userSvc).toBeDefined();
    expect(billingSvc).toBeDefined();

    expect(userSvc!.totalCost).toBeCloseTo(0.0105);
    expect(userSvc!.tasks).toHaveLength(2);
    expect(userSvc!.totalTokens.totalTokens).toBe(21500);

    expect(billingSvc!.totalCost).toBeCloseTo(0.036);
    expect(billingSvc!.tasks).toHaveLength(1);
  });

  test("should check limits and alert thresholds correctly", () => {
    const customConfig = {
      maxTotal: 0.1,
      alertAt: 0.05,
      preferredTier: "Balanced" as const,
      allowEscalation: true,
    };

    const tracker = new BudgetTracker(customConfig);

    // Initial state
    expect(tracker.isOverBudget()).toBe(false);
    expect(tracker.shouldAlert()).toBe(false);
    expect(tracker.getRemainingBudget()).toBeCloseTo(0.1);

    // Add usage below alert
    tracker.recordUsage("task-1", "Svc", "claude-3-5-sonnet-20241022", {
      inputTokens: 1000, // $0.003
      outputTokens: 1000, // $0.015
      totalTokens: 2000,
    }); // $0.018 total

    expect(tracker.isOverBudget()).toBe(false);
    expect(tracker.shouldAlert()).toBe(false);
    expect(tracker.getRemainingBudget()).toBeCloseTo(0.082);

    // Add usage to trigger alert but not limit
    tracker.recordUsage("task-2", "Svc", "claude-3-5-sonnet-20241022", {
      inputTokens: 2000, // $0.006
      outputTokens: 2000, // $0.03
      totalTokens: 4000,
    }); // $0.036 + $0.018 = $0.054 total

    expect(tracker.isOverBudget()).toBe(false);
    expect(tracker.shouldAlert()).toBe(true); // first check, triggers alert
    expect(tracker.shouldAlert()).toBe(false); // second check, alert already emitted

    // Add usage to exceed budget limit
    tracker.recordUsage("task-3", "Svc", "claude-3-5-sonnet-20241022", {
      inputTokens: 3000, // $0.009
      outputTokens: 3000, // $0.045
      totalTokens: 6000,
    }); // $0.054 + $0.054 = $0.108 total

    expect(tracker.isOverBudget()).toBe(true);
    expect(tracker.getRemainingBudget()).toBe(0); // Clamped at 0
  });

  test("should select model tier based on budget limits and remaining budget", () => {
    const tracker = new BudgetTracker({
      maxTotal: 1.0,
      alertAt: 0.8,
      preferredTier: "SmartExpensive",
      allowEscalation: true,
    });

    expect(tracker.selectModelTier()).toBe("SmartExpensive");

    // Spend most of budget, down to < $0.5 remaining
    tracker.recordUsage("task-1", "Svc", "claude-opus-4-20250514", {
      inputTokens: 10000, // $0.15
      outputTokens: 6000,  // $0.45
      totalTokens: 16000,
    }); // $0.60 total. Remaining is $0.40.

    expect(tracker.getRemainingBudget()).toBeCloseTo(0.40);
    // Budget remaining < 0.5 should trigger escalation/fallback to CheapFast
    expect(tracker.selectModelTier()).toBe("CheapFast");
  });

  test("should generate readable build cost reports", () => {
    const tracker = new BudgetTracker();
    tracker.recordUsage("task-1", "UserService", "claude-3-5-sonnet-20241022", {
      inputTokens: 1000,
      outputTokens: 1000,
      totalTokens: 2000,
    });

    const report = tracker.generateReport();
    expect(report).toContain("BUILD COST REPORT");
    expect(report).toContain("Total Cost:");
    expect(report).toContain("UserService");
  });
});

describe("estimateBuildCost", () => {
  test("should estimate build cost from SpecIR stats", () => {
    const mockIr: SpecIR = {
      ir_version: "1",
      module_path: ["test"],
      source_file: {
        module: { path: ["test"], span: { start: 0, end: 0 } },
        imports: [],
        exports: [],
        declarations: [],
      },
      types: [],
      services: [
        {
          name: "UserService",
          goal: null,
          target: null,
          operation_count: 2,
          operation_names: ["getUser", "createUser"],
          constraint_count: 0,
          constraint_names: [],
          dependency_count: 0,
          test_count: 1,
          metric_count: 0,
          metric_names: [],
          confidence: "Medium",
          evidence: []
        },
        {
          name: "BillingService",
          goal: null,
          target: null,
          operation_count: 2,
          operation_names: ["charge", "refund"],
          constraint_count: 0,
          constraint_names: [],
          dependency_count: 0,
          test_count: 1,
          metric_count: 0,
          metric_names: [],
          confidence: "Medium",
          evidence: []
        }
      ],
      build_order: [],
      type_mappings: [],
      stats: {
        type_count: 5,
        service_count: 2,
        operation_count: 4,
        test_count: 2,
        constraint_count: 0,
        metric_count: 0,
        component_count: 0,
        pipeline_count: 0,
        workflow_count: 0,
        agent_count: 0,
        schema_count: 0,
        policy_count: 0,
      },
    };

    // Calculate expected tokens:
    // serviceCount = 2
    // rpcCount = 4, testCount = 2, typeCount = 5
    // tokensPerService = 2000 + 4 * 500 = 4000
    // outputPerService = 4000 + 2 * 300 = 4600
    // totalInput = 2 * 4000 + 5 * 200 = 9000
    // totalOutput = 2 * 4600 = 9200
    // totalEstimatedTokens = 9000 + 9200 = 18200
    // Model for Balanced: claude-3-5-sonnet-20241022 (inputPer1k: 0.003, outputPer1k: 0.015)
    // estimatedCost = (9000 / 1000) * 0.003 + (9200 / 1000) * 0.015 = 9 * 0.003 + 9.2 * 0.015 = 0.027 + 0.138 = 0.165

    const est = estimateBuildCost(mockIr, "Balanced");
    expect(est.estimatedTokens).toBe(18200);
    expect(est.estimatedCost).toBeCloseTo(0.165);
    expect(est.model).toBe("claude-sonnet-4-20250514");
  });

  test("should handle unknown tiers gracefully", () => {
    const mockIr: SpecIR = {
      ir_version: "1",
      module_path: ["test"],
      source_file: {
        module: { path: ["test"], span: { start: 0, end: 0 } },
        imports: [],
        exports: [],
        declarations: [],
      },
      types: [],
      services: [],
      build_order: [],
      type_mappings: [],
      stats: {
        type_count: 0,
        service_count: 0,
        operation_count: 0,
        test_count: 0,
        constraint_count: 0,
        metric_count: 0,
        component_count: 0,
        pipeline_count: 0,
        workflow_count: 0,
        agent_count: 0,
        schema_count: 0,
        policy_count: 0,
      },
    };

    const est = estimateBuildCost(mockIr, "NonExistentTier" as any);
    expect(est.estimatedCost).toBe(0);
    expect(est.model).toBe("unknown");
  });
});
