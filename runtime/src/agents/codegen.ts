import * as fs from "fs";
import * as path from "path";
import pc from "picocolors";
import { LLMProvider } from "../providers";
import { getSystemPrompt, getUserPrompt } from "../prompts/codegen";
import { SpecIR } from "../types";

export class CodeGenAgent {
  private provider: LLMProvider;

  constructor(provider: LLMProvider) {
    this.provider = provider;
  }

  public async generateService(
    serviceName: string,
    ir: SpecIR,
    outputDir: string,
    target: string = "typescript",
    promptAdditions: string = "",
    model?: string
  ): Promise<{ success: boolean; files: string[] }> {
    console.log(pc.yellow(`   Generating service: ${pc.cyan(serviceName)} [${target}]...`));

    const systemPrompt = getSystemPrompt(target, promptAdditions);
    const userPrompt = getUserPrompt(serviceName, ir, target);

    const response = await this.provider.generateCode(systemPrompt, userPrompt, model);
    const parsed = this.parseResponse(response, serviceName);

    const writtenFiles: string[] = [];

    for (const file of parsed.files) {
      const targetPath = path.join(outputDir, file.path);
      fs.mkdirSync(path.dirname(targetPath), { recursive: true });
      
      let content = file.content;
      if (target === "typescript" && file.path.endsWith(".ts")) {
        content = this.postProcessContent(content, serviceName);
      }
      fs.writeFileSync(targetPath, content, "utf8");
      
      console.log(`     ${pc.green("✓")} Wrote ${pc.dim(file.path)}`);
      writtenFiles.push(targetPath);
    }

    // For Rust, update mod.rs to include the new service module
    if (target === "rust") {
      this.updateRustModFile(outputDir, serviceName);
    }

    return { success: true, files: writtenFiles };
  }

  private parseResponse(response: string, serviceName: string): { files: Array<{ path: string; content: string }> } {
    const cleanText = response.trim();
    const files: Array<{ path: string; content: string }> = [];

    // Try to extract JSON from markdown code blocks first
    const jsonBlockRegex = /```json\s*([\s\S]*?)\s*```/g;
    let match;
    while ((match = jsonBlockRegex.exec(cleanText)) !== null) {
      if (match[1]) {
        // Parse the raw block first: valid JSON whose string values contain
        // template-literal backticks (e.g. `Hello, ${name}!`) must not be
        // mangled. Backtick→quote conversion is only a last-resort recovery.
        const blockText = match[1].trim();
        try {
          const parsedBlock = JSON.parse(blockText);
          if (parsedBlock && typeof parsedBlock === "object") {
            if (typeof parsedBlock.path === "string" && typeof parsedBlock.content === "string") {
              files.push({ path: parsedBlock.path, content: parsedBlock.content });
            } else if (Array.isArray(parsedBlock.files)) {
              for (const f of parsedBlock.files) {
                if (typeof f.path === "string" && typeof f.content === "string") {
                  files.push(f);
                }
              }
            } else if (parsedBlock.files && typeof parsedBlock.files === "object") {
              for (const k of Object.keys(parsedBlock.files)) {
                const val = parsedBlock.files[k];
                files.push({
                  path: k,
                  content: typeof val === "string" ? val : (val.content || "")
                });
              }
            } else {
              const fileKeys = Object.keys(parsedBlock).filter(k => k.includes("/") || k.endsWith(".ts") || k.endsWith(".rs") || k.endsWith(".py") || k.endsWith(".js"));
              if (fileKeys.length > 0) {
                for (const k of fileKeys) {
                  const val = parsedBlock[k];
                  files.push({
                    path: k,
                    content: typeof val === "string" ? val : (val.content || "")
                  });
                }
              }
            }
          }
        } catch (e) {
          // Recovery candidates, in order of fidelity.
          const recoveryCandidates = [
            this.attemptJsonRecovery(blockText),
            this.convertBackticksToDoubleQuotes(blockText),
          ];
          for (const candidate of recoveryCandidates) {
            try {
              const parsedBlock = JSON.parse(candidate);
              if (parsedBlock && typeof parsedBlock === "object") {
                if (
                  typeof parsedBlock.path === "string" &&
                  typeof parsedBlock.content === "string"
                ) {
                  files.push({ path: parsedBlock.path, content: parsedBlock.content });
                } else if (Array.isArray(parsedBlock.files)) {
                  for (const f of parsedBlock.files) {
                    if (typeof f.path === "string" && typeof f.content === "string") {
                      files.push(f);
                    }
                  }
                }
              }
              if (files.length > 0) break;
            } catch (recoveryErr) {
              // try next candidate
            }
          }
        }
      }
    }

    if (files.length > 0) {
      return { files };
    }

    // Fallback: search for first '{' and last '}' of the entire response
    let singleJsonText = cleanText;
    const firstBrace = singleJsonText.indexOf("{");
    const lastBrace = singleJsonText.lastIndexOf("}");
    if (firstBrace !== -1 && lastBrace !== -1 && lastBrace > firstBrace) {
      singleJsonText = singleJsonText.substring(firstBrace, lastBrace + 1).trim();
    }

    // Try parse strategies in order of fidelity: raw JSON first (so valid JSON
    // containing template-literal backticks is preserved), then structural
    // recovery, then backtick→quote conversion as a last resort.
    const candidates = [
      singleJsonText,
      this.attemptJsonRecovery(singleJsonText),
      this.convertBackticksToDoubleQuotes(singleJsonText),
      this.attemptJsonRecovery(this.convertBackticksToDoubleQuotes(singleJsonText)),
    ];
    let lastErr: any;
    for (const candidate of candidates) {
      try {
        const parsed = JSON.parse(candidate);
        this.normalizeResponse(parsed, serviceName);
        this.validateResponseStructure(parsed);
        return parsed;
      } catch (e: any) {
        lastErr = e;
      }
    }
    console.error(
      pc.red("Failed to parse JSON response from LLM (even after recovery attempts):"),
    );
    console.error(pc.dim(response));
    throw new Error(`Invalid JSON format from code generator: ${lastErr?.message}`);
  }

