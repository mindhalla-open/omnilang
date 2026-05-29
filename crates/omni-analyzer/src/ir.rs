//! Spec IR — the validated intermediate representation output by the analyzer.

use omni_parser::ast::{
    Declaration, Expression, Literal, RefinedType, SourceFile, TrustLevel, TypeKind,
};

use crate::deps::DependencyGraph;
use crate::symbols::SymbolTable;
use crate::type_mapping::TypeMapping;

/// Current Spec IR schema version. The orchestrator/codegen runtime rejects IR
/// whose `ir_version` it does not understand, so this must be bumped whenever the
/// IR shape changes in a way consumers must be aware of.
pub const CURRENT_IR_VERSION: &str = "1";

/// Validated Specification IR — the output of the analysis phase.
/// This is consumed by the orchestrator/codegen pipeline.
#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct SpecIR {
    /// Schema version of this IR document. See [`CURRENT_IR_VERSION`].
    pub ir_version: String,
    /// Module path: `["acme", "payments", "checkout"]`
    pub module_path: Vec<String>,
    /// The parsed source file AST containing all declarations
    pub source_file: SourceFile,
    /// All type definitions
    pub types: Vec<TypeDef>,
    /// All service definitions
    pub services: Vec<ServiceDef>,
    /// Build order (topologically sorted)
    pub build_order: Vec<String>,
    /// Type mappings for the default target (typescript)
    pub type_mappings: Vec<TypeMapping>,
    /// Summary statistics
    pub stats: SpecStats,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct TypeDef {
    pub name: String,
    pub kind: String, // "enum", "struct", "refined", "alias"
    pub field_count: usize,
    /// Generator configuration for property-based testing.
    /// Present only for refined types with constraints.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generator: Option<GeneratorConfig>,
}

/// Configuration for generating constraint-aware arbitrary values.
/// Extracted from refined type constraints to drive property-based test generators.
#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct GeneratorConfig {
    /// Minimum value for numeric ranges (from `range: [min, max]`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    /// Maximum value for numeric ranges (from `range: [min, max]`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// Minimum string length (from `min_length`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    /// Maximum string length (from `max_length`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    /// Format pattern (from `format: regex(...)` or `format: "..."`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format_pattern: Option<String>,
    /// Decimal precision digits (from `precision`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<usize>,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct ServiceDef {
    pub name: String,
    pub goal: Option<String>,
    /// Per-service target language override; `None` = build-wide target.
    pub target: Option<String>,
    pub operation_count: usize,
    pub operation_names: Vec<String>,
    pub constraint_count: usize,
    pub constraint_names: Vec<String>,
    pub dependency_count: usize,
    pub test_count: usize,
    pub metric_count: usize,
    pub metric_names: Vec<String>,
    pub confidence: TrustLevel,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct SpecStats {
    pub type_count: usize,
    pub service_count: usize,
    pub operation_count: usize,
    pub test_count: usize,
    pub constraint_count: usize,
    pub metric_count: usize,
    pub component_count: usize,
    pub pipeline_count: usize,
    pub workflow_count: usize,
    pub agent_count: usize,
    pub schema_count: usize,
    pub policy_count: usize,
}

