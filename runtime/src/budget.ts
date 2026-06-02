import { SpecIR } from "./types";

/**
 * Budget System — token tracking, cost accumulation, and budget enforcement.
 *
 * Provides per-task, per-service, and total build cost tracking with
 * configurable limits and model tier selection.
 */

// ── Model tiers ──────────────────────────────────────────
export type ModelTier = "CheapFast" | "Balanced" | "SmartExpensive";

export interface ModelPricing {
  tier: ModelTier;
  model: string;
  inputPer1kTokens: number;
  outputPer1kTokens: number;
}

export const MODEL_PRICING: Record<string, ModelPricing> = {
  "claude-3-5-haiku-20241022": {
    tier: "CheapFast",
    model: "claude-3-5-haiku-20241022",
    inputPer1kTokens: 0.001,
    outputPer1kTokens: 0.005,
  },
  "claude-sonnet-4-20250514": {
    tier: "Balanced",
    model: "claude-sonnet-4-20250514",
    inputPer1kTokens: 0.003,
    outputPer1kTokens: 0.015,
  },
  "claude-3-5-sonnet-20241022": {
    tier: "Balanced",
    model: "claude-3-5-sonnet-20241022",
    inputPer1kTokens: 0.003,
    outputPer1kTokens: 0.015,
  },
  "claude-opus-4-20250514": {
    tier: "SmartExpensive",
    model: "claude-opus-4-20250514",
    inputPer1kTokens: 0.015,
    outputPer1kTokens: 0.075,
  },
  "qwen2.5-coder:7b": {
    tier: "CheapFast",
    model: "qwen2.5-coder:7b",
    inputPer1kTokens: 0.0,
    outputPer1kTokens: 0.0,
  },
  "qwen2.5-coder:latest": {
    tier: "CheapFast",
    model: "qwen2.5-coder:latest",
    inputPer1kTokens: 0.0,
    outputPer1kTokens: 0.0,
  },
};

/** A representative model for each tier (first match in the pricing table). */
export function modelForTier(tier: ModelTier): string {
  const match = Object.values(MODEL_PRICING).find((m) => m.tier === tier);
  return match ? match.model : "claude-3-5-sonnet-20241022";
}

// ── Budget configuration ─────────────────────────────────
export interface BudgetConfig {
  /** Maximum total cost in dollars. Build aborts if exceeded. */
  maxTotal: number;
  /** Alert threshold in dollars. Warning emitted when reached. */
  alertAt: number;
  /** Preferred model tier */
  preferredTier: ModelTier;
  /** Whether to allow automatic model escalation */
  allowEscalation: boolean;
}

export const DEFAULT_BUDGET: BudgetConfig = {
  maxTotal: 10.0,
  alertAt: 5.0,
  preferredTier: "Balanced",
  allowEscalation: true,
};

// ── Token tracking ───────────────────────────────────────
export interface TokenUsage {
  inputTokens: number;
  outputTokens: number;
  totalTokens: number;
}

export interface TaskCost {
  taskId: string;
  serviceName: string;
  model: string;
  tokens: TokenUsage;
  cost: number;
  timestamp: number;
}

export interface ServiceCost {
  serviceName: string;
  tasks: TaskCost[];
  totalTokens: TokenUsage;
  totalCost: number;
}

// ── Budget tracker ───────────────────────────────────────
export class BudgetTracker {
  private config: BudgetConfig;
  private tasks: TaskCost[] = [];
  private alertEmitted = false;

  constructor(config: BudgetConfig = DEFAULT_BUDGET) {
    this.config = config;
  }

  /** Record token usage for a task */
  recordUsage(
    taskId: string,
    serviceName: string,
    model: string,
    tokens: TokenUsage
  ): TaskCost {
    const pricing = MODEL_PRICING[model];
    const cost = pricing
      ? (tokens.inputTokens / 1000) * pricing.inputPer1kTokens +
        (tokens.outputTokens / 1000) * pricing.outputPer1kTokens
      : 0;

    const entry: TaskCost = {
      taskId,
      serviceName,
      model,
      tokens,
      cost,
      timestamp: Date.now(),
    };

    this.tasks.push(entry);
    return entry;
  }

  /** Get total cost so far */
  getTotalCost(): number {
    return this.tasks.reduce((sum, t) => sum + t.cost, 0);
  }

  /** Get total token usage */
  getTotalTokens(): TokenUsage {
    return this.tasks.reduce(
      (acc, t) => ({
        inputTokens: acc.inputTokens + t.tokens.inputTokens,
        outputTokens: acc.outputTokens + t.tokens.outputTokens,
        totalTokens: acc.totalTokens + t.tokens.totalTokens,
      }),
      { inputTokens: 0, outputTokens: 0, totalTokens: 0 }
    );
  }

