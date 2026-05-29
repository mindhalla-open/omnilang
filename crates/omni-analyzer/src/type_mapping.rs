//! Deterministic type mapping from OmniLang types to target language types.
//!
//! Provides mapping tables for Rust and Python targets, and validates
//! that constraints are compatible with the chosen target.

use std::collections::HashMap;

/// A type mapping entry from OmniLang to a target language type.
#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct TypeMapping {
    pub omni_type: String,
    pub target_type: String,
    /// Optional import/use path required for this type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import_path: Option<String>,
}

/// Build the type mapping table for a given target language.
pub fn get_type_mappings(target: &str) -> Vec<TypeMapping> {
    match target {
        "rust" => rust_type_mappings(),
        "python" => python_type_mappings(),
        "typescript" => typescript_type_mappings(),
        _ => typescript_type_mappings(), // default
    }
}

fn rust_type_mappings() -> Vec<TypeMapping> {
    vec![
        TypeMapping {
            omni_type: "String".into(),
            target_type: "String".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "Int".into(),
            target_type: "i64".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "Float".into(),
            target_type: "f64".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "Bool".into(),
            target_type: "bool".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "UUID".into(),
            target_type: "Uuid".into(),
            import_path: Some("uuid::Uuid".into()),
        },
        TypeMapping {
            omni_type: "DateTime".into(),
            target_type: "DateTime<Utc>".into(),
            import_path: Some("chrono::{DateTime, Utc}".into()),
        },
        TypeMapping {
            omni_type: "Date".into(),
            target_type: "NaiveDate".into(),
            import_path: Some("chrono::NaiveDate".into()),
        },
        TypeMapping {
            omni_type: "Duration".into(),
            target_type: "Duration".into(),
            import_path: Some("std::time::Duration".into()),
        },
        TypeMapping {
            omni_type: "Money".into(),
            target_type: "Decimal".into(),
            import_path: Some("rust_decimal::Decimal".into()),
        },
        TypeMapping {
            omni_type: "Decimal".into(),
            target_type: "Decimal".into(),
            import_path: Some("rust_decimal::Decimal".into()),
        },
        TypeMapping {
            omni_type: "Bytes".into(),
            target_type: "Vec<u8>".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "Void".into(),
            target_type: "()".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "Email".into(),
            target_type: "String".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "URL".into(),
            target_type: "String".into(),
            import_path: None,
        },
    ]
}

fn python_type_mappings() -> Vec<TypeMapping> {
    vec![
        TypeMapping {
            omni_type: "String".into(),
            target_type: "str".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "Int".into(),
            target_type: "int".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "Float".into(),
            target_type: "float".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "Bool".into(),
            target_type: "bool".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "UUID".into(),
            target_type: "uuid.UUID".into(),
            import_path: Some("import uuid".into()),
        },
        TypeMapping {
            omni_type: "DateTime".into(),
            target_type: "datetime".into(),
            import_path: Some("from datetime import datetime".into()),
        },
        TypeMapping {
            omni_type: "Date".into(),
            target_type: "date".into(),
            import_path: Some("from datetime import date".into()),
        },
        TypeMapping {
            omni_type: "Duration".into(),
            target_type: "timedelta".into(),
            import_path: Some("from datetime import timedelta".into()),
        },
        TypeMapping {
            omni_type: "Money".into(),
            target_type: "Decimal".into(),
            import_path: Some("from decimal import Decimal".into()),
        },
        TypeMapping {
            omni_type: "Decimal".into(),
            target_type: "Decimal".into(),
            import_path: Some("from decimal import Decimal".into()),
        },
        TypeMapping {
            omni_type: "Bytes".into(),
            target_type: "bytes".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "Void".into(),
            target_type: "None".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "Email".into(),
            target_type: "str".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "URL".into(),
            target_type: "str".into(),
            import_path: None,
        },
    ]
}

