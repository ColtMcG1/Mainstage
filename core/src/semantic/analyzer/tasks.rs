use crate::parser::{AstNode, AstType};
use crate::semantic::analyzer::SemanticAnalyzer;
use crate::semantic::types::InferredType;
use crate::semantic::inference::{infer_expr_type, unify};

pub(crate) fn infer_task_returns(an: &mut SemanticAnalyzer<'_>) {
    fn visit(an: &mut SemanticAnalyzer, node: &AstNode) {
        match &node.kind {
            AstType::Task { name, .. } => {
                let ty = infer_return_type_in_task(an, node).unwrap_or(InferredType::Unit);
                an.task_returns.insert(name.to_string(), ty);
            }
            _ => for c in &node.children { visit(an, c) }
        }
    }
    let root = an.ast.root().clone();
    visit(an, &root);
}

fn infer_return_type_in_task(an: &SemanticAnalyzer<'_>, task_node: &AstNode<'_>) -> Option<InferredType> {
    let mut acc = None;
    walk_returns(task_node, &mut |expr| {
        let t = infer_expr_type(an, expr);
        acc = Some(match acc { None => t, Some(prev) => unify(prev, t) });
    });
    acc
}

fn walk_returns<F: FnMut(&AstNode<'_>)>(node: &AstNode<'_>, f: &mut F) {
    match &node.kind {
        AstType::Return => {
            if let Some(expr) = node.children.get(0) { f(expr); }
        }
        _ => for c in &node.children { walk_returns(c, f); }
    }
}