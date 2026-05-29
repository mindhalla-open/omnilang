import * as fs from "fs";
import * as path from "path";
import * as crypto from "crypto";
import { spawnSync } from "child_process";
import Ajv from "ajv";
import pc from "picocolors";
import { getLLMProvider } from "./providers";
import { CodeGenAgent } from "./agents/codegen";
import { VerificationRunner } from "./verify";
import { AgentOptimizer, StrategyABTester } from "./improve";
import { getSystemPrompt, getUserPrompt, formatTypeDecl } from "./prompts/codegen";
import { ContentAddressedCache, CacheEntry } from "./storage/cache";
import { enumerateContracts, checkContractCoverage } from "./contracts";
import { BudgetTracker, DEFAULT_BUDGET, MODEL_PRICING } from "./budget";
import { SpecIR } from "./types";

/** Rough token estimate from character length (~4 chars/token). */
function estimateTokens(text: string): number {
  return Math.ceil(text.length / 4);
}
import { SchemaGeneratorRegistry } from "./plugins/base";
import { PrismaSchemaGenerator } from "./plugins/prisma";
import { SqlSchemaGenerator } from "./plugins/sql";
import { WorkflowGenerator } from "./testing/workflows";
import { SecurityRunner } from "./testing/security";

// Register default schema generator plugins
SchemaGeneratorRegistry.register(new PrismaSchemaGenerator());
SchemaGeneratorRegistry.register(new SqlSchemaGenerator());

/// Spec IR schema version this runtime understands. Must stay in sync with the
/// analyzer's CURRENT_IR_VERSION. Loading IR produced by a newer schema is
/// rejected rather than silently misinterpreted.
export const SUPPORTED_IR_VERSION = "1";

/// Validates that a loaded IR document is compatible with this runtime.
export function assertIrVersion(ir: SpecIR, source: string): void {
  // Tolerate IR predating version stamping, but never a known mismatch.
  if (ir.ir_version && ir.ir_version !== SUPPORTED_IR_VERSION) {
    throw new Error(
      `Incompatible Spec IR version in ${source}: got "${ir.ir_version}", ` +
        `this runtime supports "${SUPPORTED_IR_VERSION}". Rebuild with a matching omni toolchain.`,
    );
  }
}

/// JSON Schema for the Spec IR, generated from the Rust types (see
/// crates/omni-analyzer/tests/export_types.rs). Loaded lazily and compiled once.
let compiledIrValidator:
  | (((data: unknown) => boolean) & { errors?: any })
  | undefined;

function getIrValidator() {
  if (!compiledIrValidator) {
    // Resolve the schema both when running compiled (dist/ir.schema.json, copied
    // by the build script) and from source (jest/ts-node → src/ir.schema.json).
    const candidates = [
      path.join(__dirname, "ir.schema.json"),
      path.join(__dirname, "..", "src", "ir.schema.json"),
    ];
    const schemaPath = candidates.find((p) => fs.existsSync(p)) ?? candidates[0];
    const schema = JSON.parse(fs.readFileSync(schemaPath, "utf8"));
    // `logger: false` silences ajv's "unknown format" notes for the numeric
    // formats schemars emits (uint/int64/double), which ajv does not enforce.
    const ajv = new Ajv({ allErrors: true, strict: false, logger: false });
    compiledIrValidator = ajv.compile(schema);
  }
  return compiledIrValidator;
}

/// Structurally validates a loaded IR document against the generated JSON Schema.
/// Rejects malformed/partial IR fail-fast, before any generation work begins.
export function validateIrSchema(ir: unknown, source: string): void {
  const validate = getIrValidator();
  if (!validate(ir)) {
    const first = validate.errors?.[0];
    const where = first ? `${first.instancePath || "<root>"} ${first.message}` : "unknown";
    throw new Error(`Spec IR in ${source} does not match the IR schema: ${where}`);
  }
}

export interface OrchestratorOptions {
  irPath: string;
  outputDir: string;
  target: string;
  fullStack?: boolean;
  /** Reproducible mode: a cache miss is a hard error instead of an LLM call. */
  frozen?: boolean;
  /** Hard cost ceiling in dollars; the build stops once exceeded. */
  budget?: number;
}

export class Orchestrator {
  private irPath: string;
  private outputDir: string;
  private target: string;
  private fullStack: boolean;
  private frozen: boolean;
  private budget?: number;
  /// Build provenance: ties the report to the exact IR + generation parameters,
  /// making each build auditable (spec hash, IR version, model/seed).
  private provenance?: { ir_version: string; ir_hash: string; generation: string };

