import { CodeGenAgent } from "../src/agents/codegen";
import { LLMProvider } from "../src/providers";
import { SpecIR } from "../src/types";
import * as fs from "fs";
import * as path from "path";

jest.mock("fs", () => {
  const original = jest.requireActual("fs");
  return {
    ...original,
    existsSync: jest.fn(),
    mkdirSync: jest.fn(),
    writeFileSync: jest.fn(),
    readFileSync: jest.fn(),
    appendFileSync: jest.fn(),
  };
});

jest.mock("../src/prompts/codegen", () => ({
  getSystemPrompt: jest.fn().mockReturnValue("system prompt"),
  getUserPrompt: jest.fn().mockReturnValue("user prompt"),
}));

describe("CodeGenAgent", () => {
  let mockProvider: LLMProvider;
  let agent: CodeGenAgent;
  const mockExistsSync = fs.existsSync as jest.Mock;
  const mockReadFileSync = fs.readFileSync as jest.Mock;
  const mockAppendFileSync = fs.appendFileSync as jest.Mock;

  const mockIr: SpecIR = {
    ir_version: "1",
    module_path: ["test"],
    source_file: {
      module: { path: ["test"], span: { start: 0, end: 0 } },
      imports: [],
      exports: [],
      declarations: [],
    },
    types: [],
    services: [],
    build_order: [],
    type_mappings: [],
    stats: {
      type_count: 0,
      service_count: 0,
      operation_count: 0,
      test_count: 0,
      constraint_count: 0,
      metric_count: 0,
      component_count: 0,
      pipeline_count: 0,
      workflow_count: 0,
      agent_count: 0,
      schema_count: 0,
      policy_count: 0,
    },
  };

  beforeEach(() => {
    jest.resetAllMocks();
    mockProvider = {
      generateCode: jest.fn(),
    };
    agent = new CodeGenAgent(mockProvider);
  });

  test("should successfully generate files when LLM returns clean JSON", async () => {
    const cleanJson = JSON.stringify({
      files: [
        { path: "src/services/UserService.ts", content: "export class UserService {}" },
        { path: "tests/UserService.test.ts", content: "describe('UserService', () => {})" },
      ],
    });

    (mockProvider.generateCode as jest.Mock).mockResolvedValue(cleanJson);

    const result = await agent.generateService("UserService", mockIr, "test-output", "typescript");

    expect(result.success).toBe(true);
    expect(result.files).toHaveLength(2);
    expect(result.files[0]).toBe(path.join("test-output", "src/services/UserService.ts"));
    expect(fs.writeFileSync).toHaveBeenCalledTimes(2);
  });

  test("should extract JSON from markdown code blocks", async () => {
    const responseWithMarkdown = `
    Some conversational text from model...
    \`\`\`json
    {
      "files": [
        { "path": "a.ts", "content": "const x = 1;" }
      ]
    }
    \`\`\`
    And some closing text...
    `;

    (mockProvider.generateCode as jest.Mock).mockResolvedValue(responseWithMarkdown);

    const result = await agent.generateService("UserService", mockIr, "test-output", "typescript");

    expect(result.success).toBe(true);
    expect(result.files).toHaveLength(1);
    expect(fs.writeFileSync).toHaveBeenCalledWith(
      path.join("test-output", "a.ts"),
      "const x = 1;",
      "utf8"
    );
  });

  test("should preserve template-literal backticks inside string content", async () => {
    // Regression: valid JSON whose content contains a template literal
    // (`Hello, ${name}!`) must not be mangled by backtick→quote recovery.
    const content =
      "export function greet(name: string) { return `Hello, ${name}!`; }";
    const responseWithMarkdown =
      "```json\n" +
      JSON.stringify({ files: [{ path: "src/greet.ts", content }] }) +
      "\n```";

    (mockProvider.generateCode as jest.Mock).mockResolvedValue(responseWithMarkdown);

    const result = await agent.generateService("Greet", mockIr, "test-output", "typescript");

    expect(result.success).toBe(true);
    expect(result.files).toHaveLength(1);
    expect(fs.writeFileSync).toHaveBeenCalledWith(
      path.join("test-output", "src/greet.ts"),
      content,
      "utf8"
    );
  });

  test("should fall back to searching for braces if no markdown code block is found", async () => {
    const responseWithoutMarkdown = `
    Intro text
    {
      "files": [
        { "path": "b.ts", "content": "const y = 2;" }
      ]
    }
    Outro text
    `;

    (mockProvider.generateCode as jest.Mock).mockResolvedValue(responseWithoutMarkdown);

    const result = await agent.generateService("UserService", mockIr, "test-output", "typescript");

    expect(result.success).toBe(true);
    expect(result.files).toHaveLength(1);
  });

  test("should recover from trailing commas and unclosed brackets/braces", async () => {
    const malformedJson = `
    \`\`\`json
    {
      "files": [
        { "path": "c.ts", "content": "const z = 3;" }
      ],
    }
    \`\`\`
    `; // trailing comma before closing brace

    (mockProvider.generateCode as jest.Mock).mockResolvedValue(malformedJson);

    const result = await agent.generateService("UserService", mockIr, "test-output", "typescript");

    expect(result.success).toBe(true);
    expect(result.files).toHaveLength(1);
    expect(result.files[0]).toBe(path.join("test-output", "c.ts"));
  });

  test("should normalize response from impl/test key shapes", async () => {
    const alternativeShape = JSON.stringify({
      implementation: {
        path: "src/impl.ts",
        content: "impl",
      },
      tests: {
        path: "tests/test.ts",
        content: "test",
      },
    });

    (mockProvider.generateCode as jest.Mock).mockResolvedValue(alternativeShape);

    const result = await agent.generateService("UserService", mockIr, "test-output", "typescript");

    expect(result.success).toBe(true);
    expect(result.files).toHaveLength(2);
    expect(result.files[0]).toBe(path.join("test-output", "src/impl.ts"));
    expect(result.files[1]).toBe(path.join("test-output", "tests/test.ts"));
  });

  test("should fail if response structure is totally invalid", async () => {
    const invalidShape = JSON.stringify({
      invalidKey: "invalidValue",
    });

    (mockProvider.generateCode as jest.Mock).mockResolvedValue(invalidShape);

    await expect(
      agent.generateService("UserService", mockIr, "test-output", "typescript")
    ).rejects.toThrow("Response must contain a 'files' array");
  });

  test("should append service module declaration to Rust mod.rs", async () => {
    const cleanJson = JSON.stringify({
      files: [
        { path: "src/services/user_service.rs", content: "pub struct UserService;" },
      ],
    });

    (mockProvider.generateCode as jest.Mock).mockResolvedValue(cleanJson);
    mockExistsSync.mockReturnValue(true);
    mockReadFileSync.mockReturnValue("pub mod other_service;\n");

    const result = await agent.generateService("UserService", mockIr, "test-output", "rust");

    expect(result.success).toBe(true);
    expect(fs.appendFileSync).toHaveBeenCalledWith(
      path.join("test-output", "src", "services", "mod.rs"),
      "pub mod user_service;\n"
    );
  });

  test("should NOT append duplicate service module declaration to Rust mod.rs", async () => {
    const cleanJson = JSON.stringify({
      files: [
        { path: "src/services/user_service.rs", content: "pub struct UserService;" },
      ],
    });

    (mockProvider.generateCode as jest.Mock).mockResolvedValue(cleanJson);
    mockExistsSync.mockReturnValue(true);
    mockReadFileSync.mockReturnValue("pub mod user_service;\n");

    await agent.generateService("UserService", mockIr, "test-output", "rust");

    expect(fs.appendFileSync).not.toHaveBeenCalled();
  });
});
