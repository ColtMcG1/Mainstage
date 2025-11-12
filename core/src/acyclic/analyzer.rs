//! file: ./acyclic/analyzer.rs
//! description: Module for analyzing directed acyclic graphs (DAGs).
//!
//! This module provides functionality to analyze DAGs, including detecting cycles and performing topological sorting.
//!
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025/10/25
//! license: See LICENSE file in repository root.

use crate::{
    acyclic::{edge::*, node::*},
    parser::{AstNode, AstParser, AstType},
};
use std::collections::{HashMap, HashSet, VecDeque};

/// Compact DAG structure for efficient analysis.
#[derive(Debug, Clone)]
pub struct CompactDag {
    /// Ordered list of node IDs.
    pub node_ids: Vec<String>,
    /// Mapping from node ID to index in `node_ids`.
    pub id_to_index: HashMap<String, usize>,
    /// Outgoing edges for each node: (target index, edge kind).
    pub out_edges: Vec<Vec<(usize, EdgeKind)>>,
    /// Incoming edges for each node: (source index, edge kind).
    pub in_edges: Vec<Vec<(usize, EdgeKind)>>,
}

impl CompactDag {
    /// Returns the outgoing neighbors of a node.
    pub fn neighbors(&self, idx: usize) -> &[(usize, EdgeKind)] {
        &self.out_edges[idx]
    }
    /// Returns the incoming edges of a node.
    pub fn incoming(&self, idx: usize) -> &[(usize, EdgeKind)] {
        &self.in_edges[idx]
    }
}

/// Analyzer for directed acyclic graphs (DAGs).
#[derive(Debug, Clone)]
pub struct DirectedAcyclicGraphAnalyzer {
    /// All nodes in the graph, indexed by id.
    pub nodes: HashMap<String, AcyclicNode>,
    /// Entry point for DAG analysis.
    pub entry_point: Option<String>,
}

impl DirectedAcyclicGraphAnalyzer {
    /// Creates a new `DirectedAcyclicGraphAnalyzer`.
    pub fn new(ast: AstParser, entry_point: Option<AstNode>) -> Result<Self, ()> {
        let mut nodes = HashMap::new();

        fn ensure(nodes: &mut HashMap<String, AcyclicNode>, id: &str) {
            nodes.entry(id.to_string()).or_insert_with(|| AcyclicNode::new(id.to_string()));
        }

        // First pass: build nodes and structural edges (AST parent -> child)
        fn walk_ast_structural(
            ast_node: &AstNode,
            nodes: &mut HashMap<String, AcyclicNode>,
            parent_id: Option<String>,
        ) {
            let node_id = ast_node.id.clone();
            ensure(nodes, &node_id);
            if let Some(pid) = parent_id {
                ensure(nodes, &pid);
                {
                    let parent = nodes.get_mut(&pid).unwrap();
                    parent.add_structural_edge_to(node_id.clone());
                }
                {
                    let child = nodes.get_mut(&node_id).unwrap();
                    child.push_in(AcyclicEdge::new_structural(pid.clone(), node_id.clone()));
                }
            }
            for child in &ast_node.children {
                walk_ast_structural(child, nodes, Some(node_id.clone()));
            }
        }

        // Second pass: add data dependency edges
        fn add_data_edges(ast_node: &AstNode, nodes: &mut HashMap<String, AcyclicNode>) {
            match &ast_node.kind {
                AstType::Assignment => {
                    // children[0] = LHS; remaining children form the RHS (can be 1+ after expression flattening)
                    if ast_node.children.len() > 1 {
                        let mut producers = HashSet::new();
                        for rhs in ast_node.children.iter().skip(1) {
                            collect_producers(rhs, &mut producers);
                        }
                        for pid in producers {
                            // First, get the source node mutably
                            if let Some(src) = nodes.get_mut(&pid) {
                                src.add_data_edge_to(ast_node.id.clone());
                            }
                            // Then, get the destination node mutably in a separate borrow
                            if let Some(dst) = nodes.get_mut(&ast_node.id) {
                                dst.push_in(AcyclicEdge::new_data(pid.clone(), ast_node.id.clone()));
                            }
                        }
                    }
                }
                AstType::CallExpression { target: _, arguments } => {
                    let mut producers = HashSet::new();
                    for arg in arguments {
                        collect_producers(arg, &mut producers);
                    }
                    for pid in producers {
                        // First, get the source node mutably
                        if let Some(src) = nodes.get_mut(&pid) {
                            src.add_data_edge_to(ast_node.id.clone());
                        }
                        // Then, get the destination node mutably in a separate borrow
                        if let Some(dst) = nodes.get_mut(&ast_node.id) {
                            dst.push_in(AcyclicEdge::new_data(pid.clone(), ast_node.id.clone()));
                        }
                    }
                }
                _ => {}
            }
            for child in &ast_node.children {
                add_data_edges(child, nodes);
            }
        }

        // Helper: collect producer nodes (sources of data) under an expression.
        // Adjust the match to control what counts as a data producer.
        fn collect_producers(expr: &AstNode, out: &mut HashSet<String>) {
            match &expr.kind {
                // Core producers (keep these)
                AstType::Identifier { .. }
                | AstType::Index { .. }
                | AstType::MemberAccess { .. }
                | AstType::CallExpression { .. } => {
                    out.insert(expr.id.clone());
                }

                // Optional: treat literals as producers (comment out to suppress)
                AstType::Number { .. }
                | AstType::String { .. }
                | AstType::Boolean { .. } => {
                    // Uncomment if you want literal data edges:
                    // out.insert(expr.id.clone());
                }

                // Composite / container nodes: descend
                _ => {
                    for c in &expr.children {
                        collect_producers(c, out);
                    }
                }
            }
        }

        // Build structural graph
        let root = ast.root();
        walk_ast_structural(&root, &mut nodes, None);

        // Add data edges across the AST
        add_data_edges(&root, &mut nodes);

        Ok(Self {
            entry_point: entry_point.map(|n| n.id),
            nodes,
        })
    }

