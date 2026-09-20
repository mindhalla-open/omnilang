import * as fs from "fs";
import * as path from "path";
import pc from "picocolors";

export interface SecurityIssue {
  id: string;
  ruleId: string;
  level: "error" | "warning" | "note";
  message: string;
  filePath: string;
  line: number;
}

/**
 * A small, honest pattern scan of generated code, run report-only after a
 * green build: `eval(`, an `unsafeSQL` marker, and one known-bad lodash
 * version. Findings go to `.evidence/security_report.sarif` and into
 * `build-report.json` as `security_issues`.
 *
 * This is not a SAST engine and does not fuzz anything. Plugging in a real
 * scanner (Semgrep, OWASP dependency-check) is roadmap work.
 */
export class SecurityRunner {
  private evidenceDir: string;

  constructor(customEvidenceDir?: string) {
    this.evidenceDir = customEvidenceDir || path.resolve(process.cwd(), ".evidence");
    if (!fs.existsSync(this.evidenceDir)) {
      fs.mkdirSync(this.evidenceDir, { recursive: true });
    }
  }

  public runSecurityScan(projectDir: string): { success: boolean; issues: SecurityIssue[] } {
    console.log(`[Security Scan] Pattern scan of generated code (report-only)...`);

    const issues: SecurityIssue[] = [];

    // Source patterns: eval() and the unsafeSQL marker.
    const srcDir = path.join(projectDir, "src");
    if (fs.existsSync(srcDir)) {
      this.scanDirectoryRecursively(srcDir, issues);
    }

    // Dependency check: a single known-bad pin, not a vulnerability database.
    const packageJsonPath = path.join(projectDir, "package.json");
    if (fs.existsSync(packageJsonPath)) {
      const pkg = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
      if (pkg.dependencies && pkg.dependencies["lodash"] === "4.17.15") {
        issues.push({
          id: "OMNI-SEC-003",
          ruleId: "cve-prototype-pollution",
          level: "error",
          message: "OWASP A06:2021-Vulnerable and Outdated Components: lodash@4.17.15 has known prototype pollution vulnerability",
          filePath: "package.json",
          line: 10
        });
      }
    }

    // Write SARIF report
    const sarifReport = this.generateSarifReport(issues);
    const sarifPath = path.join(this.evidenceDir, "security_report.sarif");
    fs.writeFileSync(sarifPath, JSON.stringify(sarifReport, null, 2));

    const errors = issues.filter(i => i.level === "error");
    const success = errors.length === 0;

    if (success) {
      console.log(`[Security Scan] ${pc.green("✓")} No error-level pattern findings (not a full SAST run).`);
    } else {
      console.error(pc.red(`[Security Scan] ❌ ${errors.length} error-level finding(s):`));
      for (const issue of errors) {
        console.error(`   - ${pc.bold(issue.ruleId)} at ${issue.filePath}:${issue.line}: ${issue.message}`);
      }
    }

    return { success, issues };
  }

  private scanDirectoryRecursively(dir: string, issues: SecurityIssue[]): void {
    const files = fs.readdirSync(dir);
    for (const file of files) {
      const fullPath = path.join(dir, file);
      const stat = fs.statSync(fullPath);
      if (stat.isDirectory()) {
        this.scanDirectoryRecursively(fullPath, issues);
      } else if (file.endsWith(".ts") || file.endsWith(".omni")) {
        const content = fs.readFileSync(fullPath, "utf8");
        if (content.includes("eval(")) {
          issues.push({
            id: "OMNI-SEC-001",
            ruleId: "no-eval",
            level: "error",
            message: "OWASP A03:2021-Injection: Avoid using eval() as it leads to potential remote code execution",
            filePath: path.relative(process.cwd(), fullPath),
            line: content.split("\n").findIndex(l => l.includes("eval(")) + 1
          });
        }
        if (content.includes("unsafeSQL")) {
          issues.push({
            id: "OMNI-SEC-002",
            ruleId: "sql-injection-risk",
            level: "error",
            message: "OWASP A03:2021-Injection: Raw SQL concatenation detected",
            filePath: path.relative(process.cwd(), fullPath),
            line: content.split("\n").findIndex(l => l.includes("unsafeSQL")) + 1
          });
        }
      }
    }
  }

  private generateSarifReport(issues: SecurityIssue[]) {
    return {
      $schema: "https://json.schemastore.org/sarif-2.1.0-rtm.5.json",
      version: "2.1.0",
      runs: [
        {
          tool: {
            driver: {
              name: "OmniLang Security Scanner",
              version: "0.8.0",
              rules: [
                {
                  id: "no-eval",
                  name: "NoEvalRule",
                  shortDescription: { text: "Avoid use of eval()" }
                },
                {
                  id: "sql-injection-risk",
                  name: "SqlInjectionRule",
                  shortDescription: { text: "Avoid raw SQL concatenation" }
                },
                {
                  id: "cve-prototype-pollution",
                  name: "PrototypePollutionRule",
                  shortDescription: { text: "Outdated vulnerable library" }
                }
              ]
            }
          },
          results: issues.map(issue => ({
            ruleId: issue.ruleId,
            level: issue.level,
            message: { text: issue.message },
            locations: [
              {
                physicalLocation: {
                  artifactLocation: { uri: issue.filePath },
                  region: { startLine: issue.line }
                }
              }
            ]
          }))
        }
      ]
    };
  }
}
