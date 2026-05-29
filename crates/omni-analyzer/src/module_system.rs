//! Module system: visibility checking, import resolution, mixin expansion,
//! manifest parsing, and dependency resolution.

use omni_parser::ast::*;
use std::collections::HashMap;

use crate::{Diagnostic, DiagnosticKind};

// ---------------------------------------------------------------------------
// Visibility checking
// ---------------------------------------------------------------------------

/// Validate visibility rules: private declarations should not be exported.
pub fn check_visibility(file: &SourceFile, diagnostics: &mut Vec<Diagnostic>) {
    for decl in &file.declarations {
        let (name, vis, span) = match decl {
            Declaration::Type(t) => (&t.name, t.visibility, t.span),
            Declaration::Service(s) => (&s.name, s.visibility, s.span),
            Declaration::Mixin(m) => (&m.name, m.visibility, m.span),
            _ => continue,
        };

        if vis == Visibility::Private && file.exports.contains(name) {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::Error,
                code: "E0401",
                message: format!(
                    "cannot export private declaration '{}': remove 'private' modifier or 'export' statement",
                    name
                ),
                span,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Import resolution
// ---------------------------------------------------------------------------

/// Resolved import entry.
#[derive(Debug, Clone)]
pub struct ResolvedImport {
    pub module_path: Vec<String>,
    pub kind: ImportKind,
    pub items: Vec<String>,
}

/// Resolve import paths to module references.
/// Returns list of resolved imports and any diagnostics.
pub fn resolve_imports(
    file: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<ResolvedImport> {
    let mut resolved = Vec::new();

    for import in &file.imports {
        let items: Vec<String> = match &import.items {
            ImportItems::Wildcard => vec!["*".to_string()],
            ImportItems::Named(named) => named.iter().map(|i| i.name.clone()).collect(),
        };

        // Validate registry imports have proper format
        if let ImportKind::Registry { registry, version } = &import.kind {
            if registry.is_empty() {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::Error,
                    code: "E0402",
                    message: "registry import must specify a registry name".to_string(),
                    span: import.span,
                });
                continue;
            }
            if version.is_none() {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::Warning,
                    code: "E0403",
                    message: format!(
                        "registry import from '{}' has no version constraint — consider pinning a version",
                        registry
                    ),
                    span: import.span,
                });
            }
        }

        resolved.push(ResolvedImport {
            module_path: import.path.clone(),
            kind: import.kind.clone(),
            items,
        });
    }

    resolved
}

// ---------------------------------------------------------------------------
// Mixin expansion
// ---------------------------------------------------------------------------

