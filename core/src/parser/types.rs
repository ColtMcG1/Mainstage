//! ./parser/types.rs
//! 
//! Defines types used in the parsing module, including AST node types.
//! This module provides the `AstType` enum which represents various kinds of AST nodes.
//!
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-18

use std::borrow::Cow;

/// Represents the different types of AST nodes in the parser.
/// This enum defines the various kinds of nodes that can appear in the AST,
/// including top-level constructs, scopes, expressions, declarations, and values.
#[derive(Debug, Clone, PartialEq)]
pub enum AstType<'a> {
    // --- Top Level ---
    /// Represents the root of a script.
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
    Stage { name: Cow<'a, str> },
    /// Represents a `task` block with a name.
    Task { name: Cow<'a, str> },

    // --- Expressions ---
    /// Represents an assignment expression (e.g., `x = 5`).
    Assignment,

    // --- Declarations ---
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
            | AstType::Stage { name }
            | AstType::Task { name }
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
            AstType::Stage { name } => AstType::Stage { name: Cow::Owned(name.into_owned()) },
            AstType::Task { name } => AstType::Task { name: Cow::Owned(name.into_owned()) },
            AstType::Assignment => AstType::Assignment,
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
            AstType::Stage { name } => AstType::Stage { name: Cow::Owned(name.into_owned()) },
            AstType::Task { name } => AstType::Task { name: Cow::Owned(name.into_owned()) },
            AstType::Assignment => AstType::Assignment,
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
        }
    }
}