use crate::{Diagnostic, DiagnosticKind};
use omni_parser::ast::{
    BinaryOperator, Declaration, Expression, Literal, OperationDecl, SourceFile, UnaryOperator,
};
use std::collections::{HashMap, HashSet};
use std::process::Command;

/// Hard timeout (seconds) for a single Z3 invocation: a hard proof obligation
/// must never hang `omni check`/`omni build`.
const Z3_TIMEOUT_SECS: u32 = 10;

/// Outcome of running a proof obligation through Z3.
pub enum Z3Verdict {
    /// `unsat`: the negated goal has no model — the obligation is proven.
    Proven,
    /// `sat`: Z3 found a concrete counterexample (rendered as `var = value` pairs).
    Counterexample(String),
    /// `unknown` or solver error — no conclusion either way.
    Unknown(String),
    /// `z3` is not on PATH; no proof was performed at all.
    SolverUnavailable,
}

pub fn verify_proof_obligations(file: &SourceFile, diagnostics: &mut Vec<Diagnostic>) {
    let proofs_dir = std::path::Path::new(".omni-cache/proofs");
    let _ = std::fs::create_dir_all(proofs_dir);

    // Pass 1 (always on): detect operations whose *formal* preconditions are
    // mutually contradictory (jointly unsatisfiable) — such an operation can
    // never run, which is almost always a spec bug. Only the machine-checkable
    // subset is considered; natural-language (string) preconditions are skipped.
    check_contradictory_preconditions(file, proofs_dir, diagnostics);

    // Criticality gate: safety-/critical-named invariants ought to be formally
    // verifiable. Flag those expressed only as natural language.
    check_critical_contracts(file, diagnostics);

    for decl in &file.declarations {
        if let Declaration::Service(s) = decl {
            // Look for formal_verification or proven constraints
            let has_formal = s.constraints.iter().any(|c| {
                let name = c.name.to_lowercase();
                name.contains("formal_verification")
                    || name.contains("proven")
                    || name.contains("formal")
            });

            if !has_formal {
                continue;
            }

            let mut service_verified = true;
            let mut solver_missing = false;
            let mut proven_ops = 0usize;
            let mut log_messages = Vec::new();

            for op in &s.operations {
                if op.preconditions.is_empty() && op.postconditions.is_empty() {
                    continue;
                }

                // Only the machine-checkable subset goes to the solver; a
                // natural-language condition must never be asserted as SMT.
                let fpre: Vec<&Expression> = op
                    .preconditions
                    .iter()
                    .filter(|p| is_translatable(p))
                    .collect();
                let fpost: Vec<&Expression> = op
                    .postconditions
                    .iter()
                    .filter(|p| is_translatable(p))
                    .collect();
                let skipped =
                    (op.preconditions.len() - fpre.len()) + (op.postconditions.len() - fpost.len());

                if fpost.is_empty() {
                    log_messages.push(format!(
                        "  - Operation '{}': no machine-checkable postconditions to prove ({} natural-language condition(s) deferred to generated tests)",
                        op.name, skipped
                    ));
                    continue;
                }

                let sorts = declared_sorts(op);
                match verify_operation(&s.name, &op.name, &fpre, &fpost, &sorts) {
                    Ok(smt_script) => {
                        let filename = format!("{}_{}.smt2", s.name, op.name);
                        let filepath = proofs_dir.join(&filename);
                        let _ = std::fs::write(&filepath, &smt_script);

                        let skipped_note = if skipped > 0 {
                            format!(
                                " ({} natural-language condition(s) deferred to generated tests)",
                                skipped
                            )
                        } else {
                            String::new()
                        };
                        match check_with_z3(&smt_script) {
                            Z3Verdict::Proven => {
                                proven_ops += 1;
                                log_messages.push(format!(
                                    "  - Operation '{}': proof verified by Z3 (unsat){}",
                                    op.name, skipped_note
                                ));
                            }
                            Z3Verdict::Counterexample(model) => {
                                service_verified = false;
                                log_messages.push(format!(
                                    "  - Operation '{}': Z3 found a counterexample: {}{}",
                                    op.name, model, skipped_note
                                ));
                            }
                            Z3Verdict::Unknown(out) => {
                                service_verified = false;
                                log_messages.push(format!(
                                    "  - Operation '{}': Z3 returned unknown/error: {}",
                                    op.name, out
                                ));
                            }
                            Z3Verdict::SolverUnavailable => {
                                solver_missing = true;
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        service_verified = false;
                        log_messages.push(format!(
                            "  - Operation '{}' SMT translation failed: {}",
                            op.name, e
                        ));
                    }
                }
            }

            if solver_missing {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::Warning,
                    code: "E0805",
                    message: format!(
                        "Formal Verification: 'z3' was not found on PATH — verification of service '{}' was skipped. No proof was performed; install Z3 to verify the declared obligations.",
                        s.name
                    ),
                    span: s.span,
                });
            } else if !service_verified {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::Warning,
                    code: "E0802",
                    message: format!(
                        "Formal Verification: Service '{}' failed formal verification checks.\n{}",
                        s.name,
                        log_messages.join("\n")
                    ),
                    span: s.span,
                });
            } else if proven_ops == 0 {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::Warning,
                    code: "E0806",
                    message: format!(
                        "Formal Verification: Service '{}' declares formal verification but no machine-checkable proof obligation was found — nothing was proven. Express contracts as mathematical formulas to gain a Z3 guarantee.\n{}",
                        s.name,
                        log_messages.join("\n")
                    ),
                    span: s.span,
                });
            } else {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::Info,
                    code: "E0801",
                    message: format!(
                        "Formal Verification: Service '{}' formally verified ({} obligation(s) proven). Proof certificates generated under '.omni-cache/proofs/'.\n{}",
                        s.name,
                        proven_ops,
                        log_messages.join("\n")
                    ),
                    span: s.span,
                });
            }
        }
    }
}

