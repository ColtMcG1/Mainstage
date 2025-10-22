//! ./semantic/symbol.rs
//!
//! Contains definitions for symbols used in semantic analysis.
//! This module provides the `Symbol` struct and related functionality.
//!
//! author: Colton McGraw <https://github.com/ColtMcG1>
//! date: 2025-10-18
//! license: See LICENSE file in the root directory

use std::borrow::Cow;
use std::collections::HashMap;

use crate::report;

const HOT_PATH_THRESHOLD: usize = 5; // You can make this configurable

// ===== Symbol =====

/// The kind of symbol.
/// Indicates whether the symbol is a variable, function, workspace, project, stage, or task.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum SymbolKind {
    /// A variable symbol.
    Variable,
    /// A function symbol.
    Function,
    /// A workspace symbol.
    Workspace,
    /// A project symbol.
    Project,
    /// A stage symbol.
    Stage,
    /// A task symbol.
    Task,
}

/// The scope of the symbol.
/// Indicates whether the symbol is local, global, or built-in.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum SymbolScope {
    /// A local symbol.
    Local,
    /// A global symbol.
    Global,
    /// A built-in symbol.
    Builtin,
}

/// The type of the symbol.
/// Indicates the data type of the symbol, such as integer, float, string, boolean, or void.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum SymbolType {
    /// No type.
    None,
    /// An integer type.
    Integer,
    /// A float type.
    Float,
    /// A string type.
    String,
    /// An array type.
    Array,
    /// A shell command type.
    ShellCommand,
    /// A boolean type.
    Boolean,
    /// A void type.
    Void,
}

/// Represents a symbol in the semantic analysis.
/// This struct contains information about the symbol's name, kind, type, and scope.
#[derive(PartialEq, Eq, Clone)]
pub struct Symbol<'a> {
    /// The name of the symbol.
    name: Cow<'a, str>,
    /// The kind of the symbol.
    kind: SymbolKind,
    /// The type of the symbol.
    symbol_type: SymbolType,
    /// The scope of the symbol.
    scope: SymbolScope,
    /// Parameter types for functions (if applicable).
    parameters: Vec<SymbolType>,
    /// Return type for functions (if applicable).
    return_type: SymbolType,
    /// Parent symbol for nested scopes (if applicable).
    parent: Option<Box<Symbol<'a>>>,
    // How many references exist to this symbol (for optimization)
    reference_count: usize,
}

impl<'a> Symbol<'a> {
    /// Creates a new `Symbol` instance.
    /// # Arguments
    /// * `name` - The name of the symbol.
    /// * `symbol_kind` - The kind of the symbol.
    /// * `symbol_type` - The type of the symbol.
    /// * `scope` - The scope of the symbol.
    /// # Returns
    /// * A new `Symbol` instance.
    pub fn new(
        name: Cow<'a, str>,
        symbol_kind: SymbolKind,
        symbol_type: SymbolType,
        scope: SymbolScope,
    ) -> Self {
        Self {
            name,
            kind: symbol_kind,
            symbol_type,
            scope,
            parameters: Vec::new(),
            return_type: SymbolType::None,
            parent: None,
            reference_count: 0,
        }
    }

    /// Creates a new variable symbol.
    /// # Arguments
    /// * `name` - The name of the symbol.
    /// * `symbol_type` - The type of the symbol.
    /// * `scope` - The scope of the symbol.
    /// # Returns
    /// * A new variable `Symbol` instance.
    pub fn new_variable(name: Cow<'a, str>, symbol_type: SymbolType, scope: SymbolScope) -> Self {
        Self::new(name, SymbolKind::Variable, symbol_type, scope)
    }

    /// Creates a new function symbol.
    /// # Arguments
    /// * `name` - The name of the symbol.
    /// * `scope` - The scope of the symbol.
    /// # Returns
    /// * A new function `Symbol` instance.
    pub fn new_function(name: Cow<'a, str>, scope: SymbolScope) -> Self {
        Self::new(name, SymbolKind::Function, SymbolType::None, scope)
    }

