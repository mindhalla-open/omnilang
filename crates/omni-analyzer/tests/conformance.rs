//! Analyzer conformance tests.
//!
//! Runs the full analysis pipeline over the canonical `examples/*.omni` corpus
//! and snapshots both the diagnostics and the validated Spec IR. This freezes
//! the analyzer's observable output: any change to name resolution, type
//! checking, constraint validation, or IR construction surfaces as a diff.

use std::fs;
use std::path::{Path, PathBuf};

use omni_analyzer::{Diagnostic, analyze};
use omni_parser::Lexer;
use omni_parser::parser::Parser;
use serde::Serialize;

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .canonicalize()
        .expect("examples directory should exist")
}

fn corpus() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(examples_dir())
        .expect("examples directory should be readable")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|e| e == "omni").unwrap_or(false))
        .collect();
    files.sort();
    files
}

fn snapshot_name(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .replace('.', "_")
}

/// Serializable, stable view of analyzer output for snapshotting. `Diagnostic`
/// carries source spans that are noisy across edits, so we record only the
/// severity and message, plus a compact summary of the produced IR.
#[derive(Serialize)]
struct AnalysisReport {
    diagnostics: Vec<String>,
    ir: Option<IrSummary>,
}

#[derive(Serialize)]
struct IrSummary {
    module_path: String,
    types: Vec<String>,
    services: Vec<String>,
    build_order: Vec<String>,
}

fn render_diagnostics(diags: &[Diagnostic]) -> Vec<String> {
    diags.iter().map(|d| d.to_string()).collect()
}

#[test]
fn corpus_analysis_matches_snapshot() {
    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path("snapshots");
    settings.set_prepend_module_to_snapshot(false);

    for path in corpus() {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

        let (tokens, _) = Lexer::new(&source).tokenize();
        let (file, _) = Parser::new(tokens).parse();
        let (ir, diagnostics) = analyze(&file);

        let report = AnalysisReport {
            diagnostics: render_diagnostics(&diagnostics),
            ir: ir.map(|ir| IrSummary {
                module_path: ir.module_path.join("."),
                types: ir.types.iter().map(|t| t.name.clone()).collect(),
                services: ir.services.iter().map(|s| s.name.clone()).collect(),
                build_order: ir.build_order.clone(),
            }),
        };

        let name = snapshot_name(&path);
        settings.bind(|| {
            insta::assert_yaml_snapshot!(name, report);
        });
    }
}
