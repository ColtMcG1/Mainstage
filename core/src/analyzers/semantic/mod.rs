use crate::ast::{AstNode};
use crate::error::{MainstageErrorExt};

mod err;
mod kind;
mod stmt;
mod expr;
mod node;
mod symbol;
mod table;
mod analyzer;

pub use kind::InferredKind;

pub fn analyze_semantic_rules(ast: &mut AstNode) -> Result<(), Vec<Box<dyn MainstageErrorExt>>> {
    let mut analyzer = analyzer::Analyzer::new();
    analyzer.analyze(ast).ok();
    let diagnostics = analyzer.take_diagnostics();
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    Ok(())
}