fn typescript_type_mappings() -> Vec<TypeMapping> {
    vec![
        TypeMapping {
            omni_type: "String".into(),
            target_type: "string".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "Int".into(),
            target_type: "number".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "Float".into(),
            target_type: "number".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "Bool".into(),
            target_type: "boolean".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "UUID".into(),
            target_type: "string".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "DateTime".into(),
            target_type: "Date".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "Date".into(),
            target_type: "Date".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "Duration".into(),
            target_type: "number".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "Money".into(),
            target_type: "number".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "Decimal".into(),
            target_type: "number".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "Bytes".into(),
            target_type: "Buffer".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "Void".into(),
            target_type: "void".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "Email".into(),
            target_type: "string".into(),
            import_path: None,
        },
        TypeMapping {
            omni_type: "URL".into(),
            target_type: "string".into(),
            import_path: None,
        },
    ]
}

/// Build a lookup map from OmniLang type name to target type for quick resolution.
pub fn build_type_map(target: &str) -> HashMap<String, TypeMapping> {
    let mappings = get_type_mappings(target);
    let mut map = HashMap::new();
    for m in mappings {
        map.insert(m.omni_type.clone(), m.clone());
        map.insert(m.omni_type.to_lowercase(), m);
    }
    map
}

/// Constraint compatibility information for a target.
#[derive(Debug)]
pub struct ConstraintCompatibility {
    pub constraint: String,
    pub supported: bool,
    pub note: Option<String>,
}

/// Validate that constraints are compatible with the target language.
pub fn validate_target_constraints(
    constraints: &[String],
    target: &str,
) -> Vec<ConstraintCompatibility> {
    let unsupported = get_unsupported_constraints(target);
    constraints
        .iter()
        .map(|c| {
            if let Some(note) = unsupported.get(c.as_str()) {
                ConstraintCompatibility {
                    constraint: c.clone(),
                    supported: false,
                    note: Some(note.to_string()),
                }
            } else {
                ConstraintCompatibility {
                    constraint: c.clone(),
                    supported: true,
                    note: None,
                }
            }
        })
        .collect()
}

fn get_unsupported_constraints(target: &str) -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();
    match target {
        "rust" => {
            // Rust doesn't natively support some web-centric constraints without extra crates
        }
        "python" => {
            // Python has limited compile-time type safety
            map.insert(
                "latency",
                "Python's GIL makes latency guarantees unreliable for CPU-bound workloads",
            );
        }
        _ => {}
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_type_mappings() {
        let map = build_type_map("rust");
        assert_eq!(map["Int"].target_type, "i64");
        assert_eq!(map["String"].target_type, "String");
        assert_eq!(map["UUID"].target_type, "Uuid");
        assert_eq!(map["Money"].target_type, "Decimal");
        assert_eq!(map["DateTime"].target_type, "DateTime<Utc>");
        assert!(map["UUID"].import_path.is_some());
    }

    #[test]
    fn test_python_type_mappings() {
        let map = build_type_map("python");
        assert_eq!(map["Int"].target_type, "int");
        assert_eq!(map["String"].target_type, "str");
        assert_eq!(map["UUID"].target_type, "uuid.UUID");
        assert_eq!(map["Money"].target_type, "Decimal");
        assert_eq!(map["DateTime"].target_type, "datetime");
    }

    #[test]
    fn test_typescript_type_mappings() {
        let map = build_type_map("typescript");
        assert_eq!(map["Int"].target_type, "number");
        assert_eq!(map["String"].target_type, "string");
        assert_eq!(map["Bool"].target_type, "boolean");
        assert_eq!(map["Void"].target_type, "void");
    }

    #[test]
    fn test_constraint_compatibility_python() {
        let constraints = vec!["latency".to_string(), "cacheable".to_string()];
        let results = validate_target_constraints(&constraints, "python");
        assert_eq!(results.len(), 2);

        let latency = results.iter().find(|r| r.constraint == "latency").unwrap();
        assert!(!latency.supported);
        assert!(latency.note.is_some());

        let cacheable = results
            .iter()
            .find(|r| r.constraint == "cacheable")
            .unwrap();
        assert!(cacheable.supported);
    }

    #[test]
    fn test_constraint_compatibility_rust() {
        let constraints = vec!["latency".to_string(), "cacheable".to_string()];
        let results = validate_target_constraints(&constraints, "rust");
        assert!(results.iter().all(|r| r.supported));
    }
}
