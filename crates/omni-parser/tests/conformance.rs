//! Conformance and regression tests for the OmniLang frontend.
//!
//! These tests parse the canonical `examples/*.omni` corpus and snapshot the
//! resulting AST. Any change to the grammar, lexer, or parser that alters the
//! produced AST will surface here as a snapshot diff, preventing silent drift.
//!
//! Negative cases assert that malformed specs are rejected with parse errors
//! rather than silently accepted.

use std::fs;
use std::path::{Path, PathBuf};

use omni_parser::Lexer;
use omni_parser::parser::Parser;

/// Absolute path to the repository-level `examples/` directory.
///
/// Cargo runs integration tests with the crate directory as the working
/// directory, so the corpus lives two levels up.
fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .canonicalize()
        .expect("examples directory should exist")
}

/// Collects every `.omni` file in the corpus, sorted for deterministic order.
fn corpus() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(examples_dir())
        .expect("examples directory should be readable")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|e| e == "omni").unwrap_or(false))
        .collect();
    files.sort();
    files
}

/// Stable snapshot name derived from the example file stem (e.g. `checkout`).
fn snapshot_name(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .replace('.', "_")
}

#[test]
fn corpus_is_not_empty() {
    assert!(
        !corpus().is_empty(),
        "expected at least one .omni example in the corpus"
    );
}

/// Every example in the canonical corpus must lex and parse without errors,
/// and its AST is snapshotted for regression protection.
#[test]
fn corpus_parses_and_matches_snapshot() {
    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path("snapshots");
    settings.set_prepend_module_to_snapshot(false);

    for path in corpus() {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

        let (tokens, lex_errors) = Lexer::new(&source).tokenize();
        assert!(
            lex_errors.is_empty(),
            "lexer errors in {}: {lex_errors:?}",
            path.display()
        );

        let (file, parse_errors) = Parser::new(tokens).parse();
        assert!(
            parse_errors.is_empty(),
            "parse errors in {}: {parse_errors:?}",
            path.display()
        );

        let name = snapshot_name(&path);
        settings.bind(|| {
            insta::assert_yaml_snapshot!(name, file);
        });
    }
}

/// Negative cases: malformed specs must produce at least one parse error.
/// This guards against the parser silently accepting invalid input.
#[test]
fn malformed_specs_are_rejected() {
    let cases: &[(&str, &str)] = &[
        (
            "operation_missing_block",
            "module acme.bad\nservice s\n  operation\n",
        ),
        (
            "stray_garbage_at_top_level",
            "module acme.bad\n* invalid junk\ntype T = uuid\n",
        ),
        (
            "unclosed_brace_block",
            "module acme.bad\nservice s {\n  goal: \"x\"\n",
        ),
        (
            "service_without_name",
            "module acme.bad\nservice\n  goal \"missing name\"\n",
        ),
    ];

    for (name, src) in cases {
        let (tokens, lex_errors) = Lexer::new(src).tokenize();
        let (_, parse_errors) = Parser::new(tokens).parse();
        assert!(
            !lex_errors.is_empty() || !parse_errors.is_empty(),
            "expected diagnostics for malformed case `{name}`, got none"
        );
    }
}