    /// Builds the reachable subgraph from the entry point.
    pub fn build_from_entry(&self) -> HashSet<String> {
        let mut visited = HashSet::new();
        if let Some(root_id) = &self.entry_point {
            self.dfs(root_id, &mut visited);
        }
        visited
    }

    fn dfs(&self, node_id: &str, visited: &mut HashSet<String>) {
        if !visited.insert(node_id.to_string()) {
            return;
        }
        if let Some(node) = self.nodes.get(node_id) {
            for edge in &node.edges {
                self.dfs(&edge.target, visited);
            }
        }
    }

    /// Topological sort (Kahn's algorithm) over all edges (both kinds).
    pub fn topo_sort(&self) -> Result<Vec<String>, Vec<String>> {
        let mut in_degree = HashMap::new();
        for node in self.nodes.values() {
            in_degree.entry(node.id.clone()).or_insert(0);
            for edge in &node.edges {
                *in_degree.entry(edge.target.clone()).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut sorted = Vec::new();
        while let Some(id) = queue.pop_front() {
            sorted.push(id.clone());
            if let Some(node) = self.nodes.get(&id) {
                for edge in &node.edges {
                    if let Some(entry) = in_degree.get_mut(&edge.target) {
                        *entry -= 1;
                        if *entry == 0 {
                            queue.push_back(edge.target.clone());
                        }
                    }
                }
            }
        }

        if sorted.len() == self.nodes.len() {
            Ok(sorted)
        } else {
            let cycle_nodes = in_degree
                .into_iter()
                .filter(|(_, deg)| *deg > 0)
                .map(|(id, _)| id)
                .collect();
            Err(cycle_nodes)
        }
    }

    /// Topological sort over Structural edges only.
    pub fn topo_sort_structural(&self) -> Result<Vec<String>, Vec<String>> {
        use std::collections::VecDeque;
        let mut in_degree = HashMap::new();
        for node in self.nodes.values() {
            in_degree.entry(node.id.clone()).or_insert(0);
            for e in &node.edges {
                if e.kind == EdgeKind::Structural {
                    *in_degree.entry(e.target.clone()).or_insert(0) += 1;
                }
            }
        }
        let mut q: VecDeque<String> = in_degree.iter().filter(|(_, d)| **d == 0).map(|(id, _)| id.clone()).collect();
        let mut out = Vec::new();
        while let Some(id) = q.pop_front() {
            out.push(id.clone());
            if let Some(node) = self.nodes.get(&id) {
                for e in &node.edges {
                    if e.kind != EdgeKind::Structural { continue; }
                    if let Some(d) = in_degree.get_mut(&e.target) {
                        *d -= 1;
                        if *d == 0 { q.push_back(e.target.clone()); }
                    }
                }
            }
        }
        if out.len() == in_degree.len() { Ok(out) } else {
            let cyc = in_degree.into_iter().filter(|(_, d)| *d > 0).map(|(id, _)| id).collect();
            Err(cyc)
        }
    }

    /// Detect cycles via DFS.
    pub fn has_cycle(&self) -> bool {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();

        for node_id in self.nodes.keys() {
            if self.dfs_cycle(node_id, &mut visited, &mut stack) {
                return true;
            }
        }
        false
    }

    /// DFS cycle detection over Structural edges only.
    pub fn has_cycle_structural(&self) -> bool {
        fn dfs(g: &DirectedAcyclicGraphAnalyzer, id: &str, vis: &mut HashSet<String>, stack: &mut HashSet<String>) -> bool {
            if !vis.insert(id.to_string()) { return false; }
            stack.insert(id.to_string());
            if let Some(node) = g.nodes.get(id) {
                for e in &node.edges {
                    if e.kind != EdgeKind::Structural { continue; }
                    let tid = &e.target;
                    if !vis.contains(tid) && dfs(g, tid, vis, stack) { return true; }
                    else if stack.contains(tid) { return true; }
                }
            }
            stack.remove(id);
            false
        }
        let mut vis = HashSet::new();
        let mut stack = HashSet::new();
        for id in self.nodes.keys() {
            if dfs(self, id, &mut vis, &mut stack) { return true; }
        }
        false
    }

    fn dfs_cycle(
        &self,
        node_id: &str,
        visited: &mut HashSet<String>,
        stack: &mut HashSet<String>,
    ) -> bool {
        if !visited.insert(node_id.to_string()) {
            return false;
        }
        stack.insert(node_id.to_string());
        if let Some(node) = self.nodes.get(node_id) {
            for edge in &node.edges {
                let tid = &edge.target;
                if !visited.contains(tid) && self.dfs_cycle(tid, visited, stack) {
                    return true;
                } else if stack.contains(tid) {
                    return true;
                }
            }
        }
        stack.remove(node_id);
        false
    }

    /// Export as Graphviz/DOT for debugging.
    /// - reachable_only: if true, restrict to nodes reachable from entry_point.
    /// - include_data: if false, only structural edges are exported.
    pub fn to_dot(&self, reachable_only: bool, include_data: bool) -> String {
        let mut out = String::from("digraph G {\n  rankdir=TB;\n");
        let include: HashSet<String> = if reachable_only {
            self.build_from_entry()
        } else {
            self.nodes.keys().cloned().collect()
        };
        for (id, node) in &self.nodes {
            if !include.contains(id) { continue; }
            out.push_str(&format!("  \"{}\";\n", id));
            for e in &node.edges {
                if !include.contains(&e.target) { continue; }
                if !include_data && e.kind != EdgeKind::Structural { continue; }
                let (color, style) = match e.kind {
                    EdgeKind::Structural => ("black", "solid"),
                    EdgeKind::Data => ("#1f77b4", "dashed"),
                };
                out.push_str(&format!(
                    "  \"{}\" -> \"{}\" [color=\"{}\", style=\"{}\"];\n",
                    e.source, e.target, color, style
                ));
            }
        }
        out.push_str("}\n");
        out
    }

    /// Export as Graphviz/DOT for debugging (Structural edges only).
    /// - reachable_only: if true, restrict to nodes reachable from entry_point.
    pub fn to_dot_structural(&self, reachable_only: bool) -> String {
        self.to_dot(reachable_only, false)
    }

    /// Builds the compact index-based representation of the DAG.
    pub fn build_compact(&self) -> CompactDag {
        let mut node_ids: Vec<String> = self.nodes.keys().cloned().collect();
        node_ids.sort(); // deterministic
        let id_to_index: HashMap<String, usize> =
            node_ids.iter().enumerate().map(|(i, id)| (id.clone(), i)).collect();

        let mut out_edges: Vec<Vec<(usize, EdgeKind)>> = vec![Vec::new(); node_ids.len()];
        let mut in_edges: Vec<Vec<(usize, EdgeKind)>> = vec![Vec::new(); node_ids.len()];

        for id in &node_ids {
            let idx = id_to_index[id];
            if let Some(node) = self.nodes.get(id) {
                for e in &node.edges {
                    if let Some(ti) = id_to_index.get(&e.target) {
                        out_edges[idx].push((*ti, e.kind));
                    }
                }
                for e in &node.incoming {
                    if let Some(si) = id_to_index.get(&e.source) {
                        in_edges[idx].push((*si, e.kind));
                    }
                }
            }
        }
        for edges in &mut out_edges {
            edges.sort_by_key(|(ti, kind)| (*ti, *kind as u8));
        }
        for edges in &mut in_edges {
            edges.sort_by_key(|(si, kind)| (*si, *kind as u8));
        }

        CompactDag {
            node_ids,
            id_to_index,
            out_edges,
            in_edges,
        }
    }
}
