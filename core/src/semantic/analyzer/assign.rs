use crate::parser::{AstNode, AstType};
use crate::semantic::analyzer::SemanticAnalyzer;
use crate::semantic::inference::infer_type;
use crate::semantic::types::SymbolType;
use crate::semantic::{Symbol, SymbolKind, SymbolScope};
use crate::report;

pub(crate) fn handle_assignment<'a>(
    an: &mut SemanticAnalyzer<'a>,
    node: &AstNode<'a>,
) -> Result<(), ()> {
    if node.children.len() < 2 {
        return Ok(());
    }

    let (lhs, rhs) = (&node.children[0], &node.children[1]);
    
    let name = match &lhs.kind {
        AstType::Identifier { name } => name.as_ref(),
        _ => return Ok(()),
    };

    if let AstType::Assignment { op } = &node.kind {
        if matches!(
            op,
            crate::parser::AssignOperator::Add | crate::parser::AssignOperator::Sub | crate::parser::AssignOperator::Mul | crate::parser::AssignOperator::Div
        ) && !an.symbol_table.exists(name)
        {
            report!(
                crate::reports::Level::Warning,
                format!(
                    "Compound assignment to undeclared '{}'; implicitly creating.",
                    name
                ),
                Some("SemanticAnalyzer".into()),
                lhs.span.clone(),
                lhs.location.clone()
            );
        }
    }

    // Always analyze the RHS first (type-check, nested diagnostics).
    an.analyze_expression(rhs)?;

    // If RHS is an array literal, treat any identifiers inside as references.
    if matches!(rhs.kind, AstType::Array) {
        mark_identifier_uses_in_expr(an, rhs);
    }

    let ty = infer_type(an, rhs).unwrap_or(SymbolType::None);

    // Reserved member names (workspace/project/stage/task fields) are not declared as new variables.
    let reserved_sets = [
        &crate::reserved::RESERVED_WORKSPACE_MEMBERS,
        &crate::reserved::RESERVED_PROJECT_MEMBERS,
        &crate::reserved::RESERVED_STAGE_MEMBERS,
        &crate::reserved::RESERVED_TASK_MEMBERS,
    ];
    let is_reserved = reserved_sets.iter().any(|s| s.contains(&name));

    if is_reserved {
        // Optionally: if you track types for reserved members, update here.
        return Ok(());
    }

    // Regular variable assignment at the current (top-level) scope.
    if an.symbol_table.exists(name) {
        if let Some(vec) = an.symbol_table.get_mut(name) {
            for s in vec {
                if s.kind() == &SymbolKind::Variable {
                    if s.symbol_type() == &SymbolType::None {
                        s.set_symbol_type(ty.clone());
                    }
                    s.increment_reference_count();
                }
            }
        }
    } else {
        let sym = Symbol::new_variable(name.to_string().into(), ty, SymbolScope::Global);
        let _ = an.symbol_table.insert(sym);
        if let Some(vec) = an.symbol_table.get_mut(name) {
            for s in vec {
                if s.kind() == &SymbolKind::Variable {
                    s.increment_reference_count();
                }
            }
        }
    }

    Ok(())
}

// Recursively mark identifier occurrences as references within an expression.
fn mark_identifier_uses_in_expr<'a>(an: &mut SemanticAnalyzer<'a>, node: &AstNode<'a>) {
    match &node.kind {
        AstType::Identifier { name } => {
            if let Some(vec) = an.symbol_table.get_mut(name.as_ref()) {
                for s in vec {
                    s.increment_reference_count();
                }
            }
        }
        _ => {
            for c in &node.children {
                mark_identifier_uses_in_expr(an, c);
            }
        }
    }
}
