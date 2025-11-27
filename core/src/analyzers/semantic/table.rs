use super::symbol::Symbol;
use std::collections::HashMap;

// A single scope: name -> overload set
type Scope = HashMap<String, Vec<Symbol>>;

pub struct SymbolTable {
    pub symbols: Vec<Scope>,
}

impl SymbolTable {
    pub fn new() -> Self {
        SymbolTable {
            symbols: vec![HashMap::new()],
        }
    }

    /// ------- Scope Helpers -------

    pub fn enter_scope(&mut self) {
        self.symbols.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        self.symbols.pop();
    }

    /// ------- Symbol Helpers -------

    pub fn insert_symbol(&mut self, symbol: Symbol) {
        if let Some(current_scope) = self.symbols.last_mut() {
            current_scope
                .entry(symbol.name.clone())
                .or_insert_with(Vec::new)
                .push(symbol);
        }
    }

    pub fn lookup_symbol(&self, name: &str) -> Option<&Vec<Symbol>> {
        for scope in self.symbols.iter().rev() {
            if let Some(symbols) = scope.get(name) {
                return Some(symbols);
            }
        }
        None
    }

    pub fn lookup_symbol_mut(&mut self, name: &str) -> Option<&mut Vec<Symbol>> {
        for scope in self.symbols.iter_mut().rev() {
            if let Some(symbols) = scope.get_mut(name) {
                return Some(symbols);
            }
        }
        None
    }

    pub fn remove_symbol(&mut self, name: &str) -> Option<Vec<Symbol>> {
        if let Some(current_scope) = self.symbols.last_mut() {
            return current_scope.remove(name);
        }
        None
    }

    pub fn symbol_exists(&self, name: &str) -> bool {
        self.lookup_symbol(name).is_some()
    }

    pub fn symbol_exists_in_current_scope(&self, name: &str) -> bool {
        if let Some(current_scope) = self.symbols.last() {
            return current_scope.contains_key(name);
        }
        false
    }

    pub fn symbol_exists_in_global_scope(&self, name: &str) -> bool {
        if let Some(global_scope) = self.symbols.first() {
            return global_scope.contains_key(name);
        }
        false
    }

    pub fn collect_symbols_with_kind(&self, kind: super::kind::Kind) -> Vec<&Symbol> {
        let mut collected = Vec::new();

        for scope in &self.symbols {
            for symbols in scope.values() {
                for symbol in symbols {
                    if let Some(inferred_type) = &symbol.inferred_type {
                        if *inferred_type == kind {
                            collected.push(symbol);
                        }
                    }
                }
            }
        }

        collected
    }

    /// ------- Current Scope Helpers -------

    pub fn current_scope(&self) -> Option<&Scope> {
        self.symbols.last()
    }

    pub fn current_scope_mut(&mut self) -> Option<&mut Scope> {
        self.symbols.last_mut()
    }

    /// ------- Global Scope Helpers -------

    pub fn is_global_scope(&self) -> bool {
        self.symbols.len() == 1
    }

    pub fn global_scope(&self) -> &Scope {
        &self.symbols[0]
    }

    pub fn global_scope_mut(&mut self) -> &mut Scope {
        &mut self.symbols[0]
    }

    /// ------- Utility Helpers -------

    pub fn clear(&mut self) {
        self.symbols.clear();
        self.symbols.push(HashMap::new());
    }
}