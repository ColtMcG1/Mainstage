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

pub struct SymbolTable<'a> {
    /// Mapping of symbol names to their corresponding symbols. Allows for scoping by using a stack of symbol tables and overriding.
    scopes: Vec<HashMap<String, Vec<Symbol<'a>>>>,
}

impl<'a> SymbolTable<'a> {
    /// Creates a new, empty `SymbolTable`.
    /// # Examples
    /// ```
    /// use core::semantic::symbol::{SymbolTable, Symbol};
    /// let mut symbol_table = SymbolTable::new();
    /// ```
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()], // Start with a global scope
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
        self.scopes.push(HashMap::new());
    }

    /// Exits the current scope by popping the top symbol table off the stack.
    pub fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    /// Inserts a symbol into the current scope.
    /// # Arguments
    /// * `symbol` - The `Symbol` instance to insert.
    pub fn insert(&mut self, symbol: Symbol<'a>) {
        if let Some(current_scope) = self.scopes.last_mut() {
            if let Some(symbols) = current_scope.get_mut(symbol.name()) {
                if symbol.kind == SymbolKind::Function {
                    symbols.push(symbol); // Allow overloading for functions
                } else {
                    report!(
                        report::Level::Error,
                        format!(
                            "Symbol '{}' is already defined in this scope",
                            symbol.name()
                        ),
                        Some("mainstage.semantic.symbol.insert".to_string()),
                        None,
                        None
                    );
                }
            } else {
                current_scope.insert(symbol.name().to_string(), vec![symbol]);
            }
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

    /// Pretty prints the symbol table for debugging purposes.
    pub fn pretty_print(&self) {
        for (i, scope) in self.scopes.iter().enumerate() {
            println!("Scope {}:", i);
            for (name, symbols) in scope {
                println!("  {}: {:?}", name, symbols);
            }
        }
    }
}

/// Implements the `Debug` trait for the `SymbolTable` struct.
impl<'a> std::fmt::Debug for SymbolTable<'a> {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.pretty_print();
        Ok(())
    }
}
