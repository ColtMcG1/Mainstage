use std::collections::{HashMap, HashSet, VecDeque};
use crate::parser::{AstNode, AstType};
use crate::codegen::ir::ModuleIR;

#[derive(Default, Clone)]
struct RW {
    writes: HashSet<String>,
    reads: HashSet<String>,
    side_effect: bool,
}

// Extract a symbol name for project assignment key (e.g., "project:{proj}.{key}")
fn sym_for_project_key(project: &str, key: &str) -> String {
    format!("project:{project}.{key}")
}

fn collect_rw_for_stmt(
    project: &str,
    stmt: &AstNode<'_>,
    out: &mut RW,
) {
    // Writes: project key on assignment
    if let Some((key, value_node)) = super::lowering::extract_key_value(stmt) {
        out.writes.insert(sym_for_project_key(project, &key));
        collect_reads(value_node, &mut out.reads);
    }

    // Side effects: includes/imports or shell executions
    match stmt.kind {
        AstType::Include { .. } | AstType::Import { .. } => out.side_effect = true,
        _ => {}
    }
}

// Walk expressions to collect identifier reads (adapt to your AST).
fn collect_reads(node: &AstNode<'_>, reads: &mut HashSet<String>) {
    match &node.kind {
        // Example: identifiers that refer to other project keys like `root` or `project.key`
        AstType::Identifier { name } => {
            // If you have scoped names, normalize here.
            reads.insert(name.to_string());
        }
        _ => {
            for c in &node.children {
                collect_reads(c, reads);
            }
        }
    }
}

// Returns a schedule of indices into `body.children`
pub fn schedule_project_body(project_name: &str, body: &AstNode<'_>, _mod_ir: &ModuleIR) -> Vec<usize> {
    let n = body.children.len();
    let mut infos = vec![RW::default(); n];
    for (idx, stmt) in body.children.iter().enumerate() {
        collect_rw_for_stmt(project_name, stmt, &mut infos[idx]);
    }

    // Build last-writer map
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

        // Write-after-write edges (preserve last-one-wins)
        for w in &infos[j].writes {
            if let Some(&i) = last_writer.get(w) {
                adj[i].push(j);
                indeg[j] += 1;
            }
            last_writer.insert(w.clone(), j);
        }

        // Side-effect barrier
        if infos[j].side_effect {
            if let Some(i) = last_side_effect {
                adj[i].push(j);
                indeg[j] += 1;
            }
            last_side_effect = Some(j);
        }
    }

    // Stable Kahn’s algorithm: prefer smaller AST index
    let mut ready: VecDeque<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
    // Keep it stable by sorting once
    let mut ready_vec: Vec<usize> = ready.drain(..).collect();
    ready_vec.sort_unstable();
    let mut schedule = Vec::with_capacity(n);
    let push_ready = |q: &mut Vec<usize>, v: usize| {
        // binary insert to keep sorted
        let pos = q.binary_search(&v).unwrap_or_else(|p| p);
        q.insert(pos, v);
    };

    while let Some(j) = ready_vec.first().cloned() {
        ready_vec.remove(0);
        schedule.push(j);
        for &k in &adj[j] {
            indeg[k] -= 1;
            if indeg[k] == 0 {
                push_ready(&mut ready_vec, k);
            }
        }
    }

    // If we didn’t schedule all, there’s a cycle; fall back to AST order for remaining
    if schedule.len() != n {
        // You can report a cycle here using your Accumulator
        for i in 0..n {
            if !schedule.contains(&i) {
                schedule.push(i);
            }
        }
    }

    schedule
}