    /// Creates a new workspace symbol.
    /// # Arguments
    /// * `name` - The name of the symbol.
    /// * `scope` - The scope of the symbol.
    /// # Returns
    /// * A new workspace `Symbol` instance.
    pub fn new_workspace(name: Cow<'a, str>, scope: SymbolScope) -> Self {
        Self::new(name, SymbolKind::Workspace, SymbolType::None, scope)
    }

    /// Creates a new project symbol.
    /// # Arguments
    /// * `name` - The name of the symbol.
    /// * `scope` - The scope of the symbol.
    /// # Returns
    /// * A new project `Symbol` instance.
    pub fn new_project(name: Cow<'a, str>, scope: SymbolScope) -> Self {
        Self::new(name, SymbolKind::Project, SymbolType::None, scope)
    }

    /// Creates a new stage symbol.
    /// # Arguments
    /// * `name` - The name of the symbol.
    /// * `scope` - The scope of the symbol.
    /// # Returns
    /// * A new stage `Symbol` instance.
    pub fn new_stage(name: Cow<'a, str>, scope: SymbolScope) -> Self {
        Self::new(name, SymbolKind::Stage, SymbolType::None, scope)
    }

    /// Creates a new task symbol.
    /// # Arguments
    /// * `name` - The name of the symbol.
    /// * `scope` - The scope of the symbol.
    /// # Returns
    /// * A new task `Symbol` instance.
    pub fn new_task(name: Cow<'a, str>, scope: SymbolScope) -> Self {
        Self::new(name, SymbolKind::Task, SymbolType::None, scope)
    }

    /// Retrieves the name of the symbol.
    /// # Returns
    /// * A string slice containing the symbol's name.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Retrieves the kind of the symbol.
    /// # Returns
    /// * A reference to the symbol's kind.
    pub fn kind(&self) -> &SymbolKind {
        &self.kind
    }
    /// Retrieves the type of the symbol.
    /// # Returns
    /// * A reference to the symbol's type.
    pub fn symbol_type(&self) -> &SymbolType {
        &self.symbol_type
    }
    /// Retrieves the scope of the symbol.
    /// # Returns
    /// * A reference to the symbol's scope.
    pub fn scope(&self) -> &SymbolScope {
        &self.scope
    }
    /// Increments the reference count of the symbol.
    pub fn increment_reference_count(&mut self) {
        self.reference_count += 1;
    }
    /// Retrieves the reference count of the symbol.
    /// # Returns
    /// * The reference count of the symbol.
    pub fn reference_count(&self) -> usize {
        self.reference_count
    }
}

/// Implements the `Debug` trait for the `Symbol` struct.
impl<'a> std::fmt::Debug for Symbol<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Symbol {{ name: {}, kind: {:?}, type: {:?}, scope: {:?} }}",
            self.name, self.kind, self.symbol_type, self.scope
        )
    }
}

// ===== Symbol Table =====

/// Type used to represent the symbol table.
type Scope<'a> = HashMap<String, Vec<Symbol<'a>>>;

/// The symbol table used in semantic analysis.
/// This struct maintains a mapping of symbol names to their corresponding symbols,
/// allowing for scoping by using a stack of symbol tables and overriding.
#[derive(Clone)]
pub struct SymbolTable<'a> {
    pub scopes: Vec<Scope<'a>>,        // Active scopes stack
    pub scope_history: Vec<Scope<'a>>, // All scopes ever entered (for debugging)
}

impl<'a> SymbolTable<'a> {
    /// Creates a new, empty `SymbolTable`.
    /// # Examples
    /// ```
    /// use core::semantic::symbol::{SymbolTable, Symbol};
    /// let mut symbol_table = SymbolTable::new();
    /// ```
    pub fn new() -> Self {
        let global = Scope::new();
        Self {
            scopes: vec![global.clone()],
            scope_history: vec![global],
        }
    }

