//! ./semantic/scope.rs
//!
//! Symbol table and scoping utilities.

use std::collections::HashMap;

use super::symbol::{Symbol, SymbolKind};
use crate::{report, semantic::builtin};

const HOT_PATH_THRESHOLD: usize = 30;

// A single scope: name -> overload set
type Scope<'a> = HashMap<String, Vec<Symbol<'a>>>;

/// The symbol table used in semantic analysis.
/// Maintains a stack of scopes and allows insertion/lookup with shadowing.
#[derive(Clone)]
pub struct SymbolTable<'a> {
    pub scopes: Vec<Scope<'a>>,        // Active scopes stack
    pub scope_history: Vec<Scope<'a>>, // All scopes ever entered (for debugging)
}

impl<'a> SymbolTable<'a> {
    pub fn new() -> Self {
        let global = Scope::new();
        Self {
            scopes: vec![global.clone()],
            scope_history: vec![global],
        }
    }

    // Diagnostics

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

    // Queries

    /// Returns true if any scope contains a symbol with this name.
    pub fn exists(&self, name: &str) -> bool {
        for scope in self.scopes.iter().rev() {
            if scope.contains_key(name) {
                return true;
            }
        }
        false
    }

    pub fn contains_stage(&self, name: &str) -> bool {
        for scope in self.scopes.iter() {
            if let Some(symbols) = scope.get(name) {
                if symbols.iter().any(|s| s.kind() == &SymbolKind::Stage) {
                    return true;
                }
            }
        }
        false
    }

    pub fn contains_task(&self, name: &str) -> bool {
        for scope in self.scopes.iter() {
            if let Some(symbols) = scope.get(name) {
                if symbols.iter().any(|s| s.kind() == &SymbolKind::Task) {
                    return true;
                }
            }
        }
        false
    }

    /// Get only global symbols (scope index 0).
    pub fn get_global(&self, name: &str) -> Option<&Vec<Symbol<'a>>> {
        self.scopes.first().and_then(|s| s.get(name))
    }

    /// Retrieves all symbols of a specific kind.
    pub fn get_by_kind(&self, kind: SymbolKind) -> Vec<&Symbol<'a>> {
        self.scopes
            .iter()
            .flat_map(|scope| {
                scope
                    .iter()
                    .flat_map(|(_, symbols)| symbols.iter().filter(|symbol| symbol.kind() == &kind))
            })
            .collect()
    }

    /// Retrieves all symbols in the current scope.
    pub fn get_current_scope_symbols(&self) -> Option<&HashMap<String, Vec<Symbol<'a>>>> {
        self.scopes.last()
    }

    /// Checks if a symbol is global.
    pub fn is_global(&self, name: &str) -> bool {
        if let Some(global_scope) = self.scopes.first() {
            global_scope.iter().any(|(n, _)| n == name)
        } else {
            false
        }
    }

    // Scope management

    pub fn enter_scope(&mut self) {
        let new_scope = Scope::new();
        self.scopes.push(new_scope.clone());
        self.scope_history.push(new_scope);
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop();
        // Keep history for debug
    }

    // Mutations

    /// Inserts a symbol into the current scope.
    pub fn insert(&mut self, symbol: Symbol<'a>) -> Result<(), ()> {
        if let Some(current_scope) = self.scopes.last_mut() {
            let name = symbol.name().to_string();
            let entry = current_scope.entry(name.clone()).or_insert_with(Vec::new);

            if builtin::BUILTIN_FUNCS.contains_key(name.as_str())
                || builtin::BUILTIN_METHODS.contains_key(name.as_str())
            {
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

            match symbol.kind() {
                SymbolKind::Function => {
                    if entry.iter().any(|existing| {
                        existing.kind() == &SymbolKind::Function
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
                        .any(|existing| existing.kind() != &SymbolKind::Function)
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

    /// Inserts a reserved symbol into the existing scope.
    pub fn insert_reserved(&mut self, name: &str) {
        if let Some(current_scope) = self.scopes.last_mut() {
            let entry = current_scope
                .entry(name.to_string())
                .or_insert_with(Vec::new);
            entry.push(Symbol::reserved(name));
        }
    }

    /// Retrieves symbols by name (innermost to outermost scope).
    pub fn get(&self, name: &str) -> Option<&Vec<Symbol<'a>>> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbols) = scope.get(name) {
                return Some(symbols);
            }
        }
        None
    }

    /// Retrieves mutable symbols by name (innermost to outermost scope).
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Vec<Symbol<'a>>> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(symbols) = scope.get_mut(name) {
                return Some(symbols);
            }
        }
        None
    }

    /// Maps AST node kind to a `SymbolKind`.
    pub fn from_ast_kind(ast_kind: &crate::parser::AstType) -> Option<SymbolKind> {
        match ast_kind {
            crate::parser::AstType::Workspace { .. } => Some(SymbolKind::Workspace),
            crate::parser::AstType::Project { .. } => Some(SymbolKind::Project),
            crate::parser::AstType::Stage { .. } => Some(SymbolKind::Stage),
            crate::parser::AstType::Task { .. } => Some(SymbolKind::Task),
            _ => None,
        }
    }

    // ADD: does current (innermost) scope already have name
    pub fn contains_in_current(&self, name: &str) -> bool {
        self.scopes
            .last()
            .map(|s| s.contains_key(name))
            .unwrap_or(false)
    }

    pub fn insert_local(&mut self, name: &str) {
        if let Some(current) = self.scopes.last_mut() {
            if current.contains_key(name) { return; }
            current
                .entry(name.to_string())
                .or_insert_with(Vec::new)
                .push(Symbol::new_variable(
                    std::borrow::Cow::Owned(name.to_string()),
                    super::SymbolType::None,
                    super::SymbolScope::Local,
                ));
        }
    }

    pub fn bump_refs(&mut self, name: &str) {
        if let Some(list) = self.get_mut(name) {
            for sym in list {
                sym.increment_reference_count();
            }
        }
    }
}

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