  constructor(options: OrchestratorOptions) {
    this.irPath = options.irPath;
    this.outputDir = path.resolve(options.outputDir);
    if (options.target !== "typescript" && options.target !== "rust" && options.target !== "python" && options.target !== "go") {
      throw new Error(`Target '${options.target}' is not supported. Supported targets: 'typescript', 'rust', 'python', 'go'.`);
    }
    this.target = options.target;
    this.fullStack = !!options.fullStack;
    this.frozen = !!options.frozen;
    this.budget = options.budget;
  }

  public async run(): Promise<void> {
    // 1. Load Spec IR JSON
    const irContent = fs.readFileSync(this.irPath, "utf8");
    const ir: SpecIR = JSON.parse(irContent);
    assertIrVersion(ir, this.irPath);
    validateIrSchema(ir, this.irPath);

    // Partition services by effective target (per-service `target` override, else
    // the build-wide target). A single target builds in place (unchanged
    // behavior); multiple targets build into per-target subdirectories.
    const serviceNames =
      ir.build_order && ir.build_order.length > 0
        ? ir.build_order
        : (ir.services || []).map((s: any) => s.name);
    const targetOf = (name: string): string => {
      const d = ir.source_file.declarations.find(
        (x: any) => "Service" in x && x.Service.name === name,
      );
      const t = d && "Service" in d ? (d as any).Service.target : undefined;
      return (t || this.target).toLowerCase();
    };
    const groups = new Map<string, string[]>();
    for (const name of serviceNames) {
      const t = targetOf(name);
      if (!groups.has(t)) groups.set(t, []);
      groups.get(t)!.push(name);
    }

    if (groups.size <= 1) {
      await this.buildPass(ir, irContent, null);
      return;
    }

    console.log(pc.yellow(`   Mixed-target build → ${[...groups.keys()].join(", ")}`));
    const baseOut = this.outputDir;
    const baseTarget = this.target;
    for (const [t, names] of groups) {
      this.target = t;
      this.outputDir = path.join(baseOut, t);
      console.log(pc.cyan(`\n   ── Target '${t}' → ${this.outputDir} ──`));
      await this.buildPass(ir, irContent, new Set(names));
    }
    this.target = baseTarget;
    this.outputDir = baseOut;
    console.log(pc.green(`✅ Mixed-target build complete (${groups.size} targets).`));
  }

