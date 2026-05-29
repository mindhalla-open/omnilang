//! # omni-analyzer
//!
//! Semantic analyzer for OmniLang specifications.
//!
//! Takes an AST from `omni-parser` and performs:
//! - Name resolution (all types/services are defined)
//! - Type checking (inputs/outputs match between services)
//! - Constraint validation (detect conflicts)
//! - Dependency graph construction
//!
//! Produces a **Validated Spec IR** consumed by the orchestrator.

pub mod constraints;
pub mod deps;
pub mod formal;
pub mod gap_detector;
pub mod ir;
pub mod module_system;
pub mod policy;
pub mod schema;
pub mod symbols;
pub mod type_check;
pub mod type_mapping;
pub mod workflow;

use omni_parser::ParseError;
use omni_parser::ast::SourceFile;

pub use ir::SpecIR;
pub use symbols::SymbolTable;

/// Diagnostic message produced by the analyzer.
///
/// Every diagnostic carries a stable `code` (e.g. `E0301`) so it can be
/// referenced, documented (see `docs/18-error-codes.md`), and filtered
/// programmatically. Codes are grouped by analysis phase.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub code: &'static str,
    pub message: String,
    pub span: omni_parser::Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    Error,
    Warning,
    Info,
}

impl Diagnostic {
    /// Construct an error-level diagnostic with a stable code.
    pub fn error(code: &'static str, message: impl Into<String>, span: omni_parser::Span) -> Self {
        Self {
            kind: DiagnosticKind::Error,
            code,
            message: message.into(),
            span,
        }
    }

    /// Construct a warning-level diagnostic with a stable code.
    pub fn warning(
        code: &'static str,
        message: impl Into<String>,
        span: omni_parser::Span,
    ) -> Self {
        Self {
            kind: DiagnosticKind::Warning,
            code,
            message: message.into(),
            span,
        }
    }

    /// Construct an info-level diagnostic with a stable code.
    pub fn info(code: &'static str, message: impl Into<String>, span: omni_parser::Span) -> Self {
        Self {
            kind: DiagnosticKind::Info,
            code,
            message: message.into(),
            span,
        }
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prefix = match self.kind {
            DiagnosticKind::Error => "error",
            DiagnosticKind::Warning => "warning",
            DiagnosticKind::Info => "info",
        };
        write!(f, "{}[{}]: {}", prefix, self.code, self.message)
    }
}

/// Run the full analysis pipeline on a parsed source file.
/// Returns a validated SpecIR and any diagnostics.
pub fn analyze(file: &SourceFile) -> (Option<SpecIR>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();

    // Phase 1: Build symbol table
    let symbol_table = symbols::build_symbol_table(file, &mut diagnostics);

    // Phase 2: Type checking
    type_check::check_types(file, &symbol_table, &mut diagnostics);

    // Phase 3: Constraint validation
    constraints::validate_constraints(file, &mut diagnostics);

    // Phase 3.5: Module system checks (visibility, mixin expansion)
    module_system::check_visibility(file, &mut diagnostics);
    module_system::expand_mixins(file, &mut diagnostics);

    // Phase 4: Policy Enforcement
    policy::enforce_policies(file, &mut diagnostics);

    // Phase 4.1: Workflow Verification
    workflow::check_workflows(file, &mut diagnostics);

    // Phase 4.2: Intent Gap Detection
    gap_detector::analyze_gaps(file, &mut diagnostics);

    // Phase 4.3: Database Schema Evolution Check
    schema::check_schema_evolution(file, &mut diagnostics);

    // Phase 5.1: Formal Verification check
    formal::verify_proof_obligations(file, &mut diagnostics);

    // Phase 4: Dependency graph
    let dep_graph = deps::build_dependency_graph(file, &mut diagnostics);

    // If there are errors, don't produce IR
    let has_errors = diagnostics.iter().any(|d| d.kind == DiagnosticKind::Error);

    if has_errors {
        return (None, diagnostics);
    }

    // Phase 5: Build Spec IR
    let ir = ir::build_spec_ir(file, &symbol_table, &dep_graph);

    (Some(ir), diagnostics)
}

