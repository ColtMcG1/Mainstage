//! ./semantic/symbol.rs
//!
//! Contains definitions for symbols used in semantic analysis.

use std::borrow::Cow;

use super::types::SymbolType;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum SymbolKind {
    Variable,
    Function,
    Workspace,
    Project,
    Stage,
    Task,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum SymbolScope {
    Local,
    Global,
    Builtin,
}

#[derive(PartialEq, Eq, Clone)]
pub struct Symbol<'a> {
    name: Cow<'a, str>,
    kind: SymbolKind,
    pub(crate) symbol_type: SymbolType,
    scope: SymbolScope,
    pub(crate) parameters: Vec<SymbolType>,
    pub(crate) return_type: SymbolType,
    parent: Option<Box<Symbol<'a>>>,
    reference_count: usize,
    is_reserved: bool,
}

impl<'a> Symbol<'a> {
    pub fn reserved(name: &str) -> Self {
        Symbol {
            name: Cow::Owned(name.to_string()),
            kind: SymbolKind::Variable,
            symbol_type: SymbolType::None,
            scope: SymbolScope::Builtin,
            parameters: Vec::new(),
            return_type: SymbolType::None,
            parent: None,
            reference_count: 0,
            is_reserved: true,
        }
    }

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
            is_reserved: false,
        }
    }

    pub fn new_variable(name: Cow<'a, str>, symbol_type: SymbolType, scope: SymbolScope) -> Self {
        Self::new(name, SymbolKind::Variable, symbol_type, scope)
    }
    pub fn new_function(name: Cow<'a, str>, scope: SymbolScope) -> Self {
        Self::new(name, SymbolKind::Function, SymbolType::None, scope)
    }
    pub fn new_workspace(name: Cow<'a, str>, scope: SymbolScope) -> Self {
        Self::new(name, SymbolKind::Workspace, SymbolType::None, scope)
    }
    pub fn new_project(name: Cow<'a, str>, scope: SymbolScope) -> Self {
        Self::new(name, SymbolKind::Project, SymbolType::None, scope)
    }
    pub fn new_stage(name: Cow<'a, str>, scope: SymbolScope) -> Self {
        Self::new(name, SymbolKind::Stage, SymbolType::None, scope)
    }
    pub fn new_task(name: Cow<'a, str>, scope: SymbolScope) -> Self {
        Self::new(name, SymbolKind::Task, SymbolType::None, scope)
    }

    pub fn with_reserved(mut self) -> Self {
        self.is_reserved = true;
        self
    }

    pub fn name(&self) -> &str { &self.name }
    pub fn kind(&self) -> &SymbolKind { &self.kind }
    pub fn symbol_type(&self) -> &SymbolType { &self.symbol_type }
    pub fn set_symbol_type(&mut self, ty: SymbolType) { self.symbol_type = ty; }
    pub fn scope(&self) -> &SymbolScope { &self.scope }
    pub fn increment_reference_count(&mut self) { self.reference_count += 1; }
    pub fn reference_count(&self) -> usize { self.reference_count }
}

impl<'a> std::fmt::Debug for Symbol<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Symbol {{ name: {}, kind: {:?}, type: {:?}, scope: {:?} }}",
            self.name, self.kind, self.symbol_type, self.scope
        )
    }
}