    /// Emits warnings for symbols with no references.
    /// This helps identify unused symbols in the code.
    /// # Examples
    /// ```
    /// use core::semantic::symbol::{SymbolTable, Symbol};
    /// let mut symbol_table = SymbolTable::new();
    /// ```
    pub fn warn_unused_symbols(&self) {
        for (scope_idx, scope) in self.scopes.iter().enumerate() {
            for symbols in scope.values() {
                for symbol in symbols {
                    if symbol.reference_count() == 0 {
                        report!(
                            report::Level::Warning,
                            format!(
                                "Symbol '{}' in scope {} is never referenced.",
                                symbol.name(),
                                scope_idx
                            ),
                            Some("SemanticAnalyzer".into()),
                            None,
                            None
                        );
                    }
                }
            }
        }
    }

    /// Emits warnings for symbols with high reference counts (hot paths).
    /// This helps identify performance-critical symbols in the code.
    /// # Examples
    /// ```
    /// use core::semantic::symbol::{SymbolTable, Symbol};
    /// let mut symbol_table = SymbolTable::new();
    /// ```
    pub fn warn_hot_paths(&self) {
        for (scope_idx, scope) in self.scopes.iter().enumerate() {
            for symbols in scope.values() {
                for symbol in symbols {
                    if symbol.reference_count() >= HOT_PATH_THRESHOLD {
                        report!(
                            report::Level::Warning,
                            format!(
                                "Symbol '{}' in scope {} is a hot path ({} references).",
                                symbol.name(),
                                scope_idx,
                                symbol.reference_count()
                            ),
                            Some("SemanticAnalyzer".into()),
                            None,
                            None
                        );
                    }
                }
            }
        }
    }

    /// Checks if a symbol exists in any scope.
    /// # Arguments
    /// * `name` - The name of the symbol to check.
    /// # Returns
    /// * `true` if the symbol exists, `false` otherwise.
    pub fn exists(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Retrieves all symbols of a specific kind.
    /// # Arguments
    /// * `kind` - The `SymbolKind` to filter symbols by.
    /// # Returns
    /// * A vector of references to `Symbol` instances with the specified kind.
    pub fn get_by_kind(&self, kind: SymbolKind) -> Vec<&Symbol<'a>> {
        self.scopes
            .iter()
            .flat_map(|scope| {
                scope
                    .iter()
                    .flat_map(|(_, symbols)| symbols.iter().filter(|symbol| symbol.kind == kind))
            })
            .collect()
    }

