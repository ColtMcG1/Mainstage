use super::calls;
use crate::parser::{AstNode, AstType};
use crate::report;
use crate::reports::Level;
use crate::semantic::analyzer::SemanticAnalyzer;

fn report_empty(_: &SemanticAnalyzer, parent: &AstNode, kind: &str) {
    report!(
        Level::Warning,
        format!("Empty {} body", kind).into(),
        Some("semantic.analyzer".into()),
        parent.span.clone(),
        parent.location.clone()
    );
}

pub(crate) fn analyze_node<'a>(
    an: &mut SemanticAnalyzer<'a>,
    node: &mut AstNode<'a>,
) -> Result<(), ()> {
    match &node.kind {
        AstType::Workspace { name } => {
            if node.children.is_empty() {
                report_empty(an, node, "workspace");
            }
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

        AstType::Project { .. } => {
            if node.children.is_empty() {
                report_empty(an, node, "project");
                return Ok(());
            }
            an.enter_frame();
            for c in &mut node.children {
                analyze_node(an, c)?;
            }
            an.exit_frame();
        }

        AstType::Stage { params, .. } => {
            if node.children.is_empty() {
                report_empty(an, node, "stage");
            }
            an.enter_frame();
            for p in params {
                if let AstType::Identifier { name } = &p.kind {
                    super::util::declare_param(an, name.as_ref())?;
                }
            }
            for c in &mut node.children {
                analyze_node(an, c)?;
            }
            an.exit_frame();
        }

        AstType::Task { params, .. } => {
            if node.children.is_empty() {
                report_empty(an, node, "task");
            }
            an.enter_frame();
            for p in params {
                if let AstType::Identifier { name } = &p.kind {
                    super::util::declare_param(an, name.as_ref())?;
                }
            }
            for c in &mut node.children {
                analyze_node(an, c)?;
            }
            an.exit_frame();
        }

        AstType::Block => {
            if node.children.is_empty() {
                report_empty(an, node, "block");
                return Ok(());
            }
            an.enter_frame();
            for c in &mut node.children {
                analyze_node(an, c)?;
            }
            an.exit_frame();
        }

        AstType::Forin { body, .. } => {
            // body is a Box<AstNode>
            if body.children.is_empty() {
                report_empty(an, body, "for-in");
            }
            an.enter_frame();
            analyze_node(an, Box::leak(Box::clone(body)))?;
            an.exit_frame();
        }

        AstType::Forto { body, .. } => {
            if body.children.is_empty() {
                report_empty(an, body, "for-to");
            }
            an.enter_frame();
            analyze_node(an, Box::leak(Box::clone(body)))?;
            an.exit_frame();
        }

        AstType::While { body, .. } => {
            if body.children.is_empty() {
                report_empty(an, body, "while");
            }
            an.enter_frame();
            analyze_node(an, Box::leak(Box::clone(body)))?;
            an.exit_frame();
        }

        AstType::If { body, .. } => {
            if body.children.is_empty() {
                report_empty(an, body, "if");
            }
            an.enter_frame();
            analyze_node(an, Box::leak(Box::clone(body)))?;
            an.exit_frame();
        }

        AstType::IfElse { if_body, else_body, .. } => {
            if if_body.children.is_empty() {
                report_empty(an, if_body, "if");
            }
            if else_body.children.is_empty() {
                report_empty(an, else_body, "else");
            }
            an.enter_frame();
            analyze_node(an, Box::leak(Box::clone(if_body)))?;
            analyze_node(an, Box::leak(Box::clone(else_body)))?;
            an.exit_frame();
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
