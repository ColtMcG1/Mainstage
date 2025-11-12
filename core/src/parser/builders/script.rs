use crate::parser::{ast::AstNode, driver::Rule};
use crate::scripts::script::Script;

pub(crate) fn process_script_rule<'a>(
    pair: pest::iterators::Pair<'a, Rule>,
    script: &Script,
) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    let mut children = Vec::new();
    for p in pair.into_inner() {
        if p.as_rule() == Rule::EOI { continue; }
        children.push(AstNode::process_node(p, script));
    }
    AstNode {
        id: AstNode::generate_id(),
        kind: crate::parser::types::AstType::Script,
        span: Some(span),
        location: Some(location),
        children,
        attributes: vec![],
    }
}