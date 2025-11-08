use crate::codegen::ir::*;
use crate::codegen::scheduler::schedule_project_body as schedule_body;
use crate::parser::{AstNode, AstType};
use std::collections::HashSet;

/// Lower the AST into a minimal IR module.
/// Current behavior:
/// - Create an IR function for each Stage/Task node.
/// - Create a `main` function that calls each discovered function in order.
/// - Ignore Include/Import nodes (should be expanded prior to lowering).
pub fn lower_ast_to_ir(root: &AstNode<'_>) -> ModuleIR {
    let mut module = ModuleIR::new();

    // Track functions to call from main in discovery order
    let mut call_list: Vec<u32> = Vec::new();
    // Avoid duplicate function names/IDs
    let mut seen: HashSet<String> = HashSet::new();

    // Discover functions
    lower_discover(root, &mut module, &mut call_list, &mut seen);

    // Build `main` that calls all discovered functions
    let mut main_ops: Vec<IROp> = Vec::new();
    for fid in call_list {
        main_ops.push(IROp { kind: IROpKind::Call(fid, 0), span: None });
    }
    main_ops.push(IROp { kind: IROpKind::Return, span: None });

    let main = IRFunction {
        name: "main".into(),
        params: vec![],
        blocks: vec![BasicBlock { label: 0, ops: main_ops, next: vec![] }],
    };
    module.add_function(main);

    module
}

// Convert an AST value into an IRConst, supporting nested arrays.
// Adjust variant names if your AstType uses different names.
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
                let c = value_irconst(child)?;
                elems.push(c);
            }
            Some(IRConst::Array(elems))
        }
        // If your grammar supports explicit null
        AstType::Null => Some(IRConst::Null),
        _ => None,
    }
}

// When emitting ops for k/v, use value_irconst then LoadConst + StoreGlobal
fn emit_store_global_from_value(
    module: &mut ModuleIR,
    ops: &mut Vec<IROp>,
    global_id: u32,
    value_node: &AstNode<'_>,
) -> bool {
    if let Some(konst) = value_irconst(value_node) {
        let idx = module.intern_const(konst);
        ops.push(IROp { kind: IROpKind::LoadConst(idx), span: value_node.span.clone() });
        ops.push(IROp { kind: IROpKind::StoreGlobal(global_id), span: value_node.span.clone() });
        true
    } else {
        false
    }
}

// Lower k/v assignments for a generic scope (project/stage/workspace)
fn lower_kv_body(
    scope: &str,              // "project" | "stage" | "workspace"
    name: &str,
    node: &AstNode<'_>,
    module: &mut ModuleIR,
    ops: &mut Vec<IROp>,
) {
    let order = schedule_body(name, node, module);
    for idx in order {
        let stmt = &node.children[idx];

        // Pattern: Assignment node with [key, value]
        if let AstType::Assignment = stmt.kind {
            if stmt.children.len() >= 2 {
                if let Some(key) = extract_identifier(&stmt.children[0]) {
                    let gid = module.intern_global(format!("{scope}:{name}.{key}"));
                    let _ = emit_store_global_from_value(module, ops, gid, &stmt.children[1]);
                }
            }
            continue;
        }

        // Fallback: key/value as first two children
        if stmt.children.len() >= 2 {
            if let Some(key) = extract_identifier(&stmt.children[0]) {
                let gid = module.intern_global(format!("{scope}:{name}.{key}"));
                let _ = emit_store_global_from_value(module, ops, gid, &stmt.children[1]);
            }
        }
    }
}

// Recursively traverse the AST, creating IR functions for Stage and Task nodes.
// Adds created function IDs to call_list (to be invoked from main), preserving discovery order.
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
                let mut ops = Vec::new();
                lower_kv_body("project", name.as_ref(), node, module, &mut ops);
                ops.push(IROp { kind: IROpKind::Return, span: node.span.clone() });
                let fid = module.add_function(IRFunction { name: fname, params: vec![], blocks: vec![BasicBlock { label: 0, ops, next: vec![] }] });
                call_list.push(fid);
            }
            for c in &node.children { lower_discover(c, module, call_list, seen); }
        }
        AstType::Stage { name, .. } => {
            let fname = format!("stage:init:{name}");
            if seen.insert(fname.clone()) {
                let mut ops = Vec::new();
                lower_kv_body("stage", name.as_ref(), node, module, &mut ops);
                ops.push(IROp { kind: IROpKind::Return, span: node.span.clone() });
                let fid = module.add_function(IRFunction { name: fname, params: vec![], blocks: vec![BasicBlock { label: 0, ops, next: vec![] }] });
                call_list.push(fid);
            }
            for c in &node.children { lower_discover(c, module, call_list, seen); }
        }
        AstType::Workspace { name, .. } => {
            let fname = format!("workspace:init:{name}");
            if seen.insert(fname.clone()) {
                let mut ops = Vec::new();
                lower_kv_body("workspace", name.as_ref(), node, module, &mut ops);
                ops.push(IROp { kind: IROpKind::Return, span: node.span.clone() });
                let fid = module.add_function(IRFunction { name: fname, params: vec![], blocks: vec![BasicBlock { label: 0, ops, next: vec![] }] });
                call_list.push(fid);
            }
            for c in &node.children { lower_discover(c, module, call_list, seen); }
        }
        // Ignore include/import (should be expanded already), but still traverse just in case.
        AstType::Include { .. } | AstType::Import { .. } => {
            for child in &node.children {
                lower_discover(child, module, call_list, seen);
            }
        }
        // Containers (e.g., Script, Workspace, Project, others): just traverse children.
        _ => {
            for child in &node.children {
                lower_discover(child, module, call_list, seen);
            }
        }
    }
}

// Helper to pull identifier text from a node
fn extract_identifier(node: &AstNode<'_>) -> Option<String> {
    match &node.kind {
        AstType::Identifier { name } => Some(name.to_string()),
        _ => None,
    }
}

// Match your actual AST variants (String/Number/Boolean/Identifier/ShellCommand).
fn lower_literal(node: &AstNode<'_>, module: &mut ModuleIR) -> Option<IROpKind> {
    match &node.kind {
        AstType::String { value } => {
            let idx = module.intern_const(IRConst::Str(value.to_string()));
            Some(IROpKind::LoadConst(idx))
        }
        AstType::Boolean { value } => {
            let idx = module.intern_const(IRConst::Bool(*value));
            Some(IROpKind::LoadConst(idx))
        }
        AstType::Number { value } => {
            let idx = module.intern_const(IRConst::Int(*value as i64));
            Some(IROpKind::LoadConst(idx))
        }
        AstType::Identifier { name } => {
            let idx = module.intern_const(IRConst::Ident(name.to_string()));
            Some(IROpKind::LoadConst(idx))
        }
        AstType::ShellCommand { command, .. } => {
            let idx = module.intern_const(IRConst::Command(command.to_string()));
            Some(IROpKind::LoadConst(idx))
        }
        // Array currently has no payload in AstType; elements are in children.
        // TODO: introduce BuildArray opcode or IRConst::Array if you want to persist arrays.
        AstType::Array => None,
        _ => None,
    }
}

// Assignment in your AST is a bare node; key/value are first two children.
pub fn extract_key_value<'a>(node: &'a AstNode<'a>) -> Option<(String, &'a AstNode<'a>)> {
    match &node.kind {
        AstType::Assignment => {
            if node.children.len() >= 2 {
                if let Some(key) = extract_identifier(&node.children[0]) {
                    return Some((key, &node.children[1]));
                }
            }
            None
        }
        _ => None,
    }
}