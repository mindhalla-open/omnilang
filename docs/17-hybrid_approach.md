# The Hybrid Approach to AI-Native Development

OmniLang v2.5 introduces the **Hybrid Approach** — a design philosophy that balances developer productivity, readability for non-programmers, and strict correctness. 

Traditional specification languages are either too formal (forcing developers to write complex mathematical proofs) or too informal (leading to AI hallucinations and incorrect code). OmniLang bridges this gap.

---

## 1. Core Principles

### A. Separation of Concerns
We keep our specifications clean by separating business design from deployment configuration:
* **The Specification (`.omni`)**: Deals only with *what* the system does (types, services, RPC contracts, and business rules).
* **The Configuration (`omni.toml`)**: Deals with *how* the compiler behaves and *where* it deploys (target languages, databases, LLM models, budgets, and dependencies).

### B. Progressive Formalization
Developers and business stakeholders should be able to start prototyping immediately using natural language, but transition to rigorous math where safety-critical behavior is required:
* **Natural Language Constraints**: Preconditions, postconditions, and invariants can be written as simple string literals. The LLM translates these into runtime checks and test cases.
* **Formal Expressions**: For critical nodes (e.g., financial calculations, safety invariants), developers can write mathematical expressions that are statically analyzed and verified via theorem provers (like Z3).

### C. Preventing AI Hallucination
AI code generators are prone to hallucinating when given completely free-form prompts. OmniLang prevents this by creating **strict architectural rails**:
1. **Explicit Signatures**: Every RPC method must have explicit input and output types.
2. **Structured Types**: Core data models (entities, structs, enums) must be explicitly typed.
3. **Formal Context Enrichment**: The generator is fed the exact parsed AST/JSON. Because the types and interfaces are rigid, the LLM cannot hallucinate the API contracts or structure; it can only write code to satisfy the natural language business rules within those strict boundaries.

---

## 2. Example: Natural Language vs. Formal Invariants

Here is how you can write both styles in the same specification file:

```omnilang
service AccountService
  operation Deposit
    inputs:
      accountId String
      amount Money
    outputs:
      result Money

    preconditions:
      // 1. Natural Language (interpreted by LLM, turned into code checks & unit tests)
      - "Deposit amount must be strictly greater than zero"
      
      // 2. Formal Expression (statically checked by compiler / Z3 solver)
      - amount > 0
```

By leveraging this hybrid strategy, OmniLang specs remain incredibly clean, readable by non-developers, yet solid enough to guarantee production-grade code correctness.
