//! ./semantic/inference.rs
//!
//! Type inference helpers for the semantic analyzer.

use crate::parser::{AstNode, AstType};
use crate::parser::types::BinaryOperator;
use crate::report;
use crate::reports::*;
use super::analyzer::SemanticAnalyzer;
use super::types::{InferredType, SymbolType};
use crate::semantic::SymbolKind;
use crate::builtin::BUILTIN_FUNCS;

pub fn unify(a: InferredType, b: InferredType) -> InferredType {
    use InferredType::*;
    match (a, b) {
        (Unknown, t) | (t, Unknown) => t,
        (Unit, t) | (t, Unit) => t,
        (Int, Int) => Int,
        (Float, Float) => Float,
        (Int, Float) | (Float, Int) => Float,
        (Bool, Bool) => Bool,
        (Str, Str) => Str,
        (Array, Array) => Array,
        _ => Unknown,
    }
}

/// Infer the type of a call expression based on builtins, stages, and tasks.
pub fn infer_call_expr_type(an: &SemanticAnalyzer<'_>, node: &AstNode<'_>) -> Option<InferredType> {
    let (target, _args) = match &node.kind {
        AstType::Call { target, arguments } => (target, arguments),
        _ => return None,
    };

    // Simple identifiers (existing behavior)
    if let AstType::Identifier { name } = &target.kind {
        let name = name.as_ref();

        // Check if function name is builtin for type
         if let Some(def) = BUILTIN_FUNCS.get(name) {
            return Some(def.returns);
        }

        // Is the call to a stage?
        if an.is_stage_name(name) {
            return Some(InferredType::Unit);
        }

        // Is the call to a task
        if an.is_task_name(name) {
            return an.task_returns.get(name).copied().or(Some(InferredType::Unknown));
        }

        // Unknown identifier
        return None;
    }

    // Member calls: stage.task(...)
    if let AstType::Member { target: t, member: m } = &target.kind {
        if let (AstType::Identifier { name: stage_name }, AstType::Identifier { name: task_name }) = (&t.kind, &m.kind) {
            if an.is_stage_name(stage_name) && an.is_task_name(task_name) {
                return an.task_returns.get(task_name.as_ref()).copied().or(Some(InferredType::Unknown));
            }
        }
    }

    None
}

pub fn infer_expr_type(an: &SemanticAnalyzer<'_>, node: &AstNode<'_>) -> InferredType {
    match &node.kind {
        AstType::Integer { .. } => InferredType::Int,
        AstType::Float { .. } => InferredType::Float,
        AstType::Bool { .. } => InferredType::Bool,
        AstType::Str { .. } => InferredType::Str,
        AstType::Array => InferredType::Array,
        AstType::Null => InferredType::Unit,
        AstType::Call { .. } => infer_call_expr_type(an, node).unwrap_or(InferredType::Unknown),
        AstType::Identifier { name } => {
            if let Some(syms) = an.symbol_table.get(name) {
                match syms[0].symbol_type() {
                    SymbolType::Integer => InferredType::Int,
                    SymbolType::Boolean => InferredType::Bool,
                    SymbolType::String => InferredType::Str,
                    SymbolType::Array => InferredType::Array,
                    SymbolType::None => InferredType::Unit,
                    _ => InferredType::Unknown,
                }
            } else {
                InferredType::Unknown
            }
        }
        AstType::Index { target, index: _ } => {
            match &target.kind {
                AstType::Array => {
                    let mut el = InferredType::Unknown;
                    for c in &target.children {
                        let t = infer_expr_type(an, c);
                        el = match el {
                            InferredType::Unknown => t,
                            prev => unify(prev, t),
                        };
                    }
                    if el == InferredType::Unknown { InferredType::Int } else { el }
                }
                AstType::Identifier { name } => {
                    if let Some(syms) = an.symbol_table.get(name) {
                        if syms[0].symbol_type() == &SymbolType::Array {
                            InferredType::Int
                        } else {
                            InferredType::Unknown
                        }
                    } else {
                        InferredType::Unknown
                    }
                }
                _ => InferredType::Unknown,
            }
        }
        AstType::BinaryOp { op, left, right } => {
            let lt = infer_expr_type(an, left);
            let rt = infer_expr_type(an, right);
            match op {
                BinaryOperator::Add | BinaryOperator::Sub | BinaryOperator::Mul | BinaryOperator::Div => {
                    if (lt == InferredType::Int || lt == InferredType::Unknown)
                        && (rt == InferredType::Int || rt == InferredType::Unknown)
                    {
                        InferredType::Int
                    } else {
                        InferredType::Unknown
                    }
                }
                BinaryOperator::Eq
                | BinaryOperator::Ne
                | BinaryOperator::Lt
                | BinaryOperator::Gt
                | BinaryOperator::Le
                | BinaryOperator::Ge => InferredType::Bool,
            }
        }
        AstType::Member { target, member } => {
            let (container_name, member_name) = match (&target.kind, &member.kind) {
                (AstType::Identifier { name: c }, AstType::Identifier { name: m }) => (c.as_ref(), m.as_ref()),
                _ => return InferredType::Unknown,
            };
            // Resolve kind generically
            let kind = if an.is_stage_name(container_name) {
                SymbolKind::Stage
            } else if let Some(_) = an.symbol_table.get(container_name).filter(|v| v.iter().any(|s| s.kind() == &SymbolKind::Workspace)) {
                SymbolKind::Workspace
            } else if let Some(_) = an.symbol_table.get(container_name).filter(|v| v.iter().any(|s| s.kind() == &SymbolKind::Project)) {
                SymbolKind::Project
            } else if let Some(_) = an.symbol_table.get(container_name).filter(|v| v.iter().any(|s| s.kind() == &SymbolKind::Task)) {
                SymbolKind::Task
            } else {
                return InferredType::Unknown;
            };
            let key = (kind, container_name.to_string());
            if let Some(map) = an.scope_members.get(&key) {
                if let Some(t) = map.get(member_name) {
                    return *t;
                }
            }
            InferredType::Unknown
        }
        _ => InferredType::Unknown,
    }
}