/// The machine-checkable subset: numeric/boolean expressions over identifiers
/// with comparison and logical operators. String/list/range/membership and
/// unknown calls fall outside the subset and are skipped (never mistranslated).
pub fn is_translatable(expr: &Expression) -> bool {
    match expr {
        Expression::Literal(lit) => matches!(
            lit,
            Literal::Int(_)
                | Literal::Float(_)
                | Literal::Bool(_)
                | Literal::Money(_)
                | Literal::Duration(_)
        ),
        Expression::Identifier(_, _) => true,
        Expression::BinaryOp {
            left, op, right, ..
        } => {
            matches!(
                op,
                BinaryOperator::Eq
                    | BinaryOperator::NotEq
                    | BinaryOperator::Lt
                    | BinaryOperator::Gt
                    | BinaryOperator::LtEq
                    | BinaryOperator::GtEq
                    | BinaryOperator::And
                    | BinaryOperator::Or
            ) && is_translatable(left)
                && is_translatable(right)
        }
        Expression::UnaryOp { operand, .. } => is_translatable(operand),
        Expression::Call { function, args, .. } => {
            function == "old" && args.len() == 1 && is_translatable(&args[0])
        }
        Expression::FieldAccess { object, .. } => is_translatable(object),
        Expression::List(_, _) => false,
    }
}

