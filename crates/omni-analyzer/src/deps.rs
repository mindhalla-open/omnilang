//! Dependency graph construction and cycle detection.

use std::collections::{HashMap, HashSet};

use omni_parser::ast::{Declaration, SourceFile};

use crate::{Diagnostic, DiagnosticKind};

/// A dependency graph of services.
#[derive(Debug, Default)]
pub struct DependencyGraph {
    /// Maps service name → set of services it depends on.
    edges: HashMap<String, Vec<String>>,
    /// Topological order (empty if cycles detected).
    pub order: Vec<String>,
}

impl DependencyGraph {
    pub fn dependencies(&self, service: &str) -> &[String] {
        self.edges.get(service).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn services(&self) -> Vec<&String> {
        self.edges.keys().collect()
    }
}

/// Build a dependency graph from `depends_on` declarations.
pub fn build_dependency_graph(
    file: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
) -> DependencyGraph {
    let mut graph = DependencyGraph::default();

    // Collect all services
    let service_names: HashSet<String> = file
        .declarations
        .iter()
        .filter_map(|d| {
            if let Declaration::Service(s) = d {
                Some(s.name.clone())
            } else {
                None
            }
        })
        .collect();

    // Build edges
    for decl in &file.declarations {
        if let Declaration::Service(s) = decl {
            let mut deps = Vec::new();

            for dep_name in &s.depends_on {
                if !service_names.contains(dep_name) {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Error,
                        code: "E1001",
                        message: format!(
                            "service '{}' depends on undefined service '{}'",
                            s.name, dep_name
                        ),
                        span: s.span,
                    });
                } else {
                    deps.push(dep_name.clone());
                }
            }

            graph.edges.insert(s.name.clone(), deps);
        }
    }

    // Topological sort + cycle detection
    match topological_sort(&graph.edges) {
        Ok(order) => graph.order = order,
        Err(cycle) => {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::Error,
                code: "E1002",
                message: format!("circular dependency detected: {}", cycle.join(" → ")),
                span: omni_parser::Span::new(0, 0),
            });
        }
    }

    graph
}

/// Topological sort using DFS. Returns Err with cycle path if detected.
fn topological_sort(edges: &HashMap<String, Vec<String>>) -> Result<Vec<String>, Vec<String>> {
    #[derive(Clone, Copy, PartialEq)]
    enum State {
        Unvisited,
        InProgress,
        Done,
    }

    let mut state: HashMap<&str, State> = edges
        .keys()
        .map(|k| (k.as_str(), State::Unvisited))
        .collect();
    let mut order = Vec::new();
    let mut path = Vec::new();

    fn visit<'a>(
        node: &'a str,
        edges: &'a HashMap<String, Vec<String>>,
        state: &mut HashMap<&'a str, State>,
        order: &mut Vec<String>,
        path: &mut Vec<String>,
    ) -> Result<(), Vec<String>> {
        match state.get(node) {
            Some(State::Done) => return Ok(()),
            Some(State::InProgress) => {
                // Found a cycle
                path.push(node.to_string());
                return Err(path.clone());
            }
            _ => {}
        }

        state.insert(node, State::InProgress);
        path.push(node.to_string());

        if let Some(deps) = edges.get(node) {
            for dep in deps {
                visit(dep, edges, state, order, path)?;
            }
        }

        path.pop();
        state.insert(node, State::Done);
        order.push(node.to_string());
        Ok(())
    }

    for node in edges.keys() {
        if state.get(node.as_str()) == Some(&State::Unvisited) {
            visit(node, edges, &mut state, &mut order, &mut path)?;
        }
    }

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use omni_parser::Lexer;
    use omni_parser::parser::Parser;

    fn parse_and_graph(input: &str) -> (DependencyGraph, Vec<Diagnostic>) {
        let (tokens, _) = Lexer::new(input).tokenize();
        let (file, _) = Parser::new(tokens).parse();
        let mut diags = Vec::new();
        let graph = build_dependency_graph(&file, &mut diags);
        (graph, diags)
    }

    #[test]
    fn no_dependencies() {
        let (graph, diags) = parse_and_graph(
            r#"module test
service A { goal: "A" }
service B { goal: "B" }"#,
        );
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.kind == DiagnosticKind::Error)
            .collect();
        assert!(errors.is_empty());
        assert_eq!(graph.services().len(), 2);
    }

    #[test]
    fn undefined_dependency() {
        let (_, diags) = parse_and_graph(
            r#"module test
service A {
  goal: "A"
  depends_on:
    - NonExistent
}"#,
        );
        assert!(diags.iter().any(|d| d.kind == DiagnosticKind::Error
            && d.message.contains("undefined service")));
    }

    #[test]
    fn detects_circular_dependency() {
        let (graph, diags) = parse_and_graph(
            r#"module test
service A {
  goal: "A"
  depends_on:
    - B
}
service B {
  goal: "B"
  depends_on:
    - A
}"#,
        );
        assert!(
            diags
                .iter()
                .any(|d| d.kind == DiagnosticKind::Error && d.message.contains("circular")),
            "expected a circular-dependency error"
        );
        assert!(graph.order.is_empty(), "no build order when a cycle exists");
    }
}
