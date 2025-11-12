//! ./codegen/lowering/discover.rs
//! Entrypoint selection and scope discovery.

use std::collections::HashSet;

use crate::codegen::ir::*;
use crate::codegen::scheduler::{
    schedule_project_body, schedule_stage_body, schedule_task_body, schedule_workspace_body,
};
use crate::parser::{AstNode, AstType};

use super::stmt::emit_kv_ops;

// Helper: does a node have an attribute key?
fn has_attr(node: &AstNode<'_>, key: &str) -> bool {
    node.attributes.iter().any(|a| a.name == key)
}

// Resolve the function id for a given scope node (after functions are added)
fn func_id_for_node(module: &ModuleIR, node: &AstNode<'_>) -> Option<u32> {
    match &node.kind {
        AstType::Stage { name, .. } => module.func_index.get(&format!("stage:init:{name}")).copied(),
        AstType::Project { name, .. } => module.func_index.get(&format!("project:init:{name}")).copied(),
        AstType::Workspace { name, .. } => module.func_index.get(&format!("workspace:init:{name}")).copied(),
        _ => None,
    }
}

// Scan AST to select entrypoint node: prefer Stage/Project with [entrypoint], else first Workspace.
fn find_entrypoint_node<'a>(root: &'a AstNode<'a>) -> Option<&'a AstNode<'a>> {
    let mut first_workspace: Option<&AstNode> = None;
    let mut explicit: Option<&AstNode> = None;

    fn walk<'a>(n: &'a AstNode<'a>, first_ws: &mut Option<&'a AstNode<'a>>, explicit: &mut Option<&'a AstNode<'a>>) {
        match &n.kind {
            AstType::Stage { .. } | AstType::Project { .. } => {
                if has_attr(n, "entrypoint") && explicit.is_none() {
                    *explicit = Some(n);
                }
            }
            AstType::Workspace { .. } => {
                if first_ws.is_none() {
                    *first_ws = Some(n);
                }
            }
            _ => {}
        }
        for c in &n.children { walk(c, first_ws, explicit); }
    }

    walk(root, &mut first_workspace, &mut explicit);
    explicit.or(first_workspace)
}