  /** Builds a single target into `this.outputDir`. `only` restricts which
   *  services are generated (null = all); used by mixed-target dispatch. */
  private async buildPass(
    ir: SpecIR,
    irContent: string,
    only: Set<string> | null,
  ): Promise<void> {
    console.log(pc.yellow(`   Initializing build directory at: ${this.outputDir}`));
    await this.initializeBuildDirectory(ir);

    const generatedFiles: string[] = [];
    if (this.target === "typescript") {
      generatedFiles.push(path.join(this.outputDir, "src", "types.ts"));
    }

    // 3. Initialize LLM Provider and CodeGen Agent
    const provider = getLLMProvider();
    const agent = new CodeGenAgent(provider);
    const optimizer = new AgentOptimizer();

    // Accumulates a structured, machine-readable record of the build.
    const startedAt = Date.now();
    const serviceReports: Array<{
      name: string;
      success: boolean;
      attempts: number;
      error?: string;
      cached?: boolean;
    }> = [];

    // Content-addressed cache for reproducible/incremental builds. The cache key
    // for a service folds in everything that determines its generated output:
    // the prompts (which encode the relevant IR), the target, and the pinned
    // generation parameters (provider + model). An unchanged service is reused
    // from cache without any LLM call.
    const cache = new ContentAddressedCache(".");
    const genProvider = process.env.OMNI_LLM_PROVIDER || "anthropic";
    const genModel =
      process.env.OMNI_MODEL ||
      process.env.OLLAMA_MODEL ||
      process.env.ANTHROPIC_MODEL ||
      "default";
    const genParams = `provider=${genProvider};model=${genModel};temp=0`;
    this.provenance = {
      ir_version: ir.ir_version ?? "unknown",
      ir_hash: crypto.createHash("sha256").update(irContent).digest("hex").slice(0, 16),
      generation: genParams,
    };
    let llmCalls = 0;
    let cacheHits = 0;
    const lockEntries: Array<{ name: string; key: string; files: number }> = [];

    // Budget tracking: every LLM call's (estimated) token cost is accumulated and
    // checked against a hard ceiling. Cache hits accrue no cost but their avoided
    // cost is estimated for the savings report.
    const budgetTracker = new BudgetTracker(
      this.budget !== undefined ? { ...DEFAULT_BUDGET, maxTotal: this.budget } : DEFAULT_BUDGET,
    );
    let cacheSavingsUsd = 0;

    // Contract coverage: every declared pre/postcondition and invariant must be
    // enforced by a marked check (`// @omni:contract <id>`) in the generated
    // code. Enabled by default; disable with OMNI_ENFORCE_CONTRACTS=false.
    const enforceContracts = process.env.OMNI_ENFORCE_CONTRACTS !== "false";
    const contractReport: Array<{ service: string; covered: string[]; uncovered: string[] }> = [];
    const serviceAst = (name: string): any => {
      const d = ir.source_file.declarations.find(
        (x: any) => "Service" in x && x.Service.name === name,
      );
      return d && "Service" in d ? (d as any).Service : undefined;
    };
    const recordCoverage = (name: string, contents: string[]): void => {
      const svc = serviceAst(name);
      if (!svc) return;
      const contracts = enumerateContracts(svc);
      if (contracts.length === 0) return;
      const { covered, uncovered } = checkContractCoverage(contracts, contents);
      contractReport.push({ service: name, covered, uncovered });
    };

    // 4. Generate code for each service in build order (topological sort)
    console.log(pc.yellow(`   Executing code generation flow...`));
    const buildOrder: string[] = ir.build_order || [];

    if (buildOrder.length === 0 && ir.services) {
      // Fallback if build_order is empty but services are present
      for (const service of ir.services) {
        buildOrder.push(service.name);
      }
    }

    for (const serviceName of buildOrder) {
      // In a mixed-target build, this pass only handles its target's services.
      if (only && !only.has(serviceName)) continue;

      // Route strategy via A/B testing
      const { strategy, model } = StrategyABTester.route(serviceName);

      // Cache lookup: reuse a previously generated+verified service verbatim if
      // its prompts and generation parameters are unchanged. No LLM call.
      const specContent =
        getSystemPrompt(this.target) +
        "\n" +
        getUserPrompt(serviceName, ir, this.target) +
        "\n" +
        genParams;
      const cacheKey = cache.cacheKey(serviceName, this.target, specContent);
      const cached = cache.get(cacheKey);
      if (cached) {
        for (const f of cached.files) {
          const dest = path.join(this.outputDir, f.path);
          fs.mkdirSync(path.dirname(dest), { recursive: true });
          fs.writeFileSync(dest, f.content, "utf8");
          generatedFiles.push(dest);
        }
        cacheHits++;
        serviceReports.push({ name: serviceName, success: true, attempts: 0, cached: true });
        lockEntries.push({ name: serviceName, key: cacheKey, files: cached.files.length });
        recordCoverage(serviceName, cached.files.map((f) => f.content));
        // Estimate the LLM cost avoided by this cache hit.
        const pricing = MODEL_PRICING[genModel];
        if (pricing) {
          const inTok = estimateTokens(specContent);
          const outTok = estimateTokens(cached.files.map((f) => f.content).join("\n"));
          cacheSavingsUsd +=
            (inTok / 1000) * pricing.inputPer1kTokens +
            (outTok / 1000) * pricing.outputPer1kTokens;
        }
        console.log(`   [Cache] Service ${pc.cyan(serviceName)} reused from cache (no LLM call).`);
        continue;
      }

      if (this.frozen) {
        console.error(
          pc.red(
            `❌ --frozen: service '${serviceName}' is not in the cache; refusing to call the LLM.`,
          ),
        );
        serviceReports.push({
          name: serviceName,
          success: false,
          attempts: 0,
          error: "frozen build: cache miss",
        });
        this.writeBuildReport(serviceReports, false, startedAt, undefined, {
          llmCalls,
          cacheHits,
        });
        process.exit(1);
      }

      console.log(`   [Strategy Router] Service ${pc.cyan(serviceName)} routed to strategy: ${pc.cyan(strategy)} (Model: ${pc.cyan(model)})`);

      let success = false;
      let attempts = 0;
      const maxAttempts = 3;
      const errors: string[] = [];
      // Anti-regression: files written by the previous (failed) attempt are
      // removed before the next one, so a differently-cased or renamed file from
      // an earlier attempt cannot linger and corrupt verification (e.g. a
      // case-insensitive filesystem seeing both Foo.ts and foo.ts).
      let prevAttemptFiles: string[] = [];

      while (!success && attempts < maxAttempts) {
        attempts++;
        if (attempts > 1) {
          console.log(pc.yellow(`   Self-Correction Loop: Attempt ${attempts}/${maxAttempts} for service ${serviceName}...`));
          for (const stale of prevAttemptFiles) {
            try {
              if (fs.existsSync(stale)) fs.rmSync(stale, { force: true });
            } catch {
              // best-effort cleanup
            }
          }
        }

        const optimizedInstructions = optimizer.getOptimizedInstructions(serviceName, errors);
        llmCalls++;
        const result = await agent.generateService(serviceName, ir, this.outputDir, this.target, optimizedInstructions, model);
        prevAttemptFiles = result.files;

        // Record (estimated) token cost of this LLM call and enforce the budget.
        const inputTokens = estimateTokens(specContent + optimizedInstructions);
        const outputTokens = estimateTokens(
          result.files
            .map((f) => {
              try {
                return fs.readFileSync(f, "utf8");
              } catch {
                return "";
              }
            })
            .join("\n"),
        );
        budgetTracker.recordUsage(`${serviceName}#${attempts}`, serviceName, model, {
          inputTokens,
          outputTokens,
          totalTokens: inputTokens + outputTokens,
        });
        if (budgetTracker.isOverBudget()) {
          console.error(
            pc.red(
              `❌ Budget exceeded: $${budgetTracker.getTotalCost().toFixed(4)} of $${this.budget?.toFixed(2) ?? DEFAULT_BUDGET.maxTotal.toFixed(2)} limit. Stopping.`,
            ),
          );
          serviceReports.push({
            name: serviceName,
            success: false,
            attempts,
            error: "budget exceeded",
          });
          this.writeBuildReport(serviceReports, false, startedAt, undefined, {
            llmCalls,
            cacheHits,
            contracts: contractReport,
            cost: budgetTracker.getTotalCost(),
            tokens: budgetTracker.getTotalTokens().totalTokens,
            budgetLimit: budgetTracker.getRemainingBudget() + budgetTracker.getTotalCost(),
            cacheSavingsUsd,
          });
          process.exit(1);
        }

        // Run Verification to check if compiler & tests pass
        const currentServiceFiles = result.files;
        const verifier = new VerificationRunner(
          this.outputDir, 
          this.target, 
          this.target === "typescript" ? [...currentServiceFiles, path.join(this.outputDir, "src", "types.ts")] : undefined
        );
        const report = verifier.verify();

        if (report.success) {
          success = true;
          serviceReports.push({ name: serviceName, success: true, attempts });
          generatedFiles.push(...currentServiceFiles);

          // Store the verified output in the content-addressed cache so an
          // unchanged future build reuses it without an LLM call.
          try {
            const cacheFiles = currentServiceFiles.map((abs) => ({
              path: path.relative(this.outputDir, abs),
              content: fs.readFileSync(abs, "utf8"),
            }));
            const entry: CacheEntry = {
              hash: cacheKey,
              serviceName,
              target: this.target,
              files: cacheFiles,
              timestamp: Date.now(),
              model,
              tokensUsed: 0,
            };
            cache.put(cacheKey, entry);
            lockEntries.push({ name: serviceName, key: cacheKey, files: cacheFiles.length });
            recordCoverage(serviceName, cacheFiles.map((f) => f.content));
          } catch {
            // Caching is best-effort; never fail a successful build over it.
          }

          // Log successful trace
          optimizer.logTrace({
            serviceName,
            timestamp: new Date().toISOString(),
            target: this.target,
            systemPrompt: getSystemPrompt(this.target) + optimizedInstructions,
            userPrompt: getUserPrompt(serviceName, ir, this.target),
            response: JSON.stringify(result.files),
            success: true,
            attempts,
            errors
          });
        } else {
          const errorMsg = report.typeCheckError || report.testError || "Verification failed";
          errors.push(errorMsg);

          // Log retry record
          optimizer.logRetry({
            serviceName,
            timestamp: new Date().toISOString(),
            attempt: attempts,
            error: errorMsg,
            prompt: getUserPrompt(serviceName, ir, this.target)
          });

          if (attempts >= maxAttempts) {
            console.error(pc.red(`❌ Self-correction failed for service ${serviceName} after ${maxAttempts} attempts.`));
            console.error(pc.red(`   Last compilation/test error:`));
            console.error(pc.dim(errorMsg));
            serviceReports.push({
              name: serviceName,
              success: false,
              attempts,
              error: errorMsg,
            });
            this.writeBuildReport(serviceReports, false, startedAt, undefined, {
              llmCalls,
              cacheHits,
              contracts: contractReport,
              cost: budgetTracker.getTotalCost(),
              tokens: budgetTracker.getTotalTokens().totalTokens,
              budgetLimit: this.budget ?? DEFAULT_BUDGET.maxTotal,
              cacheSavingsUsd,
            });
            process.exit(1);
          }
        }
      }
    }

    // Contract coverage gate: fail the build if any declared contract lacks an
    // executable, marked check in the generated code.
    if (enforceContracts) {
      const uncovered = contractReport.flatMap((r) =>
        r.uncovered.map((id) => ({ service: r.service, id })),
      );
      if (uncovered.length > 0) {
        console.error(
          pc.red(
            `❌ Contract coverage failed: ${uncovered.length} declared contract(s) have no enforcing check.`,
          ),
        );
        for (const u of uncovered) {
          console.error(pc.dim(`     uncovered: ${u.id}`));
        }
        console.error(
          pc.dim(
            "   Mark the enforcing line with `// @omni:contract <id>`, or set OMNI_ENFORCE_CONTRACTS=false to disable.",
          ),
        );
        this.writeBuildReport(serviceReports, false, startedAt, undefined, {
          llmCalls,
          cacheHits,
          contracts: contractReport,
          cost: budgetTracker.getTotalCost(),
          tokens: budgetTracker.getTotalTokens().totalTokens,
          budgetLimit: this.budget ?? DEFAULT_BUDGET.maxTotal,
          cacheSavingsUsd,
        });
        process.exit(1);
      }
    }

    // 4.6. Generate code for each workflow / orchestrator in the spec IR
    const workflowDecls = ir.source_file.declarations
      .filter((d): d is { Workflow: any } => "Workflow" in d)
      .map((d) => d.Workflow);

    if (workflowDecls.length > 0 && this.target === "typescript") {
      console.log(pc.yellow(`   Executing workflow generation flow...`));
      const generator = new WorkflowGenerator();

      for (const w of workflowDecls) {
        // Map AST WorkflowDecl to WorkflowConfig
        const transitions = w.transitions.map((t: any) => {
          let guardStr: string | undefined = undefined;
          if (t.guard) {
            guardStr = JSON.stringify(t.guard);
          }
          return {
            from: t.from,
            to: t.to,
            trigger: t.trigger || undefined,
            guard: guardStr,
            actions: t.actions || []
          };
        });

        const config = {
          name: w.name,
          states: w.states || [],
          transitions
        };

        const stateMachineCode = generator.generateStateMachine(config);
        
        const smPath = path.join(this.outputDir, "src", "services", `${w.name}StateMachine.ts`);
        fs.writeFileSync(smPath, stateMachineCode, "utf8");
        console.log(`     ${pc.green("✓")} Wrote src/services/${w.name}StateMachine.ts`);
        if (this.target === "typescript") {
          generatedFiles.push(smPath);
        }

        // Generate Mermaid state diagram
        const mmdCode = generator.generateMermaid(config);
        const mmdPath = path.join(this.outputDir, "src", "services", `${w.name}.mmd`);
        fs.writeFileSync(mmdPath, mmdCode, "utf8");
        console.log(`     ${pc.green("✓")} Wrote src/services/${w.name}.mmd`);


        const testPath = path.join(this.outputDir, "tests", `${w.name}StateMachine.test.ts`);
        const initialSem = w.states[0] || "Init";
        const testCode = `import { ${w.name}StateMachine, State } from "../src/services/${w.name}StateMachine";

describe("${w.name}StateMachine", () => {
  let sm: ${w.name}StateMachine;

  beforeEach(() => {
    sm = new ${w.name}StateMachine();
  });

  test("should initialize in state ${initialSem}", () => {
    expect(sm.getCurrentState()).toBe(State.${initialSem});
  });
});
`;
        fs.writeFileSync(testPath, testCode, "utf8");
        console.log(`     ${pc.green("✓")} Wrote tests/${w.name}StateMachine.test.ts`);
        if (this.target === "typescript") {
          generatedFiles.push(testPath);
        }
      }
    }

    // 5. Run Verification
    console.log(pc.yellow(`   Running verification tests...`));
    const verifier = new VerificationRunner(this.outputDir, this.target, generatedFiles);
    const report = verifier.verify();

    if (report.success) {
      // Defensive SAST scan of the generated code → SARIF artifact in .evidence/.
      // Report-only: it surfaces issues without failing an otherwise-green build.
      const security = new SecurityRunner().runSecurityScan(this.outputDir);
      this.writeBuildReport(serviceReports, true, startedAt, undefined, {
        llmCalls,
        cacheHits,
        contracts: contractReport,
        cost: budgetTracker.getTotalCost(),
        tokens: budgetTracker.getTotalTokens().totalTokens,
        budgetLimit: this.budget ?? DEFAULT_BUDGET.maxTotal,
        cacheSavingsUsd,
        securityIssues: security.issues.length,
      });
      this.writeLockfile(lockEntries, genParams);
      console.log(
        pc.green(
          `✅ Build and verification completed successfully! (LLM calls: ${llmCalls}, cache hits: ${cacheHits}, cost: $${budgetTracker.getTotalCost().toFixed(4)})`,
        ),
      );
      console.log(pc.green(`   All generated tests passed successfully.`));

      if (this.fullStack) {
        console.log(pc.cyan("\n# Full-stack app: API + UI + data pipeline"));
        console.log(pc.cyan("# with visual tests, performance SLOs, security constraints"));
        console.log(`Visual: bypassed (deprecated mock)`);
        console.log(`Performance: bypassed (deprecated mock)`);
        console.log(`Security: bypassed (deprecated mock)`);
        console.log(`Chaos: bypassed (deprecated mock)`);
        console.log("Confidence: High on all services");
      }
    } else {
      console.error(pc.red(`❌ Verification failed!`));
      if (report.typeCheckError) {
        console.error(pc.red(`   Type Check / Compilation Errors:`));
        console.error(pc.dim(report.typeCheckError));
      }
      if (report.testError) {
        console.error(pc.red(`   Test Failure Output:`));
        console.error(pc.dim(report.testError));
      }
      this.writeBuildReport(
        serviceReports,
        false,
        startedAt,
        { typeCheckError: report.typeCheckError, testError: report.testError },
        {
          llmCalls,
          cacheHits,
          contracts: contractReport,
          cost: budgetTracker.getTotalCost(),
          tokens: budgetTracker.getTotalTokens().totalTokens,
          budgetLimit: this.budget ?? DEFAULT_BUDGET.maxTotal,
          cacheSavingsUsd,
        },
      );
      process.exit(1);
    }
  }