pub fn infer_type(an: &SemanticAnalyzer<'_>, node: &AstNode<'_>) -> Result<SymbolType, ()> {
    match &node.kind {
        AstType::Call { .. } => {
            let it = infer_call_expr_type(an, node).unwrap_or(InferredType::Unknown);
            match it {
                InferredType::Unknown | InferredType::Unit => Ok(SymbolType::None),
                _ => Ok(it.to_symbol_type()),
            }
        }
        AstType::Return => {
            if let Some(expr) = node.children.get(0) {
                infer_type(an, expr)
            } else {
                Ok(SymbolType::None)
            }
        }
        AstType::Integer { .. } => Ok(SymbolType::Integer),
        AstType::Float { .. } => Ok(SymbolType::Float),
        AstType::Bool { .. } => Ok(SymbolType::Boolean),
        AstType::Str { .. } => Ok(SymbolType::String),
        AstType::Array => {
            if node.children.is_empty() {
                Ok(SymbolType::Array)
            } else {
                let first = infer_type(an, &node.children[0])?;
                for e in node.children.iter().skip(1) {
                    let t = infer_type(an, e)?;
                    if t != first {
                        report!(
                            Level::Error,
                            "Array elements must have the same type.".into(),
                            Some("SemanticAnalyzer".into()),
                            node.span.clone(),
                            node.location.clone()
                        );
                        return Err(());
                    }
                }
                Ok(SymbolType::Array)
            }
        }
        AstType::Index { target, index } => {
            if infer_type(an, index)? != SymbolType::Integer {
                report!(
                    Level::Error,
                    "Index expression must be a number.".into(),
                    Some("SemanticAnalyzer".into()),
                    index.span.clone(),
                    index.location.clone()
                );
                // continue best-effort
            }
            match &target.kind {
                AstType::Array => {
                    if target.children.is_empty() {
                        Ok(SymbolType::Integer)
                    } else {
                        infer_type(an, &target.children[0]).or(Ok(SymbolType::Integer))
                    }
                }
                AstType::Identifier { name } => {
                    if let Some(syms) = an.symbol_table.get(name) {
                        if syms[0].symbol_type() == &SymbolType::Array {
                            Ok(SymbolType::Integer)
                        } else {
                            // Target not array
                            Ok(SymbolType::None)
                        }
                    } else {
                        Ok(SymbolType::None)
                    }
                }
                _ => {
                    let tt = infer_type(an, target)?;
                    if tt == SymbolType::Array {
                        Ok(SymbolType::Integer)
                    } else {
                        report!(
                            Level::Error,
                            "Target of indexing is not an array.".into(),
                            Some("SemanticAnalyzer".into()),
                            target.span.clone(),
                            target.location.clone()
                        );
                        Ok(SymbolType::None)
                    }
                }
            }
        }
        AstType::BinaryOp { op, left, right } => {
            let lt = infer_type(an, left)?;
            let rt = infer_type(an, right)?;
            match op {
                BinaryOperator::Add | BinaryOperator::Sub | BinaryOperator::Mul | BinaryOperator::Div => {
                    if lt == SymbolType::Integer && rt == SymbolType::Integer {
                        Ok(SymbolType::Integer)
                    } else {
                        // numeric operator type mismatch
                        Ok(SymbolType::None)
                    }
                }
                BinaryOperator::Eq
                | BinaryOperator::Ne
                | BinaryOperator::Lt
                | BinaryOperator::Gt
                | BinaryOperator::Le
                | BinaryOperator::Ge => Ok(SymbolType::Boolean),
            }
        }
        AstType::Identifier { name } => {
            if let Some(syms) = an.symbol_table.get(name) {
                Ok(syms[0].symbol_type().clone())
            } else {
                report!(
                    Level::Error,
                    format!("Undefined identifier: {}", name),
                    Some("SemanticAnalyzer".into()),
                    node.span.clone(),
                    node.location.clone()
                );
                Err(())
            }
        }
        AstType::Member { target: _, member: _ } => {
            // Map generic member inference result to SymbolType without emitting identifier errors.
            let it = infer_expr_type(an, node);
            Ok(match it {
                InferredType::Unknown | InferredType::Unit => SymbolType::None,
                _ => it.to_symbol_type(),
            })
        }
        AstType::Null => Ok(SymbolType::None),
        _ => {
            report!(
                Level::Error,
                format!("Unable to infer type for node: {:?}", node.kind),
                Some("SemanticAnalyzer".into()),
                node.span.clone(),
                node.location.clone()
            );
            Err(())
        }
    }
}