  private normalizeResponse(parsed: any, serviceName: string): void {
    if (!parsed || typeof parsed !== "object") {
      return;
    }

    // Normalize files object variation
    if (parsed.files && typeof parsed.files === "object" && !Array.isArray(parsed.files)) {
      const fileKeys = Object.keys(parsed.files);
      parsed.files = fileKeys.map(k => {
        const val = parsed.files[k];
        return {
          path: k,
          content: typeof val === "string" ? val : (val.content || "")
        };
      });
      return;
    }

    // If 'files' array is already present, nothing to do
    if (Array.isArray(parsed.files)) {
      return;
    }

    // Case 1: Look for any other array property that might be the files array
    const arrayKey = Object.keys(parsed).find(k => Array.isArray(parsed[k]));
    if (arrayKey) {
      parsed.files = parsed[arrayKey];
      return;
    }

    // Case 2: Check for implementation/tests key-value objects or strings
    const implKey = Object.keys(parsed).find(k => ["implementation", "impl", "service", "src"].includes(k.toLowerCase()));
    const testKey = Object.keys(parsed).find(k => ["tests", "test", "spec"].includes(k.toLowerCase()));

    if (implKey && testKey) {
      const implVal = parsed[implKey];
      const testVal = parsed[testKey];

      if (implVal && typeof implVal === "object" && testVal && typeof testVal === "object") {
        parsed.files = [
          {
            path: implVal.file || implVal.path || `src/services/${serviceName}.ts`,
            content: implVal.content || ""
          },
          {
            path: testVal.file || testVal.path || `tests/${serviceName}.test.ts`,
            content: testVal.content || ""
          }
        ];
        return;
      } else if (typeof implVal === "string" && typeof testVal === "string") {
        parsed.files = [
          {
            path: `src/services/${serviceName}.ts`,
            content: implVal
          },
          {
            path: `tests/${serviceName}.test.ts`,
            content: testVal
          }
        ];
        return;
      }
    }

    // Case 3: Check if keys themselves look like file paths (e.g., contains '/' or ends with extension)
    const fileKeys = Object.keys(parsed).filter(k => k.includes("/") || k.endsWith(".ts") || k.endsWith(".rs") || k.endsWith(".py") || k.endsWith(".js"));
    if (fileKeys.length > 0) {
      parsed.files = fileKeys.map(k => {
        const val = parsed[k];
        return {
          path: k,
          content: typeof val === "string" ? val : (val.content || "")
        };
      });
    }
  }

  private validateResponseStructure(parsed: any): void {
    if (!parsed || typeof parsed !== "object") {
      throw new Error("Response is not a JSON object");
    }
    if (!Array.isArray(parsed.files)) {
      throw new Error("Response must contain a 'files' array");
    }
    for (const file of parsed.files) {
      if (typeof file !== "object" || file === null) {
        throw new Error("File entry in response must be an object");
      }
      if (typeof file.path !== "string" || typeof file.content !== "string") {
        throw new Error("File entry must contain 'path' and 'content' as strings");
      }
    }
  }

