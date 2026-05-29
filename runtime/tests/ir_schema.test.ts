import { validateIrSchema } from "../src/orchestrator";

function minimalValidIr(): unknown {
  return {
    ir_version: "1",
    module_path: ["acme", "test"],
    source_file: {
      module: { path: ["acme", "test"], span: { start: 0, end: 0 } },
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
}

describe("validateIrSchema", () => {
  it("accepts a structurally valid IR document", () => {
    expect(() => validateIrSchema(minimalValidIr(), "valid.json")).not.toThrow();
  });

  it("rejects an empty object with a clear error", () => {
    expect(() => validateIrSchema({}, "empty.json")).toThrow(/does not match the IR schema/);
  });

  it("rejects IR missing a required top-level field", () => {
    const ir = minimalValidIr() as Record<string, unknown>;
    delete ir.services;
    expect(() => validateIrSchema(ir, "partial.json")).toThrow(/does not match the IR schema/);
  });

  it("rejects IR with a wrong field type", () => {
    const ir = minimalValidIr() as Record<string, unknown>;
    ir.module_path = "not-an-array";
    expect(() => validateIrSchema(ir, "badtype.json")).toThrow(/does not match the IR schema/);
  });
});