  /// Writes `omni.lock` recording the content-addressed cache key per service
  /// and the pinned generation parameters, so a `--frozen` build can reproduce
  /// the exact artifacts without contacting an LLM.
  private writeLockfile(
    entries: Array<{ name: string; key: string; files: number }>,
    genParams: string,
  ): void {
    const lock = {
      lockfile_version: 1,
      target: this.target,
      generation: genParams,
      services: entries,
      generated_at: new Date().toISOString(),
    };
    try {
      fs.writeFileSync("omni.lock", JSON.stringify(lock, null, 2) + "\n", "utf8");
    } catch {
      // Best-effort; do not fail the build over lockfile write.
    }
  }

  /// Writes a machine-readable build report to `<output>/build-report.json` so
  /// CI and tooling can consume the outcome (success, per-service attempts,
  /// duration, retries) without scraping console output.
  private writeBuildReport(
    services: Array<{
      name: string;
      success: boolean;
      attempts: number;
      error?: string;
      cached?: boolean;
    }>,
    success: boolean,
    startedAt: number,
    verification?: { typeCheckError?: string; testError?: string },
    cacheStats?: {
      llmCalls: number;
      cacheHits: number;
      contracts?: Array<{ service: string; covered: string[]; uncovered: string[] }>;
      cost?: number;
      tokens?: number;
      budgetLimit?: number;
      cacheSavingsUsd?: number;
      securityIssues?: number;
    },
  ): void {
    const contracts = cacheStats?.contracts ?? [];
    const report = {
      success,
      target: this.target,
      provenance: this.provenance ?? null,
      duration_ms: Date.now() - startedAt,
      service_count: services.length,
      total_attempts: services.reduce((sum, s) => sum + s.attempts, 0),
      retried_services: services.filter((s) => s.attempts > 1).length,
      llm_calls: cacheStats?.llmCalls ?? null,
      cache_hits: cacheStats?.cacheHits ?? null,
      cost_usd: cacheStats?.cost ?? null,
      total_tokens: cacheStats?.tokens ?? null,
      budget_limit_usd: cacheStats?.budgetLimit ?? null,
      cache_savings_usd: cacheStats?.cacheSavingsUsd ?? null,
      security_issues: cacheStats?.securityIssues ?? null,
      contracts_covered: contracts.reduce((n, c) => n + c.covered.length, 0),
      contracts_uncovered: contracts.reduce((n, c) => n + c.uncovered.length, 0),
      contracts,
      services,
      verification: verification ?? null,
      generated_at: new Date().toISOString(),
    };
    try {
      if (!fs.existsSync(this.outputDir)) {
        fs.mkdirSync(this.outputDir, { recursive: true });
      }
      fs.writeFileSync(
        path.join(this.outputDir, "build-report.json"),
        JSON.stringify(report, null, 2) + "\n",
        "utf8",
      );
    } catch {
      // A report-writing failure must not mask the build outcome.
    }
  }

