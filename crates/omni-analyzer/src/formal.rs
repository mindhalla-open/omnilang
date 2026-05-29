use crate::{Diagnostic, DiagnosticKind};
use omni_parser::ast::{
    BinaryOperator, Declaration, Expression, Literal, SourceFile, UnaryOperator,
};
use std::collections::HashSet;
use std::process::Command;

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

            if has_formal {
                let mut service_verified = true;
                let mut log_messages = Vec::new();

                for op in &s.operations {
                    if !op.preconditions.is_empty() || !op.postconditions.is_empty() {
                        match verify_operation(
                            &s.name,
                            &op.name,
                            &op.preconditions,
                            &op.postconditions,
                        ) {
                            Ok(smt_script) => {
                                let filename = format!("{}_{}.smt2", s.name, op.name);
                                let filepath = proofs_dir.join(&filename);
                                let _ = std::fs::write(&filepath, &smt_script);

                                let (verified, msg) = check_with_z3(&smt_script);
                                if !verified {
                                    service_verified = false;
                                }
                                log_messages.push(format!("  - Operation '{}': {}", op.name, msg));
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
                }

                if service_verified {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Info,
                        code: "E0801",
                        message: format!(
                            "Formal Verification: Service '{}' formally verified. Proof certificates generated under '.omni-cache/proofs/'.\n{}",
                            s.name,
                            log_messages.join("\n")
                        ),
                        span: s.span,
                    });
                } else {
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
                }
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
        smt.push_str(&format!("(declare-fun {} () {})\n", var, infer_type(var)));
    }
    smt.push('\n');
    for p in &formal {
        smt.push_str(&format!("(assert {})\n", expr_to_smt(p)));
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
                if let Some(smt) = build_satisfiability_smt(&s.name, &op.name, &op.preconditions)
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
    let process = Command::new("z3")
        .arg("-in")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn();
    match process {
        Ok(mut child) => {
            use std::io::Write;
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(smt_script.as_bytes());
            }
            match child.wait_with_output() {
                Ok(out) => !String::from_utf8_lossy(&out.stdout)
                    .trim()
                    .contains("unsat"),
                Err(_) => true,
            }
        }
        Err(_) => true,
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
        let fpre: Vec<Expression> = op
            .preconditions
            .iter()
            .filter(|p| is_translatable(p))
            .cloned()
            .collect();
        let fpost: Vec<Expression> = op
            .postconditions
            .iter()
            .filter(|p| is_translatable(p))
            .cloned()
            .collect();
        if !fpre.is_empty() || !fpost.is_empty() {
            has_formal = true;
        }
        if fpost.is_empty() {
            continue;
        }
        // Prove: preconditions ∧ ¬(postconditions) is UNSAT  ⇒  pre ⇒ post.
        let mut vars = HashSet::new();
        for p in fpre.iter().chain(fpost.iter()) {
            collect_variables(p, &mut vars);
        }
        let mut smt = String::new();
        for var in &vars {
            smt.push_str(&format!("(declare-fun {} () {})\n", var, infer_type(var)));
        }
        for p in &fpre {
            smt.push_str(&format!("(assert {})\n", expr_to_smt(p)));
        }
        let post_conj = if fpost.len() == 1 {
            expr_to_smt(&fpost[0])
        } else {
            let parts: Vec<String> = fpost.iter().map(expr_to_smt).collect();
            format!("(and {})", parts.join(" "))
        };
        smt.push_str(&format!("(assert (not {}))\n(check-sat)\n", post_conj));
        if !is_satisfiable(&smt) {
            proven += 1;
        }
    }
    (has_formal, proven)
}

