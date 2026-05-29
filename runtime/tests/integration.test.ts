import { spawnSync } from "child_process";
import * as path from "path";
import * as fs from "fs";

describe("OmniLang End-to-End Integration", () => {
  const rootDir = path.resolve(__dirname, "../..");
  const buildDir = path.resolve(rootDir, "build");

  beforeAll(() => {
    // 1. Build the Rust omni compiler binary first
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
    const buildResult = spawnSync(
      "cargo",
      ["run", "--bin", "omni", "--", "build", "examples/checkout.omni", "--target", "typescript"],
      {
        cwd: rootDir,
        env: {
          ...process.env,
          OMNI_MOCK_LLM: "true",
        },
      }
    );

    // Assert that the command compiled and finished successfully
    if (buildResult.status !== 0) {
      console.error("STDOUT:", buildResult.stdout?.toString());
      console.error("STDERR:", buildResult.stderr?.toString());
    }
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
    const buildResult = spawnSync(
      "cargo",
      ["run", "--bin", "omni", "--", "build", "examples/booking_flow.omni", "--target", "typescript"],
      {
        cwd: rootDir,
        env: {
          ...process.env,
          OMNI_MOCK_LLM: "true",
        },
      }
    );

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
    const buildResult = spawnSync(
      "cargo",
      ["run", "--bin", "omni", "--", "build", "examples/checkout.omni", "--target", "rust"],
      {
        cwd: rootDir,
        env: {
          ...process.env,
          OMNI_MOCK_LLM: "true",
        },
      }
    );

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

  test("should compile and generate Python code from checkout.omni using mock LLM", () => {
    console.log("   [Integration Test] Running omni build for Python...");
    const buildResult = spawnSync(
      "cargo",
      ["run", "--bin", "omni", "--", "build", "examples/checkout.omni", "--target", "python"],
      {
        cwd: rootDir,
        env: {
          ...process.env,
          OMNI_MOCK_LLM: "true",
        },
      }
    );

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
    const buildResult = spawnSync(
      "cargo",
      ["run", "--bin", "omni", "--", "build", "examples/checkout.omni", "--target", "go"],
      {
        cwd: rootDir,
        env: {
          ...process.env,
          OMNI_MOCK_LLM: "true",
        },
      }
    );

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
