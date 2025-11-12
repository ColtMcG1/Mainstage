use std::borrow::Cow;
use crate::reports::*;
use crate::scripts::script::Script;
use crate::parser::types::AstType;
use crate::parser::driver::Rule;
use crate::parser::builders;

#[derive(Clone)]
pub struct AstNode<'a> {
    pub id: String,
    pub kind: AstType<'a>,
    pub span: Option<locations::Span>,
    pub location: Option<locations::Location<'static>>,
    pub children: Vec<AstNode<'a>>,
    pub attributes: Vec<crate::parser::attributes::Attribute>,
}

impl<'a> AstNode<'a> {
    pub fn new(pairs: pest::iterators::Pairs<'a, Rule>, script: &Script) -> Result<Self, ()> {
        if pairs.clone().count() == 0 { return Err(()); }
        Ok(Self::process_node(pairs.into_iter().next().unwrap(), script))
    }

    pub(crate) fn generate_id() -> String {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        format!("node_{}", id)
    }

    pub(crate) fn convert_pest_span_to_span(span: pest::Span) -> locations::Span {
        locations::Span::new(span.start(), span.end())
    }

    pub(crate) fn convert_pest_span_to_location(
        span: pest::Span,
        script: &Script,
    ) -> locations::Location<'static> {
        let span = Self::convert_pest_span_to_span(span);
        match &script.location(span.start) {
            Some(loc) => loc.clone()
                .with_file(Cow::Owned(script.path().to_string_lossy().into()))
                .into_owned(),
            None => locations::Location {
                file: Cow::Owned(script.path().to_string_lossy().into()),
                line: 0, column: 0,
            },
        }
    }

    // Central dispatcher (thin): forwards to builders
    pub(crate) fn process_node(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> Self {
        match pair.as_rule() {
            Rule::script            => builders::script::process_script_rule(pair, script),
            Rule::statement         => builders::statements::process_statement_rule(pair, script),
            Rule::workspace_decl
            | Rule::project_decl
            | Rule::stage_decl
            | Rule::task_decl       => builders::declarations::process_declaration_rule(pair, script),
            Rule::assignment        => builders::expressions::process_assignment_rule(pair, script),
            Rule::expression        => builders::expressions::process_expression_rule(pair, script),
            Rule::call_expression   => builders::expressions::process_call_expression_rule(pair, script),
            Rule::value             => builders::values::process_value_rule(pair, script),
            Rule::identifier        => builders::expressions::process_identifier_rule(pair, script),
            Rule::EOI               => builders::utils::null_silent(pair, script), // ignore end-of-input
            _                       => builders::utils::fallback_unhandled(pair, script),
        }
    }

    pub fn into_lifetime(self) -> AstNode<'static> {
        AstNode {
            id: self.id,
            kind: self.kind.into_lifetime(),
            span: self.span,
            location: self.location.map(|l| l.into_owned()),
            children: self.children.into_iter().map(|c| c.into_lifetime()).collect(),
            attributes: self.attributes.into_iter().map(|a| a.clone()).collect(),
        }
    }

    pub fn into_owned(self) -> AstNode<'static> {
        AstNode {
            id: self.id,
            kind: self.kind.into_owned(),
            span: self.span.clone(),
            location: self.location.map(|l| l.into_owned()),
            children: self.children.into_iter().map(|c| c.into_owned()).collect(),
            attributes: self.attributes.into_iter().map(|a| a.clone()).collect(),
        }
    }
}

impl<'a> PartialEq for AstNode<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.span == other.span && self.location == other.location && self.children == other.children
    }
}

impl<'a> std::fmt::Debug for AstNode<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AstNode")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("span", &self.span)
            .field("location", &self.location)
            .field("children", &self.children)
            .field("attributes", &self.attributes)
            .finish()
    }
}