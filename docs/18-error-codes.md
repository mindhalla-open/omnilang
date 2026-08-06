# Error & Diagnostic Codes

Every analyzer diagnostic carries a stable code so it can be referenced, looked up,
and filtered programmatically. Codes are grouped by analysis phase. This table is
generated from the analyzer source (`crates/omni-analyzer/src/*.rs`).

| Prefix | Phase |
|--------|-------|
| `E00xx` | Pipeline / project (lib.rs) |
| `E01xx` | Name resolution (symbols.rs) |
| `E02xx` | Type checking (type_check.rs) |
| `E03xx` | Constraint validation (constraints.rs) |
| `E04xx` | Module system (module_system.rs) |
| `E05xx` | Policy enforcement (policy.rs) |
| `E06xx` | Workflow verification (workflow.rs) |
| `E07xx` | Schema evolution (schema.rs) |
| `E08xx` | Formal verification (formal.rs) |
| `E09xx` | Intent gap detection (gap_detector.rs) |
| `E10xx` | Dependency graph (deps.rs) |

## Codes

| Code | Level | Message |
|------|-------|---------|
| `E0001` | error | symbol '…' not found in module '…' |
| `E0002` | error | module '…' not found |
| `E0101` | error | duplicate type definition: '…' |
| `E0102` | error | duplicate service definition: '…' |
| `E0103` | error | duplicate component definition: '…' |
| `E0104` | error | duplicate pipeline definition: '…' |
| `E0105` | error | duplicate workflow definition: '…' |
| `E0106` | error | duplicate agent definition: '…' |
| `E0107` | error | duplicate schema definition: '…' |
| `E0108` | error | duplicate policy definition: '…' |
| `E0109` | error | duplicate constraint definition: '…' |
| `E0110` | error | duplicate mixin definition: '…' |
| `E0111` | error | duplicate entity definition: '…' |
| `E0112` | error | duplicate action definition: '…' |
| `E0113` | error | duplicate rule definition: '…' |
| `E0201` | error | generic parameter '…' cannot have type arguments |
| `E0202` | error | type '…' expects … type arguments, found … |
| `E0203` | error | undefined type bound: '…' |
| `E0204` | error | undefined type: '…' |
| `E0205` | error | format constraint expects regex(\ |
| `E0206` | error | range constraint expects a list of two numbers, e.g. [min, max] |
| `E0207` | error | … constraint expects an integer literal |
| `E0208` | error | precision constraint expects an integer literal |
| `E0209` | error | unknown type constraint: '…' |
| `E0210` | error | cannot infer base type; please specify it explicitly (e.g. String { ... }) |
| `E0211` | error | example value is not compatible with base type … |
| `E0212` | error | undefined constraint: '…' |
| `E0213` | error | constraint '…' does not expect any arguments |
| `E0214` | error | constraint 'cacheable' expects exactly 1 argument (ttl: Duration) |
| `E0215` | error | unknown argument '…' for constraint 'cacheable', expected 'ttl' |
| `E0216` | error | argument 'ttl' for 'cacheable' must be a Duration (e.g. 5min, 200ms) |
| `E0217` | error | constraint 'rate_limited' expects exactly 2 arguments (max: Int, window: Duration) |
| `E0218` | error | argument 'max' for 'rate_limited' must be an integer |
| `E0219` | error | argument 'window' for 'rate_limited' must be a Duration |
| `E0220` | error | unknown argument '…' for constraint 'rate_limited', expected 'max' or 'window' |
| `E0221` | error | constraint 'authorized' expects exactly 1 argument (roles: [Role]) |
| `E0222` | error | unknown argument '…' for 'authorized', expected 'roles' |
| `E0223` | error | argument 'roles' for 'authorized' must be a list of roles, e.g. [Admin, User] |
| `E0224` | error | constraint 'latency' expects at least 1 percentile argument (e.g. p95: 50ms) |
| `E0225` | error | percentile '…' value must be a Duration |
| `E0226` | error | unknown percentile '…', expected p50, p95, or p99 |
| `E0227` | error | arguments to 'latency' must be named, e.g. p95: 50ms |
| `E0228` | error | constraint 'eventual_consistency' expects exactly 1 argument (max_lag: Duration) |
| `E0229` | error | unknown argument '…' for 'eventual_consistency', expected 'max_lag' |
| `E0230` | error | argument 'max_lag' for 'eventual_consistency' must be a Duration |
| `E0231` | error | conflicting latency requirements for '…' in service '…': … vs … |
| `E0232` | error | conflicting constraints on service '…': 'authenticated' and 'anonymous' |
| `E0233` | error | conflicting constraints on service '…': 'authenticated' and 'anonymous' |
| `E0234` | error | constraint '…' propagates from service '…' to depended-on service '…', but '…' is missi… |
| `E0235` | error | Service '…' has confidence level {:?}, which violates trust policy '…' requiring {:?} |
| `E0236` | error | undefined symbol: '…' |
| `E0237` | error | old() is only allowed in postconditions |
| `E0238` | error | old() expects exactly 1 argument |
| `E0239` | info | field '…' has optional type; consider adding a null-check \
                           … |
| `E0240` | info | field '…' has optional type; consider adding a null-check \
                           … |
| `E0241` | info | output '…' has optional type; accessing field on it \
                             may … |
| `E0242` | error | service '…' declares unsupported target '…' (expected one of: typescript, rust, python, go) |
| `E0301` | warning | service '…' has no constraints — consider adding latency, reliability, or security cons… |
| `E0302` | warning | service '…' has no goal — the goal field helps AI agents understand the intent |
| `E0303` | warning | Invariant '…' in service '…' is a natural language constraint and cannot be statically … |
| `E0304` | error | duplicate constraint '…' in service '…' |
| `E0305` | warning | Precondition '…' in operation '….…' is a natural language constraint and cannot be stat… |
| `E0306` | warning | Postcondition '…' in operation '….…' is a natural language constraint and cannot be sta… |
| `E0307` | info | operation '….…' has no test scenarios — consider adding tests for verification |
| `E0401` | error | cannot export private declaration '…': remove 'private' modifier or 'export' statement |
| `E0402` | error | registry import must specify a registry name |
| `E0403` | warning | registry import from '…' has no version constraint — consider pinning a version |
| `E0404` | error | service '…' applies undefined mixin '…' |
| `E0501` | warning | Policy compliance: Goal enforcement overridden with justification for service '…' |
| `E0502` | error | Policy violation: Service '…' must define a 'goal' statement as per org-wide policy. |
| `E0503` | warning | Policy compliance: Budget limit override approved for service '…' |
| `E0504` | error | Policy violation: Service '…' budget of $… exceeds organization maximum limit of $… |
| `E0601` | error | Workflow '…': wildcard transition excepts undefined state '…' |
| `E0602` | error | Workflow '…': transition uses undefined source state '…' |
| `E0603` | error | Workflow '…': transition uses undefined target state '…' |
| `E0604` | error | Workflow '…': timeout transition uses undefined target state '…' |
| `E0605` | warning | Workflow '…': state '…' is unreachable (dead state) |
| `E0701` | error | Schema Breaking Change: Entity '…' was removed from schema '…'. This is a breaking change. |
| `E0702` | error | Schema Breaking Change: Field '….…' was removed from schema '…'. This is a breaking cha… |
| `E0703` | error | Schema Breaking Change: Field '….…' changed type from '…' to '…' in schema '…'. This is… |
| `E0704` | info | Schema Validation: Entity '…' does not enable Row Level Security (RLS). Consider adding… |
| `E0705` | info | Schema Validation: Entity '…' does not enable soft deletes. Deletes will be permanent. |
| `E0801` | info | Formal Verification: Service '…' formally verified (… obligation(s) proven). Proof certificates generated under … |
| `E0802` | warning | Formal Verification: Service '…' failed formal verification checks.\n… |
| `E0803` | error | Formal Verification: preconditions of operation '….…' are contradictory (unsatisfiable)… |
| `E0804` | warning | Critical invariant '…' in service '…' is natural-language and cannot be formally proven… |
| `E0805` | warning | Formal Verification: 'z3' was not found on PATH — verification of service '…' was skipped. No proof was performed… |
| `E0806` | warning | Formal Verification: Service '…' declares formal verification but no machine-checkable proof obligation was found — nothing was proven… |
| `E0901` | info | Intent Entropy Analysis: Goal for service '…' has low clarity (entropy: {:.2}). Conside… |
| `E0902` | warning | Logical Gap Detector: Operation '…' in service '…' has no preconditions defined. Input … |
| `E0903` | warning | Logical Gap Detector: Operation '…' in service '…' has no postconditions defined. The o… |
| `E0904` | warning | Logical Gap Detector: Component '…' has no layout/accessibility constraints configured. |
| `E1001` | error | service '…' depends on undefined service '…' |
| `E1002` | error | circular dependency detected: … |