/// Builds an SMT script asserting all translatable preconditions to test whether
/// they are jointly satisfiable. Returns `None` if no precondition is in the
/// machine-checkable subset.
pub fn build_satisfiability_smt(
    service_name: &str,
    operation_name: &str,
    preconditions: &[Expression],
    sorts: &HashMap<String, &'static str>,
) -> Option<String> {
    let formal: Vec<&Expression> = preconditions
        .iter()
        .filter(|p| is_translatable(p))
        .collect();
    if formal.is_empty() {
        return None;
    }
    let mut vars = HashSet::new();
    for p in &formal {
        collect_variables(p, &mut vars);
    }
    let mut smt = String::new();
    smt.push_str(&format!(
        "; Precondition satisfiability check for {}.{}\n\n",
        service_name, operation_name
    ));
    for var in &vars {
        smt.push_str(&format!(
            "(declare-fun {} () {})\n",
            var,
            sort_for_var(var, sorts)
        ));
    }
    smt.push('\n');
    for p in &formal {
        // `is_translatable` guarantees translation succeeds; bail out (no
        // check) rather than emit a broken script if that invariant breaks.
        smt.push_str(&format!("(assert {})\n", expr_to_smt(p).ok()?));
    }
    smt.push_str("\n(check-sat)\n");
    Some(smt)
}

fn check_contradictory_preconditions(
    file: &SourceFile,
    proofs_dir: &std::path::Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for decl in &file.declarations {
        if let Declaration::Service(s) = decl {
            for op in &s.operations {
                let sorts = declared_sorts(op);
                if let Some(smt) =
                    build_satisfiability_smt(&s.name, &op.name, &op.preconditions, &sorts)
                    && !is_satisfiable(&smt)
                {
                    let filename = format!("{}_{}_sat.smt2", s.name, op.name);
                    let _ = std::fs::write(proofs_dir.join(&filename), &smt);
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Error,
                        code: "E0803",
                        message: format!(
                            "Formal Verification: preconditions of operation '{}.{}' are contradictory (unsatisfiable) — the operation can never execute. See '.omni-cache/proofs/{}'.",
                            s.name, op.name, filename
                        ),
                        span: op.span,
                    });
                }
            }
        }
    }
}

/// Returns true unless Z3 definitively reports `unsat` (conservative: `unknown`
/// or a missing solver never produces a false-positive contradiction).
pub fn is_satisfiable(smt_script: &str) -> bool {
    match run_z3(smt_script) {
        Some(out) => out.lines().map(str::trim).find(|l| !l.is_empty()) != Some("unsat"),
        None => true,
    }
}

/// Criticality gate: an invariant whose name signals safety/criticality should
/// be machine-verifiable. If it is expressed only as natural language (a named
/// invariant `name: "text"`), warn that it cannot be formally proven.
fn check_critical_contracts(file: &SourceFile, diagnostics: &mut Vec<Diagnostic>) {
    let is_critical = |name: &str| {
        let n = name.to_lowercase();
        n.contains("safety")
            || n.contains("critical")
            || n.ends_with("_safe")
            || n.contains("money")
    };
    for decl in &file.declarations {
        if let Declaration::Service(s) = decl {
            for inv in &s.invariants {
                // Named NL invariant: BinaryOp(Identifier `name` Eq String "...").
                if let Expression::BinaryOp {
                    left, op, right, ..
                } = inv
                    && matches!(op, BinaryOperator::Eq)
                    && let Expression::Identifier(name, _) = left.as_ref()
                    && matches!(right.as_ref(), Expression::Literal(Literal::String(_)))
                    && is_critical(name)
                {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Warning,
                        code: "E0804",
                        message: format!(
                            "Critical invariant '{}' in service '{}' is natural-language and cannot be formally proven. Express it as a mathematical formula to gain a Z3 guarantee.",
                            name, s.name
                        ),
                        span: s.span,
                    });
                }
            }
        }
    }
}

