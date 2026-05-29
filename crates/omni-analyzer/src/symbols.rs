//! Symbol table construction and name resolution.

use std::collections::HashMap;

use omni_parser::Span;
use omni_parser::ast::{Declaration, SourceFile, TypeKind};

use crate::{Diagnostic, DiagnosticKind};

/// A resolved symbol in the symbol table.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
    pub type_params: Vec<TypeParamSymbol>,
}

#[derive(Debug, Clone)]
pub struct TypeParamSymbol {
    pub name: String,
    pub bounds: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Type,
    Enum,
    Struct,
    Service,
    Component,
    Pipeline,
    Workflow,
    Agent,
    Schema,
    Policy,
    Constraint,
    Entity,
    Action,
    Rule,
}

/// The symbol table maps names to their definitions.
#[derive(Debug, Default)]
pub struct SymbolTable {
    symbols: HashMap<String, Symbol>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: String, symbol: Symbol) -> Option<Symbol> {
        self.symbols.insert(name, symbol)
    }

    pub fn get(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.symbols.contains_key(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Symbol)> {
        self.symbols.iter()
    }

    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }
}

/// Built-in types that are always available.
pub const BUILTIN_TYPES: &[&str] = &[
    "String",
    "Int",
    "Float",
    "Bool",
    "UUID",
    "Email",
    "URL",
    "DateTime",
    "Date",
    "Time",
    "Timestamp",
    "Duration",
    "Money",
    "Decimal",
    "Bytes",
    "Json",
    "Void",
    "Any",
    "List",
    "Set",
    "Map",
    "Option",
    "Result",
    "Confident",
    "Image",
    "Screenshot",
    "Trace",
    "Log",
    "Diagram",
];

