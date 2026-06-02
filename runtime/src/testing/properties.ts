/**
 * Property-test generation from refined-type constraints.
 *
 * A refined type carries a `GeneratorConfig` (min/max/length/format/precision)
 * in the Spec IR. Rather than trusting the LLM to test the refinement, we derive
 * `fast-check` property tests deterministically from the config, so the
 * refinement's invariants are checked over many generated inputs.
 */

import { GeneratorConfig, TypeDef } from "../types";

/** Builds a fast-check arbitrary expression matching the refinement. */
export function generateArbitrary(cfg: GeneratorConfig): string {
  if (cfg.min != null || cfg.max != null) {
    const opts: string[] = [];
    if (cfg.min != null) opts.push(`min: ${cfg.min}`);
    if (cfg.max != null) opts.push(`max: ${cfg.max}`);
    const base = cfg.precision != null ? "fc.double" : "fc.integer";
    return `${base}({ ${opts.join(", ")} })`;
  }
  if (cfg.min_length != null || cfg.max_length != null) {
    const opts: string[] = [];
    if (cfg.min_length != null) opts.push(`minLength: ${cfg.min_length}`);
    if (cfg.max_length != null) opts.push(`maxLength: ${cfg.max_length}`);
    return `fc.string({ ${opts.join(", ")} })`;
  }
  if (cfg.format_pattern) {
    return `fc.stringMatching(/${cfg.format_pattern}/)`;
  }
  return "fc.anything()";
}

/** Property assertions that must hold for any generated value of the type. */
export function generateAssertions(cfg: GeneratorConfig): string[] {
  const a: string[] = [];
  if (cfg.min != null) a.push(`expect(value).toBeGreaterThanOrEqual(${cfg.min});`);
  if (cfg.max != null) a.push(`expect(value).toBeLessThanOrEqual(${cfg.max});`);
  if (cfg.min_length != null) a.push(`expect(value.length).toBeGreaterThanOrEqual(${cfg.min_length});`);
  if (cfg.max_length != null) a.push(`expect(value.length).toBeLessThanOrEqual(${cfg.max_length});`);
  if (cfg.format_pattern) a.push(`expect(value).toMatch(/${cfg.format_pattern}/);`);
  return a;
}

/**
 * Generates a jest + fast-check property test for a refined type, or `null` if
 * the type has no generator config.
 */
export function generateTypeScriptProperty(typeDef: TypeDef): string | null {
  const cfg = typeDef.generator;
  if (!cfg) return null;
  const arb = generateArbitrary(cfg);
  const assertions = generateAssertions(cfg);
  if (assertions.length === 0) return null;
  return [
    `import fc from "fast-check";`,
    ``,
    `// Auto-generated from the refinement on type '${typeDef.name}'.`,
    `describe("${typeDef.name} refinement", () => {`,
    `  it("every generated value satisfies the refinement", () => {`,
    `    fc.assert(`,
    `      fc.property(${arb}, (value) => {`,
    ...assertions.map((s) => `        ${s}`),
    `      }),`,
    `    );`,
    `  });`,
    `});`,
    ``,
  ].join("\n");
}

const NUMERIC_MIN = (cfg: GeneratorConfig) => cfg.min ?? cfg.min_length;
const NUMERIC_MAX = (cfg: GeneratorConfig) => cfg.max ?? cfg.max_length;
const IS_LENGTH = (cfg: GeneratorConfig) =>
  cfg.min == null && cfg.max == null && (cfg.min_length != null || cfg.max_length != null);