/// Real proof status of a service, derived from Z3 — not from constraint names.
/// Returns `(has_formal_obligations, proven_obligations)`: how many operations
/// have machine-checkable postconditions that Z3 proves are implied by their
/// (translatable) preconditions.
pub fn service_proof_status(service: &omni_parser::ast::ServiceDecl) -> (bool, usize) {
    let mut has_formal = false;
    let mut proven = 0usize;
    for op in &service.operations {
        let fpre: Vec<&Expression> = op
            .preconditions
            .iter()
            .filter(|p| is_translatable(p))
            .collect();
        let fpost: Vec<&Expression> = op
            .postconditions
            .iter()
            .filter(|p| is_translatable(p))
            .collect();
        if !fpre.is_empty() || !fpost.is_empty() {
            has_formal = true;
        }
        if fpost.is_empty() {
            continue;
        }
        let sorts = declared_sorts(op);
        // Prove: preconditions ∧ ¬(postconditions) is UNSAT  ⇒  pre ⇒ post.
        if let Ok(smt) = verify_operation(&service.name, &op.name, &fpre, &fpost, &sorts)
            && matches!(check_with_z3(&smt), Z3Verdict::Proven)
        {
            proven += 1;
        }
    }
    (has_formal, proven)
}

/// Builds the proof-obligation script `pre ∧ ¬post` for one operation. The
/// caller must pass only translatable expressions (filter with
/// [`is_translatable`]); anything outside the subset is a hard `Err`, never a
/// silent mistranslation.
pub fn verify_operation(
    service_name: &str,
    operation_name: &str,
    preconditions: &[&Expression],
    postconditions: &[&Expression],
    sorts: &HashMap<String, &'static str>,
) -> Result<String, String> {
    if postconditions.is_empty() {
        return Err("no machine-checkable postconditions to prove".to_string());
    }

    let mut vars = HashSet::new();
    for p in preconditions.iter().chain(postconditions.iter()) {
        collect_variables(p, &mut vars);
    }

    let mut smt = String::new();
    smt.push_str("; SMT-LIB v2 Verification Script for: ");
    smt.push_str(&format!("{}.{}\n\n", service_name, operation_name));

    // Declare all variables with sorts taken from the operation signature
    // where available (name heuristics are only a fallback).
    for var in &vars {
        smt.push_str(&format!(
            "(declare-fun {} () {})\n",
            var,
            sort_for_var(var, sorts)
        ));
    }
    smt.push_str("\n; Preconditions\n");
    for p in preconditions {
        smt.push_str(&format!("(assert {})\n", expr_to_smt(p)?));
    }
    smt.push_str("\n; Postconditions (proving they are implied by preconditions)\n");
    let post_conj = if postconditions.len() == 1 {
        expr_to_smt(postconditions[0])?
    } else {
        let posts: Result<Vec<String>, String> =
            postconditions.iter().map(|p| expr_to_smt(p)).collect();
        format!("(and {})", posts?.join(" "))
    };
    smt.push_str(&format!("(assert (not {}))\n", post_conj));
    smt.push_str("\n(check-sat)\n(get-model)\n");

    Ok(smt)
}

fn collect_variables(expr: &Expression, vars: &mut HashSet<String>) {
    match expr {
        Expression::Literal(_) => {}
        Expression::Identifier(name, _) => {
            vars.insert(name.clone());
        }
        Expression::BinaryOp { left, right, .. } => {
            collect_variables(left, vars);
            collect_variables(right, vars);
        }
        Expression::UnaryOp { operand, .. } => {
            collect_variables(operand, vars);
        }
        Expression::Call { function, args, .. } => {
            if function == "old" && !args.is_empty() {
                if let Expression::Identifier(name, _) = &args[0] {
                    vars.insert(format!("{}_before", name));
                } else {
                    for arg in args {
                        collect_variables(arg, vars);
                    }
                }
            } else {
                for arg in args {
                    collect_variables(arg, vars);
                }
            }
        }
        Expression::FieldAccess { object, field, .. } => {
            let mut obj_vars = HashSet::new();
            collect_variables(object, &mut obj_vars);
            for v in obj_vars {
                vars.insert(format!("{}_{}", v, field));
            }
        }
        Expression::List(items, _) => {
            for item in items {
                collect_variables(item, vars);
            }
        }
    }
}

