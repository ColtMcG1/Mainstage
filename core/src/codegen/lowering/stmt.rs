//! ./codegen/lowering/stmt.rs
//! Statement lowering: assignments, calls-as-statements, KV emission.

use super::expr::{emit_expr_value_in_scope, emit_value_in_scope, value_irconst};
use crate::codegen::ir::*;
use crate::parser::{AstNode, AstType};

pub(crate) fn emit_store_global_from_value(
    module: &mut ModuleIR,
    ops: &mut Vec<IROp>,
    global_id: u32,
    value_node: &AstNode<'_>,
) -> bool {
    if let Some(konst) = value_irconst(value_node) {
        let idx = module.intern_const(konst);
        ops.push(IROp {
            kind: IROpKind::LoadConst(idx),
            span: value_node.span.clone(),
        });
        ops.push(IROp {
            kind: IROpKind::StoreGlobal(global_id),
            span: value_node.span.clone(),
        });
        true
    } else {
        false
    }
}

// Store: if RHS is identifier, copy its value via LoadVar then StoreVar
pub(crate) fn emit_store_from_value_in_scope(
    module: &mut ModuleIR,
    ops: &mut Vec<IROp>,
    scope_prefix: &str,
    scope_name: &str,
    dest_gid: u32,
    rhs: &AstNode<'_>,
) -> bool {
    if emit_value_in_scope(module, ops, scope_prefix, scope_name, rhs) {
        ops.push(IROp {
            kind: IROpKind::StoreVar(dest_gid),
            span: rhs.span.clone(),
        });
        true
    } else {
        false
    }
}

pub(crate) fn emit_call_stmt(
    module: &mut ModuleIR,
    ops: &mut Vec<IROp>,
    scope_prefix: &str,
    scope_name: &str,
    stmt: &AstNode<'_>,
) -> bool {
    if let AstType::Call { target, .. } = &stmt.kind {
        if !emit_expr_value_in_scope(module, ops, scope_prefix, scope_name, stmt) {
            return false;
        }
        let mut need_pop = true;
        if let AstType::Identifier { name } = &target.kind {
            let n = name.to_string();
            if n == "say" || n == "write" {
                need_pop = false;
            }
        }
        if need_pop {
            ops.push(IROp {
                kind: IROpKind::Pop,
                span: stmt.span.clone(),
            });
        }
        return true;
    }
    false
}

// Lower call in value position (assignment RHS, return expr)
pub(crate) fn emit_call_value(
    module: &mut ModuleIR,
    ops: &mut Vec<IROp>,
    scope_prefix: &str,
    scope_name: &str,
    expr: &AstNode<'_>,
) -> bool {
    emit_expr_value_in_scope(module, ops, scope_prefix, scope_name, expr)
}

fn extract_identifier(node: &AstNode<'_>) -> Option<String> {
    match &node.kind {
        AstType::Identifier { name } => Some(name.to_string()),
        _ => None,
    }
}

pub(crate) fn emit_kv_ops(
    scope_prefix: &str,
    scope_name: &str,
    node: &AstNode<'_>,
    order: &[usize],
    module: &mut ModuleIR,
    ops: &mut Vec<IROp>,
) {
    for &idx in order {
        let stmt = &node.children[idx];

        if let AstType::Return = stmt.kind {
            if let Some(expr) = stmt.children.get(0) {
                if !emit_expr_value_in_scope(module, ops, scope_prefix, scope_name, expr) {
                    let k = module.intern_const(IRConst::Null);
                    ops.push(IROp {
                        kind: IROpKind::LoadConst(k),
                        span: expr.span.clone(),
                    });
                }
            }
            ops.push(IROp {
                kind: IROpKind::Return,
                span: stmt.span.clone(),
            });
            continue;
        }

        if let AstType::Call { .. } = stmt.kind {
            if emit_call_stmt(module, ops, scope_prefix, scope_name, stmt) {
                continue;
            }
        }

        if let AstType::Assignment = stmt.kind {
            if stmt.children.len() >= 2 {
                if let Some(key) = extract_identifier(&stmt.children[0]) {
                    let gid = module.intern_global(format!("{scope_prefix}:{scope_name}.{key}"));
                    let rhs = &stmt.children[1];

                    if scope_prefix == "workspace" && key == "projects" {
                        if let AstType::Array = rhs.kind {
                            let mut elems = Vec::with_capacity(rhs.children.len());
                            let mut all_idents = true;
                            for e in &rhs.children {
                                if let AstType::Identifier { name } = &e.kind {
                                    elems.push(IRConst::Ref {
                                        scope: "project".to_string(),
                                        object: name.to_string(),
                                    });
                                } else {
                                    all_idents = false;
                                    break;
                                }
                            }
                            if all_idents {
                                let arr_idx = module.intern_const(IRConst::Array(elems));
                                ops.push(IROp {
                                    kind: IROpKind::LoadConst(arr_idx),
                                    span: rhs.span.clone(),
                                });
                                ops.push(IROp {
                                    kind: IROpKind::StoreVar(gid),
                                    span: rhs.span.clone(),
                                });
                                continue; // handled
                            }
                        }
                    }

                    if emit_expr_value_in_scope(module, ops, scope_prefix, scope_name, rhs) {
                        ops.push(IROp {
                            kind: IROpKind::StoreVar(gid),
                            span: rhs.span.clone(),
                        });
                    } else {
                        let k = module.intern_const(IRConst::Null);
                        ops.push(IROp {
                            kind: IROpKind::LoadConst(k),
                            span: rhs.span.clone(),
                        });
                        ops.push(IROp {
                            kind: IROpKind::StoreVar(gid),
                            span: rhs.span.clone(),
                        });
                    }
                }
            }
            continue;
        }

        if stmt.children.len() >= 2 {
            if let Some(key) = extract_identifier(&stmt.children[0]) {
                let gid = module.intern_global(format!("{scope_prefix}:{scope_name}.{key}"));
                let rhs = &stmt.children[1];
                if emit_call_value(module, ops, scope_prefix, scope_name, rhs) {
                    ops.push(IROp {
                        kind: IROpKind::StoreVar(gid),
                        span: rhs.span.clone(),
                    });
                } else if emit_store_from_value_in_scope(
                    module,
                    ops,
                    scope_prefix,
                    scope_name,
                    gid,
                    rhs,
                ) {
                } else {
                    let _ = emit_store_global_from_value(module, ops, gid, rhs);
                }
            }
        }
    }
}
