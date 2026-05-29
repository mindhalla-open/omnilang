//! Constraint validation: detect conflicts and validate references.

use omni_parser::ast::{Declaration, Expression, Literal, SourceFile};

use crate::{Diagnostic, DiagnosticKind};

/// Validate constraints across the specification.
pub fn validate_constraints(file: &SourceFile, diagnostics: &mut Vec<Diagnostic>) {
    for decl in &file.declarations {
        if let Declaration::Service(s) = decl {
            // Warning: service without constraints
            if s.constraints.is_empty() {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::Warning,
                    code: "E0301",
                    message: format!(
                        "service '{}' has no constraints — consider adding latency, reliability, or security constraints",
                        s.name
                    ),
                    span: s.span,
                });
            }

            // Warning: service without goal
            if s.goal.is_none() {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::Warning,
                    code: "E0302",
                    message: format!(
                        "service '{}' has no goal — the goal field helps AI agents understand the intent",
                        s.name
                    ),
                    span: s.span,
                });
            }

            // Check for natural language invariants
            for inv in &s.invariants {
                if let Expression::Literal(Literal::String(text)) = inv {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Warning,
                        code: "E0303",
                        message: format!(
                            "Invariant '{}' in service '{}' is a natural language constraint and cannot be statically verified. Consider formalizing it as a mathematical expression.",
                            text, s.name
                        ),
                        span: s.span,
                    });
                }
            }

            // Check for conflicting constraints within a service
            let constraint_names: Vec<&str> =
                s.constraints.iter().map(|c| c.name.as_str()).collect();

            // Detect duplicates
            let mut seen = std::collections::HashSet::new();
            for name in &constraint_names {
                if !seen.insert(name) {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Error,
                        code: "E0304",
                        message: format!("duplicate constraint '{}' in service '{}'", name, s.name),
                        span: s.span,
                    });
                }
            }

            // Check Operation-level constraints
            for op in &s.operations {
                // Check for natural language preconditions
                for pre in &op.preconditions {
                    if let Expression::Literal(Literal::String(text)) = pre {
                        diagnostics.push(Diagnostic {
                            kind: DiagnosticKind::Warning,
                            code: "E0305",
                            message: format!(
                                "Precondition '{}' in operation '{}.{}' is a natural language constraint and cannot be statically verified. Consider formalizing it as a mathematical expression.",
                                text, s.name, op.name
                            ),
                            span: op.span,
                        });
                    }
                }

                // Check for natural language postconditions
                for post in &op.postconditions {
                    if let Expression::Literal(Literal::String(text)) = post {
                        diagnostics.push(Diagnostic {
                            kind: DiagnosticKind::Warning,
                            code: "E0306",
                            message: format!(
                                "Postcondition '{}' in operation '{}.{}' is a natural language constraint and cannot be statically verified. Consider formalizing it as a mathematical expression.",
                                text, s.name, op.name
                            ),
                            span: op.span,
                        });
                    }
                }

                if op.tests.is_empty() {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Info,
                        code: "E0307",
                        message: format!(
                            "operation '{}.{}' has no test scenarios — consider adding tests for verification",
                            s.name, op.name
                        ),
                        span: op.span,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omni_parser::Lexer;
    use omni_parser::parser::Parser;

    fn parse_and_validate(input: &str) -> Vec<Diagnostic> {
        let (tokens, _) = Lexer::new(input).tokenize();
        let (file, _) = Parser::new(tokens).parse();
        let mut diags = Vec::new();
        validate_constraints(&file, &mut diags);
        diags
    }

    #[test]
    fn service_without_constraints_warns() {
        let diags = parse_and_validate(
            r#"module test
service API {
  goal: "Test"
}"#,
        );
        assert!(
            diags
                .iter()
                .any(|d| d.kind == DiagnosticKind::Warning && d.message.contains("no constraints"))
        );
    }

    #[test]
    fn service_without_goal_warns() {
        let diags = parse_and_validate(
            r#"module test
service API {
  constraints:
    - idempotent
}"#,
        );
        assert!(
            diags
                .iter()
                .any(|d| d.kind == DiagnosticKind::Warning && d.message.contains("no goal"))
        );
    }

    #[test]
    fn service_with_goal_and_constraints_ok() {
        let diags = parse_and_validate(
            r#"module test
service API {
  goal: "Test API"
  constraints:
    - idempotent
}"#,
        );
        // Should only have info-level, no errors or warnings
        assert!(diags.iter().all(|d| d.kind != DiagnosticKind::Error));
    }

    #[test]
    fn natural_language_constraints_warn() {
        let diags = parse_and_validate(
            r#"module test
service API {
  goal: "Test"
  invariants:
    - "Service must always be active"
  operation Greet(name: String) -> String {
    preconditions:
      - "name must not be empty"
    postconditions:
      - "return message contains name"
  }
}"#,
        );
        let warnings: Vec<_> = diags
            .iter()
            .filter(|d| d.kind == DiagnosticKind::Warning)
            .collect();
        assert!(warnings.iter().any(|d| {
            d.message
                .contains("Invariant 'Service must always be active'")
        }));
        assert!(
            warnings
                .iter()
                .any(|d| d.message.contains("Precondition 'name must not be empty'"))
        );
        assert!(warnings.iter().any(|d| {
            d.message
                .contains("Postcondition 'return message contains name'")
        }));
    }
}
