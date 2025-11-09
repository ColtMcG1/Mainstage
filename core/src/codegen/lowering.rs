//! ./codegen/lowering.rs
//! (updated to use new scope-specific scheduler)

use crate::codegen::ir::*;
use crate::codegen::scheduler::{
    schedule_project_body, schedule_stage_body, schedule_task_body, schedule_workspace_body,
};
use crate::parser::{AstNode, AstType};
use std::collections::HashSet;

/// Lower the AST into a minimal IR module.
/// Current behavior:
/// - Create an IR function for each Stage/Task node.
/// - Create a `main` function that calls each discovered function in order.
/// - Ignore Include/Import nodes (should be expanded prior to lowering).
/// - Lower key/value assignments within each scope into global variable stores.
/// # Parameters
/// - `root`: The root AST node to lower.
/// # Returns
/// - `ModuleIR`: The resulting IR module.
pub fn lower_ast_to_ir(root: &AstNode<'_>) -> ModuleIR {
    let mut module = ModuleIR::new();
    let mut call_list: Vec<u32> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    lower_discover(root, &mut module, &mut call_list, &mut seen);

    let mut main_ops: Vec<IROp> = Vec::new();
    for fid in call_list {
        main_ops.push(IROp {
            kind: IROpKind::Call(fid, 0),
            span: None,
        });
    }
    main_ops.push(IROp {
        kind: IROpKind::Return,
        span: None,
    });

    let main = IRFunction {
        name: "main".into(),
        params: vec![],
        blocks: vec![BasicBlock {
            label: 0,
            ops: main_ops,
            next: vec![],
        }],
    };
    module.add_function(main);
    module
}

// Convert AST value into IRConst (supports nested arrays)
fn value_irconst(node: &AstNode<'_>) -> Option<IRConst> {
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

fn emit_store_global_from_value(
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

// Load a value into the stack, resolving identifiers to variable loads using scope.
fn emit_value_in_scope(
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
            ops.push(IROp { kind: IROpKind::LoadVar(gid), span: node.span.clone() });
            true
        }
        // Literals/arrays -> LoadConst as before
        _ => {
            if let Some(konst) = value_irconst(node) {
                let idx = module.intern_const(konst);
                ops.push(IROp { kind: IROpKind::LoadConst(idx), span: node.span.clone() });
                true
            } else {
                false
            }
        }
    }
}

// Store: if RHS is identifier, copy its value via LoadVar then StoreVar
fn emit_store_from_value_in_scope(
    module: &mut ModuleIR,
    ops: &mut Vec<IROp>,
    scope_prefix: &str,
    scope_name: &str,
    dest_gid: u32,
    rhs: &AstNode<'_>,
) -> bool {
    if emit_value_in_scope(module, ops, scope_prefix, scope_name, rhs) {
        ops.push(IROp { kind: IROpKind::StoreVar(dest_gid), span: rhs.span.clone() });
        true
    } else {
        false
    }
}

// Support lowering return statements and call expressions (task/stage) in value position.

fn emit_call_stmt(
    module: &mut ModuleIR,
    ops: &mut Vec<IROp>,
    scope_prefix: &str,
    scope_name: &str,
    stmt: &AstNode<'_>,
) -> bool {
    if let AstType::CallExpression { callee, args } = &stmt.kind {
        if let AstType::Identifier { name } = &callee.kind {
            // Builtin: say
            if name.as_ref() == "say" {
                if let Some(arg0) = args.get(0) {
                    let _ = emit_value_in_scope(module, ops, scope_prefix, scope_name, arg0);
                    ops.push(IROp { kind: IROpKind::Say, span: stmt.span.clone() });
                }
                return true;
            }
            else if name.as_ref() == "write" {
                if let Some(arg0) = args.get(0) {
                    let _ = emit_value_in_scope(module, ops, scope_prefix, scope_name, arg0);
                    ops.push(IROp { kind: IROpKind::Write, span: stmt.span.clone() });
                }
                return true;
            }
            else if name.as_ref() == "read" {
                ops.push(IROp { kind: IROpKind::Read, span: stmt.span.clone() });
                return true;
            }
            // Regular callable
            if let Some(fid) = module.get_plain_func(name.as_ref()) {
                for arg in args {
                    let _ = emit_value_in_scope(module, ops, scope_prefix, scope_name, arg);
                }
                ops.push(IROp { kind: IROpKind::Call(fid, args.len() as u8), span: stmt.span.clone() });
                return true;
            }
        }
    }
    false
}

