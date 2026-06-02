import * as fs from "fs";
import * as path from "path";

export interface TraceLog {
  serviceName: string;
  timestamp: string;
  target: string;
  systemPrompt: string;
  userPrompt: string;
  response: string;
  success: boolean;
  attempts: number;
  errors: string[];
}

export interface RetryRecord {
  serviceName: string;
  timestamp: string;
  attempt: number;
  error: string;
  prompt: string;
}

/// Structured category of a verification failure, derived deterministically from
/// the diagnostic text. Replaces ad-hoc substring sniffing scattered in prompts.
export type FailureCategory =
  | "contract"
  | "type"
  | "import"
  | "test"
  | "syntax"
  | "timeout"
  | "unknown";

/// Classifies a verification error into a single, stable category. Order matters:
/// more specific signals are checked before generic ones.
export function classifyFailure(error: string): FailureCategory {
  const e = error.toLowerCase();
  if (e.includes("contract coverage") || e.includes("@omni:contract")) return "contract";
  if (e.includes("timeout") || e.includes("timed out")) return "timeout";
  if (
    e.includes("has no exported member") ||
    e.includes("cannot find name") ||
    e.includes("cannot find module") ||
    e.includes("is not defined")
  ) {
    return "import";
  }
  // tsc type errors look like `error TS####:`
  if (/error ts\d+/.test(e) || e.includes("type ") || e.includes("not assignable")) return "type";
  // Syntax is checked before tests because "unexpected" contains "expect".
  if (e.includes("syntax") || e.includes("parse") || e.includes("unexpected")) return "syntax";
  if (e.includes("test") || e.includes("expect") || e.includes("assert")) return "test";
  return "unknown";
}

export interface TraceSummary {
  totalTraces: number;
  succeeded: number;
  failed: number;
  retriedServices: number;
  avgAttempts: number;
  byCategory: Record<string, number>;
}

export class AgentOptimizer {
  private cacheDir: string;
  private tracesDir: string;
  private retriesFile: string;

  constructor(cacheDir: string = ".omni-cache") {
    this.cacheDir = path.resolve(cacheDir);
    this.tracesDir = path.join(this.cacheDir, "traces");
    this.retriesFile = path.join(this.tracesDir, "retries.json");
    this.ensureDirectories();
  }

  private ensureDirectories() {
    if (!fs.existsSync(this.cacheDir)) {
      fs.mkdirSync(this.cacheDir, { recursive: true });
    }
    if (!fs.existsSync(this.tracesDir)) {
      fs.mkdirSync(this.tracesDir, { recursive: true });
    }
  }

  public logTrace(trace: TraceLog): void {
    const tracePath = path.join(this.tracesDir, `${trace.serviceName}_${Date.now()}.json`);
    fs.writeFileSync(tracePath, JSON.stringify(trace, null, 2), "utf8");
  }

  /// Aggregates recorded traces + retries into a summary: success/failure
  /// counts, average attempts, and a breakdown of failures by category. Powers
  /// `omni agents benchmark` and surfaces self-correction convergence.
  public summarizeTraces(): TraceSummary {
    const summary: TraceSummary = {
      totalTraces: 0,
      succeeded: 0,
      failed: 0,
      retriedServices: 0,
      avgAttempts: 0,
      byCategory: {},
    };
    if (!fs.existsSync(this.tracesDir)) return summary;

    const traceFiles = fs
      .readdirSync(this.tracesDir)
      .filter((f) => f.endsWith(".json") && f !== "retries.json");

    let attemptsSum = 0;
    const bump = (err: string) => {
      const cat = classifyFailure(err);
      summary.byCategory[cat] = (summary.byCategory[cat] ?? 0) + 1;
    };

    for (const f of traceFiles) {
      try {
        const t: TraceLog = JSON.parse(fs.readFileSync(path.join(this.tracesDir, f), "utf8"));
        summary.totalTraces++;
        if (t.success) summary.succeeded++;
        else summary.failed++;
        if (t.attempts > 1) summary.retriedServices++;
        attemptsSum += t.attempts || 0;
        for (const err of t.errors ?? []) bump(err);
      } catch {
        // skip malformed trace
      }
    }

    if (fs.existsSync(this.retriesFile)) {
      try {
        const retries: RetryRecord[] = JSON.parse(fs.readFileSync(this.retriesFile, "utf8"));
        for (const r of retries) bump(r.error);
      } catch {
        // ignore
      }
    }

    summary.avgAttempts =
      summary.totalTraces > 0 ? attemptsSum / summary.totalTraces : 0;
    return summary;
  }