/// Build a symbol table from a parsed source file.
pub fn build_symbol_table(file: &SourceFile, diagnostics: &mut Vec<Diagnostic>) -> SymbolTable {
    let mut table = SymbolTable::new();

    // Register built-in types
    for &name in BUILTIN_TYPES {
        let type_params = match name {
            "Option" | "List" | "Set" | "Confident" => vec![TypeParamSymbol {
                name: "T".to_string(),
                bounds: vec![],
            }],
            "Result" => vec![
                TypeParamSymbol {
                    name: "T".to_string(),
                    bounds: vec![],
                },
                TypeParamSymbol {
                    name: "E".to_string(),
                    bounds: vec![],
                },
            ],
            "Map" => vec![
                TypeParamSymbol {
                    name: "K".to_string(),
                    bounds: vec![],
                },
                TypeParamSymbol {
                    name: "V".to_string(),
                    bounds: vec![],
                },
            ],
            _ => vec![],
        };
        let symbol = Symbol {
            name: name.to_string(),
            kind: SymbolKind::Type,
            span: Span::new(0, 0),
            type_params,
        };
        table.insert(name.to_string(), symbol.clone());
        table.insert(name.to_lowercase(), symbol);
    }

    // Register all declarations
    for decl in &file.declarations {
        match decl {
            Declaration::Type(t) => {
                let kind = match &t.kind {
                    TypeKind::Enum(_) => SymbolKind::Enum,
                    TypeKind::Struct(_) => SymbolKind::Struct,
                    _ => SymbolKind::Type,
                };

                if let Some(prev) = table.get(&t.name)
                    && (prev.span.start != 0 || prev.span.end != 0)
                {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Error,
                        code: "E0101",
                        message: format!("duplicate type definition: '{}'", t.name),
                        span: t.span,
                    });
                }

                let type_params = t
                    .type_params
                    .iter()
                    .map(|p| TypeParamSymbol {
                        name: p.name.clone(),
                        bounds: p.bounds.iter().map(|b| b.name.clone()).collect(),
                    })
                    .collect();

                table.insert(
                    t.name.clone(),
                    Symbol {
                        name: t.name.clone(),
                        kind,
                        span: t.span,
                        type_params,
                    },
                );
            }
            Declaration::Service(s) => {
                if let Some(prev) = table.get(&s.name)
                    && (prev.span.start != 0 || prev.span.end != 0)
                {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Error,
                        code: "E0102",
                        message: format!("duplicate service definition: '{}'", s.name),
                        span: s.span,
                    });
                }

                table.insert(
                    s.name.clone(),
                    Symbol {
                        name: s.name.clone(),
                        kind: SymbolKind::Service,
                        span: s.span,
                        type_params: Vec::new(),
                    },
                );
            }
            Declaration::Component(c) => {
                if let Some(prev) = table.get(&c.name)
                    && (prev.span.start != 0 || prev.span.end != 0)
                {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Error,
                        code: "E0103",
                        message: format!("duplicate component definition: '{}'", c.name),
                        span: c.span,
                    });
                }
                table.insert(
                    c.name.clone(),
                    Symbol {
                        name: c.name.clone(),
                        kind: SymbolKind::Component,
                        span: c.span,
                        type_params: Vec::new(),
                    },
                );
            }
            Declaration::Pipeline(p) => {
                if let Some(prev) = table.get(&p.name)
                    && (prev.span.start != 0 || prev.span.end != 0)
                {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Error,
                        code: "E0104",
                        message: format!("duplicate pipeline definition: '{}'", p.name),
                        span: p.span,
                    });
                }
                table.insert(
                    p.name.clone(),
                    Symbol {
                        name: p.name.clone(),
                        kind: SymbolKind::Pipeline,
                        span: p.span,
                        type_params: Vec::new(),
                    },
                );
            }
            Declaration::Workflow(w) => {
                if let Some(prev) = table.get(&w.name)
                    && (prev.span.start != 0 || prev.span.end != 0)
                {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Error,
                        code: "E0105",
                        message: format!("duplicate workflow definition: '{}'", w.name),
                        span: w.span,
                    });
                }
                table.insert(
                    w.name.clone(),
                    Symbol {
                        name: w.name.clone(),
                        kind: SymbolKind::Workflow,
                        span: w.span,
                        type_params: Vec::new(),
                    },
                );
            }
            Declaration::Agent(a) => {
                if let Some(prev) = table.get(&a.name)
                    && (prev.span.start != 0 || prev.span.end != 0)
                {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Error,
                        code: "E0106",
                        message: format!("duplicate agent definition: '{}'", a.name),
                        span: a.span,
                    });
                }
                table.insert(
                    a.name.clone(),
                    Symbol {
                        name: a.name.clone(),
                        kind: SymbolKind::Agent,
                        span: a.span,
                        type_params: Vec::new(),
                    },
                );
            }
            Declaration::Schema(s) => {
                if let Some(prev) = table.get(&s.name)
                    && (prev.span.start != 0 || prev.span.end != 0)
                {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Error,
                        code: "E0107",
                        message: format!("duplicate schema definition: '{}'", s.name),
                        span: s.span,
                    });
                }
                table.insert(
                    s.name.clone(),
                    Symbol {
                        name: s.name.clone(),
                        kind: SymbolKind::Schema,
                        span: s.span,
                        type_params: Vec::new(),
                    },
                );
            }
            Declaration::Policy(p) => {
                if let Some(prev) = table.get(&p.name)
                    && (prev.span.start != 0 || prev.span.end != 0)
                {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Error,
                        code: "E0108",
                        message: format!("duplicate policy definition: '{}'", p.name),
                        span: p.span,
                    });
                }
                table.insert(
                    p.name.clone(),
                    Symbol {
                        name: p.name.clone(),
                        kind: SymbolKind::Policy,
                        span: p.span,
                        type_params: Vec::new(),
                    },
                );
            }
            Declaration::Constraint(c) => {
                if let Some(prev) = table.get(&c.name)
                    && (prev.span.start != 0 || prev.span.end != 0)
                {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Error,
                        code: "E0109",
                        message: format!("duplicate constraint definition: '{}'", c.name),
                        span: c.span,
                    });
                }
                table.insert(
                    c.name.clone(),
                    Symbol {
                        name: c.name.clone(),
                        kind: SymbolKind::Constraint,
                        span: c.span,
                        type_params: Vec::new(),
                    },
                );
            }
            Declaration::Mixin(m) => {
                if let Some(prev) = table.get(&m.name)
                    && (prev.span.start != 0 || prev.span.end != 0)
                {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Error,
                        code: "E0110",
                        message: format!("duplicate mixin definition: '{}'", m.name),
                        span: m.span,
                    });
                }
                table.insert(
                    m.name.clone(),
                    Symbol {
                        name: m.name.clone(),
                        kind: SymbolKind::Constraint, // Mixins act like constraint bundles
                        span: m.span,
                        type_params: Vec::new(),
                    },
                );
            }
            Declaration::TargetDependencies(_) => {}
            Declaration::Entity(e) => {
                if let Some(prev) = table.get(&e.name)
                    && (prev.span.start != 0 || prev.span.end != 0)
                {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Error,
                        code: "E0111",
                        message: format!("duplicate entity definition: '{}'", e.name),
                        span: e.span,
                    });
                }
                table.insert(
                    e.name.clone(),
                    Symbol {
                        name: e.name.clone(),
                        kind: SymbolKind::Entity,
                        span: e.span,
                        type_params: Vec::new(),
                    },
                );
            }
            Declaration::Action(a) => {
                if let Some(prev) = table.get(&a.name)
                    && (prev.span.start != 0 || prev.span.end != 0)
                {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Error,
                        code: "E0112",
                        message: format!("duplicate action definition: '{}'", a.name),
                        span: a.span,
                    });
                }
                table.insert(
                    a.name.clone(),
                    Symbol {
                        name: a.name.clone(),
                        kind: SymbolKind::Action,
                        span: a.span,
                        type_params: Vec::new(),
                    },
                );
            }
            Declaration::Rule(r) => {
                if let Some(prev) = table.get(&r.name)
                    && (prev.span.start != 0 || prev.span.end != 0)
                {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Error,
                        code: "E0113",
                        message: format!("duplicate rule definition: '{}'", r.name),
                        span: r.span,
                    });
                }
                table.insert(
                    r.name.clone(),
                    Symbol {
                        name: r.name.clone(),
                        kind: SymbolKind::Rule,
                        span: r.span,
                        type_params: Vec::new(),
                    },
                );
            }
        }
    }

    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use omni_parser::Lexer;
    use omni_parser::parser::Parser;

    fn parse_and_build(input: &str) -> (SymbolTable, Vec<Diagnostic>) {
        let (tokens, _) = Lexer::new(input).tokenize();
        let (file, _) = Parser::new(tokens).parse();
        let mut diags = Vec::new();
        let table = build_symbol_table(&file, &mut diags);
        (table, diags)
    }

    #[test]
    fn builtin_types_are_registered() {
        let (table, _) = parse_and_build("module test");
        assert!(table.contains("String"));
        assert!(table.contains("Int"));
        assert!(table.contains("List"));
        assert!(table.contains("UUID"));
    }

    #[test]
    fn user_types_are_registered() {
        let (table, diags) =
            parse_and_build("module test\ntype OrderId = UUID\ntype Status = enum { Active }");
        assert!(diags.is_empty());
        assert!(table.contains("OrderId"));
        assert!(table.contains("Status"));
        assert_eq!(table.get("Status").unwrap().kind, SymbolKind::Enum);
    }

    #[test]
    fn services_are_registered() {
        let (table, diags) = parse_and_build("module test\nservice Checkout { goal: \"test\" }");
        assert!(diags.is_empty());
        assert!(table.contains("Checkout"));
        assert_eq!(table.get("Checkout").unwrap().kind, SymbolKind::Service);
    }
}
