import { spawnSync } from "child_process";
import * as path from "path";
import * as fs from "fs";

describe("OmniLang End-to-End Integration", () => {
  const rootDir = path.resolve(__dirname, "../..");
  const buildDir = path.resolve(rootDir, "build");

  // Every target's toolchain must be on PATH, or a failure below is a bare
  // "Expected: 0, Received: 1" with the cause buried in a child process. That
  // is how CI stayed red for six weeks in 2026 over a missing pytest. Fail here
  // instead and name the tool. CI installs this exact set before the suite
  // runs (.github/workflows/ci.yml); CONTRIBUTING.md lists it for laptops.
  const REQUIRED_TOOLS: Array<[string, string[], string]> = [
    ["cargo", ["--version"], "Rust toolchain, rustup.rs"],
    ["node", ["--version"], "Node.js 18+"],
    ["go", ["version"], "Go 1.22+, needed for the go target"],
    ["python3", ["-m", "pytest", "--version"], "Python 3 with pytest, needed for the python target"],
  ];

  function assertToolchains(): void {
    const missing = REQUIRED_TOOLS.filter(([bin, args]) => {
      const r = spawnSync(bin, args, { stdio: "ignore" });
      return r.error !== undefined || r.status !== 0;
    }).map(([bin, , what]) => `${bin} (${what})`);
    if (missing.length > 0) {
      throw new Error(`integration suite needs these tools on PATH: ${missing.join(", ")}`);
    }
  }

  /** Runs `omni build <spec> --target <target>` with the mock LLM and prints the
   *  child's output when it fails, so the cause is in the log, not just the code. */
  function omniBuild(spec: string, target: string) {
    const result = spawnSync(
      "cargo",
      ["run", "--bin", "omni", "--", "build", spec, "--target", target],
      { cwd: rootDir, env: { ...process.env, OMNI_MOCK_LLM: "true" } }
    );
    if (result.status !== 0) {
      console.error(`omni build ${spec} --target ${target} exited with ${result.status}`);
      console.error("STDOUT:", result.stdout?.toString());
      console.error("STDERR:", result.stderr?.toString());
    }
    return result;
  }

  beforeAll(() => {
    assertToolchains();
    // Build the Rust omni compiler binary first
    console.log("   [Integration Test] Building Rust compiler...");
    const cargoBuild = spawnSync("cargo", ["build", "--bin", "omni"], {
      cwd: rootDir,
      stdio: "ignore",
    });
    if (cargoBuild.status !== 0) {
      throw new Error("Failed to compile omni binary for integration testing");
    }
  });

  function cleanBuildDir(dir: string, retries = 5, delay = 200) {
    if (!fs.existsSync(dir)) return;
    for (let i = 0; i < retries; i++) {
      try {
        fs.rmSync(dir, { recursive: true, force: true });
        return;
      } catch (err: any) {
        if (i === retries - 1) {
          console.warn(`[Warning] Failed to clean build directory ${dir} after ${retries} attempts: ${err.message}`);
        } else {
          // Sync sleep
          const start = Date.now();
          while (Date.now() - start < delay) {}
        }
      }
    }
  }

  beforeEach(() => {
    // Clean up any existing build dir
    cleanBuildDir(buildDir);
  });

  afterAll(() => {
    // Clean up after all tests complete
    cleanBuildDir(buildDir);
  });

  test("should compile and generate TypeScript code from checkout.omni using mock LLM", () => {
    console.log("   [Integration Test] Running omni build...");
    const buildResult = omniBuild("examples/checkout.omni", "typescript");
    expect(buildResult.status).toBe(0);

    // Verify expected TypeScript code files were generated
    const serviceFile = path.join(buildDir, "src", "services", "Checkout.ts");
    const testFile = path.join(buildDir, "tests", "Checkout.test.ts");

    expect(fs.existsSync(serviceFile)).toBe(true);
    expect(fs.existsSync(testFile)).toBe(true);

    const serviceCode = fs.readFileSync(serviceFile, "utf8");
    expect(serviceCode).toContain("class CheckoutService");
    expect(serviceCode).toContain("placeOrder");
  });

  test("should compile and generate TypeScript state machine from booking_flow.omni using mock LLM", () => {
    console.log("   [Integration Test] Running omni build for booking_flow.omni...");
    const buildResult = omniBuild("examples/booking_flow.omni", "typescript");
    expect(buildResult.status).toBe(0);

    const stateMachineFile = path.join(buildDir, "src", "services", "BookingFlowStateMachine.ts");
    const testFile = path.join(buildDir, "tests", "BookingFlowStateMachine.test.ts");

    expect(fs.existsSync(stateMachineFile)).toBe(true);
    expect(fs.existsSync(testFile)).toBe(true);

    const smCode = fs.readFileSync(stateMachineFile, "utf8");
    expect(smCode).toContain("class BookingFlowStateMachine");
    expect(smCode).toContain("timeoutOrUserAbort");
  });

  test("should compile and generate Rust code from checkout.omni using mock LLM", () => {
    console.log("   [Integration Test] Running omni build for Rust...");
    const buildResult = omniBuild("examples/checkout.omni", "rust");
    expect(buildResult.status).toBe(0);

    // Verify expected Rust code files were generated
    const serviceFile = path.join(buildDir, "src", "services", "checkout.rs");
    const testFile = path.join(buildDir, "tests", "checkout_test.rs");

    expect(fs.existsSync(serviceFile)).toBe(true);
    expect(fs.existsSync(testFile)).toBe(true);

    const serviceCode = fs.readFileSync(serviceFile, "utf8");
    expect(serviceCode).toContain("pub struct CheckoutService");
    expect(serviceCode).toContain("pub fn place_order");
  });

  test("should produce a valid Rust crate on a warm cache (mod.rs regression)", () => {
    // First build warms the .omni-cache for the rust target.
    const warmup = omniBuild("examples/checkout.omni", "rust");
    expect(warmup.status).toBe(0);

    // A fresh build dir + warm cache is the regression case: cached files are
    // restored verbatim, so module registration must be replayed or mod.rs
    // stays empty and the generated crate fails to compile.
    cleanBuildDir(buildDir);

    const cachedBuild = omniBuild("examples/checkout.omni", "rust");
    expect(cachedBuild.status).toBe(0);

    const modRs = fs.readFileSync(path.join(buildDir, "src", "services", "mod.rs"), "utf8");
    expect(modRs).toContain("pub mod checkout;");
  });

  test("should compile and generate Python code from checkout.omni using mock LLM", () => {
    console.log("   [Integration Test] Running omni build for Python...");
    const buildResult = omniBuild("examples/checkout.omni", "python");
    expect(buildResult.status).toBe(0);

    // Verify expected Python code files were generated
    const serviceFile = path.join(buildDir, "app", "services", "checkout.py");
    const testFile = path.join(buildDir, "tests", "test_checkout.py");

    expect(fs.existsSync(serviceFile)).toBe(true);
    expect(fs.existsSync(testFile)).toBe(true);

    const serviceCode = fs.readFileSync(serviceFile, "utf8");
    expect(serviceCode).toContain("class CheckoutService");
    expect(serviceCode).toContain("def place_order");
  });

  test("should compile and generate Go code from checkout.omni using mock LLM", () => {
    console.log("   [Integration Test] Running omni build for Go...");
    const buildResult = omniBuild("examples/checkout.omni", "go");
    expect(buildResult.status).toBe(0);

    // Verify expected Go code files were generated
    const serviceFile = path.join(buildDir, "services", "checkout.go");
    const testFile = path.join(buildDir, "services", "checkout_test.go");

    expect(fs.existsSync(serviceFile)).toBe(true);
    expect(fs.existsSync(testFile)).toBe(true);

    const serviceCode = fs.readFileSync(serviceFile, "utf8");
    expect(serviceCode).toContain("type CheckoutService struct");
    expect(serviceCode).toContain("func (s *CheckoutService) PlaceOrder");
  });
});
