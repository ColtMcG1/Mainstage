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
    /// The edges connected to this node (outgoing).
    pub edges: Vec<AcyclicEdge>,
    /// The edges connected to this node (incoming).
    pub incoming: Vec<AcyclicEdge>,
}

impl AcyclicNode {
    /// Creates a new `Node`.
    pub fn new(id: String) -> Self {
        Self { id, edges: Vec::new(), incoming: Vec::new() }
    }

    pub fn push_out(&mut self, edge: AcyclicEdge) {
        if !self.edges.iter().any(|e| e.target == edge.target && e.kind == edge.kind) {
            self.edges.push(edge);
        }
    }
    pub fn push_in(&mut self, edge: AcyclicEdge) {
        if !self.incoming.iter().any(|e| e.source == edge.source && e.kind == edge.kind) {
            self.incoming.push(edge);
        }
    }

    /// Adds an edge to target with kind, deduping if already present.
    pub fn add_edge_to_with_kind<S: Into<String>>(&mut self, target_id: S, kind: EdgeKind) {
        let target = target_id.into();
        if !self.edges.iter().any(|e| e.target == target && e.kind == kind) {
            self.edges.push(AcyclicEdge::new_with_kind(self.id.clone(), target, kind));
        }
    }

    /// Adds a structural edge from this node to the target node id.
    pub fn add_structural_edge_to<S: Into<String>>(&mut self, target_id: S) {
        self.push_out(AcyclicEdge::new_structural(self.id.clone(), target_id.into()));
    }

    /// Adds a data edge from this node to the target node id.
    pub fn add_data_edge_to<S: Into<String>>(&mut self, target_id: S) {
        self.push_out(AcyclicEdge::new_data(self.id.clone(), target_id.into()));
    }

    /// Back-compat helper: add edge given a target node (structural).
    pub fn add_edge(&mut self, target: AcyclicNode) {
        self.add_structural_edge_to(target.id);
    }

    /// Removes an edge from this node to the target node.
    pub fn remove_edge(&mut self, target_id: &str) {
        self.edges.retain(|e| e.target != target_id);
    }

    /// Checks if there is an edge from this node to the given target ID.
    pub fn has_edge_to(&self, target_id: &str) -> bool {
        self.edges.iter().any(|e| e.target == target_id)
    }

    /// Checks if this node has a self-loop (should not happen in a DAG).
    pub fn has_self_loop(&self) -> bool {
        self.edges.iter().any(|e| e.target == self.id)
    }

    /// Returns the out-degree (number of outgoing edges).
    pub fn out_degree(&self) -> usize { self.edges.len() }

    /// Returns the in-degree (number of incoming edges).
    pub fn in_degree(&self) -> usize { self.incoming.len() }

    /// Returns an iterator over the IDs of neighboring nodes.
    pub fn neighbors(&self) -> impl Iterator<Item = &String> {
        self.edges.iter().map(|e| &e.target)
    }

    /// Returns an iterator over the sources of incoming edges.
    pub fn incoming_sources(&self) -> impl Iterator<Item = &String> {
        self.incoming.iter().map(|e| &e.source)
    }

    /// Prints the edges of the node for debugging purposes.
    pub fn print_edges(&self) {
        for edge in &self.edges {
            println!("{}", edge);
        }
    }
}

use std::fmt;
impl fmt::Display for AcyclicNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Node({})", self.id)?;
        writeln!(f, "Edges:")?;
        for edge in &self.edges {
            writeln!(f, "\t| {}", edge)?;
        }
        Ok(())
    }
}