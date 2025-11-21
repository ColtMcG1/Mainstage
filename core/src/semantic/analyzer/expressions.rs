use super::{calls};
use crate::parser::{AstNode, AstType};
use crate::semantic::analyzer::SemanticAnalyzer;

pub(crate) fn analyze_node<'a>(
    an: &mut SemanticAnalyzer<'a>,
    node: &mut AstNode<'a>,
) -> Result<(), ()> {
    match &node.kind {
        AstType::Workspace { name } => {
            if let Some(vec) = an.symbol_table.get_mut(name.as_ref()) {
                for s in vec {
                    s.increment_reference_count();
                }
            }
            an.enter_frame();
            for c in &mut node.children {
                analyze_node(an, c)?;
            }
            an.exit_frame();
        }

        AstType::Project { .. } | AstType::Stage { .. } | AstType::Task { .. } | AstType::Block => {
            an.enter_frame();
            for c in &mut node.children {
                analyze_node(an, c)?;
            }
            an.exit_frame();
        }
        AstType::Include { .. } | AstType::Import { .. } => {
            for c in &mut node.children {
                analyze_node(an, c)?;
            }
        }
        AstType::Assignment { .. } => super::assign::handle_assignment(an, node)?,
        AstType::Call { .. } => calls::analyze_call(an, node, false)?,
        _ => an.analyze_expression(node)?,
    }
    Ok(())
}

impl<'a> SemanticAnalyzer<'a> {
    pub(crate) fn analyze_expression(&mut self, node: &AstNode<'a>) -> Result<(), ()> {
        use crate::parser::types::AstType::*;
        match &node.kind {
            Call { .. } => calls::analyze_call(self, node, true),

            Identifier { name } => super::util::handle_identifier(self, node, name),

            Member { target, member } => {
                // Analyze target; validate member generically
                self.analyze_expression(target)?;
                match &member.kind {
                    Identifier { .. } => super::util::handle_scoped_member(self, target, member),
                    _ => self.analyze_expression(member),
                }
            }

            Array => {
                for c in &node.children {
                    self.analyze_expression(c)?;
                }
                Ok(())
            }
            Index { target, index } => {
                self.analyze_expression(target)?;
                self.analyze_expression(index)?;
                Ok(())
            }
            BinaryOp { left, right, .. } => {
                self.analyze_expression(left)?;
                self.analyze_expression(right)?;
                Ok(())
            }
            Return => {
                if let Some(e) = node.children.get(0) {
                    self.analyze_expression(e)?;
                }
                Ok(())
            }

            Str { .. } | Bool { .. } | Integer { .. } | Float { .. } | Null | ShellCmd { .. } => {
                Ok(())
            }

            _ => Ok(()),
        }
    }
}