/// Translates an expression from the machine-checkable subset to SMT-LIB.
/// Anything outside the subset is an `Err` — a wrong translation (e.g. mapping
/// `in` to `=`) could otherwise produce a false proof.
fn expr_to_smt(expr: &Expression) -> Result<String, String> {
    Ok(match expr {
        Expression::Literal(lit) => match lit {
            Literal::Int(i) => i.to_string(),
            Literal::Float(f) => f.to_string(),
            Literal::Bool(b) => b.to_string(),
            Literal::Duration(d) => {
                let numeric: String = d.chars().filter(|c| c.is_ascii_digit()).collect();
                numeric.parse::<i64>().unwrap_or(0).to_string()
            }
            Literal::Money(m) => {
                let numeric: String = m
                    .chars()
                    .filter(|c| c.is_ascii_digit() || *c == '.')
                    .collect();
                numeric.parse::<f64>().unwrap_or(0.0).to_string()
            }
            Literal::String(_) | Literal::Null => {
                return Err("string/null literals are outside the machine-checkable subset".into());
            }
        },
        Expression::Identifier(name, _) => name.clone(),
        Expression::BinaryOp {
            left, op, right, ..
        } => {
            let op_str = match op {
                BinaryOperator::Eq => "=",
                BinaryOperator::NotEq => "distinct",
                BinaryOperator::Lt => "<",
                BinaryOperator::Gt => ">",
                BinaryOperator::LtEq => "<=",
                BinaryOperator::GtEq => ">=",
                BinaryOperator::And => "and",
                BinaryOperator::Or => "or",
                other => {
                    return Err(format!(
                        "operator {:?} is outside the machine-checkable subset",
                        other
                    ));
                }
            };
            if op_str == "distinct" {
                format!("(not (= {} {}))", expr_to_smt(left)?, expr_to_smt(right)?)
            } else {
                format!(
                    "({} {} {})",
                    op_str,
                    expr_to_smt(left)?,
                    expr_to_smt(right)?
                )
            }
        }
        Expression::UnaryOp { op, operand, .. } => {
            let op_str = match op {
                UnaryOperator::Not => "not",
                UnaryOperator::Neg => "-",
            };
            format!("({} {})", op_str, expr_to_smt(operand)?)
        }
        Expression::Call { function, args, .. } => {
            if function == "old" && args.len() == 1 {
                format!("{}_before", expr_to_smt(&args[0])?)
            } else {
                return Err(format!(
                    "call to '{}' is outside the machine-checkable subset",
                    function
                ));
            }
        }
        Expression::FieldAccess { object, field, .. } => {
            format!("{}_{}", expr_to_smt(object)?, field)
        }
        Expression::List(_, _) => {
            return Err("list literals are outside the machine-checkable subset".into());
        }
    })
}

/// SMT sort for a declared OmniLang type name, if it maps onto one.
fn smt_sort_for_type(type_name: &str) -> Option<&'static str> {
    match type_name.to_lowercase().as_str() {
        "int" | "integer" | "i32" | "i64" | "u32" | "u64" | "duration" => Some("Int"),
        "float" | "double" | "real" | "decimal" | "money" | "number" => Some("Real"),
        "bool" | "boolean" => Some("Bool"),
        "string" | "text" | "uuid" | "email" | "url" | "date" | "datetime" | "timestamp" => {
            Some("String")
        }
        _ => None,
    }
}

/// Sorts for an operation's contract variables, taken from its declared
/// input/output types. `old(x)` references resolve to `x_before` with the same
/// sort as `x`.
pub fn declared_sorts(op: &OperationDecl) -> HashMap<String, &'static str> {
    let mut sorts = HashMap::new();
    for field in op.inputs.iter().chain(op.outputs.iter()) {
        if let Some(sort) = smt_sort_for_type(&field.ty.name) {
            sorts.insert(format!("{}_before", field.name), sort);
            sorts.insert(field.name.clone(), sort);
        }
    }
    sorts
}

