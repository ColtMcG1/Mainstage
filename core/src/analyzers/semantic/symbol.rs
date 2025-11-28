use super::kind::{self, InferredKind};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum SymbolKind {
    Variable,
    Function,
    Object
}

#[derive(Debug, Clone)]
pub enum SymbolScope {
    Global,
    Local,
    Builtin
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub(crate) name: String,

    kind: SymbolKind,
    pub(crate) inferred_type: Option<InferredKind>,

    scope: SymbolScope,

    parameters: Option<Vec<Symbol>>,
    returns: Option<InferredKind>,

    /// For object symbols, store properties (fields) declared inside their bodies.
    properties: Option<HashMap<String, Symbol>>,

    ref_count: usize,
    location: Option<crate::location::Location>,
    span: Option<crate::location::Span>,

    /// Recorded usage sites (location + optional span) where this symbol was read.
    /// Kept as a small vector to aid diagnostics (e.g., where a symbol is used).
    pub(crate) usages: Vec<(crate::location::Location, Option<crate::location::Span>)>,
}

impl Symbol {
    pub fn new(
        name: String,
        kind: SymbolKind,
        inferred_type: Option<InferredKind>,
        scope: SymbolScope,
        parameters: Option<Vec<Symbol>>,
    returns: Option<InferredKind>,
        location: Option<crate::location::Location>,
        span: Option<crate::location::Span>,
    ) -> Self {
        Symbol {
            name,
            kind,
            inferred_type,
            scope,
            parameters,
            returns,
            properties: None,
            ref_count: 0,
            location,
            span,
            usages: Vec::new(),
        }
    }

    pub fn new_object(
        name: String,
        scope: SymbolScope,
        arguments: Option<Vec<Symbol>>,
        returns: Option<InferredKind>,
        location: Option<crate::location::Location>,
        span: Option<crate::location::Span>,
    ) -> Self {
        let mut s = Symbol::new(
            name,
            SymbolKind::Object,
            Some(InferredKind::new(
                kind::Kind::Object,
                crate::analyzers::semantic::kind::Origin::Unknown,
                location.clone(),
                span.clone(),
            )),
            scope,
            arguments,
            returns,
            location,
            span,
        );
        s.properties = Some(HashMap::new());
        s
    }

    pub fn new_variable(
        name: String,
        inferred_type: Option<InferredKind>,
        scope: SymbolScope,
        location: Option<crate::location::Location>,
        span: Option<crate::location::Span>,
    ) -> Self {
        Symbol::new(
            name,
            SymbolKind::Variable,
            inferred_type,
            scope,
            None,
            None,
            location,
            span,
        )
    }

    pub fn increment_ref_count(&mut self) {
        self.ref_count += 1;
    }

    /// Record a usage site for this symbol. If `loc` is None nothing is recorded.
    /// Call this when an identifier is resolved/read so diagnostics can point to
    /// where the symbol was referenced.
    pub fn record_usage(&mut self, loc: Option<crate::location::Location>, span: Option<crate::location::Span>) {
        if let Some(l) = loc {
            self.usages.push((l, span));
        }
    }

    pub fn is_referenced(&self) -> bool {
        self.ref_count > 0
    }

    pub fn kind(&self) -> &SymbolKind {
        &self.kind
    }

    pub fn scope(&self) -> &SymbolScope {
        &self.scope
    }

    pub fn parameters(&self) -> Option<&Vec<Symbol>> {
        self.parameters.as_ref()
    }

    pub fn returns(&self) -> Option<&InferredKind> {
        self.returns.as_ref()
    }

    pub fn set_returns(&mut self, k: InferredKind) {
        self.returns = Some(k);
    }

    pub fn inferred_type(&self) -> Option<&InferredKind> {
        self.inferred_type.as_ref()
    }

    pub fn set_inferred_type(&mut self, k: InferredKind) {
        self.inferred_type = Some(k);
    }

    /// Return the original declaration location if available.
    pub fn location(&self) -> Option<crate::location::Location> {
        self.location.clone()
    }

    /// Return the original span if available.
    pub fn span(&self) -> Option<crate::location::Span> {
        self.span.clone()
    }

    /// Insert or replace a property on this symbol (only meaningful for Object symbols).
    pub fn insert_property(&mut self, name: String, symbol: Symbol) {
        if self.properties.is_none() {
            self.properties = Some(HashMap::new());
        }
        if let Some(map) = &mut self.properties {
            map.insert(name, symbol);
        }
    }

    /// Get an immutable reference to a property symbol, if present.
    pub fn get_property(&self, name: &str) -> Option<&Symbol> {
        self.properties.as_ref()?.get(name)
    }

    /// Get a mutable reference to a property symbol, if present.
    pub fn get_property_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        self.properties.as_mut()?.get_mut(name)
    }

    /// Return recorded usage sites (if any).
    pub fn usages(&self) -> &Vec<(crate::location::Location, Option<crate::location::Span>)> {
        &self.usages
    }

    /// Return the last recorded usage site, if any.
    pub fn last_usage(&self) -> Option<&(crate::location::Location, Option<crate::location::Span>)> {
        self.usages.last()
    }
}