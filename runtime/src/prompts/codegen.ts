import * as fs from "fs";
import * as path from "path";
import { SpecIR, ServiceDecl, TypeDecl, TypeRef, TypeMapping } from "../types";
import { enumerateContracts, formatContractsForPrompt } from "../contracts";

function toSnakeCase(name: string): string {
  return name.replace(/([A-Z])/g, "_$1").toLowerCase().replace(/^_/, "");
}

export function getSystemPrompt(target: string = "typescript", promptAdditions: string = ""): string {
  if (target === "rust") {
    return getRustSystemPrompt(promptAdditions);
  } else if (target === "python") {
    return getPythonSystemPrompt(promptAdditions);
  } else if (target === "go") {
    return getGoSystemPrompt(promptAdditions);
  } else if (target === "typescript") {
    return getTypeScriptSystemPrompt(promptAdditions);
  } else {
    throw new Error(`Unsupported target: ${target}`);
  }
}

export function getUserPrompt(serviceName: string, ir: SpecIR, target: string = "typescript"): string {
  if (target !== "typescript" && target !== "rust" && target !== "python" && target !== "go") {
    throw new Error(`Unsupported target: ${target}`);
  }

  // Find the target service in IR
  const serviceDecl = ir.source_file.declarations.find(
    (d) => "Service" in d && d.Service.name === serviceName
  );
  const service = serviceDecl && "Service" in serviceDecl ? serviceDecl.Service : undefined;

  if (!service) {
    throw new Error(`Service ${serviceName} not found in Spec IR AST.`);
  }

  // Format type definitions depending on target
  let types = "";
  if (target === "typescript") {
    types = ir.source_file.declarations
      .filter((d): d is { Type: TypeDecl } => "Type" in d)
      .map((d) => {
        const t = d.Type;
        const doc = t.doc_comment
          ? t.doc_comment.split("\n").map((line: string) => `/// ${line}`).join("\n") + "\n"
          : "";
        return `${doc}${formatTypeDecl(t, ir.type_mappings)}`;
      })
      .join("\n\n");
  } else if (target === "rust") {
    types = ir.source_file.declarations
      .filter((d): d is { Type: TypeDecl } => "Type" in d)
      .map((d) => formatRustTypeDecl(d.Type))
      .join("\n\n");
  } else if (target === "python") {
    types = ir.source_file.declarations
      .filter((d): d is { Type: TypeDecl } => "Type" in d)
      .map((d) => formatPythonTypeDecl(d.Type))
      .join("\n\n");
  } else if (target === "go") {
    types = ir.source_file.declarations
      .filter((d): d is { Type: TypeDecl } => "Type" in d)
      .map((d) => formatGoTypeDecl(d.Type))
      .join("\n\n");
  }

  // Format service spec
  const specDetails = JSON.stringify(service, null, 2);

  const targetInfo = getTargetFileInfo(target, serviceName);
  const contractsSection = formatContractsForPrompt(enumerateContracts(service));

  return `Generate the implementation and tests for the service: "${serviceName}".

Here are the defined Type Definitions for the module:
${types}

Here is the Service Specification details:
${specDetails}
${contractsSection}

Please write the complete ${target === "typescript" ? "TypeScript" : target === "rust" ? "Rust" : target === "python" ? "Python" : "Go"} source code for both the service implementation and its corresponding tests.
- The implementation code must be generated for the path: "${targetInfo.implPath}"
- The test code must be generated for the path: "${targetInfo.testPath}"

${targetInfo.extra}
Remember to return ONLY the raw JSON string matching the required structure.`;
}

