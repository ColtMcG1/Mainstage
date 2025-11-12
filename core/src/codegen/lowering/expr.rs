//! ./codegen/lowering/expr.rs
//! Expression lowering helpers (produce values on the stack).

use crate::codegen::ir::*;
use crate::parser::{AstNode, AstType};

fn flatten_member_access<'a>(node: &AstNode<'a>) -> Option<Vec<String>> {
    match &node.kind {
        AstType::Identifier { name } => Some(vec![name.to_string()]),
        AstType::MemberAccess { target, member } => {
            let mut v = flatten_member_access(target)?;
            if let AstType::Identifier { name } = &member.kind {
                v.push(name.to_string());
                Some(v)
            } else {
                None
            }
        }
        _ => None,
    }
}

// Convert AST literal into IRConst (supports nested arrays)
pub(crate) fn value_irconst(node: &AstNode<'_>) -> Option<IRConst> {
    match &node.kind {
        AstType::String { value } => Some(IRConst::Str(value.to_string())),
        AstType::Boolean { value } => Some(IRConst::Bool(*value)),
        AstType::Number { value } => Some(IRConst::Int(*value as i64)),
        AstType::Identifier { name } => Some(IRConst::Ident(name.to_string())),
        AstType::ShellCommand { command, .. } => Some(IRConst::Command(command.to_string())),
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

// Resolve which scope an object name belongs to by inspecting function index
fn resolve_object_scope<'a>(module: &ModuleIR, object: &str) -> Option<&'static str> {
    if module.func_index.contains_key(&format!("stage:init:{object}")) {
        Some("stage")
    } else if module.func_index.contains_key(&format!("project:init:{object}")) {
        Some("project")
    } else if module.func_index.contains_key(&format!("workspace:init:{object}")) {
        Some("workspace")
    } else if module.func_index.contains_key(&format!("task:init:{object}")) {
        Some("task")
    } else {
        None
    }
}

// Build fully-qualified global key for object.member using resolved scope
fn fq_for_member(module: &ModuleIR, object: &str, field: &str) -> Option<String> {
    resolve_object_scope(module, object).map(|scope| format!("{scope}:{object}.{field}"))
}

// Load a member access value using resolved scope (e.g., "project:core_lib.name")
pub(crate) fn emit_member_access_value(
    module: &mut ModuleIR,
    ops: &mut Vec<IROp>,
    scope_prefix: &str,
    scope_name: &str,
    node: &AstNode<'_>,
) -> bool {
    if let AstType::MemberAccess { target, member } = &node.kind {
        let field = if let AstType::Identifier { name } = &member.kind {
            name.as_ref()
        } else {
            return false;
        };
        let field_cidx = module.intern_const(IRConst::Str(field.to_string()));

        // call().member (static object)
        if let AstType::CallExpression { target, arguments } = &target.kind {
            if let AstType::Identifier { name: base } = &target.kind {
                if let Some(fid) = module.get_plain_func(base.as_ref()) {
                    for a in arguments {
                        let _ = emit_expr_value_in_scope(module, ops, "stage", base, a);
                    }
                    ops.push(IROp {
                        kind: IROpKind::Call(fid, arguments.len() as u8),
                        span: target.span.clone(),
                    });
                }
                if let Some(fq) = fq_for_member(module, base, field) {
                    let gid = module.intern_global(fq);
                    ops.push(IROp {
                        kind: IROpKind::LoadVar(gid),
                        span: node.span.clone(),
                    });
                    return true;
                }
            }
            return false;
        }

        // Static identifier base
        if let AstType::Identifier { name: obj } = &target.kind {
            if let Some(fq) = fq_for_member(module, obj, field) {
                let gid = module.intern_global(fq);
                ops.push(IROp {
                    kind: IROpKind::LoadVar(gid),
                    span: node.span.clone(),
                });
                return true;
            }
            // Dynamic: treat identifier as variable holding object name
            let var_fq = format!("{scope_prefix}:{scope_name}.{obj}");
            let gid = module.intern_global(var_fq);
            ops.push(IROp {
                kind: IROpKind::LoadVar(gid),
                span: target.span.clone(),
            });
            ops.push(IROp {
                kind: IROpKind::LoadMemberDyn(field_cidx),
                span: node.span.clone(),
            });
            return true;
        }

        // Nested flatten attempt
        if let Some(parts) = flatten_member_access(target) {
            let object = parts.last().unwrap();
            if let Some(fq) = fq_for_member(module, object, field) {
                let gid = module.intern_global(fq);
                ops.push(IROp {
                    kind: IROpKind::LoadVar(gid),
                    span: node.span.clone(),
                });
                return true;
            }
        }

        // Generic dynamic fallback: evaluate target as value then dynamic member
        if emit_expr_value_in_scope(module, ops, scope_prefix, scope_name, target) {
            ops.push(IROp {
                kind: IROpKind::LoadMemberDyn(field_cidx),
                span: node.span.clone(),
            });
            return true;
        }
    }
    false
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
        AstType::MemberAccess { .. } => {
            return emit_member_access_value(module, ops, scope_prefix, scope_name, expr);
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
        AstType::CallExpression { target, arguments } => {
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