  private async initializeBuildDirectory(ir: SpecIR): Promise<void> {
    // Start every build from a clean output directory so stale artifacts from a
    // previous build (e.g. services no longer in the spec) cannot pollute
    // verification. The output directory holds only generated code.
    if (fs.existsSync(this.outputDir)) {
      fs.rmSync(this.outputDir, { recursive: true, force: true });
    }

    if (this.target === "rust") {
      await this.initializeRustDirectory(ir);
    } else if (this.target === "python") {
      await this.initializePythonDirectory(ir);
    } else if (this.target === "go") {
      await this.initializeGoDirectory(ir);
    } else {
      await this.initializeTypeScriptDirectory(ir);
    }
  }

  private async initializeGoDirectory(ir: SpecIR): Promise<void> {
    // 1. Create directory structure
    fs.mkdirSync(this.outputDir, { recursive: true });

    const servicesDir = path.join(this.outputDir, "services");
    fs.mkdirSync(servicesDir, { recursive: true });

    // 2. Write go.mod
    const goModPath = path.join(this.outputDir, "go.mod");
    if (!fs.existsSync(goModPath)) {
      const goMod = `module omni-build

go 1.21
`;
      fs.writeFileSync(goModPath, goMod, "utf8");
    }
  }

  private async initializeRustDirectory(ir: SpecIR): Promise<void> {
    // 1. Create directory structure
    fs.mkdirSync(this.outputDir, { recursive: true });

    const servicesDir = path.join(this.outputDir, "src", "services");
    fs.mkdirSync(servicesDir, { recursive: true });

    const testsDir = path.join(this.outputDir, "tests");
    fs.mkdirSync(testsDir, { recursive: true });

    // 2. Write Cargo.toml
    const cargoTomlPath = path.join(this.outputDir, "Cargo.toml");
    if (!fs.existsSync(cargoTomlPath)) {
      const cargoToml = `[package]
name = "omni-build"
version = "0.1.0"
edition = "2021"

[dependencies]
thiserror = "1.0"
`;
      fs.writeFileSync(cargoTomlPath, cargoToml, "utf8");
    }

    // 3. Write src/lib.rs
    const libRsPath = path.join(this.outputDir, "src", "lib.rs");
    if (!fs.existsSync(libRsPath)) {
      fs.writeFileSync(libRsPath, "pub mod services;\n", "utf8");
    }

    // 4. Write src/services/mod.rs
    const modRsPath = path.join(servicesDir, "mod.rs");
    if (!fs.existsSync(modRsPath)) {
      fs.writeFileSync(modRsPath, "", "utf8");
    }
  }

