import {
  enumerateContracts,
  checkContractCoverage,
  formatContractsForPrompt,
} from "../src/contracts";

// Mirrors the AST shape the analyzer emits for hybrid_billing's paymentService.
const service = {
  name: "paymentService",
  operations: [
    {
      name: "Deposit",
      preconditions: [{ Literal: { String: "Deposit amount must be > 0" } }],
      postconditions: [{ Literal: { String: "balance increases by amount" } }],
    },
    {
      name: "Charge",
      preconditions: [
        { Literal: { String: "balance >= amount" } },
        { Literal: { String: "Account must be Active" } },
      ],
      postconditions: [],
    },
  ],
  invariants: [
    {
      BinaryOp: {
        left: { Identifier: ["balance_safety", { start: 0, end: 0 }] },
        op: "Eq",
        right: { Literal: { String: "account.balance >= 0" } },
      },
    },
    {
      BinaryOp: {
        left: { Identifier: ["balance_formal", { start: 0, end: 0 }] },
        op: "Ge",
        right: { Literal: { Int: 0 } },
      },
    },
  ],
};

describe("enumerateContracts", () => {
  it("assigns stable ids per operation and invariant", () => {
    const ids = enumerateContracts(service).map((c) => c.id);
    // Named invariants (`name: "text"`) use the name; a bare formal invariant
    // is positional (its left operand is a variable, not a name).
    expect(ids).toEqual([
      "paymentService.Deposit.pre.1",
      "paymentService.Deposit.post.1",
      "paymentService.Charge.pre.1",
      "paymentService.Charge.pre.2",
      "paymentService.inv.balance_safety",
      "paymentService.inv.2",
    ]);
  });

  it("classifies string contracts as nl and numeric expressions as formal", () => {
    const byId = Object.fromEntries(enumerateContracts(service).map((c) => [c.id, c]));
    expect(byId["paymentService.Deposit.pre.1"].classification).toBe("nl");
    expect(byId["paymentService.inv.balance_safety"].classification).toBe("nl");
    expect(byId["paymentService.inv.2"].classification).toBe("formal");
  });

  it("returns nothing for a service without contracts", () => {
    expect(enumerateContracts({ name: "S", operations: [], invariants: [] })).toEqual([]);
  });
});

describe("checkContractCoverage", () => {
  const contracts = enumerateContracts(service);

  it("reports a contract covered when its marker is present", () => {
    const file = "// @omni:contract paymentService.Deposit.pre.1\nif (x) {}";
    const { covered, uncovered } = checkContractCoverage(contracts, [file]);
    expect(covered).toContain("paymentService.Deposit.pre.1");
    expect(uncovered).toContain("paymentService.Charge.pre.1");
  });

  it("reports all uncovered when no markers exist", () => {
    const { covered, uncovered } = checkContractCoverage(contracts, ["no markers here"]);
    expect(covered).toHaveLength(0);
    expect(uncovered).toHaveLength(contracts.length);
  });
});

describe("formatContractsForPrompt", () => {
  it("lists ids and the marker instruction", () => {
    const text = formatContractsForPrompt(enumerateContracts(service));
    expect(text).toContain("@omni:contract <id>");
    expect(text).toContain("paymentService.Deposit.pre.1");
  });

  it("is empty when there are no contracts", () => {
    expect(formatContractsForPrompt([])).toBe("");
  });
});