/// Build the validated Spec IR from analyzed data.
pub fn build_spec_ir(
    file: &SourceFile,
    _symbols: &SymbolTable,
    dep_graph: &DependencyGraph,
) -> SpecIR {
    let confidence_map = crate::type_check::compute_confidence_map(file);
    let mut types = Vec::new();
    let mut services = Vec::new();
    let mut total_operations = 0;
    let mut total_tests = 0;
    let mut total_constraints = 0;
    let mut total_metrics = 0;

    let mut component_count = 0;
    let mut pipeline_count = 0;
    let mut workflow_count = 0;
    let mut agent_count = 0;
    let mut schema_count = 0;
    let mut policy_count = 0;

    for decl in &file.declarations {
        match decl {
            Declaration::Type(t) => {
                let (kind, field_count, gen_config) = match &t.kind {
                    TypeKind::Enum(e) => ("enum".to_string(), e.variants.len(), None),
                    TypeKind::Struct(s) => ("struct".to_string(), s.fields.len(), None),
                    TypeKind::Refined(r) => ("refined".to_string(), 0, extract_generator_config(r)),
                    TypeKind::Alias(_) => ("alias".to_string(), 0, None),
                };
                types.push(TypeDef {
                    name: t.name.clone(),
                    kind,
                    field_count,
                    generator: gen_config,
                });
            }
            Declaration::Service(s) => {
                let operation_names: Vec<String> =
                    s.operations.iter().map(|r| r.name.clone()).collect();
                let constraint_names: Vec<String> =
                    s.constraints.iter().map(|c| c.name.clone()).collect();
                let metric_names: Vec<String> = s.metrics.iter().map(|m| m.name.clone()).collect();
                let test_count: usize = s.operations.iter().map(|r| r.tests.len()).sum();

                total_operations += s.operations.len();
                total_tests += test_count;
                total_constraints += s.constraints.len();
                total_metrics += s.metrics.len();

                let (confidence, evidence) = confidence_map
                    .get(&s.name)
                    .cloned()
                    .unwrap_or((TrustLevel::Speculative, Vec::new()));

                services.push(ServiceDef {
                    name: s.name.clone(),
                    goal: s.goal.clone(),
                    target: s.target.clone(),
                    operation_count: s.operations.len(),
                    operation_names,
                    constraint_count: s.constraints.len(),
                    constraint_names,
                    dependency_count: s.depends_on.len(),
                    test_count,
                    metric_count: s.metrics.len(),
                    metric_names,
                    confidence,
                    evidence,
                });
            }
            Declaration::Component(c) => {
                component_count += 1;
                total_constraints += c.constraints.len();
                total_tests += c.tests.len();
            }
            Declaration::Pipeline(p) => {
                pipeline_count += 1;
                total_constraints += p.constraints.len();
                total_tests += p.tests.len();
            }
            Declaration::Workflow(w) => {
                workflow_count += 1;
                total_constraints += w.constraints.len();
                total_tests += w.tests.len();
            }
            Declaration::Agent(a) => {
                agent_count += 1;
                total_tests += a.tests.len();
            }
            Declaration::Schema(s) => {
                schema_count += 1;
                total_constraints += s.constraints.len();
            }
            Declaration::Policy(_) => {
                policy_count += 1;
            }
            Declaration::Constraint(_) => {}
            Declaration::Mixin(m) => {
                total_constraints += m.constraints.len();
                total_tests += m.tests.len();
            }
            Declaration::TargetDependencies(_) => {}
            Declaration::Entity(_) => {}
            Declaration::Action(_) => {}
            Declaration::Rule(_) => {}
        }
    }

    let stats = SpecStats {
        type_count: types.len(),
        service_count: services.len(),
        operation_count: total_operations,
        test_count: total_tests,
        constraint_count: total_constraints,
        metric_count: total_metrics,
        component_count,
        pipeline_count,
        workflow_count,
        agent_count,
        schema_count,
        policy_count,
    };

    SpecIR {
        ir_version: CURRENT_IR_VERSION.to_string(),
        module_path: file.module.path.clone(),
        source_file: file.clone(),
        types,
        services,
        build_order: dep_graph.order.clone(),
        type_mappings: crate::type_mapping::get_type_mappings("typescript"),
        stats,
    }
}

/// Extract generator configuration from a refined type's constraints.
fn extract_generator_config(r: &RefinedType) -> Option<GeneratorConfig> {
    let mut config = GeneratorConfig {
        min: None,
        max: None,
        min_length: None,
        max_length: None,
        format_pattern: None,
        precision: None,
    };
    let mut has_any = false;

    for constraint in &r.constraints {
        match constraint.name.as_str() {
            "range" => {
                if let Expression::List(elements, _) = &constraint.value
                    && elements.len() == 2
                {
                    config.min = extract_number(&elements[0]);
                    config.max = extract_number(&elements[1]);
                    has_any = true;
                }
            }
            "min_length" => {
                if let Some(n) = extract_usize(&constraint.value) {
                    config.min_length = Some(n);
                    has_any = true;
                }
            }
            "max_length" => {
                if let Some(n) = extract_usize(&constraint.value) {
                    config.max_length = Some(n);
                    has_any = true;
                }
            }
            "precision" => {
                if let Some(n) = extract_usize(&constraint.value) {
                    config.precision = Some(n);
                    has_any = true;
                }
            }
            "format" => match &constraint.value {
                Expression::Call { function, args, .. } if function == "regex" => {
                    if let Some(Expression::Literal(Literal::String(s))) = args.first() {
                        config.format_pattern = Some(s.clone());
                        has_any = true;
                    }
                }
                Expression::Literal(Literal::String(s)) => {
                    config.format_pattern = Some(s.clone());
                    has_any = true;
                }
                _ => {}
            },
            _ => {}
        }
    }

    if has_any { Some(config) } else { None }
}

