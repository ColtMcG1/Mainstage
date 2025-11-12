//! ./codegen/lowering.rs
//! (updated to use new scope-specific scheduler)

use crate::codegen::ir::*;
use crate::codegen::scheduler::{
    schedule_project_body, schedule_stage_body, schedule_task_body, schedule_workspace_body,
};
use crate::parser::{AstNode, AstType};
use std::collections::HashSet;

// Helper: does a node have an attribute key?
fn has_attr(node: &AstNode<'_>, key: &str) -> bool {
    node.attributes.iter().any(|a| a.name == key)
}

// Resolve the function id for a given scope node (after functions are added)
fn func_id_for_node(module: &ModuleIR, node: &AstNode<'_>) -> Option<u32> {
    match &node.kind {
        AstType::Stage { name, .. } => module
            .func_index
            .get(&format!("stage:init:{name}"))
            .copied(),
        AstType::Project { name, .. } => module
            .func_index
            .get(&format!("project:init:{name}"))
            .copied(),
        AstType::Workspace { name, .. } => module
            .func_index
            .get(&format!("workspace:init:{name}"))
            .copied(),
        _ => None,
    }
}

// Scan AST to select entrypoint node: prefer Stage/Project with [entrypoint], else first Workspace.
fn find_entrypoint_node<'a>(root: &'a AstNode<'a>) -> Option<&'a AstNode<'a>> {
    let mut first_workspace: Option<&AstNode> = None;
    let mut explicit: Option<&AstNode> = None;

    fn walk<'a>(
        n: &'a AstNode<'a>,
        first_ws: &mut Option<&'a AstNode<'a>>,
        explicit: &mut Option<&'a AstNode<'a>>,
    ) {
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
        for c in &n.children {
            walk(c, first_ws, explicit);
        }
    }

    walk(root, &mut first_workspace, &mut explicit);
    explicit.or(first_workspace)
}

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

    // 1) Discover and add functions for all scopes (unchanged)
    lower_discover(root, &mut module, &mut call_list, &mut seen);

    // 2) Choose a single entrypoint function id
    let entry_fid: Option<u32> =
        find_entrypoint_node(root).and_then(|ep| func_id_for_node(&module, ep));

    // 3) Build main to call ONLY the entrypoint (fallback: no call)
    let mut main_ops: Vec<IROp> = Vec::new();
    if let Some(fid) = entry_fid {
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
        ops.push(IROp {
            kind: IROpKind::StoreVar(dest_gid),
            span: rhs.span.clone(),
        });
        true
    } else {
        false
    }
}

// Flatten "a.b.c" to ["a","b","c"]
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

// Resolve which scope an object name belongs to by inspecting function index
fn resolve_object_scope<'a>(module: &ModuleIR, object: &str) -> Option<&'static str> {
    if module
        .func_index
        .contains_key(&format!("stage:init:{object}"))
    {
        Some("stage")
    } else if module
        .func_index
        .contains_key(&format!("project:init:{object}"))
    {
        Some("project")
    } else if module
        .func_index
        .contains_key(&format!("workspace:init:{object}"))
    {
        Some("workspace")
    } else if module
        .func_index
        .contains_key(&format!("task:init:{object}"))
    {
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
fn emit_member_access_value(
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
                    ops.push(IROp { kind: IROpKind::Call(fid, arguments.len() as u8), span: target.span.clone() });
                }
                if let Some(fq) = fq_for_member(module, base, field) {
                    let gid = module.intern_global(fq);
                    ops.push(IROp { kind: IROpKind::LoadVar(gid), span: node.span.clone() });
                    return true;
                }
            }
            return false;
        }

        // Static identifier base
        if let AstType::Identifier { name: obj } = &target.kind {
            if let Some(fq) = fq_for_member(module, obj, field) {
                let gid = module.intern_global(fq);
                ops.push(IROp { kind: IROpKind::LoadVar(gid), span: node.span.clone() });
                return true;
            }
            // Dynamic: treat identifier as variable holding object name
            let var_fq = format!("{scope_prefix}:{scope_name}.{obj}");
            let gid = module.intern_global(var_fq);
            ops.push(IROp { kind: IROpKind::LoadVar(gid), span: target.span.clone() });
            ops.push(IROp { kind: IROpKind::LoadMemberDyn(field_cidx), span: node.span.clone() });
            return true;
        }

        // Nested flatten attempt
        if let Some(parts) = flatten_member_access(target) {
            let object = parts.last().unwrap();
            if let Some(fq) = fq_for_member(module, object, field) {
                let gid = module.intern_global(fq);
                ops.push(IROp { kind: IROpKind::LoadVar(gid), span: node.span.clone() });
                return true;
            }
        }

        // Generic dynamic fallback: evaluate target as value then dynamic member
        if emit_expr_value_in_scope(module, ops, scope_prefix, scope_name, target) {
            ops.push(IROp { kind: IROpKind::LoadMemberDyn(field_cidx), span: node.span.clone() });
            return true;
        }
    }
    false
}