  public logRetry(retry: RetryRecord): void {
    let retries: RetryRecord[] = [];
    if (fs.existsSync(this.retriesFile)) {
      try {
        retries = JSON.parse(fs.readFileSync(this.retriesFile, "utf8"));
      } catch (e) {
        retries = [];
      }
    }
    retries.push(retry);
    fs.writeFileSync(this.retriesFile, JSON.stringify(retries, null, 2), "utf8");
  }

  /**
   * Automatic prompt optimization: analyzing retry history for a service
   * and generating additional instructions to append to the system prompt
   */
  public getOptimizedInstructions(serviceName: string, errorHistory?: string[]): string {
    const instructions: string[] = [];
    const errorsToAnalyze: string[] = errorHistory ? [...errorHistory] : [];

    // Also read from retries.json if any exist for this service
    if (fs.existsSync(this.retriesFile)) {
      try {
        const retries: RetryRecord[] = JSON.parse(fs.readFileSync(this.retriesFile, "utf8"));
        const serviceRetries = retries.filter(r => r.serviceName === serviceName);
        for (const r of serviceRetries) {
          errorsToAnalyze.push(r.error);
        }
      } catch (e) {
        // Ignore
      }
    }

    if (errorsToAnalyze.length > 0) {
      instructions.push("\n### IMPORTANT: LLM Prompt Adaptations from past generation retries:");
      instructions.push("The previous compilation/testing attempts failed with the following diagnostics. You MUST fix these errors in your new generation:");
      
      const uniqueErrors = Array.from(new Set(errorsToAnalyze));
      for (const err of uniqueErrors) {
        instructions.push(`\nError Diagnostic:\n\`\`\`\n${err}\n\`\`\``);
      }
      
      instructions.push("\nGeneral Guidelines based on analysis:");
      for (const err of uniqueErrors) {
        const lowerErr = err.toLowerCase();
        if (lowerErr.includes("precondition") || lowerErr.includes("pre-condition") || lowerErr.includes("assert") || lowerErr.includes("constraint")) {
          instructions.push("- Ensure all preconditions and constraints on operation inputs are strictly verified in code and throw appropriate errors on violation.");
        }
        if (lowerErr.includes("postcondition") || lowerErr.includes("post-condition")) {
          instructions.push("- Ensure postconditions are satisfied by returning output structures conforming to the defined schemas and rules.");
        }
        if (lowerErr.includes("syntax") || lowerErr.includes("parse") || lowerErr.includes("compiler") || lowerErr.includes("error")) {
          instructions.push("- Verify target language syntax, imports, and interface/module matching before finalizing files.");
        }
        if (lowerErr.includes("metric") || lowerErr.includes("counter")) {
          instructions.push("- Correctly initialize and increment all required metrics with matching labels/dimensions.");
        }
        if (lowerErr.includes("undefined") || lowerErr.includes("not found")) {
          instructions.push("- Make sure all types and interfaces referenced are imported or declared correctly in scope.");
        }
      }
    }

    return instructions.join("\n");
  }
}

/**
 * A/B Testing framework for routing agent strategies.
 * Evaluates Haiku-only vs. Sonnet-4 vs. Hybrid strategies.
 */
export class StrategyABTester {
  public static route(serviceName: string): { strategy: string; model: string } {
    const model = process.env.OMNI_MODEL || "claude-3-5-sonnet-20241022";
    return { strategy: "Standard Strategy", model };
  }
}
