import { spawnSync } from "child_process";
import {
  generateRustProperty,
  generatePythonProperty,
  generateGoProperty,
} from "../src/testing/properties";
import { TypeDef } from "../src/types";

/**
 * These tests close the parity gap noted for non-TypeScript targets: the
 * TypeScript property test is self-verified by running `fast-check`, but the
 * Rust/Python/Go generators were previously only checked structurally (string
 * `toContain`). Here we feed the emitted source through each language's real
 * syntax validator (`python3 -c "import ast"`, `gofmt`, `rustfmt`) so a
 * malformed template is caught, not just a missing substring.
 *
 * Each check skips gracefully when its toolchain is absent so the suite stays
 * green on minimal environments, while running for real wherever the toolchain
 * exists (local dev + CI, which install Python, Go and the Rust toolchain).
 */

function toolAvailable(bin: string): boolean {
  const probe = spawnSync(bin, ["--help"], { stdio: "ignore" });
  // `gofmt` has no --help and exits non-zero, but spawning still succeeds;
  // ENOENT (binary missing) surfaces as `error`.
  return !probe.error;
}

/** Runs `bin args…` with `source` on stdin; returns {ok, stderr}. */
function checkSyntax(bin: string, args: string[], source: string): { ok: boolean; stderr: string } {
  const res = spawnSync(bin, args, { input: source, encoding: "utf8" });
  return { ok: res.status === 0, stderr: res.stderr ?? "" };
}

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

describe("emitted property tests pass each language's syntax validator", () => {
  const pyIt = toolAvailable("python3") ? it : it.skip;
  pyIt("python hypothesis output parses (ast.parse)", () => {
    for (const t of [numericType, lengthType]) {
      const code = generatePythonProperty(t)!;
      const { ok, stderr } = checkSyntax(
        "python3",
        ["-c", "import ast,sys; ast.parse(sys.stdin.read())"],
        code,
      );
      expect(stderr).toBe("");
      expect(ok).toBe(true);
    }
  });

  const goIt = toolAvailable("gofmt") ? it : it.skip;
  goIt("go gopter output parses (gofmt)", () => {
    const code = generateGoProperty(numericType)!;
    const { ok, stderr } = checkSyntax("gofmt", [], code);
    expect(stderr).toBe("");
    expect(ok).toBe(true);
  });

  const rsIt = toolAvailable("rustfmt") ? it : it.skip;
  rsIt("rust proptest output parses (rustfmt)", () => {
    for (const t of [numericType, lengthType]) {
      const code = generateRustProperty(t)!;
      const { ok, stderr } = checkSyntax("rustfmt", ["--edition", "2021", "--emit", "stdout"], code);
      expect(stderr).toBe("");
      expect(ok).toBe(true);
    }
  });
});