// Lower call in value position (assignment RHS, return expr)
fn emit_call_value(
    module: &mut ModuleIR,
    ops: &mut Vec<IROp>,
    scope_prefix: &str,
    scope_name: &str,
    expr: &AstNode<'_>,
) -> bool {
    if let AstType::CallExpression { callee, args } = &expr.kind {
        if let AstType::Identifier { name } = &callee.kind {
            if name.as_ref() == "say" {
                if let Some(arg0) = args.get(0) {
                    let _ = emit_value_in_scope(module, ops, scope_prefix, scope_name, arg0);
                    // ops.push(IROp { kind: IROpKind::BuiltinSay, span: expr.span.clone() });
                }
                // say returns unit; caller should not store this
                return true;
            }
            if let Some(fid) = module.get_plain_func(name.as_ref()) {
                for arg in args {
                    let _ = emit_value_in_scope(module, ops, scope_prefix, scope_name, arg);
                }
                ops.push(IROp { kind: IROpKind::Call(fid, args.len() as u8), span: expr.span.clone() });
                return true;
            }
        }
    }
    false
}

fn emit_kv_ops(
    scope_prefix: &str,
    scope_name: &str,
    node: &AstNode<'_>,
    order: &[usize],
    module: &mut ModuleIR,
    ops: &mut Vec<IROp>,
) {
    for &idx in order {
        let stmt = &node.children[idx];

        // Return statement
        if let AstType::Return = stmt.kind {
            if let Some(expr) = stmt.children.get(0) {
                // Value before Return
                if !emit_call_value(module, ops, scope_prefix, scope_name, expr)
                    && !emit_value_in_scope(module, ops, scope_prefix, scope_name, expr)
                {
                    // Unknown expression -> treat as Null
                    let k = module.intern_const(IRConst::Null);
                    ops.push(IROp { kind: IROpKind::LoadConst(k), span: expr.span.clone() });
                }
            }
            ops.push(IROp { kind: IROpKind::Return, span: stmt.span.clone() });
            continue;
        }

        // Call statement
        if let AstType::CallExpression { .. } = stmt.kind {
            if emit_call_stmt(module, ops, scope_prefix, scope_name, stmt) {
                continue;
            }
        }

        // Assignment
        if let AstType::Assignment = stmt.kind {
            if stmt.children.len() >= 2 {
                if let Some(key) = extract_identifier(&stmt.children[0]) {
                    let gid = module.intern_global(format!("{scope_prefix}:{scope_name}.{key}"));
                    let rhs = &stmt.children[1];
                    // Try call value first
                    if emit_call_value(module, ops, scope_prefix, scope_name, rhs) {
                        ops.push(IROp { kind: IROpKind::StoreVar(gid), span: rhs.span.clone() });
                    } else if emit_store_from_value_in_scope(module, ops, scope_prefix, scope_name, gid, rhs) {
                        // done
                    } else {
                        // Fallback Null
                        let k = module.intern_const(IRConst::Null);
                        ops.push(IROp { kind: IROpKind::LoadConst(k), span: rhs.span.clone() });
                        ops.push(IROp { kind: IROpKind::StoreVar(gid), span: rhs.span.clone() });
                    }
                }
            }
            continue;
        }

        // Fallback key/value
        if stmt.children.len() >= 2 {
            if let Some(key) = extract_identifier(&stmt.children[0]) {
                let gid = module.intern_global(format!("{scope_prefix}:{scope_name}.{key}"));
                let rhs = &stmt.children[1];
                if emit_call_value(module, ops, scope_prefix, scope_name, rhs) {
                    ops.push(IROp { kind: IROpKind::StoreVar(gid), span: rhs.span.clone() });
                } else if emit_store_from_value_in_scope(module, ops, scope_prefix, scope_name, gid, rhs) {
                } else {
                    let _ = emit_store_global_from_value(module, ops, gid, rhs); // if you keep it around
                }
            }
        }
    }
}

