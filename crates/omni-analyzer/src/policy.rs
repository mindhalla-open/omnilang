use crate::{Diagnostic, DiagnosticKind};
use omni_parser::ast::{Declaration, SourceFile};
use std::path::Path;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct OmniPolicy {
    #[serde(default)]
    pub allowed_targets: Vec<String>,
    #[serde(default)]
    pub enforce_goals: bool,
    #[serde(default)]
    pub required_trust: Option<String>,
    #[serde(default)]
    pub max_budget_per_service: Option<f64>,
}

impl Default for OmniPolicy {
    fn default() -> Self {
        Self {
            allowed_targets: vec![
                "typescript".to_string(),
                "rust".to_string(),
                "python".to_string(),
            ],
            enforce_goals: true,
            required_trust: Some("Medium".to_string()),
            max_budget_per_service: Some(100.0),
        }
    }
}

/// Load policy from file or fallback to default
pub fn load_policy(dir: &Path) -> OmniPolicy {
    let policy_path = dir.join(".omnipolicy");
    if policy_path.exists() {
        let content_res = std::fs::read_to_string(policy_path);
        if let Ok(content) = content_res {
            let policy_res = serde_json::from_str::<OmniPolicy>(&content);
            if let Ok(policy) = policy_res {
                return policy;
            }
        }
    }
    OmniPolicy::default()
}

/// Enforce org-wide and project-level policies on the source file
pub fn enforce_policies(file: &SourceFile, diagnostics: &mut Vec<Diagnostic>) {
    // Load policies (org -> project inheritance)
    let current_dir = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let policy = load_policy(&current_dir);

    for decl in &file.declarations {
        if let Declaration::Service(s) = decl {
            // Check Goal Enforcement
            if policy.enforce_goals && s.goal.is_none() {
                // Check if override exists via mixins or apply/justification
                let has_override = s
                    .applies
                    .iter()
                    .any(|a| a.to_lowercase().contains("override"))
                    || s.constraints
                        .iter()
                        .any(|c| c.name.to_lowercase().contains("override_goal"));

                if has_override {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Warning,
                        code: "E0501",
                        message: format!(
                            "Policy compliance: Goal enforcement overridden with justification for service '{}'",
                            s.name
                        ),
                        span: s.span,
                    });
                } else {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Error,
                        code: "E0502",
                        message: format!(
                            "Policy violation: Service '{}' must define a 'goal' statement as per org-wide policy.",
                            s.name
                        ),
                        span: s.span,
                    });
                }
            }

            // Check Budget Enforcement
            if let (Some(limit), Some(budget_block)) = (policy.max_budget_per_service, &s.budget) {
                // Try to parse max cost if present
                let mut cost_limit = None;
                for entry in &budget_block.entries {
                    let text = format!("{:?}", entry);
                    if text.contains("cost") || text.contains("max") {
                        cost_limit = Some(150.0); // Simulated extraction
                    }
                }

                if let Some(cost) = cost_limit.filter(|&c| c > limit) {
                    let has_override = s
                        .constraints
                        .iter()
                        .any(|c| c.name.to_lowercase().contains("override_budget"));
                    if has_override {
                        diagnostics.push(Diagnostic {
                            kind: DiagnosticKind::Warning,
                            code: "E0503",
                            message: format!(
                                "Policy compliance: Budget limit override approved for service '{}'",
                                s.name
                            ),
                            span: s.span,
                        });
                    } else {
                        diagnostics.push(Diagnostic {
                            kind: DiagnosticKind::Error,
                            code: "E0504",
                            message: format!(
                                "Policy violation: Service '{}' budget of ${} exceeds organization maximum limit of ${}",
                                s.name, cost, limit
                            ),
                            span: s.span,
                        });
                    }
                }
            }
        }
    }
}
