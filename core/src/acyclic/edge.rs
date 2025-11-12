//! file: ./acyclic/edge.rs
//! Lightweight DAG edges (store IDs only) with an explicit kind.

/// Kind of DAG edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    /// Structural parent -> child (AST tree shape)
    Structural,
    /// Data dependency producer -> consumer (values used by Assignments/Calls)
    Data,
}

/// Represents an edge in a directed acyclic graph (DAG).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AcyclicEdge {
    /// The source node id of the edge.
    pub source: String,
    /// The target node id of the edge.
    pub target: String,
    /// The kind of the edge.
    pub kind: EdgeKind,
}

impl AcyclicEdge {
    pub fn new_with_kind<S: Into<String>, T: Into<String>>(source: S, target: T, kind: EdgeKind) -> Self {
        Self { source: source.into(), target: target.into(), kind }
    }
    pub fn new_structural<S: Into<String>, T: Into<String>>(source: S, target: T) -> Self {
        Self::new_with_kind(source, target, EdgeKind::Structural)
    }
    pub fn new_data<S: Into<String>, T: Into<String>>(source: S, target: T) -> Self {
        Self::new_with_kind(source, target, EdgeKind::Data)
    }
    pub fn source_id(&self) -> &str { &self.source }
    pub fn target_id(&self) -> &str { &self.target }
    pub fn is_between_ids(&self, source_id: &str, target_id: &str) -> bool {
        self.source == source_id && self.target == target_id
    }
    pub fn is_self_loop(&self) -> bool { self.source == self.target }
    pub fn reversed(&self) -> AcyclicEdge {
        AcyclicEdge::new_with_kind(self.target.clone(), self.source.clone(), self.kind)
    }
}

use std::fmt;
impl fmt::Display for AcyclicEdge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let k = match self.kind { EdgeKind::Structural => "struct", EdgeKind::Data => "data" };
        write!(f, "{} -[{}]-> {}", self.source, k, self.target)
    }
}