// Support lowering return statements and call expressions (task/stage) in value position.

fn emit_expr_value_in_scope(
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
        AstType::CallExpression { target, arguments } => {
            // Callee must be identifier
            let name = if let AstType::Identifier { name } = &target.kind {
                name.as_ref()
            } else {
                return false;
            };

            // Builtin: say(expr)
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

            // Builtin: ask(prompt?)
            if name == "ask" {
                if arguments.len() > 1 {
                    return false;
                }
                let argc = arguments.len() as u8;
                if argc == 1 && !emit_expr_value_in_scope(module, ops, scope_prefix, scope_name, &arguments[0]) {
                    return false;
                }
                ops.push(IROp {
                    kind: IROpKind::Ask(argc),
                    span: expr.span.clone(),
                });
                return true;
            }

            // Builtin: read(path) -> string
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

            // Builtin: write(path, data) -> unit
            if name == "write" {
                if arguments.len() != 2 {
                    return false;
                }
                // Evaluate args left-to-right: path, then data
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

            // Regular callable: resolve fid, lower args (recursively), then Call
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

fn emit_call_stmt(
    module: &mut ModuleIR,
    ops: &mut Vec<IROp>,
    scope_prefix: &str,
    scope_name: &str,
    stmt: &AstNode<'_>,
) -> bool {
    if let AstType::CallExpression { .. } = &stmt.kind {
        // Lower as value-producing expression. Builtin say will consume its arg.
        if emit_expr_value_in_scope(module, ops, scope_prefix, scope_name, stmt) {
            // NOTE: Regular task calls leave their return value on the stack.
            // If you want to discard it, add a Pop opcode and emit it here.
            return true;
        }
    }
    false
}

// Lower call in value position (assignment RHS, return expr)fn emit_call_value(
fn emit_call_value(
    module: &mut ModuleIR,
    ops: &mut Vec<IROp>,
    scope_prefix: &str,
    scope_name: &str,
    expr: &AstNode<'_>,
) -> bool {
    emit_expr_value_in_scope(module, ops, scope_prefix, scope_name, expr)
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

        // Fallback key/value
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
                    let _ = emit_store_global_from_value(module, ops, gid, rhs); // if you keep it around
                }
            }
        }
    }
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
                            ops.push(IROp {
                                kind: IROpKind::Call(*fid, 0),
                                span: node.span.clone(),
                            });
                        }
                    }
                }

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

                // 1) Extract workspace.projects and store as a global
                let linked_projects = scan_workspace_projects(node);
                module
                    .workspace_projects
                    .insert(name.to_string(), linked_projects.clone());

                if !linked_projects.is_empty() {
                    // For now serialize as comma-separated string; later switch to array Value
                    let serialized = linked_projects.join(",");
                    let cidx = module.intern_const(IRConst::Str(serialized));
                    let w_gid = module.intern_global(format!("workspace:{name}.projects"));
                    ops.push(IROp { kind: IROpKind::LoadConst(cidx), span: node.span.clone() });
                    ops.push(IROp { kind: IROpKind::StoreVar(w_gid), span: node.span.clone() });
                }

                // 2) Prelude: ensure referenced objects are initialized (stage/project/workspace/task)
                //    and propagate workspace.projects into stages/tasks before their init runs
                let mut refs = std::collections::HashMap::new();
                collect_member_object_refs(module, node, &mut refs);

                for (scope, set) in refs {
                    for obj in set {
                        // If workspace has projects, copy to stage/task scope prior to init
                        if !linked_projects.is_empty() && (scope == "stage" || scope == "task") {
                            let w_gid = module.intern_global(format!("workspace:{name}.projects"));
                            let dst_gid = module.intern_global(format!("{scope}:{obj}.projects"));
                            // Load from workspace, store into stage/task
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
}