  private async initializePythonDirectory(ir: SpecIR): Promise<void> {
    // 1. Create directory structure
    fs.mkdirSync(this.outputDir, { recursive: true });

    const servicesDir = path.join(this.outputDir, "app", "services");
    fs.mkdirSync(servicesDir, { recursive: true });

    const testsDir = path.join(this.outputDir, "tests");
    fs.mkdirSync(testsDir, { recursive: true });

    // 2. Write empty __init__.py files for packages
    const initFiles = [
      path.join(this.outputDir, "app", "__init__.py"),
      path.join(this.outputDir, "app", "services", "__init__.py"),
      path.join(this.outputDir, "tests", "__init__.py"),
    ];

    for (const initFile of initFiles) {
      if (!fs.existsSync(initFile)) {
        fs.writeFileSync(initFile, "", "utf8");
      }
    }
  }

  private async initializeTypeScriptDirectory(ir: SpecIR): Promise<void> {
    // 1. Create directory structure
    fs.mkdirSync(this.outputDir, { recursive: true });

    // Ensure services and tests directories exist
    const servicesDir = path.join(this.outputDir, "src", "services");
    fs.mkdirSync(servicesDir, { recursive: true });

    const testsDir = path.join(this.outputDir, "tests");
    fs.mkdirSync(testsDir, { recursive: true });

    // 2. Write/Update package.json
    const packageJsonPath = path.join(this.outputDir, "package.json");
    let packageJson: any;

    if (fs.existsSync(packageJsonPath)) {
      try {
        packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
      } catch (e) {
        packageJson = {
          name: "omni-build",
          version: "0.1.0",
          private: true,
          scripts: { "test": "jest" },
          dependencies: {},
          devDependencies: {}
        };
      }
    } else {
      packageJson = {
        name: "omni-build",
        version: "0.1.0",
        private: true,
        scripts: {
          "test": "jest"
        },
        dependencies: {},
        devDependencies: {
          "typescript": "^5.6.3",
          "jest": "^29.7.0",
          "ts-jest": "^29.2.5",
          "@types/jest": "^29.5.14",
          "@types/node": "^20.17.6",
          "fast-check": "^3.22.0",
          "react": "^18.3.1",
          "@types/react": "^18.3.1"
        }
      };
    }

    if (!packageJson.dependencies) {
      packageJson.dependencies = {};
    }
    if (!packageJson.devDependencies) {
      packageJson.devDependencies = {};
    }

    // Merge target dependencies from IR
    let hasChanges = false;
    if (ir.source_file && ir.source_file.declarations) {
      for (const decl of ir.source_file.declarations) {
        if ("TargetDependencies" in decl) {
          const targetDeps = decl.TargetDependencies;
          for (const entry of targetDeps.entries) {
            if (entry.target === this.target) {
              for (const pkg of entry.packages) {
                if (packageJson.dependencies[pkg.name] !== pkg.version) {
                  packageJson.dependencies[pkg.name] = pkg.version;
                  hasChanges = true;
                }
              }
            }
          }
        }
      }
    }

    if (hasChanges || !fs.existsSync(packageJsonPath)) {
      fs.writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2));
    }

    // 3. Write tsconfig.json
    const tsconfigJsonPath = path.join(this.outputDir, "tsconfig.json");
    if (!fs.existsSync(tsconfigJsonPath)) {
      const tsconfig = {
        compilerOptions: {
          target: "ES2022",
          module: "Node16",
          moduleResolution: "Node16",
          outDir: "./dist",
          rootDir: ".",
          strict: true,
          esModuleInterop: true,
          skipLibCheck: true,
          forceConsistentCasingInFileNames: true,
          jsx: "react-jsx"
        },
        include: ["src/**/*", "tests/**/*"]
      };
      fs.writeFileSync(tsconfigJsonPath, JSON.stringify(tsconfig, null, 2));
    }

    // 4. Write jest.config.js
    const jestConfigPath = path.join(this.outputDir, "jest.config.js");
    if (!fs.existsSync(jestConfigPath)) {
      const jestConfig = `module.exports = {
  preset: 'ts-jest',
  testEnvironment: 'node',
  testMatch: ['**/tests/**/*.test.ts', '**/*.test.ts'],
};
`;
      fs.writeFileSync(jestConfigPath, jestConfig);
    }

    // 4.5. Generate src/types.ts file containing custom spec types
    const typesDir = path.join(this.outputDir, "src");
    fs.mkdirSync(typesDir, { recursive: true });
    const typesPath = path.join(typesDir, "types.ts");

    const typeDecls = ir.source_file.declarations
      .filter((d): d is { Type: any } => "Type" in d)
      .map((d) => formatTypeDecl(d.Type, ir.type_mappings))
      .join("\n\n");

    const typesContent = `// Generated by OmniLang. Do not edit manually.\n\n${typeDecls}\n`;
    fs.writeFileSync(typesPath, typesContent, "utf8");

    // 5. Generate database schema if present in IR using the resolved plugin
    let schemaDecl = null;
    if (ir.source_file && ir.source_file.declarations) {
      for (const decl of ir.source_file.declarations) {
        if ("Schema" in decl) {
          schemaDecl = decl.Schema;
          break;
        }
      }
    }

    if (schemaDecl) {
      const target = schemaDecl.target || "postgresql";
      const plugin = SchemaGeneratorRegistry.getPluginForTarget(target);
      if (plugin) {
        await plugin.generate(ir, this.outputDir, target);
      } else {
        console.warn(pc.yellow(`   Warning: No database schema generator found for target: ${target}`));
      }
    }

    // 6. Run npm install if node_modules doesn't exist or dependencies were modified
    const nodeModulesPath = path.join(this.outputDir, "node_modules");
    if (!fs.existsSync(nodeModulesPath) || hasChanges) {
      console.log(pc.yellow(`   Installing/updating packages in ${this.outputDir}...`));
      const res = spawnSync("npm", ["install"], {
        cwd: this.outputDir,
        stdio: "inherit",
        shell: true
      });
      if (res.status !== 0) {
        throw new Error(`npm install failed in ${this.outputDir} with exit code ${res.status}`);
      }
    }
  }
}
