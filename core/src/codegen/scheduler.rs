use std::collections::{HashMap, HashSet};
use crate::parser::{AstNode, AstType};
use crate::codegen::ir::ModuleIR;

// Scope we schedule within
#[derive(Debug, Clone)]
pub enum ScopeKind<'a> {
    Workspace(&'a str),
    Project(&'a str),
    Stage(&'a str),
    Task(&'a str),
    Global,
    Local,
}

#[derive(Default, Clone)]
struct RW {
    writes: HashSet<String>,
    reads: HashSet<String>,
    side_effect: bool,
}

// FQ symbol helpers
fn fq(scope: &ScopeKind<'_>, key: &str) -> String {
    match scope {
        ScopeKind::Workspace(n) => format!("workspace:{n}.{key}"),
        ScopeKind::Project(n)   => format!("project:{n}.{key}"),
        ScopeKind::Stage(n)     => format!("stage:{n}.{key}"),
        ScopeKind::Task(n)      => format!("task:{n}.{key}"),
        ScopeKind::Global       => format!("global:{key}"),
        ScopeKind::Local        => format!("local:{key}"),
    }
}

// Extract key/value pair from an assignment-shaped node: Assignment(key, value)
pub fn extract_key_value<'a>(node: &'a AstNode<'a>) -> Option<(String, &'a AstNode<'a>)> {
    if let AstType::Assignment = node.kind {
        if node.children.len() >= 2 {
            if let Some(key) = extract_identifier(&node.children[0]) {
                return Some((key, &node.children[1]));
            }
        }
    }
    None
}

// Identifier text
fn extract_identifier(node: &AstNode<'_>) -> Option<String> {
    match &node.kind {
        AstType::Identifier { name } => Some(name.to_string()),
        _ => None,
    }
}

// Collect read/write info for a single statement within a scope
fn collect_rw_for_stmt(
    scope: &ScopeKind<'_>,
    stmt: &AstNode<'_>,
    out: &mut RW,
) {
    if let Some((key, value_node)) = extract_key_value(stmt) {
        out.writes.insert(fq(scope, &key));
        collect_reads(scope, value_node, &mut out.reads);
    }
    if let AstType::CallExpression { args, .. } = &stmt.kind {
        for a in args {
            collect_reads(scope, a, &mut out.reads);
        }
    }
    match stmt.kind {
        AstType::Include { .. } | AstType::Import { .. } => out.side_effect = true,
        _ => {}
    }
}

// Walk value expressions to collect reads within this scope
fn collect_reads(scope: &ScopeKind<'_>, node: &AstNode<'_>, reads: &mut HashSet<String>) {
    match &node.kind {
        // Bare identifiers inside values are treated as referring to a key in the same scope
        // e.g., default_project = members[0] reads "members" in workspace scope
        AstType::Identifier { name } => {
            reads.insert(fq(scope, name));
        }
        _ => {
            for c in &node.children {
                collect_reads(scope, c, reads);
            }
        }
    }
}

// Generic scheduler for a "key/value" body under a scope
pub fn schedule_kv_body(scope: ScopeKind<'_>, body: &AstNode<'_>, _mod_ir: &ModuleIR) -> Vec<usize> {
    let n = body.children.len();
    if n == 0 { return Vec::new(); }

    let mut infos = vec![RW::default(); n];
    for (idx, stmt) in body.children.iter().enumerate() {
        collect_rw_for_stmt(&scope, stmt, &mut infos[idx]);
    }

    // Graph
    let mut last_writer: HashMap<String, usize> = HashMap::new();
    let mut indeg = vec![0usize; n];
    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
    let mut last_side_effect: Option<usize> = None;

    for j in 0..n {
        // Def-use edges
        for r in &infos[j].reads {
            if let Some(&i) = last_writer.get(r) {
                adj[i].push(j);
                indeg[j] += 1;
            }
        }
        // Write-after-write edges (last-write wins)
        for w in &infos[j].writes {
            if let Some(&i) = last_writer.get(w) {
                adj[i].push(j);
                indeg[j] += 1;
            }
            last_writer.insert(w.clone(), j);
        }
        // Side-effect barrier sequencing
        if infos[j].side_effect {
            if let Some(i) = last_side_effect {
                adj[i].push(j);
                indeg[j] += 1;
            }
            last_side_effect = Some(j);
        }
    }

    // Stable Kahn’s topo (preserve AST order among independents)
    let mut ready: Vec<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
    ready.sort_unstable();
    let mut schedule = Vec::with_capacity(n);

    while let Some(j) = if ready.is_empty() { None } else { Some(ready.remove(0)) } {
        schedule.push(j);
        for &k in &adj[j] {
            indeg[k] -= 1;
            if indeg[k] == 0 {
                let pos = ready.binary_search(&k).unwrap_or_else(|p| p);
                ready.insert(pos, k);
            }
        }
    }

    // Fallback to AST order for any leftover (cycle or missing edges)
    if schedule.len() != n {
        for i in 0..n {
            if !schedule.contains(&i) {
                schedule.push(i);
            }
        }
    }

    schedule
}

// Convenience wrappers per scope
pub fn schedule_workspace_body(name: &str, body: &AstNode<'_>, ir: &ModuleIR) -> Vec<usize> {
    schedule_kv_body(ScopeKind::Workspace(name), body, ir)
}
pub fn schedule_project_body(name: &str, body: &AstNode<'_>, ir: &ModuleIR) -> Vec<usize> {
    schedule_kv_body(ScopeKind::Project(name), body, ir)
}
pub fn schedule_stage_body(name: &str, body: &AstNode<'_>, ir: &ModuleIR) -> Vec<usize> {
    schedule_kv_body(ScopeKind::Stage(name), body, ir)
}
pub fn schedule_task_body(name: &str, body: &AstNode<'_>, ir: &ModuleIR) -> Vec<usize> {
    schedule_kv_body(ScopeKind::Task(name), body, ir)
}