// NEW: scan a workspace body for `projects = [ ... ]` and return project names
fn scan_workspace_projects(ws: &AstNode<'_>) -> Vec<String> {
    let mut out = Vec::new();
    for stmt in &ws.children {
        if let AstType::Assignment = stmt.kind {
            if stmt.children.len() >= 2 {
                if let Some(key) = extract_identifier(&stmt.children[0]) {
                    if key == "projects" {
                        let rhs = &stmt.children[1];
                        if let AstType::Array { .. } = rhs.kind {
                            for elem in &rhs.children {
                                if let AstType::Identifier { name } = &elem.kind {
                                    out.push(name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

fn extract_identifier(node: &AstNode<'_>) -> Option<String> {
    match &node.kind {
        AstType::Identifier { name } => Some(name.to_string()),
        _ => None,
    }
}

// Collect referenced objects by scope from member access chains in a subtree
fn collect_member_object_refs<'a>(
    module: &ModuleIR,
    node: &AstNode<'a>,
    out: &mut std::collections::HashMap<&'static str, std::collections::HashSet<String>>,
) {
    match &node.kind {
        AstType::MemberAccess { target, member: _ } => {
            if let AstType::Identifier { name: base } = &target.kind {
                if let Some(scope) = resolve_object_scope(module, base) {
                    out.entry(scope).or_default().insert(base.to_string());
                }
            } else if let AstType::CallExpression { target, .. } = &target.kind {
                if let AstType::Identifier { name: base } = &target.kind {
                    if let Some(scope) = resolve_object_scope(module, base) {
                        out.entry(scope).or_default().insert(base.to_string());
                    }
                }
            }
            collect_member_object_refs(module, target, out);
        }
        // NEW: also traverse call expression target/arguments so app().result is seen
        AstType::CallExpression { target, arguments } => {
            collect_member_object_refs(module, target, out);
            for a in arguments {
                collect_member_object_refs(module, a, out);
            }
            for c in &node.children {
                collect_member_object_refs(module, c, out);
            }
        }
        _ => {
            for c in &node.children {
                collect_member_object_refs(module, c, out);
            }
        }
    }

    // Nested helper reused from expr module
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
}

/// Lower the AST into a minimal IR module.
pub fn lower_ast_to_ir(root: &AstNode<'_>) -> ModuleIR {
    let mut module = ModuleIR::new();
    let mut call_list: Vec<u32> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // 1) Discover and add functions for all scopes (unchanged)
    lower_discover(root, &mut module, &mut call_list, &mut seen);

    // 2) Choose a single entrypoint function id
    let entry_fid: Option<u32> =
        find_entrypoint_node(root).and_then(|ep| func_id_for_node(&module, ep));

    // 3) Build main to call ONLY the entrypoint (fallback: no call)
    let mut main_ops: Vec<IROp> = Vec::new();
    if let Some(fid) = entry_fid {
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

                // Prelude: ensure referenced objects are initialized (stage/project/workspace/task)
                let mut refs = std::collections::HashMap::new();
                collect_member_object_refs(module, node, &mut refs);
                for (scope, set) in refs {
                    for obj in set {
                        if let Some(fid) = module.func_index.get(&format!("{scope}:init:{obj}")) {
                            ops.push(IROp { kind: IROpKind::Call(*fid, 0), span: node.span.clone() });
                        }
                    }
                }

                emit_kv_ops("project", name, node, &order, module, &mut ops);
                ops.push(IROp { kind: IROpKind::Return, span: node.span.clone() });
                let fid = module.add_function(IRFunction {
                    name: fname,
                    params: vec![],
                    blocks: vec![BasicBlock { label: 0, ops, next: vec![] }],
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
                ops.push(IROp { kind: IROpKind::Return, span: node.span.clone() });
                let fid = module.add_function(IRFunction {
                    name: fname,
                    params: vec![],
                    blocks: vec![BasicBlock { label: 0, ops, next: vec![] }],
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

                // 1) Extract workspace.projects and store as a global
                let linked_projects = scan_workspace_projects(node);
                module.workspace_projects.insert(name.to_string(), linked_projects.clone());

                if !linked_projects.is_empty() {
                    // For now serialize as comma-separated string; later switch to array Value
                    let serialized = linked_projects.join(",");
                    let cidx = module.intern_const(IRConst::Str(serialized));
                    let w_gid = module.intern_global(format!("workspace:{name}.projects"));
                    ops.push(IROp { kind: IROpKind::LoadConst(cidx), span: node.span.clone() });
                    ops.push(IROp { kind: IROpKind::StoreVar(w_gid), span: node.span.clone() });
                }

                // 2) Prelude: ensure referenced objects are initialized and propagate projects
                let mut refs = std::collections::HashMap::new();
                collect_member_object_refs(module, node, &mut refs);

                for (scope, set) in refs {
                    for obj in set {
                        if !linked_projects.is_empty() && (scope == "stage" || scope == "task") {
                            let w_gid = module.intern_global(format!("workspace:{name}.projects"));
                            let dst_gid = module.intern_global(format!("{scope}:{obj}.projects"));
                            ops.push(IROp { kind: IROpKind::LoadVar(w_gid), span: node.span.clone() });
                            ops.push(IROp { kind: IROpKind::StoreVar(dst_gid), span: node.span.clone() });
                        }
                        if let Some(fid) = module.func_index.get(&format!("{scope}:init:{obj}")) {
                            ops.push(IROp { kind: IROpKind::Call(*fid, 0), span: node.span.clone() });
                        }
                    }
                }

                // 3) Lower workspace body statements
                emit_kv_ops("workspace", name, node, &order, module, &mut ops);

                ops.push(IROp { kind: IROpKind::Return, span: node.span.clone() });
                let fid = module.add_function(IRFunction {
                    name: fname,
                    params: vec![],
                    blocks: vec![BasicBlock { label: 0, ops, next: vec![] }],
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
                ops.push(IROp { kind: IROpKind::Return, span: node.span.clone() });
                let fid = module.add_function(IRFunction {
                    name: fname,
                    params: vec![],
                    blocks: vec![BasicBlock { label: 0, ops, next: vec![] }],
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