function getTargetFileInfo(target: string, serviceName: string): { implPath: string; testPath: string; extra: string } {
  if (target !== "typescript" && target !== "rust" && target !== "python" && target !== "go") {
    throw new Error(`Unsupported target: ${target}`);
  }
  const snakeName = toSnakeCase(serviceName);
  if (target === "rust") {
    return {
      implPath: `src/services/${snakeName}.rs`,
      testPath: `tests/${snakeName}_test.rs`,
      extra: `Ensure that unit tests import the implementation module. For example: 'use omni_build::services::${snakeName}::*;'`,
    };
  }
  if (target === "python") {
    return {
      implPath: `app/services/${snakeName}.py`,
      testPath: `tests/test_${snakeName}.py`,
      extra: `Ensure that unit tests import the implementation. For example: 'from app.services.${snakeName} import ${serviceName}'`,
    };
  }
  if (target === "go") {
    return {
      implPath: `services/${snakeName}.go`,
      testPath: `services/${snakeName}_test.go`,
      extra: `Ensure that unit tests reside in the same package 'services'.`,
    };
  }
  return {
    implPath: `src/services/${serviceName}.ts`,
    testPath: `tests/${serviceName}.test.ts`,
    extra: `Ensure that the unit tests import the implementation from "../src/services/${serviceName}".\nUse fast-check for property-based tests where the spec contains forall/property blocks.`,
  };
}

function getRustSystemPrompt(promptAdditions: string = ""): string {
  const additionsText = promptAdditions ? `\n\n${promptAdditions}\n` : "";
  return `You are a Senior Rust and Test Engineer.
Your task is to generate a verified Rust implementation and matching unit tests based on an OmniLang specification.

You MUST follow these rules:
1. Generate clean, idiomatic Rust code.
2. Implement all operations defined in the service spec.
3. Validate inputs and enforce preconditions and postconditions.
4. If the service declares invariants, verify them before and after each operation call.
5. If the service declares custom error types, implement them as enum variants or struct types, deriving Debug and using thiserror for std::fmt::Display.
6. Write comprehensive unit tests covering all success scenarios and error cases defined in the test blocks.
7. Use Cargo-standard modules: expose implementation modules inside 'src/services/<service_name>.rs' and register them in 'src/services/mod.rs'.
8. Tests should be written in a separate test module or a test file under 'tests/<service_name>_test.rs'.

${additionsText}
Return your output EXACTLY as a JSON object with the following structure, and do not wrap it in markdown code blocks or add any conversational text.

JSON Structure:
{
  "files": [
    {
      "path": "src/services/<service_name>.rs",
      "content": "// Put the full, complete Rust implementation code here"
    },
    {
      "path": "tests/<service_name>_test.rs",
      "content": "// Put the full, complete Rust test code here"
    }
  ]
}`;
}

function getPythonSystemPrompt(promptAdditions: string = ""): string {
  const additionsText = promptAdditions ? `\n\n${promptAdditions}\n` : "";
  return `You are a Senior Python and Test Engineer.
Your task is to generate a verified Python implementation and matching pytest unit tests based on an OmniLang specification.

You MUST follow these follow these rules:
1. Generate clean, PEP 8 compliant, well-typed Python code using dataclasses and standard type hints.
2. Implement all operations defined in the service spec.
3. Validate inputs and enforce preconditions and postconditions.
4. If the service declares invariants, verify them before and after each operation call.
5. If the service declares custom error types, implement them as custom exception classes inheriting from Exception or a base error class.
6. Write comprehensive unit tests using pytest covering all success scenarios and error cases defined in the test blocks.
7. Use pytest conventions: tests go in 'tests/test_<service_name>.py' and implementations in 'app/services/<service_name>.py'.

${additionsText}
Return your output EXACTLY as a JSON object with the following structure, and do not wrap it in markdown code blocks or add any conversational text.

JSON Structure:
{
  "files": [
    {
      "path": "app/services/<service_name>.py",
      "content": "// Put the full, complete Python implementation code here"
    },
    {
      "path": "tests/test_<service_name>.py",
      "content": "// Put the full, complete pytest code here"
    }
  ]
}`;
}

function formatRustTypeRef(ref: TypeRef): string {
  if (ref.name === "String" || ref.name === "Email" || ref.name === "URL" || ref.name === "UUID") {
    return "String";
  }
  if (ref.name === "Int") {
    return "i64";
  }
  if (ref.name === "Float" || ref.name === "Decimal" || ref.name === "Money") {
    return "f64";
  }
  if (ref.name === "Bool") {
    return "bool";
  }
  if (ref.name === "DateTime" || ref.name === "Date" || ref.name === "Timestamp") {
    return "String";
  }
  if (ref.name === "Duration") {
    return "String";
  }
  if (ref.name === "Bytes") {
    return "Vec<u8>";
  }
  if (ref.name === "Void") {
    return "()";
  }
  if (ref.name === "List") {
    const arg = ref.type_args[0] ? formatRustTypeRef(ref.type_args[0]) : "String";
    return `Vec<${arg}>`;
  }
  if (ref.name === "Option") {
    const arg = ref.type_args[0] ? formatRustTypeRef(ref.type_args[0]) : "String";
    return `Option<${arg}>`;
  }
  if (ref.type_args.length > 0) {
    const args = ref.type_args.map(arg => formatRustTypeRef(arg)).join(", ");
    return `${ref.name}<${args}>`;
  }
  return ref.name;
}

