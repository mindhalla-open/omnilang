import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import { SecurityRunner } from "../src/testing/security";

describe("SecurityRunner", () => {
  let project: string;
  let evidence: string;

  beforeEach(() => {
    project = fs.mkdtempSync(path.join(os.tmpdir(), "omni-sec-"));
    evidence = fs.mkdtempSync(path.join(os.tmpdir(), "omni-evi-"));
    fs.mkdirSync(path.join(project, "src"), { recursive: true });
  });
  afterEach(() => {
    fs.rmSync(project, { recursive: true, force: true });
    fs.rmSync(evidence, { recursive: true, force: true });
  });

  it("flags eval() as an injection risk and writes a SARIF report", () => {
    fs.writeFileSync(
      path.join(project, "src", "danger.ts"),
      "export function run(x: string) { return eval(x); }\n",
    );
    const runner = new SecurityRunner(evidence);
    const result = runner.runSecurityScan(project);

    expect(result.success).toBe(false);
    expect(result.issues.some((i) => i.ruleId === "no-eval")).toBe(true);

    const sarif = JSON.parse(fs.readFileSync(path.join(evidence, "security_report.sarif"), "utf8"));
    expect(sarif.$schema).toContain("sarif");
    expect(sarif.runs[0].results.length).toBeGreaterThan(0);
  });

  it("passes clean generated code with zero issues", () => {
    fs.writeFileSync(
      path.join(project, "src", "ok.ts"),
      "export function greet(n: string) { return `Hello, ${n}!`; }\n",
    );
    const result = new SecurityRunner(evidence).runSecurityScan(project);
    expect(result.success).toBe(true);
    expect(result.issues).toHaveLength(0);
  });
});
