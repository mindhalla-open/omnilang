use crate::{Diagnostic, DiagnosticKind};
use omni_parser::ast::{Declaration, SourceFile, WorkflowDecl};
use std::collections::{HashMap, HashSet, VecDeque};

/// Check workflow transitions, detect dead states, and validate state definitions
pub fn check_workflows(file: &SourceFile, diagnostics: &mut Vec<Diagnostic>) {
    for decl in &file.declarations {
        if let Declaration::Workflow(w) = decl {
            validate_workflow_transitions(w, diagnostics);
            detect_dead_states(w, diagnostics);
        }
    }
}

fn is_wildcard_state(state: &str) -> bool {
    state.starts_with("AnyState")
}

fn get_matching_concrete_states<'a>(wildcard: &str, all_states: &'a [String]) -> Vec<&'a str> {
    let mut matches = Vec::new();
    let except_suffix = "AnyStateExcept";
    let except_state = wildcard.strip_prefix(except_suffix);

    for s in all_states {
        let s_str = s.as_str();
        if is_wildcard_state(s_str) {
            continue;
        }
        if let Some(exc) = except_state
            && s_str == exc
        {
            continue;
        }
        matches.push(s_str);
    }
    matches
}

fn validate_workflow_transitions(w: &WorkflowDecl, diagnostics: &mut Vec<Diagnostic>) {
    let states_set: HashSet<&str> = w.states.iter().map(|s| s.as_str()).collect();

    for trans in &w.transitions {
        // Validate from state (which can be a wildcard)
        if trans.from.starts_with("AnyStateExcept") {
            let except_state = &trans.from["AnyStateExcept".len()..];
            if !states_set.contains(except_state) {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::Error,
                    code: "E0601",
                    message: format!(
                        "Workflow '{}': wildcard transition excepts undefined state '{}'",
                        w.name, except_state
                    ),
                    span: trans.span,
                });
            }
        } else if !is_wildcard_state(trans.from.as_str())
            && !states_set.contains(trans.from.as_str())
        {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::Error,
                code: "E0602",
                message: format!(
                    "Workflow '{}': transition uses undefined source state '{}'",
                    w.name, trans.from
                ),
                span: trans.span,
            });
        }

        // Validate to state
        if !states_set.contains(trans.to.as_str()) {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::Error,
                code: "E0603",
                message: format!(
                    "Workflow '{}': transition uses undefined target state '{}'",
                    w.name, trans.to
                ),
                span: trans.span,
            });
        }

        // Validate timeout target state if configured
        if let Some(timeout) = trans
            .timeout
            .as_ref()
            .filter(|t| !states_set.contains(t.target_state.as_str()))
        {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::Error,
                code: "E0604",
                message: format!(
                    "Workflow '{}': timeout transition uses undefined target state '{}'",
                    w.name, timeout.target_state
                ),
                span: timeout.span,
            });
        }
    }
}

fn detect_dead_states(w: &WorkflowDecl, diagnostics: &mut Vec<Diagnostic>) {
    if w.states.is_empty() {
        return;
    }

    let start_state = &w.states[0];
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    // Build transition adjacency list expanding wildcards
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for trans in &w.transitions {
        let sources = if is_wildcard_state(&trans.from) {
            get_matching_concrete_states(&trans.from, &w.states)
        } else {
            vec![trans.from.as_str()]
        };

        for src in sources {
            adj.entry(src).or_default().push(trans.to.as_str());
            if let Some(timeout) = &trans.timeout {
                adj.entry(src)
                    .or_default()
                    .push(timeout.target_state.as_str());
            }
        }
    }

    visited.insert(start_state.as_str());
    queue.push_back(start_state.as_str());

    while let Some(current) = queue.pop_front() {
        if let Some(neighbors) = adj.get(current) {
            for &next in neighbors {
                if !visited.contains(next) {
                    visited.insert(next);
                    queue.push_back(next);
                }
            }
        }
    }

    for state in &w.states {
        // Skip wildcard states since they are meta-states, not concrete reachable states
        if is_wildcard_state(state.as_str()) {
            continue;
        }
        if !visited.contains(state.as_str()) {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::Warning,
                code: "E0605",
                message: format!(
                    "Workflow '{}': state '{}' is unreachable (dead state)",
                    w.name, state
                ),
                span: w.span,
            });
        }
    }
}

/// Helper to generate Mermaid state machine representation
pub fn generate_mermaid_chart(w: &WorkflowDecl) -> String {
    let mut chart = String::new();
    chart.push_str("stateDiagram-v2\n");

    if !w.states.is_empty() {
        chart.push_str(&format!("  [*] --> {}\n", w.states[0]));
    }

    for trans in &w.transitions {
        let trigger = trans.trigger.as_deref().unwrap_or("transition");
        chart.push_str(&format!(
            "  {} --> {} : {}\n",
            trans.from, trans.to, trigger
        ));

        if let Some(timeout) = &trans.timeout {
            chart.push_str(&format!(
                "  {} --> {} : timeout\n",
                trans.from, timeout.target_state
            ));
        }
    }

    chart
}
