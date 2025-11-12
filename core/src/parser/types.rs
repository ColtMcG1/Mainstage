//! ./parser/types.rs
//! 
//! Defines types used in the parsing module, including AST node types.
//! This module provides the `AstType` enum which represents various kinds of AST nodes.
//!
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-18
//! license: See LICENSE file in the root directory

use std::borrow::Cow;

use crate::parser::AstNode;

/// Represents binary operators in the AST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryOperator {
    Add, Sub, Mul, Div,
    Eq, Neq,
    Lt, Gt, Le, Ge,
}

/// Represents unary operators in the AST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnaryOperator {
    Plus, Minus, Inc, Dec,
}

/// Represents the different types of AST nodes in the parser.
/// This enum defines the various kinds of nodes that can appear in the AST,
/// including top-level constructs, scopes, expressions, declarations, and values.
#[derive(Debug, Clone)]
pub enum AstType<'a> {
    // --- Top Level ---
    Script,
    /// Represents an `include` directive with a file path.
    Include { path: Cow<'a, str> },
    /// Represents an `import` directive with a file path and an alias.
    Import { path: Cow<'a, str>, alias: Cow<'a, str> },

    // --- Scopes ---
    /// Represents a `workspace` block with a name.
    Workspace { name: Cow<'a, str> },
    /// Represents a `project` block with a name.
    Project { name: Cow<'a, str> },
    /// Represents a `stage` block with a name.
    Stage { name: Cow<'a, str>, params: Vec<AstNode<'a>> },
    /// Represents a `task` block with a name.
    Task  { name: Cow<'a, str>, params: Vec<AstNode<'a>> },

    // --- Expressions / Statements ---
    /// Represents an assignment expression (e.g., `x = 5`).
    Assignment,
    /// Represents a binary operation (e.g., `a + b`).
    BinaryOp {
        op: BinaryOperator,
        left: Box<AstNode<'a>>,
        right: Box<AstNode<'a>>,
    },
    /// Represents a unary operation (e.g., `-a` or `++a`).
    UnaryOp {
        op: UnaryOperator,
        expr: Box<AstNode<'a>>,
        prefix: bool,          // true if prefix (++x), false if postfix (x++)
    },
    /// Represents an index access (e.g., `array[index]`).
    Index {
        target: Box<AstNode<'a>>,
        index: Box<AstNode<'a>>,
    },
    /// Represents a member access expression (e.g., `object.property`).
    MemberAccess {
        target: Box<AstNode<'a>>,
        member: Box<AstNode<'a>>,
    },
    /// Represents a call to execute an object (e.g., `object(args?)`)
    CallExpression {
        target: Box<AstNode<'a>>,
        arguments: Vec<AstNode<'a>>,
    },

    /// Represents a return statement.
    Return,

    /// Represents a variable declaration (e.g., `let x = 5`).
    VariableDeclaration { name: Cow<'a, str>, value: Option<Cow<'a, str>> },

    // --- Values ---
    /// Represents an identifier (e.g., a variable name).
    Identifier { name: Cow<'a, str> },
    /// Represents a shell command (e.g., `bash -c "echo Hello"`).
    ShellCommand { shell: Cow<'a, str>, command: Cow<'a, str> },
    /// Represents a string value.
    String { value: Cow<'a, str> },
    /// Represents a numeric value.
    Number { value: f64 },
    /// Represents a boolean value (`true` or `false`).
    Boolean { value: bool },
    /// Represents an array value.
    Array,
    /// Represents a `null` value.
    Null,
}

impl<'a> AstType<'a> {

    /// Returns `true` if the node is a scope (e.g., `Workspace`, `Project`, `Stage`, or `Task`).
    /// # Returns
    /// * `true` if the node is a scope, otherwise `false`.
    pub fn is_scope(&self) -> bool {
        matches!(
            self,
            AstType::Workspace { .. }
                | AstType::Project { .. }
                | AstType::Stage { .. }
                | AstType::Task { .. }
        )
    }

    /// Returns `true` if the node is a value (e.g., `String`, `Number`, `Boolean`, or `Null`).
    /// # Returns
    /// * `true` if the node is a value, otherwise `false`.
    pub fn is_value(&self) -> bool {
        matches!(
            self,
            AstType::String { .. }
                | AstType::Number { .. }
                | AstType::Boolean { .. }
                | AstType::Null
        )
    }

    /// Extracts the name of the node if it has one (e.g., `Workspace`, `Project`, `Stage`, `Task`, or `Identifier`).
    /// # Returns
    /// * `Some(&str)` containing the name if the node has a name, otherwise `None`.
    pub fn name(&self) -> Option<&str> {
        match self {
            AstType::Workspace { name }
            | AstType::Project { name }
            | AstType::Stage { name, .. }
            | AstType::Task { name, .. }
            | AstType::Identifier { name } => Some(name),
            _ => None,
        }
    }

    /// Converts the AST node into a version with a different lifetime.
    /// This is useful for adapting the AST to different lifetime requirements.
    /// # Returns
    /// * An `AstType` instance with the specified lifetime.
    pub fn into_lifetime(self) -> AstType<'static> {
        match self {
            AstType::Script => AstType::Script,
            AstType::Include { path } => AstType::Include { path: Cow::Owned(path.into_owned()) },
            AstType::Import { path, alias } => AstType::Import {
                path: Cow::Owned(path.into_owned()),
                alias: Cow::Owned(alias.into_owned()),
            },
            AstType::Workspace { name } => AstType::Workspace { name: Cow::Owned(name.into_owned()) },
            AstType::Project { name } => AstType::Project { name: Cow::Owned(name.into_owned()) },
            AstType::Stage { name, params } => AstType::Stage { name: Cow::Owned(name.into_owned()), params: params.into_iter().map(|p| p.into_lifetime()).collect() },
            AstType::Task  { name, params } => AstType::Task  { name: Cow::Owned(name.into_owned()), params: params.into_iter().map(|p| p.into_lifetime()).collect() },
            AstType::Assignment => AstType::Assignment,
            AstType::BinaryOp { op, left, right } => AstType::BinaryOp {
                op,
                left: Box::new(left.into_lifetime()),
                right: Box::new(right.into_lifetime()),
            },
            AstType::UnaryOp { op, expr, prefix } => AstType::UnaryOp {
                op,
                expr: Box::new(expr.into_lifetime()),
                prefix,
            },
            AstType::Index { target, index } => AstType::Index {
                target: Box::new(target.into_lifetime()),
                index: Box::new(index.into_lifetime()),
            },
            AstType::MemberAccess { target, member } => AstType::MemberAccess {
                target: Box::new(target.into_lifetime()),
                member: Box::new(member.into_lifetime()),
            },
            AstType::CallExpression { target, arguments } => AstType::CallExpression { 
                target: Box::new(target.into_lifetime()), 
                arguments: arguments.into_iter().map(|arg| arg.into_lifetime()).collect(),
            },
            AstType::VariableDeclaration { name, value } => AstType::VariableDeclaration {
                name: Cow::Owned(name.into_owned()),
                value: value.map(|v| Cow::Owned(v.into_owned())),
            },
            AstType::Identifier { name } => AstType::Identifier { name: Cow::Owned(name.into_owned()) },
            AstType::ShellCommand { shell, command } => AstType::ShellCommand {
                shell: Cow::Owned(shell.into_owned()),
                command: Cow::Owned(command.into_owned()),
            },
            AstType::String { value } => AstType::String { value: Cow::Owned(value.into_owned()) },
            AstType::Number { value } => AstType::Number { value },
            AstType::Boolean { value } => AstType::Boolean { value },
            AstType::Array => AstType::Array,
            AstType::Null => AstType::Null,
            AstType::Return => AstType::Return,
        }
    }

    /// Converts the AST node into an owned version.
    /// This is useful for ensuring that the AST node owns its data.
    /// # Returns
    /// * An `AstType` instance with owned data.
    pub fn into_owned(self) -> AstType<'static> {
        match self {
            AstType::Script => AstType::Script,
            AstType::Include { path } => AstType::Include { path: Cow::Owned(path.into_owned()) },
            AstType::Import { path, alias } => AstType::Import {
                path: Cow::Owned(path.into_owned()),
                alias: Cow::Owned(alias.into_owned()),
            },
            AstType::Workspace { name } => AstType::Workspace { name: Cow::Owned(name.into_owned()) },
            AstType::Project { name } => AstType::Project { name: Cow::Owned(name.into_owned()) },
            AstType::Stage { name, params } => AstType::Stage { name: Cow::Owned(name.into_owned()), params: params.into_iter().map(|p| p.into_owned()).collect() },
            AstType::Task  { name, params } => AstType::Task  { name: Cow::Owned(name.into_owned()), params: params.into_iter().map(|p| p.into_owned()).collect() },
            AstType::Assignment => AstType::Assignment,
            AstType::BinaryOp { op, left, right } => AstType::BinaryOp {
                op,
                left: Box::new(left.into_owned()),
                right: Box::new(right.into_owned()),
            },
            AstType::UnaryOp { op, expr, prefix } => AstType::UnaryOp {
                op,
                expr: Box::new(expr.into_owned()),
                prefix,
            },
            AstType::Index { target, index } => AstType::Index {
                target: Box::new(target.into_owned()),
                index: Box::new(index.into_owned()),
            },
            AstType::MemberAccess { target, member } => AstType::MemberAccess {
                target: Box::new(target.into_owned()),
                member: Box::new(member.into_owned()),
            },
            AstType::VariableDeclaration { name, value } => AstType::VariableDeclaration {
                name: Cow::Owned(name.into_owned()),
                value: value.map(|v| Cow::Owned(v.into_owned())),
            },
            AstType::CallExpression { target, arguments } => AstType::CallExpression { 
                target: Box::new(target.into_owned()), 
                arguments: arguments.into_iter().map(|arg| arg.into_owned()).collect(),
            },
            AstType::Identifier { name } => AstType::Identifier { name: Cow::Owned(name.into_owned()) },
            AstType::ShellCommand { shell, command } => AstType::ShellCommand {
                shell: Cow::Owned(shell.into_owned()),
                command: Cow::Owned(command.into_owned()),
            },
            AstType::String { value } => AstType::String { value: Cow::Owned(value.into_owned()) },
            AstType::Number { value } => AstType::Number { value },
            AstType::Boolean { value } => AstType::Boolean { value },
            AstType::Array => AstType::Array,
            AstType::Null => AstType::Null,
            AstType::Return => AstType::Return,
        }
    }
}

/// Implements equality comparison for `AstType`.
impl<'a> PartialEq for AstType<'a> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (AstType::Script, AstType::Script) => true,
            (AstType::Include { path: p1 }, AstType::Include { path: p2 }) => p1 == p2,
            (AstType::Import { path: p1, alias: a1 }, AstType::Import { path: p2, alias: a2 }) => p1 == p2 && a1 == a2,
            (AstType::Workspace { name: n1 }, AstType::Workspace { name: n2 }) => n1 == n2,
            (AstType::Project { name: n1 }, AstType::Project { name: n2 }) => n1 == n2,
            (AstType::Stage { name: n1, params: params1 }, AstType::Stage { name: n2, params: params2 }) => n1 == n2 && params1 == params2,
            (AstType::Task  { name: n1, params: params1 }, AstType::Task  { name: n2, params: params2 }) => n1 == n2 && params1 == params2,
            (AstType::Assignment, AstType::Assignment) => true,
            (AstType::VariableDeclaration { name: n1, value: v1 }, AstType::VariableDeclaration { name: n2, value: v2 }) => n1 == n2 && v1 == v2,
            (AstType::Identifier { name: n1 }, AstType::Identifier { name: n2 }) => n1 == n2,
            (AstType::ShellCommand { shell: s1, command: c1 }, AstType::ShellCommand { shell: s2, command: c2 }) => s1 == s2 && c1 == c2,
            (AstType::String { value: v1 }, AstType::String { value: v2 }) => v1 == v2,
            (AstType::Number { value: v1 }, AstType::Number { value: v2 }) => v1 == v2,
            (AstType::Boolean { value: b1 }, AstType::Boolean { value: b2 }) => b1 == b2,
            (AstType::Array, AstType::Array) => true,
            (AstType::Null, AstType::Null) => true,
            (AstType::Return, AstType::Return) => true,
            _ => false,
        }
    }
}