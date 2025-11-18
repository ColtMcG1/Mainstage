//! ./parser/types.rs
//! 
//! Defines types used in the parsing module, including AST node types.
//! This module provides the `AstType` enum which represents various kinds of AST nodes.
//!
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-18
//! license: See LICENSE file in the root directory

use std::borrow::Cow;

/// Represents binary operators in the AST.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
}

/// Represents unary operators in the AST.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Inc,
    Dec,
    Plus,
    Minus,
}

/// Represents the different types of AST nodes in the parser.
/// This enum defines the various kinds of nodes that can appear in the AST,
/// including top-level constructs, scopes, expressions, declarations, and values.
#[derive(Debug, Clone)]
pub enum AstType<'a> {
    // Top-level
    Script,

    // Declarations
    Workspace { name: Cow<'a, str> },
    Project   { name: Cow<'a, str> },
    Stage     { name: Cow<'a, str>, params: Vec<super::ast::AstNode<'a>> },
    Task      { name: Cow<'a, str>, params: Vec<super::ast::AstNode<'a>> },

    // Blocks
    Block,

    // Statements
    Return,
    Assignment,
    Include   { path: Cow<'a, str> },
    Import    { path: Cow<'a, str>, alias: Cow<'a, str> },

    // Loops
    While { 
        cond: Box<super::ast::AstNode<'a>>, 
        body: Box<super::ast::AstNode<'a>> 
    },
    Forto {
        init: Box<super::ast::AstNode<'a>>, // Assignment node x = ...
        limt: Box<super::ast::AstNode<'a>>, // Expression node ...
        body: Box<super::ast::AstNode<'a>>, // Body block { ... }
    },
    Forin {
        iden: Cow<'a, str>,
        iter: Box<super::ast::AstNode<'a>>,
        body: Box<super::ast::AstNode<'a>>,
    },

    // Expressions
    Identifier { name: Cow<'a, str> },
    Integer    { value: i64 },
    Float      { value: f64 },
    Str        { value: Cow<'a, str> },
    ShellCmd   { shell: Cow<'a, str>, command: Cow<'a, str> },
    Bool       { value: bool },
    Array,
    Call { target: Box<super::ast::AstNode<'a>>, arguments: Vec<super::ast::AstNode<'a>> },
    Member  { target: Box<super::ast::AstNode<'a>>, member: Box<super::ast::AstNode<'a>> },
    Index         { target: Box<super::ast::AstNode<'a>>, index: Box<super::ast::AstNode<'a>> },
    UnaryOp       { op: UnaryOperator, expr: Box<super::ast::AstNode<'a>>, prefix: bool },
    BinaryOp      { op: BinaryOperator, left: Box<super::ast::AstNode<'a>>, right: Box<super::ast::AstNode<'a>> },

    // Other
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
            AstType::Str { .. }
                | AstType::Integer { .. }
                | AstType::Float { .. }
                | AstType::Bool { .. }
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
            AstType::Import { path, alias } => AstType::Import { path: Cow::Owned(path.into_owned()), alias: Cow::Owned(alias.into_owned()) },
            AstType::Workspace { name } => AstType::Workspace { name: Cow::Owned(name.into_owned()) },
            AstType::Project { name } => AstType::Project { name: Cow::Owned(name.into_owned()) },
            AstType::Stage { name, params } => AstType::Stage { name: Cow::Owned(name.into_owned()), params: params.into_iter().map(|p| p.into_lifetime()).collect() },
            AstType::Task  { name, params } => AstType::Task  { name: Cow::Owned(name.into_owned()), params: params.into_iter().map(|p| p.into_lifetime()).collect() },
            AstType::Block => AstType::Block,
            AstType::Assignment => AstType::Assignment,
            AstType::BinaryOp { op, left, right } => AstType::BinaryOp { op, left: Box::new(left.into_lifetime()), right: Box::new(right.into_lifetime()) },
            AstType::UnaryOp { op, expr, prefix } => AstType::UnaryOp { op, expr: Box::new(expr.into_lifetime()), prefix },
            AstType::Index { target, index } => AstType::Index { target: Box::new(target.into_lifetime()), index: Box::new(index.into_lifetime()) },
            AstType::Member { target, member } => AstType::Member { target: Box::new(target.into_lifetime()), member: Box::new(member.into_lifetime()) },
            AstType::Call { target, arguments } => AstType::Call { target: Box::new(target.into_lifetime()), arguments: arguments.into_iter().map(|a| a.into_lifetime()).collect() },
            AstType::Forto { init, limt, body } => AstType::Forto {
                init: Box::new(init.into_lifetime()),
                limt: Box::new(limt.into_lifetime()),
                body: Box::new(body.into_lifetime()),
            },
            AstType::Forin { iden, iter, body } => AstType::Forin {
                iden: Cow::Owned(iden.into_owned()),
                iter: Box::new(iter.into_lifetime()),
                body: Box::new(body.into_lifetime()),
            },
            AstType::While { cond, body } => AstType::While {
                cond: Box::new(cond.into_lifetime()),
                body: Box::new(body.into_lifetime()),
            },
            AstType::Identifier { name } => AstType::Identifier { name: Cow::Owned(name.into_owned()) },
            AstType::ShellCmd { shell, command } => AstType::ShellCmd { shell: Cow::Owned(shell.into_owned()), command: Cow::Owned(command.into_owned()) },
            AstType::Str { value } => AstType::Str { value: Cow::Owned(value.into_owned()) },
            AstType::Integer { value } => AstType::Integer { value },
            AstType::Float { value } => AstType::Float { value },
            AstType::Bool { value } => AstType::Bool { value },
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
            AstType::Import { path, alias } => AstType::Import { path: Cow::Owned(path.into_owned()), alias: Cow::Owned(alias.into_owned()) },
            AstType::Workspace { name } => AstType::Workspace { name: Cow::Owned(name.into_owned()) },
            AstType::Project { name } => AstType::Project { name: Cow::Owned(name.into_owned()) },
            AstType::Stage { name, params } => AstType::Stage { name: Cow::Owned(name.into_owned()), params: params.into_iter().map(|p| p.into_owned()).collect() },
            AstType::Task  { name, params } => AstType::Task  { name: Cow::Owned(name.into_owned()), params: params.into_iter().map(|p| p.into_owned()).collect() },
            AstType::Block => AstType::Block,
            AstType::Assignment => AstType::Assignment,
            AstType::BinaryOp { op, left, right } => AstType::BinaryOp { op, left: Box::new(left.into_owned()), right: Box::new(right.into_owned()) },
            AstType::UnaryOp { op, expr, prefix } => AstType::UnaryOp { op, expr: Box::new(expr.into_owned()), prefix },
            AstType::Index { target, index } => AstType::Index { target: Box::new(target.into_owned()), index: Box::new(index.into_owned()) },
            AstType::Member { target, member } => AstType::Member { target: Box::new(target.into_owned()), member: Box::new(member.into_owned()) },
            AstType::Call { target, arguments } => AstType::Call { target: Box::new(target.into_owned()), arguments: arguments.into_iter().map(|a| a.into_owned()).collect() },
            AstType::Forto { init, limt, body } => AstType::Forto {
                init: Box::new(init.into_owned()),
                limt: Box::new(limt.into_owned()),
                body: Box::new(body.into_owned()),
            },
            AstType::Forin { iden, iter, body } => AstType::Forin {
                iden: Cow::Owned(iden.into_owned()),
                iter: Box::new(iter.into_owned()),
                body: Box::new(body.into_owned()),
            },
            AstType::While { cond, body } => AstType::While {
                cond: Box::new(cond.into_owned()),
                body: Box::new(body.into_owned()),
            },
            AstType::Identifier { name } => AstType::Identifier { name: Cow::Owned(name.into_owned()) },
            AstType::ShellCmd { shell, command } => AstType::ShellCmd { shell: Cow::Owned(shell.into_owned()), command: Cow::Owned(command.into_owned()) },
            AstType::Str { value } => AstType::Str { value: Cow::Owned(value.into_owned()) },
            AstType::Integer { value } => AstType::Integer { value },
            AstType::Float { value } => AstType::Float { value },
            AstType::Bool { value } => AstType::Bool { value },
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
            (AstType::Identifier { name: n1 }, AstType::Identifier { name: n2 }) => n1 == n2,
            (AstType::ShellCmd { shell: s1, command: c1 }, AstType::ShellCmd { shell: s2, command: c2 }) => s1 == s2 && c1 == c2,
            (AstType::Str { value: v1 }, AstType::Str { value: v2 }) => v1 == v2,
            (AstType::Integer { value: i1 }, AstType::Integer { value: i2 }) => i1 == i2,
            (AstType::Float { value: f1 }, AstType::Float { value: f2 }) => f1 == f2,
            (AstType::Bool { value: b1 }, AstType::Bool { value: b2 }) => b1 == b2,
            (AstType::Array, AstType::Array) => true,
            (AstType::Null, AstType::Null) => true,
            (AstType::Return, AstType::Return) => true,
            (AstType::Block, AstType::Block) => true,
            (AstType::BinaryOp { op: o1, left: l1, right: r1 }, AstType::BinaryOp { op: o2, left: l2, right: r2 }) => o1 == o2 && l1 == l2 && r1 == r2,
            (AstType::UnaryOp { op: o1, expr: e1, prefix: p1 }, AstType::UnaryOp { op: o2, expr: e2, prefix: p2 }) => o1 == o2 && e1 == e2 && p1 == p2,
            (AstType::Index { target: t1, index: i1 }, AstType::Index { target: t2, index: i2 }) => t1 == t2 && i1 == i2,
            (AstType::Member { target: t1, member: m1 }, AstType::Member { target: t2, member: m2 }) => t1 == t2 && m1 == m2,
            (AstType::Call { target: t1, arguments: a1 }, AstType::Call { target: t2, arguments: a2 }) => t1 == t2 && a1 == a2,
            (AstType::Forto { init: i1, limt: l1, body: b1 }, AstType::Forto { init: i2, limt: l2, body: b2 }) => i1 == i2 && l1 == l2 && b1 == b2,
            (AstType::Forin { iden: v1, iter: i1, body: b1 }, AstType::Forin { iden: v2, iter: i2, body: b2 }) => v1 == v2 && i1 == i2 && b1 == b2,
            (AstType::While { cond: c1, body: b1 }, AstType::While { cond: c2, body: b2 }) => c1 == c2 && b1 == b2,
            _ => false,
        }
    }
}