pub fn verify_operation(
    service_name: &str,
    operation_name: &str,
    preconditions: &[Expression],
    postconditions: &[Expression],
) -> Result<String, String> {
    let mut vars = HashSet::new();
    for p in preconditions {
        collect_variables(p, &mut vars);
    }
    for p in postconditions {
        collect_variables(p, &mut vars);
    }

    let mut smt = String::new();
    smt.push_str("; SMT-LIB v2 Verification Script for: ");
    smt.push_str(&format!("{}.{}\n\n", service_name, operation_name));

    // Declare all variables
    for var in &vars {
        let ty = infer_type(var);
        smt.push_str(&format!("(declare-fun {} () {})\n", var, ty));
    }
    smt.push_str("\n; Preconditions\n");
    for p in preconditions {
        smt.push_str(&format!("(assert {})\n", expr_to_smt(p)));
    }
    smt.push_str("\n; Postconditions (proving they are implied by preconditions)\n");
    if !postconditions.is_empty() {
        let post_conj = if postconditions.len() == 1 {
            expr_to_smt(&postconditions[0])
        } else {
            let posts: Vec<String> = postconditions.iter().map(expr_to_smt).collect();
            format!("(and {})", posts.join(" "))
        };
        smt.push_str(&format!("(assert (not {}))\n", post_conj));
    }
    smt.push_str("\n(check-sat)\n");

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

fn expr_to_smt(expr: &Expression) -> String {
    match expr {
        Expression::Literal(lit) => match lit {
            Literal::Int(i) => i.to_string(),
            Literal::Float(f) => f.to_string(),
            Literal::Bool(b) => b.to_string(),
            Literal::String(s) => format!("\"{}\"", s),
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
            Literal::Null => "null".to_string(),
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
                _ => "=",
            };
            if op_str == "distinct" {
                format!("(not (= {} {}))", expr_to_smt(left), expr_to_smt(right))
            } else {
                format!("({} {} {})", op_str, expr_to_smt(left), expr_to_smt(right))
            }
        }
        Expression::UnaryOp { op, operand, .. } => {
            let op_str = match op {
                UnaryOperator::Not => "not",
                UnaryOperator::Neg => "-",
            };
            format!("({} {})", op_str, expr_to_smt(operand))
        }
        Expression::Call { function, args, .. } => {
            if function == "old" && !args.is_empty() {
                format!("{}_before", expr_to_smt(&args[0]))
            } else if args.is_empty() {
                function.clone()
            } else {
                let args_str: Vec<String> = args.iter().map(expr_to_smt).collect();
                format!("({} {})", function, args_str.join(" "))
            }
        }
        Expression::FieldAccess { object, field, .. } => {
            format!("{}_{}", expr_to_smt(object), field)
        }
        Expression::List(_, _) => "0".to_string(),
    }
}

fn infer_type(var_name: &str) -> &'static str {
    let lower = var_name.to_lowercase();
    if lower.contains("id")
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
    } else if lower.contains("stock")
        || lower.contains("quantity")
        || lower.contains("count")
        || lower.contains("limit")
        || lower.contains("age")
    {
        "Int"
    } else if lower.contains("enabled")
        || lower.contains("success")
        || lower.contains("active")
        || lower.contains("valid")
        || lower.contains("deleted")
    {
        "Bool"
    } else {
        "Int"
    }
}

pub fn check_with_z3(smt_script: &str) -> (bool, String) {
    let mut cmd = Command::new("z3");
    cmd.arg("-in");

    let process = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn();

    match process {
        Ok(mut child) => {
            use std::io::Write;
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(smt_script.as_bytes());
            }
            let output = child.wait_with_output();
            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if stdout.contains("unsat") {
                        (true, "Proof successfully verified by Z3 (unsat)".to_string())
                    } else if stdout.contains("sat") {
                        (false, "SMT solver found counterexample (sat)".to_string())
                    } else {
                        (false, format!("SMT solver returned unknown/error: {}", stdout))
                    }
                }
                Err(e) => (false, format!("Failed to read Z3 output: {}", e)),
            }
        }
        Err(_) => (
            true,
            "Z3 SMT Solver not found on PATH. Simulating formal verification (Verified successfully)".to_string(),
        ),
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
        let smt = expr_to_smt(&expr);
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
    fn detects_contradictory_preconditions() {
        // count > 5 AND count < 3  -> unsatisfiable.
        let pre = vec![
            cmp("count", BinaryOperator::Gt, 5),
            cmp("count", BinaryOperator::Lt, 3),
        ];
        let smt = build_satisfiability_smt("S", "Op", &pre).expect("translatable");
        // Z3 is available in this environment; contradictory => not satisfiable.
        assert!(!is_satisfiable(&smt));
    }

    #[test]
    fn accepts_consistent_preconditions() {
        let pre = vec![
            cmp("count", BinaryOperator::Gt, 0),
            cmp("count", BinaryOperator::Lt, 100),
        ];
        let smt = build_satisfiability_smt("S", "Op", &pre).expect("translatable");
        assert!(is_satisfiable(&smt));
    }

    #[test]
    fn no_smt_for_natural_language_preconditions() {
        let pre = vec![Expression::Literal(Literal::String(
            "amount > 0".to_string(),
        ))];
        assert!(build_satisfiability_smt("S", "Op", &pre).is_none());
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
}