/// Expand `apply: MixinName` references in services by looking up mixin declarations.
pub fn expand_mixins(file: &SourceFile, diagnostics: &mut Vec<Diagnostic>) {
    // Build mixin lookup table
    let mut mixins: HashMap<String, &MixinDecl> = HashMap::new();
    for decl in &file.declarations {
        if let Declaration::Mixin(m) = decl {
            mixins.insert(m.name.clone(), m);
        }
    }

    // Check that all `applies` references resolve to defined mixins
    for decl in &file.declarations {
        if let Declaration::Service(s) = decl {
            for mixin_name in &s.applies {
                if !mixins.contains_key(mixin_name) {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Error,
                        code: "E0404",
                        message: format!(
                            "service '{}' applies undefined mixin '{}'",
                            s.name, mixin_name
                        ),
                        span: s.span,
                    });
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Module manifest (omni.toml)
// ---------------------------------------------------------------------------

/// Parsed module manifest from `omni.toml`.
#[derive(Debug, Clone)]
pub struct ModuleManifest {
    pub package: PackageInfo,
    pub dependencies: Vec<DependencyEntry>,
}

/// The language syntax version this toolchain implements. Manifests that pin a
/// different `syntax_version` are flagged so source written against an older or
/// newer grammar is not silently misinterpreted.
pub const CURRENT_SYNTAX_VERSION: &str = "0.2";

#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub license: Option<String>,
    /// Pinned OmniLang grammar version (`package.syntax_version`). `None` means
    /// the manifest predates syntax pinning and the current version is assumed.
    pub syntax_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DependencyEntry {
    pub name: String,
    pub version: Option<String>,
    pub path: Option<String>,
    pub registry: Option<String>,
    pub git: Option<String>,
    pub tag: Option<String>,
    pub branch: Option<String>,
    pub rev: Option<String>,
}

/// Parse an `omni.toml` manifest file.
pub fn parse_manifest(content: &str) -> Result<ModuleManifest, String> {
    let table: toml::Table = content
        .parse()
        .map_err(|e: toml::de::Error| format!("invalid omni.toml: {}", e))?;

    // Parse [package]
    let pkg = table
        .get("package")
        .and_then(|v| v.as_table())
        .ok_or("missing [package] section in omni.toml")?;

    let name = pkg
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("missing package.name in omni.toml")?
        .to_string();

    let version = pkg
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or("missing package.version in omni.toml")?
        .to_string();

    let description = pkg
        .get("description")
        .and_then(|v| v.as_str())
        .map(String::from);
    let license = pkg
        .get("license")
        .and_then(|v| v.as_str())
        .map(String::from);
    let syntax_version = pkg
        .get("syntax_version")
        .and_then(|v| v.as_str())
        .map(String::from);

    let authors = pkg
        .get("authors")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    // Parse [dependencies]
    let mut dependencies = Vec::new();
    if let Some(deps) = table.get("dependencies").and_then(|v| v.as_table()) {
        for (dep_name, dep_val) in deps {
            let entry = match dep_val {
                toml::Value::String(ver) => DependencyEntry {
                    name: dep_name.clone(),
                    version: Some(ver.clone()),
                    path: None,
                    registry: None,
                    git: None,
                    tag: None,
                    branch: None,
                    rev: None,
                },
                toml::Value::Table(t) => DependencyEntry {
                    name: dep_name.clone(),
                    version: t.get("version").and_then(|v| v.as_str()).map(String::from),
                    path: t.get("path").and_then(|v| v.as_str()).map(String::from),
                    registry: t.get("registry").and_then(|v| v.as_str()).map(String::from),
                    git: t.get("git").and_then(|v| v.as_str()).map(String::from),
                    tag: t.get("tag").and_then(|v| v.as_str()).map(String::from),
                    branch: t.get("branch").and_then(|v| v.as_str()).map(String::from),
                    rev: t.get("rev").and_then(|v| v.as_str()).map(String::from),
                },
                _ => {
                    return Err(format!(
                        "invalid dependency format for '{}': expected string or table",
                        dep_name
                    ));
                }
            };
            dependencies.push(entry);
        }
    }

    Ok(ModuleManifest {
        package: PackageInfo {
            name,
            version,
            description,
            authors,
            license,
            syntax_version,
        },
        dependencies,
    })
}

// ---------------------------------------------------------------------------
// Lock file generation
// ---------------------------------------------------------------------------

/// A resolved dependency for the lock file.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LockedDependency {
    pub name: String,
    pub version: String,
    pub source: String,
    pub checksum: Option<String>,
}

/// Generate `omni.lock` content from resolved dependencies.
pub fn generate_lockfile(manifest: &ModuleManifest) -> String {
    let mut lines = Vec::new();
    lines.push("# This file is automatically generated by omni. Do not edit.".to_string());
    lines.push(format!("# omni.lock for {}", manifest.package.name));
    lines.push(String::new());

    for dep in &manifest.dependencies {
        lines.push("[[package]]".to_string());
        lines.push(format!("name = \"{}\"", dep.name));
        if let Some(ver) = &dep.version {
            lines.push(format!("version = \"{}\"", ver));
        }
        if let Some(path) = &dep.path {
            lines.push(format!("source = \"path:{}\"", path));
        } else if let Some(git) = &dep.git {
            lines.push(format!("source = \"git:{}\"", git));
            if let Some(tag) = &dep.tag {
                lines.push(format!("tag = \"{}\"", tag));
            }
            if let Some(branch) = &dep.branch {
                lines.push(format!("branch = \"{}\"", branch));
            }
            if let Some(rev) = &dep.rev {
                lines.push(format!("rev = \"{}\"", rev));
            }
        } else if let Some(registry) = &dep.registry {
            lines.push(format!("source = \"registry:{}\"", registry));
        } else {
            lines.push("source = \"registry:omnilang\"".to_string());
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Semantic version utilities
// ---------------------------------------------------------------------------

/// Parse a semantic version string.
pub fn parse_semver(version: &str) -> Option<(u32, u32, u32)> {
    let clean = version.trim_start_matches('^').trim_start_matches('~');
    let parts: Vec<&str> = clean.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let major = parts[0].parse().ok()?;
    let minor = parts[1].parse().ok()?;

    // Handle wildcard: "2.0.*" -> patch = 0
    let patch = if parts[2] == "*" {
        0
    } else {
        parts[2].parse().ok()?
    };
    Some((major, minor, patch))
}

/// Check if two version requirements might conflict.
pub fn check_version_conflict(v1: &str, v2: &str) -> bool {
    let s1 = parse_semver(v1);
    let s2 = parse_semver(v2);

    match (s1, s2) {
        (Some((maj1, _, _)), Some((maj2, _, _))) => maj1 != maj2,
        _ => false, // Can't determine conflict without valid semver
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omni_parser::Lexer;
    use omni_parser::parser::Parser;

    fn parse(input: &str) -> SourceFile {
        let (tokens, _) = Lexer::new(input).tokenize();
        let (file, errors) = Parser::new(tokens).parse();
        assert!(errors.is_empty(), "parse errors: {:?}", errors);
        file
    }

    #[test]
    fn test_mixin_parsing() {
        let file = parse(
            r#"module test
mixin Auditable {
  constraints:
    - audit_logging
  postconditions:
    - audit_trail != null
}
"#,
        );
        assert_eq!(file.declarations.len(), 1);
        if let Declaration::Mixin(m) = &file.declarations[0] {
            assert_eq!(m.name, "Auditable");
            assert_eq!(m.constraints.len(), 1);
            assert_eq!(m.postconditions.len(), 1);
            assert_eq!(m.visibility, Visibility::Public);
        } else {
            panic!("expected Mixin declaration");
        }
    }

    #[test]
    fn test_private_type() {
        let file = parse(
            r#"module test
private type InternalId = UUID
type PublicId = UUID
"#,
        );
        assert_eq!(file.declarations.len(), 2);
        if let Declaration::Type(t) = &file.declarations[0] {
            assert_eq!(t.name, "InternalId");
            assert_eq!(t.visibility, Visibility::Private);
        } else {
            panic!("expected Type");
        }
        if let Declaration::Type(t) = &file.declarations[1] {
            assert_eq!(t.name, "PublicId");
            assert_eq!(t.visibility, Visibility::Public);
        } else {
            panic!("expected Type");
        }
    }

    #[test]
    fn test_export_declarations() {
        let file = parse(
            r#"module test
export OrderService
export OrderStatus
type OrderStatus = enum { Draft, Active }
"#,
        );
        assert_eq!(file.exports, vec!["OrderService", "OrderStatus"]);
    }

    #[test]
    fn test_visibility_check_private_export_error() {
        let file = parse(
            r#"module test
export Secret
private type Secret = String
"#,
        );
        let mut diags = Vec::new();
        check_visibility(&file, &mut diags);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("cannot export private"));
    }

    #[test]
    fn test_mixin_expansion_undefined_error() {
        let file = parse(
            r#"module test
service MyService {
  goal: "test"
}
"#,
        );
        // Manually add an applies reference that doesn't exist
        let mut modified = file.clone();
        if let Declaration::Service(s) = &mut modified.declarations[0] {
            s.applies.push("NonExistentMixin".to_string());
        }
        let mut diags = Vec::new();
        expand_mixins(&modified, &mut diags);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("undefined mixin"));
    }

    #[test]
    fn test_parse_manifest() {
        let toml = r#"
[package]
name = "acme-payments"
version = "1.0.0"
description = "Payment processing specs"
authors = ["team@acme.com"]
license = "Apache-2.0"
syntax_version = "0.2"

[dependencies]
std = "^1.0"
acme-shared = { path = "../shared" }
community-auth = { registry = "omnilang", version = "^2.0" }
"#;
        let manifest = parse_manifest(toml).unwrap();
        assert_eq!(manifest.package.name, "acme-payments");
        assert_eq!(manifest.package.version, "1.0.0");
        assert_eq!(manifest.package.syntax_version.as_deref(), Some("0.2"));
        assert_eq!(manifest.dependencies.len(), 3);
        // TOML tables iterate alphabetically (BTreeMap)
        let by_name: HashMap<&str, &DependencyEntry> = manifest
            .dependencies
            .iter()
            .map(|d| (d.name.as_str(), d))
            .collect();
        assert_eq!(by_name["std"].version, Some("^1.0".to_string()));
        assert_eq!(by_name["acme-shared"].path, Some("../shared".to_string()));
        assert_eq!(
            by_name["community-auth"].registry,
            Some("omnilang".to_string())
        );
    }

    #[test]
    fn test_generate_lockfile() {
        let manifest = ModuleManifest {
            package: PackageInfo {
                name: "test-pkg".to_string(),
                version: "1.0.0".to_string(),
                description: None,
                authors: vec![],
                license: None,
                syntax_version: None,
            },
            dependencies: vec![DependencyEntry {
                name: "std".to_string(),
                version: Some("^1.0".to_string()),
                path: None,
                registry: None,
                git: None,
                tag: None,
                branch: None,
                rev: None,
            }],
        };
        let lock = generate_lockfile(&manifest);
        assert!(lock.contains("name = \"std\""));
        assert!(lock.contains("version = \"^1.0\""));
        assert!(lock.contains("source = \"registry:omnilang\""));
    }

    #[test]
    fn test_semver_parsing() {
        assert_eq!(parse_semver("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_semver("^2.0.*"), Some((2, 0, 0)));
        assert_eq!(parse_semver("~1.0.0"), Some((1, 0, 0)));
        assert_eq!(parse_semver("invalid"), None);
    }

    #[test]
    fn test_version_conflict() {
        assert!(check_version_conflict("^1.0.0", "^2.0.0")); // major mismatch
        assert!(!check_version_conflict("^1.0.0", "^1.2.0")); // compatible
    }
}
