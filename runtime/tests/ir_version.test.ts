import { assertIrVersion, SUPPORTED_IR_VERSION } from "../src/orchestrator";
import { SpecIR } from "../src/types";

function makeIr(version: string | undefined): SpecIR {
  return {
    ir_version: version as string,
    module_path: ["acme", "test"],
    source_file: {} as any,
    types: [],
    services: [],
    build_order: [],
    type_mappings: [],
    stats: {} as any,
  };
}

describe("assertIrVersion", () => {
  it("accepts IR stamped with the supported version", () => {
    expect(() => assertIrVersion(makeIr(SUPPORTED_IR_VERSION), "test.json")).not.toThrow();
  });

  it("rejects IR stamped with a different version", () => {
    expect(() => assertIrVersion(makeIr("999"), "test.json")).toThrow(/Incompatible Spec IR version/);
  });

  it("tolerates legacy IR without a version stamp", () => {
    expect(() => assertIrVersion(makeIr(undefined), "legacy.json")).not.toThrow();
  });
});
