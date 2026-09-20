import { spawnSync } from "child_process";
import * as fs from "fs";
import * as path from "path";
import pc from "picocolors";

export interface VerificationReport {
  success: boolean;
  typeCheckError?: string;
  testError?: string;
}

export class VerificationRunner {
  private outputDir: string;
  private target: string;
  private files?: string[];

  constructor(outputDir: string, target: string = "typescript", files?: string[]) {
    this.outputDir = outputDir;
    this.target = target;
    this.files = files;
  }

  public verify(): VerificationReport {
    switch (this.target) {
      case "rust":
        return this.verifyRust();
      case "python":
        return this.verifyPython();
      case "go":
        return this.verifyGo();
      case "typescript":
      default:
        return this.verifyTypeScript();
    }
  }

  private verifyTypeScript(): VerificationReport {
    console.log(pc.yellow("     Running TypeScript type check (tsc)..."));

    // 1. Run tsc --noEmit
    const tscBin = path.join(this.outputDir, "node_modules", ".bin", "tsc");
    const tscArgs = ["--noEmit"];
    if (this.files && this.files.length > 0) {
      tscArgs.push(
        ...this.files,
        "--target", "ES2022",
        "--module", "Node16",
        "--moduleResolution", "Node16",
        "--strict",
        "--esModuleInterop",
        "--skipLibCheck"
      );
    }
    
    const tscRes = fs.existsSync(tscBin)
      ? spawnSync(tscBin, tscArgs, { cwd: this.outputDir, stdio: "pipe", shell: true })
      : spawnSync("npx", ["-p", "typescript", "tsc", ...tscArgs], {
          cwd: this.outputDir,
          stdio: "pipe",
          shell: true,
        });

    if (tscRes.status !== 0) {
      const errorText = (tscRes.stdout ? tscRes.stdout.toString() : "") + (tscRes.stderr ? tscRes.stderr.toString() : "");
      return {
        success: false,
        typeCheckError: errorText,
      };
    }

    console.log(`     ${pc.green("✓")} TypeScript type check passed`);

    // 2. Run jest tests
    console.log(pc.yellow("     Running Jest tests..."));
    
    const jestArgs = ["jest", "--passWithNoTests"];
    if (this.files && this.files.length > 0) {
      const testFiles = this.files.filter(f => f.endsWith(".test.ts") || f.endsWith(".spec.ts"));
      if (testFiles.length > 0) {
        jestArgs.push(...testFiles);
      }
    }

    const jestRes = spawnSync("npx", jestArgs, {
      cwd: this.outputDir,
      stdio: "pipe",
      shell: true,
    });

    if (jestRes.status !== 0) {
      const errorText = jestRes.stdout.toString() + jestRes.stderr.toString();
      return {
        success: false,
        testError: errorText,
      };
    }

    console.log(`     ${pc.green("✓")} All Jest tests passed`);

    return { success: true };
  }

  private verifyRust(): VerificationReport {
    console.log(pc.yellow("     Running cargo build..."));

    // 1. Run cargo build
    const buildRes = spawnSync("cargo", ["build"], {
      cwd: this.outputDir,
      stdio: "pipe",
      shell: true,
    });

    if (buildRes.status !== 0) {
      const errorText = buildRes.stdout.toString() + buildRes.stderr.toString();
      return {
        success: false,
        typeCheckError: errorText,
      };
    }

    console.log(`     ${pc.green("✓")} Rust cargo build passed`);

    // 2. Run cargo test
    console.log(pc.yellow("     Running cargo test..."));
    const testRes = spawnSync("cargo", ["test"], {
      cwd: this.outputDir,
      stdio: "pipe",
      shell: true,
    });

    if (testRes.status !== 0) {
      const errorText = testRes.stdout.toString() + testRes.stderr.toString();
      return {
        success: false,
        testError: errorText,
      };
    }

    console.log(`     ${pc.green("✓")} All Rust tests passed`);

    return { success: true };
  }

  private verifyPython(): VerificationReport {
    console.log(pc.yellow("     Running mypy type check..."));

    // 1. Run mypy
    const mypyRes = spawnSync("python3", ["-m", "mypy", "."], {
      cwd: this.outputDir,
      stdio: "pipe",
      shell: true,
    });

    if (mypyRes.status !== 0) {
      const errorText = mypyRes.stdout.toString() + mypyRes.stderr.toString();
      // mypy failures are non-fatal warnings for now
      console.log(
        `     ${pc.yellow("⚠")} mypy reported issues (non-blocking):`
      );
      console.log(pc.dim(errorText.trim()));
    } else {
      console.log(`     ${pc.green("✓")} mypy type check passed`);
    }

    // 2. Run pytest
    console.log(pc.yellow("     Running pytest..."));
    const pytestRes = spawnSync("python3", ["-m", "pytest", "-v"], {
      cwd: this.outputDir,
      stdio: "pipe",
      shell: true,
    });

    if (pytestRes.status !== 0) {
      const errorText =
        pytestRes.stdout.toString() + pytestRes.stderr.toString();
      return {
        success: false,
        testError: errorText,
      };
    }

    console.log(`     ${pc.green("✓")} All Python tests passed`);

    return { success: true };
  }

  private verifyGo(): VerificationReport {
    console.log(pc.yellow("     Running go test..."));
    const testRes = spawnSync("go", ["test", "./..."], {
      cwd: this.outputDir,
      stdio: "pipe",
      shell: true,
    });

    if (testRes.status !== 0) {
      const errorText = testRes.stdout.toString() + testRes.stderr.toString();
      return {
        success: false,
        testError: errorText,
      };
    }

    console.log(`     ${pc.green("✓")} All Go tests passed`);
    return { success: true };
  }
}
