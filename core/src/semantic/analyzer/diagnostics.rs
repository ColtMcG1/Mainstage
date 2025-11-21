use crate::parser::{AstNode, AstType};
use crate::report;
use crate::reports::Level;

fn is_executable_stmt(n: &AstNode) -> bool {
    match &n.kind {
        AstType::Call { .. } | AstType::Assignment { .. } | AstType::Return
        | AstType::While { .. } | AstType::Forin { .. } | AstType::Forto { .. } => true,
        AstType::Block => n.children.iter().any(is_executable_stmt),
        _ => false,
    }
}

pub fn warn_empty_bodies(root: &AstNode) {
    fn walk(n: &AstNode) {
        match &n.kind {
            AstType::Workspace { name }
            | AstType::Project { name }
            | AstType::Stage { name, .. }
            | AstType::Task { name, .. } => {
                let has_exec = n.children.iter().any(is_executable_stmt);
                if !has_exec {
                    report!(
                        Level::Warning,
                        format!("'{}' has an empty body.", name.as_ref()),
                        Some("SemanticAnalyzer".into()),
                        n.span.clone(),
                        n.location.clone()
                    );
                }
            }
            _ => {}
        }
        for c in &n.children { walk(c); }
    }
    walk(root);
}