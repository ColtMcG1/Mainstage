//! file: ./acyclic/node.rs
//! description: Module for representing nodes in a directed acyclic graph (DAG).
//! 
//! This module provides functionality to create and manage nodes within a DAG structure.
//! 
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025/10/25
//! license: See LICENSE file in repository root.

use crate::acyclic::edge::*;

/// Represents a node in a directed acyclic graph (DAG).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AcyclicNode {
    /// The unique identifier of the node.
    pub id: String,
    /// The edges connected to this node.
    pub edges: Vec<AcyclicEdge>,
}

impl AcyclicNode {
    /// Creates a new `Node`.
    /// # Examples
    /// ```
    /// let node = Node::new("node_id".to_string());
    /// ```
    pub fn new(id: String) -> Self {
        Self { id, edges: Vec::new() }
    }

    /// Adds an edge from this node to the target node.
    /// # Arguments
    /// * `target` - The target node to connect to.
    pub fn add_edge(&mut self, target: AcyclicNode) {
        let edge = AcyclicEdge::new(self.clone(), target);
        self.edges.push(edge);
    }

    /// Removes an edge from this node to the target node.
    /// # Arguments
    /// * `target_id` - The ID of the target node to remove the edge to.
    pub fn remove_edge(&mut self, target_id: &str) {
        self.edges.retain(|e| e.target.id != target_id);
    }

    /// Checks if there is an edge from this node to the given target ID.
    /// # Arguments
    /// * `target_id` - The ID of the target node to check for an edge to.
    /// # Returns
    /// `true` if there is an edge to the target node, `false` otherwise.
    pub fn has_edge_to(&self, target_id: &str) -> bool {
        self.edges.iter().any(|e| e.target.id == target_id)
    }

    /// Checks if this node has a self-loop (should not happen in a DAG).
    /// # Returns
    /// `true` if there is a self-loop, `false` otherwise.
    pub fn has_self_loop(&self) -> bool {
        self.edges.iter().any(|e| e.target.id == self.id)
    }

    /// Returns the out-degree (number of outgoing edges).
    /// # Returns
    /// The number of outgoing edges.
    pub fn out_degree(&self) -> usize {
        self.edges.len()
    }

    /// Returns an iterator over the IDs of neighboring nodes.
    /// # Returns
    /// An iterator over the IDs of neighboring nodes.
    pub fn neighbors(&self) -> impl Iterator<Item = &String> {
        self.edges.iter().map(|e| &e.target.id)
    }

    /// Prints the edges of the node for debugging purposes.
    pub fn print_edges(&self) {
        for edge in &self.edges {
            println!("{} -> {}", edge.source.id, edge.target.id);
        }
    }
}

use std::fmt;
impl fmt::Display for AcyclicNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Node({})", self.id)?;
        writeln!(f, "Edges:")?;
        for edge in &self.edges {
            writeln!(f, "\t| {} -> {}", edge.source.id, edge.target.id)?;
        }
        Ok(())
    }
}