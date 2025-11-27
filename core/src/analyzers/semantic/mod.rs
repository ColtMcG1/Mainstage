use crate::ast::{AstNode, AstNodeKind};
use crate::error::{Level, MainstageErrorExt};

mod err;
mod kind;
mod stmt;
mod expr;
mod node;
mod symbol;
mod table;
mod analyzer;

pub use kind::InferredKind;

pub fn analyze_semantic_rules(ast: &mut AstNode) -> Result<(), Box<dyn MainstageErrorExt>> {
    let mut analyzer = analyzer::Analyzer::new();
    analyzer.analyze(ast)
}