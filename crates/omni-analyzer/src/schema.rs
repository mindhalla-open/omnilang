use crate::{Diagnostic, DiagnosticKind};
use omni_parser::ast::{Declaration, SchemaDecl, SourceFile};

pub fn check_schema_evolution(file: &SourceFile, diagnostics: &mut Vec<Diagnostic>) {
    // 1. Validate current constraints
    for decl in &file.declarations {
        if let Declaration::Schema(s) = decl {
            validate_schema_constraints(s, diagnostics);
        }
    }

    // 2. Load previous Spec IR from .omni-cache if it exists and compare schemas for breaking changes
    if let Ok(entries) = std::fs::read_dir(".omni-cache") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content_res = std::fs::read_to_string(&path);
                if let Ok(content) = content_res {
                    let old_ir_res = serde_json::from_str::<serde_json::Value>(&content);
                    if let Ok(old_ir) = old_ir_res {
                        detect_breaking_changes_from_old_ir(file, &old_ir, diagnostics);
                    }
                }
            }
        }
    }
}

fn detect_breaking_changes_from_old_ir(
    file: &SourceFile,
    old_ir: &serde_json::Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut old_schemas = Vec::new();
    if let Some(decls) = old_ir
        .pointer("/source_file/declarations")
        .and_then(|v| v.as_array())
    {
        for decl in decls {
            if let Some(schema) = decl.get("Schema") {
                old_schemas.push(schema);
            }
        }
    }

    for decl in &file.declarations {
        if let Declaration::Schema(new_schema) = decl {
            let old_schema_opt = old_schemas
                .iter()
                .find(|s| s.get("name").and_then(|n| n.as_str()) == Some(&new_schema.name));
            if let Some(old_schema) = old_schema_opt {
                // 1. Check if any entity was removed
                if let Some(old_entities) = old_schema.get("entities").and_then(|v| v.as_array()) {
                    for old_entity in old_entities {
                        if let Some(old_entity_name) =
                            old_entity.get("name").and_then(|n| n.as_str())
                        {
                            let still_exists = new_schema
                                .entities
                                .iter()
                                .any(|e| e.name == old_entity_name);
                            if !still_exists {
                                diagnostics.push(Diagnostic {
                                    kind: DiagnosticKind::Error,
                                    code: "E0701",
                                    message: format!(
                                        "Schema Breaking Change: Entity '{}' was removed from schema '{}'. This is a breaking change.",
                                        old_entity_name, new_schema.name
                                    ),
                                    span: new_schema.span,
                                });
                            } else {
                                // Entity still exists, check fields
                                let new_entity = new_schema
                                    .entities
                                    .iter()
                                    .find(|e| e.name == old_entity_name)
                                    .unwrap();
                                if let Some(old_fields) =
                                    old_entity.get("fields").and_then(|v| v.as_array())
                                {
                                    for old_field in old_fields {
                                        if let Some(old_field_name) =
                                            old_field.get("name").and_then(|n| n.as_str())
                                        {
                                            let new_field = new_entity
                                                .fields
                                                .iter()
                                                .find(|f| f.name == old_field_name);
                                            match new_field {
                                                None => {
                                                    diagnostics.push(Diagnostic {
                                                        kind: DiagnosticKind::Error,
                                                        code: "E0702",
                                                        message: format!(
                                                            "Schema Breaking Change: Field '{}.{}' was removed from schema '{}'. This is a breaking change.",
                                                            old_entity_name, old_field_name, new_schema.name
                                                        ),
                                                        span: new_entity.span,
                                                    });
                                                }
                                                Some(nf) => {
                                                    let old_type_name_opt = old_field
                                                        .pointer("/ty/name")
                                                        .and_then(|t| t.as_str());
                                                    if let Some(old_type_name) = old_type_name_opt
                                                        .filter(|&otn| nf.ty.name != otn)
                                                    {
                                                        diagnostics.push(Diagnostic {
                                                            kind: DiagnosticKind::Error,
                                                            code: "E0703",
                                                            message: format!(
                                                                "Schema Breaking Change: Field '{}.{}' changed type from '{}' to '{}' in schema '{}'. This is a breaking change.",
                                                                old_entity_name, old_field_name, old_type_name, nf.ty.name, new_schema.name
                                                            ),
                                                            span: nf.span,
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn validate_schema_constraints(s: &SchemaDecl, diagnostics: &mut Vec<Diagnostic>) {
    for entity in &s.entities {
        // Enforce soft_delete or row_level_security checking if defined
        let has_rls = s.constraints.iter().any(|c| c.name == "row_level_security")
            || entity
                .fields
                .iter()
                .any(|f| f.decorators.iter().any(|d| d.name == "row_level_security"));
        let has_soft_delete = s.constraints.iter().any(|c| c.name == "soft_delete")
            || entity
                .fields
                .iter()
                .any(|f| f.decorators.iter().any(|d| d.name == "soft_delete"));

        if !has_rls {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::Info,
                code: "E0704",
                message: format!(
                    "Schema Validation: Entity '{}' does not enable Row Level Security (RLS). Consider adding @row_level_security decorator.",
                    entity.name
                ),
                span: entity.span,
            });
        }

        if !has_soft_delete {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::Info,
                code: "E0705",
                message: format!(
                    "Schema Validation: Entity '{}' does not enable soft deletes. Deletes will be permanent.",
                    entity.name
                ),
                span: entity.span,
            });
        }
    }
}

/// Generate SQL/NoSQL migration scripts for PostgreSQL, MySQL, MongoDB, DynamoDB
pub fn generate_migrations(s: &SchemaDecl, db_type: &str) -> String {
    let mut script = String::new();
    match db_type.to_lowercase().as_str() {
        "postgres" | "postgresql" => {
            script.push_str(&format!("-- Migration for Schema: {}\n", s.name));
            for entity in &s.entities {
                script.push_str(&format!("CREATE TABLE IF NOT EXISTS {} (\n", entity.name));
                let mut cols = Vec::new();
                for field in &entity.fields {
                    let pg_type = match field.ty.name.as_str() {
                        "Int" => "INTEGER",
                        "Float" => "DOUBLE PRECISION",
                        "Boolean" => "BOOLEAN",
                        "DateTime" => "TIMESTAMP WITH TIME ZONE",
                        _ => "VARCHAR(255)",
                    };
                    cols.push(format!("  {} {}", field.name, pg_type));
                }
                script.push_str(&cols.join(",\n"));
                script.push_str("\n);\n\n");
            }
        }
        "mysql" => {
            script.push_str(&format!("-- Migration for Schema: {}\n", s.name));
            for entity in &s.entities {
                script.push_str(&format!("CREATE TABLE IF NOT EXISTS {} (\n", entity.name));
                let mut cols = Vec::new();
                for field in &entity.fields {
                    let mysql_type = match field.ty.name.as_str() {
                        "Int" => "INT",
                        "Float" => "DOUBLE",
                        "Boolean" => "TINYINT(1)",
                        "DateTime" => "DATETIME",
                        _ => "VARCHAR(255)",
                    };
                    cols.push(format!("  `{}` {}", field.name, mysql_type));
                }
                script.push_str(&cols.join(",\n"));
                script.push_str("\n) ENGINE=InnoDB;\n\n");
            }
        }
        "mongodb" => {
            script.push_str(&format!(
                "// MongoDB Collection Initializer for Schema: {}\n",
                s.name
            ));
            for entity in &s.entities {
                script.push_str(&format!("db.createCollection(\"{}\");\n", entity.name));
            }
        }
        _ => {
            script.push_str(&format!(
                "# DynamoDB Table Definitions for Schema: {}\n",
                s.name
            ));
            for entity in &s.entities {
                script.push_str(&format!(
                    "aws dynamodb create-table --table-name {} --attribute-definitions AttributeName=id,AttributeType=S --key-schema AttributeName=id,KeyType=HASH\n",
                    entity.name
                ));
            }
        }
    }
    script
}

#[cfg(test)]
mod tests {
    use super::*;
    use omni_parser::Span;
    use omni_parser::ast::{EntityDecl, EntityField, ModuleDecl, TypeRef};

    #[test]
    fn test_detect_breaking_changes_missing_entity() {
        let old_ir = serde_json::json!({
            "source_file": {
                "declarations": [
                    {
                        "Schema": {
                            "name": "ECommerceDB",
                            "entities": [
                                {
                                    "name": "Product",
                                    "fields": [
                                        {
                                            "name": "id",
                                            "ty": { "name": "Int", "type_args": [], "union_members": [], "intersection_members": [], "span": [0,0] },
                                            "decorators": [],
                                            "span": [0,0]
                                        }
                                    ],
                                    "span": [0,0]
                                }
                            ],
                            "constraints": [],
                            "span": [0,0]
                        }
                    }
                ]
            }
        });

        // New schema has no entities (Product removed)
        let new_schema = SchemaDecl {
            name: "ECommerceDB".to_string(),
            goal: None,
            target: None,
            entities: vec![],
            relations: vec![],
            indexes: vec![],
            constraints: vec![],
            span: Span { start: 0, end: 0 },
        };
        let file = SourceFile {
            module: ModuleDecl {
                path: vec!["ECommerceDB".to_string()],
                span: Span { start: 0, end: 0 },
            },
            imports: vec![],
            exports: vec![],
            declarations: vec![Declaration::Schema(new_schema)],
        };
        let mut diagnostics = Vec::new();
        detect_breaking_changes_from_old_ir(&file, &old_ir, &mut diagnostics);

        assert!(!diagnostics.is_empty());
        assert!(
            diagnostics[0]
                .message
                .contains("Entity 'Product' was removed")
        );
    }

    #[test]
    fn test_detect_breaking_changes_changed_field_type() {
        let old_ir = serde_json::json!({
            "source_file": {
                "declarations": [
                    {
                        "Schema": {
                            "name": "ECommerceDB",
                            "entities": [
                                {
                                    "name": "Product",
                                    "fields": [
                                        {
                                            "name": "id",
                                            "ty": { "name": "Int", "type_args": [], "union_members": [], "intersection_members": [], "span": [0,0] },
                                            "decorators": [],
                                            "span": [0,0]
                                        }
                                    ],
                                    "span": [0,0]
                                }
                            ],
                            "constraints": [],
                            "span": [0,0]
                        }
                    }
                ]
            }
        });

        // New schema changed field "id" type to "String"
        let new_schema = SchemaDecl {
            name: "ECommerceDB".to_string(),
            goal: None,
            target: None,
            entities: vec![EntityDecl {
                name: "Product".to_string(),
                fields: vec![EntityField {
                    name: "id".to_string(),
                    ty: TypeRef::simple("String".to_string(), Span { start: 0, end: 0 }),
                    default: None,
                    decorators: vec![],
                    doc_comment: None,
                    span: Span { start: 0, end: 0 },
                }],
                doc_comment: None,
                span: Span { start: 0, end: 0 },
            }],
            relations: vec![],
            indexes: vec![],
            constraints: vec![],
            span: Span { start: 0, end: 0 },
        };
        let file = SourceFile {
            module: ModuleDecl {
                path: vec!["ECommerceDB".to_string()],
                span: Span { start: 0, end: 0 },
            },
            imports: vec![],
            exports: vec![],
            declarations: vec![Declaration::Schema(new_schema)],
        };
        let mut diagnostics = Vec::new();
        detect_breaking_changes_from_old_ir(&file, &old_ir, &mut diagnostics);

        assert!(!diagnostics.is_empty());
        assert!(
            diagnostics[0]
                .message
                .contains("changed type from 'Int' to 'String'")
        );
    }
}
