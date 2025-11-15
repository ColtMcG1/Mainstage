//! ./codegen/lowering/expr.rs
//! Expression lowering helpers (produce values on the stack).

use crate::codegen::ir::*;
use crate::parser::{AstNode, AstType};

// Convert AST literal into IRConst (supports nested arrays)
pub(crate) fn value_irconst(node: &AstNode<'_>) -> Option<IRConst> {
    match &node.kind {
        AstType::Str { value } => Some(IRConst::Str(value.to_string())),
        AstType::Bool { value } => Some(IRConst::Bool(*value)),
        AstType::Number { value } => Some(IRConst::Int(*value)),
        AstType::Identifier { name } => Some(IRConst::Ident(name.to_string())),
        AstType::ShellCmd { command, .. } => Some(IRConst::Command(command.to_string())),
        AstType::Array => {
            let mut elems = Vec::with_capacity(node.children.len());
            for child in &node.children {
                elems.push(value_irconst(child)?);
            }
            Some(IRConst::Array(elems))
        }
        AstType::Null => Some(IRConst::Null),
        _ => None,
    }
}

// Load a value into the stack, resolving identifiers to variable loads using scope.
pub(crate) fn emit_value_in_scope(
    module: &mut ModuleIR,
    ops: &mut Vec<IROp>,
    scope_prefix: &str,
    scope_name: &str,
    node: &AstNode<'_>,
) -> bool {
    match &node.kind {
        // Identifier -> LoadVar of FQ global (scope:key)
        AstType::Identifier { name } => {
            let fq = format!("{scope_prefix}:{scope_name}.{name}");
            let gid = module.intern_global(fq);
            ops.push(IROp {
                kind: IROpKind::LoadVar(gid),
                span: node.span.clone(),
            });
            true
        }
        // Literals/arrays -> LoadConst as before
        _ => {
            if let Some(konst) = value_irconst(node) {
                let idx = module.intern_const(konst);
                ops.push(IROp {
                    kind: IROpKind::LoadConst(idx),
                    span: node.span.clone(),
                });
                true
            } else {
                false
            }
        }
    }
}

// Expression lowering entry point (value position)
pub(crate) fn emit_expr_value_in_scope(
    module: &mut ModuleIR,
    ops: &mut Vec<IROp>,
    scope_prefix: &str,
    scope_name: &str,
    expr: &AstNode<'_>,
) -> bool {
    match &expr.kind {
        AstType::Member { target, member } => {
            let field = if let AstType::Identifier { name } = &member.kind {
                name.as_ref()
            } else {
                return false;
            };
            let field_cidx = module.intern_const(IRConst::Str(field.to_string()));
            if let AstType::Identifier { name: obj } = &target.kind {
                let fq = format!("{scope_prefix}:{scope_name}.{obj}");
                let gid = module.intern_global(fq);
                ops.push(IROp {
                    kind: IROpKind::LoadVar(gid),
                    span: target.span.clone(),
                });
                ops.push(IROp {
                    kind: IROpKind::LoadRefMember(field_cidx),
                    span: expr.span.clone(),
                });
                return true;
            }
            if emit_expr_value_in_scope(module, ops, scope_prefix, scope_name, target) {
                ops.push(IROp {
                    kind: IROpKind::LoadRefMember(field_cidx),
                    span: expr.span.clone(),
                });
                return true;
            }
            false
        }
        AstType::Index { target, index } => {
            if !emit_expr_value_in_scope(module, ops, scope_prefix, scope_name, target) {
                return false;
            }
            if !emit_expr_value_in_scope(module, ops, scope_prefix, scope_name, index) {
                return false;
            }
            ops.push(IROp {
                kind: IROpKind::Index,
                span: expr.span.clone(),
            });
            true
        }
        AstType::BinaryOp { op, left, right } => {
            if !emit_expr_value_in_scope(module, ops, scope_prefix, scope_name, left) {
                return false;
            }
            if !emit_expr_value_in_scope(module, ops, scope_prefix, scope_name, right) {
                return false;
            }
            use crate::parser::types::BinaryOperator;
            let kind = match op {
                BinaryOperator::Add => IROpKind::Add,
                BinaryOperator::Sub => IROpKind::Sub,
                BinaryOperator::Mul => IROpKind::Mul,
                BinaryOperator::Div => IROpKind::Div,
                _ => return false,
            };
            ops.push(IROp {
                kind,
                span: expr.span.clone(),
            });
            true
        }
        AstType::Call { target, arguments } => {
            let name = if let AstType::Identifier { name } = &target.kind {
                name.as_ref()
            } else {
                return false;
            };
            if name == "say" {
                if arguments.len() != 1 {
                    return false;
                }
                let arg = &arguments[0];
                if !emit_expr_value_in_scope(module, ops, scope_prefix, scope_name, arg) {
                    return false;
                }
                ops.push(IROp {
                    kind: IROpKind::Say,
                    span: expr.span.clone(),
                });
                return true;
            }
            if name == "ask" {
                if arguments.len() > 1 {
                    return false;
                }
                let argc = arguments.len() as u8;
                if argc == 1
                    && !emit_expr_value_in_scope(
                        module,
                        ops,
                        scope_prefix,
                        scope_name,
                        &arguments[0],
                    )
                {
                    return false;
                }
                ops.push(IROp {
                    kind: IROpKind::Ask(argc),
                    span: expr.span.clone(),
                });
                return true;
            }
            if name == "read" {
                if arguments.len() != 1 {
                    return false;
                }
                if !emit_expr_value_in_scope(module, ops, scope_prefix, scope_name, &arguments[0]) {
                    return false;
                }
                ops.push(IROp {
                    kind: IROpKind::Read,
                    span: expr.span.clone(),
                });
                return true;
            }
            if name == "write" {
                if arguments.len() != 2 {
                    return false;
                }
                if !emit_expr_value_in_scope(module, ops, scope_prefix, scope_name, &arguments[0]) {
                    return false;
                }
                if !emit_expr_value_in_scope(module, ops, scope_prefix, scope_name, &arguments[1]) {
                    return false;
                }
                ops.push(IROp {
                    kind: IROpKind::Write,
                    span: expr.span.clone(),
                });
                return true;
            }
            if let Some(fid) = module.get_plain_func(name) {
                for a in arguments {
                    if !emit_expr_value_in_scope(module, ops, scope_prefix, scope_name, a) {
                        return false;
                    }
                }
                ops.push(IROp {
                    kind: IROpKind::Call(fid, arguments.len() as u8),
                    span: expr.span.clone(),
                });
                return true;
            }
            false
        }
        AstType::Identifier { name } => {
            let fq = format!("{scope_prefix}:{scope_name}.{name}");
            let gid = module.intern_global(fq);
            ops.push(IROp {
                kind: IROpKind::LoadVar(gid),
                span: expr.span.clone(),
            });
            true
        }
        _ => {
            if let Some(konst) = value_irconst(expr) {
                let idx = module.intern_const(konst);
                ops.push(IROp {
                    kind: IROpKind::LoadConst(idx),
                    span: expr.span.clone(),
                });
                true
            } else {
                false
            }
        }
    }
}