fn lower_discover(
    node: &AstNode<'_>,
    module: &mut ModuleIR,
    call_list: &mut Vec<u32>,
    seen: &mut HashSet<String>,
) {
    match &node.kind {
        AstType::Project { name, .. } => {
            let fname = format!("project:init:{name}");
            if seen.insert(fname.clone()) {
                let order = schedule_project_body(name, node, module);
                let mut ops = Vec::new();
                emit_kv_ops("project", name, node, &order, module, &mut ops);
                ops.push(IROp {
                    kind: IROpKind::Return,
                    span: node.span.clone(),
                });
                let fid = module.add_function(IRFunction {
                    name: fname,
                    params: vec![],
                    blocks: vec![BasicBlock {
                        label: 0,
                        ops,
                        next: vec![],
                    }],
                });
                call_list.push(fid);
            }
            for c in &node.children {
                lower_discover(c, module, call_list, seen);
            }
        }
        AstType::Stage { name, .. } => {
            let fname = format!("stage:init:{name}");
            if seen.insert(fname.clone()) {
                let order = schedule_stage_body(name, node, module);
                let mut ops = Vec::new();
                emit_kv_ops("stage", name, node, &order, module, &mut ops);
                ops.push(IROp {
                    kind: IROpKind::Return,
                    span: node.span.clone(),
                });
                let fid = module.add_function(IRFunction {
                    name: fname,
                    params: vec![],
                    blocks: vec![BasicBlock {
                        label: 0,
                        ops,
                        next: vec![],
                    }],
                });
                call_list.push(fid);
            }
            for c in &node.children {
                lower_discover(c, module, call_list, seen);
            }
        }
        AstType::Workspace { name, .. } => {
            let fname = format!("workspace:init:{name}");
            if seen.insert(fname.clone()) {
                let order = schedule_workspace_body(name, node, module);
                let mut ops = Vec::new();
                emit_kv_ops("workspace", name, node, &order, module, &mut ops);
                ops.push(IROp {
                    kind: IROpKind::Return,
                    span: node.span.clone(),
                });
                let fid = module.add_function(IRFunction {
                    name: fname,
                    params: vec![],
                    blocks: vec![BasicBlock {
                        label: 0,
                        ops,
                        next: vec![],
                    }],
                });
                call_list.push(fid);
            }
            for c in &node.children {
                lower_discover(c, module, call_list, seen);
            }
        }
        AstType::Task { name, .. } => {
            let fname = format!("task:init:{name}");
            if seen.insert(fname.clone()) {
                let order = schedule_task_body(name, node, module);
                let mut ops = Vec::new();
                emit_kv_ops("task", name, node, &order, module, &mut ops);
                ops.push(IROp {
                    kind: IROpKind::Return,
                    span: node.span.clone(),
                });
                let fid = module.add_function(IRFunction {
                    name: fname,
                    params: vec![],
                    blocks: vec![BasicBlock {
                        label: 0,
                        ops,
                        next: vec![],
                    }],
                });
                call_list.push(fid);
            }
            for c in &node.children {
                lower_discover(c, module, call_list, seen);
            }
        }
        AstType::Include { .. } | AstType::Import { .. } => {
            for child in &node.children {
                lower_discover(child, module, call_list, seen);
            }
        }
        _ => {
            for child in &node.children {
                lower_discover(child, module, call_list, seen);
            }
        }
    }
}

fn extract_identifier(node: &AstNode<'_>) -> Option<String> {
    match &node.kind {
        AstType::Identifier { name } => Some(name.to_string()),
        _ => None,
    }
}
