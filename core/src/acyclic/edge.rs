//! file: ./acyclic/edge.rs
//! description: Module for representing edges in a directed acyclic graph (DAG).
//! 
//! This module provides functionality to create and manage edges within a DAG structure.
//! 
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025/10/25
//! license: See LICENSE file in repository root.

use crate::acyclic::node::*;

/// Represents an edge in a directed acyclic graph (DAG).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AcyclicEdge {
    /// The source node of the edge.
    pub source: AcyclicNode,
    /// The target node of the edge.
    pub target: AcyclicNode,
}

impl AcyclicEdge {
    /// Creates a new `Edge`.
    /// # Examples
    /// ```
    /// let edge = Edge::new(source_node, target_node);
    /// ```
    pub fn new(source: AcyclicNode, target: AcyclicNode) -> Self {
        Self { source, target }
    }

    /// Returns the ID of the source node.
    /// # Returns
    /// A string slice representing the ID of the source node.
    pub fn source_id(&self) -> &str {
        &self.source.id
    }
    /// Returns the ID of the target node.
    /// # Returns
    /// A string slice representing the ID of the target node.
    pub fn target_id(&self) -> &str {
        &self.target.id
    }

    /// Checks if the edge is between the given source and target nodes.
    /// # Arguments
    /// * `source` - The source node.
    /// * `target` - The target node.
    /// # Returns
    /// `true` if the edge is between the given source and target nodes, `false` otherwise.
    pub fn is_between(&self, source: &AcyclicNode, target: &AcyclicNode) -> bool {
        &self.source == source && &self.target == target
    }

    /// Checks if the edge is a self-loop (i.e., source and target are the same).
    /// # Returns
    /// `true` if the edge is a self-loop, `false` otherwise.
    pub fn is_self_loop(&self) -> bool {
        self.source == self.target
    }

    /// Returns a new `Edge` that is the reverse of this edge.
    /// # Returns
    /// A new `Edge` with source and target swapped.
    pub fn reversed(&self) -> AcyclicEdge {
        AcyclicEdge::new(self.target.clone(), self.source.clone())
    }
}

use std::fmt;
impl fmt::Display for AcyclicEdge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}", self.source.id, self.target.id)
    }
}
