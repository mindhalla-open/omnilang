import fc from "fast-check";
import {
  generateArbitrary,
  generateAssertions,
  generatePropertyTest,
  generateRustProperty,
  generatePythonProperty,
  generateGoProperty,
} from "../src/testing/properties";
import { TypeDef } from "../src/types";

const numericType: TypeDef = {
  name: "score",
  kind: "refined",
  field_count: 0,
  generator: { min: 0, max: 100 },
};
const lengthType: TypeDef = {
  name: "orderId",
  kind: "refined",
  field_count: 0,
  generator: { min_length: 5, max_length: 12 },
};

describe("per-target property generators", () => {
  it("rust proptest covers numeric ranges and string length", () => {
    const r = generateRustProperty(numericType)!;
    expect(r).toContain("use proptest::prelude::*;");
    expect(r).toContain("0i64..=100i64");
    expect(r).toContain("prop_assert!(n >= 0)");
    const rl = generateRustProperty(lengthType)!;
    expect(rl).toContain(".{5,12}");
    expect(rl).toContain("chars().count()");
  });

  it("python hypothesis covers integers and text", () => {
    const p = generatePythonProperty(numericType)!;
    expect(p).toContain("from hypothesis import given, strategies as st");
    expect(p).toContain("st.integers(min_value=0, max_value=100)");
    expect(p).toContain("assert value >= 0");
    const pl = generatePythonProperty(lengthType)!;
    expect(pl).toContain("st.text(min_size=5, max_size=12)");
    expect(pl).toContain("assert len(value) >= 5");
  });

  it("go gopter covers numeric ranges", () => {
    const g = generateGoProperty(numericType)!;
    expect(g).toContain("github.com/leanovate/gopter");
    expect(g).toContain("gen.IntRange(0, 100)");
    expect(g).toContain("func TestScoreRefinement");
    // gopter int-range only: length refinements are not generated.
    expect(generateGoProperty(lengthType)).toBeNull();
  });

  it("dispatches by target", () => {
    expect(generatePropertyTest(numericType, "rust")).toContain("proptest");
    expect(generatePropertyTest(numericType, "python")).toContain("hypothesis");
    expect(generatePropertyTest(numericType, "go")).toContain("gopter");
    expect(generatePropertyTest(numericType, "typescript")).toContain("fast-check");
  });
});

describe("generateArbitrary", () => {
  it("maps numeric ranges to fc.integer / fc.double", () => {
    expect(generateArbitrary({ min: 0, max: 9999 })).toBe("fc.integer({ min: 0, max: 9999 })");
    expect(generateArbitrary({ min: 0, max: 1, precision: 2 })).toBe("fc.double({ min: 0, max: 1 })");
  });

  it("maps string length to fc.string", () => {
    expect(generateArbitrary({ min_length: 5, max_length: 10 })).toBe(
      "fc.string({ minLength: 5, maxLength: 10 })",
    );
  });

  it("maps a format pattern to fc.stringMatching", () => {
    expect(generateArbitrary({ format_pattern: "^[0-9]+$" })).toBe("fc.stringMatching(/^[0-9]+$/)");
  });
});

describe("generatePropertyTest", () => {
  it("returns null when there is no generator config", () => {
    const td: TypeDef = { name: "Plain", kind: "alias", field_count: 0 };
    expect(generatePropertyTest(td)).toBeNull();
  });

  it("emits a fast-check test with the refinement assertions", () => {
    const td: TypeDef = {
      name: "OrderId",
      kind: "refined",
      field_count: 0,
      generator: { min_length: 5, max_length: 12 },
    };
    const code = generatePropertyTest(td)!;
    expect(code).toContain('import fc from "fast-check"');
    expect(code).toContain("OrderId refinement");
    expect(code).toContain("toBeGreaterThanOrEqual(5)");
    expect(code).toContain("toBeLessThanOrEqual(12)");
  });
});

// Self-verification: the arbitrary + assertions the generator emits actually hold.
describe("generated property holds under fast-check", () => {
  it("numeric range", () => {
    const cfg = { min: 5, max: 10 };
    expect(generateAssertions(cfg)).toHaveLength(2);
    fc.assert(
      fc.property(fc.integer({ min: 5, max: 10 }), (value) => {
        expect(value).toBeGreaterThanOrEqual(5);
        expect(value).toBeLessThanOrEqual(10);
      }),
    );
  });

  it("string length", () => {
    fc.assert(
      fc.property(fc.string({ minLength: 3, maxLength: 8 }), (value) => {
        expect(value.length).toBeGreaterThanOrEqual(3);
        expect(value.length).toBeLessThanOrEqual(8);
      }),
    );
  });
});