/// Declared sort first; the name heuristic is only a fallback for variables the
/// operation signature does not cover (e.g. flattened field accesses).
fn sort_for_var(var: &str, declared: &HashMap<String, &'static str>) -> &'static str {
    if let Some(sort) = declared.get(var) {
        return sort;
    }
    if let Some(base) = var.strip_suffix("_before")
        && let Some(sort) = declared.get(base)
    {
        return sort;
    }
    infer_type(var)
}

fn infer_type(var_name: &str) -> &'static str {
    let lower = var_name.to_lowercase();
    // Boolean indicators come first: names like "valid" or "active" contain
    // "id" as a substring and must not fall into the String branch below.
    if lower.contains("enabled")
        || lower.contains("success")
        || lower.contains("active")
        || lower.contains("valid")
        || lower.contains("deleted")
        || lower.starts_with("is_")
        || lower.starts_with("has_")
    {
        "Bool"
    } else if lower.contains("id")
        || lower.contains("name")
        || lower.contains("status")
        || lower.contains("token")
        || lower.contains("email")
    {
        "String"
    } else if lower.contains("balance")
        || lower.contains("cost")
        || lower.contains("price")
        || lower.contains("amount")
        || lower.contains("total")
    {
        "Real"
    } else {
        // stock/quantity/count/limit/age and anything unknown.
        "Int"
    }
}

/// Runs Z3 over stdin with a hard timeout. `None` means the solver could not
/// be started at all (not installed / not on PATH).
fn run_z3(smt_script: &str) -> Option<String> {
    let mut child = Command::new("z3")
        .arg("-in")
        .arg(format!("-T:{}", Z3_TIMEOUT_SECS))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    use std::io::Write;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(smt_script.as_bytes());
    }
    let out = child.wait_with_output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn check_with_z3(smt_script: &str) -> Z3Verdict {
    let Some(out) = run_z3(smt_script) else {
        return Z3Verdict::SolverUnavailable;
    };
    let mut lines = out.lines().map(str::trim).filter(|l| !l.is_empty());
    match lines.next() {
        Some("unsat") => Z3Verdict::Proven,
        Some("sat") => {
            let model: String = lines.collect::<Vec<_>>().join(" ");
            Z3Verdict::Counterexample(summarize_model(&model))
        }
        _ => Z3Verdict::Unknown(out.trim().to_string()),
    }
}