  private attemptJsonRecovery(text: string): string {
    let fixed = text.trim();
    
    // Remove trailing commas before closing braces/brackets
    fixed = fixed.replace(/,\s*([\]}])/g, "$1");
    
    // If it doesn't end with } but we can count braces and add missing closing braces
    let openBraces = 0;
    let openBrackets = 0;
    let inString = false;
    let escape = false;
    
    for (let i = 0; i < fixed.length; i++) {
      const char = fixed[i];
      if (escape) {
        escape = false;
        continue;
      }
      if (char === "\\") {
        escape = true;
        continue;
      }
      if (char === '"') {
        inString = !inString;
        continue;
      }
      if (!inString) {
        if (char === "{") openBraces++;
        else if (char === "}") openBraces--;
        else if (char === "[") openBrackets++;
        else if (char === "]") openBrackets--;
      }
    }
    
    // If it ends with } and has unclosed brackets, try inserting closing bracket before the last brace
    if (fixed.endsWith("}") && openBraces === 0 && openBrackets > 0) {
      const lastIndex = fixed.lastIndexOf("}");
      const candidate = fixed.substring(0, lastIndex) + "]" + fixed.substring(lastIndex);
      try {
        JSON.parse(candidate);
        return candidate;
      } catch (err) {
        // Fall through
      }
    }
    
    while (openBraces > 0) {
      fixed += "}";
      openBraces--;
    }
    while (openBrackets > 0) {
      fixed += "]";
      openBrackets--;
    }
    
    return fixed;
  }

  private convertBackticksToDoubleQuotes(text: string): string {
    return text.replace(/:\s*`([\s\S]*?)`(\s*(?:,|\n|}))/g, (match, content, suffix) => {
      const escaped = content
        .replace(/\\/g, "\\\\")
        .replace(/"/g, '\\"')
        .replace(/\n/g, "\\n")
        .replace(/\r/g, "\\r")
        .replace(/\t/g, "\\t");
      return `: "${escaped}"${suffix}`;
    });
  }

  private updateRustModFile(outputDir: string, serviceName: string): void {
    const modRsPath = path.join(outputDir, "src", "services", "mod.rs");
    const modName = this.toSnakeCase(serviceName);
    const modLine = `pub mod ${modName};\n`;
    
    if (fs.existsSync(modRsPath)) {
      const content = fs.readFileSync(modRsPath, "utf8");
      if (!content.includes(modLine.trim())) {
        fs.appendFileSync(modRsPath, modLine);
      }
    }
  }

  private postProcessContent(content: string, serviceName: string): string {
    let processed = content;

    // Fix "export default class ServiceName" -> "export class ServiceName"
    processed = processed.replace(new RegExp(`export\\s+default\\s+class\\s+${serviceName}\\b`, 'g'), `export class ${serviceName}`);

    // Fix separate "export default ServiceName;" at the end
    const defaultExportRegex = new RegExp(`export\\s+default\\s+${serviceName}\\b;?`, 'g');
    if (defaultExportRegex.test(processed)) {
      processed = processed.replace(defaultExportRegex, '');
      // Ensure "class ServiceName" is exported
      const classDeclRegex = new RegExp(`(^|\\n)class\\s+${serviceName}\\b`, 'g');
      processed = processed.replace(classDeclRegex, `$1export class ${serviceName}`);
    }

    // Fix LLM using private for state map/helper methods needed in E2E tests
    processed = processed.replace(/\bprivate\s+accounts\b/g, 'public accounts');
    processed = processed.replace(/\bprivate\s+getAccount\b/g, 'public getAccount');

    // Automatically re-export types imported from ../types so external code can access them
    const importTypesRegex = /import\s+\{\s*([^}]+)\s*\}\s+from\s+['"]\.\.\/types(?:\.ts)?['"]/g;
    let importMatch;
    importTypesRegex.lastIndex = 0;
    while ((importMatch = importTypesRegex.exec(processed)) !== null) {
      const typesList = importMatch[1].trim();
      const exportString = `export { ${typesList} };`;
      // Check if already exported to avoid duplicate exports
      if (!processed.includes(exportString) && !processed.includes(`export {${typesList}}`)) {
        processed += `\n${exportString}\n`;
      }
    }

    return processed.trim();
  }

  private toSnakeCase(name: string): string {
    return name
      .replace(/([A-Z])/g, "_$1")
      .toLowerCase()
      .replace(/^_/, "");
  }
}
