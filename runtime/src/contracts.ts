/**
 * Contract enumeration and coverage enforcement.
 *
 * Every precondition, postcondition, and invariant declared on a service is
 * assigned a stable `contract_id`. Generated code/tests are expected to mark the
 * check that enforces each contract with a comment `// @omni:contract <id>`.
 * The build can then prove that every declared contract has at least one
 * executable check, rather than trusting that the LLM honored the spec.
 */

export type ContractKind = "precondition" | "postcondition" | "invariant";
export type ContractClassification = "formal" | "nl";

export interface ContractItem {
  id: string;
  kind: ContractKind;
  /** Operation name, or "service" for invariants. */
  scope: string;
  text: string;
  classification: ContractClassification;
}

/** Best-effort: does an expression reference a string literal (natural language)? */
function literalString(expr: any): string | null {
  if (expr && typeof expr === "object" && expr.Literal && typeof expr.Literal.String === "string") {
    return expr.Literal.String;
  }
  return null;
}

/** Named invariant shape: `name: "<text>"` parses as BinaryOp(Identifier Eq String). */
function namedInvariant(expr: any): { name: string; text: string } | null {
  const bin = expr?.BinaryOp;
  if (bin && bin.left?.Identifier && literalString(bin.right) !== null) {
    const name = Array.isArray(bin.left.Identifier) ? bin.left.Identifier[0] : bin.left.Identifier;
    return { name, text: literalString(bin.right) as string };
  }
  return null;
}

/** A contract is "nl" if its semantic content is a free-text string, else "formal". */
function classify(expr: any): ContractClassification {
  if (literalString(expr) !== null) return "nl";
  const named = namedInvariant(expr);
  if (named) return "nl";
  return "formal";
}

function exprText(expr: any): string {
  const lit = literalString(expr);
  if (lit !== null) return lit;
  const named = namedInvariant(expr);
  if (named) return `${named.name}: ${named.text}`;
  // Fall back to a compact rendering of a formal expression.
  const bin = expr?.BinaryOp;
  if (bin) {
    return `${ident(bin.left)} ${bin.op} ${ident(bin.right)}`;
  }
  return JSON.stringify(expr);
}

function ident(expr: any): string {
  if (expr?.Identifier) {
    return Array.isArray(expr.Identifier) ? expr.Identifier[0] : expr.Identifier;
  }
  const lit = literalString(expr);
  if (lit !== null) return `"${lit}"`;
  if (expr?.Literal?.Int !== undefined) return String(expr.Literal.Int);
  if (expr?.Literal?.Float !== undefined) return String(expr.Literal.Float);
  return "…";
}

/** Enumerates every contract on a service with a stable id. */
export function enumerateContracts(service: any): ContractItem[] {
  const items: ContractItem[] = [];
  const svc = service?.name ?? "service";

  for (const op of service?.operations ?? []) {
    (op.preconditions ?? []).forEach((e: any, i: number) => {
      items.push({
        id: `${svc}.${op.name}.pre.${i + 1}`,
        kind: "precondition",
        scope: op.name,
        text: exprText(e),
        classification: classify(e),
      });
    });
    (op.postconditions ?? []).forEach((e: any, i: number) => {
      items.push({
        id: `${svc}.${op.name}.post.${i + 1}`,
        kind: "postcondition",
        scope: op.name,
        text: exprText(e),
        classification: classify(e),
      });
    });
  }

  (service?.invariants ?? []).forEach((e: any, i: number) => {
    const named = namedInvariant(e);
    const suffix = named ? named.name : String(i + 1);
    items.push({
      id: `${svc}.inv.${suffix}`,
      kind: "invariant",
      scope: "service",
      text: exprText(e),
      classification: classify(e),
    });
  });

  return items;
}

/** Renders the contracts as a prompt section instructing the LLM to mark checks. */
export function formatContractsForPrompt(contracts: ContractItem[]): string {
  if (contracts.length === 0) return "";
  const lines = contracts.map(
    (c) => `  - [${c.id}] (${c.kind}, ${c.classification}) ${c.text}`,
  );
  return `
Contracts to enforce (each MUST have at least one executable check, and you MUST
annotate the enforcing line with a comment \`// @omni:contract <id>\`):
${lines.join("\n")}
`;
}

/** Scans generated file contents for `@omni:contract <id>` markers. */
export function checkContractCoverage(
  contracts: ContractItem[],
  fileContents: string[],
): { covered: string[]; uncovered: string[] } {
  const haystack = fileContents.join("\n");
  const covered: string[] = [];
  const uncovered: string[] = [];
  for (const c of contracts) {
    if (haystack.includes(`@omni:contract ${c.id}`)) {
      covered.push(c.id);
    } else {
      uncovered.push(c.id);
    }
  }
  return { covered, uncovered };
}
