//! file: core/src/ast/kind.rs
//! description: AST node kind definitions and operator enums.
//!
//! Defines `AstNodeKind` along with `BinaryOperator` and `UnaryOperator`.
//! These enums are used throughout parsing, analysis and lowering stages.
//!
use super::node::AstNode;

/// Represents binary operators in the AST.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Eq,  // ==
    Ne,  // !=
    Lt,  // <
    Le,  // <=
    Gt,  // >
    Ge,  // >=
    Add, // +
    Sub, // -
    Mul, // *
    Div, // /
    Mod, // %
}

/// Represents unary operators in the AST.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Inc,   // ++
    Dec,   // --
    Plus,  // +
    Minus, // -
    Not,   // !
}

#[derive(Debug, Clone, PartialEq)]
pub enum AstNodeKind {
    Script { body: Vec<AstNode> },
    Import { module: String, alias: String },
    Include { file: String },

    Statement,
    Arguments { args: Vec<AstNode> },

    Workspace { name: String, body: Box<AstNode> },
    Project { name: String, body: Box<AstNode> },
    Stage { name: String, args: Option<Box<AstNode>>, body: Box<AstNode> },

    Block { statements: Vec<AstNode> },

    If { condition: Box<AstNode>, body: Box<AstNode> },
    IfElse { condition: Box<AstNode>, if_body: Box<AstNode>, else_body: Box<AstNode> },

    ForIn { iterator: String, iterable: Box<AstNode>, body: Box<AstNode> },
    ForTo { initializer: Box<AstNode>, limit: Box<AstNode>, body: Box<AstNode> },
    While { condition: Box<AstNode>, body: Box<AstNode> },

    UnaryOp { op: UnaryOperator, expr: Box<AstNode> },
    BinaryOp { left: Box<AstNode>, op: BinaryOperator, right: Box<AstNode> },
    Assignment { target: Box<AstNode>, value: Box<AstNode> },

    Command { name: String, arg: String },
    Call { callee: Box<AstNode>, args: Vec<AstNode> },
    Member { object: Box<AstNode>, property: String },
    Index { object: Box<AstNode>, index: Box<AstNode> },
    Return { value: Option<Box<AstNode>> },

    Identifier { name: String },
    String { value: String },
    Integer { value: i64 },
    Float { value: f64 },
    Bool { value: bool },
    List { elements: Vec<AstNode> },
    Null,
}

impl AstNodeKind {
    pub fn is_expression(&self) -> bool {
        matches!(
            self,
            AstNodeKind::UnaryOp { .. }
                | AstNodeKind::BinaryOp { .. }
                | AstNodeKind::Assignment { .. }
                | AstNodeKind::Call { .. }
                | AstNodeKind::Member { .. }
                | AstNodeKind::Index { .. }
                | AstNodeKind::Identifier { .. }
                | AstNodeKind::String { .. }
                | AstNodeKind::Integer { .. }
                | AstNodeKind::Float { .. }
                | AstNodeKind::Bool { .. }
                | AstNodeKind::List { .. }
                | AstNodeKind::Null
        )
    }

    /// If this kind is a container (Workspace or Project), return a reference
    /// to the contained body `AstNode`.
    pub fn container_body(&self) -> Option<&AstNode> {
        match self {
            AstNodeKind::Workspace { body, .. } | AstNodeKind::Project { body, .. } | AstNodeKind::Stage { body, .. } => {
                Some(body.as_ref())
            }
            _ => None,
        }
    }
}

use std::fmt;

impl fmt::Display for AstNodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AstNodeKind::Script { .. } => write!(f, "Script"),
            AstNodeKind::Import { .. } => write!(f, "Import"),
            AstNodeKind::Include { .. } => write!(f, "Include"),
            AstNodeKind::Statement => write!(f, "Statement"),
            AstNodeKind::Arguments { .. } => write!(f, "Arguments"),
            AstNodeKind::Workspace { .. } => write!(f, "Workspace"),
            AstNodeKind::Project { .. } => write!(f, "Project"),
            AstNodeKind::Stage { .. } => write!(f, "Stage"),
            AstNodeKind::Block { .. } => write!(f, "Block"),
            AstNodeKind::If { .. } => write!(f, "If"),
            AstNodeKind::IfElse { .. } => write!(f, "IfElse"),
            AstNodeKind::ForIn { .. } => write!(f, "ForIn"),
            AstNodeKind::ForTo { .. } => write!(f, "ForTo"),
            AstNodeKind::While { .. } => write!(f, "While"),
            AstNodeKind::UnaryOp { .. } => write!(f, "UnaryOp"),
            AstNodeKind::BinaryOp { .. } => write!(f, "BinaryOp"),
            AstNodeKind::Assignment { .. } => write!(f, "Assignment"),
            AstNodeKind::Command { .. } => write!(f, "Command"),
            AstNodeKind::Call { .. } => write!(f, "Call"),
            AstNodeKind::Member { .. } => write!(f, "Member"),
            AstNodeKind::Index { .. } => write!(f, "Index"),
            AstNodeKind::Return { .. } => write!(f, "Return"),
            AstNodeKind::Identifier { .. } => write!(f, "Identifier"),
            AstNodeKind::String { .. } => write!(f, "String"),
            AstNodeKind::Integer { .. } => write!(f, "Integer"),
            AstNodeKind::Float { .. } => write!(f, "Float"),
            AstNodeKind::Bool { .. } => write!(f, "Bool"),
            AstNodeKind::List { .. } => write!(f, "List"),
            AstNodeKind::Null => write!(f, "Null"),
        }
    }
}