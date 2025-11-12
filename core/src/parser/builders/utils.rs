use crate::parser::{ast::AstNode, driver::Rule};
use crate::scripts::script::Script;
use crate::parser::types::AstType;
use crate::reports::*;
use crate::report; // for the report! macro

pub(crate) fn null_node<'a>(
    pair: &pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Null,
        span: Some(span),
        location: Some(location),
        children: vec![],
        attributes: vec![],
    }
}

// Silent variant (no warning)
pub(crate) fn null_silent<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Null,
        span: Some(span),
        location: Some(location),
        children: vec![],
        attributes: vec![],
    }
}

pub(crate) fn fallback_unhandled<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    if pair.as_rule() == Rule::EOI {
        return null_silent(pair, script);
    }
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    report!(
        Level::Warning,
        format!("Unhandled rule: {:?}", pair.as_rule()),
        Some("parser.builders.utils".into()),
        Some(span.clone()),
        Some(location.clone())
    );
    let children = pair.into_inner().map(|p| AstNode::process_node(p, script)).collect();
    AstNode {
        id: AstNode::generate_id(),
        kind: AstType::Null,
        span: Some(span),
        location: Some(location),
        children,
        attributes: vec![],
    }
}