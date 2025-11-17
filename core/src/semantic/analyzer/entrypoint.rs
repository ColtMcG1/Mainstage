use crate::parser::{AstNode, AstType};
use crate::semantic::analyzer::SemanticAnalyzer;
use crate::reports::*;
use crate::report;

pub(crate) fn has_entrypoint_attr(node: &AstNode) -> bool {
    node.attributes.iter().any(|a| a.name == "entrypoint")
}

impl<'a> SemanticAnalyzer<'a> {
    pub(crate) fn detect_entrypoint(&self) -> Result<AstNode<'a>, ()> {
        let mut entrypoints = Vec::new();
        let mut workspace = None;
        for n in &self.ast.root().children {
            match &n.kind {
                AstType::Workspace { .. } => workspace = Some(n.clone()),
                AstType::Project { .. } | AstType::Stage { .. } if has_entrypoint_attr(n) => {
                    entrypoints.push(n.clone());
                }
                _ => {}
            }
        }
        match entrypoints.len() {
            0 => workspace.ok_or_else(|| {
                report!(Level::Critical, "No entrypoint or workspace found.".into(), Some("SemanticAnalyzer".into()), None, None);
                ()
            }),
            1 => Ok(entrypoints.remove(0)),
            _ => {
                report!(Level::Critical, "Multiple entrypoints specified.".into(), Some("SemanticAnalyzer".into()), None, None);
                Err(())
            }
        }
    }
}