export function formatRustTypeDecl(t: TypeDecl): string {
  const name = t.name;
  const kind = t.kind;

  if ("Alias" in kind) {
    return `pub type ${name} = ${formatRustTypeRef(kind.Alias)};`;
  }

  if ("Struct" in kind) {
    const fields = kind.Struct.fields.map(f => {
      const doc = f.doc_comment ? `  /// ${f.doc_comment}\n` : "";
      return `${doc}  pub ${toSnakeCase(f.name)}: ${formatRustTypeRef(f.ty)},`;
    }).join("\n");
    return `#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]\npub struct ${name} {\n${fields}\n}`;
  }

  if ("Enum" in kind) {
    const variants = kind.Enum.variants.map(v => {
      return `  ${v.name},`;
    }).join("\n");
    return `#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]\npub enum ${name} {\n${variants}\n}`;
  }

  if ("Refined" in kind) {
    const base = kind.Refined.base ? formatRustTypeRef(kind.Refined.base) : "String";
    return `pub type ${name} = ${base}; // Refined type with constraints`;
  }

  return `pub type ${name} = ();`;
}

function formatPythonTypeRef(ref: TypeRef): string {
  if (ref.name === "String" || ref.name === "Email" || ref.name === "URL" || ref.name === "UUID") {
    return "str";
  }
  if (ref.name === "Int") {
    return "int";
  }
  if (ref.name === "Float" || ref.name === "Decimal" || ref.name === "Money") {
    return "float";
  }
  if (ref.name === "Bool") {
    return "bool";
  }
  if (ref.name === "DateTime" || ref.name === "Date" || ref.name === "Timestamp") {
    return "str";
  }
  if (ref.name === "Duration") {
    return "str";
  }
  if (ref.name === "Bytes") {
    return "bytes";
  }
  if (ref.name === "Void") {
    return "None";
  }
  if (ref.name === "List") {
    const arg = ref.type_args[0] ? formatPythonTypeRef(ref.type_args[0]) : "any";
    return `List[${arg}]`;
  }
  if (ref.name === "Option") {
    const arg = ref.type_args[0] ? formatPythonTypeRef(ref.type_args[0]) : "any";
    return `Optional[${arg}]`;
  }
  if (ref.type_args.length > 0) {
    const args = ref.type_args.map(arg => formatPythonTypeRef(arg)).join(", ");
    return `${ref.name}[${args}]`;
  }
  return ref.name;
}

export function formatPythonTypeDecl(t: TypeDecl): string {
  const name = t.name;
  const kind = t.kind;

  if ("Alias" in kind) {
    return `${name} = ${formatPythonTypeRef(kind.Alias)}`;
  }

  if ("Struct" in kind) {
    const fields = kind.Struct.fields.map(f => {
      const doc = f.doc_comment ? `    # ${f.doc_comment}\n` : "";
      return `${doc}    ${toSnakeCase(f.name)}: ${formatPythonTypeRef(f.ty)}`;
    }).join("\n");
    return `@dataclass\nclass ${name}:\n${fields || "    pass"}`;
  }

  if ("Enum" in kind) {
    const variants = kind.Enum.variants.map(v => {
      return `    ${v.name} = "${v.name}"`;
    }).join("\n");
    return `class ${name}(Enum):\n${variants || "    pass"}`;
  }

  if ("Refined" in kind) {
    const base = kind.Refined.base ? formatPythonTypeRef(kind.Refined.base) : "str";
    return `${name} = ${base} # Refined type with constraints`;
  }

  return `${name} = Any`;
}

