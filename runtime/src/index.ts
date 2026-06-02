import * as fs from "fs";
import minimist from "minimist";
import * as dotenv from "dotenv";
import pc from "picocolors";
import { Orchestrator } from "./orchestrator";
import { DocGenerator } from "./docs";

dotenv.config();

function printUsage() {
  console.log(pc.bold("Usage:"));
  console.log("  node dist/index.js <path-to-spec-ir.json> [options]");
  console.log(pc.bold("Options:"));
  console.log("  --output <dir>  Directory to output generated code (default: build)");
  console.log("  --target <lang> Target language (default: typescript)");
  console.log("  --mode <mode>   Mode: 'build' (default) or 'docs'");
}

async function main() {
  const argv = minimist(process.argv.slice(2));
  const irPath = argv._[0];

  if (!irPath) {
    console.error(pc.red("error: missing path to Spec IR JSON file."));
    printUsage();
    process.exit(1);
  }

  if (!fs.existsSync(irPath)) {
    console.error(pc.red(`error: file not found: ${irPath}`));
    process.exit(1);
  }

  const outputDir = argv.output || "build";
  const target = argv.target || "typescript";
  if (target !== "typescript" && target !== "rust" && target !== "python" && target !== "go") {
    console.error(pc.red(`error: target '${target}' is not supported. Supported targets: 'typescript', 'rust', 'python', 'go'.`));
    process.exit(1);
  }
  const fullStack = !!argv["full-stack"];
  const frozen = !!argv["frozen"];
  const budget = argv.budget !== undefined ? parseFloat(String(argv.budget)) : undefined;
  const mode = argv.mode || "build";

  console.log(pc.green(`🚀 Starting OmniLang Generator Runtime [Mode: ${mode}]`));
  console.log(`   IR Path:   ${pc.cyan(irPath)}`);
  console.log(`   Output:    ${pc.cyan(outputDir)}`);
  if (mode === "build") {
    console.log(`   Target:    ${pc.cyan(target)}`);
    if (fullStack) {
      console.log(`   Mode:      ${pc.cyan("full-stack")}`);
    }
  }

  try {
    if (mode === "docs") {
      const docGen = new DocGenerator({ irPath, outputDir });
      await docGen.generate();
    } else {
      const orchestrator = new Orchestrator({ irPath, outputDir, target, fullStack, frozen, budget });
      await orchestrator.run();
    }
    process.exit(0);
  } catch (err: any) {
    console.error(pc.red(`error: runtime failed: ${err.message || err}`));
    process.exit(1);
  }
}

main();