    /// Retrieves all symbols in the current scope.
    /// # Returns
    /// * An optional reference to a HashMap containing symbol names and their corresponding symbols in the current scope.
    pub fn get_current_scope_symbols(&self) -> Option<&HashMap<String, Vec<Symbol<'a>>>> {
        self.scopes.last().map(|scope| scope)
    }

    /// Checks if a symbol is global.
    /// # Arguments
    /// * `name` - The name of the symbol to check.
    /// # Returns
    /// * `true` if the symbol is global, `false` otherwise.
    pub fn is_global(&self, name: &str) -> bool {
        if let Some(global_scope) = self.scopes.first() {
            global_scope.iter().any(|(n, _)| n == name)
        } else {
            false
        }
    }

    /// Enters a new scope by pushing an empty symbol table onto the stack.
    pub fn enter_scope(&mut self) {
        let new_scope = Scope::new();
        self.scopes.push(new_scope.clone());
        self.scope_history.push(new_scope);
    }

    /// Exits the current scope by popping the top symbol table off the stack.
    pub fn exit_scope(&mut self) {
        self.scopes.pop();
        // Do NOT remove from scope_history
    }

    /// Inserts a symbol into the current scope.
    /// # Arguments
    /// * `symbol` - The `Symbol` instance to insert.
    pub fn insert(&mut self, symbol: Symbol<'a>) -> Result<(), ()> {
        if let Some(current_scope) = self.scopes.last_mut() {
            let name = symbol.name().to_string();
            let entry = current_scope.entry(name.clone()).or_insert_with(Vec::new);

            let banned_words = vec!["if", "else", "while", "for", "return", "function", "workspace", "project", "stage", "task"];
            if banned_words.contains(&name.as_str()) {
                report!(
                    report::Level::Error,
                    format!(
                        "'{}' is a reserved keyword and cannot be used as a symbol name.",
                        name
                    ),
                    Some("SemanticAnalyzer".into()),
                    None,
                    None
                );
                return Err(());
            }

            match symbol.kind {
                SymbolKind::Function => {
                    if entry.iter().any(|existing| {
                        existing.kind == SymbolKind::Function
                            && existing.parameters == symbol.parameters
                    }) {
                        report!(
                            report::Level::Error,
                            format!(
                                "Function '{}' is already defined with the same signature.",
                                name
                            ),
                            Some("SemanticAnalyzer".into()),
                            None,
                            None
                        );
                        return Err(());
                    }
                    entry.push(symbol);
                }
                _ => {
                    if entry
                        .iter()
                        .any(|existing| existing.kind != SymbolKind::Function)
                    {
                        report!(
                            report::Level::Error,
                            format!("Symbol '{}' is already defined in this scope.", name),
                            Some("SemanticAnalyzer".into()),
                            None,
                            None
                        );
                        return Err(());
                    }
                    entry.push(symbol);
                }
            }

            Ok(())
        } else {
            report!(
                report::Level::Critical,
                "No active scope to insert symbol into.".to_string(),
                Some("SemanticAnalyzer".into()),
                None,
                None
            );
            Err(())
        }
    }

    /// Retrieves a symbol by name, searching from the innermost scope outward.
    /// # Arguments
    /// * `name` - The name of the symbol to retrieve.
    /// # Returns
    /// * `Option<&Vec<Symbol>>` - A reference to a vector of `Symbol` instances if found, or `None` if not found.
    pub fn get(&self, name: &str) -> Option<&Vec<Symbol<'a>>> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbols) = scope.get(name) {
                return Some(symbols);
            }
        }
        None
    }

    /// Retrieves a mutable reference to a symbol by name, searching from the innermost scope outward.
    /// # Arguments
    /// * `name` - The name of the symbol to retrieve.
    /// # Returns
    /// * `Option<&mut Vec<Symbol>>` - A mutable reference to a vector of `Symbol` instances if found, or `None` if not found.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Vec<Symbol<'a>>> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(symbols) = scope.get_mut(name) {
                return Some(symbols);
            }
        }
        None
    }

    /// Creates a `SymbolKind` from an AST node kind.
    /// # Arguments
    /// * `ast_kind` - The AST node kind to convert.
    /// # Returns
    /// * `Option<SymbolKind>` - The corresponding `SymbolKind` if found, or `None` if not found.
    pub fn from_ast_kind(ast_kind: &crate::parser::node::AstType) -> Option<SymbolKind> {
        match ast_kind {
            crate::parser::node::AstType::Workspace { .. } => Some(SymbolKind::Workspace),
            crate::parser::node::AstType::Project { .. } => Some(SymbolKind::Project),
            crate::parser::node::AstType::Stage { .. } => Some(SymbolKind::Stage),
            crate::parser::node::AstType::Task { .. } => Some(SymbolKind::Task),
            _ => None,
        }
    }
}

/// Implements the `Debug` trait for the `SymbolTable` struct.
impl std::fmt::Debug for SymbolTable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, scope) in self.scope_history.iter().enumerate() {
            writeln!(f, "Scope {}:", i)?;
            for (name, symbols) in scope {
                writeln!(f, "  {}: {:?}", name, symbols)?;
            }
        }
        Ok(())
    }
}