/// Run project-wide analysis on a set of parsed source files.
/// Resolves imports and merges the AST into a unified project SpecIR.
pub fn analyze_project(files: &[SourceFile]) -> (Option<SpecIR>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();

    if files.is_empty() {
        return (None, diagnostics);
    }

    // Step 1: Build a map of module paths to their exported symbols (Visibility::Public)
    let mut module_exports = std::collections::HashMap::new();
    for file in files {
        let mod_path = file.module.path.clone();
        let mut exports_table = SymbolTable::new();

        let mut local_diags = Vec::new();
        let local_table = symbols::build_symbol_table(file, &mut local_diags);
        for (name, symbol) in local_table.iter() {
            // Built-in types are always public but we don't need to re-export them from user modules
            if symbols::BUILTIN_TYPES.contains(&name.as_str())
                || symbols::BUILTIN_TYPES.contains(&name.to_lowercase().as_str())
            {
                continue;
            }
            // Check visibility of the local declaration
            let is_public = file.declarations.iter().any(|decl| {
                let (decl_name, vis) = match decl {
                    omni_parser::ast::Declaration::Type(t) => (t.name.clone(), t.visibility),
                    omni_parser::ast::Declaration::Service(s) => (s.name.clone(), s.visibility),
                    omni_parser::ast::Declaration::Mixin(m) => (m.name.clone(), m.visibility),
                    _ => return true, // All other declarations are public by default
                };
                decl_name == *name && vis == omni_parser::ast::Visibility::Public
            });

            if is_public {
                exports_table.insert(name.clone(), symbol.clone());
            }
        }
        module_exports.insert(mod_path, exports_table);
    }

    // Step 2: Merge all files into a single virtual SourceFile
    let mut merged_file = files[0].clone();
    for file in files.iter().skip(1) {
        merged_file.imports.extend(file.imports.clone());
        merged_file.exports.extend(file.exports.clone());
        merged_file.declarations.extend(file.declarations.clone());
    }

    // Step 3: Build a merged SymbolTable from the merged SourceFile
    let mut merged_symbols = symbols::build_symbol_table(&merged_file, &mut diagnostics);

    // Step 4: Resolve imports and register aliases/mappings in the merged symbol table
    for file in files {
        for import in &file.imports {
            let target_mod = &import.path;
            if let Some(exports) = module_exports.get(target_mod) {
                match &import.items {
                    omni_parser::ast::ImportItems::Wildcard => {
                        // Wildcard import: register all exports in the merged symbol table under their names
                        for (name, symbol) in exports.iter() {
                            merged_symbols.insert(name.clone(), symbol.clone());
                            merged_symbols.insert(name.to_lowercase(), symbol.clone());
                        }
                    }
                    omni_parser::ast::ImportItems::Named(items) => {
                        for item in items {
                            if let Some(symbol) = exports.get(&item.name) {
                                if let Some(alias) = &item.alias {
                                    let mut resolved_symbol = symbol.clone();
                                    resolved_symbol.name = alias.clone();
                                    merged_symbols.insert(alias.clone(), resolved_symbol.clone());
                                    merged_symbols.insert(alias.to_lowercase(), resolved_symbol);
                                }
                            } else {
                                diagnostics.push(Diagnostic {
                                    kind: DiagnosticKind::Error,
                                    code: "E0001",
                                    message: format!(
                                        "symbol '{}' not found in module '{}'",
                                        item.name,
                                        target_mod.join(".")
                                    ),
                                    span: item.span,
                                });
                            }
                        }
                    }
                }
            } else {
                // If the module is not found in local files, it might be std or registry/external.
                // We only report an error if it's not a standard library path (like std.*).
                if !target_mod.is_empty() && target_mod[0] != "std" {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Error,
                        code: "E0002",
                        message: format!("module '{}' not found", target_mod.join(".")),
                        span: import.span,
                    });
                }
            }
        }
    }

    // Step 5: Run all analysis check phases on the merged SourceFile using the merged SymbolTable
    type_check::check_types(&merged_file, &merged_symbols, &mut diagnostics);
    constraints::validate_constraints(&merged_file, &mut diagnostics);
    module_system::check_visibility(&merged_file, &mut diagnostics);
    module_system::expand_mixins(&merged_file, &mut diagnostics);
    policy::enforce_policies(&merged_file, &mut diagnostics);
    workflow::check_workflows(&merged_file, &mut diagnostics);
    gap_detector::analyze_gaps(&merged_file, &mut diagnostics);
    schema::check_schema_evolution(&merged_file, &mut diagnostics);
    formal::verify_proof_obligations(&merged_file, &mut diagnostics);

    let has_errors = diagnostics.iter().any(|d| d.kind == DiagnosticKind::Error);
    if has_errors {
        return (None, diagnostics);
    }

    // Step 6: Build the merged SpecIR
    let dep_graph = deps::build_dependency_graph(&merged_file, &mut diagnostics);
    let ir = ir::build_spec_ir(&merged_file, &merged_symbols, &dep_graph);

    (Some(ir), diagnostics)
}

/// Convenience: parse + analyze in one step
pub fn parse_and_analyze(source: &str) -> (Option<SpecIR>, Vec<Diagnostic>, Vec<ParseError>) {
    let (tokens, lex_errors) = omni_parser::Lexer::new(source).tokenize();

    if !lex_errors.is_empty() {
        return (None, Vec::new(), lex_errors);
    }

    let (file, parse_errors) = omni_parser::parser::Parser::new(tokens).parse();

    if !parse_errors.is_empty() {
        return (None, Vec::new(), parse_errors);
    }

    let (ir, diagnostics) = analyze(&file);
    (ir, diagnostics, Vec::new())
}