function getTypeScriptSystemPrompt(promptAdditions: string = ""): string {
  let templatesDir = path.join(__dirname, "templates");
  let systemPromptPath = path.join(templatesDir, "system_prompt.md");
  if (!fs.existsSync(systemPromptPath)) {
    templatesDir = path.resolve(__dirname, "../../src/prompts/templates");
    systemPromptPath = path.join(templatesDir, "system_prompt.md");
  }
  const refImplPath = path.join(templatesDir, "reference_example.ts");
  const refTestPath = path.join(templatesDir, "reference_example.test.ts");

  if (!fs.existsSync(systemPromptPath) || !fs.existsSync(refImplPath) || !fs.existsSync(refTestPath)) {
    throw new Error(`Prompt template files not found in: ${templatesDir}`);
  }

  let systemPrompt = fs.readFileSync(systemPromptPath, "utf8");
  const refImpl = fs.readFileSync(refImplPath, "utf8");
  const refTest = fs.readFileSync(refTestPath, "utf8");

  const additionsText = promptAdditions ? `\n\n${promptAdditions}\n` : "";

  systemPrompt = systemPrompt
    .replace("{{reference_impl}}", refImpl.trim())
    .replace("{{reference_test}}", refTest.trim())
    .replace("{{additions}}", additionsText);

  return systemPrompt;
}

function formatTypeRef(ref: TypeRef, typeMappings?: TypeMapping[]): string {
  if (typeMappings) {
    const mapping = typeMappings.find(m => m.omni_type.toLowerCase() === ref.name.toLowerCase());
    if (mapping) {
      return mapping.target_type;
    }
  }

  const nameLower = ref.name.toLowerCase();
  if (nameLower === "string" || nameLower === "email" || nameLower === "url" || nameLower === "uuid") {
    return "string";
  }
  if (nameLower === "int" || nameLower === "float" || nameLower === "decimal" || nameLower === "money" || nameLower === "duration") {
    return "number";
  }
  if (nameLower === "bool" || nameLower === "boolean") {
    return "boolean";
  }
  if (nameLower === "datetime" || nameLower === "date") {
    return "Date";
  }
  if (nameLower === "bytes") {
    return "Buffer";
  }
  if (nameLower === "void") {
    return "void";
  }
  if (nameLower === "list") {
    const arg = ref.type_args[0] ? formatTypeRef(ref.type_args[0], typeMappings) : "any";
    return `${arg}[]`;
  }
  if (nameLower === "option") {
    const arg = ref.type_args[0] ? formatTypeRef(ref.type_args[0], typeMappings) : "any";
    return `${arg} | null`;
  }

  // Handle generic type arguments (user-defined generic type)
  if (ref.type_args.length > 0) {
    const args = ref.type_args.map(arg => formatTypeRef(arg, typeMappings)).join(", ");
    return `${toPascalCaseType(ref.name)}<${args}>`;
  }

  // User-defined type reference — match the PascalCase name emitted in types.ts.
  return toPascalCaseType(ref.name);
}

/// Converts an OmniLang type name to an idiomatic PascalCase TypeScript type
/// name (`accountId` → `AccountId`, `order_item` → `OrderItem`). Applied
/// consistently to both the emitted `types.ts` and the prompt so generated code
/// references the exact names that exist.
export function toPascalCaseType(name: string): string {
  return name
    .split(/[_\s-]+/)
    .map((s) => (s.length ? s.charAt(0).toUpperCase() + s.slice(1) : s))
    .join("");
}

export function formatTypeDecl(t: TypeDecl, typeMappings?: TypeMapping[]): string {
  const name = toPascalCaseType(t.name);
  const kind = t.kind;

  if ("Alias" in kind) {
    return `export type ${name} = ${formatTypeRef(kind.Alias, typeMappings)};`;
  }

  if ("Struct" in kind) {
    const fields = kind.Struct.fields.map(f => {
      const doc = f.doc_comment ? `  /** ${f.doc_comment} */\n` : "";
      return `${doc}  ${f.name}: ${formatTypeRef(f.ty, typeMappings)};`;
    }).join("\n");
    return `export interface ${name} {\n${fields}\n}`;
  }

  if ("Enum" in kind) {
    const variants = kind.Enum.variants.map(v => {
      return `  ${v.name} = '${v.name}',`;
    }).join("\n");
    return `export enum ${name} {\n${variants}\n}`;
  }

  if ("Refined" in kind) {
    const base = kind.Refined.base ? formatTypeRef(kind.Refined.base, typeMappings) : "any";
    return `export type ${name} = ${base}; // Refined type with constraints`;
  }

  return `export type ${name} = any;`;
}

