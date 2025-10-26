//! file: ./acyclic/analyzer.rs
//! description: Module for analyzing directed acyclic graphs (DAGs).
//!
//! This module provides functionality to analyze DAGs, including detecting cycles and performing topological sorting.
//!
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025/10/25
//! license: See LICENSE file in repository root.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    acyclic::{AcyclicEdge, node::*},
    parser::{AstNode, AstParser},
};

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
    /// # Arguments
    /// * `entry_point` - The entry point node for the DAG.
    /// * `nodes` - A vector of all nodes in the graph.
    pub fn new(ast: AstParser, entry_point: Option<AstNode>) -> Result<Self, ()> {
        let mut nodes = HashMap::new();

        // Helper to recursively walk the AST and build nodes/edges
        fn walk_ast(
            ast_node: &AstNode,
            nodes: &mut HashMap<String, AcyclicNode>,
            parent_id: Option<String>,
        ) {
            // Copy the node ID from the AST node for mutability
            let node_id = ast_node.id.clone();

            // Ensure the node exists
            if !nodes.contains_key(&node_id) {
                nodes.insert(
                    node_id.clone(),
                    AcyclicNode {
                        id: node_id.clone(),
                        edges: Vec::new(),
                    },
                );
            }

            // If there is a parent, add an edge from parent to this node
            if let Some(pid) = parent_id {
                // Avoid holding two mutable borrows at once:
                let edge = {
                    let parent = nodes.get(&pid).cloned();
                    let node = nodes.get(&node_id).cloned();
                    match (parent, node) {
                        (Some(mut parent), Some(node)) => {
                            parent.edges.push(AcyclicEdge {
                                source: parent.clone(),
                                target: node.clone(),
                            });
                            Some((pid, parent))
                        }
                        _ => None,
                    }
                };
                // Write back the updated parent if needed
                if let Some((pid, parent)) = edge {
                    nodes.insert(pid, parent);
                }
            }

            // Recurse for children
            for child in &ast_node.children {
                walk_ast(child, nodes, Some(node_id.clone()));
            }
        }

        // Start from the root of the AST
        let root = ast.root();
        walk_ast(&root, &mut nodes, None);

        Ok(Self {
            entry_point: entry_point.map(|n| n.id),
            nodes,
        })
    }

    /// Builds the reachable subgraph from the entry point.
    /// # Returns
    /// `HashSet<String>` - A set of node IDs that are reachable from the entry point.
    pub fn build_from_entry(&self) -> HashSet<String> {
        let mut visited = HashSet::new();
        if let Some(root_id) = &self.entry_point {
            self.dfs(root_id, &mut visited);
        }
        visited
    }

    /// Depth-first search helper function.
    /// # Arguments
    /// * `node_id` - The ID of the node to start the DFS from.
    /// * `visited` - A mutable reference to a set of visited node IDs.
    fn dfs(&self, node_id: &str, visited: &mut HashSet<String>) {
        if !visited.insert(node_id.to_string()) {
            return;
        }
        if let Some(node) = self.nodes.get(node_id) {
            for edge in &node.edges {
                self.dfs(&edge.target.id, visited);
            }
        }
    }

    /// Topological sort (Kahn's algorithm).
    /// # Returns
    /// * `Ok(sorted list)` - A list of node IDs in topological order.
    /// * `Err(cycle nodes)` - A list of node IDs that are part of a cycle.
    pub fn topo_sort(&self) -> Result<Vec<String>, Vec<String>> {
        // Compute in-degree for each node
        let mut in_degree = HashMap::new();
        for node in self.nodes.values() {
            in_degree.entry(node.id.clone()).or_insert(0);
            for edge in &node.edges {
                *in_degree.entry(edge.target.id.clone()).or_insert(0) += 1;
            }
        }

        // Queue of nodes with in-degree 0
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
                    let entry = in_degree.get_mut(&edge.target.id).unwrap();
                    *entry -= 1;
                    if *entry == 0 {
                        queue.push_back(edge.target.id.clone());
                    }
                }
            }
        }

        if sorted.len() == self.nodes.len() {
            Ok(sorted)
        } else {
            // Cycle detected: return nodes with nonzero in-degree
            let cycle_nodes = in_degree
                .into_iter()
                .filter(|(_, deg)| *deg > 0)
                .map(|(id, _)| id)
                .collect();
            Err(cycle_nodes)
        }
    }

    /// Detects if there is a cycle in the DAG (DFS-based).
    /// # Returns
    /// `true` if a cycle is detected, `false` otherwise.
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

    /// Helper function for cycle detection using DFS.
    /// # Arguments
    /// * `node_id` - The current node ID being visited.
    /// * `visited` - A set of visited node IDs.
    /// * `stack` - A set of node IDs in the current recursion stack.
    /// # Returns
    /// * `true` if a cycle is detected, `false` otherwise.
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
                let tid = &edge.target.id;
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
}
