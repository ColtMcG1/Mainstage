use crate::parser::{ast::AstNode, driver::Rule};
use crate::report;
use crate::scripts::script::Script;

pub(crate) fn unquote(s: &str) -> &str {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') { &s[1..s.len()-1] } else { s }
}

pub(crate) fn unhandled_rule<'a>(pair: pest::iterators::Pair<'a, Rule>, script: &Script) -> AstNode<'a> {
    let span = AstNode::convert_pest_span_to_span(pair.as_span());
    let location = AstNode::convert_pest_span_to_location(pair.as_span(), script);
    report!(
        report::Level::Warning,
        format!("Unhandled rule: {:?}", pair.as_rule()).into(),
        Some("parser.builders.utils".into()),
        Some(span.clone()),
        Some(location.clone())
    );
    AstNode::null()
}