function getGoSystemPrompt(promptAdditions: string = ""): string {
  const additionsText = promptAdditions ? `\n\n${promptAdditions}\n` : "";
  return `You are a Senior Go and Test Engineer.
Your task is to generate a verified Go implementation and matching unit tests based on an OmniLang specification.

You MUST follow these rules:
1. Generate clean, idiomatic Go code.
2. Implement all operations defined in the service spec.
3. Validate inputs and enforce preconditions and postconditions.
4. If the service declares invariants, verify them before and after each operation call.
5. If the service declares custom error types, implement them as sentinel errors using 'errors.New' or custom types implementing the 'error' interface.
6. Write comprehensive unit tests using Go's standard 'testing' package covering all success scenarios and error cases defined in the test blocks.
7. Use directory-based packages: place Go files inside the 'services/' directory under package 'services'.

${additionsText}
Return your output EXACTLY as a JSON object with the following structure, and do not wrap it in markdown code blocks or add any conversational text.

JSON Structure:
{
  "files": [
    {
      "path": "services/<service_name_snake>.go",
      "content": "// Put the full, complete Go implementation code here"
    },
    {
      "path": "services/<service_name_snake>_test.go",
      "content": "// Put the full, complete Go test code here"
    }
  ]
}`;
}

function formatGoTypeRef(ref: TypeRef): string {
  if (ref.name === "String" || ref.name === "Email" || ref.name === "URL" || ref.name === "UUID") {
    return "string";
  }
  if (ref.name === "Int") {
    return "int";
  }
  if (ref.name === "Float" || ref.name === "Decimal" || ref.name === "Money") {
    return "float64";
  }
  if (ref.name === "Bool") {
    return "bool";
  }
  if (ref.name === "DateTime" || ref.name === "Date" || ref.name === "Timestamp") {
    return "string";
  }
  if (ref.name === "Duration") {
    return "string";
  }
  if (ref.name === "Bytes") {
    return "[]byte";
  }
  if (ref.name === "Void") {
    return "";
  }
  if (ref.name === "List") {
    const arg = ref.type_args[0] ? formatGoTypeRef(ref.type_args[0]) : "string";
    return `[]${arg}`;
  }
  if (ref.name === "Option") {
    const arg = ref.type_args[0] ? formatGoTypeRef(ref.type_args[0]) : "string";
    return `*${arg}`;
  }
  if (ref.type_args.length > 0) {
    const args = ref.type_args.map(arg => formatGoTypeRef(arg)).join(", ");
    return `${ref.name}[${args}]`;
  }
  return ref.name;
}

export function formatGoTypeDecl(t: TypeDecl): string {
  const name = t.name;
  const kind = t.kind;

  if ("Alias" in kind) {
    return `type ${name} ${formatGoTypeRef(kind.Alias)}`;
  }

  if ("Struct" in kind) {
    const fields = kind.Struct.fields.map(f => {
      const doc = f.doc_comment ? `  // ${f.doc_comment}\n` : "";
      const capitalizedFieldName = f.name.charAt(0).toUpperCase() + f.name.slice(1);
      return `${doc}  ${capitalizedFieldName} ${formatGoTypeRef(f.ty)} \`json:"${f.name}"\``;
    }).join("\n");
    return `type ${name} struct {\n${fields}\n}`;
  }

  if ("Enum" in kind) {
    const variants = kind.Enum.variants.map(v => {
      return `  ${name}${v.name} ${name} = "${v.name}"`;
    }).join("\n");
    return `type ${name} string\n\nconst (\n${variants}\n)`;
  }

  if ("Refined" in kind) {
    const base = kind.Refined.base ? formatGoTypeRef(kind.Refined.base) : "string";
    return `type ${name} ${base} // Refined type with constraints`;
  }

  return `type ${name} struct{}`;
}
