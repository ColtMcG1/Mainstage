use crate::ast::AstNode;
use crate::error::{MainstageErrorExt};

mod err;
mod kind;
mod stmt;
mod expr;
mod node;
mod symbol;
pub mod table;
mod analyzer;

pub use kind::InferredKind;

pub fn analyze_semantic_rules(ast: &mut AstNode) -> Result<String, Vec<Box<dyn MainstageErrorExt>>> {
    let mut analyzer = analyzer::Analyzer::new();

    // Run the analyzer and propagate any fatal analysis error immediately.
    if let Err(e) = analyzer.analyze(ast) {
        return Err(vec![e]);
    }

    // Collect diagnostics (warnings/infos) produced by the analyzer. If any
    // diagnostics were collected, return them as the error path so callers can
    // display or handle them.
    let diagnostics = analyzer.take_diagnostics();
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    // The symbol table holds the chosen entrypoint workspace name (if any).
    // Return it to the caller as a String. If no workspace was found, return
    // a diagnostic error.
    let entrypoint = analyzer.get_symbol_table().entrypoint();
    if let Some(node) = entrypoint {
        Ok(node)
    } else {
        Err(vec![Box::new(
            err::SemanticError::with(
                crate::error::Level::Error,
                "No entrypoint workspace found in script.".to_string(),
                "mainstage.analyzers.semantic.analyze_semantic_rules".to_string(),
                ast.location.clone(),
                ast.span.clone(),
            ),
        )])
    }
}