  /** Get per-service cost breakdown */
  getServiceBreakdown(): ServiceCost[] {
    const byService = new Map<string, TaskCost[]>();

    for (const task of this.tasks) {
      if (!byService.has(task.serviceName)) {
        byService.set(task.serviceName, []);
      }
      byService.get(task.serviceName)!.push(task);
    }

    return Array.from(byService.entries()).map(([name, tasks]) => ({
      serviceName: name,
      tasks,
      totalTokens: tasks.reduce(
        (acc, t) => ({
          inputTokens: acc.inputTokens + t.tokens.inputTokens,
          outputTokens: acc.outputTokens + t.tokens.outputTokens,
          totalTokens: acc.totalTokens + t.tokens.totalTokens,
        }),
        { inputTokens: 0, outputTokens: 0, totalTokens: 0 }
      ),
      totalCost: tasks.reduce((sum, t) => sum + t.cost, 0),
    }));
  }

  /** Check if budget limit is exceeded */
  isOverBudget(): boolean {
    return this.getTotalCost() >= this.config.maxTotal;
  }

  /** Check if alert threshold is reached */
  shouldAlert(): boolean {
    if (this.alertEmitted) return false;
    if (this.getTotalCost() >= this.config.alertAt) {
      this.alertEmitted = true;
      return true;
    }
    return false;
  }

  /** Get remaining budget */
  getRemainingBudget(): number {
    return Math.max(0, this.config.maxTotal - this.getTotalCost());
  }

  /** Select the best model tier within remaining budget */
  selectModelTier(): ModelTier {
    const remaining = this.getRemainingBudget();

    // Downgrade to the cheapest tier when the budget is nearly exhausted, so a
    // build degrades gracefully instead of overshooting the limit.
    if (remaining < 0.5) return "CheapFast";
    if (!this.config.allowEscalation) return this.config.preferredTier;

    return this.config.preferredTier;
  }

  /** The model to use given the currently-selected tier. */
  selectModel(): string {
    return modelForTier(this.selectModelTier());
  }

  /** Generate a cost report */
  generateReport(): string {
    const lines: string[] = [];
    const total = this.getTotalCost();
    const tokens = this.getTotalTokens();

    lines.push("╔════════════════════════════════════════════╗");
    lines.push("║           BUILD COST REPORT                ║");
    lines.push("╠════════════════════════════════════════════╣");
    lines.push(`║ Total Cost:     $${total.toFixed(4).padStart(10)}`);
    lines.push(`║ Budget Limit:   $${this.config.maxTotal.toFixed(2).padStart(10)}`);
    lines.push(`║ Remaining:      $${this.getRemainingBudget().toFixed(4).padStart(10)}`);
    lines.push(`║ Input Tokens:   ${tokens.inputTokens.toString().padStart(11)}`);
    lines.push(`║ Output Tokens:  ${tokens.outputTokens.toString().padStart(11)}`);
    lines.push("╠════════════════════════════════════════════╣");

    const services = this.getServiceBreakdown();
    for (const svc of services) {
      lines.push(`║ ${svc.serviceName.padEnd(20)} $${svc.totalCost.toFixed(4).padStart(8)} (${svc.tasks.length} tasks)`);
    }

    lines.push("╚════════════════════════════════════════════╝");

    return lines.join("\n");
  }
}

// ── Cost estimation ──────────────────────────────────────

/** Estimate the cost of building from a spec IR */
export function estimateBuildCost(
  ir: SpecIR,
  tier: ModelTier = "Balanced"
): { estimatedCost: number; estimatedTokens: number; model: string } {
  const serviceCount = ir.services?.length || 0;
  const operationCount = ir.stats?.operation_count || 0;
  const typeCount = ir.stats?.type_count || 0;
  const testCount = ir.stats?.test_count || 0;

  // Rough estimates: ~2K tokens input per service, ~4K output per service
  const tokensPerService = 2000 + operationCount * 500;
  const outputPerService = 4000 + testCount * 300;
  const totalInput = serviceCount * tokensPerService + typeCount * 200;
  const totalOutput = serviceCount * outputPerService;

  // Find model for tier
  const model = Object.values(MODEL_PRICING).find((m) => m.tier === tier);
  if (!model) {
    return { estimatedCost: 0, estimatedTokens: totalInput + totalOutput, model: "unknown" };
  }

  const cost =
    (totalInput / 1000) * model.inputPer1kTokens +
    (totalOutput / 1000) * model.outputPer1kTokens;

  return {
    estimatedCost: Math.round(cost * 10000) / 10000,
    estimatedTokens: totalInput + totalOutput,
    model: model.model,
  };
}