fn extract_number(expr: &Expression) -> Option<f64> {
    match expr {
        Expression::Literal(Literal::Int(n)) => Some(*n as f64),
        Expression::Literal(Literal::Float(n)) => Some(*n),
        _ => None,
    }
}

fn extract_usize(expr: &Expression) -> Option<usize> {
    if let Expression::Literal(Literal::Int(n)) = expr
        && *n >= 0
    {
        return Some(*n as usize);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deps;
    use crate::symbols;

    #[test]
    fn test_generator_config_from_refined_type() {
        let source = r#"
module test

type Price = Float {
  range: [0.00, 999_999.99]
  precision: 2
}
"#;
        let (tokens, _) = omni_parser::Lexer::new(source).tokenize();
        let (file, _) = omni_parser::parser::Parser::new(tokens).parse();
        let mut diags = Vec::new();
        let symbols = symbols::build_symbol_table(&file, &mut diags);
        let dep_graph = deps::build_dependency_graph(&file, &mut diags);
        let ir = build_spec_ir(&file, &symbols, &dep_graph);

        let price_type = ir.types.iter().find(|t| t.name == "Price").unwrap();
        assert_eq!(price_type.kind, "refined");
        let gen_cfg = price_type
            .generator
            .as_ref()
            .expect("expected GeneratorConfig");
        assert_eq!(gen_cfg.min, Some(0.0));
        assert_eq!(gen_cfg.max, Some(999_999.99));
        assert_eq!(gen_cfg.precision, Some(2));
        assert!(gen_cfg.min_length.is_none());
        assert!(gen_cfg.format_pattern.is_none());
    }

    #[test]
    fn test_generator_config_for_string_type() {
        let source = r#"
module test

type Email = String {
  format: regex("^[a-z]+@[a-z]+\.com$")
  min_length: 5
  max_length: 255
}
"#;
        let (tokens, _) = omni_parser::Lexer::new(source).tokenize();
        let (file, _) = omni_parser::parser::Parser::new(tokens).parse();
        let mut diags = Vec::new();
        let symbols = symbols::build_symbol_table(&file, &mut diags);
        let dep_graph = deps::build_dependency_graph(&file, &mut diags);
        let ir = build_spec_ir(&file, &symbols, &dep_graph);

        let email_type = ir.types.iter().find(|t| t.name == "Email").unwrap();
        let gen_cfg = email_type
            .generator
            .as_ref()
            .expect("expected GeneratorConfig");
        assert_eq!(gen_cfg.min_length, Some(5));
        assert_eq!(gen_cfg.max_length, Some(255));
        assert!(gen_cfg.format_pattern.is_some());
        assert!(gen_cfg.min.is_none());
    }

    #[test]
    fn test_no_generator_config_for_struct() {
        let source = "module test\ntype User = struct {\n  name: String\n  age: Int\n}\n";
        let (tokens, _) = omni_parser::Lexer::new(source).tokenize();
        let (file, _) = omni_parser::parser::Parser::new(tokens).parse();
        let mut diags = Vec::new();
        let symbols = symbols::build_symbol_table(&file, &mut diags);
        let dep_graph = deps::build_dependency_graph(&file, &mut diags);
        let ir = build_spec_ir(&file, &symbols, &dep_graph);

        let user_type = ir.types.iter().find(|t| t.name == "User").unwrap();
        assert_eq!(user_type.kind, "struct");
        assert!(user_type.generator.is_none());
    }
}
