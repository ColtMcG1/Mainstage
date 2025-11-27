use super::kind;

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
    pub(crate) inferred_type: Option<kind::Kind>,

    scope: SymbolScope,

    parameters: Option<Vec<Symbol>>,
    returns: Option<kind::Kind>,

    ref_count: usize,

    location: Option<crate::location::Location>,
    span: Option<crate::location::Span>,
}

impl Symbol {
    pub fn new(
        name: String,
        kind: SymbolKind,
        inferred_type: Option<kind::Kind>,
        scope: SymbolScope,
        parameters: Option<Vec<Symbol>>,
        returns: Option<kind::Kind>,
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
            ref_count: 0,
            location,
            span,
        }
    }

    pub fn new_object(
        name: String,
        scope: SymbolScope,
        arguments: Option<Vec<Symbol>>,
        returns: Option<kind::Kind>,
        location: Option<crate::location::Location>,
        span: Option<crate::location::Span>,
    ) -> Self {
        Symbol::new(
            name,
            SymbolKind::Object,
            Some(kind::Kind::Object),
            scope,
            arguments,
            returns,
            location,
            span,
        )
    }

    pub fn increment_ref_count(&mut self) {
        self.ref_count += 1;
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

    pub fn returns(&self) -> Option<&kind::Kind> {
        self.returns.as_ref()
    }
}