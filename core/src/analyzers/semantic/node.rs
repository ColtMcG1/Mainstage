use crate::ast::{AstNode, AstNodeKind};
use super::table::SymbolTable;

pub(crate) fn analyze_node(
     node: &mut crate::ast::AstNode,
     tbl: &mut crate::analyzers::semantic::table::SymbolTable,
 ) -> Result<(), Box<dyn crate::error::MainstageErrorExt>> {
     match &mut node.kind {
        //AstNodeKind::Statement { .. } => super::stmt::analyze_statement(node, tbl)?,
        //AstNodeKind::Assignment { .. } => super::expr::analyze_expression(node, tbl)?,
        //AstNodeKind::Block { .. } => super::expr::analyze_block(node, tbl)?,
         _ => {
            return Err(Box::new(
                 crate::analyzers::semantic::err::SemanticError::with(
                     crate::error::Level::Error,
                     format!("Unsupported node kind for analyze_node: {}", node.kind),
                     "mainstage.analyzers.semantic.node.analyze_node".to_string(),
                     node.location.clone(),
                     node.span.clone(),
                 ),
             ));
         }
     }
     Ok(())
 }