/// Renders a Z3 model as compact `var = value` pairs; falls back to the raw
/// (whitespace-collapsed) model text if the shape is unexpected.
fn summarize_model(model: &str) -> String {
    let flat = model.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut pairs = Vec::new();
    let mut rest = flat.as_str();
    while let Some(i) = rest.find("(define-fun ") {
        rest = &rest[i + "(define-fun ".len()..];
        let Some(name_end) = rest.find(' ') else {
            break;
        };
        let name = &rest[..name_end];
        rest = &rest[name_end..];
        // Expect a constant: " () <Sort> <value...>" where the value ends at
        // the define-fun's closing paren.
        let Some(after_args) = rest.strip_prefix(" () ") else {
            continue;
        };
        let Some(sort_end) = after_args.find(' ') else {
            break;
        };
        let value_region = &after_args[sort_end + 1..];
        let mut depth = 0i32;
        let mut end = value_region.len();
        for (j, c) in value_region.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth < 0 {
                        end = j;
                        break;
                    }
                }
                _ => {}
            }
        }
        pairs.push(format!("{} = {}", name, value_region[..end].trim()));
        rest = &value_region[end..];
    }
    if pairs.is_empty() {
        flat
    } else {
        pairs.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omni_parser::Span;
    use omni_parser::ast::Literal;

    #[test]
    fn test_translation_to_smt() {
        let expr = Expression::BinaryOp {
            left: Box::new(Expression::Identifier(
                "quantity".to_string(),
                Span { start: 0, end: 0 },
            )),
            op: BinaryOperator::GtEq,
            right: Box::new(Expression::Literal(Literal::Int(0))),
            span: Span { start: 0, end: 0 },
        };
        let smt = expr_to_smt(&expr).expect("translatable");
        assert_eq!(smt, "(>= quantity 0)");
    }

    fn sp() -> Span {
        Span { start: 0, end: 0 }
    }

    fn cmp(var: &str, op: BinaryOperator, n: i64) -> Expression {
        Expression::BinaryOp {
            left: Box::new(Expression::Identifier(var.to_string(), sp())),
            op,
            right: Box::new(Expression::Literal(Literal::Int(n))),
            span: sp(),
        }
    }

    #[test]
    fn formal_subset_accepts_numeric_comparisons() {
        assert!(is_translatable(&cmp("count", BinaryOperator::GtEq, 0)));
    }

    #[test]
    fn formal_subset_rejects_string_and_range() {
        // A natural-language (string) precondition is not machine-checkable.
        let nl = Expression::Literal(Literal::String("must be positive".to_string()));
        assert!(!is_translatable(&nl));
        // Range/membership operators are outside the subset.
        let range = Expression::BinaryOp {
            left: Box::new(Expression::Identifier("x".to_string(), sp())),
            op: BinaryOperator::In,
            right: Box::new(Expression::Identifier("xs".to_string(), sp())),
            span: sp(),
        };
        assert!(!is_translatable(&range));
    }

    #[test]
    fn out_of_subset_operator_is_an_error_not_a_mistranslation() {
        // `x in xs` must never silently translate to `(= x xs)`.
        let range = Expression::BinaryOp {
            left: Box::new(Expression::Identifier("x".to_string(), sp())),
            op: BinaryOperator::In,
            right: Box::new(Expression::Identifier("xs".to_string(), sp())),
            span: sp(),
        };
        assert!(expr_to_smt(&range).is_err());
        // Unknown function calls are equally out of subset.
        let call = Expression::Call {
            function: "len".to_string(),
            type_args: None,
            args: vec![Expression::Identifier("xs".to_string(), sp())],
            span: sp(),
        };
        assert!(expr_to_smt(&call).is_err());
    }

    #[test]
    fn detects_contradictory_preconditions() {
        // count > 5 AND count < 3  -> unsatisfiable.
        let pre = vec![
            cmp("count", BinaryOperator::Gt, 5),
            cmp("count", BinaryOperator::Lt, 3),
        ];
        let smt = build_satisfiability_smt("S", "Op", &pre, &HashMap::new()).expect("translatable");
        // Z3 is available in this environment; contradictory => not satisfiable.
        assert!(!is_satisfiable(&smt));
    }

    #[test]
    fn accepts_consistent_preconditions() {
        let pre = vec![
            cmp("count", BinaryOperator::Gt, 0),
            cmp("count", BinaryOperator::Lt, 100),
        ];
        let smt = build_satisfiability_smt("S", "Op", &pre, &HashMap::new()).expect("translatable");
        assert!(is_satisfiable(&smt));
    }

    #[test]
    fn no_smt_for_natural_language_preconditions() {
        let pre = vec![Expression::Literal(Literal::String(
            "amount > 0".to_string(),
        ))];
        assert!(build_satisfiability_smt("S", "Op", &pre, &HashMap::new()).is_none());
    }

    fn first_service(src: &str) -> omni_parser::ast::ServiceDecl {
        use omni_parser::Lexer;
        use omni_parser::parser::Parser;
        let (tokens, _) = Lexer::new(src).tokenize();
        let (file, _) = Parser::new(tokens).parse();
        file.declarations
            .into_iter()
            .find_map(|d| match d {
                Declaration::Service(s) => Some(s),
                _ => None,
            })
            .expect("a service")
    }

    #[test]
    fn proves_postcondition_implied_by_precondition() {
        // x > 0  ⇒  x >= 0  is provable by Z3.
        let svc = first_service(
            "module m\nservice S\n  operation Op\n    inputs:\n      x int\n    preconditions:\n      - x > 0\n    postconditions:\n      - x >= 0\n",
        );
        let (has_formal, proven) = service_proof_status(&svc);
        assert!(has_formal);
        assert_eq!(proven, 1, "x>0 should prove x>=0");
    }

    #[test]
    fn does_not_prove_unsound_postcondition() {
        // x > 0  does NOT imply  x > 100.
        let svc = first_service(
            "module m\nservice S\n  operation Op\n    inputs:\n      x int\n    preconditions:\n      - x > 0\n    postconditions:\n      - x > 100\n",
        );
        let (_has_formal, proven) = service_proof_status(&svc);
        assert_eq!(proven, 0, "x>0 must not prove x>100");
    }

    #[test]
    fn declared_types_override_name_heuristics() {
        // `valid` would be typed String by the old substring heuristic
        // ("valid".contains("id")); the declared `bool` must win — and the
        // heuristic itself now classifies it as Bool anyway.
        let svc = first_service(
            "module m\nservice S\n  operation Op\n    inputs:\n      valid bool\n      count int\n    preconditions:\n      - count > 0\n",
        );
        let sorts = declared_sorts(&svc.operations[0]);
        assert_eq!(sorts.get("valid"), Some(&"Bool"));
        assert_eq!(sorts.get("count"), Some(&"Int"));
        assert_eq!(sorts.get("count_before"), Some(&"Int"));
        assert_eq!(sort_for_var("valid", &sorts), "Bool");
        assert_eq!(infer_type("valid"), "Bool");
    }

    #[test]
    fn counterexample_includes_model_values() {
        // pre: x > 0, post: x > 100 — Z3 must produce a concrete model.
        let pre_expr = cmp("x", BinaryOperator::Gt, 0);
        let post_expr = cmp("x", BinaryOperator::Gt, 100);
        let pre: Vec<&Expression> = vec![&pre_expr];
        let post: Vec<&Expression> = vec![&post_expr];
        let smt = verify_operation("S", "Op", &pre, &post, &HashMap::new()).expect("translatable");
        match check_with_z3(&smt) {
            Z3Verdict::Counterexample(model) => {
                assert!(model.contains("x ="), "model should bind x, got: {model}");
            }
            _ => panic!("expected a counterexample for x>0 ⇏ x>100"),
        }
    }

    #[test]
    fn proof_obligation_script_is_proven_for_valid_implication() {
        let pre_expr = cmp("x", BinaryOperator::Gt, 0);
        let post_expr = cmp("x", BinaryOperator::GtEq, 0);
        let pre: Vec<&Expression> = vec![&pre_expr];
        let post: Vec<&Expression> = vec![&post_expr];
        let smt = verify_operation("S", "Op", &pre, &post, &HashMap::new()).expect("translatable");
        assert!(matches!(check_with_z3(&smt), Z3Verdict::Proven));
    }

    #[test]
    fn summarize_model_extracts_bindings() {
        let raw = "( (define-fun x () Int 101) (define-fun y () Real (- 2.5)) )";
        let summary = summarize_model(raw);
        assert!(summary.contains("x = 101"), "got: {summary}");
        assert!(summary.contains("y = (- 2.5)"), "got: {summary}");
    }

    #[test]
    fn no_postconditions_is_an_error_not_a_fake_counterexample() {
        // Previously `pre`-only obligations ran `check-sat` on the preconditions
        // alone: `sat` (i.e. "preconditions are consistent") was reported as a
        // counterexample. There must be no proof attempt without a goal.
        let pre_expr = cmp("x", BinaryOperator::Gt, 0);
        let pre: Vec<&Expression> = vec![&pre_expr];
        assert!(verify_operation("S", "Op", &pre, &[], &HashMap::new()).is_err());
    }
}