/** Rust `proptest` property test from a numeric/length refinement. */
export function generateRustProperty(typeDef: TypeDef): string | null {
  const cfg = typeDef.generator;
  if (!cfg) return null;
  const min = NUMERIC_MIN(cfg);
  const max = NUMERIC_MAX(cfg);
  if (min == null && max == null) return null;
  const lo = min ?? 0;
  const hi = max ?? 1_000_000;
  const accessor = IS_LENGTH(cfg) ? "value.chars().count() as i64" : "value";
  const strategy = IS_LENGTH(cfg) ? `".{${lo},${hi}}"` : `${lo}i64..=${hi}i64`;
  const param = IS_LENGTH(cfg) ? "value in " + strategy : "value in " + strategy;
  return [
    `use proptest::prelude::*;`,
    ``,
    `// Auto-generated from the refinement on type '${typeDef.name}'.`,
    `proptest! {`,
    `    #[test]`,
    `    fn ${snake(typeDef.name)}_refinement(${param}) {`,
    `        let n = ${accessor};`,
    ...(min != null ? [`        prop_assert!(n >= ${lo});`] : []),
    ...(max != null ? [`        prop_assert!(n <= ${hi});`] : []),
    `    }`,
    `}`,
    ``,
  ].join("\n");
}

/** Python `hypothesis` property test from a numeric/length refinement. */
export function generatePythonProperty(typeDef: TypeDef): string | null {
  const cfg = typeDef.generator;
  if (!cfg) return null;
  const length = IS_LENGTH(cfg);
  const min = NUMERIC_MIN(cfg);
  const max = NUMERIC_MAX(cfg);
  if (min == null && max == null) return null;
  const strat = length
    ? `st.text(min_size=${cfg.min_length ?? 0}, max_size=${cfg.max_length ?? 1024})`
    : `st.integers(min_value=${cfg.min ?? "-(10**9)"}, max_value=${cfg.max ?? "10**9"})`;
  const accessor = length ? "len(value)" : "value";
  const asserts: string[] = [];
  if (cfg.min != null) asserts.push(`    assert value >= ${cfg.min}`);
  if (cfg.max != null) asserts.push(`    assert value <= ${cfg.max}`);
  if (cfg.min_length != null) asserts.push(`    assert len(value) >= ${cfg.min_length}`);
  if (cfg.max_length != null) asserts.push(`    assert len(value) <= ${cfg.max_length}`);
  return [
    `from hypothesis import given, strategies as st`,
    ``,
    `# Auto-generated from the refinement on type '${typeDef.name}'.`,
    `@given(${strat})`,
    `def test_${snake(typeDef.name)}_refinement(value):`,
    `    _ = ${accessor}`,
    ...asserts,
    ``,
  ].join("\n");
}

/** Go `gopter` property test from a numeric refinement. */
export function generateGoProperty(typeDef: TypeDef): string | null {
  const cfg = typeDef.generator;
  if (!cfg) return null;
  if (IS_LENGTH(cfg) || cfg.min == null || cfg.max == null) return null; // gopter int range only
  const name = pascal(typeDef.name);
  return [
    `package services`,
    ``,
    `import (`,
    `\t"testing"`,
    ``,
    `\t"github.com/leanovate/gopter"`,
    `\t"github.com/leanovate/gopter/gen"`,
    `\t"github.com/leanovate/gopter/prop"`,
    `)`,
    ``,
    `// Auto-generated from the refinement on type '${typeDef.name}'.`,
    `func Test${name}Refinement(t *testing.T) {`,
    `\tprops := gopter.NewProperties(nil)`,
    `\tprops.Property("${typeDef.name} refinement", prop.ForAll(`,
    `\t\tfunc(value int) bool { return value >= ${cfg.min} && value <= ${cfg.max} },`,
    `\t\tgen.IntRange(${cfg.min}, ${cfg.max}),`,
    `\t))`,
    `\tprops.TestingRun(t)`,
    `}`,
    ``,
  ].join("\n");
}

function snake(name: string): string {
  return name.replace(/([A-Z])/g, "_$1").toLowerCase().replace(/^_/, "");
}
function pascal(name: string): string {
  return name.charAt(0).toUpperCase() + name.slice(1);
}

/** Dispatches to the per-target property-test generator. */
export function generatePropertyTest(
  typeDef: TypeDef,
  target: "typescript" | "rust" | "python" | "go" = "typescript",
): string | null {
  switch (target) {
    case "rust":
      return generateRustProperty(typeDef);
    case "python":
      return generatePythonProperty(typeDef);
    case "go":
      return generateGoProperty(typeDef);
    default:
      return generateTypeScriptProperty(typeDef);
  }
}
